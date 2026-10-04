//! Windows yerel dosya ve klasör seçme diyalogları (`IFileOpenDialog`).
//!
//! Sistem diyalogu kullanılır: modern Windows görünümü, hızlı erişim, ağ konumları ve
//! kabuk tümleşimi bedava gelir. Diyalog COM tek iş parçacıklı dairesinde (STA) ve
//! pencereyi oluşturan iş parçacığında çalışmak zorundadır; bu yüzden uygulama olay
//! döngüsünden çağrılır ve diyalog kapanana kadar olay akışı duraklar (modal davranış).

use std::path::{Path, PathBuf};

use crate::cekirdek::hata::{GorselHatasi, Sonuc};

/// Açılacak diyalog türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiyalogTuru {
    /// Tek bir görsel dosyası seçtirir.
    Dosya,
    /// Bir klasör seçtirir (içindeki görseller taranır).
    Klasor,
}

impl DiyalogTuru {
    /// Diyalog penceresinin başlığı.
    pub fn baslik(self) -> &'static str {
        match self {
            Self::Dosya => "Görsel Aç",
            Self::Klasor => "Klasör Aç",
        }
    }
}

/// Kullanıcıdan dosya veya klasör seçmesini ister.
///
/// Dönüş: `Ok(Some(yol))` seçim yapıldı, `Ok(None)` kullanıcı iptal etti,
/// `Err(..)` diyalog kurulamadı (uygulama çalışmaya devam eder).
pub fn sec(tur: DiyalogTuru, baslangic: Option<&Path>) -> Sonuc<Option<PathBuf>> {
    #[cfg(windows)]
    {
        win32::sec(tur, baslangic)
    }
    #[cfg(not(windows))]
    {
        let _ = (tur, baslangic);
        Err(GorselHatasi::Kabuk(
            "dosya diyalogu bu platformda desteklenmiyor".into(),
        ))
    }
}

/// Uygulama açılışında bir kez çağrılır: COM tek iş parçacıklı dairesini (STA) kurar.
///
/// Zaten başka bir modelle başlatılmışsa (`RPC_E_CHANGED_MODE`) hata sayılmaz;
/// diyalog çağrısı o durumda da çalışır.
pub fn com_hazirla() {
    #[cfg(windows)]
    {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        let sonuc = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if sonuc.is_err() {
            log::debug!("COM zaten kurulu ya da farklı modelde: {sonuc:?}");
        }
    }
}

/// Diyalogda gösterilecek dosya filtresi deseni: `*.jpg;*.jpeg;...`.
///
/// Uzantı listesi, kabuk ilişkilendirmesiyle aynı kaynaktan (tek doğruluk kaynağı)
/// türetilir; böylece ikisi hiçbir zaman ayrışmaz.
pub fn gorsel_deseni() -> String {
    crate::kabuk::iliskilendirme::desteklenen_uzantilar()
        .iter()
        .map(|u| format!("*.{u}"))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(windows)]
mod win32 {
    use super::{DiyalogTuru, GorselHatasi, Path, PathBuf, Sonuc};
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::{
        Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileOpenDialog, IShellItem,
        FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS,
        SIGDN_FILESYSPATH,
    };

    /// UTF-16, sonlandırıcı dahil.
    fn genis(metin: &str) -> Vec<u16> {
        metin.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn sec(tur: DiyalogTuru, baslangic: Option<&Path>) -> Sonuc<Option<PathBuf>> {
        let diyalog: IFileOpenDialog =
            unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
                .map_err(|k| GorselHatasi::Kabuk(format!("dosya diyalogu açılamadı: {k}")))?;

        unsafe {
            let baslik = genis(tur.baslik());
            let _ = diyalog.SetTitle(PCWSTR::from_raw(baslik.as_ptr()));

            let mut secenekler = FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST;
            if tur == DiyalogTuru::Klasor {
                secenekler |= FOS_PICKFOLDERS;
            } else {
                secenekler |= FOS_FILEMUSTEXIST;
            }
            diyalog
                .SetOptions(secenekler)
                .map_err(|k| GorselHatasi::Kabuk(format!("diyalog seçenekleri kurulamadı: {k}")))?;

            // Dosya kipinde tür filtresi: desteklenen görseller + tüm dosyalar.
            // Filtre dizeleri `SetFileTypes` çağrısı boyunca yaşamak zorundadır.
            let ad_gorsel = genis("Görseller");
            let desen_gorsel = genis(&super::gorsel_deseni());
            let ad_tum = genis("Tüm dosyalar");
            let desen_tum = genis("*.*");
            if tur == DiyalogTuru::Dosya {
                let filtreler = [
                    COMDLG_FILTERSPEC {
                        pszName: PCWSTR::from_raw(ad_gorsel.as_ptr()),
                        pszSpec: PCWSTR::from_raw(desen_gorsel.as_ptr()),
                    },
                    COMDLG_FILTERSPEC {
                        pszName: PCWSTR::from_raw(ad_tum.as_ptr()),
                        pszSpec: PCWSTR::from_raw(desen_tum.as_ptr()),
                    },
                ];
                let _ = diyalog.SetFileTypes(&filtreler);
            }

            // Başlangıç klasörü: aktif görselin bulunduğu dizin.
            if let Some(dizin) = baslangic {
                let dizin_genis = genis(&dizin.to_string_lossy());
                if let Ok(oge) = windows::Win32::UI::Shell::SHCreateItemFromParsingName::<
                    _,
                    Option<&windows::Win32::System::Com::IBindCtx>,
                    IShellItem,
                >(PCWSTR::from_raw(dizin_genis.as_ptr()), None)
                {
                    let _ = diyalog.SetFolder(&oge);
                }
            }

            // Göstermek: kullanıcının iptali hata değil, `None` olarak raporlanır.
            if let Err(k) = diyalog.Show(None) {
                log::debug!("diyalog kapatıldı: {k}");
                return Ok(None);
            }

            let oge = diyalog
                .GetResult()
                .map_err(|k| GorselHatasi::Kabuk(format!("seçim alınamadı: {k}")))?;
            let ham = oge
                .GetDisplayName(SIGDN_FILESYSPATH)
                .map_err(|k| GorselHatasi::Kabuk(format!("yol okunamadı: {k}")))?;

            // Yol dizesi kopyalanır, ardından COM belleği serbest bırakılır.
            let metin = ham.to_string().unwrap_or_default();
            CoTaskMemFree(Some(ham.0 as *const core::ffi::c_void));

            if metin.is_empty() {
                return Ok(None);
            }
            Ok(Some(PathBuf::from(metin)))
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn basliklar_turkce() {
        assert_eq!(DiyalogTuru::Dosya.baslik(), "Görsel Aç");
        assert_eq!(DiyalogTuru::Klasor.baslik(), "Klasör Aç");
    }

    #[test]
    fn desen_tum_desteklenen_uzantilari_kapsar() {
        let desen = gorsel_deseni();
        assert!(desen.starts_with("*."), "desen yıldızla başlamalı: {desen}");
        assert!(desen.contains("*.jpg"), "JPEG desende olmalı: {desen}");
        assert!(desen.contains("*.png"));
        assert!(desen.contains("*.jxl"));
        // Uzantı sayısı ilişkilendirme listesiyle aynı olmalı (tek doğruluk kaynağı).
        let parca = desen.split(';').count();
        assert_eq!(
            parca,
            crate::kabuk::iliskilendirme::desteklenen_uzantilar().len(),
            "desen parça sayısı uzantı listesiyle eşleşmeli"
        );
    }

    #[test]
    fn desen_ayirici_kullanir() {
        let desen = gorsel_deseni();
        assert!(!desen.contains(','), "Windows desen ayırıcısı noktalı virgüldür");
    }
}
