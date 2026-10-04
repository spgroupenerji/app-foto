//! Geçici bildirimler: durum ve hata mesajları ekranın altında kısa süre görünür.

use crate::cekirdek::durum::UygulamaDurumu;

/// Bildirim kutusunun alt kenardan yüksekliği (durum çubuğunun üstünde kalır).
const DUVAR_DIBI_KAYDIRMA: f32 = -52.0;

/// Aktif mesajı çizer; mesaj yoksa hiçbir şey yapmaz.
pub fn ciz(ui: &mut egui::Ui, durum: &UygulamaDurumu) {
    let Some(mesaj) = &durum.mesaj else {
        return;
    };

    // Süre dolarken yumuşakça kaybolur.
    let kalan = mesaj.kalan_oran();
    let alfa = (kalan * 4.0).clamp(0.0, 1.0);
    let (arka, kenar) = if mesaj.onemli {
        (
            egui::Color32::from_rgba_unmultiplied(70, 26, 26, (240.0 * alfa) as u8),
            egui::Color32::from_rgba_unmultiplied(220, 90, 90, (200.0 * alfa) as u8),
        )
    } else {
        (
            egui::Color32::from_rgba_unmultiplied(26, 30, 38, (230.0 * alfa) as u8),
            egui::Color32::from_rgba_unmultiplied(90, 110, 140, (180.0 * alfa) as u8),
        )
    };

    // Yüzen alanlar bağlam üzerinden çizilir; bağlam tutamacı klonlanarak ödünç çakışması önlenir.
    let ctx = ui.ctx().clone();
    egui::Area::new(egui::Id::new("gorsel-bildirim"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, DUVAR_DIBI_KAYDIRMA))
        .interactable(false)
        .show(&ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(arka)
                .stroke(egui::Stroke::new(1.0, kenar))
                .corner_radius(egui::CornerRadius::same(6))
                .show(ui, |ui| {
                    ui.set_max_width(520.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&mesaj.metin).strong());
                        if let Some(ipucu) = &mesaj.ipucu {
                            ui.label(egui::RichText::new(ipucu).small().weak());
                        }
                    });
                });
        });

    // Görünürlük süresince sürekli yeniden çizim iste.
    ctx.request_repaint_after(std::time::Duration::from_millis(50));
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::cekirdek::ayar::Ayarlar;
    use crate::cekirdek::hata::GorselHatasi;

    #[test]
    fn mesaj_yoksa_cizim_yapmaz() {
        let durum = UygulamaDurumu::yeni(Ayarlar::default());
        assert!(durum.mesaj.is_none());
    }

    #[test]
    fn hata_mesaji_onemli_isaretlenir() {
        let mut durum = UygulamaDurumu::yeni(Ayarlar::default());
        durum.hata_ver(&GorselHatasi::Bulunamadi("x.jpg".into()));
        assert!(durum.mesaj.as_ref().unwrap().onemli);
    }
}
