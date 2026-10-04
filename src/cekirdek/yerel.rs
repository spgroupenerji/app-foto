use std::path::{Path, PathBuf};

use super::hata::{GorselHatasi, Sonuc};

/// Uygulama verilerinin toplandığı klasör adı (%APPDATA% ve %LOCALAPPDATA% altında).
pub const UYGULAMA_KLASORU: &str = "Gorsel";

/// Kalıcı ayar ve pencere yerleşimi için %APPDATA%\Gorsel.
pub fn ayar_dizini() -> Sonuc<PathBuf> {
    let kok = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| GorselHatasi::Ayar("APPDATA ortam değişkeni bulunamadı".into()))?;
    Ok(kok.join(UYGULAMA_KLASORU))
}

/// Meta veri veritabanı ve geçici önbellek için %LOCALAPPDATA%\Gorsel.
pub fn veri_dizini() -> Sonuc<PathBuf> {
    let kok = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .ok_or_else(|| GorselHatasi::Ayar("LOCALAPPDATA ortam değişkeni bulunamadı".into()))?;
    Ok(kok.join(UYGULAMA_KLASORU))
}

/// Dizini (ve varsa üstlerini) oluşturur; zaten varsa hata vermez.
pub fn dizini_hazirla(yol: &Path) -> Sonuc<()> {
    std::fs::create_dir_all(yol)
        .map_err(|k| GorselHatasi::Ayar(format!("{} oluşturulamadı: {k}", yol.display())))
}

/// Ayar dizinindeki bir dosyanın tam yolu.
pub fn ayar_dosyasi(ad: &str) -> Sonuc<PathBuf> {
    Ok(ayar_dizini()?.join(ad))
}

/// Veri dizinindeki bir dosyanın tam yolu.
pub fn veri_dosyasi(ad: &str) -> Sonuc<PathBuf> {
    Ok(veri_dizini()?.join(ad))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ayar_ve_veri_dizini_ayrisir() {
        let ayar = ayar_dizini().expect("APPDATA tanımlı olmalı");
        let veri = veri_dizini().expect("LOCALAPPDATA tanımlı olmalı");
        assert!(ayar.ends_with(UYGULAMA_KLASORU));
        assert!(veri.ends_with(UYGULAMA_KLASORU));
    }

    #[test]
    fn hazirla_tekrar_cagrilabilir() {
        let hedef = std::env::temp_dir().join("gorsel-test-dizin");
        let _ = std::fs::remove_dir_all(&hedef);
        dizini_hazirla(&hedef).expect("oluşturulmalı");
        dizini_hazirla(&hedef).expect("ikinci çağrı hata vermemeli");
        assert!(hedef.is_dir());
        let _ = std::fs::remove_dir_all(&hedef);
    }
}
