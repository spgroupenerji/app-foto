use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::hata::{GorselHatasi, Sonuc};
use super::yerel;

/// Ölçekleme filtresi. GPU hattı Catmull-Rom, CPU önizleme hattı SIMD filtreler kullanır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filtre {
    #[default]
    CatmullRom,
    Bilinear,
    Lanczos3,
}

/// Sağ tık davranışı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SagTikDavranisi {
    /// Sağ tık = önceki görsel (sol tık ileri, sağ tık geri akışı).
    #[default]
    Gezinme,
    /// Sağ tık = bağlam menüsü.
    Menu,
}

/// Ön yükleme penceresi: aktif indeksin çevresinde kaç komşu hazır tutulacak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnYuklemeGenisligi {
    Kapali,
    Dar,
    #[default]
    Normal,
    Genis,
}

impl OnYuklemeGenisligi {
    /// Aktif görselin her iki yanında önceden çözülecek komşu sayısı.
    pub fn komsu_sayisi(self) -> usize {
        match self {
            Self::Kapali => 0,
            Self::Dar => 1,
            Self::Normal => 2,
            Self::Genis => 3,
        }
    }

    /// GPU'da hazır tutulacak komşu doku sayısı.
    pub fn vram_komsu(self) -> usize {
        match self {
            Self::Kapali => 0,
            Self::Dar => 1,
            Self::Normal => 1,
            Self::Genis => 2,
        }
    }
}

/// Kullanıcı ayarları. `%APPDATA%\Gorsel\ayarlar.json` altında saklanır.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Ayarlar {
    pub sag_tik: SagTikDavranisi,
    pub on_yukleme: OnYuklemeGenisligi,
    pub olcek_filtresi: Filtre,
    /// RAM önbelleği üst sınırı (megabayt).
    pub ram_onbellek_mb: u64,
    /// HDR yüzey etkinleştirilsin mi (destekleyen ekranlarda).
    pub hdr_etkin: bool,
    /// Windows Gezgini ile aynı doğal sıralama (StrCmpLogicalW).
    pub dogal_siralama: bool,
    /// Fare tekerleği başına zoom çarpanı.
    pub tekerlek_zoom_carpani: f32,
    /// Yeni görsel açıldığında varsayılan yakınlaştırma kipi.
    pub varsayilan_zoom: VarsayilanZoom,
    /// Arka plan rengi (doğrusal olmayan sRGB, 0-255).
    pub arka_plan: [u8; 3],
    /// Çift tıklama pencereyi çerçevesiz tam ekrana alır mı.
    pub cift_tik_tam_ekran: bool,
    /// Pencere kapatılırken kenar yumuşatma (Mica) kullanılsın mı.
    pub mica_etkin: bool,
    /// Dizin izleyici (notify) etkin mi.
    pub dizin_izle: bool,
    /// Arayüz teması.
    pub tema: Tema,
    /// Bilgi paneli açık mı (varsayılan kapalı; araç çubuğu ikonuyla açılır).
    pub bilgi_paneli_acik: bool,
    /// Dizin listesi paneli açık mı.
    pub dosya_listesi_acik: bool,
    /// Dizin listesi panelinin ekran kenarı.
    pub dosya_listesi_yeri: PanelYeri,
}

/// Yeni görsel açılışında uygulanacak yakınlaştırma kipi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarsayilanZoom {
    #[default]
    Sigdir,
    GercekBoyut,
    PencereGenisligi,
}

/// Arayüz teması; "sistem" Windows'un koyu/açık tercihini izler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tema {
    #[default]
    Koyu,
    Acik,
    Sistem,
}

/// Yan panelin ekran kenarı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelYeri {
    #[default]
    Sol,
    Sag,
}

impl Default for Ayarlar {
    fn default() -> Self {
        Self {
            sag_tik: SagTikDavranisi::default(),
            on_yukleme: OnYuklemeGenisligi::default(),
            olcek_filtresi: Filtre::default(),
            ram_onbellek_mb: 512,
            hdr_etkin: true,
            dogal_siralama: true,
            tekerlek_zoom_carpani: 1.15,
            varsayilan_zoom: VarsayilanZoom::default(),
            arka_plan: [22, 22, 26],
            cift_tik_tam_ekran: true,
            mica_etkin: true,
            dizin_izle: true,
            tema: Tema::default(),
            bilgi_paneli_acik: false,
            dosya_listesi_acik: true,
            dosya_listesi_yeri: PanelYeri::default(),
        }
    }
}

impl Ayarlar {
    pub const DOSYA_ADI: &'static str = "ayarlar.json";

    /// Ayar dosyası yoksa varsayılanları döndürür; bozuksa hatayı raporlar.
    pub fn yukle() -> Sonuc<Self> {
        let yol = yerel::ayar_dosyasi(Self::DOSYA_ADI)?;
        Self::yukle_dosyadan(&yol)
    }

    pub fn yukle_dosyadan(yol: &Path) -> Sonuc<Self> {
        if !yol.exists() {
            return Ok(Self::default());
        }
        let metin = std::fs::read_to_string(yol)
            .map_err(|k| GorselHatasi::Ayar(format!("{} okunamadı: {k}", yol.display())))?;
        serde_json::from_str(&metin)
            .map_err(|k| GorselHatasi::Ayar(format!("{} çözümlenemedi: {k}", yol.display())))
    }

    /// Ayar dosyasını atomik biçimde yazar (önce .gecici, sonra taşıma).
    pub fn kaydet(&self) -> Sonuc<()> {
        let dizin = yerel::ayar_dizini()?;
        yerel::dizini_hazirla(&dizin)?;
        self.kaydet_dosyaya(&dizin.join(Self::DOSYA_ADI))
    }

    pub fn kaydet_dosyaya(&self, yol: &Path) -> Sonuc<()> {
        let metin = serde_json::to_string_pretty(self)
            .map_err(|k| GorselHatasi::Ayar(format!("ayarlar serileştirilemedi: {k}")))?;
        let gecici: PathBuf = yol.with_extension("json.gecici");
        std::fs::write(&gecici, metin)
            .map_err(|k| GorselHatasi::Ayar(format!("{} yazılamadı: {k}", gecici.display())))?;
        std::fs::rename(&gecici, yol)
            .map_err(|k| GorselHatasi::Ayar(format!("{} taşınamadı: {k}", yol.display())))?;
        Ok(())
    }

    /// RAM önbellek sınırını bayta çevirir; en az 64 MB, en çok 8 GB.
    pub fn ram_siniri_bayt(&self) -> u64 {
        const MB: u64 = 1024 * 1024;
        self.ram_onbellek_mb.clamp(64, 8 * 1024) * MB
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilanlar_makul() {
        let ayar = Ayarlar::default();
        assert_eq!(ayar.ram_siniri_bayt(), 512 * 1024 * 1024);
        assert!(ayar.dogal_siralama);
        assert_eq!(OnYuklemeGenisligi::Normal.komsu_sayisi(), 2);
        assert_eq!(OnYuklemeGenisligi::Kapali.vram_komsu(), 0);
        // Bilgi paneli kapalı başlar; dizin listesi solda açık gelir.
        assert_eq!(ayar.tema, Tema::Koyu);
        assert!(!ayar.bilgi_paneli_acik);
        assert!(ayar.dosya_listesi_acik);
        assert_eq!(ayar.dosya_listesi_yeri, PanelYeri::Sol);
    }

    #[test]
    fn gidis_donus_korunur() {
        let dizin = std::env::temp_dir().join("gorsel-ayar-testi");
        let _ = std::fs::remove_dir_all(&dizin);
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = dizin.join(Ayarlar::DOSYA_ADI);

        let mut ayar = Ayarlar::default();
        ayar.olcek_filtresi = Filtre::Lanczos3;
        ayar.sag_tik = SagTikDavranisi::Menu;
        ayar.arka_plan = [1, 2, 3];
        ayar.tema = Tema::Sistem;
        ayar.bilgi_paneli_acik = true;
        ayar.dosya_listesi_yeri = PanelYeri::Sag;
        ayar.kaydet_dosyaya(&yol).expect("kaydedilmeli");

        let geri = Ayarlar::yukle_dosyadan(&yol).expect("yüklenmeli");
        assert_eq!(geri.olcek_filtresi, Filtre::Lanczos3);
        assert_eq!(geri.sag_tik, SagTikDavranisi::Menu);
        assert_eq!(geri.arka_plan, [1, 2, 3]);
        assert_eq!(geri.tema, Tema::Sistem);
        assert!(geri.bilgi_paneli_acik);
        assert_eq!(geri.dosya_listesi_yeri, PanelYeri::Sag);

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn olmayan_dosya_varsayilan_dondurur() {
        let yol = std::env::temp_dir().join("gorsel-yok-ayar.json");
        let _ = std::fs::remove_file(&yol);
        let ayar = Ayarlar::yukle_dosyadan(&yol).expect("varsayılan gelmeli");
        assert_eq!(ayar.ram_onbellek_mb, Ayarlar::default().ram_onbellek_mb);
    }

    #[test]
    fn eski_ayar_dosyasi_yeni_alanlari_tamamlar() {
        let dizin = std::env::temp_dir().join("gorsel-ayar-eski-testi");
        let _ = std::fs::remove_dir_all(&dizin);
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = dizin.join(Ayarlar::DOSYA_ADI);
        // Yeni alanları içermeyen eski ayar dosyası: eksikler varsayılanla doldurulmalı.
        std::fs::write(&yol, r#"{"ram_onbellek_mb":256}"#).expect("yazılmalı");
        let geri = Ayarlar::yukle_dosyadan(&yol).expect("yüklenmeli");
        assert_eq!(geri.ram_onbellek_mb, 256);
        assert_eq!(geri.tema, Tema::Koyu);
        assert!(!geri.bilgi_paneli_acik);
        assert!(geri.dosya_listesi_acik);
        assert_eq!(geri.dosya_listesi_yeri, PanelYeri::Sol);
        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn ram_siniri_kirpilir() {
        let mut ayar = Ayarlar::default();
        ayar.ram_onbellek_mb = 1;
        assert_eq!(ayar.ram_siniri_bayt(), 64 * 1024 * 1024);
        ayar.ram_onbellek_mb = 999_999;
        assert_eq!(ayar.ram_siniri_bayt(), 8 * 1024 * 1024 * 1024);
    }
}
