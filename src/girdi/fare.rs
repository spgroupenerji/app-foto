//! Fare haritası: bölge duyarlı tıklama, imleç odaklı tekerlek yakınlaştırma.

use winit::event::MouseScrollDelta;

use super::Eylem;
use super::bolge::{Bolge, Pivot};

/// Piksel cinsinden tekerlek hareketinin "satır" karşılığı (yüksek çözünürlüklü tekerlekler
/// küçük piksel adımları üretir; bunlar satır birimine çevrilir).
const PIKSEL_SATIR_BOLEN: f64 = 100.0;

/// Tekerlek miktarını zoom çarpanına çevirir (pozitif = yakınlaştır).
///
/// Miktar sınırlandırılır: tek bir olayın aşırı miktarda zoom yapması engellenir.
pub fn tekerlek_carpani(miktar: f64, adim: f32) -> f64 {
    if miktar == 0.0 || !miktar.is_finite() {
        return 1.0;
    }
    let taban = f64::from(adim.max(1.001));
    let sinirli = miktar.abs().min(EN_COK_TEKERLEK_ADIMI);
    if miktar > 0.0 {
        taban.powf(sinirli)
    } else {
        1.0 / taban.powf(sinirli)
    }
}

/// Tek bir tekerlek olayında uygulanabilecek en fazla adım.
pub const EN_COK_TEKERLEK_ADIMI: f64 = 10.0;

/// Tekerlek olayını eyleme çevirir.
///
/// Davranış sözleşmesi:
/// - `Ctrl` basılıysa: imlecin altındaki nokta sabit kalacak biçimde yakınlaştırma.
/// - `Ctrl` basılı değilse: tekerlek yukarı = önceki görsel, aşağı = sonraki görsel.
pub fn tekerlek_eylemi(delta: MouseScrollDelta, ctrl: bool, pivot: Pivot, adim: f32) -> Eylem {
    let miktar = match delta {
        MouseScrollDelta::LineDelta(_, y) => f64::from(y),
        MouseScrollDelta::PixelDelta(konum) => konum.y / PIKSEL_SATIR_BOLEN,
    };
    if miktar == 0.0 {
        return Eylem::Yok;
    }
    if ctrl {
        Eylem::Yakinlastir {
            carpan: tekerlek_carpani(miktar, adim),
            pivot: Some(pivot),
        }
    } else if miktar > 0.0 {
        Eylem::Onceki
    } else {
        Eylem::Sonraki
    }
}

/// Sol tık: bölgeye göre gezinme veya kaydırma başlangıcı.
///
/// Sol üçte bir → önceki, sağ üçte bir → sonraki, orta üçte bir → kaydırma.
pub fn sol_tik_eylemi(bolge: Bolge) -> Eylem {
    match bolge {
        Bolge::Sol => Eylem::Onceki,
        Bolge::Sag => Eylem::Sonraki,
        Bolge::Orta => Eylem::KaydirmaBaslat,
    }
}

/// Sağ tık: kullanıcı ayarına göre önceki görsele döner ya da bağlam menüsü açılır.
pub fn sag_tik_eylemi(sag_tik_gezinme: bool, x: f64, y: f64) -> Eylem {
    if sag_tik_gezinme {
        Eylem::Onceki
    } else {
        Eylem::BaglamMenusu { x, y }
    }
}

/// Çift tıklama: ayar açıksa tam ekran geçişi.
pub fn cift_tik_eylemi(cift_tik_tam_ekran: bool) -> Eylem {
    if cift_tik_tam_ekran {
        Eylem::TamEkranDegistir
    } else {
        Eylem::Yok
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use winit::dpi::PhysicalPosition;

    fn pivot() -> Pivot {
        Pivot {
            ekran_x: 100.0,
            ekran_y: 50.0,
        }
    }

    #[test]
    fn tekerlek_yukari_onceki_asagi_sonraki() {
        let yukari = MouseScrollDelta::LineDelta(0.0, 1.0);
        let asagi = MouseScrollDelta::LineDelta(0.0, -1.0);
        assert_eq!(tekerlek_eylemi(yukari, false, pivot(), 1.15), Eylem::Onceki);
        assert_eq!(tekerlek_eylemi(asagi, false, pivot(), 1.15), Eylem::Sonraki);
    }

    #[test]
    fn ctrl_tekerlek_imlec_odakli_zoom_uretir() {
        let delta = MouseScrollDelta::LineDelta(0.0, 1.0);
        match tekerlek_eylemi(delta, true, pivot(), 1.15) {
            Eylem::Yakinlastir { carpan, pivot: Some(p) } => {
                assert!(carpan > 1.0, "yukarı tekerlek yakınlaştırır: {carpan}");
                assert_eq!(p, pivot());
            }
            diger => panic!("zoom bekleniyordu: {diger:?}"),
        }
        let asagi = MouseScrollDelta::LineDelta(0.0, -1.0);
        match tekerlek_eylemi(asagi, true, pivot(), 1.15) {
            Eylem::Yakinlastir { carpan, .. } => assert!(carpan < 1.0),
            diger => panic!("uzaklaştırma bekleniyordu: {diger:?}"),
        }
    }

    #[test]
    fn piksel_deltasi_satira_cevrilir() {
        // 100 piksel = 1 satır → yakınlaştırma çarpanı adım kadar olmalı.
        let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 100.0));
        // Adım f32 olduğu için beklenen değer de aynı dönüşümle hesaplanır.
        match tekerlek_eylemi(delta, true, pivot(), 1.15) {
            Eylem::Yakinlastir { carpan, .. } => {
                assert!((carpan - f64::from(1.15f32)).abs() < 1e-9, "gelen {carpan}");
            }
            diger => panic!("zoom bekleniyordu: {diger:?}"),
        }
    }

    #[test]
    fn sifir_hareket_eylem_uretmez() {
        let delta = MouseScrollDelta::LineDelta(0.0, 0.0);
        assert_eq!(tekerlek_eylemi(delta, true, pivot(), 1.15), Eylem::Yok);
        assert_eq!(tekerlek_eylemi(delta, false, pivot(), 1.15), Eylem::Yok);
    }

    #[test]
    fn asiri_tekerlek_miktari_sinirlanir() {
        let cok = tekerlek_carpani(500.0, 1.15);
        let sinir = tekerlek_carpani(EN_COK_TEKERLEK_ADIMI, 1.15);
        assert!((cok - sinir).abs() < 1e-9, "tek olayda aşırı zoom engellenmeli");
        assert!(cok.is_finite());
    }

    #[test]
    fn gecersiz_miktar_notr_dondurur() {
        assert_eq!(tekerlek_carpani(f64::NAN, 1.15), 1.0);
        assert_eq!(tekerlek_carpani(0.0, 1.15), 1.0);
        assert_eq!(tekerlek_carpani(1.0, 0.0), {
            // Adım 1'den küçükse en az 1,001 tabanına yükseltilir.
            f64::from(1.001f32)
        });
    }

    #[test]
    fn sol_tik_bolgeye_gore_davranir() {
        assert_eq!(sol_tik_eylemi(Bolge::Sol), Eylem::Onceki);
        assert_eq!(sol_tik_eylemi(Bolge::Sag), Eylem::Sonraki);
        assert_eq!(sol_tik_eylemi(Bolge::Orta), Eylem::KaydirmaBaslat);
    }

    #[test]
    fn sag_tik_ayara_gore_ayrisir() {
        assert_eq!(sag_tik_eylemi(true, 0.0, 0.0), Eylem::Onceki);
        match sag_tik_eylemi(false, 320.0, 240.0) {
            Eylem::BaglamMenusu { x, y } => {
                assert_eq!(x, 320.0);
                assert_eq!(y, 240.0);
            }
            diger => panic!("bağlam menüsü bekleniyordu: {diger:?}"),
        }
    }

    #[test]
    fn cift_tik_ayara_bagli() {
        assert_eq!(cift_tik_eylemi(true), Eylem::TamEkranDegistir);
        assert_eq!(cift_tik_eylemi(false), Eylem::Yok);
    }
}
