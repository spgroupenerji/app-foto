//! Windows dosya türü ilişkilendirme ve varsayılan uygulama kaydı (yalnızca HKCU).

use std::path::Path;

use crate::cekirdek::hata::Sonuc;

/// ProgID: HKCU\Software\Classes altında tanımlanır.
pub const PROGID: &str = "Gorsel.Goruntuleyici.v1";
pub const UYGULAMA_ADI: &str = "Gorsel Goruntuleyici";

/// İlişkilendirme durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durum {
    Kayitli,
    KayitliDegil,
    Kismi,
}

/// Desteklenen uzantılar listesi (noktasız, küçük harf).
const UZANTILAR: &[&str] = &[
    "jpg", "jpeg", "jpe", "jfif", "png", "apng", "webp", "gif", "bmp", "dib", "tga", "icb",
    "vda", "vst", "ico", "cur", "tif", "tiff", "avif", "heic", "heif", "jxl", "exr", "hdr",
    "pic", "svg", "svgz", "pnm", "pgm", "ppm", "pbm", "pam", "qoi", "dds", "ff", "cr2",
    "cr3", "nef", "arw", "dng", "orf", "rw2", "raf", "pef", "srw",
];

/// Desteklenen uzantılar (noktasız, küçük harf).
pub fn desteklenen_uzantilar() -> &'static [&'static str] {
    UZANTILAR
}

/// HKCU altına ProgID + Capabilities + RegisteredApplications yazar, kabuğu tazeler.
pub fn iliskilendirmeyi_kaydet(exe_yolu: &Path) -> Sonuc<()> {
    #[cfg(windows)]
    {
        win32::iliskilendirmeyi_kaydet(exe_yolu)
    }

    #[cfg(not(windows))]
    {
        let _ = exe_yolu;
        Err(GorselHatasi::Kabuk(
            "İlişkilendirme yalnızca Windows üzerinde desteklenir".into(),
        ))
    }
}

/// Yazılan kayıtları siler ve kabuğu tazeler.
pub fn iliskilendirmeyi_kaldir() -> Sonuc<()> {
    #[cfg(windows)]
    {
        win32::iliskilendirmeyi_kaldir()
    }

    #[cfg(not(windows))]
    {
        Err(GorselHatasi::Kabuk(
            "İlişkilendirme yalnızca Windows üzerinde desteklenir".into(),
        ))
    }
}

/// Kayıt defterini okuyarak mevcut durumu bildirir (yazma yapmaz).
pub fn durum() -> Durum {
    #[cfg(windows)]
    {
        win32::durum()
    }

    #[cfg(not(windows))]
    {
        Durum::KayitliDegil
    }
}

/// Windows'un varsayılan uygulamalar sayfasını açar (ms-settings:defaultapps).
pub fn varsayilan_uygulama_sayfasini_ac() -> Sonuc<()> {
    #[cfg(windows)]
    {
        win32::varsayilan_uygulama_sayfasini_ac()
    }

    #[cfg(not(windows))]
    {
        Err(GorselHatasi::Kabuk(
            "Varsayılan uygulamalar sayfası yalnızca Windows üzerinde açılabilir".into(),
        ))
    }
}

#[cfg(windows)]
mod win32 {
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHChangeNotify, ShellExecuteW, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    use super::{Durum, PROGID, UYGULAMA_ADI, UZANTILAR};
    use crate::cekirdek::hata::{GorselHatasi, Sonuc};
    use crate::kabuk::genis_dizi;

    pub fn iliskilendirmeyi_kaydet(exe_yolu: &Path) -> Sonuc<()> {
        let exe_str = exe_yolu.to_string_lossy();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 1. ProgID kaydı: HKCU\Software\Classes\<PROGID>
        let (progid_key, _) = hkcu
            .create_subkey(format!(r"Software\Classes\{PROGID}"))
            .map_err(|e| GorselHatasi::Kabuk(format!("ProgID anahtarı oluşturulamadı: {e}")))?;
        progid_key
            .set_value("", &UYGULAMA_ADI)
            .map_err(|e| GorselHatasi::Kabuk(format!("ProgID adı yazılamadı: {e}")))?;

        let simge_degeri = format!("\"{}\",0", exe_str);
        progid_key
            .set_value("DefaultIcon", &simge_degeri)
            .map_err(|e| GorselHatasi::Kabuk(format!("Simge yazılamadı: {e}")))?;

        let (komut_key, _) = progid_key
            .create_subkey(r"shell\open\command")
            .map_err(|e| GorselHatasi::Kabuk(format!("Açma komutu oluşturulamadı: {e}")))?;
        let komut_degeri = format!("\"{}\" \"%1\"", exe_str);
        komut_key
            .set_value("", &komut_degeri)
            .map_err(|e| GorselHatasi::Kabuk(format!("Komut yazılamadı: {e}")))?;

        let (tipler_key, _) = progid_key
            .create_subkey("SupportedTypes")
            .map_err(|e| GorselHatasi::Kabuk(format!("SupportedTypes oluşturulamadı: {e}")))?;
        for uzanti in UZANTILAR {
            let nokta_uzanti = format!(".{uzanti}");
            let _ = tipler_key.set_value(&nokta_uzanti, &"");
        }

        // 2. Yetenek kaydı: HKCU\Software\<UYGULAMA_ADI>\Capabilities
        let (yet_key, _) = hkcu
            .create_subkey(format!(r"Software\{UYGULAMA_ADI}\Capabilities"))
            .map_err(|e| GorselHatasi::Kabuk(format!("Capabilities oluşturulamadı: {e}")))?;
        let _ = yet_key.set_value("ApplicationDescription", &UYGULAMA_ADI);

        let (iliski_key, _) = yet_key
            .create_subkey("FileAssociations")
            .map_err(|e| GorselHatasi::Kabuk(format!("FileAssociations oluşturulamadı: {e}")))?;
        for uzanti in UZANTILAR {
            let nokta_uzanti = format!(".{uzanti}");
            let _ = iliski_key.set_value(&nokta_uzanti, &PROGID);
        }

        // 3. Kayıtlı uygulama bildirimi: HKCU\Software\RegisteredApplications
        let (reg_key, _) = hkcu
            .create_subkey(r"Software\RegisteredApplications")
            .map_err(|e| GorselHatasi::Kabuk(format!("RegisteredApplications açılamadı: {e}")))?;
        let capabilities_yolu = format!(r"Software\{UYGULAMA_ADI}\Capabilities");
        reg_key
            .set_value(UYGULAMA_ADI, &capabilities_yolu)
            .map_err(|e| GorselHatasi::Kabuk(format!("Uygulama kaydı yazılamadı: {e}")))?;

        // 4. Kabuk önbelleğini tazele
        kabuk_degisikligini_bildir();

        Ok(())
    }

    pub fn iliskilendirmeyi_kaldir() -> Sonuc<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // ProgID ve yetenek anahtarlarını temizle
        let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\{PROGID}"));
        let _ = hkcu.delete_subkey_all(format!(r"Software\{UYGULAMA_ADI}"));

        // RegisteredApplications altından kaldır
        if let Ok(reg_key) = hkcu.open_subkey_with_flags(r"Software\RegisteredApplications", KEY_SET_VALUE) {
            let _ = reg_key.delete_value(UYGULAMA_ADI);
        }

        kabuk_degisikligini_bildir();
        Ok(())
    }

    pub fn durum() -> Durum {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        let g1 = hkcu
            .open_subkey_with_flags(format!(r"Software\Classes\{PROGID}\shell\open\command"), KEY_READ)
            .is_ok();
        let g2 = hkcu
            .open_subkey_with_flags(
                format!(r"Software\{UYGULAMA_ADI}\Capabilities\FileAssociations"),
                KEY_READ,
            )
            .is_ok();
        let g3 = hkcu
            .open_subkey_with_flags(r"Software\RegisteredApplications", KEY_READ)
            .and_then(|k| k.get_value::<String, _>(UYGULAMA_ADI))
            .is_ok();

        if g1 && g2 && g3 {
            Durum::Kayitli
        } else if !g1 && !g2 && !g3 {
            Durum::KayitliDegil
        } else {
            Durum::Kismi
        }
    }

    pub fn varsayilan_uygulama_sayfasini_ac() -> Sonuc<()> {
        let islem = genis_dizi("open");
        let hedef = genis_dizi("ms-settings:defaultapps");

        let sonuc = unsafe {
            ShellExecuteW(
                None,
                PCWSTR::from_raw(islem.as_ptr()),
                PCWSTR::from_raw(hedef.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };

        if (sonuc.0 as usize) <= 32 {
            return Err(GorselHatasi::Kabuk(format!(
                "Ayarlar sayfası açılamadı: ShellExecuteW hata kodu {}",
                sonuc.0 as usize
            )));
        }

        Ok(())
    }

    fn kabuk_degisikligini_bildir() {
        // ponytail: SHChangeNotify dönüş değeri olmayan (void) standart kabuk tazeleme çağrısıdır
        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::goruntu::bicim::uzanti_desteklenir;

    #[test]
    fn desteklenen_uzantilar_bicimle_tutarlidir() {
        let liste = desteklenen_uzantilar();
        assert!(!liste.is_empty(), "Uzantı listesi boş olamaz");

        for uzanti in liste {
            assert!(!uzanti.starts_with('.'), "Uzantı nokta içermemeli: {uzanti}");
            assert_eq!(
                *uzanti,
                uzanti.to_lowercase(),
                "Uzantı küçük harf olmalı: {uzanti}"
            );
            assert!(
                uzanti_desteklenir(uzanti),
                "Uzantı bicim modülünde de desteklenmeli: {uzanti}"
            );
        }
    }

    #[test]
    fn iliskilendirme_kayit_ve_kaldirma_dongusu() {
        let sahte_exe = std::env::temp_dir().join("gorsel_sahte.exe");

        // Test öncesi olası artıkları temizle
        let _ = iliskilendirmeyi_kaldir();

        let sonuc = iliskilendirmeyi_kaydet(&sahte_exe);
        assert!(sonuc.is_ok(), "Kayıt başarılı olmalı: {sonuc:?}");
        assert_eq!(durum(), Durum::Kayitli, "Kayıt sonrası durum Kayitli olmalı");

        let kaldir_sonuc = iliskilendirmeyi_kaldir();
        assert!(kaldir_sonuc.is_ok(), "Kaldırma başarılı olmalı: {kaldir_sonuc:?}");
        assert_eq!(durum(), Durum::KayitliDegil, "Kaldırma sonrası durum KayitliDegil olmalı");

        // ponytail: test sonunda kalıntı kalmaması garantilenir
        let _ = iliskilendirmeyi_kaldir();
    }
}
