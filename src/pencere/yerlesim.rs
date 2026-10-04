//! Pencere yerleşimi kalıcılığı: `WINDOWPLACEMENT` yapısının diskte saklanması.
//!
//! Ham piksel koordinatları saklamak çoklu monitör ve farklı DPI düzenlerinde pencereyi
//! ekran dışına taşırır. Windows'un `WINDOWPLACEMENT` yapısı ise pencerenin normal haldeki
//! konumunu (`rcNormalPosition`) durumdan bağımsız olarak taşır; Aero Snap veya maksimize
//! durumda kapatılsa bile pencere doğru yerde açılır.

use std::path::Path;

use winit::window::Window;

use crate::cekirdek::hata::{GorselHatasi, Sonuc};
use crate::cekirdek::yerel;

/// Yerleşim dosyasının adı (%APPDATA%\Gorsel altında).
pub const DOSYA_ADI: &str = "pencere.bin";

/// Windows'un `WINDOWPLACEMENT` yapısının sürümlenmiş, sabit boyutlu ikili gösterimi.
///
/// Alanlar tek tek yazılır; yapının bellekteki dolgularına bağımlı olmak yerine
/// sürümden bağımsız, taşınabilir bir biçim elde edilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Yerlesim {
    /// Yapı sürüm uzunluğu (Win32 `length` alanı).
    pub uzunluk: u32,
    pub bayraklar: u32,
    /// Gösterim komutu (SW_SHOWNORMAL = 1, SW_SHOWMINIMIZED = 2, SW_SHOWMAXIMIZED = 3).
    pub gosterim_komutu: u32,
    pub min_konum: (i32, i32),
    pub maks_konum: (i32, i32),
    /// Normal (geri yüklenmiş) durumdaki pencere dikdörtgeni: sol, üst, sağ, alt.
    pub normal_dikdortgen: (i32, i32, i32, i32),
}

/// İkili gösterimin bayt uzunluğu: 3×u32 + 4×i32 (min/maks/dikdörtgen).
pub const YERLESIM_BAYT: usize = 3 * 4 + 2 * 8 + 16;

/// Win32 `WINDOWPLACEMENT.length` alanının beklenen değeri (64-bit sistemlerde).
pub const WIN32_YAPI_UZUNLUGU: u32 = 44;

impl Yerlesim {
    /// Baytlara serileştirir (küçük endian, alan sırası sabit).
    pub fn baytlara(&self) -> [u8; YERLESIM_BAYT] {
        let mut cikti = [0u8; YERLESIM_BAYT];
        let mut yaz = |ofset: usize, deger: i32| {
            cikti[ofset..ofset + 4].copy_from_slice(&deger.to_le_bytes());
        };
        yaz(0, self.uzunluk as i32);
        yaz(4, self.bayraklar as i32);
        yaz(8, self.gosterim_komutu as i32);
        yaz(12, self.min_konum.0);
        yaz(16, self.min_konum.1);
        yaz(20, self.maks_konum.0);
        yaz(24, self.maks_konum.1);
        yaz(28, self.normal_dikdortgen.0);
        yaz(32, self.normal_dikdortgen.1);
        yaz(36, self.normal_dikdortgen.2);
        yaz(40, self.normal_dikdortgen.3);
        cikti
    }

    /// Baytlardan çözer; boyut uyuşmazsa `None`.
    pub fn baytlardan(baytlar: &[u8]) -> Option<Self> {
        if baytlar.len() < YERLESIM_BAYT {
            return None;
        }
        let oku = |ofset: usize| -> i32 {
            i32::from_le_bytes([
                baytlar[ofset],
                baytlar[ofset + 1],
                baytlar[ofset + 2],
                baytlar[ofset + 3],
            ])
        };
        Some(Self {
            uzunluk: oku(0) as u32,
            bayraklar: oku(4) as u32,
            gosterim_komutu: oku(8) as u32,
            min_konum: (oku(12), oku(16)),
            maks_konum: (oku(20), oku(24)),
            normal_dikdortgen: (oku(28), oku(32), oku(36), oku(40)),
        })
    }

    /// Pencere dikdörtgeni geçerli mi (ekranda görülebilir bir alan var mı).
    pub fn gecerli_mi(&self) -> bool {
        let (sol, ust, sag, alt) = self.normal_dikdortgen;
        let genislik = sag - sol;
        let yukseklik = alt - ust;
        // Aşırı küçük veya saçma konumlanmış pencereler geri yüklenmez.
        genislik >= 100 && yukseklik >= 100 && genislik < 100_000 && yukseklik < 100_000
    }
}

/// Diskteki yerleşim dosyasının yolu.
pub fn dosya_yolu() -> Sonuc<std::path::PathBuf> {
    yerel::ayar_dosyasi(DOSYA_ADI)
}

/// Baytları diske yazar (önce geçici dosya, sonra taşıma).
pub fn baytlari_yaz(yol: &Path, baytlar: &[u8]) -> Sonuc<()> {
    if let Some(ust) = yol.parent() {
        yerel::dizini_hazirla(ust)?;
    }
    let gecici = yol.with_extension("bin.gecici");
    std::fs::write(&gecici, baytlar)
        .map_err(|k| GorselHatasi::Ayar(format!("yerleşim yazılamadı: {k}")))?;
    std::fs::rename(&gecici, yol)
        .map_err(|k| GorselHatasi::Ayar(format!("yerleşim taşınamadı: {k}")))?;
    Ok(())
}

/// Diskteki yerleşimi okur; dosya yoksa veya bozuksa `None` döner.
pub fn baytlari_oku(yol: &Path) -> Option<Vec<u8>> {
    if !yol.exists() {
        return None;
    }
    std::fs::read(yol).ok().filter(|b| b.len() >= YERLESIM_BAYT)
}

/// Aktif pencerenin yerleşimini diske kaydeder.
pub fn kaydet(pencere: &Window) -> Sonuc<()> {
    let yerlesim = pencere_yerlesimi(pencere)?;
    let yol = dosya_yolu()?;
    baytlari_yaz(&yol, &yerlesim.baytlara())
}

/// Diskteki yerleşimi pencereye uygular; kayıt yoksa `false` döner.
pub fn yukle(pencere: &Window) -> Sonuc<bool> {
    let yol = dosya_yolu()?;
    let Some(baytlar) = baytlari_oku(&yol) else {
        return Ok(false);
    };
    let Some(yerlesim) = Yerlesim::baytlardan(&baytlar) else {
        return Ok(false);
    };
    if !yerlesim.gecerli_mi() {
        return Ok(false);
    }
    pencereyi_yerlestir(pencere, &yerlesim)?;
    Ok(true)
}

/// Pencerenin mevcut yerleşimini Win32'den okur.
pub fn pencere_yerlesimi(pencere: &Window) -> Sonuc<Yerlesim> {
    #[cfg(windows)]
    {
        win32::oku(pencere)
    }
    #[cfg(not(windows))]
    {
        // Diğer platformlarda pencere sisteminin kendi konum verisi kullanılır.
        let konum = pencere.outer_position().map_err(|k| {
            GorselHatasi::Pencere(format!("pencere konumu okunamadı: {k}"))
        })?;
        let olcu = pencere.outer_size().map_err(|k| {
            GorselHatasi::Pencere(format!("pencere ölçüsü okunamadı: {k}"))
        })?;
        Ok(Yerlesim {
            uzunluk: WIN32_YAPI_UZUNLUGU,
            bayraklar: 0,
            gosterim_komutu: 1,
            min_konum: (0, 0),
            maks_konum: (0, 0),
            normal_dikdortgen: (
                konum.x,
                konum.y,
                konum.x + olcu.width as i32,
                konum.y + olcu.height as i32,
            ),
        })
    }
}

/// Yerleşimi pencereye uygular (pencere görünür kılınmadan önce çağrılmalıdır).
pub fn pencereyi_yerlestir(pencere: &Window, yerlesim: &Yerlesim) -> Sonuc<()> {
    #[cfg(windows)]
    {
        win32::uygula(pencere, yerlesim)
    }
    #[cfg(not(windows))]
    {
        let (sol, ust, sag, alt) = yerlesim.normal_dikdortgen;
        pencere.set_outer_position(winit::dpi::PhysicalPosition::new(sol, ust));
        let _ = pencere.request_inner_size(winit::dpi::PhysicalSize::new(
            (sag - sol).max(1) as u32,
            (alt - ust).max(1) as u32,
        ));
        Ok(())
    }
}

#[cfg(windows)]
mod win32 {
    use super::*;

    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, SetWindowPlacement, WINDOWPLACEMENT,
    };

    /// Pencere tanıtıcısı ortak yardımcıdan alınır (tek noktada toplanmıştır).
    fn hwnd(pencere: &Window) -> Sonuc<windows::Win32::Foundation::HWND> {
        crate::pencere::tanitici(pencere)
    }

    /// Win32 `WINDOWPLACEMENT` yapısını kendi taşınabilir gösterimimize çevirir.
    fn donustur(wp: &WINDOWPLACEMENT) -> Yerlesim {
        Yerlesim {
            uzunluk: wp.length,
            bayraklar: wp.flags.0,
            gosterim_komutu: wp.showCmd,
            min_konum: (wp.ptMinPosition.x, wp.ptMinPosition.y),
            maks_konum: (wp.ptMaxPosition.x, wp.ptMaxPosition.y),
            normal_dikdortgen: (
                wp.rcNormalPosition.left,
                wp.rcNormalPosition.top,
                wp.rcNormalPosition.right,
                wp.rcNormalPosition.bottom,
            ),
        }
    }

    fn geri_donustur(y: &Yerlesim) -> WINDOWPLACEMENT {
        let mut wp = WINDOWPLACEMENT {
            length: WIN32_YAPI_UZUNLUGU,
            ..Default::default()
        };
        wp.flags = windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT_FLAGS(y.bayraklar);
        wp.showCmd = y.gosterim_komutu;
        wp.ptMinPosition.x = y.min_konum.0;
        wp.ptMinPosition.y = y.min_konum.1;
        wp.ptMaxPosition.x = y.maks_konum.0;
        wp.ptMaxPosition.y = y.maks_konum.1;
        wp.rcNormalPosition.left = y.normal_dikdortgen.0;
        wp.rcNormalPosition.top = y.normal_dikdortgen.1;
        wp.rcNormalPosition.right = y.normal_dikdortgen.2;
        wp.rcNormalPosition.bottom = y.normal_dikdortgen.3;
        wp
    }

    pub fn oku(pencere: &Window) -> Sonuc<Yerlesim> {
        let tanitici = hwnd(pencere)?;
        let mut wp = WINDOWPLACEMENT {
            length: WIN32_YAPI_UZUNLUGU,
            ..Default::default()
        };
        unsafe { GetWindowPlacement(tanitici, &mut wp) }
            .map_err(|k| GorselHatasi::Pencere(format!("pencere yerleşimi okunamadı: {k}")))?;
        Ok(donustur(&wp))
    }

    pub fn uygula(pencere: &Window, yerlesim: &Yerlesim) -> Sonuc<()> {
        let tanitici = hwnd(pencere)?;
        let wp = geri_donustur(yerlesim);
        unsafe { SetWindowPlacement(tanitici, &wp) }
            .map_err(|k| GorselHatasi::Pencere(format!("pencere yerleşimi uygulanamadı: {k}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek() -> Yerlesim {
        Yerlesim {
            uzunluk: WIN32_YAPI_UZUNLUGU,
            bayraklar: 0,
            gosterim_komutu: 3,
            min_konum: (-32000, -32000),
            maks_konum: (-8, -8),
            normal_dikdortgen: (100, 50, 1380, 830),
        }
    }

    #[test]
    fn serilestirme_gidis_donus_korunur() {
        let y = ornek();
        let baytlar = y.baytlara();
        assert_eq!(baytlar.len(), YERLESIM_BAYT);
        let geri = Yerlesim::baytlardan(&baytlar).expect("çözülmeli");
        assert_eq!(geri, y);
    }

    #[test]
    fn negatif_konumlar_dogru_saklanir() {
        let y = Yerlesim {
            min_konum: (-32000, -32000),
            ..ornek()
        };
        let geri = Yerlesim::baytlardan(&y.baytlara()).expect("çözülmeli");
        assert_eq!(geri.min_konum, (-32000, -32000));
    }

    #[test]
    fn eksik_bayt_none_dondurur() {
        assert!(Yerlesim::baytlardan(&[0u8; 10]).is_none());
        assert!(Yerlesim::baytlardan(&[]).is_none());
    }

    #[test]
    fn gecerlilik_denetimi() {
        assert!(ornek().gecerli_mi());
        // Çok küçük pencere geri yüklenmez.
        let kucuk = Yerlesim {
            normal_dikdortgen: (0, 0, 50, 50),
            ..ornek()
        };
        assert!(!kucuk.gecerli_mi());
        // Aşırı büyük değerler reddedilir.
        let dev = Yerlesim {
            normal_dikdortgen: (0, 0, 200_000, 200_000),
            ..ornek()
        };
        assert!(!dev.gecerli_mi());
    }

    #[test]
    fn dosya_yaz_oku_dongusu() {
        let dizin = std::env::temp_dir().join("gorsel-yerlesim-testi");
        let _ = std::fs::remove_dir_all(&dizin);
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = dizin.join("pencere.bin");

        let y = ornek();
        baytlari_yaz(&yol, &y.baytlara()).expect("yazılmalı");
        let okunan = baytlari_oku(&yol).expect("okunmalı");
        assert_eq!(Yerlesim::baytlardan(&okunan), Some(y));

        let _ = std::fs::remove_dir_all(&dizin);
    }

    #[test]
    fn olmayan_dosya_none() {
        let yol = std::env::temp_dir().join("gorsel-yok-pencere.bin");
        let _ = std::fs::remove_file(&yol);
        assert!(baytlari_oku(&yol).is_none());
    }

    #[test]
    fn bozuk_dosya_none() {
        let dizin = std::env::temp_dir().join("gorsel-bozuk-yerlesim");
        let _ = std::fs::remove_dir_all(&dizin);
        std::fs::create_dir_all(&dizin).expect("dizin");
        let yol = dizin.join("pencere.bin");
        std::fs::write(&yol, b"kisa").expect("yazılmalı");
        assert!(baytlari_oku(&yol).is_none(), "kısa dosya reddedilmeli");
        let _ = std::fs::remove_dir_all(&dizin);
    }
}
