//! SVG çözümleme: `resvg` + `tiny-skia` ile vektörel rasterleştirme.
//!
//! Vektörel içerik sabit bir piksel dizisi olarak çözülemez. Bu yüzden SVG, istenen
//! görünüm ölçüsünde (tipik olarak pencere çözünürlüğü) rasterleştirilir; böylece
//! yakınlaştırma sırasında detay kaybı oluşmaz.

use std::sync::OnceLock;

use resvg::{tiny_skia, usvg};

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};
use super::cozucu::{HamGoruntu, Pikseller};

/// Rasterleştirme üst sınırı: aşırı büyük SVG'lerde bellek taşmasını engeller.
pub const EN_COK_OLCU: u32 = 16_384;

static SECENEKLER: OnceLock<usvg::Options<'static>> = OnceLock::new();

/// SVG ayrıştırma seçenekleri; sistem yazı tipleri bir kez yüklenir.
fn secenekler() -> &'static usvg::Options<'static> {
    SECENEKLER.get_or_init(|| {
        let mut secenek = usvg::Options::default();
        // Metin içeren SVG'lerin doğru rasterleşmesi için sistem yazı tipleri gerekir.
        secenek.fontdb_mut().load_system_fonts();
        secenek
    })
}

/// SVG baytlarını istenen ölçüde rasterleştirir.
///
/// `hedef` verilmezse SVG'nin kendi bildirdiği ölçü (ceiling) kullanılır.
pub fn coz(baytlar: &[u8], hedef: Option<(u32, u32)>) -> Sonuc<HamGoruntu> {
    let agac = usvg::Tree::from_data(baytlar, secenekler())
        .map_err(|k| GorselHatasi::BozukVeri(format!("SVG çözümlenemedi: {k}")))?;

    let boyut = agac.size();
    let dogal_w = boyut.width().max(1.0);
    let dogal_h = boyut.height().max(1.0);

    // Hedef ölçü, SVG'nin en-boy oranı korunarak kutunun içine sığdırılır; böylece
    // çıktı tamponu gevşek (letterbox) kalmaz ve CPU ön küçültme eşiği doğru çalışır.
    let (hedef_w, hedef_h) = match hedef {
        Some((kutu_w, kutu_h)) if kutu_w > 0 && kutu_h > 0 => {
            let olcek = (kutu_w as f32 / dogal_w).min(kutu_h as f32 / dogal_h);
            (
                ((dogal_w * olcek).round() as u32).clamp(1, EN_COK_OLCU),
                ((dogal_h * olcek).round() as u32).clamp(1, EN_COK_OLCU),
            )
        }
        _ => (
            (dogal_w.ceil() as u32).clamp(1, EN_COK_OLCU),
            (dogal_h.ceil() as u32).clamp(1, EN_COK_OLCU),
        ),
    };

    // En-boy oranı korunur: tek bir ölçek katsayısı iki eksende de kullanılır.
    let olcek = (hedef_w as f32 / dogal_w).min(hedef_h as f32 / dogal_h);
    let mut pixmap = tiny_skia::Pixmap::new(hedef_w, hedef_h).ok_or_else(|| {
        GorselHatasi::BellekYetersiz {
            genislik: hedef_w,
            yukseklik: hedef_h,
            bayt: u64::from(hedef_w) * u64::from(hedef_h) * 4,
        }
    })?;

    resvg::render(
        &agac,
        tiny_skia::Transform::from_scale(olcek, olcek),
        &mut pixmap.as_mut(),
    );

    // tiny-skia çıktısı ön çarpımlı (premultiplied) alfadır; RGBA'ya çevrilir.
    let mut rgba = pixmap.take();
    on_carpansizlastir(&mut rgba);

    Ok(HamGoruntu {
        genislik: hedef_w,
        yukseklik: hedef_h,
        veri: Pikseller::Rgba8(rgba),
        icc: None,
        yonelim: 1,
    })
}

/// Ön çarpımlı alfayı düz (straight) alfaya çevirir.
pub fn on_carpansizlastir(rgba: &mut [u8]) {
    for p in rgba.chunks_exact_mut(4) {
        let alfa = p[3];
        if alfa == 0 || alfa == 255 {
            continue;
        }
        let a = u32::from(alfa);
        for kanal in 0..3 {
            let deger = (u32::from(p[kanal]) * 255 + a / 2) / a;
            p[kanal] = deger.min(255) as u8;
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const BASIT_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20">
        <rect width="40" height="20" fill="#ff0000"/></svg>"##;

    #[test]
    fn svg_dogal_olcude_rasterlesir() {
        let g = coz(BASIT_SVG, None).expect("rasterleşmeli");
        assert_eq!((g.genislik, g.yukseklik), (40, 20));
        let Pikseller::Rgba8(v) = &g.veri else {
            panic!("SVG RGBA8 üretmeli");
        };
        assert_eq!(v.len(), 40 * 20 * 4);
        // Kırmızı dolgu: ilk piksel kırmızı ve opak olmalı.
        assert!(v[0] > 200, "kırmızı kanal: {}", v[0]);
        assert!(v[1] < 40, "yeşil kanal: {}", v[1]);
        assert_eq!(v[3], 255, "opak olmalı");
    }

    #[test]
    fn hedef_olcu_uygulanir_ve_oran_korunur() {
        let g = coz(BASIT_SVG, Some((200, 200))).expect("rasterleşmeli");
        // 40x20 → 200x100 (oran korunur, tek ölçek katsayısı).
        assert_eq!((g.genislik, g.yukseklik), (200, 100));
    }

    #[test]
    fn sifir_hedef_dogal_olcuye_duser() {
        let g = coz(BASIT_SVG, Some((0, 0))).expect("rasterleşmeli");
        assert_eq!((g.genislik, g.yukseklik), (40, 20));
    }

    #[test]
    fn asiri_olcu_sinirlanir() {
        let g = coz(BASIT_SVG, Some((99_999, 99_999))).expect("rasterleşmeli");
        assert!(g.genislik <= EN_COK_OLCU);
        assert!(g.yukseklik <= EN_COK_OLCU);
    }

    #[test]
    fn bozuk_svg_hata_dondurur() {
        assert!(coz(b"<svg><bu bir svg degil", None).is_err());
        assert!(coz(b"", None).is_err());
    }

    #[test]
    fn alfa_on_carpimi_geri_alınır() {
        // %50 alfa ile yarı saydam beyaz: ön çarpımlı (128,128,128,128) → düz (255,255,255,128)
        let mut veri = vec![128u8, 128, 128, 128];
        on_carpansizlastir(&mut veri);
        assert_eq!(veri, vec![255, 255, 255, 128]);
        // Tam opak ve tam saydam pikseller değişmez.
        let mut opak = vec![10u8, 20, 30, 255];
        on_carpansizlastir(&mut opak);
        assert_eq!(opak, vec![10, 20, 30, 255]);
        let mut saydam = vec![0u8, 0, 0, 0];
        on_carpansizlastir(&mut saydam);
        assert_eq!(saydam, vec![0, 0, 0, 0]);
    }
}
