//! Dosya görsel meta verisi (boyut, yönelim vb.) için yerel redb önbelleği.

use std::path::Path;
use redb::{Database, ReadableDatabase, ReadableTableMetadata, TableDefinition};

use crate::cekirdek::hata::{GorselHatasi, Sonuc};

/// Dosya meta verisi önbellek kaydı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetaKayit {
    pub genislik: u32,
    pub yukseklik: u32,
    pub yonelim: u8,
    pub dosya_boyutu: u64,
    /// Son değişim zamanı (UNIX epoch, milisaniye).
    pub degisim_ms: u64,
}

const TABLO: TableDefinition<&[u8], &[u8; 32]> = TableDefinition::new("meta");

pub struct MetaVeritabani {
    vt: Database,
}

impl MetaVeritabani {
    /// Verilen dosyada veritabanını açar veya oluşturur.
    pub fn ac(yol: &Path) -> Sonuc<Self> {
        let vt = Database::create(yol)
            .or_else(|_| Database::open(yol))
            .map_err(|e| GorselHatasi::Onbellek(format!("Meta veritabanı açılamadı: {e}")))?;

        // Tabloyu oluşturmak/varlığından emin olmak için boş bir yazma işlemi yapabiliriz
        {
            let yazma = vt
                .begin_write()
                .map_err(|e| GorselHatasi::Onbellek(format!("Yazma başlatılamadı: {e}")))?;
            {
                let _ = yazma
                    .open_table(TABLO)
                    .map_err(|e| GorselHatasi::Onbellek(format!("Tablo açılamadı: {e}")))?;
            }
            yazma
                .commit()
                .map_err(|e| GorselHatasi::Onbellek(format!("İşlem onaylanamadı: {e}")))?;
        }

        Ok(Self { vt })
    }

    /// %LOCALAPPDATA%\Gorsel\meta.redb üzerinde açar.
    pub fn varsayilan() -> Sonuc<Self> {
        let yol = crate::cekirdek::yerel::veri_dosyasi("meta.redb")?;
        if let Some(ust) = yol.parent() {
            crate::cekirdek::yerel::dizini_hazirla(ust)?;
        }
        Self::ac(&yol)
    }

    /// Kayıt yalnızca dosya boyutu ve değişim zamanı eşleşiyorsa geçerlidir.
    // Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
    #[allow(dead_code)]
    pub fn al(&self, yol: &Path, dosya_boyutu: u64, degisim_ms: u64) -> Option<MetaKayit> {
        let okuma = self.vt.begin_read().ok()?;
        let tablo = okuma.open_table(TABLO).ok()?;
        let yol_metni = yol.to_string_lossy();
        let guard = tablo.get(yol_metni.as_bytes()).ok()??;
        let baytlar = guard.value();

        let kayit = kayit_coz(baytlar);
        if kayit.dosya_boyutu == dosya_boyutu && kayit.degisim_ms == degisim_ms {
            Some(kayit)
        } else {
            None
        }
    }

    pub fn koy(&self, yol: &Path, kayit: MetaKayit) -> Sonuc<()> {
        let yol_metni = yol.to_string_lossy();
        let baytlar = kayit_paketle(&kayit);

        let yazma = self
            .vt
            .begin_write()
            .map_err(|e| GorselHatasi::Onbellek(format!("Yazma başlatılamadı: {e}")))?;
        {
            let mut tablo = yazma
                .open_table(TABLO)
                .map_err(|e| GorselHatasi::Onbellek(format!("Tablo açılamadı: {e}")))?;
            tablo
                .insert(yol_metni.as_bytes(), &baytlar)
                .map_err(|e| GorselHatasi::Onbellek(format!("Kayıt eklenemedi: {e}")))?;
        }
        yazma
            .commit()
            .map_err(|e| GorselHatasi::Onbellek(format!("İşlem onaylanamadı: {e}")))?;

        Ok(())
    }

    pub fn kayit_sayisi(&self) -> Sonuc<u64> {
        let okuma = self
            .vt
            .begin_read()
            .map_err(|e| GorselHatasi::Onbellek(format!("Okuma başlatılamadı: {e}")))?;
        let tablo = okuma
            .open_table(TABLO)
            .map_err(|e| GorselHatasi::Onbellek(format!("Tablo açılamadı: {e}")))?;
        tablo
            .len()
            .map_err(|e| GorselHatasi::Onbellek(format!("Kayıt sayısı alınamadı: {e}")))
    }

    // Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
    #[allow(dead_code)]
    pub fn temizle(&self) -> Sonuc<()> {
        let yazma = self
            .vt
            .begin_write()
            .map_err(|e| GorselHatasi::Onbellek(format!("Yazma başlatılamadı: {e}")))?;
        {
            let mut tablo = yazma
                .open_table(TABLO)
                .map_err(|e| GorselHatasi::Onbellek(format!("Tablo açılamadı: {e}")))?;
            // ponytail: retain(|_, _| false) tabloyu temizlemenin en kısa ve doğru yoludur
            tablo
                .retain(|_, _| false)
                .map_err(|e| GorselHatasi::Onbellek(format!("Temizleme başarısız: {e}")))?;
        }
        yazma
            .commit()
            .map_err(|e| GorselHatasi::Onbellek(format!("İşlem onaylanamadı: {e}")))?;

        Ok(())
    }
}

fn kayit_paketle(kayit: &MetaKayit) -> [u8; 32] {
    let mut baytlar = [0u8; 32];
    baytlar[0..4].copy_from_slice(&kayit.genislik.to_le_bytes());
    baytlar[4..8].copy_from_slice(&kayit.yukseklik.to_le_bytes());
    baytlar[8] = kayit.yonelim;
    // 9..16 dolgu (sıfır)
    baytlar[16..24].copy_from_slice(&kayit.dosya_boyutu.to_le_bytes());
    baytlar[24..32].copy_from_slice(&kayit.degisim_ms.to_le_bytes());
    baytlar
}

fn kayit_coz(baytlar: &[u8; 32]) -> MetaKayit {
    let genislik = u32::from_le_bytes(baytlar[0..4].try_into().unwrap());
    let yukseklik = u32::from_le_bytes(baytlar[4..8].try_into().unwrap());
    let yonelim = baytlar[8];
    let dosya_boyutu = u64::from_le_bytes(baytlar[16..24].try_into().unwrap());
    let degisim_ms = u64::from_le_bytes(baytlar[24..32].try_into().unwrap());
    MetaKayit {
        genislik,
        yukseklik,
        yonelim,
        dosya_boyutu,
        degisim_ms,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn veritabani_yaz_oku_ve_temizle_dongusu() {
        let temp_dizin = std::env::temp_dir().join(format!("gorsel_vt_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dizin);
        let vt_yolu = temp_dizin.join("test_meta.redb");

        let vt = MetaVeritabani::ac(&vt_yolu).expect("Veritabanı açılmalı");
        assert_eq!(vt.kayit_sayisi().expect("Sayı alınmalı"), 0);

        let test_yol = Path::new("C:/foto/ornek.jpg");
        let kayit = MetaKayit {
            genislik: 1920,
            yukseklik: 1080,
            yonelim: 1,
            dosya_boyutu: 102400,
            degisim_ms: 1700000000000,
        };

        vt.koy(test_yol, kayit).expect("Kayıt konulabilmeli");
        assert_eq!(vt.kayit_sayisi().expect("Sayı alınmalı"), 1);

        // Doğru damga ile alma
        let alinan = vt.al(test_yol, 102400, 1700000000000);
        assert_eq!(alinan, Some(kayit));

        // Boyut uyuşmazlığında None
        let boyut_farkli = vt.al(test_yol, 102401, 1700000000000);
        assert_eq!(boyut_farkli, None);

        // Değişim zamanı uyuşmazlığında None
        let zaman_farkli = vt.al(test_yol, 102400, 1700000000001);
        assert_eq!(zaman_farkli, None);

        // Farklı yolda None
        let baska_yol = vt.al(Path::new("C:/foto/baska.jpg"), 102400, 1700000000000);
        assert_eq!(baska_yol, None);

        // Temizleme
        vt.temizle().expect("Temizlenebilmeli");
        assert_eq!(vt.kayit_sayisi().expect("Sayı 0 olmalı"), 0);
        assert_eq!(vt.al(test_yol, 102400, 1700000000000), None);

        drop(vt);
        let _ = std::fs::remove_dir_all(&temp_dizin);
    }
}
