//! Kamera RAW (CR2/CR3, NEF, ARW, DNG, ORF, RW2, RAF, PEF, SRW) çözümleme.
//!
//! İki aşamalı strateji:
//! 1. **Hızlı yol (öncelikli):** Dosyanın içine gömülü en büyük JPEG önizleme taranır.
//!    Üretici yazılımları bu önizlemeyi kendi görüntü işleme boru hattından geçirerek
//!    üretir; bu nedenle hem hızlıdır hem de renk açısından doğrudur.
//! 2. **Tam çözümleme:** Gömülü önizleme yoksa `rawler` ile sensör verisi çözülür.
//!    Sensör verisi doğrusal kabul edilir ve tepe değere göre normalize edilir;
//!    kamera renk profili uygulanmaz.

use std::path::Path;

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};
use super::cozucu::{HamGoruntu, Pikseller};
use super::{jpeg, renk};

/// JPEG başlangıç (SOI) ve bitiş (EOI) imzaları.
const JPEG_SOI: [u8; 3] = [0xFF, 0xD8, 0xFF];
const JPEG_EOI: [u8; 2] = [0xFF, 0xD9];

/// Gömülü önizleme aramasında incelenecek en fazla aday sayısı: bozuk bir dosyanın
/// tarama süresini patlatmasını engeller.
const EN_COK_ADAY: usize = 512;

/// Dosyaya gömülü en büyük JPEG akışını bulur.
///
/// RAW dosyalarında birden çok önizleme bulunur (küçük EXIF, orta boy, tam çözünürlük);
/// en büyük bayt aralığına sahip olan tam çözünürlüklü önizlemedir.
pub fn gomulu_jpeg_ara(baytlar: &[u8]) -> Option<&[u8]> {
    if baytlar.len() < JPEG_SOI.len() + JPEG_EOI.len() {
        return None;
    }
    let mut en_iyi: Option<(usize, usize)> = None;
    let mut aday = 0usize;
    let mut indis = 0usize;

    while indis + JPEG_SOI.len() <= baytlar.len() && aday < EN_COK_ADAY {
        let Some(goreli) = bul(&baytlar[indis..], &JPEG_SOI) else {
            break;
        };
        let baslangic = indis + goreli;
        let arama_basi = baslangic + JPEG_SOI.len();
        let Some(bitis_goreli) = bul(&baytlar[arama_basi..], &JPEG_EOI) else {
            break;
        };
        let bitis = arama_basi + bitis_goreli + JPEG_EOI.len();
        aday += 1;
        let uzunluk = bitis - baslangic;
        if en_iyi.is_none_or(|(b, s)| uzunluk > s - b) {
            en_iyi = Some((baslangic, uzunluk));
        }
        indis = bitis;
    }

    en_iyi.map(|(baslangic, uzunluk)| &baytlar[baslangic..baslangic + uzunluk])
}

fn bul(igne: &[u8], desen: &[u8]) -> Option<usize> {
    if desen.is_empty() || igne.len() < desen.len() {
        return None;
    }
    igne.windows(desen.len()).position(|p| p == desen)
}

/// RAW dosyasını çözer: önce gömülü önizleme, sonra tam sensör verisi denenir.
pub fn coz(yol: &Path, baytlar: &[u8]) -> Sonuc<HamGoruntu> {
    if let Some(onizleme) = gomulu_jpeg_ara(baytlar) {
        if let Ok(goruntu) = jpeg::coz(onizleme) {
            return Ok(goruntu);
        }
        // Önizleme bozuksa tam çözümlemeye düşülür.
    }
    rawler_ile_coz(yol)
}

/// `rawler` ile sensör verisini çözer ve doğrusal f32 RGBA tamponuna çevirir.
fn rawler_ile_coz(yol: &Path) -> Sonuc<HamGoruntu> {
    let ham = rawler::decode_file(yol)
        .map_err(|k| GorselHatasi::BozukVeri(format!("RAW çözülemedi: {k}")))?;

    let genislik = ham.width as u32;
    let yukseklik = ham.height as u32;
    if genislik == 0 || yukseklik == 0 {
        return Err(GorselHatasi::BozukVeri("RAW sıfır ölçü bildiriyor".into()));
    }

    // Sensör verisi ham tam sayı veya kayan nokta olabilir; tam sayılar ölçeklenmemiş
    // gelir, bu yüzden tepe değere göre normalize edilir.
    let mut ornekler = ham.data.as_f32().into_owned();
    if ornekler.is_empty() {
        return Err(GorselHatasi::BozukVeri("RAW piksel verisi boş".into()));
    }
    tepe_degerine_gore_normalize(&mut ornekler);

    let kanal = ham.cpp.max(1);
    let dogrusal = renk::hdr_tamponu_uret(&ornekler, kanal);
    if dogrusal.is_empty() {
        return Err(GorselHatasi::BozukVeri(format!(
            "RAW kanal düzeni desteklenmiyor: {kanal} kanal"
        )));
    }

    Ok(HamGoruntu {
        genislik,
        yukseklik,
        veri: Pikseller::Rgba32F(dogrusal),
        icc: None,
        yonelim: 1,
    })
}

/// Örnekleri tepe değerin 1,0 olacağı biçimde ölçekler; tepe sıfırsa dokunmaz.
pub fn tepe_degerine_gore_normalize(ornekler: &mut [f32]) {
    let tepe = ornekler
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0f32, f32::max);
    if tepe <= 0.0 {
        return;
    }
    let carpan = 1.0 / tepe;
    for v in ornekler.iter_mut() {
        if v.is_finite() {
            *v *= carpan;
        } else {
            *v = 0.0;
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::goruntu::bicim::{tespit, Bicim, HamTuru};

    fn jpeg_uret(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([90, 120, 200]));
        let mut cikti = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut cikti, image::ImageFormat::Jpeg)
            .expect("JPEG üretilmeli");
        cikti.into_inner()
    }

    /// Gömülü iki JPEG (küçük + büyük) içeren sahte RAW dosyası üretir.
    fn sahte_raw() -> Vec<u8> {
        let kucuk = jpeg_uret(2, 2);
        let buyuk = jpeg_uret(24, 24);
        let mut v = vec![0u8; 64];
        v.extend_from_slice(b"II\x2A\x00");
        v.extend_from_slice(&[0u8; 16]);
        v.extend_from_slice(&kucuk);
        v.extend_from_slice(&[0u8; 32]);
        v.extend_from_slice(&buyuk);
        v.extend_from_slice(&[0u8; 16]);
        v
    }

    #[test]
    fn en_buyuk_gomulu_onizleme_seciliir() {
        let raw = sahte_raw();
        let onizleme = gomulu_jpeg_ara(&raw).expect("önizleme bulunmalı");
        let cozulen = jpeg::coz(onizleme).expect("çözülmeli");
        assert_eq!((cozulen.genislik, cozulen.yukseklik), (24, 24));
    }

    #[test]
    fn gomulu_jpeg_yoksa_none() {
        assert!(gomulu_jpeg_ara(&vec![0x11u8; 512]).is_none());
        assert!(gomulu_jpeg_ara(&[]).is_none());
        assert!(gomulu_jpeg_ara(&[0xFF, 0xD8, 0xFF]).is_none(), "EOI yok");
    }

    #[test]
    fn cr2_bicimi_taninir_ve_onizleme_kullanilir() {
        let mut dosya = sahte_raw();
        dosya[0..2].copy_from_slice(b"II");
        dosya[2..4].copy_from_slice(&42u16.to_le_bytes());
        dosya[8..10].copy_from_slice(b"CR");
        assert_eq!(tespit(&dosya, Some("cr2")), Bicim::KameraHam(HamTuru::Cr2));

        let sonuc = coz(Path::new("test.cr2"), &dosya).expect("önizlemeden çözülmeli");
        assert_eq!(sonuc.genislik, 24);
        assert!(matches!(sonuc.veri, Pikseller::Rgba8(_)));
    }

    #[test]
    fn bozuk_raw_panik_yapmaz() {
        let cop = vec![0x33u8; 300];
        assert!(coz(Path::new(r"C:\yok\gorunmez.cr3"), &cop).is_err());
    }

    #[test]
    fn arama_tum_dosyayi_tarar() {
        let mut v = vec![0u8; 200_000];
        let jpeg = jpeg_uret(8, 8);
        let baslangic = v.len() - jpeg.len() - 10;
        v[baslangic..baslangic + jpeg.len()].copy_from_slice(&jpeg);
        assert!(gomulu_jpeg_ara(&v).is_some(), "sondaki önizleme de bulunmalı");
    }

    #[test]
    fn normalizasyon_tepe_degeri_bire_ceker() {
        let mut v = vec![1000.0f32, 2000.0, 500.0];
        tepe_degerine_gore_normalize(&mut v);
        assert!((v[1] - 1.0).abs() < 1e-6);
        assert!((v[0] - 0.5).abs() < 1e-6);
        assert!((v[2] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn normalizasyon_sifir_ve_nan_guvenli() {
        let mut sifir = vec![0.0f32; 4];
        tepe_degerine_gore_normalize(&mut sifir);
        assert!(sifir.iter().all(|v| *v == 0.0), "sıfırlar değişmemeli");

        let mut nanli = vec![f32::NAN, 4.0, f32::INFINITY];
        tepe_degerine_gore_normalize(&mut nanli);
        assert_eq!(nanli[0], 0.0, "NaN temizlenmeli");
        assert!((nanli[1] - 1.0).abs() < 1e-6);
    }
}
