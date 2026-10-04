//! Görüntü çizim boru hattı: WGSL gölgelendiricisi, üniform tampon ve örnekleyici.
//!
//! Yeniden örnekleme tamamen GPU'da yapılır; kaydırma ve yakınlaştırma yalnızca üniform
//! değerleri değiştirir, bu yüzden 144 Hz ve üzeri yenileme hızlarında kare kaybı olmaz.

use super::Gpu;
use super::hdr::HdrBilgi;
use crate::girdi::bolge::Gorunum;
use crate::goruntu::renk::srgb_bayt_doğrusal;

/// Gölgelendiriciye aktarılan kare sabitleri. WGSL tarafındaki `GoruntuUniform` ile
/// alan alan örtüşür (16 bayt hizalama kuralına uygun dolgu içerir).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GoruntuUniform {
    /// Kaynak dokunun piksel ölçüsü.
    pub doku_boyutu: [f32; 2],
    /// 16 bayt hizalaması için dolgu.
    pub dolgu: [f32; 2],
    /// Görüntü merkezinin pencere içindeki yeri (0-1) ve pencere ölçüsü (piksel).
    pub yerlesim: [f32; 4],
    /// Görüntünün pencereye göre yarı ölçüsü (0-1).
    pub donusum: [f32; 4],
    /// tepe nit, SDR beyaz nit, HDR kaynak (0/1), shader gama kodlaması (0/1).
    pub ton: [f32; 4],
    /// Arka plan rengi (doğrusal RGBA).
    pub arka_plan: [f32; 4],
}

impl Default for GoruntuUniform {
    fn default() -> Self {
        Self {
            doku_boyutu: [1.0, 1.0],
            dolgu: [0.0, 0.0],
            yerlesim: [0.5, 0.5, 1.0, 1.0],
            donusum: [0.5, 0.5, 0.0, 0.0],
            ton: [
                super::hdr::VARSAYILAN_TEPE_NITS,
                super::hdr::VARSAYILAN_SDR_BEYAZ_NITS,
                0.0,
                0.0,
            ],
            arka_plan: [0.01, 0.01, 0.012, 1.0],
        }
    }
}

/// Görünüm ve ekran bilgisinden gölgelendirici sabitlerini üretir.
#[allow(clippy::too_many_arguments)]
pub fn uniform_olustur(
    goruntu_olcusu: (u32, u32),
    gorunum: &Gorunum,
    pencere_olcusu: (u32, u32),
    hdr: &HdrBilgi,
    hdr_kaynak: bool,
    gama_shaderda: bool,
    arka_plan_srgb: [u8; 3],
) -> GoruntuUniform {
    let (gw, gh) = (goruntu_olcusu.0 as f32, goruntu_olcusu.1 as f32);
    let (ww, wh) = (pencere_olcusu.0.max(1) as f32, pencere_olcusu.1.max(1) as f32);

    // Görüntünün ekrandaki ölçüsü ve merkezi (pencere pikseli, y aşağı).
    let gosterim_w = gw * gorunum.olcek as f32;
    let gosterim_h = gh * gorunum.olcek as f32;
    let merkez_x = ww / 2.0 + gorunum.kaydirma_x as f32 * gorunum.olcek as f32;
    let merkez_y = wh / 2.0 + gorunum.kaydirma_y as f32 * gorunum.olcek as f32;

    GoruntuUniform {
        doku_boyutu: [gw.max(1.0), gh.max(1.0)],
        dolgu: [0.0, 0.0],
        yerlesim: [merkez_x / ww, merkez_y / wh, ww, wh],
        donusum: [gosterim_w / ww / 2.0, gosterim_h / wh / 2.0, 0.0, 0.0],
        ton: [
            hdr.tepe_nits,
            hdr.sdr_beyaz_nits,
            if hdr_kaynak { 1.0 } else { 0.0 },
            if gama_shaderda { 1.0 } else { 0.0 },
        ],
        arka_plan: [
            srgb_bayt_doğrusal(arka_plan_srgb[0]),
            srgb_bayt_doğrusal(arka_plan_srgb[1]),
            srgb_bayt_doğrusal(arka_plan_srgb[2]),
            1.0,
        ],
    }
}

/// Görüntüyü pencereye çizen boru hattı ve kaynakları.
pub struct GoruntuBorusu {
    pub pipeline: wgpu::RenderPipeline,
    pub yerlesim: wgpu::BindGroupLayout,
    pub uniform_tamponu: wgpu::Buffer,
    pub ornekleyici: wgpu::Sampler,
}

impl GoruntuBorusu {
    /// Boru hattını, üniform tamponu ve örnekleyiciyi kurar.
    pub fn yeni(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("goruntu-golgelendirici"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../shader/goruntu.wgsl").into(),
                ),
            });

        let yerlesim = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("goruntu-yerlesim"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let boru_yerlesimi = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("goruntu-boru-yerlesimi"),
                bind_group_layouts: &[Some(&yerlesim)],
                immediate_size: 0,
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("goruntu-boru"),
                layout: Some(&boru_yerlesimi),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_ana"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_ana"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        let uniform_tamponu = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("goruntu-uniform"),
            size: std::mem::size_of::<GoruntuUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Catmull-Rom tekniği donanımın çift doğrusal filtrelemesine dayanır:
        // mag/min filtresi Doğrusal olmak ZORUNDADIR, aksi halde sonuç bloklu olur.
        let ornekleyici = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("goruntu-ornekleyici"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            ..Default::default()
        });

        Self {
            pipeline,
            yerlesim,
            uniform_tamponu,
            ornekleyici,
        }
    }

    /// Kare sabitlerini GPU'ya yazar.
    pub fn uniform_yaz(&self, gpu: &Gpu, ayar: &GoruntuUniform) {
        gpu.queue
            .write_buffer(&self.uniform_tamponu, 0, bytemuck::bytes_of(ayar));
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn uniform_boyu_on_alti_bayt_katı() {
        assert_eq!(std::mem::size_of::<GoruntuUniform>() % 16, 0);
        assert_eq!(std::mem::size_of::<GoruntuUniform>(), 80);
    }

    #[test]
    fn yerlesim_matematigi_dogru() {
        // %100 ölçek, kaydırma yok, 400x300 pencere ve 200x100 görüntü.
        let gorunum = Gorunum::default();
        let u = uniform_olustur((200, 100), &gorunum, (400, 300), &HdrBilgi::default(), false, true, [0, 0, 0]);
        assert_eq!(u.doku_boyutu, [200.0, 100.0]);
        // Merkez pencere ortasında.
        assert!((u.yerlesim[0] - 0.5).abs() < 1e-6);
        assert!((u.yerlesim[1] - 0.5).abs() < 1e-6);
        assert_eq!([u.yerlesim[2], u.yerlesim[3]], [400.0, 300.0]);
        // Görüntü 200x100 → pencereye göre yarı ölçü: 200/400/2 = 0.25, 100/300/2 ≈ 0.1667
        assert!((u.donusum[0] - 0.25).abs() < 1e-6);
        assert!((u.donusum[1] - 1.0 / 6.0).abs() < 1e-6);
    }

    #[test]
    fn kaydirma_merkezi_oynatir() {
        let gorunum = Gorunum {
            olcek: 2.0,
            kaydirma_x: 50.0,
            kaydirma_y: -25.0,
        };
        let u = uniform_olustur((100, 100), &gorunum, (400, 400), &HdrBilgi::default(), false, false, [0, 0, 0]);
        // merkez_piksel = 200 + 50*2 = 300 → 300/400 = 0,75
        assert!((u.yerlesim[0] - 0.75).abs() < 1e-6);
        // 200 - 25*2 = 150 → 150/400 = 0,375
        assert!((u.yerlesim[1] - 0.375).abs() < 1e-6);
        // Görüntü 100x100, ölçek 2 → 200x200 → yarı ölçü 200/400/2 = 0,25
        assert!((u.donusum[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn hdr_bayraklari_aktarilir() {
        let gorunum = Gorunum::default();
        let hdr = HdrBilgi {
            hdr_kullanilabilir: true,
            tepe_nits: 800.0,
            sdr_beyaz_nits: 200.0,
            headroom: 4.0,
        };
        let u = uniform_olustur((10, 10), &gorunum, (10, 10), &hdr, true, true, [0, 0, 0]);
        assert_eq!(u.ton[0], 800.0);
        assert_eq!(u.ton[1], 200.0);
        assert_eq!(u.ton[2], 1.0, "HDR kaynak bayrağı");
        assert_eq!(u.ton[3], 1.0, "shader gama kodlaması");

        let sdr = uniform_olustur((10, 10), &gorunum, (10, 10), &HdrBilgi::default(), false, false, [0, 0, 0]);
        assert_eq!(sdr.ton[2], 0.0);
        assert_eq!(sdr.ton[3], 0.0);
    }

    #[test]
    fn arka_plan_dogrusallastirilir() {
        let u = uniform_olustur((10, 10), &Gorunum::default(), (10, 10), &HdrBilgi::default(), false, false, [255, 255, 255]);
        assert!((u.arka_plan[0] - 1.0).abs() < 1e-6);
        assert_eq!(u.arka_plan[3], 1.0, "arka plan opak olmalı");
        let siyah = uniform_olustur((10, 10), &Gorunum::default(), (10, 10), &HdrBilgi::default(), false, false, [0, 0, 0]);
        assert_eq!(siyah.arka_plan[0], 0.0);
    }

    #[test]
    fn sifir_pencere_bolme_hatasi_vermez() {
        let u = uniform_olustur((10, 10), &Gorunum::default(), (0, 0), &HdrBilgi::default(), false, false, [0, 0, 0]);
        assert!(u.yerlesim[0].is_finite());
        assert!(u.donusum[0].is_finite());
    }
}
