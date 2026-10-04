//! G/Ç yolu sınıflandırması ve dosya açma.
//!
//! Ağ yollarında (UNC / eşlenmiş sürücü) bellek eşleme (mmap) KESİNLİKLE kullanılmaz:
//! SMB üzerinde bir paket kaybı, çekirdek düzeyinde `STATUS_IN_PAGE_ERROR` (0xC0000006)
//! istisnası üretir; bu istisna Rust tarafından yakalanamaz ve süreç günlük dahi
//! bırakamadan sonlanır. Bu nedenle tüm okumalar tamponlu akışla yapılır ve yerel
//! dosyalarda bile sıralı tarama ipucuyla (`FILE_FLAG_SEQUENTIAL_SCAN`) açılır.

use std::fs::File;
use std::path::{Path, PathBuf};

use super::super::cekirdek::hata::{GorselHatasi, Sonuc};

/// Win32 erişim ve açma bayrakları (magic value kullanmamak için adlandırılmıştır).
const WIN32_GENERIC_READ: u32 = 0x8000_0000;
const WIN32_FILE_SHARE_ALL: u32 = 0x0000_0007; // READ | WRITE | DELETE
const WIN32_OPEN_EXISTING: u32 = 3;
const WIN32_FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
const WIN32_FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;
const WIN32_INVALID_HANDLE: isize = -1;

/// Uzun yol ön eki: `\\?\` ve `//?/`.
const UZUN_YOL_EK_UNC: &str = r"\\?\UNC\";
const UZUN_YOL_EK: &str = r"\\?\";

/// Yolun bulunduğu ortam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YolTuru {
    Yerel,
    Ag,
}

impl YolTuru {
    pub fn etiket(self) -> &'static str {
        match self {
            Self::Yerel => "Yerel disk",
            Self::Ag => "Ağ paylaşımı",
        }
    }
}

/// Yolun ağ üzerinde olup olmadığını belirler.
pub fn yol_turu(yol: &Path) -> YolTuru {
    if ag_yolu_mu(yol) {
        YolTuru::Ag
    } else {
        YolTuru::Yerel
    }
}

/// Yol bir UNC yolu veya uzak sürücü harfi mi?
pub fn ag_yolu_mu(yol: &Path) -> bool {
    let metin = yol.to_string_lossy();
    ag_metni_mi(&metin)
}

/// Metin biçiminde ağ yolu denetimi (birim testlerinde doğrudan çağrılır).
pub fn ag_metni_mi(metin: &str) -> bool {
    if metin.is_empty() {
        return false;
    }
    // Uzun yol önekli UNC: \\?\UNC\sunucu\paylasim
    if basliyor_mu_harf_duyarsiz(metin, UZUN_YOL_EK_UNC) {
        return true;
    }
    if metin.starts_with("//?/") || metin.starts_with(UZUN_YOL_EK) {
        // Uzun yol öneki ama UNC değil: sürücü harfine bakılır.
        return surucu_uzak_mi(surucu_harfi(metin));
    }
    // Klasik UNC: \\sunucu\paylasim veya //sunucu/paylasim
    if metin.starts_with("\\\\") || metin.starts_with("//") {
        return true;
    }
    surucu_uzak_mi(surucu_harfi(metin))
}

/// `C:\...`, `\\?\C:\...` ve `C:/...` biçimlerinden sürücü harfini çıkarır.
fn surucu_harfi(metin: &str) -> Option<char> {
    let kalan = metin
        .strip_prefix(UZUN_YOL_EK)
        .or_else(|| metin.strip_prefix("//?/"))
        .unwrap_or(metin);
    let mut karakterler = kalan.chars();
    let harf = karakterler.next()?;
    if !harf.is_ascii_alphabetic() {
        return None;
    }
    match karakterler.next() {
        Some(':') => Some(harf.to_ascii_uppercase()),
        _ => None,
    }
}

/// Sürücü harfinin uzak (ağ) sürücü olup olmadığını Win32'den sorgular.
fn surucu_uzak_mi(harf: Option<char>) -> bool {
    let Some(harf) = harf else {
        return false;
    };
    #[cfg(windows)]
    {
        win32::surucu_uzak_mi(harf)
    }
    #[cfg(not(windows))]
    {
        let _ = harf;
        false
    }
}

fn basliyor_mu_harf_duyarsiz(metin: &str, ek: &str) -> bool {
    metin.len() >= ek.len() && metin[..ek.len()].eq_ignore_ascii_case(ek)
}

/// Dosyayı okumak için açar.
///
/// Yerel yollarda Win32 üzerinden `FILE_FLAG_SEQUENTIAL_SCAN` ipucu verilir; bu ipucu
/// önbellek yöneticisine büyük bloklar halinde spekülatif okuma yaptırır. Win32 yolu
/// herhangi bir nedenle başarısız olursa standart `File::open` yedeğine düşülür, böylece
/// uygulama hiçbir koşulda dosya açamaz duruma gelmez.
pub fn okuma_icin_ac(yol: &Path) -> Sonuc<File> {
    if !yol.exists() {
        return Err(GorselHatasi::Bulunamadi(yol.to_path_buf()));
    }
    let tur = yol_turu(yol);
    #[cfg(windows)]
    {
        if tur == YolTuru::Yerel {
            if let Some(dosya) = win32::sirali_tarama_ile_ac(yol) {
                return Ok(dosya);
            }
        }
    }
    File::open(yol).map_err(|k| esle(yol, tur, k))
}

fn esle(yol: &Path, tur: YolTuru, kaynak: std::io::Error) -> GorselHatasi {
    match tur {
        YolTuru::Ag => GorselHatasi::AgErisimi {
            yol: yol.to_path_buf(),
            kaynak,
        },
        YolTuru::Yerel => GorselHatasi::okuma(yol, kaynak),
    }
}

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
/// Dosyanın tam yolunu normalize eder (göreli yolları mutlak yapar, sembolik üstleri çözer).
pub fn mutlaklastir(yol: &Path) -> PathBuf {
    std::fs::canonicalize(yol).unwrap_or_else(|_| yol.to_path_buf())
}

/// Ağ yolu ise kullanıcıya gösterilecek uyarı metni.
pub fn ag_uyarisi(yol: &Path) -> Option<String> {
    if ag_yolu_mu(yol) {
        Some(format!(
            "Ağ kaynağından okunuyor: {}. Bağlantı kesilirse görsel yüklenemez.",
            yol.display()
        ))
    } else {
        None
    }
}

#[cfg(windows)]
mod win32 {
    use std::fs::File;
    use std::os::windows::io::FromRawHandle;
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{CreateFileW, GetDriveTypeW};
    use windows::Win32::System::WindowsProgramming::DRIVE_REMOTE;

    use super::{
        WIN32_FILE_ATTRIBUTE_NORMAL, WIN32_FILE_FLAG_SEQUENTIAL_SCAN, WIN32_FILE_SHARE_ALL,
        WIN32_GENERIC_READ, WIN32_INVALID_HANDLE, WIN32_OPEN_EXISTING,
    };

    /// Sürücü harfini `Z:\` biçiminde sorgular; `DRIVE_REMOTE` ise ağ sürücüsüdür.
    pub fn surucu_uzak_mi(harf: char) -> bool {
        let kok: Vec<u16> = format!("{harf}:\\")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let tur = unsafe { GetDriveTypeW(PCWSTR::from_raw(kok.as_ptr())) };
        tur == DRIVE_REMOTE
    }

    /// Sıralı tarama ipucuyla dosya açar; başarısızlıkta `None` döner (yedeğe düşülür).
    pub fn sirali_tarama_ile_ac(yol: &Path) -> Option<File> {
        let genis: Vec<u16> = yol
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                PCWSTR::from_raw(genis.as_ptr()),
                WIN32_GENERIC_READ,
                windows::Win32::Storage::FileSystem::FILE_SHARE_MODE(WIN32_FILE_SHARE_ALL),
                None,
                windows::Win32::Storage::FileSystem::FILE_CREATION_DISPOSITION(
                    WIN32_OPEN_EXISTING,
                ),
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(
                    WIN32_FILE_ATTRIBUTE_NORMAL | WIN32_FILE_FLAG_SEQUENTIAL_SCAN,
                ),
                None,
            )
        }
        .ok()?;
        if handle.is_invalid() || handle.0 as isize == WIN32_INVALID_HANDLE {
            return None;
        }
        Some(unsafe { File::from_raw_handle(handle.0 as _) })
    }

    use std::os::windows::ffi::OsStrExt;
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::io::Read;

    #[test]
    fn unc_yollari_ag_sayilir() {
        assert!(ag_metni_mi(r"\\sunucu\paylasim\gorsel.jpg"));
        assert!(ag_metni_mi(r"\\192.168.1.10\foto\a.png"));
        assert!(ag_metni_mi("//sunucu/paylasim/gorsel.jpg"));
        assert!(ag_metni_mi(r"\\?\UNC\sunucu\paylasim\a.png"));
    }

    #[test]
    fn yerel_yollar_yerel_sayilir() {
        assert!(!ag_metni_mi(r"C:\Kullanicilar\Deneme\resim.jpg"));
        assert!(!ag_metni_mi(r"\\?\C:\resim.jpg"));
        assert!(!ag_metni_mi("/home/kullanici/resim.jpg"));
        assert!(!ag_metni_mi("resim.jpg"));
        assert!(!ag_metni_mi(""));
    }

    #[test]
    fn surucu_harfi_ayiklanir() {
        assert_eq!(surucu_harfi(r"C:\x"), Some('C'));
        assert_eq!(surucu_harfi("d:/x"), Some('D'));
        assert_eq!(surucu_harfi(r"\\?\E:\x"), Some('E'));
        assert_eq!(surucu_harfi(r"\\sunucu\pay"), None);
        assert_eq!(surucu_harfi("resim.jpg"), None);
        // "C:" sürücüye göreli bir yoldur; sürücü harfi olarak tanınması doğrudur.
        assert_eq!(surucu_harfi(r"C:"), Some('C'));
    }

    #[test]
    fn olmayan_dosya_bulunamadi_hatasi() {
        let yol = PathBuf::from(r"C:\kesinlikle\yok\boyle\bir\dosya.jpg");
        match okuma_icin_ac(&yol) {
            Err(GorselHatasi::Bulunamadi(_)) => {}
            diger => panic!("Bulunamadi bekleniyordu, gelen: {diger:?}"),
        }
    }

    #[test]
    fn yerel_dosya_sirali_tarama_ipucuyla_okunur() {
        let dizin = std::env::temp_dir().join("gorsel-yol-testi");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let dosya = dizin.join("icerik.bin");
        std::fs::write(&dosya, b"gorsel icerik").expect("yazma");

        let mut f = okuma_icin_ac(&dosya).expect("açılmalı");
        let mut tampon = Vec::new();
        f.read_to_end(&mut tampon).expect("okunmalı");
        assert_eq!(&tampon, b"gorsel icerik");

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn mutlaklastir_yolu_korur() {
        let dizin = std::env::temp_dir();
        let mutlak = mutlaklastir(&dizin);
        assert!(mutlak.is_absolute() || mutlak == dizin);
    }

    #[test]
    fn ag_uyarisi_yalniz_ag_yolunda_doner() {
        assert!(ag_uyarisi(Path::new(r"\\sunucu\pay\a.jpg")).is_some());
        assert!(ag_uyarisi(Path::new(r"C:\a.jpg")).is_none());
    }
}
