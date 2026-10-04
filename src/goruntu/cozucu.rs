//! Biçim yönlendirmesi: sihirli baytlara göre doğru çözücüyü seçer.
//!
//! Her biçim en uygun motora yönlendirilir: JPEG → zune-jpeg (AVX2), JPEG XL → jxl-oxide,
//! SVG → resvg, kamera RAW → rawler (gömülü önizleme yedeğiyle), kalan raster biçimler → image.

use std::path::Path;

use image::{DynamicImage, ImageFormat};

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};
use super::{bicim, jpeg, jxl, raw, renk, svg};

/// Çözümlenmiş ham piksel verisi.
#[derive(Debug, Clone)]
pub enum Pikseller {
    /// sRGB kodlu 8-bit RGBA (bitişik, 4 kanal).
    Rgba8(Vec<u8>),
    /// Doğrusal f32 RGBA (bitişik, 4 kanal) — HDR ve kamera RAW kaynakları.
    Rgba32F(Vec<f32>),
}

impl Pikseller {
    pub fn hdr_mi(&self) -> bool {
        matches!(self, Self::Rgba32F(_))
    }
}

/// Çözücülerin ortak çıktısı: ham piksel verisi + renk/yönelim bilgisi.
#[derive(Debug, Clone)]
pub struct HamGoruntu {
    pub genislik: u32,
    pub yukseklik: u32,
    pub veri: Pikseller,
    /// Gömülü ICC profili (varsa).
    pub icc: Option<Vec<u8>>,
    /// EXIF yönelim kodu (1 = dönüşüm yok).
    pub yonelim: u8,
}

/// İstenen çıktı ölçüsü (SVG rasterleştirmesi için).
pub type HedefOlcu = Option<(u32, u32)>;

/// Görseli çözer; biçim sihirli baytlardan belirlenir, uzantı yalnızca yardımcı kanıttır.
pub fn coz(yol: &Path, baytlar: &[u8], hedef: HedefOlcu) -> Sonuc<HamGoruntu> {
    if baytlar.is_empty() {
        return Err(GorselHatasi::BozukVeri("dosya boş".into()));
    }
    let uzanti = yol
        .extension()
        .map(|e| e.to_string_lossy().into_owned());
    let bicim = bicim::tespit(baytlar, uzanti.as_deref());

    let mut goruntu = match bicim {
        bicim::Bicim::Jpeg => jpeg::coz(baytlar)?,
        bicim::Bicim::Jxl => jxl::coz(baytlar)?,
        bicim::Bicim::Svg => svg::coz(baytlar, hedef)?,
        bicim::Bicim::KameraHam(_) => raw::coz(yol, baytlar)?,
        bicim::Bicim::Heic => {
            return Err(GorselHatasi::DesteklenmeyenBicim(
                "HEIC/HEIF (bu sürümde desteklenmiyor; AVIF veya JPEG kullanın)".into(),
            ));
        }
        diger => resim_kutuphanesi_ile_coz(baytlar, diger)?,
    };

    // EXIF yönelimi yalnızca çözücü bildirmediyse dosyadan okunur.
    if goruntu.yonelim == 1 {
        if let Some(kod) = super::meta::exif_yonelim(baytlar) {
            goruntu.yonelim = kod;
        }
    }

    if goruntu.genislik == 0 || goruntu.yukseklik == 0 {
        return Err(GorselHatasi::BozukVeri(
            "görüntü sıfır ölçü bildiriyor".into(),
        ));
    }
    Ok(goruntu)
}

/// `image` kütüphanesi ile çözülen biçimlerin yönlendirmesi.
pub fn resim_formati(bicim: bicim::Bicim) -> Option<ImageFormat> {
    use bicim::Bicim;
    Some(match bicim {
        Bicim::Png => ImageFormat::Png,
        Bicim::WebP => ImageFormat::WebP,
        Bicim::Gif => ImageFormat::Gif,
        Bicim::Bmp => ImageFormat::Bmp,
        Bicim::Tga => ImageFormat::Tga,
        Bicim::Ico => ImageFormat::Ico,
        Bicim::Tiff => ImageFormat::Tiff,
        Bicim::Pnm => ImageFormat::Pnm,
        Bicim::Qoi => ImageFormat::Qoi,
        Bicim::Dds => ImageFormat::Dds,
        Bicim::Hdr => ImageFormat::Hdr,
        Bicim::Exr => ImageFormat::OpenExr,
        Bicim::Avif => ImageFormat::Avif,
        _ => return None,
    })
}

/// Raster biçimleri `image` motoruyla çözer; 32-bit kayan noktalı çıktılar korunur.
fn resim_kutuphanesi_ile_coz(baytlar: &[u8], bicim: bicim::Bicim) -> Sonuc<HamGoruntu> {
    let ad = bicim::bicim_adi(bicim);
    let dinamik = match resim_formati(bicim) {
        Some(format) => image::load_from_memory_with_format(baytlar, format),
        // Sihirli baytı olmayan biçimler için (ör. farbfeld) otomatik algılama denenir.
        None => image::load_from_memory(baytlar),
    }
    .map_err(|k| match k {
        image::ImageError::Unsupported(_) => GorselHatasi::DesteklenmeyenBicim(ad.into()),
        _ => GorselHatasi::BozukVeri(format!("{ad} çözülemedi: {k}")),
    })?;

    let genislik = dinamik.width();
    let yukseklik = dinamik.height();
    let veri = match dinamik {
        DynamicImage::ImageRgb32F(t) => {
            Pikseller::Rgba32F(renk::hdr_tamponu_uret(t.as_raw(), 3))
        }
        DynamicImage::ImageRgba32F(t) => Pikseller::Rgba32F(t.into_raw()),
        diger => Pikseller::Rgba8(diger.to_rgba8().into_raw()),
    };
    Ok(HamGoruntu {
        genislik,
        yukseklik,
        veri,
        icc: icc_cek(baytlar, bicim),
        yonelim: 1,
    })
}

/// Gömülü ICC profilini çözücüden okur; desteklenmiyor veya yoksa `None`.
fn icc_cek(baytlar: &[u8], bicim: bicim::Bicim) -> Option<Vec<u8>> {
    use image::ImageDecoder;
    let format = resim_formati(bicim)?;
    let okuyucu = image::ImageReader::with_format(std::io::Cursor::new(baytlar), format);
    let mut cozucu = okuyucu.into_decoder().ok()?;
    cozucu.icc_profile().ok().flatten()
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::goruntu::bicim::Bicim;

    fn png_uret(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([(x * 7) as u8, (y * 11) as u8, 200, 255])
        });
        let mut cikti = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut cikti, image::ImageFormat::Png)
            .expect("PNG üretilmeli");
        cikti.into_inner()
    }

    #[test]
    fn png_cozulur() {
        let baytlar = png_uret(12, 9);
        let g = coz(Path::new("test.png"), &baytlar, None).expect("çözülmeli");
        assert_eq!((g.genislik, g.yukseklik), (12, 9));
        let Pikseller::Rgba8(v) = &g.veri else {
            panic!("PNG RGBA8 olmalı");
        };
        assert_eq!(v.len(), 12 * 9 * 4);
    }

    #[test]
    fn webp_ve_bmp_yonlendirilir() {
        let img = image::RgbaImage::from_pixel(5, 5, image::Rgba([10, 20, 30, 255]));
        for (ad, format) in [
            ("a.bmp", image::ImageFormat::Bmp),
            ("a.tga", image::ImageFormat::Tga),
            ("a.png", image::ImageFormat::Png),
        ] {
            let mut cikti = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(img.clone())
                .write_to(&mut cikti, format)
                .expect("yazılmalı");
            let baytlar = cikti.into_inner();
            let g = coz(Path::new(ad), &baytlar, None).expect("çözülmeli");
            assert_eq!((g.genislik, g.yukseklik), (5, 5), "{ad}");
        }
    }

    #[test]
    fn bicim_yonlendirmesi_dogru() {
        assert_eq!(resim_formati(Bicim::Png), Some(ImageFormat::Png));
        assert_eq!(resim_formati(Bicim::Exr), Some(ImageFormat::OpenExr));
        assert_eq!(resim_formati(Bicim::Avif), Some(ImageFormat::Avif));
        assert_eq!(resim_formati(Bicim::Svg), None);
        assert_eq!(resim_formati(Bicim::Jpeg), None, "JPEG ayrı motorda");
    }

    #[test]
    fn heic_acik_mesajla_reddedilir() {
        let mut sahte = vec![0u8, 0, 0, 0x20];
        sahte.extend_from_slice(b"ftypheic");
        sahte.extend_from_slice(&[0u8; 16]);
        match coz(Path::new("a.heic"), &sahte, None) {
            Err(GorselHatasi::DesteklenmeyenBicim(m)) => assert!(m.contains("HEIC")),
            diger => panic!("HEIC reddedilmeliydi: {diger:?}"),
        }
    }

    #[test]
    fn bos_dosya_hata() {
        assert!(coz(Path::new("a.png"), &[], None).is_err());
    }

    #[test]
    fn taninmayan_veri_hata_dondurur() {
        let cop = vec![0x11u8; 512];
        assert!(coz(Path::new("a.dat"), &cop, None).is_err());
    }

    #[test]
    fn hdr_isareti_dogru() {
        assert!(!Pikseller::Rgba8(vec![]).hdr_mi());
        assert!(Pikseller::Rgba32F(vec![]).hdr_mi());
    }
}
