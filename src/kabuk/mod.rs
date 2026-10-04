//! Windows kabuk entegrasyonu: Birlikte aç, dosya ilişkilendirme ve tek örnek yönetimi.

pub mod birlikte_ac;
pub mod dosya_sec;
pub mod iliskilendirme;
pub mod tekornek;

#[cfg(windows)]
pub(crate) fn genis_dizi(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub(crate) fn yol_genis_dizi(yol: &std::path::Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    yol.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}
