//! Windows kabuk "Birlikte Aç" ve varsayılan uygulama entegrasyonu.

use std::path::Path;

use crate::cekirdek::hata::{GorselHatasi, Sonuc};

/// Aktif görseli Windows "Birlikte aç" iletişim kutusuyla harici bir uygulamada açar.
pub fn birlikte_ac(yol: &Path) -> Sonuc<()> {
    if !yol.exists() {
        return Err(GorselHatasi::Bulunamadi(yol.to_path_buf()));
    }

    #[cfg(windows)]
    {
        win32::birlikte_ac(yol)
    }

    #[cfg(not(windows))]
    {
        let _ = yol;
        Err(GorselHatasi::Kabuk(
            "Birlikte aç yalnızca Windows üzerinde desteklenir".into(),
        ))
    }
}

/// Menü olmadan, kayıtlı varsayılan uygulamayla açar (ShellExecuteW "open").
// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
pub fn varsayilan_ile_ac(yol: &Path) -> Sonuc<()> {
    if !yol.exists() {
        return Err(GorselHatasi::Bulunamadi(yol.to_path_buf()));
    }

    #[cfg(windows)]
    {
        win32::varsayilan_ile_ac(yol)
    }

    #[cfg(not(windows))]
    {
        let _ = yol;
        Err(GorselHatasi::Kabuk(
            "Varsayılan ile aç yalnızca Windows üzerinde desteklenir".into(),
        ))
    }
}

#[cfg(windows)]
mod win32 {
    use std::path::Path;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        OPENASINFO, OPEN_AS_INFO_FLAGS, SHOpenWithDialog, ShellExecuteW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    use crate::cekirdek::hata::{GorselHatasi, Sonuc};
    use crate::kabuk::yol_genis_dizi;

    const OAIF_EXEC: OPEN_AS_INFO_FLAGS = OPEN_AS_INFO_FLAGS(4);
    const SHELLEXECUTE_HATA_SINIRI: usize = 32;

    pub fn birlikte_ac(yol: &Path) -> Sonuc<()> {
        let genis_yol = yol_genis_dizi(yol);
        let bilgi = OPENASINFO {
            pcszFile: PCWSTR::from_raw(genis_yol.as_ptr()),
            pcszClass: PCWSTR::null(),
            oaifInFlags: OAIF_EXEC,
        };

        // ponytail: SHOpenWithDialog bloklayan standart Windows kabuk iletişim kutusudur
        unsafe { SHOpenWithDialog(None, &bilgi) }
            .map_err(|e| GorselHatasi::Kabuk(format!("Birlikte aç iletişim kutusu hatası: {e}")))
    }

    pub fn varsayilan_ile_ac(yol: &Path) -> Sonuc<()> {
        let islem = super::super::genis_dizi("open");
        let genis_yol = yol_genis_dizi(yol);

        let sonuc = unsafe {
            ShellExecuteW(
                None,
                PCWSTR::from_raw(islem.as_ptr()),
                PCWSTR::from_raw(genis_yol.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };

        let kod = sonuc.0 as usize;
        if kod <= SHELLEXECUTE_HATA_SINIRI {
            return Err(GorselHatasi::Kabuk(format!(
                "Varsayılan uygulama açılamadı: ShellExecuteW hata kodu {kod}"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn olmayan_dosyada_birlikte_ac_bulunamadi_doner() {
        let yol = PathBuf::from(r"C:\kesinlikle\var\olmayan\bir_gorsel_12345.png");
        match birlikte_ac(&yol) {
            Err(GorselHatasi::Bulunamadi(p)) => assert_eq!(p, yol),
            diger => panic!("Bulunamadi bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn olmayan_dosyada_varsayilan_ile_ac_bulunamadi_doner() {
        let yol = PathBuf::from(r"C:\kesinlikle\var\olmayan\bir_gorsel_12345.png");
        match varsayilan_ile_ac(&yol) {
            Err(GorselHatasi::Bulunamadi(p)) => assert_eq!(p, yol),
            diger => panic!("Bulunamadi bekleniyordu, gelen: {diger:?}"),
        }
    }
}
