//! CPU üzerinde SIMD hızlandırmalı yeniden örnekleme.
//!
//! Görüntü ekran çözünürlüğünden belirgin biçimde büyükse, GPU'ya göndermeden önce
//! `fast_image_resize` ile (AVX2/SSE4.1) küçültülür. Bu, VRAM tüketimini ve doku yükleme
//! süresini düşürürken ekranda görülemeyecek detayı hesaplamayı önler; geri kalan
//! ölçekleme işi GPU'da 4-örneklemeli Catmull-Rom gölgelendiricisine bırakılır.

use fast_image_resize::{
    images::{Image, ImageRef},
    FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer,
};

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};

/// Ön küçültme eşiği: görüntü, hedefin bu katından büyükse CPU'da küçültülür.
pub const ON_KUCULTME_KATI: u32 = 2;

/// Ayarlardan gelen filtre seçimini kütüphane filtresine çevirir.
pub fn filtre_secimi(filtre: super::super::cekirdek::ayar::Filtre) -> FilterType {
    use super::super::cekirdek::ayar::Filtre;
    match filtre {
        Filtre::CatmullRom => FilterType::CatmullRom,
        Filtre::Bilinear => FilterType::Bilinear,
        Filtre::Lanczos3 => FilterType::Lanczos3,
    }
}

/// Görüntüyü hedef ölçüye küçültmek gerekip gerekmediğini söyler.
///
/// Yalnızca küçültme yapılır; büyütme GPU gölgelendiricisinin işidir.
pub fn kucultme_gerekli(genislik: u32, yukseklik: u32, hedef_w: u32, hedef_h: u32) -> Option<(u32, u32)> {
    if genislik == 0 || yukseklik == 0 || hedef_w == 0 || hedef_h == 0 {
        return None;
    }
    let esik_w = hedef_w.saturating_mul(ON_KUCULTME_KATI);
    let esik_h = hedef_h.saturating_mul(ON_KUCULTME_KATI);
    if genislik <= esik_w && yukseklik <= esik_h {
        return None;
    }
    // En-boy oranı korunur; hedef, oranı bozmayan en büyük ölçüdür.
    let oran_w = f64::from(esik_w) / f64::from(genislik);
    let oran_h = f64::from(esik_h) / f64::from(yukseklik);
    let oran = oran_w.min(oran_h);
    let yeni_w = (f64::from(genislik) * oran).round().max(1.0) as u32;
    let yeni_h = (f64::from(yukseklik) * oran).round().max(1.0) as u32;
    Some((yeni_w, yeni_h))
}

/// 8-bit RGBA tamponunu SIMD ile küçültür.
pub fn olcekle_rgba8(
    kaynak: &[u8],
    genislik: u32,
    yukseklik: u32,
    hedef_w: u32,
    hedef_h: u32,
    filtre: FilterType,
) -> Sonuc<Vec<u8>> {
    let beklenen = genislik as usize * yukseklik as usize * 4;
    if kaynak.len() < beklenen {
        return Err(GorselHatasi::BozukVeri(format!(
            "ölçekleme girdisi eksik: {} < {beklenen}",
            kaynak.len()
        )));
    }
    let kaynak_goruntu =
        ImageRef::new(genislik, yukseklik, kaynak, PixelType::U8x4).map_err(olcekleme_hatasi)?;
    let mut hedef = Image::new(hedef_w, hedef_h, PixelType::U8x4);
    let secenekler = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filtre));
    let mut boyutlandirici = Resizer::new();
    boyutlandirici
        .resize(&kaynak_goruntu, &mut hedef, Some(&secenekler))
        .map_err(olcekleme_hatasi)?;
    Ok(hedef.buffer().to_vec())
}

/// 32-bit kayan noktalı RGBA tamponunu SIMD ile küçültür (HDR kaynaklar).
pub fn olcekle_rgba32f(
    kaynak: &[f32],
    genislik: u32,
    yukseklik: u32,
    hedef_w: u32,
    hedef_h: u32,
    filtre: FilterType,
) -> Sonuc<Vec<f32>> {
    let beklenen = genislik as usize * yukseklik as usize * 4;
    if kaynak.len() < beklenen {
        return Err(GorselHatasi::BozukVeri(format!(
            "ölçekleme girdisi eksik: {} < {beklenen}",
            kaynak.len()
        )));
    }
    let baytlar: &[u8] = bytemuck::cast_slice(kaynak);
    let kaynak_goruntu =
        ImageRef::new(genislik, yukseklik, baytlar, PixelType::F32x4).map_err(olcekleme_hatasi)?;
    let mut hedef = Image::new(hedef_w, hedef_h, PixelType::F32x4);
    let secenekler = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filtre));
    let mut boyutlandirici = Resizer::new();
    boyutlandirici
        .resize(&kaynak_goruntu, &mut hedef, Some(&secenekler))
        .map_err(olcekleme_hatasi)?;
    Ok(bytemuck::cast_slice(hedef.buffer()).to_vec())
}

fn olcekleme_hatasi(k: impl std::fmt::Display) -> GorselHatasi {
    GorselHatasi::BozukVeri(format!("ölçekleme başarısız: {k}"))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn duz_tampon(w: u32, h: u32, renk: [u8; 4]) -> Vec<u8> {
        renk.iter()
            .copied()
            .cycle()
            .take(w as usize * h as usize * 4)
            .collect()
    }

    #[test]
    fn kucuk_goruntu_kucultulmez() {
        assert_eq!(kucultme_gerekli(100, 100, 1920, 1080), None);
        assert_eq!(kucultme_gerekli(1920, 1080, 1920, 1080), None);
    }

    #[test]
    fn buyuk_goruntu_orani_korunarak_kucultulur() {
        let (w, h) = kucultme_gerekli(8000, 6000, 1000, 1000).expect("küçültülmeli");
        // Hedef eşiği 2000x2000; 8000x6000 için sınırlayıcı eksen genişliktir.
        assert_eq!((w, h), (2000, 1500));
        let oran = w as f64 / h as f64;
        assert!((oran - 8000.0 / 6000.0).abs() < 1e-6, "en-boy oranı korunmalı");
    }

    #[test]
    fn sifir_olcu_guvenli() {
        assert_eq!(kucultme_gerekli(0, 100, 10, 10), None);
        assert_eq!(kucultme_gerekli(100, 100, 0, 0), None);
    }

    #[test]
    fn olcekleme_boyutu_dogru() {
        let kaynak = duz_tampon(64, 64, [200, 100, 50, 255]);
        let sonuc = olcekle_rgba8(&kaynak, 64, 64, 16, 16, FilterType::CatmullRom)
            .expect("ölçeklenmeli");
        assert_eq!(sonuc.len(), 16 * 16 * 4);
        // Düz renk korunmalı (kenar yumuşatma renk bozmaz).
        for p in sonuc.chunks_exact(4) {
            assert!(p[0].abs_diff(200) <= 2, "kırmızı kanal korunmalı: {p:?}");
            assert!(p[1].abs_diff(100) <= 2, "yeşil kanal korunmalı: {p:?}");
        }
    }

    #[test]
    fn eksik_tampon_hata_dondurur() {
        let kaynak = vec![0u8; 10];
        assert!(olcekle_rgba8(&kaynak, 64, 64, 8, 8, FilterType::Bilinear).is_err());
    }

    #[test]
    fn alfa_kanali_korunur() {
        let kaynak = duz_tampon(32, 32, [10, 20, 30, 128]);
        let sonuc = olcekle_rgba8(&kaynak, 32, 32, 8, 8, FilterType::Lanczos3).expect("ölçeklenmeli");
        for p in sonuc.chunks_exact(4) {
            assert!(p[3].abs_diff(128) <= 2, "alfa korunmalı: {p:?}");
        }
    }

    #[test]
    fn f32_olcekleme_calisir() {
        let kaynak: Vec<f32> = (0..32 * 32 * 4).map(|i| (i % 7) as f32 / 7.0).collect();
        let sonuc = olcekle_rgba32f(&kaynak, 32, 32, 8, 8, FilterType::CatmullRom).expect("ölçeklenmeli");
        assert_eq!(sonuc.len(), 8 * 8 * 4);
    }

    #[test]
    fn filtre_secimi_eslesir() {
        use super::super::super::cekirdek::ayar::Filtre;
        assert_eq!(filtre_secimi(Filtre::CatmullRom), FilterType::CatmullRom);
        assert_eq!(filtre_secimi(Filtre::Bilinear), FilterType::Bilinear);
        assert_eq!(filtre_secimi(Filtre::Lanczos3), FilterType::Lanczos3);
    }
}
