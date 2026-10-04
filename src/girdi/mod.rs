//! Girdi eşlemesi: pencere olayları soyut [`Eylem`] değerlerine çevrilir.
//!
//! Toplama (ham olay → eylem) ile uygulama (eylem → durum değişikliği) ayrılır; böylece
//! eşleme mantığı pencere sistemi olmadan birim testleriyle doğrulanabilir.

pub mod bolge;
pub mod fare;
pub mod klavye;

use bolge::Pivot;

/// Kullanıcının tetiklediği soyut eylem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Eylem {
    Onceki,
    Sonraki,
    Ilk,
    Son,
    /// Yakınlaştırma; `pivot` verilmezse pencere merkezi kullanılır.
    Yakinlastir {
        carpan: f64,
        pivot: Option<Pivot>,
    },
    Sigdir,
    GercekBoyut,
    /// Orta bölgede basılı tutma başladı (kaydırma).
    KaydirmaBaslat,
    KaydirmaBitir,
    /// Basılı tutarken fare hareketi (ekran pikseli).
    Surukle {
        dx: f64,
        dy: f64,
    },
    TamEkranDegistir,
    /// Dosya seçme diyalogu ile görsel aç.
    DosyaAc,
    /// Klasör seçme diyalogu ile dizindeki görselleri aç.
    KlasorAc,
    BirlikteAc,
    /// Dosya ilişkilendirmesini kayıt defterine yaz (yalnızca HKCU).
    IliskilendirmeKaydet,
    /// Dosya ilişkilendirme kayıtlarını sil.
    IliskilendirmeKaldir,
    /// Windows'un varsayılan uygulamalar sayfasını aç.
    VarsayilanUygulamaSayfasi,
    YenidenYukle,
    AyarPenceresi,
    MetaPaneliDegistir,
    BaglamMenusu {
        x: f64,
        y: f64,
    },
    BaglamMenusuKapat,
    Kapat,
    /// Eşlenmeyen olay.
    Yok,
}

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
impl Eylem {
    /// Eylem sonrası yeniden çizim gerekiyor mu.
    pub fn yeniden_cizim_gerekli(self) -> bool {
        !matches!(self, Self::Yok | Self::Kapat)
    }

    /// Eylem görünümü (zoom/kaydırma) değiştiriyor mu.
    pub fn gorunumu_etkiler(self) -> bool {
        matches!(
            self,
            Self::Yakinlastir { .. }
                | Self::Sigdir
                | Self::GercekBoyut
                | Self::Surukle { .. }
                | Self::Onceki
                | Self::Sonraki
                | Self::Ilk
                | Self::Son
        )
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn yok_eylemi_yeniden_cizim_istemez() {
        assert!(!Eylem::Yok.yeniden_cizim_gerekli());
        assert!(!Eylem::Kapat.yeniden_cizim_gerekli());
        assert!(Eylem::Sonraki.yeniden_cizim_gerekli());
        assert!(Eylem::AyarPenceresi.yeniden_cizim_gerekli());
    }

    #[test]
    fn gorunum_eylemleri_dogru_isaretli() {
        assert!(Eylem::Sigdir.gorunumu_etkiler());
        assert!(Eylem::Surukle { dx: 1.0, dy: 0.0 }.gorunumu_etkiler());
        assert!(Eylem::Yakinlastir {
            carpan: 1.2,
            pivot: None
        }
        .gorunumu_etkiler());
        assert!(!Eylem::BirlikteAc.gorunumu_etkiler());
        assert!(!Eylem::TamEkranDegistir.gorunumu_etkiler());
    }
}
