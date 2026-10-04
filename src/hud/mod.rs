//! HUD (baş üstü arayüz) katmanı: araç çubuğu, meta veri paneli, bağlam menüsü,
//! ayarlar penceresi ve geçici bildirimler. Tüm arayüz Türkçedir.

pub mod arac_cubugu;
pub mod ayar_pencere;
pub mod baglam_menu;
pub mod bildirim;
pub mod meta_panel;

use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

/// HUD'un bir karede ürettiği sonuç.
#[derive(Debug, Default)]
pub struct HudSonucu {
    /// Kullanıcının tetiklediği eylemler (sırayla uygulanır).
    pub eylemler: Vec<Eylem>,
    /// Ayarlar değişti: diske yazılmalı.
    pub ayar_degisti: bool,
}

impl HudSonucu {
    /// Tek bir eylem ekler.
    pub fn ekle(&mut self, eylem: Eylem) {
        if eylem != Eylem::Yok {
            self.eylemler.push(eylem);
        }
    }
}

/// Uygulama temasını kurar (koyu, yüksek kontrastlı HUD).
pub fn tema_kur(ctx: &egui::Context) {
    let mut gorsel = egui::Visuals::dark();
    gorsel.panel_fill = egui::Color32::from_rgba_unmultiplied(18, 18, 22, 235);
    gorsel.window_fill = egui::Color32::from_rgba_unmultiplied(22, 22, 27, 245);
    gorsel.override_text_color = Some(egui::Color32::from_rgb(232, 232, 236));
    ctx.set_visuals(gorsel);
}

/// Tüm HUD katmanını çizer.
///
/// egui 0.36 kök `Ui` üzerinden çalışır; paneller bu kökün içine yerleştirilir,
/// yüzen öğeler (pencere, alan) bağlam üzerinden çizilir.
pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu) -> HudSonucu {
    let mut sonuc = HudSonucu::default();
    arac_cubugu::ciz(ui, durum, &mut sonuc);
    meta_panel::ciz(ui, durum, &mut sonuc);

    if durum.ayar_penceresi {
        ayar_pencere::ciz(ui, durum, &mut sonuc);
    }
    if durum.baglam_menusu.is_some() {
        baglam_menu::ciz(ui, durum, &mut sonuc);
    }
    bildirim::ciz(ui, durum);
    sonuc
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn sonuc_eylem_biriktirir() {
        let mut s = HudSonucu::default();
        s.ekle(Eylem::Yok);
        assert!(s.eylemler.is_empty(), "Yok eylemi eklenmez");
        s.ekle(Eylem::Sonraki);
        assert_eq!(s.eylemler, vec![Eylem::Sonraki]);
    }
}
