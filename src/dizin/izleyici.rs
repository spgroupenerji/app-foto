//! Dosya sistemi değişiklik izleyicisi (notify tabanlı).
//!
//! Yalnızca desteklenen görsel uzantılarına sahip dosyalar için olay üretilir;
//! UI ipliğini tıkamamak için kanal tamponu dolduğunda olaylar sessizce düşürülür.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::cekirdek::hata::{GorselHatasi, Sonuc};
use crate::goruntu::bicim::uzanti_desteklenir;

const KANAL_KAPASITESI: usize = 256;

/// Dizin değişiklik olayı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DizinOlayi {
    /// Dizine yeni bir görsel eklendi.
    Eklendi(PathBuf),
    /// Dizinden bir görsel silindi.
    Silindi(PathBuf),
    /// Dizin içeriği toplu olarak değişti (yeniden tarama gerekir).
    TopluDegisim,
}

/// notify tabanlı dizin izleyici. Örnek düşürülürse izleme durur; canlı tutulmalıdır.
pub struct DizinIzleyici {
    _izleyici: RecommendedWatcher,
    dizin: PathBuf,
}

impl DizinIzleyici {
    /// Dizini izlemeye başlar; olaylar `Receiver` üzerinden UI ipliğine iletilir.
    pub fn baslat(dizin: &Path) -> Sonuc<(Self, Receiver<DizinOlayi>)> {
        let (tx, rx) = sync_channel(KANAL_KAPASITESI);

        let mut izleyici = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let Ok(olay) = res else {
                return;
            };

            if olay.paths.is_empty() {
                return;
            }

            match olay.kind {
                EventKind::Create(_) => {
                    for yol in olay.paths {
                        if yol_destekleniyor_mu(&yol) {
                            // ponytail: kanal doluysa olay düşürülür, UI asla bloklanmaz
                            let _ = tx.try_send(DizinOlayi::Eklendi(yol));
                        }
                    }
                }
                EventKind::Remove(_) => {
                    for yol in olay.paths {
                        if yol_destekleniyor_mu(&yol) {
                            let _ = tx.try_send(DizinOlayi::Silindi(yol));
                        }
                    }
                }
                EventKind::Modify(notify::event::ModifyKind::Name(_)) | EventKind::Any => {
                    let _ = tx.try_send(DizinOlayi::TopluDegisim);
                }
                // ponytail: veri/meta değişimleri dosya listesini değiştirmediğinden yok sayılır
                _ => {}
            }
        })
        .map_err(|e| GorselHatasi::Gio(format!("İzleyici oluşturulamadı: {e}")))?;

        izleyici
            .watch(dizin, RecursiveMode::NonRecursive)
            .map_err(|e| GorselHatasi::Gio(format!("Dizin izlenemedi: {e}")))?;

        Ok((
            Self {
                _izleyici: izleyici,
                dizin: dizin.to_path_buf(),
            },
            rx,
        ))
    }

    pub fn dizin(&self) -> &Path {
        &self.dizin
    }
}

fn yol_destekleniyor_mu(yol: &Path) -> bool {
    let Some(uzanti) = yol.extension().and_then(|u| u.to_str()) else {
        return false;
    };
    uzanti_desteklenir(uzanti)
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::time::Duration;

    #[test]
    fn izleyici_olaylari_iletir() {
        let temp_dizin = std::env::temp_dir().join(format!("gorsel_izleyici_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dizin);

        let (izleyici, rx) = DizinIzleyici::baslat(&temp_dizin).expect("İzleyici başlamalı");
        assert_eq!(izleyici.dizin(), temp_dizin);

        // Desteklenmeyen uzantı olayı üretmemeli
        let metin_dosyasi = temp_dizin.join("not.txt");
        std::fs::write(&metin_dosyasi, b"onemsiz metin").expect("Yazma");

        // Kısa timeout ile olay gelmediğini kontrol et
        let beklenmeyen = rx.recv_timeout(Duration::from_millis(200));
        assert!(beklenmeyen.is_err(), "Desteklenmeyen dosya olay üretmemeli");

        // Desteklenen uzantı ekleme
        let gorsel_dosyasi = temp_dizin.join("resim.png");
        std::fs::write(&gorsel_dosyasi, b"sahte png verisi").expect("Görsel yazma");

        let alinan = rx.recv_timeout(Duration::from_secs(4));
        assert!(
            alinan.is_ok(),
            "Görsel eklendiğinde bir olay alınmalıydı: {alinan:?}"
        );
        match alinan.unwrap() {
            DizinOlayi::Eklendi(yol) => assert_eq!(yol, gorsel_dosyasi),
            DizinOlayi::TopluDegisim => {} // Windows bazen yeniden adlandırma/toplu değişim gönderebilir
            DizinOlayi::Silindi(_) => panic!("Ekleme işleminde Silindi beklenmez"),
        }

        drop(izleyici);
        let _ = std::fs::remove_dir_all(&temp_dizin);
    }
}
