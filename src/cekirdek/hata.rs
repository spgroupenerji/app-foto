use std::path::PathBuf;

/// Uygulama geneli hata tipi. Alan (domain) hataları ile sistem hataları ayrı varyantlardır;
/// hiçbir hata sessizce yutulmaz, her biri kullanıcıya Türkçe mesajla raporlanır.
#[derive(Debug, thiserror::Error)]
pub enum GorselHatasi {
    #[error("Dosya bulunamadı: {0}")]
    Bulunamadi(PathBuf),

    #[error("Dosya okunamadı: {yol}")]
    Okuma {
        yol: PathBuf,
        #[source]
        kaynak: std::io::Error,
    },

    #[error("Ağ kaynağına erişilemedi: {yol}")]
    AgErisimi {
        yol: PathBuf,
        #[source]
        kaynak: std::io::Error,
    },

    #[error("Desteklenmeyen görsel biçimi: {0}")]
    DesteklenmeyenBicim(String),

    #[error("Bozuk görsel verisi: {0}")]
    BozukVeri(String),

    #[error("Renk profili işlenemedi: {0}")]
    RenkProfili(String),

    #[error("Görsel belleğe sığmıyor: {genislik}×{yukseklik} ({bayt} bayt)")]
    BellekYetersiz {
        genislik: u32,
        yukseklik: u32,
        bayt: u64,
    },

    #[error("Grafik aygıtı hatası: {0}")]
    Grafik(String),

    #[error("Pencere sistemi hatası: {0}")]
    Pencere(String),

    #[error("Kabuk işlemi başarısız: {0}")]
    Kabuk(String),

    #[error("Önbellek hatası: {0}")]
    Onbellek(String),

    #[error("Ayar dosyası hatası: {0}")]
    Ayar(String),

    #[error("Girdi/çıktı hatası: {0}")]
    Gio(String),
}

pub type Sonuc<T> = Result<T, GorselHatasi>;

impl GorselHatasi {
    /// Bağlam ekleyerek okuma hatası üretir; kaynak `io::Error` korunur.
    pub fn okuma(yol: impl Into<PathBuf>, kaynak: std::io::Error) -> Self {
        Self::Okuma {
            yol: yol.into(),
            kaynak,
        }
    }

    /// Görselin gösterilememesi durumunda kullanıcıya önerilecek kısa eylem ipucu.
    pub fn eylem_ipucu(&self) -> &'static str {
        match self {
            Self::Bulunamadi(_) => "Dosya taşınmış veya silinmiş olabilir; dizini yenileyin.",
            Self::Okuma { .. } => "Dosya başka bir uygulama tarafından kilitlenmiş olabilir.",
            Self::AgErisimi { .. } => "Ağ bağlantısını kontrol edip tekrar deneyin.",
            Self::DesteklenmeyenBicim(_) => "Farklı bir görsel dosyası açın.",
            Self::BozukVeri(_) => "Dosya eksik indirilmiş olabilir.",
            Self::RenkProfili(_) => "Görsel sRGB varsayımıyla gösteriliyor.",
            Self::BellekYetersiz { .. } => "Diğer uygulamaları kapatıp tekrar deneyin.",
            Self::Grafik(_) => "Sürücüyü güncelleyin veya yazılım oluşturucuyu kullanın.",
            Self::Pencere(_) | Self::Kabuk(_) => "İşlem tamamlanamadı, tekrar deneyin.",
            Self::Onbellek(_) | Self::Ayar(_) | Self::Gio(_) => {
                "Dosya izinlerini kontrol edin."
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn her_varyantin_eylem_ipucu_var() {
        let ornekler = [
            GorselHatasi::Bulunamadi(PathBuf::from("x.jpg")),
            GorselHatasi::DesteklenmeyenBicim("xyz".into()),
            GorselHatasi::BozukVeri("başlık yok".into()),
            GorselHatasi::RenkProfili("icc".into()),
            GorselHatasi::BellekYetersiz {
                genislik: 1,
                yukseklik: 1,
                bayt: 4,
            },
            GorselHatasi::Grafik("yüzey".into()),
            GorselHatasi::Pencere("olay".into()),
            GorselHatasi::Kabuk("kayıt".into()),
            GorselHatasi::Onbellek("lru".into()),
            GorselHatasi::Ayar("json".into()),
            GorselHatasi::Gio("kanal".into()),
        ];
        for hata in &ornekler {
            assert!(!hata.eylem_ipucu().is_empty());
            assert!(!hata.to_string().is_empty());
        }
    }

    #[test]
    fn okuma_hatasi_yolu_korur() {
        let hata = GorselHatasi::okuma(
            "C:/yok.jpg",
            std::io::Error::new(std::io::ErrorKind::NotFound, "yok"),
        );
        assert!(hata.to_string().contains("yok.jpg"));
    }
}
