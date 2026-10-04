//! Dizin taraması: desteklenen görseller bulunur ve seçilen sıralamaya dizilir.
//!
//! Tarama, `std::fs::read_dir` üzerinden tek geçişte yapılır; uzantı filtresi
//! `goruntu::bicim::uzanti_desteklenir` ile aynı kaynaktan beslenir, böylece çözücü
//! matrisi ile dizin listesi hiçbir zaman ayrışmaz. Her dosya için boyut ve değişim
//! zamanı da toplanır; bunlar sıralama ve listede küçük bilgi metni için kullanılır.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use super::sirala;
use crate::cekirdek::ayar::{SiralamaTuru, SiralamaYonu};
use crate::cekirdek::hata::{GorselHatasi, Sonuc};
use crate::goruntu::bicim::uzanti_desteklenir;

/// Atlanan dosya adları: sistem tarafından üretilen küçük resim/ayar dosyaları.
const ATLANAN_ADLAR: [&str; 2] = ["desktop.ini", "thumbs.db"];

/// Listedeki tek bir görselin bilgisi: yol, boyut ve son değişim zamanı (UNIX ms).
///
/// Boyut/zaman okunamazsa 0 döner; gösterim ve sıralama bozulmaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DosyaBilgi {
    pub yol: PathBuf,
    pub boyut: u64,
    pub tarih_ms: u64,
}

impl DosyaBilgi {
    pub fn yol_damgasi(yol: &Path) -> Self {
        let (boyut, tarih_ms) = crate::gio::okuyucu::dosya_damgasi(yol).unwrap_or((0, 0));
        Self {
            yol: yol.to_path_buf(),
            boyut,
            tarih_ms,
        }
    }

    fn ad(&self) -> String {
        self.yol
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn uzanti(&self) -> String {
        self.yol
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }
}

/// Dizindeki desteklenen görselleri toplar; sıralama çağıranın sorumluluğundadır.
///
/// Okunamayan tek bir dosya listeyi bozmaz; yalnızca gizli ve sistem dosyaları atlanır.
pub fn tara(dizin: &Path) -> Sonuc<Vec<DosyaBilgi>> {
    let okunus = std::fs::read_dir(dizin).map_err(|k| GorselHatasi::okuma(dizin, k))?;
    let mut dosyalar: Vec<DosyaBilgi> = Vec::new();

    for girdi in okunus.flatten() {
        let yol = girdi.path();
        if !gorsel_adayi_mi(&yol) {
            continue;
        }
        dosyalar.push(DosyaBilgi::yol_damgasi(&yol));
    }

    Ok(dosyalar)
}

/// Taranan bilgileri seçilen sıralamaya dizer.
pub fn bilgileri_sirala(
    bilgiler: &mut [DosyaBilgi],
    tur: SiralamaTuru,
    yon: SiralamaYonu,
    dogal_ad: bool,
) {
    bilgiler.sort_by(|a, b| {
        let ana = match tur {
            SiralamaTuru::Ad => ad_karsilastir(a, b, dogal_ad),
            SiralamaTuru::Tur => a.uzanti().cmp(&b.uzanti()).then(ad_karsilastir(a, b, dogal_ad)),
            SiralamaTuru::Tarih => a.tarih_ms.cmp(&b.tarih_ms).then(ad_karsilastir(a, b, dogal_ad)),
            SiralamaTuru::Boyut => a.boyut.cmp(&b.boyut).then(ad_karsilastir(a, b, dogal_ad)),
        };
        // Ön yalnızca ana anahtara uygulanır: eşitlik bozmada adlar her zaman artan kalır
        // (Windows Gezgini davranışı; "azalan tarih" listesinde adlar A→Z akar).
        let ana = match yon {
            SiralamaYonu::Artan => ana,
            SiralamaYonu::Azalan => ana.reverse(),
        };
        if ana == Ordering::Equal {
            ad_karsilastir(a, b, dogal_ad)
        } else {
            ana
        }
    });
}

fn ad_karsilastir(a: &DosyaBilgi, b: &DosyaBilgi, dogal: bool) -> Ordering {
    if dogal {
        sirala::yol_karsilastir(&a.yol, &b.yol)
    } else {
        a.ad().cmp(&b.ad())
    }
}

/// Bir yolun listeye alınıp alınmayacağına karar verir.
pub fn gorsel_adayi_mi(yol: &Path) -> bool {
    if !yol.is_file() {
        return false;
    }
    let Some(ad) = yol.file_name().map(|a| a.to_string_lossy().into_owned()) else {
        return false;
    };
    if ad.starts_with('.') {
        return false;
    }
    if ATLANAN_ADLAR
        .iter()
        .any(|a| a.eq_ignore_ascii_case(&ad))
    {
        return false;
    }
    let Some(uzanti) = yol.extension().map(|e| e.to_string_lossy().into_owned()) else {
        return false;
    };
    uzanti_desteklenir(&uzanti)
}

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
/// Tarama sonucu; aynı dizin yeniden tarandığında listenin değişip değişmediğini bildirir.
pub fn listeyi_guncelle(mevcut: &[PathBuf], yeni: Vec<PathBuf>) -> (Vec<PathBuf>, bool) {
    let degisti = mevcut.len() != yeni.len() || mevcut != yeni.as_slice();
    (yeni, degisti)
}

#[cfg(test)]
mod testler {
    use super::*;

    struct GeciciDizin {
        yol: PathBuf,
    }

    impl GeciciDizin {
        fn yeni(ad: &str) -> Self {
            let yol = std::env::temp_dir().join(format!("gorsel-tarama-{ad}"));
            let _ = std::fs::remove_dir_all(&yol);
            std::fs::create_dir_all(&yol).expect("dizin oluşmalı");
            Self { yol }
        }

        fn dosya(&self, ad: &str) -> PathBuf {
            let yol = self.yol.join(ad);
            std::fs::write(&yol, b"veri").expect("dosya yazılmalı");
            yol
        }
    }

    impl Drop for GeciciDizin {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.yol);
        }
    }

    #[test]
    fn yalnizca_desteklenen_gorseller_listelenir() {
        let d = GeciciDizin::yeni("filtre");
        d.dosya("resim1.jpg");
        d.dosya("resim2.PNG");
        d.dosya("belge.txt");
        d.dosya("video.mp4");
        d.dosya("desktop.ini");
        d.dosya(".gizli.jpg");
        d.dosya("uzantisiz");

        let liste = tara(&d.yol).expect("taranmalı");
        let mut adlar: Vec<String> = liste
            .iter()
            .map(|b| b.yol.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        adlar.sort();
        assert_eq!(adlar, vec!["resim1.jpg", "resim2.PNG"]);
    }

    #[test]
    fn dogal_siralama_uygulanir() {
        let d = GeciciDizin::yeni("siralama");
        for ad in ["resim10.jpg", "resim2.jpg", "resim1.jpg"] {
            d.dosya(ad);
        }
        let mut liste = tara(&d.yol).expect("taranmalı");
        bilgileri_sirala(&mut liste, SiralamaTuru::Ad, SiralamaYonu::Artan, true);
        let adlar: Vec<String> = liste
            .iter()
            .map(|b| b.yol.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(adlar, vec!["resim1.jpg", "resim2.jpg", "resim10.jpg"]);
    }

    #[test]
    fn ada_gore_azalan_siralama_tersine_cevirir() {
        let d = GeciciDizin::yeni("azalan");
        for ad in ["a.jpg", "b.jpg", "c.jpg"] {
            d.dosya(ad);
        }
        let mut liste = tara(&d.yol).expect("taranmalı");
        bilgileri_sirala(&mut liste, SiralamaTuru::Ad, SiralamaYonu::Azalan, true);
        let adlar: Vec<String> = liste
            .iter()
            .map(|b| b.yol.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(adlar, vec!["c.jpg", "b.jpg", "a.jpg"]);
    }

    #[test]
    fn ture_gore_siralama_uzantida_birlesir() {
        let d = GeciciDizin::yeni("tur");
        for ad in ["iki.png", "bir.jpg", "uc.png"] {
            d.dosya(ad);
        }
        let mut liste = tara(&d.yol).expect("taranmalı");
        bilgileri_sirala(&mut liste, SiralamaTuru::Tur, SiralamaYonu::Artan, true);
        let adlar: Vec<String> = liste
            .iter()
            .map(|b| b.yol.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(adlar, vec!["bir.jpg", "iki.png", "uc.png"]);
    }

    #[test]
    fn boyut_ve_tarihe_gore_siralama_bilgi_damgasini_kullanir() {
        let d = GeciciDizin::yeni("boyut-tarih");
        let kucuk = d.dosya("kucuk.jpg");
        std::fs::write(&kucuk, b"ab").expect("yazılmalı");
        // NTFS değişim zamanı 10 ms tiktir; farklı zamanlar üretmek için bekle.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let buyuk = d.dosya("buyuk.jpg");
        std::fs::write(&buyuk, vec![0u8; 2048]).expect("yazılmalı");

        let mut liste = tara(&d.yol).expect("taranmalı");
        bilgileri_sirala(&mut liste, SiralamaTuru::Boyut, SiralamaYonu::Artan, true);
        assert_eq!(liste[0].yol, kucuk);
        assert_eq!(liste[1].boyut, 2048);
        assert!(liste.iter().all(|b| b.tarih_ms > 0), "değişim zamanı okunmalı");

        bilgileri_sirala(&mut liste, SiralamaTuru::Tarih, SiralamaYonu::Azalan, true);
        // En son yazılan buyuk.jpg en yeni olmalı (aynı saniyede yazılsa da ms çözünürlük korunur).
        assert_eq!(liste[0].yol, buyuk);
    }

    #[test]
    fn bos_dizin_bos_liste_dondurur() {
        let d = GeciciDizin::yeni("bos");
        assert!(tara(&d.yol).expect("taranmalı").is_empty());
    }

    #[test]
    fn olmayan_dizin_hata_dondurur() {
        let yol = std::env::temp_dir().join("gorsel-yok-boyle-dizin-12345");
        assert!(tara(&yol).is_err());
    }

    #[test]
    fn alt_dizinler_listelenmez() {
        let d = GeciciDizin::yeni("altdizin");
        std::fs::create_dir_all(d.yol.join("alt.jpg")).expect("alt dizin");
        d.dosya("ust.jpg");
        let liste = tara(&d.yol).expect("taranmalı");
        assert_eq!(liste.len(), 1, "klasörler görsel sayılmaz");
    }

    #[test]
    fn liste_degisimi_algilanir() {
        let a = vec![PathBuf::from("a.jpg")];
        let (_, degisti) = listeyi_guncelle(&a, vec![PathBuf::from("b.jpg")]);
        assert!(degisti);
        let (_, ayni) = listeyi_guncelle(&a, vec![PathBuf::from("a.jpg")]);
        assert!(!ayni);
        let (_, uzunluk) = listeyi_guncelle(&a, vec![PathBuf::from("a.jpg"), PathBuf::from("b.jpg")]);
        assert!(uzunluk);
    }

    #[test]
    fn gorsel_adayi_kontrolu() {
        let d = GeciciDizin::yeni("aday");
        let jpg = d.dosya("a.jpg");
        assert!(gorsel_adayi_mi(&jpg));
        let txt = d.dosya("a.txt");
        assert!(!gorsel_adayi_mi(&txt));
        assert!(!gorsel_adayi_mi(&d.yol), "klasör aday değil");
        assert!(!gorsel_adayi_mi(Path::new(r"C:\yok\olmayan.jpg")));
    }
}
