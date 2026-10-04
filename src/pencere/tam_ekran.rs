//! Çerçevesiz tam ekran geçişi.
//!
//! Tam ekran, sistemin kenarlıklı tam ekranı yerine `Borderless` kipinde yapılır: geçiş
//! anında çözünürlük değişmediği için GPU yüzeyi yeniden oluşturulmaz ve görüntü
//! anlık sıçrama yapmaz.

use winit::window::{Fullscreen, Window};

/// Tam ekran durumunu değiştirir ve yeni durumu döndürür.
pub fn degistir(pencere: &Window, tam_ekran: bool) -> bool {
    let yeni = yeni_durum(tam_ekran);
    if yeni {
        pencere.set_fullscreen(Some(Fullscreen::Borderless(None)));
    } else {
        pencere.set_fullscreen(None);
    }
    yeni
}

/// İstenen duruma göre uygulanacak tam ekran durumu.
///
/// Şu an istek doğrudan uygulanır; ayrı bir işlev olarak tutulması, davranışın
/// pencere sistemi olmadan test edilebilmesini sağlar.
pub fn yeni_durum(istenen: bool) -> bool {
    istenen
}

/// Pencerenin tam ekran kipinde olup olmadığı.
pub fn tam_ekranda(pencere: &Window) -> bool {
    pencere.fullscreen().is_some()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn durum_istegi_yansitilir() {
        assert!(yeni_durum(true));
        assert!(!yeni_durum(false));
    }
}
