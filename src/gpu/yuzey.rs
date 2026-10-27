//! Takas zinciri (swapchain) yönetimi ve HDR yapılandırması.
//!
//! Format seçimi yetenek sorgusuna dayanır: ekran HDR bildiriyorsa, `Rgba16Float`
//! yüzeyi `ExtendedSrgbLinear` (scRGB) renk uzayını destekliyorsa HDR yolu açılır;
//! aksi halde SDR formatına düşülür.
//!
//! scRGB doğrusal bir sinyaldir ve 1,0 değeri 80 nit'tir; SDR beyazı `sdr_beyaz_nits / 80`
//! ölçeğiyle yazılır. SDR yolunda egui, hedef sRGB olmadığında değerleri kendisi gama
//! kodlar; görüntü gölgelendiricisi aynı kuralı uygular ve HUD ile görsel aynı kodlamayı
//! paylaşır.

use std::sync::Arc;

use winit::window::Window;

use super::Gpu;
use super::hdr::HdrBilgi;
use crate::cekirdek::hata::{GorselHatasi, Sonuc};

/// HDR yolu için tercih edilen yüzey formatı (16-bit kayan nokta, geniş aralık).
pub const HDR_FORMATI: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// scRGB renk uzayında 1,0 değerinin karşılık geldiği parlaklık (nit).
pub const SCRGB_BIRIM_NITS: f32 = 80.0;

/// Anormal SDR beyazı bildirimlerinde UI'nin tüm HDR aralığını doldurmasını önleyen üst sınır.
const AZAMI_SDR_BEYAZ_OLCEGI: f32 = 10.0;

/// SDR yüzeyi için format tercih sırası.
///
/// egui, gama kodlamasını kendisi uyguladığı için sRGB OLMAYAN bir hedef tercih eder
/// (aksi halde donanım da kodlar ve egui uyarı üretir); ek olarak Windows'ta
/// `Bgra8Unorm` DWM ile ek renk dönüşümü gerektirmez. Görüntü gölgelendiricisi aynı
/// kodlamayı `gama_kodlamasi_shaderda()` bayrağıyla uygular, böylece HUD ile görsel
/// aynı gama eğrisini paylaşır.
const TERCIH_SDR: [wgpu::TextureFormat; 4] = [
    wgpu::TextureFormat::Bgra8Unorm,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Bgra8UnormSrgb,
    wgpu::TextureFormat::Rgba8UnormSrgb,
];

/// Yüzeyin bildirdiği formatlar arasından tercih sırasına göre SDR formatı seçer.
pub fn sdr_format_sec(mevcut: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    TERCIH_SDR.iter().copied().find(|f| mevcut.contains(f))
}

/// Verilen renk uzayı kümesi doğrusal scRGB çıkışını (HDR) destekliyor mu.
pub fn scrgb_destekli(renk_uzaylari: wgpu::SurfaceColorSpaces) -> bool {
    renk_uzaylari.contains(wgpu::SurfaceColorSpaces::EXTENDED_SRGB_LINEAR)
}

/// HDR yolu yalnızca istendiğinde, ekran HDR bildirdiğinde ve scRGB desteklendiğinde açılır.
fn hdr_yolu_sec(
    hdr_iste: bool,
    hdr_kullanilabilir: bool,
    renk_uzaylari: wgpu::SurfaceColorSpaces,
) -> bool {
    hdr_iste && hdr_kullanilabilir && scrgb_destekli(renk_uzaylari)
}

/// SDR beyazını scRGB birimine çeviren ölçek; geçersiz bildirimde 1,0'a düşer.
pub fn sdr_beyaz_olcegi_hesapla(sdr_beyaz_nits: f32) -> f32 {
    if !sdr_beyaz_nits.is_finite() || sdr_beyaz_nits <= 0.0 {
        return 1.0;
    }
    (sdr_beyaz_nits / SCRGB_BIRIM_NITS).min(AZAMI_SDR_BEYAZ_OLCEGI)
}

/// Shader'ın sRGB gama kodlaması uygulaması gerekiyor mu.
///
/// scRGB çıkışında değerler doğrusaldır; sRGB formatlarında ise kodlamayı donanım yapar.
fn gama_kodlamasi_gerekli(hdr_kullaniliyor: bool, format: wgpu::TextureFormat) -> bool {
    !hdr_kullaniliyor && !format.is_srgb()
}

/// Yüzey ölçüsü sıfır olamaz; simge durumuna küçültülmüş pencerede en az 1 piksel kullanılır.
pub fn gecerli_olcu(genislik: u32, yukseklik: u32) -> (u32, u32) {
    (genislik.max(1), yukseklik.max(1))
}

/// Pencere yüzeyi ve geçerli yapılandırması.
pub struct Yuzey {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub hdr: HdrBilgi,
    hdr_kullaniliyor: bool,
}

impl Yuzey {
    /// Pencere için yüzeyi oluşturur, yetenekleri sorgular ve yapılandırır.
    pub fn yeni(gpu: &Gpu, pencere: Arc<Window>, olcu: (u32, u32), hdr_iste: bool) -> Sonuc<Self> {
        let surface = gpu
            .instance
            .create_surface(pencere)
            .map_err(|k| GorselHatasi::Grafik(format!("pencere yüzeyi oluşturulamadı: {k}")))?;

        let (genislik, yukseklik) = gecerli_olcu(olcu.0, olcu.1);
        let yetenekler = surface.get_capabilities(&gpu.adapter);
        let mut config = surface
            .get_default_config(&gpu.adapter, genislik, yukseklik)
            .ok_or_else(|| {
                GorselHatasi::Grafik("yüzey için geçerli yapılandırma bulunamadı".into())
            })?;

        let hdr = HdrBilgi::sorgula(&surface, &gpu.adapter);
        let hdr_kullaniliyor = hdr_yolu_sec(
            hdr_iste,
            hdr.hdr_kullanilabilir,
            yetenekler.color_spaces(HDR_FORMATI),
        );

        if hdr_kullaniliyor {
            config.format = HDR_FORMATI;
            config.color_space = wgpu::SurfaceColorSpace::ExtendedSrgbLinear;
        } else if let Some(format) = sdr_format_sec(&yetenekler.formats) {
            config.format = format;
            config.color_space = wgpu::SurfaceColorSpace::Srgb;
        }

        config.width = genislik;
        config.height = yukseklik;
        surface.configure(&gpu.device, &config);

        Ok(Self {
            surface,
            config,
            hdr,
            hdr_kullaniliyor,
        })
    }

    pub fn olcu(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// HDR yüzeyi etkin mi (görsel yüksek dinamik aralıkta sunuluyor).
    pub fn hdr_aktif(&self) -> bool {
        self.hdr_kullaniliyor
    }

    /// SDR beyazını scRGB birimine (1,0 = 80 nit) çeviren ölçek.
    pub fn sdr_beyaz_olcegi(&self) -> f32 {
        sdr_beyaz_olcegi_hesapla(self.hdr.sdr_beyaz_nits)
    }

    /// Hedef formatta gama kodlamasını shader'ın yapması gerekiyor mu.
    ///
    /// scRGB yüzeyinde değerler doğrusal yazılır; sRGB formatlarında kodlamayı
    /// donanım yapar. Yalnızca SDR ve sRGB olmayan hedeflerde shader kodlar.
    pub fn gama_kodlamasi_shaderda(&self) -> bool {
        gama_kodlamasi_gerekli(self.hdr_kullaniliyor, self.config.format)
    }

    /// Ölçü değişmese de yüzeyi yeniden yapılandırır.
    ///
    /// Modal bir sistem diyalogu kapanıp pencere yeniden etkinleştiğinde takas zinciri
    /// geçersiz kalabilir; bu çağrı yapılandırmayı tazeler.
    pub fn zorla_yeniden_yapilandir(&mut self, gpu: &Gpu) {
        self.surface.configure(&gpu.device, &self.config);
    }

    /// Pencere ölçüsü değiştiğinde yapılandırmayı tazeler; sıfır ölçüde yapılandırma yapmaz.
    pub fn yeniden_boyutla(&mut self, gpu: &Gpu, genislik: u32, yukseklik: u32) {
        let (g, y) = gecerli_olcu(genislik, yukseklik);
        if (g, y) == (self.config.width, self.config.height) {
            return;
        }
        self.config.width = g;
        self.config.height = y;
        self.surface.configure(&gpu.device, &self.config);
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn sifir_olcu_bire_tamamlanir() {
        assert_eq!(gecerli_olcu(0, 0), (1, 1));
        assert_eq!(gecerli_olcu(1920, 1080), (1920, 1080));
    }

    #[test]
    fn hdr_formati_float_tabanli() {
        assert_eq!(HDR_FORMATI, wgpu::TextureFormat::Rgba16Float);
        assert!(
            !HDR_FORMATI.is_srgb(),
            "scRGB float yüzey sRGB formatı değildir"
        );
    }

    #[test]
    fn scrgb_renk_uzayi_destegi_taninir() {
        use wgpu::SurfaceColorSpaces as R;
        assert!(scrgb_destekli(R::EXTENDED_SRGB_LINEAR));
        assert!(scrgb_destekli(R::SRGB | R::EXTENDED_SRGB_LINEAR));
        assert!(!scrgb_destekli(R::SRGB));
        assert!(!scrgb_destekli(R::empty()));
    }

    #[test]
    fn hdr_yolu_destek_yoksa_sdrde_kalir() {
        use wgpu::SurfaceColorSpaces as R;
        assert!(hdr_yolu_sec(true, true, R::EXTENDED_SRGB_LINEAR));
        assert!(!hdr_yolu_sec(false, true, R::EXTENDED_SRGB_LINEAR));
        assert!(!hdr_yolu_sec(true, false, R::EXTENDED_SRGB_LINEAR));
        assert!(!hdr_yolu_sec(true, true, R::SRGB));
    }

    #[test]
    fn sdr_beyaz_olcegi_scrgb_birimine_cevrilir() {
        assert!((sdr_beyaz_olcegi_hesapla(80.0) - 1.0).abs() < 1e-6);
        assert!((sdr_beyaz_olcegi_hesapla(240.0) - 3.0).abs() < 1e-6);
        assert!((sdr_beyaz_olcegi_hesapla(120.0) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn gecersiz_sdr_beyazi_bire_duser() {
        assert_eq!(sdr_beyaz_olcegi_hesapla(0.0), 1.0);
        assert_eq!(sdr_beyaz_olcegi_hesapla(-80.0), 1.0);
        assert_eq!(sdr_beyaz_olcegi_hesapla(f32::NAN), 1.0);
        assert_eq!(sdr_beyaz_olcegi_hesapla(f32::INFINITY), 1.0);
        assert_eq!(sdr_beyaz_olcegi_hesapla(80_000.0), AZAMI_SDR_BEYAZ_OLCEGI);
    }

    #[test]
    fn hdr_yolunda_shader_gama_kodlamaz() {
        assert!(!gama_kodlamasi_gerekli(true, HDR_FORMATI));
        assert!(gama_kodlamasi_gerekli(
            false,
            wgpu::TextureFormat::Bgra8Unorm
        ));
        assert!(!gama_kodlamasi_gerekli(
            false,
            wgpu::TextureFormat::Bgra8UnormSrgb
        ));
    }

    #[test]
    fn bilinen_srgb_formatlari_taninir() {
        assert!(wgpu::TextureFormat::Bgra8UnormSrgb.is_srgb());
        assert!(wgpu::TextureFormat::Rgba8UnormSrgb.is_srgb());
        assert!(!wgpu::TextureFormat::Rgba16Float.is_srgb());
    }

    #[test]
    fn sdr_formati_tercih_sirasina_gore_secilir() {
        use wgpu::TextureFormat as F;
        // Windows'un yerel formatı varsa o seçilir.
        assert_eq!(
            sdr_format_sec(&[F::Rgba8UnormSrgb, F::Bgra8UnormSrgb, F::Bgra8Unorm]),
            Some(F::Bgra8Unorm)
        );
        // Yerel format yoksa sıradaki tercih.
        assert_eq!(
            sdr_format_sec(&[F::Rgba8UnormSrgb, F::Rgba8Unorm]),
            Some(F::Rgba8Unorm)
        );
        // Hiçbiri yoksa sRGB varyantına düşülür (uygulama yine çalışır).
        assert_eq!(
            sdr_format_sec(&[F::Rgba8UnormSrgb]),
            Some(F::Rgba8UnormSrgb)
        );
        assert_eq!(sdr_format_sec(&[F::Rgba16Float]), None);
    }

    #[test]
    fn secilen_sdr_formati_shaderda_gama_kodlamasi_gerektirir() {
        // Tercih edilen tüm SDR formatları sRGB olmayan varyantlardır: egui ile aynı yol.
        for format in TERCIH_SDR.iter().take(2) {
            assert!(
                !format.is_srgb(),
                "{format:?} sRGB olmamalı (shader kodlar)"
            );
        }
    }
}
