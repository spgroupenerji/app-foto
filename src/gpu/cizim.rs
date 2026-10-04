//! Kare çizimi: görüntü geçişi ve egui arayüz katmanının aynı hedefe çizilmesi.
//!
//! İki geçiş kullanılır: ilk geçiş görüntüyü (ya da yalnızca arka planı) temizleyip çizer,
//! ikinci geçiş mevcut içeriğin üzerine egui katmanını yükler (`LoadOp::Load`). Böylece
//! HUD, görüntüyle aynı takas zinciri yüzeyini paylaşır ve ayrı bir ara doku gerekmez.

use super::Gpu;
use super::boru::{GoruntuBorusu, GoruntuUniform};
use super::doku::GoruntuDokusu;
use super::yuzey::Yuzey;

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

/// Çizim katmanı: görüntü boru hattı ve egui çizici.
pub struct Cizim {
    pub boru: GoruntuBorusu,
    pub egui: egui_wgpu::Renderer,
}

impl Cizim {
    /// Boru hattını ve egui çizicisini verilen hedef formatı için kurar.
    pub fn yeni(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let boru = GoruntuBorusu::yeni(gpu, format);
        let egui = egui_wgpu::Renderer::new(
            &gpu.device,
            format,
            egui_wgpu::RendererOptions::default(),
        );
        Self { boru, egui }
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

        let (kare, durum) = match yuzey.surface.get_current_texture() {
            YuzeyDurumu::Success(t) => (t, KareDurumu::Cizildi),
            YuzeyDurumu::Suboptimal(t) => (t, KareDurumu::AltOptimal),
            YuzeyDurumu::Timeout => return KareDurumu::ZamanAsimi,
            YuzeyDurumu::Occluded => return KareDurumu::Gizli,
            YuzeyDurumu::Outdated | YuzeyDurumu::Lost => return KareDurumu::YenidenYapilandir,
            YuzeyDurumu::Validation => return KareDurumu::Kayip,
        };

        let hedef = kare
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut kodlayici = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kare-kodlayici"),
            });

        // egui doku güncellemeleri ve tepe noktası tamponları geçişten önce hazırlanır.
        // Bir doku kimliği için birden çok delta gelebilir (kısmi güncellemeler).
        let egui_komutlari = match egui_kare {
            Some(ref veri) => {
                for (kimlik, deltalar) in &veri.doku_deltalari.set {
                    for delta in deltalar {
                        self.egui
                            .update_texture(&gpu.device, &gpu.queue, *kimlik, delta);
                    }
                }
                self.egui.update_buffers(
                    &gpu.device,
                    &gpu.queue,
                    &mut kodlayici,
                    &veri.cizim_isleri,
                    &veri.ekran,
                )
            }
            None => Vec::new(),
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(uniform.arka_plan[0]),
                            g: f64::from(uniform.arka_plan[1]),
                            b: f64::from(uniform.arka_plan[2]),
                            a: 1.0,
                        }),
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

        // egui çizici, sonraki kare için serbest bırakılması gereken dokuları bildirir.
        // Liste, `egui_kare` taşınmadan önce alınır.
        let serbest = egui_kare
            .as_ref()
            .map(|v| v.doku_deltalari.free.clone())
            .unwrap_or_default();

        // 2. geçiş: egui katmanı, mevcut içeriğin üzerine.
        if let Some(veri) = egui_kare {
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
            // egui, yaşam süresi unutulmuş bir geçiş bekler (tamponlar çiziciye ait).
            let mut kalici = gecis.forget_lifetime();
            self.egui.render(
                &mut kalici,
                &veri.cizim_isleri,
                &veri.ekran,
            );
        }

        gpu.queue
            .submit(egui_komutlari.into_iter().chain(std::iter::once(kodlayici.finish())));
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
}
