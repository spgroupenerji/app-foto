//! Renk yönetimi: ICC profillerini doğrusal (linear) scRGB çalışma uzayına haritalar.
//!
//! İki yol vardır:
//! 1. **Hızlı yol (yaygın durum):** Profil yoksa veya profil sRGB ise, 8-bit girdi
//!    4096 girdili bir arama tablosuyla doğrusal f32'ye çevrilir (kesin ve hızlı).
//! 2. **ICC yolu:** Gömülü profil sRGB dışındaysa (AdobeRGB, DCI-P3, ProPhoto) lcms2
//!    motoru ile renk dönüşümü yapılır, ardından doğrusallastırma uygulanır.

use std::sync::OnceLock;

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};

/// Arama tablosu çözünürlüğü (2^12): sRGB eğrisinin doğrusal bölgesini de kapsar.
const LUT_BOYU: usize = 4096;

/// Kanallar arası ölçek: LUT indeksine çevrim için.
const KANAL_ARALIGI: f32 = 255.0;

static SRGB_LUT: OnceLock<Vec<f32>> = OnceLock::new();

/// sRGB (gama kodlu) 8-bit değeri doğrusal değere çevirir.
///
/// IEC 61966-2-1 transfer fonksiyonu; 0,04045 eşiğinin altında doğrusal bölge geçerlidir.
pub fn srgb_bayt_doğrusal(deger: u8) -> f32 {
    srgb_oran_doğrusal(f32::from(deger) / KANAL_ARALIGI)
}

/// sRGB oranını (0.0-1.0) doğrusal orana çevirir.
pub fn srgb_oran_doğrusal(deger: f32) -> f32 {
    let v = deger.clamp(0.0, 1.0);
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
/// Doğrusal oranı sRGB oranına çevirir (sRGB kodlama yolu için).
pub fn doğrusal_oran_srgb(deger: f32) -> f32 {
    let v = deger.max(0.0);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// 8-bit için arama tablosu (ilk erişimde hesaplanır).
fn lut() -> &'static [f32] {
    SRGB_LUT.get_or_init(|| {
        (0..=LUT_BOYU)
            .map(|i| {
                let oran = i as f32 / LUT_BOYU as f32;
                srgb_oran_doğrusal(oran)
            })
            .collect()
    })
}

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
/// Arama tablosundan 16-bit kanal değerini doğrusal orana çevirir.
pub fn onalti_bit_doğrusal(deger: u16) -> f32 {
    let tablo = lut();
    let indeks = (usize::from(deger) * LUT_BOYU) / u16::MAX as usize;
    tablo[indeks.min(LUT_BOYU)]
}

/// 8-bit kanal değerini arama tablosu indeksine çevirir.
///
/// Tablo 12-bit çözünürlüklü olduğu için 8-bit değer ölçeklenir; yuvarlama eklenerek
/// tablo aralığının tamamı kullanılır (255 → son girdi).
fn lut_indisi(deger: u8) -> usize {
    (usize::from(deger) * LUT_BOYU + 127) / 255
}

/// RGBA8 tamponu doğrusal f32 RGBA tamponuna çevirir (arama tablosuyla).
pub fn dogrusallastir_rgba8(rgba: &[u8]) -> Vec<f32> {
    let tablo = lut();
    let mut cikti = Vec::with_capacity(rgba.len());
    for p in rgba.chunks_exact(4) {
        cikti.push(tablo[lut_indisi(p[0])]);
        cikti.push(tablo[lut_indisi(p[1])]);
        cikti.push(tablo[lut_indisi(p[2])]);
        cikti.push(f32::from(p[3]) / KANAL_ARALIGI);
    }
    cikti
}

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
/// RGB16 tamponu (alfa yok) doğrusal f32 RGBA tamponuna çevirir.
pub fn dogrusallastir_rgb16(rgb: &[u16]) -> Vec<f32> {
    let mut cikti = Vec::with_capacity(rgb.len() / 3 * 4);
    for p in rgb.chunks_exact(3) {
        cikti.push(onalti_bit_doğrusal(p[0]));
        cikti.push(onalti_bit_doğrusal(p[1]));
        cikti.push(onalti_bit_doğrusal(p[2]));
        cikti.push(1.0);
    }
    cikti
}

/// Gömülü ICC profilinin sRGB olup olmadığını sezgisel olarak belirler.
///
/// lcms2 çağrısı yapmadan hızlı yolun kullanılıp kullanılamayacağına karar verir.
pub fn profil_srgb_mi(icc: &[u8]) -> bool {
    if icc.len() < 128 {
        return false;
    }
    // ICC profil açıklaması 'desc' etiketinde ASCII olarak bulunur.
    let pencere = &icc[..icc.len().min(4096)];
    let metin: Vec<u8> = pencere
        .iter()
        .map(|b| if b.is_ascii_graphic() || *b == b' ' { *b } else { b' ' })
        .collect();
    let metin = String::from_utf8_lossy(&metin).to_ascii_lowercase();
    metin.contains("srgb") && !metin.contains("adobe")
}

/// ICC profili ile 8-bit RGBA verisini sRGB renk uzayına dönüştürür (lcms2).
///
/// Dönüşüm başarısız olursa hata döner; çağıran taraf sRGB varsayımıyla devam edebilir.
/// lcms2 alfa kanalını taşımadığı için alfa değerleri kaynaktan birebir kopyalanır;
/// aksi halde saydam olmayan görseller saydam hale gelirdi.
pub fn icc_ile_srgb(rgba: &[u8], icc: &[u8]) -> Sonuc<Vec<u8>> {
    use lcms2::{Intent, PixelFormat, Profile, Transform};

    let girdi_profili = Profile::new_icc(icc)
        .map_err(|k| GorselHatasi::RenkProfili(format!("profil açılamadı: {k}")))?;
    let hedef_profil = Profile::new_srgb();
    let donusum = Transform::new(
        &girdi_profili,
        PixelFormat::RGBA_8,
        &hedef_profil,
        PixelFormat::RGBA_8,
        Intent::Perceptual,
    )
    .map_err(|k| GorselHatasi::RenkProfili(format!("dönüşüm kurulamadı: {k}")))?;

    let mut cikti = vec![0u8; rgba.len()];
    donusum.transform_pixels(rgba, &mut cikti);
    alfa_kanalini_koru(rgba, &mut cikti);
    Ok(cikti)
}

/// Renk dönüşümü sonrası alfa kanalını kaynaktan geri yazar.
pub fn alfa_kanalini_koru(kaynak: &[u8], hedef: &mut [u8]) {
    let piksel = kaynak.len().min(hedef.len()) / 4;
    for i in 0..piksel {
        hedef[i * 4 + 3] = kaynak[i * 4 + 3];
    }
}

/// Kaynak veriye uygulanacak renk işlemini seçer ve doğrusal f32 tamponu üretir.
///
/// Akış: (gerekirse ICC dönüşümü) → doğrusallastırma.
pub fn dogrusal_tampon_uret(rgba8: &[u8], icc: Option<&[u8]>) -> (Vec<f32>, bool) {
    match icc {
        Some(profil) if !profil.is_empty() && !profil_srgb_mi(profil) => match icc_ile_srgb(rgba8, profil) {
            Ok(donusmus) => (dogrusallastir_rgba8(&donusmus), true),
            // Profil bozuksa kullanıcıyı bekletmeden sRGB varsayımına düşülür.
            Err(_) => (dogrusallastir_rgba8(rgba8), false),
        },
        _ => (dogrusallastir_rgba8(rgba8), false),
    }
}

/// f32 tamponu doğrusal kabul edip 4 kanallı hale getirir (HDR kaynaklar için).
pub fn hdr_tamponu_uret(rgb32f: &[f32], kanal: usize) -> Vec<f32> {
    match kanal {
        4 => rgb32f.to_vec(),
        3 => {
            let mut cikti = Vec::with_capacity(rgb32f.len() / 3 * 4);
            for p in rgb32f.chunks_exact(3) {
                cikti.extend_from_slice(&[p[0], p[1], p[2], 1.0]);
            }
            cikti
        }
        1 => {
            let mut cikti = Vec::with_capacity(rgb32f.len() * 4);
            for v in rgb32f {
                cikti.extend_from_slice(&[*v, *v, *v, 1.0]);
            }
            cikti
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn srgb_uc_noktalari_dogru() {
        assert!((srgb_bayt_doğrusal(0) - 0.0).abs() < 1e-6);
        assert!((srgb_bayt_doğrusal(255) - 1.0).abs() < 1e-6);
        // Bilinen referans: 128 → ~0,2158
        assert!((srgb_bayt_doğrusal(128) - 0.215_86).abs() < 1e-3);
        // 10 → doğrusal bölge (10/255/12.92)
        assert!((srgb_bayt_doğrusal(10) - 0.003_035).abs() < 1e-5);
    }

    #[test]
    fn gidis_donus_kapali() {
        for d in [0u8, 1, 64, 128, 200, 254, 255] {
            let d = f32::from(d) / 255.0;
            let geri = doğrusal_oran_srgb(srgb_oran_doğrusal(d));
            assert!((geri - d).abs() < 1e-4, "d={d} geri={geri}");
        }
    }

    #[test]
    fn lut_ile_hesaplama_ayni_sonucu_verir() {
        for d in [0u8, 5, 37, 128, 255] {
            let lut_degeri = onalti_bit_doğrusal(u16::from(d) * 257);
            let hesap = srgb_bayt_doğrusal(d);
            assert!(
                (lut_degeri - hesap).abs() < 2e-4,
                "d={d}: lut={lut_degeri} hesap={hesap}"
            );
        }
    }

    #[test]
    fn sekiz_bit_tum_aralik_dogru_eslenir() {
        // Sınırlar ve monotonluk: tablo indekslemesi tüm 0-255 aralığını kapsamalı.
        let tam = dogrusallastir_rgba8(&[0, 0, 0, 255]);
        assert!((tam[0] - 0.0).abs() < 1e-6, "siyah sıfır olmalı: {}", tam[0]);
        let beyaz = dogrusallastir_rgba8(&[255, 255, 255, 255]);
        assert!((beyaz[0] - 1.0).abs() < 1e-4, "beyaz bir olmalı: {}", beyaz[0]);

        let mut onceki = -1.0f32;
        for d in 0..=255u8 {
            let v = dogrusallastir_rgba8(&[d, 0, 0, 255])[0];
            let beklenen = srgb_bayt_doğrusal(d);
            assert!(
                (v - beklenen).abs() < 1e-3,
                "d={d}: lut={v} beklenen={beklenen}"
            );
            assert!(v >= onceki, "d={d}: monotonluk bozuldu");
            onceki = v;
        }
    }

    #[test]
    fn rgba8_dort_kanal_uretir() {
        let girdi = vec![0u8, 128, 255, 200, 255, 0, 0, 255];
        let cikti = dogrusallastir_rgba8(&girdi);
        assert_eq!(cikti.len(), 8);
        assert!((cikti[0] - 0.0).abs() < 1e-6);
        assert!((cikti[2] - 1.0).abs() < 1e-6);
        assert!((cikti[3] - 200.0 / 255.0).abs() < 1e-6, "alfa doğrusallaştırılmaz");
    }

    #[test]
    fn alfa_kanali_gama_uygulanmaz() {
        let cikti = dogrusallastir_rgba8(&[0, 0, 0, 128]);
        assert!((cikti[3] - 0.501_96).abs() < 1e-3);
    }

    #[test]
    fn hdr_kanal_sayilari_dogru() {
        let rgb = [1.0f32, 2.0, 3.0];
        let dort = hdr_tamponu_uret(&rgb, 3);
        assert_eq!(dort, vec![1.0, 2.0, 3.0, 1.0]);
        let gri = hdr_tamponu_uret(&[0.5f32], 1);
        assert_eq!(gri, vec![0.5, 0.5, 0.5, 1.0]);
        // Bilinmeyen kanal sayısı boş döner (panik yok).
        assert!(hdr_tamponu_uret(&rgb, 2).is_empty());
    }

    #[test]
    fn srgb_profili_sezgisel_taninir() {
        let mut sahte = vec![0u8; 512];
        sahte[..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x00]);
        sahte[64..72].copy_from_slice(b"sRGB IEC");
        assert!(profil_srgb_mi(&sahte));

        let mut adobe = vec![0u8; 512];
        adobe[64..81].copy_from_slice(b"Adobe RGB (1998) ");
        assert!(!profil_srgb_mi(&adobe));
        assert!(!profil_srgb_mi(&[0u8; 10]), "çok kısa profil sRGB sayılmaz");
    }

    #[test]
    fn profil_yoksa_hizli_yol_kullanilir() {
        let (tampon, icc_uygulandi) = dogrusal_tampon_uret(&[255, 255, 255, 255], None);
        assert!(!icc_uygulandi);
        assert_eq!(tampon.len(), 4);
        assert!((tampon[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn bozuk_icc_panik_yapmaz_varsayilana_duser() {
        let bozuk = vec![7u8; 300];
        let (tampon, icc_uygulandi) = dogrusal_tampon_uret(&[0, 0, 0, 255], Some(&bozuk));
        assert_eq!(tampon.len(), 4);
        let _ = icc_uygulandi;
    }

    #[test]
    fn rgb16_cevrimi_alfa_ekler() {
        let rgb = [0u16, 32768, 65535];
        let c = dogrusallastir_rgb16(&rgb);
        assert_eq!(c.len(), 4);
        assert!((c[0] - 0.0).abs() < 1e-6);
        assert!((c[3] - 1.0).abs() < 1e-6);
    }
}
