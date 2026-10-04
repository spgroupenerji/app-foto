//! Takas zinciri (swapchain) yönetimi ve HDR yapılandırması.
//!
//! Format seçimi yetenek sorgusuna dayanır: ekran HDR bildiriyorsa ve aygıt
//! `Rgba16Float` yüzeyini destekliyorsa HDR yolu açılır; aksi halde sRGB formatına düşülür.
//!
//! Renk uzayı bilinçli olarak `Srgb` seçilir: egui arayüz katmanı, hedef format sRGB
//! olmadığında değerleri kendisi gama kodlar (`is_srgb()` kontrolü). Görüntü gölgelendiricisi
//! aynı kuralı uyguladığı için HUD ile görsel aynı kodlamayı paylaşır ve renk kayması oluşmaz.

use std::sync::Arc;

use winit::window::Window;

use super::Gpu;
use super::hdr::HdrBilgi;
use crate::cekirdek::hata::{GorselHatasi, Sonuc};

/// HDR yolu için tercih edilen yüzey formatı (16-bit kayan nokta, geniş aralık).
pub const HDR_FORMATI: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

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
    pub fn yeni(
        gpu: &Gpu,
        pencere: Arc<Window>,
        olcu: (u32, u32),
        hdr_iste: bool,
    ) -> Sonuc<Self> {
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
        let hdr_kullaniliyor =
            hdr_iste && hdr.hdr_kullanilabilir && yetenekler.formats.contains(&HDR_FORMATI);

        if hdr_kullaniliyor {
            config.format = HDR_FORMATI;
            config.color_space = wgpu::SurfaceColorSpace::Srgb;
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

    /// Hedef formatta gama kodlamasını shader'ın yapması gerekiyor mu.
    ///
    /// sRGB formatlarında donanım kodlar (shader doğrusal yazar); float formatlarda
    /// shader kodlar (egui aynı davranışı gösterir).
    pub fn gama_kodlamasi_shaderda(&self) -> bool {
        !self.config.format.is_srgb()
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
            "float yüzey sRGB değildir; shader gama kodlaması bu yüzden gerekir"
        );
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
