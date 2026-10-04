//! Windows Gezgini ile birebir aynı dizin sıralaması.
//!
//! Windows'ta `shlwapi!StrCmpLogicalW` çağrılır; böylece "resim2.jpg" < "resim10.jpg"
//! sırası Gezgin ile aynı olur. Diğer platformlarda (ve birim testlerinde) aynı
//! semantiği taşıyan saf Rust yedeği kullanılır.

use std::cmp::Ordering;
use std::path::Path;

/// İki metni Windows doğal sıralama kuralına göre karşılaştırır.
pub fn dogal_karsilastir(a: &str, b: &str) -> Ordering {
    #[cfg(windows)]
    {
        let sonuc = win32::karsilastir(a, b);
        if sonuc != 0 {
            return sonuc.cmp(&0);
        }
        // Win32 eşit derse sıralamayı kararlı kılmak için bayt karşılaştırmasına düş.
        return a.cmp(b);
    }
    #[cfg(not(windows))]
    {
        saf_dogal_karsilastir(a, b).then_with(|| a.cmp(b))
    }
}

/// İki yolu dosya adlarına göre doğal sıralama ile karşılaştırır.
pub fn yol_karsilastir(a: &Path, b: &Path) -> Ordering {
    let ad_a = a.file_name().map(|s| s.to_string_lossy().into_owned());
    let ad_b = b.file_name().map(|s| s.to_string_lossy().into_owned());
    match (ad_a, ad_b) {
        (Some(x), Some(y)) => dogal_karsilastir(&x, &y),
        _ => a.cmp(b),
    }
}

/// Yol listesini Windows Gezgini sırasına göre yerinde sıralar.
pub fn yollari_sirala(yollar: &mut [std::path::PathBuf]) {
    yollar.sort_by(|a, b| yol_karsilastir(a, b));
}

#[cfg_attr(windows, allow(dead_code))]
/// Saf Rust doğal sıralama: rakam öbekleri sayısal, diğer karakterler harf olarak
/// karşılaştırılır. Büyük/küçük harf farkı ikincil önemdedir.
pub fn saf_dogal_karsilastir(a: &str, b: &str) -> Ordering {
    let pa: Vec<char> = a.chars().collect();
    let pb: Vec<char> = b.chars().collect();
    let (mut i, mut j) = (0usize, 0usize);

    while i < pa.len() && j < pb.len() {
        let ca = pa[i];
        let cb = pb[j];

        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            // Rakam öbeğinin tamamını al, baştaki sıfırları yok say.
            let bas_a = i;
            while i < pa.len() && pa[i].is_ascii_digit() {
                i += 1;
            }
            let bas_b = j;
            while j < pb.len() && pb[j].is_ascii_digit() {
                j += 1;
            }
            let sayi_a = &pa[bas_a..i];
            let sayi_b = &pb[bas_b..j];
            let kirp_a = kirp_sifirlari(sayi_a);
            let kirp_b = kirp_sifirlari(sayi_b);

            match kirp_a.len().cmp(&kirp_b.len()) {
                Ordering::Equal => {}
                esitsiz => return esitsiz,
            }
            match kirp_a.cmp(kirp_b) {
                Ordering::Equal => {}
                esitsiz => return esitsiz,
            }
            // Sayılar eşit: daha az basamaklı (daha az sıfırlı) olan önce gelir.
            match sayi_a.len().cmp(&sayi_b.len()) {
                Ordering::Equal => {}
                esitsiz => return esitsiz,
            }
            continue;
        }

        // Harfler büyük/küçük harf duyarsız, sonra duyarlı karşılaştırılır.
        let ka = ca.to_lowercase().next().unwrap_or(ca);
        let kb = cb.to_lowercase().next().unwrap_or(cb);
        match ka.cmp(&kb) {
            Ordering::Equal => match ca.cmp(&cb) {
                Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
                esitsiz => return esitsiz,
            },
            esitsiz => return esitsiz,
        }
    }
    (pa.len() - i).cmp(&(pb.len() - j))
}

#[cfg_attr(windows, allow(dead_code))]
fn kirp_sifirlari(s: &[char]) -> &[char] {
    let ilk_sifir_olmayan = s.iter().position(|c| *c != '0').unwrap_or(s.len());
    &s[ilk_sifir_olmayan..]
}

#[cfg(windows)]
mod win32 {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::StrCmpLogicalW;

    /// Win32 `StrCmpLogicalW` çağrısı; UTF-16 dönüşümü yerel olarak yapılır.
    pub fn karsilastir(a: &str, b: &str) -> i32 {
        let genis_a: Vec<u16> = a.encode_utf16().chain(std::iter::once(0)).collect();
        let genis_b: Vec<u16> = b.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            StrCmpLogicalW(
                PCWSTR::from_raw(genis_a.as_ptr()),
                PCWSTR::from_raw(genis_b.as_ptr()),
            )
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rakamlar_sayisal_siralanir() {
        let mut adlar = vec!["resim10.jpg", "resim2.jpg", "resim1.jpg"];
        adlar.sort_by(|a, b| saf_dogal_karsilastir(a, b));
        assert_eq!(adlar, vec!["resim1.jpg", "resim2.jpg", "resim10.jpg"]);
    }

    #[test]
    fn sifir_dolgulu_adlar_dogru_siralanir() {
        let mut adlar = vec!["g003.png", "g1.png", "g02.png", "g10.png"];
        adlar.sort_by(|a, b| saf_dogal_karsilastir(a, b));
        assert_eq!(adlar, vec!["g1.png", "g02.png", "g003.png", "g10.png"]);
    }

    #[test]
    fn coklu_rakam_obekleri() {
        let mut adlar = vec!["2026-10-01.jpg", "2026-2-01.jpg", "2026-2-10.jpg"];
        adlar.sort_by(|a, b| saf_dogal_karsilastir(a, b));
        assert_eq!(
            adlar,
            vec!["2026-2-01.jpg", "2026-2-10.jpg", "2026-10-01.jpg"]
        );
    }

    #[test]
    fn ayni_sayida_farkli_uzunluk() {
        // Aynı değer, daha az sıfırlı olan önce.
        assert_eq!(
            saf_dogal_karsilastir("a7.jpg", "a07.jpg"),
            Ordering::Less
        );
    }

    #[test]
    fn buyuk_kucuk_harf_ikincil() {
        assert_eq!(saf_dogal_karsilastir("A.jpg", "a.jpg"), Ordering::Less);
        assert_eq!(saf_dogal_karsilastir("abc", "ABC"), Ordering::Greater);
    }

    #[test]
    fn on_ek_farkliysa_harf_sirasi() {
        assert_eq!(saf_dogal_karsilastir("ay.jpg", "ba.jpg"), Ordering::Less);
        assert_eq!(saf_dogal_karsilastir("resim9", "resim10"), Ordering::Less);
    }

    #[test]
    fn esit_metinler_esittir() {
        assert_eq!(saf_dogal_karsilastir("aynı.jpg", "aynı.jpg"), Ordering::Equal);
    }

    #[test]
    fn turkce_karakterler_panik_yapmaz() {
        let mut adlar = ["ığüş.jpg", "ağaç.jpg", "öğle.png"];
        adlar.sort_by(|a, b| saf_dogal_karsilastir(a, b));
        assert_eq!(adlar.len(), 3);
    }

    #[test]
    fn yollar_dosya_adiyla_siralanir() {
        let mut yollar = vec![
            PathBuf::from(r"C:\p\resim10.jpg"),
            PathBuf::from(r"C:\p\resim2.jpg"),
            PathBuf::from(r"C:\p\resim1.jpg"),
        ];
        yollari_sirala(&mut yollar);
        let adlar: Vec<String> = yollar
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(adlar, vec!["resim1.jpg", "resim2.jpg", "resim10.jpg"]);
    }

    #[test]
    fn bos_metin_en_basta() {
        assert_eq!(saf_dogal_karsilastir("", "a"), Ordering::Less);
        assert_eq!(saf_dogal_karsilastir("", ""), Ordering::Equal);
    }

    /// Win32 yolunun gerçekten Gezgin sırası verdiğini doğrular.
    #[cfg(windows)]
    #[test]
    fn win32_dogal_siralama_gezgin_ile_ayni() {
        assert_eq!(dogal_karsilastir("resim2.jpg", "resim10.jpg"), Ordering::Less);
        assert_eq!(dogal_karsilastir("resim10.jpg", "resim2.jpg"), Ordering::Greater);
        assert_eq!(dogal_karsilastir("resim1.jpg", "resim1.jpg"), Ordering::Equal);
        assert_eq!(dogal_karsilastir("g1.png", "g02.png"), Ordering::Less);
    }
}
