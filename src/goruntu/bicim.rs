//! Görsel biçim tespiti: dosya başlığındaki sihirli baytlar (magic bytes) esastır,
//! uzantı yalnızca sihirli bayt bulunmayan biçimlerde (TGA) veya TIFF tabanlı
//! kamera RAW türlerini ayırmak için yardımcı kanıt olarak kullanılır.

/// Kamera ham (RAW) türleri. Hepsi TIFF/ISOBMFF tabanlı olduğundan uzantı ile ayrışır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HamTuru {
    Cr2,
    Cr3,
    Nef,
    Arw,
    Dng,
    Orf,
    Rw2,
    Raf,
    Pef,
    Srw,
    BilinmeyenHam,
}

/// Tanınan görsel biçimleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bicim {
    Jpeg,
    Png,
    WebP,
    Gif,
    Bmp,
    Tga,
    Ico,
    Tiff,
    Avif,
    Heic,
    Jxl,
    Exr,
    Hdr,
    Svg,
    Pnm,
    Qoi,
    Dds,
    KameraHam(HamTuru),
    Bilinmeyen,
}

const JPEG_ON_EK: [u8; 3] = [0xFF, 0xD8, 0xFF];
const PNG_ON_EK: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const TIFF_LE: [u8; 4] = [b'I', b'I', 0x2A, 0x00];
const TIFF_BE: [u8; 4] = [b'M', b'M', 0x00, 0x2A];
const ICO_ON_EK: [u8; 4] = [0x00, 0x00, 0x01, 0x00];
const CUR_ON_EK: [u8; 4] = [0x00, 0x00, 0x02, 0x00];
const EXR_ON_EK: [u8; 4] = [0x76, 0x2F, 0x31, 0x01];
const JXL_IMZA: [u8; 2] = [0xFF, 0x0A];
const JXL_KUTU: [u8; 12] = [
    0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A,
];
const RAF_ON_EK: &[u8] = b"FUJIFILMCCD-RAW";

/// Pencere başlığından (en az 32 bayt) ve varsa uzantıdan biçimi tespit eder.
pub fn tespit(baslik: &[u8], uzanti: Option<&str>) -> Bicim {
    let uzanti = uzanti.map(kucuk_ascii);

    if baslik.starts_with(&PNG_ON_EK) {
        return Bicim::Png;
    }
    if baslik.starts_with(&JPEG_ON_EK) {
        return Bicim::Jpeg;
    }
    if baslik.len() >= 6 && &baslik[0..4] == b"GIF8" {
        return Bicim::Gif;
    }
    if baslik.len() >= 2 && &baslik[0..2] == b"BM" {
        return Bicim::Bmp;
    }
    if baslik.len() >= 12 && &baslik[0..4] == b"RIFF" && &baslik[8..12] == b"WEBP" {
        return Bicim::WebP;
    }
    if baslik.starts_with(&EXR_ON_EK) {
        return Bicim::Exr;
    }
    if baslik.starts_with(&JXL_IMZA) || baslik.starts_with(&JXL_KUTU) {
        return Bicim::Jxl;
    }
    if baslik.len() >= 4 && (baslik[0..4] == ICO_ON_EK || baslik[0..4] == CUR_ON_EK) {
        return Bicim::Ico;
    }
    if baslik.starts_with(RAF_ON_EK) {
        return Bicim::KameraHam(HamTuru::Raf);
    }
    if baslik.len() >= 2 && &baslik[0..2] == b"qo" && baslik.len() >= 4 && &baslik[0..4] == b"qoif" {
        return Bicim::Qoi;
    }
    if baslik.len() >= 4 && &baslik[0..4] == b"DDS " {
        return Bicim::Dds;
    }
    if baslik.starts_with(b"#?RADIANCE") || baslik.starts_with(b"#?RGBE") {
        return Bicim::Hdr;
    }
    if pnm_mi(baslik) {
        return Bicim::Pnm;
    }
    if svg_mi(baslik) {
        return Bicim::Svg;
    }
    if isobmff_mi(baslik) {
        return isobmff_markasina_gore(baslik);
    }
    if baslik.starts_with(&TIFF_LE) || baslik.starts_with(&TIFF_BE) {
        return tiff_tabanli(baslik, uzanti.as_deref());
    }
    // Sihirli baytı olmayan biçimler yalnızca uzantıyla ayırt edilir.
    match uzanti.as_deref() {
        Some("tga") | Some("icb") | Some("vda") | Some("vst") => Bicim::Tga,
        Some("svg") | Some("svgz") => Bicim::Svg,
        Some("hdr") | Some("pic") => Bicim::Hdr,
        Some("exr") => Bicim::Exr,
        Some("jxl") => Bicim::Jxl,
        Some("avif") => Bicim::Avif,
        Some("heic") | Some("heif") => Bicim::Heic,
        Some("pnm") | Some("pgm") | Some("ppm") | Some("pbm") | Some("pam") => Bicim::Pnm,
        Some("qoi") => Bicim::Qoi,
        Some("dds") => Bicim::Dds,
        Some("tif") | Some("tiff") => Bicim::Tiff,
        _ => Bicim::Bilinmeyen,
    }
}

/// Dizin taramasında listelenecek uzantı mı?
pub fn uzanti_desteklenir(uzanti: &str) -> bool {
    matches!(
        kucuk_ascii(uzanti).as_str(),
        "jpg"
            | "jpeg"
            | "jpe"
            | "jfif"
            | "png"
            | "apng"
            | "webp"
            | "gif"
            | "bmp"
            | "dib"
            | "tga"
            | "icb"
            | "vda"
            | "vst"
            | "ico"
            | "cur"
            | "tif"
            | "tiff"
            | "avif"
            | "heic"
            | "heif"
            | "jxl"
            | "exr"
            | "hdr"
            | "pic"
            | "svg"
            | "svgz"
            | "pnm"
            | "pgm"
            | "ppm"
            | "pbm"
            | "pam"
            | "qoi"
            | "dds"
            | "ff"
            | "cr2"
            | "cr3"
            | "nef"
            | "arw"
            | "dng"
            | "orf"
            | "rw2"
            | "raf"
            | "pef"
            | "srw"
    )
}

/// Biçimin kullanıcıya gösterilecek kısa adı.
pub fn bicim_adi(bicim: Bicim) -> &'static str {
    match bicim {
        Bicim::Jpeg => "JPEG",
        Bicim::Png => "PNG",
        Bicim::WebP => "WebP",
        Bicim::Gif => "GIF",
        Bicim::Bmp => "BMP",
        Bicim::Tga => "TGA",
        Bicim::Ico => "ICO",
        Bicim::Tiff => "TIFF",
        Bicim::Avif => "AVIF",
        Bicim::Heic => "HEIC",
        Bicim::Jxl => "JPEG XL",
        Bicim::Exr => "OpenEXR",
        Bicim::Hdr => "Radiance HDR",
        Bicim::Svg => "SVG",
        Bicim::Pnm => "PNM",
        Bicim::Qoi => "QOI",
        Bicim::Dds => "DDS",
        Bicim::KameraHam(_) => "Kamera RAW",
        Bicim::Bilinmeyen => "Bilinmeyen",
    }
}

/// Biçim yüksek dinamik aralıklı (f32) veri taşır mı?
pub fn hdr_mi(bicim: Bicim) -> bool {
    matches!(bicim, Bicim::Hdr | Bicim::Exr)
}

fn pnm_mi(baslik: &[u8]) -> bool {
    if baslik.len() < 3 || baslik[0] != b'P' {
        return false;
    }
    matches!(baslik[1], b'1'..=b'6')
        && (baslik[2].is_ascii_whitespace() || baslik[2] == b'#' || baslik.len() > 3)
}

fn svg_mi(baslik: &[u8]) -> bool {
    // BOM ve baştaki boşluklar atlanır, ilk 512 bayt içinde <svg veya <?xml aranır.
    let pencere = &baslik[..baslik.len().min(512)];
    let metin = String::from_utf8_lossy(pencere);
    let temiz = metin.trim_start_matches('\u{feff}').trim_start();
    temiz.starts_with("<svg") || (temiz.starts_with("<?xml") && temiz.contains("<svg"))
}

fn isobmff_mi(baslik: &[u8]) -> bool {
    baslik.len() >= 12 && &baslik[4..8] == b"ftyp"
}

fn isobmff_markasina_gore(baslik: &[u8]) -> Bicim {
    let marka = &baslik[8..12];
    match marka {
        b"avif" | b"avis" => Bicim::Avif,
        b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1" => Bicim::Heic,
        b"crx " => Bicim::KameraHam(HamTuru::Cr3),
        _ => Bicim::Bilinmeyen,
    }
}

/// TIFF tabanlı dosyalar: CR2 dışındaki RAW türleri yalnızca uzantı ile ayrışır.
fn tiff_tabanli(baslik: &[u8], uzanti: Option<&str>) -> Bicim {
    // Canon CR2, TIFF başlığından sonra 8. konumda "CR" imzası taşır.
    if baslik.len() >= 10 && &baslik[8..10] == b"CR" {
        return Bicim::KameraHam(HamTuru::Cr2);
    }
    match uzanti {
        Some("cr2") => Bicim::KameraHam(HamTuru::Cr2),
        Some("nef") | Some("nrw") => Bicim::KameraHam(HamTuru::Nef),
        Some("arw") | Some("srf") | Some("sr2") => Bicim::KameraHam(HamTuru::Arw),
        Some("dng") => Bicim::KameraHam(HamTuru::Dng),
        Some("orf") => Bicim::KameraHam(HamTuru::Orf),
        Some("rw2") | Some("raw") => Bicim::KameraHam(HamTuru::Rw2),
        Some("pef") | Some("ptx") => Bicim::KameraHam(HamTuru::Pef),
        Some("srw") => Bicim::KameraHam(HamTuru::Srw),
        Some("cr3") => Bicim::KameraHam(HamTuru::Cr3),
        Some("raf") => Bicim::KameraHam(HamTuru::Raf),
        Some("nefx") | Some("arwx") => Bicim::KameraHam(HamTuru::BilinmeyenHam),
        _ => Bicim::Tiff,
    }
}

fn kucuk_ascii(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod testler {
    use super::*;

    fn bas(sihir: &[u8]) -> Vec<u8> {
        let mut v = sihir.to_vec();
        v.extend_from_slice(&[0u8; 32]);
        v
    }

    #[test]
    fn bilinen_sihirli_baytlar_taninir() {
        assert_eq!(tespit(&bas(&PNG_ON_EK), None), Bicim::Png);
        assert_eq!(tespit(&bas(&JPEG_ON_EK), None), Bicim::Jpeg);
        assert_eq!(tespit(&bas(b"GIF89a"), None), Bicim::Gif);
        assert_eq!(tespit(&bas(b"BM"), None), Bicim::Bmp);
        assert_eq!(tespit(&bas(&EXR_ON_EK), None), Bicim::Exr);
        assert_eq!(tespit(&bas(&JXL_IMZA), None), Bicim::Jxl);
        assert_eq!(tespit(&bas(&JXL_KUTU), None), Bicim::Jxl);
        assert_eq!(tespit(&bas(b"#?RADIANCE"), None), Bicim::Hdr);
        assert_eq!(tespit(&bas(b"qoif"), None), Bicim::Qoi);
        assert_eq!(tespit(&bas(b"DDS "), None), Bicim::Dds);
        assert_eq!(tespit(&bas(b"FUJIFILMCCD-RAW"), None), Bicim::KameraHam(HamTuru::Raf));
    }

    #[test]
    fn webp_riff_sarmali_taninir() {
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&[0x24, 0x00, 0x00, 0x00]);
        v.extend_from_slice(b"WEBPVP8 ");
        v.extend_from_slice(&[0u8; 16]);
        assert_eq!(tespit(&v, None), Bicim::WebP);
        // RIFF ama WEBP değil (ör. WAV) → bilinmeyen.
        let mut w = b"RIFF".to_vec();
        w.extend_from_slice(&[0x24, 0x00, 0x00, 0x00]);
        w.extend_from_slice(b"WAVEfmt ");
        assert_eq!(tespit(&w, None), Bicim::Bilinmeyen);
    }

    #[test]
    fn isobmff_markalari_ayrisir() {
        let insa = |marka: &[u8; 4]| {
            let mut v = vec![0u8, 0, 0, 0x20];
            v.extend_from_slice(b"ftyp");
            v.extend_from_slice(marka);
            v.extend_from_slice(&[0u8; 16]);
            v
        };
        assert_eq!(tespit(&insa(b"avif"), None), Bicim::Avif);
        assert_eq!(tespit(&insa(b"avis"), None), Bicim::Avif);
        assert_eq!(tespit(&insa(b"heic"), None), Bicim::Heic);
        assert_eq!(tespit(&insa(b"crx "), None), Bicim::KameraHam(HamTuru::Cr3));
    }

    #[test]
    fn cr2_tiff_imzasiyla_ayrisir() {
        let mut v = TIFF_LE.to_vec();
        v.extend_from_slice(&[0x10, 0x00, 0x00, 0x00]);
        v.extend_from_slice(b"CR");
        v.extend_from_slice(&[0x02, 0x00]);
        assert_eq!(tespit(&v, None), Bicim::KameraHam(HamTuru::Cr2));
    }

    #[test]
    fn tiff_uzanti_ile_raw_ayrisir() {
        let tiff = bas(&TIFF_LE);
        assert_eq!(tespit(&tiff, Some("tif")), Bicim::Tiff);
        assert_eq!(tespit(&tiff, Some("NEF")), Bicim::KameraHam(HamTuru::Nef));
        assert_eq!(tespit(&tiff, Some("arw")), Bicim::KameraHam(HamTuru::Arw));
        assert_eq!(tespit(&tiff, Some("dng")), Bicim::KameraHam(HamTuru::Dng));
        assert_eq!(tespit(&bas(&TIFF_BE), Some("tiff")), Bicim::Tiff);
    }

    #[test]
    fn svg_metin_olarak_taninir() {
        assert_eq!(tespit(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>", None), Bicim::Svg);
        assert_eq!(
            tespit(b"<?xml version=\"1.0\"?>\n<svg width=\"10\"></svg>", None),
            Bicim::Svg
        );
        assert_eq!(tespit(b"\xef\xbb\xbf<svg></svg>", None), Bicim::Svg);
    }

    #[test]
    fn pnm_taninir() {
        assert_eq!(tespit(b"P6\n100 100\n255\n", None), Bicim::Pnm);
        assert_eq!(tespit(b"P3\n", None), Bicim::Pnm);
    }

    #[test]
    fn sihirli_bayti_olmayanlar_uzantiyla_bulunur() {
        let bos = vec![0u8; 32];
        assert_eq!(tespit(&bos, Some("tga")), Bicim::Tga);
        assert_eq!(tespit(&bos, Some("TGA")), Bicim::Tga);
        assert_eq!(tespit(&bos, Some("hdr")), Bicim::Hdr);
        assert_eq!(tespit(&bos, Some("xyz")), Bicim::Bilinmeyen);
        assert_eq!(tespit(&bos, None), Bicim::Bilinmeyen);
    }

    #[test]
    fn kisa_baslik_panik_yapmaz() {
        for uzunluk in 0..8 {
            let v = vec![0xFFu8; uzunluk];
            let _ = tespit(&v, Some("jpg"));
        }
        assert_eq!(tespit(&[], None), Bicim::Bilinmeyen);
    }

    #[test]
    fn uzanti_destegi_dogru() {
        assert!(uzanti_desteklenir("JPG"));
        assert!(uzanti_desteklenir("avif"));
        assert!(uzanti_desteklenir("cr3"));
        assert!(uzanti_desteklenir("jxl"));
        assert!(!uzanti_desteklenir("txt"));
        assert!(!uzanti_desteklenir("mp4"));
        assert!(!uzanti_desteklenir(""));
    }

    #[test]
    fn hdr_bicimleri_isaretli() {
        assert!(hdr_mi(Bicim::Hdr));
        assert!(hdr_mi(Bicim::Exr));
        assert!(!hdr_mi(Bicim::Jpeg));
        assert!(!hdr_mi(Bicim::Tiff));
    }
}
