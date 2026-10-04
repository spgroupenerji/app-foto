//! Pencere ikonu: favikon PNG'si derleme anında ikiliye gömülür, açılışta çözümlenir.

use winit::window::Icon;

/// Uygulama favikonu (başlık çubuğu, Alt-Tab ve görev çubuğu kaynağı).
const FAVIKON: &[u8] = include_bytes!("../../assets/favikon_SP-GROUP_gorsel.png");

/// Gömülü favikondan winit pencere ikonu üretir.
///
/// Çözümleme başarısız olursa `None` döner: pencere ikonsuz ama işlevsel açılır.
pub fn pencere_ikonu() -> Option<Icon> {
    let rgba = match image::load_from_memory(FAVIKON) {
        Ok(goruntu) => goruntu.into_rgba8(),
        Err(k) => {
            log::warn!("favikon çözümlenemedi, pencere ikonsuz açılacak: {k}");
            return None;
        }
    };
    let (genislik, yukseklik) = rgba.dimensions();
    match Icon::from_rgba(rgba.into_raw(), genislik, yukseklik) {
        Ok(ikon) => Some(ikon),
        Err(k) => {
            log::warn!("pencere ikonu oluşturulamadı: {k}");
            None
        }
    }
}
