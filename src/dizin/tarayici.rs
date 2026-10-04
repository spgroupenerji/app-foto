//! Dizin taraması: desteklenen görseller bulunur ve Windows Gezgini sırasına dizilir.
//!
//! Tarama, `std::fs::read_dir` üzerinden tek geçişte yapılır; uzantı filtresi
//! `goruntu::bicim::uzanti_desteklenir` ile aynı kaynaktan beslenir, böylece çözücü
//! matrisi ile dizin listesi hiçbir zaman ayrışmaz.

use std::path::{Path, PathBuf};

use super::sirala;
use crate::cekirdek::hata::{GorselHatasi, Sonuc};
use crate::goruntu::bicim::uzanti_desteklenir;

/// Atlanan dosya adları: sistem tarafından üretilen küçük resim/ayar dosyaları.
const ATLANAN_ADLAR: [&str; 2] = ["desktop.ini", "thumbs.db"];

/// Dizindeki desteklenen görselleri Windows doğal sıralamasında döndürür.
///
/// Okunamayan tek bir dosya listeyi bozmaz; yalnızca gizli ve sistem dosyaları atlanır.
pub fn tara(dizin: &Path) -> Sonuc<Vec<PathBuf>> {
    let okunus = std::fs::read_dir(dizin).map_err(|k| GorselHatasi::okuma(dizin, k))?;
    let mut dosyalar: Vec<PathBuf> = Vec::new();

    for girdi in okunus.flatten() {
        let yol = girdi.path();
        if !gorsel_adayi_mi(&yol) {
            continue;
        }
        dosyalar.push(yol);
    }

    sirala::yollari_sirala(&mut dosyalar);
    Ok(dosyalar)
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
        let adlar: Vec<String> = liste
            .iter()
            .map(|y| y.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(adlar, vec!["resim1.jpg", "resim2.PNG"]);
    }

    #[test]
    fn dogal_siralama_uygulanir() {
        let d = GeciciDizin::yeni("siralama");
        for ad in ["resim10.jpg", "resim2.jpg", "resim1.jpg"] {
            d.dosya(ad);
        }
        let liste = tara(&d.yol).expect("taranmalı");
        let adlar: Vec<String> = liste
            .iter()
            .map(|y| y.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(adlar, vec!["resim1.jpg", "resim2.jpg", "resim10.jpg"]);
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
