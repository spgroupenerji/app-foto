//! Klavye tuş haritası (Windows görsel görüntüleyici alışkanlıklarına göre).

use winit::keyboard::KeyCode;

use super::Eylem;

/// Klavye ile yakınlaştırma adım çarpanı.
pub const TUS_ZOOM_CARPI: f64 = 1.15;

/// Tuşu eyleme çevirir.
///
/// `ctrl` basılıyken yakınlaştırma kısayolları, `shift` basılıyken ilk/son atlama etkindir.
pub fn tus_eylemi(kod: KeyCode, ctrl: bool, shift: bool) -> Eylem {
    if ctrl {
        match kod {
            KeyCode::Equal | KeyCode::NumpadAdd => {
                return Eylem::Yakinlastir {
                    carpan: TUS_ZOOM_CARPI,
                    pivot: None,
                };
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                return Eylem::Yakinlastir {
                    carpan: 1.0 / TUS_ZOOM_CARPI,
                    pivot: None,
                };
            }
            KeyCode::Digit0 => return Eylem::GercekBoyut,
            // Ctrl+O klasik "aç", Ctrl+Shift+O klasör aç, Ctrl+E harici uygulamada aç.
            KeyCode::KeyO => {
                return if shift {
                    Eylem::KlasorAc
                } else {
                    Eylem::DosyaAc
                };
            }
            KeyCode::KeyE => return Eylem::BirlikteAc,
            KeyCode::KeyK | KeyCode::Comma => return Eylem::AyarPenceresi,
            KeyCode::KeyW | KeyCode::KeyQ => return Eylem::Kapat,
            _ => {}
        }
    }

    match kod {
        // Gezinme: sol/sağ ok ve sayfa tuşları kesintisiz geçiş sağlar.
        KeyCode::ArrowLeft | KeyCode::PageUp | KeyCode::Backspace => Eylem::Onceki,
        KeyCode::ArrowRight | KeyCode::PageDown | KeyCode::Space => Eylem::Sonraki,
        KeyCode::Home => Eylem::Ilk,
        KeyCode::End => Eylem::Son,
        // Görünüm.
        KeyCode::KeyS | KeyCode::Digit1 => Eylem::Sigdir,
        KeyCode::KeyG | KeyCode::Digit0 => Eylem::GercekBoyut,
        KeyCode::ArrowUp => Eylem::Yakinlastir {
            carpan: TUS_ZOOM_CARPI,
            pivot: None,
        },
        KeyCode::ArrowDown => Eylem::Yakinlastir {
            carpan: 1.0 / TUS_ZOOM_CARPI,
            pivot: None,
        },
        // Pencere ve kabuk.
        KeyCode::KeyF | KeyCode::F11 => Eylem::TamEkranDegistir,
        KeyCode::Escape => Eylem::TamEkranDegistir,
        KeyCode::KeyR => Eylem::YenidenYukle,
        KeyCode::KeyL => Eylem::DosyaListesiDegistir,
        // Kaydırma: Shift ile yatay, normalde dikey onarım yapılmaz.
        _ if shift => Eylem::Yok,
        _ => Eylem::Yok,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ok_tuslari_gezinir() {
        assert_eq!(tus_eylemi(KeyCode::ArrowLeft, false, false), Eylem::Onceki);
        assert_eq!(
            tus_eylemi(KeyCode::ArrowRight, false, false),
            Eylem::Sonraki
        );
        assert_eq!(tus_eylemi(KeyCode::PageUp, false, false), Eylem::Onceki);
        assert_eq!(tus_eylemi(KeyCode::PageDown, false, false), Eylem::Sonraki);
        assert_eq!(tus_eylemi(KeyCode::Space, false, false), Eylem::Sonraki);
    }

    #[test]
    fn home_end_uclara_gider() {
        assert_eq!(tus_eylemi(KeyCode::Home, false, false), Eylem::Ilk);
        assert_eq!(tus_eylemi(KeyCode::End, false, false), Eylem::Son);
    }

    #[test]
    fn ctrl_ile_zoom_kisayollari() {
        match tus_eylemi(KeyCode::Equal, true, false) {
            Eylem::Yakinlastir { carpan, pivot } => {
                assert!(carpan > 1.0, "yakınlaştırmalı");
                assert!(pivot.is_none(), "klavye zoom'u pencere merkezinden");
            }
            diger => panic!("zoom bekleniyordu: {diger:?}"),
        }
        match tus_eylemi(KeyCode::Minus, true, false) {
            Eylem::Yakinlastir { carpan, .. } => assert!(carpan < 1.0),
            diger => panic!("uzaklaştırma bekleniyordu: {diger:?}"),
        }
    }

    #[test]
    fn ctrl_olmadan_zoom_tuslari_zoom_yapmaz() {
        // Ctrl'siz "+" eşlenmez, gezinmeye düşmez.
        assert_eq!(tus_eylemi(KeyCode::Equal, false, false), Eylem::Yok);
    }

    #[test]
    fn gorunum_ve_pencere_kisayollari() {
        assert_eq!(tus_eylemi(KeyCode::KeyS, false, false), Eylem::Sigdir);
        assert_eq!(tus_eylemi(KeyCode::KeyG, false, false), Eylem::GercekBoyut);
        assert_eq!(
            tus_eylemi(KeyCode::F11, false, false),
            Eylem::TamEkranDegistir
        );
        assert_eq!(
            tus_eylemi(KeyCode::Escape, false, false),
            Eylem::TamEkranDegistir
        );
        assert_eq!(tus_eylemi(KeyCode::KeyR, false, false), Eylem::YenidenYukle);
        assert_eq!(
            tus_eylemi(KeyCode::KeyL, false, false),
            Eylem::DosyaListesiDegistir
        );
    }

    #[test]
    fn ctrl_kisayollari_ayrisir() {
        assert_eq!(tus_eylemi(KeyCode::KeyO, true, false), Eylem::DosyaAc);
        assert_eq!(tus_eylemi(KeyCode::KeyO, true, true), Eylem::KlasorAc);
        assert_eq!(tus_eylemi(KeyCode::KeyE, true, false), Eylem::BirlikteAc);
        assert_eq!(tus_eylemi(KeyCode::KeyO, false, false), Eylem::Yok);
        assert_eq!(tus_eylemi(KeyCode::KeyK, true, false), Eylem::AyarPenceresi);
        assert_eq!(tus_eylemi(KeyCode::KeyW, true, false), Eylem::Kapat);
        assert_eq!(tus_eylemi(KeyCode::KeyQ, true, false), Eylem::Kapat);
        assert_eq!(tus_eylemi(KeyCode::Digit0, true, false), Eylem::GercekBoyut);
    }

    #[test]
    fn bilinmeyen_tus_yok_dondurur() {
        assert_eq!(tus_eylemi(KeyCode::KeyZ, false, false), Eylem::Yok);
        assert_eq!(tus_eylemi(KeyCode::F5, false, false), Eylem::Yok);
    }
}
