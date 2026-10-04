//! Dosya okuma ve zaman damgası yardımcıları.
//!
//! Bellek eşleme (mmap) kesinlikle kullanılmaz; SMB/UNC ve yerel yollar
//! `crate::gio::yol::okuma_icin_ac` üzerinden güvenli akışla okunur.

use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::cekirdek::hata::{GorselHatasi, Sonuc};

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
/// Dosyayı asenkron (tokio) okur; SMB/UNC yollarında da tamponlu okuma yapar,
/// bellek eşleme KULLANMAZ.
pub async fn dosya_oku(yol: &Path) -> Sonuc<Vec<u8>> {
    let yol_buf = yol.to_path_buf();

    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        // ponytail: okuma_icin_ac Win32 sequential scan ve yerel/ağ ayrımını tek noktada çözer
        let mut dosya = crate::gio::yol::okuma_icin_ac(&yol_buf)?;
        let mut tampon = Vec::new();
        dosya
            .read_to_end(&mut tampon)
            .map_err(|k| GorselHatasi::okuma(&yol_buf, k))?;
        Ok(tampon)
    })
    .await
    .map_err(|e| GorselHatasi::Gio(format!("Asenkron okuma iş parçacığı hatası: {e}")))?
}

/// Dosya boyutunu ve son değişim zamanını (UNIX ms) döndürür.
pub fn dosya_damgasi(yol: &Path) -> Sonuc<(u64, u64)> {
    let meta = std::fs::metadata(yol).map_err(|k| {
        if k.kind() == std::io::ErrorKind::NotFound {
            GorselHatasi::Bulunamadi(yol.to_path_buf())
        } else {
            GorselHatasi::okuma(yol, k)
        }
    })?;

    let boyut = meta.len();
    let degisim = meta
        .modified()
        .map_err(|k| GorselHatasi::okuma(yol, k))?;
    let ms = degisim
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok((boyut, ms))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[tokio::test]
    async fn dosya_oku_ve_damga_dogru_calisir() {
        let temp_dizin = std::env::temp_dir().join(format!("gorsel_okuyucu_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dizin);
        let dosya_yolu = temp_dizin.join("deneme.bin");

        let icerik = b"Test gorsel verisi 1234567890";
        std::fs::write(&dosya_yolu, icerik).expect("Dosya yazılabilmeli");

        // dosya_damgasi testi
        let (boyut, ms) = dosya_damgasi(&dosya_yolu).expect("Damga alınabilmeli");
        assert_eq!(boyut, icerik.len() as u64);
        assert!(ms > 0, "Zaman damgası 0'dan büyük olmalı");

        // dosya_oku testi
        let okunan = dosya_oku(&dosya_yolu).await.expect("Dosya okunabilmeli");
        assert_eq!(okunan, icerik);

        // Olmayan dosya testi
        let olmayan = temp_dizin.join("yok.bin");
        assert!(dosya_damgasi(&olmayan).is_err());
        assert!(dosya_oku(&olmayan).await.is_err());

        let _ = std::fs::remove_dir_all(&temp_dizin);
    }
}
