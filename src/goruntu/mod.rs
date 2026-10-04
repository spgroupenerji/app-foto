//! Görüntü işleme boru hattı: dosya baytlarından GPU'ya yüklenecek doğrusal f16 tampona.
//!
//! Aşamalar: çözümleme → EXIF yönelimi → (gerekirse) ICC dönüşümü → doğrusallastırma →
//! (gerekirse) CPU'da SIMD küçültme → yarım duyarlıklı (f16) paketleme.

pub mod bicim;
pub mod cozucu;
pub mod jpeg;
pub mod jxl;
pub mod meta;
pub mod olcekleme;
pub mod raw;
pub mod renk;
pub mod svg;

use std::path::Path;

use crate::cekirdek::ayar::Ayarlar;
use crate::cekirdek::hata::{GorselHatasi, Sonuc};
use crate::gio::yol;
use cozucu::{HamGoruntu, Pikseller};
use meta::MetaBilgi;

/// GPU'ya yüklenmeye hazır görüntü: doğrusal scRGB, yarım duyarlıklı (f16) RGBA.
#[derive(Debug, Clone)]
pub struct IslenmisGoruntu {
    pub genislik: u32,
    pub yukseklik: u32,
    /// f16 RGBA, bitişik.
    pub veri: Vec<u16>,
    pub meta: MetaBilgi,
    /// Kaynak yüksek dinamik aralıklı mı (ton haritalama uygulanır).
    pub hdr: bool,
}

impl IslenmisGoruntu {
    /// VRAM'de kaplayacağı tahmini boyut (bayt): 4 kanal × 2 bayt.
    pub fn vram_boyutu(&self) -> u64 {
        u64::from(self.genislik) * u64::from(self.yukseklik) * 8
    }
}

/// Görseli okur, çözer ve GPU'ya hazır hale getirir.
///
/// `hedef` yalnızca SVG rasterleştirmesi için kullanılır; SVG bu ölçüde üretilir.
pub fn isle(yol: &Path, ayarlar: &Ayarlar, hedef: Option<(u32, u32)>) -> Sonuc<IslenmisGoruntu> {
    let baytlar = dosya_oku(yol)?;
    let ham = cozucu::coz(yol, &baytlar, hedef)?;

    // 1) EXIF yönelimi ham veriye uygulanır.
    let ham = yonelim_uygula(ham)?;

    let ham_genislik = ham.genislik;
    let ham_yukseklik = ham.yukseklik;
    let hdr = ham.veri.hdr_mi();

    // 2) Ön küçültme kaynağın KENDİ biçiminde yapılır: 8-bit veri için SIMD u8 yolu kullanılır,
    //    böylece bellek trafiği dörde iner ve doğrusallastırma küçültülmüş veri üzerinde yapılır.
    let kucultme = hedef.and_then(|(hedef_w, hedef_h)| {
        olcekleme::kucultme_gerekli(ham_genislik, ham_yukseklik, hedef_w, hedef_h)
    });
    let filtre = olcekleme::filtre_secimi(ayarlar.olcek_filtresi);

    let (kaynak, genislik, yukseklik, kucultuldu) = match ham.veri {
        Pikseller::Rgba8(rgba) => match kucultme {
            Some((yeni_w, yeni_h)) => (
                Pikseller::Rgba8(olcekleme::olcekle_rgba8(
                    &rgba,
                    ham_genislik,
                    ham_yukseklik,
                    yeni_w,
                    yeni_h,
                    filtre,
                )?),
                yeni_w,
                yeni_h,
                true,
            ),
            None => (Pikseller::Rgba8(rgba), ham_genislik, ham_yukseklik, false),
        },
        Pikseller::Rgba32F(f) => match kucultme {
            Some((yeni_w, yeni_h)) => (
                Pikseller::Rgba32F(olcekleme::olcekle_rgba32f(
                    &f,
                    ham_genislik,
                    ham_yukseklik,
                    yeni_w,
                    yeni_h,
                    filtre,
                )?),
                yeni_w,
                yeni_h,
                true,
            ),
            None => (Pikseller::Rgba32F(f), ham_genislik, ham_yukseklik, false),
        },
    };

    // 3) Renk: ICC varsa dönüştürülür, ardından doğrusal f32 tampona geçilir.
    let (dogrusal, icc_uygulandi) = match &kaynak {
        Pikseller::Rgba8(rgba) => renk::dogrusal_tampon_uret(rgba, ham.icc.as_deref()),
        Pikseller::Rgba32F(f) => (f.clone(), false),
    };

    // 4) f16 paketleme (VRAM'de yarı yer, HDR aralığını korur).
    let veri = f16_paketle(&dogrusal);

    let dosya_boyutu = MetaBilgi::dosya_boyutu_oku(yol);
    let meta = MetaBilgi {
        dosya_adi: yol
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        yol: yol.to_path_buf(),
        bicim: bicim::tespit(&baytlar, yol.extension().and_then(|e| e.to_str())),
        ham_genislik,
        ham_yukseklik,
        genislik,
        yukseklik,
        dosya_boyutu,
        icc_var: ham.icc.is_some(),
        icc_uygulandi,
        kucultuldu,
        hdr,
        yonelim: ham.yonelim,
    };

    Ok(IslenmisGoruntu {
        genislik,
        yukseklik,
        veri,
        meta,
        hdr,
    })
}

/// Dosyayı tamponlu okuma ile baştan sona okur.
///
/// Ağ yollarında bile bellek eşleme kullanılmaz; okuma Win32 sıralı tarama ipucuyla yapılır.
pub fn dosya_oku(yol: &Path) -> Sonuc<Vec<u8>> {
    use std::io::Read;
    let mut dosya = yol::okuma_icin_ac(yol)?;
    let boyut = dosya
        .metadata()
        .map(|m| m.len())
        .map_err(|k| GorselHatasi::okuma(yol, k))?;
    let mut tampon = Vec::with_capacity(boyut.min(MAX_DOSYA_ON_BELLEK) as usize);
    dosya
        .read_to_end(&mut tampon)
        .map_err(|k| GorselHatasi::okuma(yol, k))?;
    Ok(tampon)
}

/// Aşırı büyük dosyalarda ön bellek ayırmayı sınırlayan üst değer (256 MB).
const MAX_DOSYA_ON_BELLEK: u64 = 256 * 1024 * 1024;

/// EXIF yönelimini ham görüntüye uygular (8-bit tamponlarda yerinde dönüşüm).
fn yonelim_uygula(ham: HamGoruntu) -> Sonuc<HamGoruntu> {
    if ham.yonelim <= 1 {
        return Ok(ham);
    }
    let Pikseller::Rgba8(rgba) = &ham.veri else {
        // HDR tamponlar için yönelim dönüşümü 8-bite indirgenmeden yapılmaz; atlanır.
        return Ok(ham);
    };
    let Some((yeni, g, y)) = meta::yonelim_uygula(rgba, ham.genislik, ham.yukseklik, ham.yonelim)
    else {
        return Ok(ham);
    };
    Ok(HamGoruntu {
        genislik: g,
        yukseklik: y,
        veri: Pikseller::Rgba8(yeni),
        icc: ham.icc,
        yonelim: ham.yonelim,
    })
}

/// Doğrusal f32 tamponu f16'ya paketler.
pub fn f16_paketle(dogrusal: &[f32]) -> Vec<u16> {
    dogrusal
        .iter()
        .map(|v| half::f16::from_f32(*v).to_bits())
        .collect()
}

#[cfg(test)]
mod testler {
    use super::*;

    fn png_yaz(dir: &Path, ad: &str, w: u32, h: u32) -> std::path::PathBuf {
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
        });
        let yol = dir.join(ad);
        image::DynamicImage::ImageRgba8(img)
            .save_with_format(&yol, image::ImageFormat::Png)
            .expect("PNG yazılmalı");
        yol
    }

    #[test]
    fn tam_boru_hatti_f16_uretir() {
        let dizin = std::env::temp_dir().join("gorsel-boru-testi");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = png_yaz(&dizin, "a.png", 20, 10);

        let ayar = Ayarlar::default();
        let islenmis = isle(&yol, &ayar, Some((4000, 3000))).expect("işlenmeli");
        assert_eq!((islenmis.genislik, islenmis.yukseklik), (20, 10));
        assert_eq!(islenmis.veri.len(), 20 * 10 * 4, "f16 RGBA tamponu");
        assert!(!islenmis.hdr, "PNG HDR değil");
        assert!(!islenmis.meta.kucultuldu, "küçük görsel küçültülmez");
        assert_eq!(islenmis.meta.ham_genislik, 20);
        assert_eq!(islenmis.vram_boyutu(), 20 * 10 * 8);

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn buyuk_goruntu_cpu_da_kucultulur() {
        let dizin = std::env::temp_dir().join("gorsel-boru-kucultme");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = png_yaz(&dizin, "buyuk.png", 600, 400);

        let ayar = Ayarlar::default();
        let islenmis = isle(&yol, &ayar, Some((100, 100))).expect("işlenmeli");
        assert!(islenmis.meta.kucultuldu, "600x400, hedef 100x100 eşiğini aşar");
        assert!(
            !islenmis.meta.icc_uygulandi,
            "ICC profili olmayan PNG'de dönüşüm uygulanmamalı"
        );
        assert_eq!(islenmis.genislik, 200, "hedefin 2 katına küçültülür");
        assert_eq!(islenmis.yukseklik, 133);
        assert!(islenmis.meta.ham_genislik == 600, "ham ölçü meta veride korunur");

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn f16_paketleme_degerleri_korur() {
        let paket = f16_paketle(&[0.0, 0.5, 1.0, 2.5]);
        assert_eq!(paket.len(), 4);
        assert!((half::f16::from_bits(paket[1]).to_f32() - 0.5).abs() < 1e-3);
        assert!((half::f16::from_bits(paket[3]).to_f32() - 2.5).abs() < 1e-2);
    }

    #[test]
    fn olmayan_dosya_hata_dondurur() {
        let ayar = Ayarlar::default();
        assert!(isle(Path::new(r"C:\yok\boyle\dosya.png"), &ayar, None).is_err());
    }
}
