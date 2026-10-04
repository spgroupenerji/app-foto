//! Pencere yönetimi: yerleşim kalıcılığı, sistem arka plan malzemesi ve tam ekran.

pub mod ikon;
pub mod mica;
pub mod tam_ekran;
pub mod yerlesim;

#[cfg(windows)]
use crate::cekirdek::hata::{GorselHatasi, Sonuc};
#[cfg(windows)]
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(windows)]
use winit::window::Window;

/// winit penceresinin Win32 tanıtıcısını (HWND) döndürür.
///
/// Win32 çağrıları yalnızca bu tanıtıcı üzerinden yapılır; pencere sistemi soyutlaması
/// winit'te kalır, böylece platforma özgü kod tek noktada toplanır.
#[cfg(windows)]
pub fn tanitici(pencere: &Window) -> Sonuc<windows::Win32::Foundation::HWND> {
    let tutamak = pencere
        .window_handle()
        .map_err(|k| GorselHatasi::Pencere(format!("pencere tutamacı alınamadı: {k}")))?;
    match tutamak.as_raw() {
        // `hwnd` alanı `NonZero<isize>` tutar; Win32 tanıtıcısı ham işaretçiye çevrilir.
        RawWindowHandle::Win32(h) => Ok(windows::Win32::Foundation::HWND(
            h.hwnd.get() as *mut std::ffi::c_void,
        )),
        _ => Err(GorselHatasi::Pencere(
            "beklenmeyen pencere tutamacı türü".into(),
        )),
    }
}
