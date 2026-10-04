//! Arka plan işçi havuzu: görsel çözümleme ve dizin taraması UI ipliğini bloklamaz.
//!
//! İstekler iki kuyruğa yazılır: normal kuyruk (ön yükleme, küçük resim) ve öncelik
//! kuyruğu (ekranda beklenen görsel). İşçiler önce öncelik kuyruğunu boşaltır; böylece
//! kullanıcı eklediği görsel, arka planda kalan ön yükleme işlerinin arkasında beklemez.
//! Nesil (generation) sayacı ile eski istekler işlenmeden atılır: kullanıcı hızlıca
//! ilerlerse arkada kalan çözümlemeler boşa GPU/CPU harcamaz.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::cekirdek::ayar::{Ayarlar, SiralamaTuru, SiralamaYonu};
use crate::cekirdek::hata::Sonuc;
use crate::dizin::tarayici::{DosyaBilgi, bilgileri_sirala};
use crate::goruntu::{self, IslenmisGoruntu, Onizleme, YuklemeIlerleme};

/// İşçiye gönderilen iş.
#[derive(Debug, Clone)]
pub enum Istek {
    /// Görseli çöz, renk işle ve GPU'ya hazır hale getir.
    Coz {
        nesil: u64,
        sira: usize,
        yol: PathBuf,
        hedef: Option<(u32, u32)>,
        on_yukleme: bool,
        /// Verilirse okuma/çözme aşamaları bu sayaca yazılır (arayüz % göstergesi).
        ilerleme: Option<Arc<YuklemeIlerleme>>,
    },
    /// Dizini tara, meta damgası topla ve istenen sıralamaya dizer.
    Tara {
        nesil: u64,
        dizin: PathBuf,
        odak: Option<PathBuf>,
        siralama: SiralamaTuru,
        siralama_yonu: SiralamaYonu,
        dogal_ad: bool,
    },
    /// Dizin listesi için küçük resim üret.
    OnIzleme {
        nesil: u64,
        yol: PathBuf,
        kenar: u32,
    },
}

/// İşçiden dönen sonuç.
#[derive(Debug)]
pub enum Yanit {
    Cozuldu {
        nesil: u64,
        sira: usize,
        on_yukleme: bool,
        sonuc: Sonuc<IslenmisGoruntu>,
    },
    Tarandi {
        nesil: u64,
        odak: Option<PathBuf>,
        sonuc: Sonuc<Vec<DosyaBilgi>>,
    },
    Onizlendi {
        nesil: u64,
        yol: PathBuf,
        sonuc: Sonuc<Option<Onizleme>>,
    },
}

/// Varsayılan işçi ipliği sayısı: en az 1, en çok 4 (aşırı paralel çözümleme bellek baskısı yaratır).
pub fn varsayilan_isci_sayisi() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get() / 2)
        .unwrap_or(2)
        .clamp(1, 4)
}

/// Arka plan işçi havuzu.
pub struct IsciHavuzu {
    istek_gonderen: Sender<Istek>,
    /// Ekranda beklenen işler için ayrı kuyruk; işçiler bunu önce boşaltır.
    oncelik_gonderen: Sender<Istek>,
    yanit_alan: Receiver<Yanit>,
    guncel_nesil: Arc<AtomicU64>,
    bekleyen: Arc<AtomicUsize>,
    isler: Vec<JoinHandle<()>>,
}

impl IsciHavuzu {
    /// Havuzu kurar ve işçi ipliklerini başlatır.
    pub fn baslat(ayarlar: Ayarlar) -> Self {
        Self::baslat_ile(ayarlar, varsayilan_isci_sayisi())
    }

    /// Belirtilen sayıda işçi ile havuzu kurar.
    pub fn baslat_ile(ayarlar: Ayarlar, isci_sayisi: usize) -> Self {
        let (istek_gonderen, istek_alan) = channel::<Istek>();
        let (oncelik_gonderen, oncelik_alan) = channel::<Istek>();
        let (yanit_gonderen, yanit_alan) = channel::<Yanit>();
        let istek_alan = Arc::new(Mutex::new(istek_alan));
        let oncelik_alan = Arc::new(Mutex::new(oncelik_alan));
        let guncel_nesil = Arc::new(AtomicU64::new(0));
        let bekleyen = Arc::new(AtomicUsize::new(0));
        let sayi = isci_sayisi.clamp(1, 8);

        let mut isler = Vec::with_capacity(sayi);
        for _ in 0..sayi {
            let kuyruk = Arc::clone(&istek_alan);
            let oncelik = Arc::clone(&oncelik_alan);
            let cikis = yanit_gonderen.clone();
            let nesil = Arc::clone(&guncel_nesil);
            let ayarlar = ayarlar.clone();
            let kol = std::thread::Builder::new()
                .name("gorsel-isci".to_string())
                .spawn(move || isci_dongusu(&kuyruk, &oncelik, &cikis, &nesil, &ayarlar));
            match kol {
                Ok(kol) => isler.push(kol),
                // İplik açılamazsa kalan işçilerle devam edilir; uygulama çalışmaya devam eder.
                Err(k) => log::error!("işçi ipliği açılamadı: {k}"),
            }
        }

        Self {
            istek_gonderen,
            oncelik_gonderen,
            yanit_alan,
            guncel_nesil,
            bekleyen,
            isler,
        }
    }

    /// Yeni nesli bildirir; bu numaradan eski istekler işçiler tarafından atılır.
    pub fn nesli_guncelle(&self, nesil: u64) {
        self.guncel_nesil.store(nesil, Ordering::Release);
    }

    /// İş kuyruğa yazılır; kuyruk kapalıysa `false` döner (uygulama çalışmaya devam eder).
    pub fn gonder(&self, istek: Istek) -> bool {
        self.gonder_uzerinden(&self.istek_gonderen, istek)
    }

    /// İş öncelik kuyruğuna yazılır; ekranda beklenen görsel için kullanılır.
    pub fn gonder_oncelikli(&self, istek: Istek) -> bool {
        self.gonder_uzerinden(&self.oncelik_gonderen, istek)
    }

    fn gonder_uzerinden(&self, kanal: &Sender<Istek>, istek: Istek) -> bool {
        match kanal.send(istek) {
            Ok(()) => {
                self.bekleyen.fetch_add(1, Ordering::AcqRel);
                true
            }
            Err(_) => false,
        }
    }

    /// Tamamlanmış yanıtları bloklamadan toplar.
    pub fn yanitlari_topla(&self) -> Vec<Yanit> {
        let mut yanitlar = Vec::new();
        loop {
            match self.yanit_alan.try_recv() {
                Ok(yanit) => {
                    self.bekleyen.fetch_sub(1, Ordering::AcqRel);
                    yanitlar.push(yanit);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        yanitlar
    }

    /// Kuyrukta bekleyen iş sayısı (ön yükleme kısıtlaması için).
    pub fn bekleyen_is_sayisi(&self) -> usize {
        self.bekleyen.load(Ordering::Acquire)
    }

    /// Kuyruk boş mu (yeni ön yükleme göndermek için uygun zaman).
    pub fn kuyruk_bos(&self) -> bool {
        self.bekleyen_is_sayisi() == 0
    }

    /// İşçi ipliklerinin sayısı.
    pub fn isci_sayisi(&self) -> usize {
        self.isler.len()
    }
}

/// Tek bir işçinin yaşam döngüsü: öncelik kuyruğunu boşalt, sonra normal kuyruktan iş al.
fn isci_dongusu(
    kuyruk: &Arc<Mutex<Receiver<Istek>>>,
    oncelik: &Arc<Mutex<Receiver<Istek>>>,
    cikis: &Sender<Yanit>,
    guncel_nesil: &AtomicU64,
    ayarlar: &Ayarlar,
) {
    loop {
        let istek = match oncelikten_ustten(oncelik) {
            Some(i) => Ok(i),
            None => {
                let kilit = match kuyruk.lock() {
                    Ok(k) => k,
                    // Zehirlenmiş kilit: işçi sessizce sonlanmaz, hatayı kaydeder ve çıkar.
                    Err(k) => {
                        log::error!("işçi kuyruğu kilitli kaldı: {k}");
                        return;
                    }
                };
                kilit.recv()
            }
        };
        let Ok(istek) = istek else {
            // Gönderici düştü: uygulama kapanıyor.
            return;
        };
        let guncel = guncel_nesil.load(Ordering::Acquire);
        let yanit = match istek {
            Istek::Coz {
                nesil,
                sira,
                yol,
                hedef,
                on_yukleme,
                ilerleme,
            } => {
                if nesil < guncel {
                    continue;
                }
                let sonuc = goruntu::isle(&yol, ayarlar, hedef, ilerleme.as_deref());
                Yanit::Cozuldu {
                    nesil,
                    sira,
                    on_yukleme,
                    sonuc,
                }
            }
            Istek::Tara {
                nesil,
                dizin,
                odak,
                siralama,
                siralama_yonu,
                dogal_ad,
            } => {
                if nesil < guncel {
                    continue;
                }
                let sonuc = tarayici_ile_sirala(&dizin, siralama, siralama_yonu, dogal_ad);
                Yanit::Tarandi {
                    nesil,
                    odak,
                    sonuc,
                }
            }
            Istek::OnIzleme { nesil, yol, kenar } => {
                if nesil < guncel {
                    continue;
                }
                Yanit::Onizlendi {
                    nesil,
                    yol: yol.clone(),
                    sonuc: goruntu::onizleme_uret(&yol, kenar),
                }
            }
        };
        // Sayaç yalnızca `yanitlari_topla` tarafında azaltılır: işçi işi bıraktığında değil,
        // sonuç UI tarafından alındığında "bekleyen" azalır (çift azaltma alt taşmaya yol açar).
        if cikis.send(yanit).is_err() {
            return;
        }
    }
}

/// Öncelik kuyruğunun tepesinden bloklamadan iş alır; kuyruk boşsa veya
/// kilit zehirlenmişse `None` döner (normal kuyruğa düşülür).
fn oncelikten_ustten(oncelik: &Arc<Mutex<Receiver<Istek>>>) -> Option<Istek> {
    let Ok(kilit) = oncelik.lock() else {
        return None;
    };
    kilit.try_recv().ok()
}

/// Dizini tarar ve istenen sıralamaya dizer.
fn tarayici_ile_sirala(
    dizin: &std::path::Path,
    siralama: SiralamaTuru,
    siralama_yonu: SiralamaYonu,
    dogal_ad: bool,
) -> Sonuc<Vec<DosyaBilgi>> {
    let mut bilgiler = crate::dizin::tarayici::tara(dizin)?;
    bilgileri_sirala(&mut bilgiler, siralama, siralama_yonu, dogal_ad);
    Ok(bilgiler)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn goruntu_uret(dizin: &std::path::Path, ad: &str) -> PathBuf {
        let yol = dizin.join(ad);
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([10, 20, 30, 255]));
        image::DynamicImage::ImageRgba8(img)
            .save_with_format(&yol, image::ImageFormat::Png)
            .expect("PNG yazılmalı");
        yol
    }

    #[test]
    fn isci_sayisi_makul_aralikta() {
        let n = varsayilan_isci_sayisi();
        assert!((1..=4).contains(&n), "işçi sayısı {n}");
    }

    #[test]
    fn cozme_istegi_yanit_dondurur() {
        let dizin = std::env::temp_dir().join("gorsel-isci-testi");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = goruntu_uret(&dizin, "a.png");

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        assert!(havuz.gonder(Istek::Coz {
            nesil: 1,
            sira: 0,
            yol,
            hedef: None,
            on_yukleme: false,
            ilerleme: None,
        }));

        let yanit = bekle(&havuz, 1);
        match yanit.first() {
            Some(Yanit::Cozuldu {
                sira: 0,
                sonuc: Ok(g),
                ..
            }) => {
                assert_eq!((g.genislik, g.yukseklik), (8, 8));
            }
            diger => panic!("çözülmüş görsel bekleniyordu: {diger:?}"),
        }

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn tarama_istegi_liste_dondurur() {
        let dizin = std::env::temp_dir().join("gorsel-isci-tarama");
        std::fs::create_dir_all(&dizin).expect("dizin");
        goruntu_uret(&dizin, "a.png");
        goruntu_uret(&dizin, "b.png");

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        havuz.gonder(Istek::Tara {
            nesil: 0,
            dizin: dizin.clone(),
            odak: None,
            siralama: SiralamaTuru::Ad,
            siralama_yonu: SiralamaYonu::Artan,
            dogal_ad: true,
        });

        let yanit = bekle(&havuz, 1);
        match yanit.first() {
            Some(Yanit::Tarandi { sonuc: Ok(l), .. }) => assert_eq!(l.len(), 2),
            diger => panic!("tarama sonucu bekleniyordu: {diger:?}"),
        }

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn eski_nesil_istegi_atlanir() {
        let dizin = std::env::temp_dir().join("gorsel-isci-nesil");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = goruntu_uret(&dizin, "a.png");

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        // Önce nesli ilerlet, sonra eski nesille istek gönder.
        havuz.nesli_guncelle(10);
        havuz.gonder(Istek::Coz {
            nesil: 1,
            sira: 0,
            yol,
            hedef: None,
            on_yukleme: false,
            ilerleme: None,
        });

        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            havuz.yanitlari_topla().is_empty(),
            "eski nesil isteği işlenmemeli"
        );

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn bozuk_dosya_hata_yaniti_uretir_panik_yok() {
        let dizin = std::env::temp_dir().join("gorsel-isci-bozuk");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = dizin.join("bozuk.png");
        std::fs::write(&yol, b"bu bir png degil").expect("yazılmalı");

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        havuz.gonder(Istek::Coz {
            nesil: 0,
            sira: 3,
            yol,
            hedef: None,
            on_yukleme: false,
            ilerleme: None,
        });

        let yanit = bekle(&havuz, 1);
        match yanit.first() {
            Some(Yanit::Cozuldu {
                sira: 3,
                sonuc: Err(_),
                ..
            }) => {}
            diger => panic!("hata yanıtı bekleniyordu: {diger:?}"),
        }

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn kuyruk_sayaci_azalir() {
        let dizin = std::env::temp_dir().join("gorsel-isci-sayac");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = goruntu_uret(&dizin, "a.png");

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        havuz.gonder(Istek::Coz {
            nesil: 0,
            sira: 0,
            yol,
            hedef: None,
            on_yukleme: false,
            ilerleme: None,
        });
        assert_eq!(havuz.bekleyen_is_sayisi(), 1);
        let _ = bekle(&havuz, 1);
        assert_eq!(havuz.bekleyen_is_sayisi(), 0);
        assert!(havuz.kuyruk_bos());

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn tarama_istegi_siralamayi_uygular() {
        let dizin = std::env::temp_dir().join("gorsel-isci-tarama-sirali");
        std::fs::create_dir_all(&dizin).expect("dizin");
        for ad in ["resim10.png", "resim2.png", "resim1.png"] {
            goruntu_uret(&dizin, ad);
        }

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        havuz.gonder(Istek::Tara {
            nesil: 0,
            dizin: dizin.clone(),
            odak: None,
            siralama: SiralamaTuru::Ad,
            siralama_yonu: SiralamaYonu::Azalan,
            dogal_ad: true,
        });

        let yanit = bekle(&havuz, 1);
        match yanit.first() {
            Some(Yanit::Tarandi { sonuc: Ok(l), .. }) => {
                let ilk = l[0].yol.file_name().unwrap().to_string_lossy().into_owned();
                assert_eq!(ilk, "resim10.png", "azalan sıralamada en büyük ad önce");
            }
            diger => panic!("tarama sonucu bekleniyordu: {diger:?}"),
        }

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn oncelikli_istek_yanitlanir() {
        let dizin = std::env::temp_dir().join("gorsel-isci-oncelik");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = goruntu_uret(&dizin, "a.png");

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        assert!(havuz.gonder_oncelikli(Istek::Coz {
            nesil: 0,
            sira: 0,
            yol,
            hedef: None,
            on_yukleme: false,
            ilerleme: None,
        }));

        let yanit = bekle(&havuz, 1);
        assert!(
            matches!(
                yanit.first(),
                Some(Yanit::Cozuldu {
                    sonuc: Ok(_),
                    ..
                })
            ),
            "öncelik kuyruğundaki iş yanıtlanmalı"
        );

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn onizleme_istegi_resim_uretir() {
        let dizin = std::env::temp_dir().join("gorsel-isci-onizleme");
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = goruntu_uret(&dizin, "a.png");

        let havuz = IsciHavuzu::baslat_ile(Ayarlar::default(), 1);
        havuz.gonder(Istek::OnIzleme {
            nesil: 0,
            yol: yol.clone(),
            kenar: 32,
        });

        let yanit = bekle(&havuz, 1);
        match yanit.first() {
            Some(Yanit::Onizlendi {
                yol: gelen,
                sonuc: Ok(Some(o)),
                ..
            }) => {
                assert_eq!(gelen, &yol);
                assert_eq!(o.genislik, 8, "8×8 kaynak kenar 32'ye sığar, büyütülmez");
                assert_eq!(o.rgba.len(), (8 * 8 * 4) as usize);
            }
            diger => panic!("onizleme bekleniyordu: {diger:?}"),
        }

        let _ = std::fs::remove_dir_all(&dizin);
    }

    /// Yanıt gelene kadar (en çok 10 sn) bekler.
    fn bekle(havuz: &IsciHavuzu, en_az: usize) -> Vec<Yanit> {
        let baslangic = std::time::Instant::now();
        loop {
            let yanitlar = havuz.yanitlari_topla();
            if yanitlar.len() >= en_az {
                return yanitlar;
            }
            if baslangic.elapsed() > std::time::Duration::from_secs(10) {
                panic!("zaman aşımı: {} yanıt geldi, {en_az} bekleniyordu", yanitlar.len());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
