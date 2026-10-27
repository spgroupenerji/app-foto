//! Kare çizimi: görüntü geçişi ve egui arayüz katmanının aynı hedefe çizilmesi.
//!
//! SDR yolunda iki geçiş kullanılır: ilk geçiş görüntüyü (ya da yalnızca arka planı)
//! temizleyip çizer, ikinci geçiş mevcut içeriğin üzerine egui katmanını yükler
//! (`LoadOp::Load`). Böylece HUD, görüntüyle aynı takas zinciri yüzeyini paylaşır.
//!
//! HDR (scRGB) yolunda egui doğrudan yüzeye çizilemez: egui gama kodlu değer üretir,
//! yüzey ise doğrusal scRGB bekler. Bu yüzden egui `Rgba8Unorm` ara dokuya çizilir;
//! `bilesim.wgsl` geçişi değerleri doğrusallaştırıp SDR beyazı ölçeğiyle yüzeye harmanlar.

use super::Gpu;
use super::boru::{GoruntuBorusu, GoruntuUniform};
use super::doku::GoruntuDokusu;
use super::yuzey::{HDR_FORMATI, Yuzey};

/// HDR yolunda egui'nin gama kodlu çizim hedefi.
pub const ARAYUZ_FORMATI: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// egui katmanının bir kare için ürettiği veri.
pub struct EguiKare {
    pub doku_deltalari: egui::TexturesDelta,
    pub cizim_isleri: Vec<egui::ClippedPrimitive>,
    pub ekran: egui_wgpu::ScreenDescriptor,
}

/// Kare sonucu: takas zincirinin durumu ve yapılması gerekenler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KareDurumu {
    Cizildi,
    /// Kare sunuldu ama yapılandırma artık en uygun değil (ör. pencere boyutu değişti).
    AltOptimal,
    /// Takas zinciri meşgul; kare atlanır ve bir sonraki turda yeniden denenir.
    ZamanAsimi,
    /// Pencere görünür değil; çizim yapılmaz, kaynak harcanmaz.
    Gizli,
    /// Yüzey geçersiz; yapılandırma yenilenmeli.
    YenidenYapilandir,
    /// Yüzey kaybedildi ve geri alınamadı.
    Kayip,
}

impl KareDurumu {
    /// Yüzeyin yeniden yapılandırılması gerekiyor mu.
    pub fn yeniden_yapilandirma_gerekli(self) -> bool {
        matches!(self, Self::YenidenYapilandir | Self::AltOptimal)
    }

    /// Kullanıcıya gösterilecek hata metni (yoksa `None`).
    pub fn hata_metni(self) -> Option<&'static str> {
        match self {
            Self::Kayip => Some("Grafik yüzeyi kaybedildi; pencere yeniden oluşturuluyor."),
            Self::ZamanAsimi => Some("Grafik aygıtı meşgul; kare atlandı."),
            _ => None,
        }
    }
}

/// egui'nin çizim hedefi: HDR yolunda gama kodlu ara doku, SDR yolunda yüzeyin kendisi.
fn arayuz_hedef_formati(hdr: bool, yuzey_formati: wgpu::TextureFormat) -> wgpu::TextureFormat {
    if hdr { ARAYUZ_FORMATI } else { yuzey_formati }
}

/// Ara doku ölçüsü değişti mi; ilk oluşturma da yenileme sayılır.
fn olcu_yenilenmeli(mevcut: Option<(u32, u32)>, yeni: (u32, u32)) -> bool {
    mevcut != Some(yeni)
}

/// Doğrusal bileşeni sRGB kodlamasına çevirir (gölgelendiricinin arka plan yoluyla aynı).
fn dogrusaldan_srgb(deger: f32) -> f64 {
    let guvenli = f64::from(deger.max(0.0));
    if guvenli <= 0.003_130_8 {
        guvenli * 12.92
    } else {
        1.055 * guvenli.powf(1.0 / 2.4) - 0.055
    }
}

/// Görüntü geçişinin temizleme rengi; shader'ın arka plan çıkışıyla aynı kodlamayı kullanır.
fn temizleme_rengi(uniform: &GoruntuUniform) -> wgpu::Color {
    let arka = uniform.arka_plan;
    if uniform.dolgu[0] > 0.5 {
        let olcek = f64::from(uniform.dolgu[1]);
        return wgpu::Color {
            r: f64::from(arka[0]) * olcek,
            g: f64::from(arka[1]) * olcek,
            b: f64::from(arka[2]) * olcek,
            a: 1.0,
        };
    }
    if uniform.ton[3] > 0.5 {
        return wgpu::Color {
            r: dogrusaldan_srgb(arka[0]),
            g: dogrusaldan_srgb(arka[1]),
            b: dogrusaldan_srgb(arka[2]),
            a: 1.0,
        };
    }
    wgpu::Color {
        r: f64::from(arka[0]),
        g: f64::from(arka[1]),
        b: f64::from(arka[2]),
        a: 1.0,
    }
}

/// Bileşim geçişinin SDR beyazı ölçeği (16 bayt hizalı üniform).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BilesimUniform {
    olcek: [f32; 4],
}

/// Yüzey ölçüsüne bağlı ara doku ve bağlama grubu.
struct BilesimHedef {
    #[allow(dead_code)]
    doku: wgpu::Texture,
    gorunum: wgpu::TextureView,
    baglama_grubu: wgpu::BindGroup,
    olcu: (u32, u32),
}

/// HDR yolunun egui ara dokusu, örnekleyicisi ve bileşim boru hattı.
struct Bilesim {
    boru_hatti: wgpu::RenderPipeline,
    yerlesim: wgpu::BindGroupLayout,
    ornekleyici: wgpu::Sampler,
    uniform_tamponu: wgpu::Buffer,
    hedef: Option<BilesimHedef>,
}

impl Bilesim {
    fn yeni(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("bilesim-golgelendirici"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shader/bilesim.wgsl").into()),
            });

        let yerlesim = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bilesim-yerlesim"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let boru_yerlesimi = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bilesim-boru-yerlesimi"),
                bind_group_layouts: &[Some(&yerlesim)],
                immediate_size: 0,
            });

        let boru_hatti = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("bilesim-boru"),
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
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        let uniform_tamponu = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bilesim-uniform"),
            size: std::mem::size_of::<BilesimUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue.write_buffer(
            &uniform_tamponu,
            0,
            bytemuck::bytes_of(&BilesimUniform {
                olcek: [1.0, 0.0, 0.0, 0.0],
            }),
        );

        let ornekleyici = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("arayuz-ornekleyici"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            boru_hatti,
            yerlesim,
            ornekleyici,
            uniform_tamponu,
            hedef: None,
        }
    }

    /// SDR beyazı ölçeğini bileşim üniformuna yazar; geçersiz değer 1,0'a düşer.
    fn olcek_yaz(&self, gpu: &Gpu, sdr_beyaz_olcek: f32) {
        let olcek = if sdr_beyaz_olcek.is_finite() && sdr_beyaz_olcek > 0.0 {
            sdr_beyaz_olcek
        } else {
            1.0
        };
        gpu.queue.write_buffer(
            &self.uniform_tamponu,
            0,
            bytemuck::bytes_of(&BilesimUniform {
                olcek: [olcek, 0.0, 0.0, 0.0],
            }),
        );
    }

    /// Ara doku ve bağlama grubunu yüzey ölçüsüne göre (gerekiyorsa) yeniden kurar.
    fn hazirla(&mut self, gpu: &Gpu, olcu: (u32, u32)) {
        let (genislik, yukseklik) = (olcu.0.max(1), olcu.1.max(1));
        let mevcut = self.hedef.as_ref().map(|h| h.olcu);
        if !olcu_yenilenmeli(mevcut, (genislik, yukseklik)) {
            return;
        }

        let doku = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("arayuz-dokusu"),
            size: wgpu::Extent3d {
                width: genislik,
                height: yukseklik,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ARAYUZ_FORMATI,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let gorunum = doku.create_view(&wgpu::TextureViewDescriptor::default());
        let baglama_grubu = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("arayuz-baglama"),
            layout: &self.yerlesim,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gorunum),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.ornekleyici),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_tamponu.as_entire_binding(),
                },
            ],
        });

        self.hedef = Some(BilesimHedef {
            doku,
            gorunum,
            baglama_grubu,
            olcu: (genislik, yukseklik),
        });
    }
}

/// Çizim katmanı: görüntü boru hattı, egui çizici ve HDR bileşim kaynakları.
pub struct Cizim {
    pub boru: GoruntuBorusu,
    pub egui: egui_wgpu::Renderer,
    bilesim: Option<Bilesim>,
}

impl Cizim {
    /// Boru hattını ve egui çizicisini verilen hedef formatı için kurar.
    ///
    /// `Rgba16Float` yalnızca HDR (scRGB) yüzeyinde kullanıldığından bu format HDR
    /// yolunu seçer: egui gama kodlu ara dokuya çizilir ve bileşim geçişi kurulur.
    pub fn yeni(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let hdr = format == HDR_FORMATI;
        let boru = GoruntuBorusu::yeni(gpu, format);
        let egui = egui_wgpu::Renderer::new(
            &gpu.device,
            arayuz_hedef_formati(hdr, format),
            egui_wgpu::RendererOptions::default(),
        );
        let bilesim = hdr.then(|| Bilesim::yeni(gpu, format));
        Self {
            boru,
            egui,
            bilesim,
        }
    }

    /// Bir kareyi çizer.
    ///
    /// `doku` verilmezse yalnızca arka plan temizlenir (görsel yüklenirken olduğu gibi).
    pub fn kare_ciz(
        &mut self,
        gpu: &Gpu,
        yuzey: &Yuzey,
        doku: Option<&GoruntuDokusu>,
        uniform: &GoruntuUniform,
        egui_kare: Option<EguiKare>,
    ) -> KareDurumu {
        use wgpu::CurrentSurfaceTexture as YuzeyDurumu;

        // Doku deltaları yüzey alınmadan önce uygulanır. Yüzey meşgul/örtük olsa bile
        // TexturesDelta sahipliği burada tüketilir; aksi halde debug derlemesinde
        // "unapplied deltas" paniği oluşur.
        let egui_var = egui_kare.is_some();
        let (mut doku_deltalari, cizim_isleri, ekran) = match egui_kare {
            Some(veri) => (Some(veri.doku_deltalari), veri.cizim_isleri, veri.ekran),
            None => (
                None,
                Vec::new(),
                egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [1, 1],
                    pixels_per_point: 1.0,
                },
            ),
        };
        let mut serbest = Vec::new();
        if let Some(deltalari) = doku_deltalari.as_mut() {
            for (kimlik, deltalar) in &deltalari.set {
                for delta in deltalar {
                    self.egui
                        .update_texture(&gpu.device, &gpu.queue, *kimlik, delta);
                }
            }
            serbest.extend(deltalari.free.iter().copied());
            deltalari.clear();
        }

        let (kare, durum) = match yuzey.surface.get_current_texture() {
            YuzeyDurumu::Success(t) => (t, KareDurumu::Cizildi),
            YuzeyDurumu::Suboptimal(t) => (t, KareDurumu::AltOptimal),
            YuzeyDurumu::Timeout => {
                for kimlik in &serbest {
                    self.egui.free_texture(kimlik);
                }
                return KareDurumu::ZamanAsimi;
            }
            YuzeyDurumu::Occluded => {
                for kimlik in &serbest {
                    self.egui.free_texture(kimlik);
                }
                return KareDurumu::Gizli;
            }
            YuzeyDurumu::Outdated | YuzeyDurumu::Lost => {
                for kimlik in &serbest {
                    self.egui.free_texture(kimlik);
                }
                return KareDurumu::YenidenYapilandir;
            }
            YuzeyDurumu::Validation => {
                for kimlik in &serbest {
                    self.egui.free_texture(kimlik);
                }
                return KareDurumu::Kayip;
            }
        };

        let hedef = kare
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut kodlayici = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kare-kodlayici"),
            });

        // HDR yolunda egui hedefi yüzey ölçüsüne göre hazırlanır; pencere boyutu
        // değiştiğinde ara doku ve bağlama grubu burada yenilenir.
        if egui_var
            && let Some(bilesim) = self.bilesim.as_mut()
        {
            bilesim.hazirla(gpu, yuzey.olcu());
            bilesim.olcek_yaz(gpu, yuzey.sdr_beyaz_olcegi());
        }

        let egui_komutlari = if egui_var {
            self.egui.update_buffers(
                &gpu.device,
                &gpu.queue,
                &mut kodlayici,
                &cizim_isleri,
                &ekran,
            )
        } else {
            Vec::new()
        };

        self.boru.uniform_yaz(gpu, uniform);

        // 1. geçiş: arka plan + görüntü.
        {
            let mut gecis = kodlayici.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("goruntu-gecisi"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &hedef,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(temizleme_rengi(uniform)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if let Some(gorsel) = doku {
                gecis.set_pipeline(&self.boru.pipeline);
                gecis.set_bind_group(0, &gorsel.baglama_grubu, &[]);
                gecis.draw(0..3, 0..1);
            }
        }

        if egui_var {
            if let Some(bilesim) = self.bilesim.as_ref() {
                let arayuz = bilesim
                    .hedef
                    .as_ref()
                    .expect("HDR arayüz hedefi hazırlanmalı");

                // 2. geçiş: egui, gama kodlu ara dokuya.
                {
                    let gecis = kodlayici.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("arayuz-gecisi"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &arayuz.gorunum,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    // egui, yaşam süresi unutulmuş bir geçiş bekler (tamponlar çiziciye ait).
                    let mut kalici = gecis.forget_lifetime();
                    self.egui
                        .render(&mut kalici, &cizim_isleri, &ekran);
                }

                // 3. geçiş: egui katmanını doğrusal scRGB yüzeyine harmanla.
                {
                    let mut gecis = kodlayici.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("bilesim-gecisi"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &hedef,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    gecis.set_pipeline(&bilesim.boru_hatti);
                    gecis.set_bind_group(0, &arayuz.baglama_grubu, &[]);
                    gecis.draw(0..3, 0..1);
                }
            } else {
                // SDR yolu: egui doğrudan yüzeye, mevcut içeriğin üzerine.
                let gecis = kodlayici.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui-gecisi"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &hedef,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                let mut kalici = gecis.forget_lifetime();
                self.egui
                    .render(&mut kalici, &cizim_isleri, &ekran);
            }
        }

        gpu.queue.submit(
            egui_komutlari
                .into_iter()
                .chain(std::iter::once(kodlayici.finish())),
        );
        // Sunum wgpu 30'da kuyruk üzerinden yapılır (`SurfaceTexture::present` kaldırıldı).
        gpu.queue.present(kare);

        for kimlik in &serbest {
            self.egui.free_texture(kimlik);
        }

        durum
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kare_durumu_kararlari() {
        assert!(KareDurumu::AltOptimal.yeniden_yapilandirma_gerekli());
        assert!(KareDurumu::YenidenYapilandir.yeniden_yapilandirma_gerekli());
        assert!(!KareDurumu::Cizildi.yeniden_yapilandirma_gerekli());
        assert!(KareDurumu::Kayip.hata_metni().is_some());
        assert!(KareDurumu::Cizildi.hata_metni().is_none());
        assert!(KareDurumu::Gizli.hata_metni().is_none());
    }

    #[test]
    fn hdr_yolunda_egui_gama_dokusuna_cizilir() {
        assert_eq!(
            arayuz_hedef_formati(true, wgpu::TextureFormat::Rgba16Float),
            ARAYUZ_FORMATI
        );
        assert!(
            !ARAYUZ_FORMATI.is_srgb(),
            "egui gama değerleri kendisi yazar"
        );
        assert_eq!(
            arayuz_hedef_formati(false, wgpu::TextureFormat::Bgra8Unorm),
            wgpu::TextureFormat::Bgra8Unorm
        );
    }

    #[test]
    fn olcu_degisimi_algilanir() {
        assert!(olcu_yenilenmeli(None, (800, 600)));
        assert!(!olcu_yenilenmeli(Some((800, 600)), (800, 600)));
        assert!(olcu_yenilenmeli(Some((800, 600)), (801, 600)));
    }

    #[test]
    fn hdr_temizleme_rengi_sdr_beyazi_olcegiyle_carpilir() {
        let mut uniform = GoruntuUniform::default();
        uniform.arka_plan = [0.1, 0.2, 0.3, 1.0];
        let sdr = temizleme_rengi(&uniform);
        assert!((sdr.r - 0.1).abs() < 1e-6);

        uniform.dolgu = [1.0, 3.0];
        let hdr = temizleme_rengi(&uniform);
        assert!((hdr.r - 0.3).abs() < 1e-6);
        assert!((hdr.g - 0.6).abs() < 1e-6);
        assert!((hdr.b - 0.9).abs() < 1e-6);
        assert_eq!(hdr.a, 1.0);

        uniform.dolgu = [0.0, 1.0];
        uniform.ton[3] = 1.0;
        let gama = temizleme_rengi(&uniform);
        assert!((gama.r - dogrusaldan_srgb(0.1)).abs() < 1e-6);
        assert!((gama.g - dogrusaldan_srgb(0.2)).abs() < 1e-6);
    }

    #[test]
    fn bilesim_golgelendiricisi_gecerli_wgsl() {
        let modul = wgpu::naga::front::wgsl::parse_str(include_str!("../shader/bilesim.wgsl"))
            .expect("bilesim.wgsl ayrıştırılmalı");
        let mut dogrulayici = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        );
        dogrulayici
            .validate(&modul)
            .expect("bilesim.wgsl doğrulanmalı");
    }
}
