//! Meta veri: EXIF yönelim kodu çözümleme, yönelim dönüşümlerinin uygulanması ve
//! HUD'da gösterilecek özet bilgiler. EXIF ayrıştırma bağımlılık eklemeden,
//! TIFF/IFD0 yapısının yalnızca gereken kısmı okunarak yapılır.

use std::path::{Path, PathBuf};

use super::bicim::Bicim;

/// EXIF arama penceresi: EXIF verisi JPEG APP1 segmentinde ilk 64 KB içindedir.
const EXIF_ARAMA_PENCERESI: usize = 64 * 1024;

/// EXIF Orientation etiketi.
const ETIKET_YONELIM: u16 = 0x0112;

/// IFD girdi boyutu (bayt).
const IFD_GIRDI_BOYU: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endian {
    Kucuk,
    Buyuk,
}

impl Endian {
    fn u16_oku(self, b: &[u8], ofset: usize) -> Option<u16> {
        let dilim = b.get(ofset..ofset + 2)?;
        Some(match self {
            Self::Kucuk => u16::from_le_bytes([dilim[0], dilim[1]]),
            Self::Buyuk => u16::from_be_bytes([dilim[0], dilim[1]]),
        })
    }

    fn u32_oku(self, b: &[u8], ofset: usize) -> Option<u32> {
        let dilim = b.get(ofset..ofset + 4)?;
        let d = [dilim[0], dilim[1], dilim[2], dilim[3]];
        Some(match self {
            Self::Kucuk => u32::from_le_bytes(d),
            Self::Buyuk => u32::from_be_bytes(d),
        })
    }
}

/// Dosya baytlarından EXIF yönelim kodunu (1-8) çıkarır.
///
/// Bulunamazsa `None` döner; çağıran taraf `1` (dönüşüm yok) varsaymalıdır.
pub fn exif_yonelim(baytlar: &[u8]) -> Option<u8> {
    let tiff = tiff_baslangici(baytlar)?;
    let govde = baytlar.get(tiff..)?;
    if govde.len() < 8 {
        return None;
    }
    let endian = match &govde[0..2] {
        b"II" => Endian::Kucuk,
        b"MM" => Endian::Buyuk,
        _ => return None,
    };
    if endian.u16_oku(govde, 2)? != 42 {
        return None;
    }
    let ifd0 = endian.u32_oku(govde, 4)? as usize;
    let girdi_sayisi = endian.u16_oku(govde, ifd0)? as usize;
    // Girdiler IFD başlığından sonra 2 bayt sonra başlar.
    let temel = ifd0.checked_add(2)?;
    for i in 0..girdi_sayisi {
        let ofset = temel.checked_add(i.checked_mul(IFD_GIRDI_BOYU)?)?;
        let etiket = endian.u16_oku(govde, ofset)?;
        if etiket != ETIKET_YONELIM {
            continue;
        }
        let tip = endian.u16_oku(govde, ofset + 2)?;
        let adet = endian.u32_oku(govde, ofset + 4)?;
        // Orientation tek bir SHORT (tip 3) değeridir ve değer alanına satır içi yazılır.
        if tip != 3 || adet != 1 {
            return None;
        }
        let deger = endian.u16_oku(govde, ofset + 8)?;
        return match deger {
            1..=8 => Some(deger as u8),
            _ => None,
        };
    }
    None
}

/// TIFF başlığının dosya içindeki başlangıcını bulur.
fn tiff_baslangici(baytlar: &[u8]) -> Option<usize> {
    if baytlar.starts_with(b"II\x2A\x00") || baytlar.starts_with(b"MM\x00\x2A") {
        return Some(0);
    }
    let pencere = &baytlar[..baytlar.len().min(EXIF_ARAMA_PENCERESI)];
    let imza = b"Exif\x00\x00";
    pencere
        .windows(imza.len())
        .position(|p| p == imza)
        .map(|i| i + imza.len())
}

/// Yönelim kodunu insan okunur Türkçe etikete çevirir (HUD için).
pub fn yonelim_adi(kod: u8) -> &'static str {
    match kod {
        2 => "Yatay ayna",
        3 => "180° döndürülmüş",
        4 => "Dikey ayna",
        5 => "Devrik (transpose)",
        6 => "90° saat yönü",
        7 => "Ters devrik",
        8 => "90° saat yönü tersi",
        _ => "Normal",
    }
}

/// Yönelim kodunu RGBA8 tamponuna uygular.
///
/// Kod 1 veya geçersizse `None` döner (dönüşüm gerekmez); aksi halde yeni tampon,
/// yeni genişlik ve yükseklik döner. 5-8 kodlarında eksenler yer değiştirir.
pub fn yonelim_uygula(
    rgba: &[u8],
    genislik: u32,
    yukseklik: u32,
    kod: u8,
) -> Option<(Vec<u8>, u32, u32)> {
    if !(2..=8).contains(&kod) {
        return None;
    }
    let g = genislik as usize;
    let y = yukseklik as usize;
    if g == 0 || y == 0 || rgba.len() < g * y * 4 {
        return None;
    }
    let eksen_degisir = matches!(kod, 5..=8);
    let (yeni_g, yeni_y) = if eksen_degisir { (y, g) } else { (g, y) };
    let mut hedef = vec![0u8; yeni_g * yeni_y * 4];

    for dy in 0..yeni_y {
        for dx in 0..yeni_g {
            // Hedef (dx, dy) pikselinin kaynak koordinatı.
            let (sx, sy) = match kod {
                2 => (g - 1 - dx, dy),
                3 => (g - 1 - dx, y - 1 - dy),
                4 => (dx, y - 1 - dy),
                5 => (dy, dx),
                6 => (dy, y.wrapping_sub(1).wrapping_sub(dx)),
                7 => (g - 1 - dy, y - 1 - dx),
                8 => (g.wrapping_sub(1).wrapping_sub(dy), dx),
                _ => (dx, dy),
            };
            if sx >= g || sy >= y {
                continue;
            }
            let kaynak = (sy * g + sx) * 4;
            let hedef_indis = (dy * yeni_g + dx) * 4;
            hedef[hedef_indis..hedef_indis + 4].copy_from_slice(&rgba[kaynak..kaynak + 4]);
        }
    }
    Some((hedef, yeni_g as u32, yeni_y as u32))
}

/// HUD meta veri panelinde gösterilen özet.
#[derive(Debug, Clone)]
pub struct MetaBilgi {
    pub dosya_adi: String,
    pub yol: PathBuf,
    pub bicim: Bicim,
    /// Dosyadaki ham piksel ölçüsü.
    pub ham_genislik: u32,
    pub ham_yukseklik: u32,
    /// Yönelim uygulandıktan sonraki gösterim ölçüsü.
    pub genislik: u32,
    pub yukseklik: u32,
    pub dosya_boyutu: u64,
    pub icc_var: bool,
    /// ICC dönüşümü gerçekten uygulandı mı.
    pub icc_uygulandi: bool,
    /// CPU'da ön küçültme yapıldı mı.
    pub kucultuldu: bool,
    pub hdr: bool,
    pub yonelim: u8,
}

impl MetaBilgi {
    /// Dosya sisteminden boyut okur; okunamazsa 0 döner (gösterim engellenmez).
    pub fn dosya_boyutu_oku(yol: &Path) -> u64 {
        std::fs::metadata(yol).map(|m| m.len()).unwrap_or(0)
    }

    /// Çözünürlük metni: 1920×1080
    pub fn cozunurluk_metni(&self) -> String {
        format!("{}×{}", self.genislik, self.yukseklik)
    }

    /// Dosya boyutunu okunabilir biçime çevirir (1,4 MB gibi).
    pub fn boyut_metni(&self) -> String {
        bayt_bicimle(self.dosya_boyutu)
    }

    /// Toplam piksel sayısı (megapiksel metni için).
    pub fn megapiksel_metni(&self) -> String {
        let mp = f64::from(self.genislik) * f64::from(self.yukseklik) / 1_000_000.0;
        format!("{mp:.1} MP")
    }
}

/// Bayt değerini Türkçe ondalık ayracıyla okunabilir biçime çevirir.
pub fn bayt_bicimle(bayt: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bayt as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB).replace('.', ",")
    } else if b >= MB {
        format!("{:.1} MB", b / MB).replace('.', ",")
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bayt} bayt")
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Belirtilen yönelim koduna sahip minimal bir EXIF/TIFF bloğu üretir.
    fn sahte_exif(kod: u16, endian: Endian) -> Vec<u8> {
        let mut v = Vec::new();
        match endian {
            Endian::Kucuk => {
                v.extend_from_slice(b"II");
                v.extend_from_slice(&42u16.to_le_bytes());
                v.extend_from_slice(&8u32.to_le_bytes());
                // IFD0: 1 girdi
                v.extend_from_slice(&1u16.to_le_bytes());
                v.extend_from_slice(&ETIKET_YONELIM.to_le_bytes());
                v.extend_from_slice(&3u16.to_le_bytes()); // SHORT
                v.extend_from_slice(&1u32.to_le_bytes()); // adet
                v.extend_from_slice(&kod.to_le_bytes());
                v.extend_from_slice(&[0u8, 0u8]);
                v.extend_from_slice(&0u32.to_le_bytes()); // sonraki IFD yok
            }
            Endian::Buyuk => {
                v.extend_from_slice(b"MM");
                v.extend_from_slice(&42u16.to_be_bytes());
                v.extend_from_slice(&8u32.to_be_bytes());
                v.extend_from_slice(&1u16.to_be_bytes());
                v.extend_from_slice(&ETIKET_YONELIM.to_be_bytes());
                v.extend_from_slice(&3u16.to_be_bytes());
                v.extend_from_slice(&1u32.to_be_bytes());
                v.extend_from_slice(&kod.to_be_bytes());
                v.extend_from_slice(&[0u8, 0u8]);
                v.extend_from_slice(&0u32.to_be_bytes());
            }
        }
        v
    }

    #[test]
    fn jpeg_app1_icindeki_exif_okunur() {
        let mut dosya = vec![0xFF, 0xD8, 0xFF, 0xE1]; // SOI + APP1
        dosya.extend_from_slice(&[0x00, 0x20]); // uzunluk
        dosya.extend_from_slice(b"Exif\x00\x00");
        dosya.extend_from_slice(&sahte_exif(6, Endian::Kucuk));
        assert_eq!(exif_yonelim(&dosya), Some(6));
    }

    #[test]
    fn tiff_dosyasindan_dogrudan_okunur() {
        let tiff = sahte_exif(8, Endian::Buyuk);
        assert_eq!(exif_yonelim(&tiff), Some(8));
    }

    #[test]
    fn exif_yoksa_none() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        assert_eq!(exif_yonelim(&png), None);
        assert_eq!(exif_yonelim(&[]), None);
    }

    #[test]
    fn gecersiz_kod_yok_sayilir() {
        let mut dosya = vec![0xFF, 0xD8];
        dosya.extend_from_slice(&sahte_exif(99, Endian::Kucuk));
        assert_eq!(exif_yonelim(&dosya), None);
    }

    #[test]
    fn bozuk_veri_panik_yapmaz() {
        // Kesilmiş IFD: girdi sayısı büyük, veri yok.
        let mut v = b"II".to_vec();
        v.extend_from_slice(&42u16.to_le_bytes());
        v.extend_from_slice(&8u32.to_le_bytes());
        v.extend_from_slice(&500u16.to_le_bytes());
        assert_eq!(exif_yonelim(&v), None);
        // Rastgele baytlar.
        assert_eq!(exif_yonelim(&[0u8; 32]), None);
    }

    /// 2x1 görüntü: sol kırmızı, sağ mavi. Dönüşümleri elle doğrularız.
    fn iki_piksel() -> Vec<u8> {
        vec![255, 0, 0, 255, 0, 0, 255, 255]
    }

    #[test]
    fn yonelim_1_donusum_gerektirmez() {
        assert!(yonelim_uygula(&iki_piksel(), 2, 1, 1).is_none());
    }

    #[test]
    fn kod_2_yatay_ayna() {
        let (v, g, y) = yonelim_uygula(&iki_piksel(), 2, 1, 2).expect("dönüşmeli");
        assert_eq!((g, y), (2, 1));
        assert_eq!(&v[0..4], &[0, 0, 255, 255], "önce mavi gelmeli");
        assert_eq!(&v[4..8], &[255, 0, 0, 255]);
    }

    #[test]
    fn kod_3_180_dondurme() {
        let (v, g, y) = yonelim_uygula(&iki_piksel(), 2, 1, 3).expect("dönüşmeli");
        assert_eq!((g, y), (2, 1));
        assert_eq!(&v[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn kod_6_90_derece_eksen_degistirir() {
        let (v, g, y) = yonelim_uygula(&iki_piksel(), 2, 1, 6).expect("dönüşmeli");
        assert_eq!((g, y), (1, 2), "eksenler yer değiştirmeli");
        // 90° saat yönü: sol (kırmızı) üste, sağ (mavi) alta gelir.
        assert_eq!(&v[0..4], &[255, 0, 0, 255]);
        assert_eq!(&v[4..8], &[0, 0, 255, 255]);
    }

    #[test]
    fn kod_8_270_derece_eksen_degistirir() {
        let (v, g, y) = yonelim_uygula(&iki_piksel(), 2, 1, 8).expect("dönüşmeli");
        assert_eq!((g, y), (1, 2));
        // 270° saat yönü: sol (kırmızı) alta, sağ (mavi) üste gelir.
        assert_eq!(&v[0..4], &[0, 0, 255, 255]);
        assert_eq!(&v[4..8], &[255, 0, 0, 255]);
    }

    #[test]
    fn yonelim_tum_pikselleri_korur() {
        // 3x2 görüntüde her piksel benzersiz; dönüşüm sonrası küme aynı kalmalı.
        let mut kaynak = Vec::new();
        for i in 0..6u8 {
            kaynak.extend_from_slice(&[i, i, i, 255]);
        }
        for kod in 2..=8u8 {
            let (v, g, y) = yonelim_uygula(&kaynak, 3, 2, kod).expect("dönüşmeli");
            assert_eq!(v.len(), kaynak.len(), "kod {kod}: boyut korunmalı");
            assert_eq!(g * y, 6, "kod {kod}: piksel sayısı korunmalı");
            let mut a: Vec<u8> = (0..6u8).collect();
            let mut b: Vec<u8> = v.chunks(4).map(|p| p[0]).collect();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "kod {kod}: piksel kümesi korunmalı");
        }
    }

    #[test]
    fn eksik_tampon_none_dondurur() {
        assert!(yonelim_uygula(&[0u8; 4], 4, 4, 6).is_none());
        assert!(yonelim_uygula(&[], 0, 0, 3).is_none());
    }

    #[test]
    fn boyut_metni_turkce_ondalik() {
        assert_eq!(bayt_bicimle(512), "512 bayt");
        assert_eq!(bayt_bicimle(2048), "2 KB");
        assert_eq!(bayt_bicimle(1024 * 1024 * 3 / 2), "1,5 MB");
        assert!(bayt_bicimle(2 * 1024 * 1024 * 1024).ends_with("GB"));
    }

    #[test]
    fn yonelim_adi_turkce() {
        assert_eq!(yonelim_adi(1), "Normal");
        assert_eq!(yonelim_adi(6), "90° saat yönü");
        assert_eq!(yonelim_adi(0), "Normal");
    }
}
