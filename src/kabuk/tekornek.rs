//! Adlandırılmış kanal (named pipe) tabanlı tek örnek yönetimi.
//!
//! İkinci kez başlatılan süreç, açılacak dosya yolunu çalışan ana sürece iletip
//! kendisini kapatır. Böylece her görsel için yeni bir pencere/süreç açılmaz.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use crate::cekirdek::hata::Sonuc;

/// Adlandırılmış kanal adı.
pub const BORU_ADI: &str = r"\\.\pipe\GorselGoruntuleyiciBoru";

/// İkincil süreç: açılacak yolu çalışan ana sürece iletir.
/// Ana süreç yoksa `false` döner ve çağıran normal başlatmaya devam eder.
pub fn yolu_ana_surece_gonder(yol: &Path) -> bool {
    yolu_belirli_borudan_gonder(BORU_ADI, yol)
}

/// Testlerde ve dahili çağrılarda parametrik kanal adı kullanımı.
pub(crate) fn yolu_belirli_borudan_gonder(boru_adi: &str, yol: &Path) -> bool {
    #[cfg(windows)]
    {
        win32::yolu_gonder(boru_adi, yol)
    }

    #[cfg(not(windows))]
    {
        let _ = (boru_adi, yol);
        false
    }
}

/// Ana süreç: kanalı dinler, gelen yolları kanal (channel) üzerinden UI ipliğine taşır.
pub struct BoruDinleyici {
    calisiyor: Arc<AtomicBool>,
}

impl BoruDinleyici {
    /// Sunucu kanalını oluşturur ve dinleme iş parçacığını başlatır.
    pub fn baslat() -> Sonuc<(Self, Receiver<PathBuf>)> {
        boru_dinleyici_baslat(BORU_ADI)
    }
}

pub(crate) fn boru_dinleyici_baslat(boru_adi: &str) -> Sonuc<(BoruDinleyici, Receiver<PathBuf>)> {
    #[cfg(windows)]
    {
        win32::dinleyici_baslat(boru_adi)
    }

    #[cfg(not(windows))]
    {
        let _ = boru_adi;
        Err(crate::cekirdek::hata::GorselHatasi::Kabuk(
            "Adlandırılmış kanal yalnızca Windows üzerinde desteklenir".into(),
        ))
    }
}

impl Drop for BoruDinleyici {
    fn drop(&mut self) {
        self.calisiyor.store(false, Ordering::SeqCst);
    }
}

#[cfg(windows)]
mod win32 {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{channel, Receiver};
    use std::sync::Arc;
    use std::thread;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, HANDLE,
        INVALID_HANDLE_VALUE,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE,
        OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, NAMED_PIPE_MODE,
        PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };

    use super::BoruDinleyici;
    use crate::cekirdek::hata::{GorselHatasi, Sonuc};
    use crate::kabuk::genis_dizi;

    const TAMPON_BOYUTU: u32 = 4096;

    pub fn yolu_gonder(boru_adi: &str, yol: &Path) -> bool {
        let genis_ad = genis_dizi(boru_adi);
        let handle = unsafe {
            CreateFileW(
                PCWSTR::from_raw(genis_ad.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        };

        let handle = match handle {
            Ok(h) if !h.is_invalid() && h != INVALID_HANDLE_VALUE => h,
            _ => return false,
        };

        let yol_metni = yol.to_string_lossy();
        let baytlar = yol_metni.as_bytes();
        let mut yazilan = 0u32;

        let sonuc = unsafe {
            WriteFile(
                handle,
                Some(baytlar),
                Some(&mut yazilan),
                None,
            )
        };

        unsafe {
            let _ = CloseHandle(handle);
        }

        sonuc.is_ok() && (yazilan as usize == baytlar.len())
    }

    /// Borudan kabul edilecek yol uzunluğu üst sınırı (güvenilmeyen veri süzgeci).
    const YOL_UST_SINIRI: usize = 4096;

    pub fn dinleyici_baslat(boru_adi: &str) -> Sonuc<(BoruDinleyici, Receiver<PathBuf>)> {
        let genis_ad = genis_dizi(boru_adi);
        let pipemode = NAMED_PIPE_MODE(PIPE_TYPE_MESSAGE.0 | PIPE_READMODE_MESSAGE.0 | PIPE_WAIT.0);

        // İlk kanal örneğini senkron olarak oluştur
        let ilk_handle = unsafe {
            CreateNamedPipeW(
                PCWSTR::from_raw(genis_ad.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                pipemode,
                PIPE_UNLIMITED_INSTANCES,
                TAMPON_BOYUTU,
                TAMPON_BOYUTU,
                0,
                None,
            )
        };

        if ilk_handle.is_invalid() || ilk_handle == INVALID_HANDLE_VALUE {
            return Err(GorselHatasi::Kabuk(
                "Adlandırılmış kanal sunucusu oluşturulamadı".into(),
            ));
        }

        let (tx, rx) = channel();
        let calisiyor = Arc::new(AtomicBool::new(true));
        let calisiyor_iplik = Arc::clone(&calisiyor);
        let ilk_handle_ham = ilk_handle.0 as usize;

        thread::spawn(move || {
            let mevcut_handle = HANDLE(ilk_handle_ham as _);

            while calisiyor_iplik.load(Ordering::SeqCst) {
                // ponytail: istemci önceden bağlandıysa ERROR_PIPE_CONNECTED başarılı sayılır
                let baglanti_sonuc = unsafe { ConnectNamedPipe(mevcut_handle, None) };
                let baglandi = match baglanti_sonuc {
                    Ok(_) => true,
                    Err(e) => {
                        let win32_kod = (e.code().0 as u32) & 0xFFFF;
                        win32_kod == ERROR_PIPE_CONNECTED.0
                    }
                };

                if baglandi {
                    let mut tampon = [0u8; TAMPON_BOYUTU as usize];
                    let mut okunan = 0u32;
                    let oku_sonuc = unsafe {
                        ReadFile(
                            mevcut_handle,
                            Some(&mut tampon),
                            Some(&mut okunan),
                            None,
                        )
                    };

                    if oku_sonuc.is_ok() && okunan > 0 {
                        // Boru aynı kullanıcının herhangi bir sürecine açıktır; gelen
                        // bayt güvenilmeyendir: uzunluk, NUL ve dosya adı denetimi
                        // olmadan yol olarak kullanılmaz.
                        if let Ok(metin) = std::str::from_utf8(&tampon[..okunan as usize]) {
                            let aday = PathBuf::from(metin);
                            if metin.len() <= YOL_UST_SINIRI
                                && !metin.contains('\0')
                                && aday.file_name().is_some()
                            {
                                let _ = tx.send(aday);
                            }
                        }
                    }

                    unsafe {
                        let _ = DisconnectNamedPipe(mevcut_handle);
                    }
                }

                if !calisiyor_iplik.load(Ordering::SeqCst) {
                    break;
                }
            }

            unsafe {
                let _ = CloseHandle(mevcut_handle);
            }
        });

        Ok((BoruDinleyici { calisiyor }, rx))
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::time::Duration;

    #[test]
    fn sunucu_yokken_gonderim_false_doner() {
        // Gerçek BORU_ADI ile sunucu yokken gönderim
        let rastgele_ad = format!(r"\\.\pipe\GorselTestYok_{}", std::process::id());
        let sonuc = yolu_belirli_borudan_gonder(&rastgele_ad, Path::new("C:/deneme.png"));
        assert!(!sonuc);
    }

    #[test]
    fn dinleyici_iki_istemci_yolunu_alir() {
        let test_boru_adi = format!(r"\\.\pipe\GorselTestBoru_{}", std::process::id());
        let (dinleyici, rx) = boru_dinleyici_baslat(&test_boru_adi).expect("Dinleyici başlamalı");

        let yol1 = Path::new(r"C:\test\gorsel1.png");
        let yol2 = Path::new(r"C:\test\gorsel2.jpg");

        // Kısa bir süre dinleyicinin ConnectNamedPipe'a girmesini bekle
        std::thread::sleep(Duration::from_millis(50));

        let gonderildi1 = yolu_belirli_borudan_gonder(&test_boru_adi, yol1);
        assert!(gonderildi1, "İlk yol gönderilmeli");

        let alinan1 = rx.recv_timeout(Duration::from_secs(3)).expect("İlk yol alınmalı");
        assert_eq!(alinan1, yol1);

        std::thread::sleep(Duration::from_millis(50));

        let gonderildi2 = yolu_belirli_borudan_gonder(&test_boru_adi, yol2);
        assert!(gonderildi2, "İkinci yol gönderilmeli");

        let alinan2 = rx.recv_timeout(Duration::from_secs(3)).expect("İkinci yol alınmalı");
        assert_eq!(alinan2, yol2);

        drop(dinleyici);
    }
}
