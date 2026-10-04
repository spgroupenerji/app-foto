//! Windows 11 sistem arka plan malzemesi (Mica / Mica Alt / Akrilik) ve karanlık tema.
//!
//! Mica, DWM'nin pencere arkasına masaüstü duvar kağıdını bulanıklaştırarak uyguladığı
//! sistem malzemesidir. Etkin olması için istemci alanının pencere çerçevesine
//! genişletilmesi (`DwmExtendFrameIntoClientArea`) gerekir. Desteklemeyen Windows
//! sürümlerinde çağrılar başarısız olur ve uygulama düz arka planla çalışmaya devam eder.

use winit::window::Window;

/// Sistem arka plan malzemesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Malzeme {
    /// Düz (sistem malzemesi yok).
    #[default]
    Yok,
    /// Windows 11 ana pencere malzemesi.
    Mica,
    /// Sekmeli pencere malzemesi (daha belirgin). Ayarlardan seçilebilir hale
    /// getirilmesi Faz 2 kapsamındadır; Win32 karşılığı burada tanımlıdır.
    #[allow(dead_code)]
    MicaAlt,
    /// Bulanık akrilik (eski adıyla "transient").
    #[allow(dead_code)]
    Akrilik,
}

impl Malzeme {
    /// Win32 `DWMSBT_*` değeri.
    pub fn dwm_degeri(self) -> i32 {
        match self {
            // DWMSBT_NONE
            Self::Yok => 1,
            // DWMSBT_MAINWINDOW
            Self::Mica => 2,
            // DWMSBT_TABBEDWINDOW
            Self::MicaAlt => 4,
            // DWMSBT_TRANSIENTWINDOW
            Self::Akrilik => 3,
        }
    }

    // Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
    #[allow(dead_code)]
    /// Kullanıcıya gösterilecek ad.
    pub fn etiket(self) -> &'static str {
        match self {
            Self::Yok => "Düz",
            Self::Mica => "Mica",
            Self::MicaAlt => "Mica Alt",
            Self::Akrilik => "Akrilik",
        }
    }

    /// Ayarlardan malzeme seçimini çevirir.
    pub fn ayardan(mica_etkin: bool) -> Self {
        if mica_etkin {
            Self::Mica
        } else {
            Self::Yok
        }
    }
}

/// Malzemeyi ve karanlık tema tercihini pencereye uygular.
///
/// Dönüş değeri malzemenin gerçekten uygulanıp uygulanmadığını bildirir; `false`
/// döndüğünde çağıran taraf düz arka plana düşmelidir (uygulama hata vermez).
pub fn uygula(pencere: &Window, malzeme: Malzeme, karanlik: bool) -> bool {
    #[cfg(windows)]
    {
        win32::uygula(pencere, malzeme, karanlik)
    }
    #[cfg(not(windows))]
    {
        let _ = (pencere, malzeme, karanlik);
        false
    }
}

#[cfg(windows)]
mod win32 {
    use super::Malzeme;
    use crate::pencere::tanitici;
    use winit::window::Window;
    use windows::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use windows::Win32::UI::Controls::MARGINS;

    /// Malzemeyi pencereye uygular; herhangi bir adım başarısızsa `false` döner.
    pub fn uygula(pencere: &Window, malzeme: Malzeme, karanlik: bool) -> bool {
        let Ok(tanitici) = tanitici(pencere) else {
            return false;
        };

        // İstemci alanını çerçeveye genişlet: malzemenin görünmesi için ön koşul.
        let kenar = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let genisletme = unsafe { DwmExtendFrameIntoClientArea(tanitici, &kenar) };
        if genisletme.is_err() {
            return false;
        }

        let deger = malzeme.dwm_degeri();
        let ayar = unsafe {
            DwmSetWindowAttribute(
                tanitici,
                DWMWA_SYSTEMBACKDROP_TYPE,
                std::ptr::from_ref(&deger).cast(),
                std::mem::size_of::<i32>() as u32,
            )
        };
        if ayar.is_err() {
            return false;
        }

        // Karanlık başlık çubuğu: uygulama temasıyla uyum için.
        let karanlik_degeri = i32::from(karanlik);
        let _ = unsafe {
            DwmSetWindowAttribute(
                tanitici,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                std::ptr::from_ref(&karanlik_degeri).cast(),
                std::mem::size_of::<i32>() as u32,
            )
        };

        malzeme != Malzeme::Yok
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn malzeme_dwm_degerleri_dogru() {
        assert_eq!(Malzeme::Mica.dwm_degeri(), 2);
        assert_eq!(Malzeme::Akrilik.dwm_degeri(), 3);
        assert_eq!(Malzeme::MicaAlt.dwm_degeri(), 4);
        assert_eq!(Malzeme::Yok.dwm_degeri(), 1);
    }

    #[test]
    fn ayar_eslemesi() {
        assert_eq!(Malzeme::ayardan(true), Malzeme::Mica);
        assert_eq!(Malzeme::ayardan(false), Malzeme::Yok);
    }

    #[test]
    fn etiketler_turkce() {
        for m in [Malzeme::Yok, Malzeme::Mica, Malzeme::MicaAlt, Malzeme::Akrilik] {
            assert!(!m.etiket().is_empty());
        }
    }
}
