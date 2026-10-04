//! JPEG XL çözümleme: saf Rust `jxl-oxide` motoru (harici C bağımlılığı yok).
//!
//! Çözücüden 8-bit RGBA istenir; `jxl-oxide` örnekleri dosyanın renk kodlamasından
//! sRGB'ye eşleyerek yazdığı için çıktı sRGB kodlu kabul edilir ve boru hattının
//! geri kalanıyla tutarlıdır. PQ/HLG ile kodlanmış yüksek dinamik aralıklı içerik
//! SDR aralığa eşlenerek sunulur.

use std::io::Cursor;

use jxl_oxide::JxlImage;

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};
use super::cozucu::{HamGoruntu, Pikseller};

/// Fazladan kanalları (nokta renkleri) karşılamak için ayrılan kanal üst sınırı.
const MAX_KANAL: usize = 8;

/// Ayrım üst sınırı: 8192×8192 piksel (512 MiB tampon; image crate öntanımlı sınırıyla aynı).
const EN_COK_PIXEL: usize = 67_108_864;

/// JPEG XL baytlarını çözer.
pub fn coz(baytlar: &[u8]) -> Sonuc<HamGoruntu> {
    let goruntu = JxlImage::builder()
        .read(Cursor::new(baytlar))
        .map_err(|k| GorselHatasi::BozukVeri(format!("JPEG XL başlığı okunamadı: {k}")))?;

    let kare = goruntu
        .render_frame(0)
        .map_err(|k| GorselHatasi::BozukVeri(format!("JPEG XL çözülemedi: {k}")))?;

    let yonelim = (kare.orientation() as u8).clamp(1, 8);
    // Çerçeve, kanal akışı üzerinden okunur: `ImageStream` kanal sayısını bildirir ve
    // 8-bit örnekleri dosyanın renk kodlamasından sRGB'ye çevirerek yazar.
    let mut akis = kare.stream();
    let genislik = akis.width();
    let yukseklik = akis.height();
    if genislik == 0 || yukseklik == 0 {
        return Err(GorselHatasi::BozukVeri(
            "JPEG XL sıfır ölçülü görüntü bildiriyor".into(),
        ));
    }

    let piksel_sayisi = genislik as usize * yukseklik as usize;
    // Başlıktaki ölçü güvenilmeyendir: üst sınır denetimi olmadan ayrım, bozuk/amatör
    // dosyayla süreci bellek yetersizliğine sokar.
    if piksel_sayisi > EN_COK_PIXEL {
        return Err(GorselHatasi::BellekYetersiz {
            genislik,
            yukseklik,
            bayt: (piksel_sayisi * MAX_KANAL) as u64,
        });
    }
    // Nokta renkleri gibi fazladan kanallar olabileceği için tampon 4 kanaldan büyük
    // ayrılır; gerçek kanal sayısı yazılan örnek sayısından türetilir.
    let mut tampon = vec![0u8; piksel_sayisi * MAX_KANAL];
    let yazilan = akis.write_to_buffer(&mut tampon);
    if yazilan == 0 {
        return Err(GorselHatasi::BozukVeri("JPEG XL piksel üretmedi".into()));
    }
    let kanal = (yazilan / piksel_sayisi).max(1);
    let rgba = kanallari_rgba_ya(&tampon, yazilan, kanal, genislik, yukseklik);

    Ok(HamGoruntu {
        genislik,
        yukseklik,
        veri: Pikseller::Rgba8(rgba),
        icc: None,
        yonelim,
    })
}

/// Kanallı (1/2/3/4+) bitişik 8-bit tamponu RGBA8'e genişletir.
pub fn kanallari_rgba_ya(
    kaynak: &[u8],
    yazilan: usize,
    kanal: usize,
    genislik: u32,
    yukseklik: u32,
) -> Vec<u8> {
    let piksel_sayisi = genislik as usize * yukseklik as usize;
    let mut cikti = Vec::with_capacity(piksel_sayisi * 4);
    let kullanilabilir = yazilan.min(kaynak.len());
    match kanal {
        4 | 5 | 6 => {
            for p in kaynak[..kullanilabilir].chunks_exact(kanal).take(piksel_sayisi) {
                cikti.extend_from_slice(&[p[0], p[1], p[2], p[3]]);
            }
        }
        3 => {
            for p in kaynak[..kullanilabilir].chunks_exact(3).take(piksel_sayisi) {
                cikti.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        2 => {
            for p in kaynak[..kullanilabilir].chunks_exact(2).take(piksel_sayisi) {
                cikti.extend_from_slice(&[p[0], p[0], p[0], p[1]]);
            }
        }
        _ => {
            for v in kaynak[..kullanilabilir].iter().take(piksel_sayisi) {
                cikti.extend_from_slice(&[*v, *v, *v, 255]);
            }
        }
    }
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bozuk_veri_hata_dondurur_panik_yok() {
        assert!(coz(&[0xFFu8; 256]).is_err());
        assert!(coz(&[]).is_err());
    }

    #[test]
    fn jxl_imzasi_taninir_ve_hata_turkce() {
        let imza = [0xFFu8, 0x0A, 0x00, 0x00];
        let hata = coz(&imza).expect_err("eksik kod akışı hata vermeli");
        assert!(hata.to_string().contains("JPEG XL"), "gelen: {hata}");
    }

    #[test]
    fn uc_kanal_rgba_ya_genisler() {
        let kaynak = [10u8, 20, 30, 40, 50, 60];
        let rgba = kanallari_rgba_ya(&kaynak, 6, 3, 2, 1);
        assert_eq!(rgba, vec![10, 20, 30, 255, 40, 50, 60, 255]);
    }

    #[test]
    fn dort_kanal_korunur() {
        let kaynak = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let rgba = kanallari_rgba_ya(&kaynak, 8, 4, 2, 1);
        assert_eq!(rgba, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn gri_kanal_rgb_ye_yayilir() {
        let kaynak = [7u8, 9];
        let rgba = kanallari_rgba_ya(&kaynak, 2, 1, 2, 1);
        assert_eq!(rgba, vec![7, 7, 7, 255, 9, 9, 9, 255]);
    }

    #[test]
    fn fazla_kanal_ilk_dordunu_alir() {
        // 5 kanal (nokta rengi): ilk 4 kanal kullanılır, taşma olmaz.
        let kaynak = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let rgba = kanallari_rgba_ya(&kaynak, 10, 5, 2, 1);
        assert_eq!(rgba, vec![1, 2, 3, 4, 6, 7, 8, 9]);
    }

    #[test]
    fn eksik_veri_panik_yapmaz() {
        // Bildirilen kanal 4 ama veri kısa: kalan pikseller atlanır.
        let kaynak = [1u8, 2, 3];
        let rgba = kanallari_rgba_ya(&kaynak, 3, 4, 2, 1);
        assert!(rgba.is_empty(), "eksik veri sessizce boş bırakılır");
    }
}
