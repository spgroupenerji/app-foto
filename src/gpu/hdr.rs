//! Ekranın yüksek dinamik aralık (HDR) yeteneği ve ton haritalama parametreleri.
//!
//! wgpu 30, `Surface::display_hdr_info` ile ekranın tepe parlaklığını (nit), SDR beyaz
//! referansını ve ek dinamik aralık payını (headroom) bildirir. Bu değerler ton
//! haritalama gölgelendiricisine aktarılır; bilgi yoksa güvenli varsayılanlar kullanılır
//! (bilinmeyen değer "SDR ekran" anlamına gelmez).

/// Ton haritalama için güvenli varsayılan tepe parlaklık (nit).
pub const VARSAYILAN_TEPE_NITS: f32 = 1000.0;

/// Windows'ta tipik SDR beyaz referansı (nit). scRGB'de 1,0 değeri bu parlaklığa karşılık gelir.
pub const VARSAYILAN_SDR_BEYAZ_NITS: f32 = 240.0;

/// Ton haritalama altında kabul edilen en düşük tepe değer.
const EN_AZ_TEPE_NITS: f32 = 100.0;

/// Ekranın HDR karakteristiği.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdrBilgi {
    /// Ekran HDR çıkışı yapabiliyor mu (bildirilen tepe parlaklık SDR beyazını belirgin biçimde aşıyor).
    pub hdr_kullanilabilir: bool,
    /// Ekranın bildirdiği tepe parlaklık (nit).
    pub tepe_nits: f32,
    /// SDR beyaz referansı (nit).
    pub sdr_beyaz_nits: f32,
    /// Tepe parlaklığın SDR beyazına oranı (ek dinamik aralık payı).
    pub headroom: f32,
}

impl Default for HdrBilgi {
    fn default() -> Self {
        Self {
            hdr_kullanilabilir: false,
            tepe_nits: VARSAYILAN_TEPE_NITS,
            sdr_beyaz_nits: VARSAYILAN_SDR_BEYAZ_NITS,
            headroom: 1.0,
        }
    }
}

impl HdrBilgi {
    /// Ekran bilgisini sorgular; eksik alanlar güvenli varsayılana tamamlanır.
    pub fn sorgula(yuzey: &wgpu::Surface<'_>, adapter: &wgpu::Adapter) -> Self {
        let bilgi = yuzey.display_hdr_info(adapter);
        let tepe = bilgi
            .luminance
            .and_then(|l| l.max_nits)
            .filter(|n| n.is_finite() && *n > 0.0)
            .unwrap_or(VARSAYILAN_TEPE_NITS)
            .max(EN_AZ_TEPE_NITS);
        let beyaz = bilgi
            .luminance
            .and_then(|l| l.sdr_white_nits)
            .filter(|n| n.is_finite() && *n > 0.0)
            .unwrap_or(VARSAYILAN_SDR_BEYAZ_NITS)
            .min(tepe);
        let headroom = bilgi
            .headroom
            .and_then(|h| h.current)
            .filter(|h| h.is_finite() && *h > 0.0)
            .unwrap_or_else(|| tepe / beyaz);

        Self {
            hdr_kullanilabilir: headroom > HDR_ESIGI,
            tepe_nits: tepe,
            sdr_beyaz_nits: beyaz,
            headroom,
        }
    }

    /// Bilgi yoksa kullanılan SDR varsayılanı.
    pub fn varsayilan() -> Self {
        Self::default()
    }

    // Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
    #[allow(dead_code)]
    /// Ton haritalama gölgelendiricisinin uygulanıp uygulanmayacağı.
    pub fn ton_haritalama_gerekli(&self) -> bool {
        self.hdr_kullanilabilir
    }

    /// HUD'da gösterilecek kısa özet.
    pub fn ozet(&self) -> String {
        if self.hdr_kullanilabilir {
            format!(
                "HDR: {:.0} nit tepe, {:.0} nit SDR beyazı",
                self.tepe_nits, self.sdr_beyaz_nits
            )
        } else {
            "SDR ekran".to_string()
        }
    }
}

/// Headroom bu oranın üzerindeyse ekran HDR kabul edilir (1,5 = %50 ek dinamik aralık).
const HDR_ESIGI: f32 = 1.5;

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilan_sdr_davranisi() {
        let b = HdrBilgi::varsayilan();
        assert!(!b.hdr_kullanilabilir);
        assert!(!b.ton_haritalama_gerekli());
        assert_eq!(b.ozet(), "SDR ekran");
    }

    #[test]
    fn hdr_esigi_asan_headroom_hdr_sayilir() {
        let b = HdrBilgi {
            hdr_kullanilabilir: true,
            tepe_nits: 1000.0,
            sdr_beyaz_nits: 240.0,
            headroom: 4.0,
        };
        assert!(b.ton_haritalama_gerekli());
        assert!(b.ozet().contains("1000"));
        assert!(b.ozet().contains("HDR"));
    }

    #[test]
    fn varsayilan_tepe_degerleri_makul() {
        let b = HdrBilgi::default();
        assert!(b.tepe_nits >= EN_AZ_TEPE_NITS);
        assert!(b.sdr_beyaz_nits <= b.tepe_nits);
        assert!(b.headroom > 0.0);
    }
}
