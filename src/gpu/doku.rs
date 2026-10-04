//! Görüntü dokusu: doğrusal f16 piksel tamponunun VRAM'e aktarılması ve bağlama grubu.

use super::Gpu;
use crate::cekirdek::hata::{GorselHatasi, Sonuc};

/// GPU'da duran görüntü dokusu ve ona bağlı kaynak grubu.
pub struct GoruntuDokusu {
    // Doku ve görünüm, bağlama grubunun dayandığı kaynaklardır; sahiplikleri burada
    // tutulur, aksi halde GPU kaynakları kare arasında serbest bırakılır.
    #[allow(dead_code)]
    pub doku: wgpu::Texture,
    #[allow(dead_code)]
    pub gorunum: wgpu::TextureView,
    pub baglama_grubu: wgpu::BindGroup,
    pub genislik: u32,
    pub yukseklik: u32,
}

impl GoruntuDokusu {
    /// f16 RGBA tamponunu VRAM'e yükler ve kaynak grubunu kurar.
    ///
    /// Kaynak grubu, kare üniform tamponunu (binding 0), dokuyu (1) ve örnekleyiciyi (2)
    /// birlikte bağlar; bu yüzden üniform tampon ve örnekleyici dışarıdan verilir.
    pub fn yeni(
        gpu: &Gpu,
        veri: &[u16],
        genislik: u32,
        yukseklik: u32,
        katman: &wgpu::BindGroupLayout,
        uniform: &wgpu::Buffer,
        ornekleyici: &wgpu::Sampler,
    ) -> Sonuc<Self> {
        if genislik == 0 || yukseklik == 0 {
            return Err(GorselHatasi::Grafik(
                "doku ölçüsü sıfır olamaz".into(),
            ));
        }
        let beklenen = genislik as usize * yukseklik as usize * 4;
        if veri.len() < beklenen {
            return Err(GorselHatasi::Grafik(format!(
                "doku verisi eksik: {} < {beklenen}",
                veri.len()
            )));
        }
        let en_buyuk = gpu.device.limits().max_texture_dimension_2d;
        if genislik > en_buyuk || yukseklik > en_buyuk {
            return Err(GorselHatasi::BellekYetersiz {
                genislik,
                yukseklik,
                bayt: u64::from(genislik) * u64::from(yukseklik) * 8,
            });
        }

        let boyut = wgpu::Extent3d {
            width: genislik,
            height: yukseklik,
            depth_or_array_layers: 1,
        };
        let doku = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("goruntu-dokusu"),
            size: boyut,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Satır başına bayt: 4 kanal × 2 bayt. wgpu satır hizası şartı FLOAT16 için 2'dir,
        // 8'e bölünebildiğinden ek dolgu gerekmez.
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &doku,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&veri[..beklenen]),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(genislik * 8),
                rows_per_image: Some(yukseklik),
            },
            boyut,
        );

        let gorunum = doku.create_view(&wgpu::TextureViewDescriptor::default());
        let baglama_grubu = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("goruntu-baglama"),
            layout: katman,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gorunum),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(ornekleyici),
                },
            ],
        });

        Ok(Self {
            doku,
            gorunum,
            baglama_grubu,
            genislik,
            yukseklik,
        })
    }

    /// Dokunun VRAM'de kapladığı bayt (4 kanal × 2 bayt).
    pub fn vram_boyutu(&self) -> u64 {
        u64::from(self.genislik) * u64::from(self.yukseklik) * 8
    }

    /// Dokuyu temsil eden ölçü metni.
    pub fn olcu_metni(&self) -> String {
        format!("{}×{}", self.genislik, self.yukseklik)
    }
}
