//! GPU katmanı: wgpu örneği/aygıtı, HDR yapılandırmalı takas zinciri, görüntü dokusu,
//! çizim boru hattı ve egui arayüz katmanının tek karede birleştirilmesi.

pub mod boru;
pub mod cizim;
pub mod doku;
pub mod hdr;
pub mod yuzey;

use crate::cekirdek::hata::{GorselHatasi, Sonuc};

/// Grafik aygıtı ve kuyruğu. Tek örnek olarak oluşturulur ve uygulama boyunca yaşar.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// HUD'da gösterilecek insan okunur adaptör bilgisi ("NVIDIA ... (Vulkan)").
    pub adaptor_bilgisi: String,
}

impl Gpu {
    /// Yüksek başarımlı adaptörü seçip aygıtı kurar.
    ///
    /// Pencere yüzeyi sonradan bağlanır (`compatible_surface: None`); böylece aygıt
    /// kurulumu pencere oluşturulmadan önce yapılabilir ve pencere yeniden
    /// oluşturulduğunda aygıt yeniden kurulmaz.
    pub fn yeni() -> Sonuc<Self> {
        // `InstanceDescriptor` varsayılanı yoktur; kurucu üzerinden üretilip arka uç
        // tercihi (DX12 → Vulkan) açıkça yazılır.
        let mut tanim = wgpu::InstanceDescriptor::new_without_display_handle();
        tanim.backends = tercih_edilen_arka_uclar();
        let instance = wgpu::Instance::new(tanim);

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .map_err(|k| GorselHatasi::Grafik(format!("uygun grafik aygıtı bulunamadı: {k}")))?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gorsel-aygit"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .map_err(|k| GorselHatasi::Grafik(format!("grafik aygıtı kurulamadı: {k}")))?;

        let bilgi = adapter.get_info();
        let adaptor_bilgisi = format!("{} ({:?})", bilgi.name, bilgi.backend);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            adaptor_bilgisi,
        })
    }

    /// Bu aygıtın desteklediği en büyük 2B doku kenarı (CPU ön küçültme eşiği için).
    pub fn en_buyuk_doku_kenari(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }
}

/// Windows'ta önce DirectX 12, sonra Vulkan denenir; ikisi de yoksa ortam değişkenine bırakılır.
fn tercih_edilen_arka_uclar() -> wgpu::Backends {
    wgpu::Backends::from_env().unwrap_or(wgpu::Backends::DX12 | wgpu::Backends::VULKAN)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn tercih_edilen_arka_uclar_bos_degil() {
        let uclar = tercih_edilen_arka_uclar();
        assert!(!uclar.is_empty(), "en az bir arka uç seçilmeli");
    }
}
