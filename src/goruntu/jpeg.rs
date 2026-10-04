//! JPEG çözümleme: x86_64 AVX2 optimizasyonlu `zune-jpeg` motoru.
//!
//! JPEG'in YCbCr→RGB dönüşümü sRGB primaries varsayımıyla yapıldığı için çıktı
//! sRGB kodlu kabul edilir; gömülü ICC profili varsa renk yönetimi ayrıca uygulanır.

use zune_jpeg::JpegDecoder;

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};
use super::cozucu::{HamGoruntu, Pikseller};

/// JPEG baytlarını çözer.
///
/// Çözücü, geri sarma (`Seek`) yeteneği isteyen bir akış okur; bu yüzden bellekteki
/// baytlar imleç (`Cursor`) üzerinden verilir.
pub fn coz(baytlar: &[u8]) -> Sonuc<HamGoruntu> {
    let mut cozucu = JpegDecoder::new(std::io::Cursor::new(baytlar));
    let pikseller = cozucu
        .decode()
        .map_err(|k| GorselHatasi::BozukVeri(format!("JPEG çözülemedi: {k}")))?;

    let bilgi = cozucu
        .info()
        .ok_or_else(|| GorselHatasi::BozukVeri("JPEG başlığı okunamadı".into()))?;
    let genislik = u32::from(bilgi.width);
    let yukseklik = u32::from(bilgi.height);
    if genislik == 0 || yukseklik == 0 {
        return Err(GorselHatasi::BozukVeri(
            "JPEG sıfır ölçülü görüntü bildiriyor".into(),
        ));
    }

    // Çözücü, kanal sayısına göre bitişik tampon üretir (gri: 1, renkli: 3).
    let beklenen_renkli = genislik as usize * yukseklik as usize * 3;
    let rgba = match pikseller.len() {
        4 => 4,
        1 => 1,
        _ if pikseller.len() >= beklenen_renkli => 3,
        _ => {
            return Err(GorselHatasi::BozukVeri(format!(
                "JPEG piksel tamponu beklenenden küçük: {} < {beklenen_renkli}",
                pikseller.len()
            )));
        }
    };

    let rgba8 = rgba_ya_genislet(&pikseller, rgba, genislik, yukseklik);
    let icc = cozucu.icc_profile();
    Ok(HamGoruntu {
        genislik,
        yukseklik,
        veri: Pikseller::Rgba8(rgba8),
        icc,
        // JPEG yönelimi EXIF'ten okunur, çözücüden değil.
        yonelim: 1,
    })
}

/// 1/3/4 kanallı bitişik tamponu RGBA8'e genişletir.
pub fn rgba_ya_genislet(kaynak: &[u8], kanal: usize, genislik: u32, yukseklik: u32) -> Vec<u8> {
    let piksel_sayisi = genislik as usize * yukseklik as usize;
    let mut cikti = Vec::with_capacity(piksel_sayisi * 4);
    match kanal {
        4 => cikti.extend_from_slice(&kaynak[..piksel_sayisi * 4]),
        3 => {
            for p in kaynak.chunks_exact(3).take(piksel_sayisi) {
                cikti.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        1 => {
            for v in kaynak.iter().take(piksel_sayisi) {
                cikti.extend_from_slice(&[*v, *v, *v, 255]);
            }
        }
        _ => {}
    }
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::goruntu::bicim::{tespit, Bicim};

    /// jpeg-encoder bağımlılığı olmadan geçerli bir JPEG üretir: 8x8 gri, kalite 90.
    fn ornek_jpeg() -> Vec<u8> {
        use image::{ImageFormat, RgbImage};
        let mut img = RgbImage::new(8, 8);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = image::Rgb([(x * 30) as u8, (y * 30) as u8, 128]);
        }
        let mut cikti = std::io::Cursor::new(Vec::new());
        img.write_to(&mut cikti, ImageFormat::Jpeg)
            .expect("JPEG üretilmeli");
        cikti.into_inner()
    }

    #[test]
    fn jpeg_cozulur_ve_rgba_uretir() {
        let baytlar = ornek_jpeg();
        assert_eq!(tespit(&baytlar, Some("jpg")), Bicim::Jpeg);
        let goruntu = coz(&baytlar).expect("çözülmeli");
        assert_eq!(goruntu.genislik, 8);
        assert_eq!(goruntu.yukseklik, 8);
        let Pikseller::Rgba8(veri) = &goruntu.veri else {
            panic!("JPEG çıktısı RGBA8 olmalı");
        };
        assert_eq!(veri.len(), 8 * 8 * 4);
    }

    #[test]
    fn renkler_yakin_cikar() {
        let baytlar = ornek_jpeg();
        let goruntu = coz(&baytlar).expect("çözülmeli");
        let Pikseller::Rgba8(veri) = &goruntu.veri else {
            panic!("RGBA8 bekleniyordu");
        };
        // İlk piksel (0,0): kırmızı 0, yeşil 0, mavi 128 civarı olmalı.
        assert!(veri[0] < 40, "kırmızı düşük olmalı: {}", veri[0]);
        assert!((100..=160).contains(&veri[2]), "mavi ~128 olmalı: {}", veri[2]);
        assert_eq!(veri[3], 255, "alfa tam opak olmalı");
    }

    #[test]
    fn bozuk_veri_hata_dondurur() {
        let bozuk = vec![0xFF, 0xD8, 0xFF, 0x00, 0x11, 0x22, 0x33];
        assert!(coz(&bozuk).is_err());
    }

    #[test]
    fn bos_veri_panik_yapmaz() {
        assert!(coz(&[]).is_err());
    }

    #[test]
    fn gri_tampon_rgba_ya_genisler() {
        let gri = vec![10u8, 20, 30, 40];
        let rgba = rgba_ya_genislet(&gri, 1, 2, 2);
        assert_eq!(rgba, vec![10, 10, 10, 255, 20, 20, 20, 255, 30, 30, 30, 255, 40, 40, 40, 255]);
    }

    #[test]
    fn rgb_tampon_alfa_ekler() {
        let rgb = vec![1u8, 2, 3, 4, 5, 6];
        let rgba = rgba_ya_genislet(&rgb, 3, 2, 1);
        assert_eq!(rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }
}
