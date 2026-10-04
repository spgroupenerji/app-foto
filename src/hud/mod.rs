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

/// Dokunmatik cihazlar için en küçük etkileşim hedefi (Apple HIG 44 pt, WCAG 2.5.5).
pub const DOKUNMATIK_HEDEF: f32 = 44.0;

/// Uygulama temasını kurar: 44 px dokunmatik hedefler ve kurumsal gri koyu palet.
pub fn tema_kur(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.global_style_mut(|stil| {
        let gorsel = &mut stil.visuals;
        *gorsel = egui::Visuals::dark();

        // Zemin tonları: nötr kurumsal gri ölçek (zinc), araç çubuğu yarı saydam kalır.
        gorsel.panel_fill = egui::Color32::from_rgba_unmultiplied(26, 26, 30, 235);
        gorsel.window_fill = egui::Color32::from_rgba_unmultiplied(33, 33, 38, 245);
        gorsel.extreme_bg_color = egui::Color32::from_rgb(18, 18, 21);
        gorsel.faint_bg_color = egui::Color32::from_rgb(40, 40, 46);
        gorsel.override_text_color = Some(egui::Color32::from_rgb(232, 232, 236));
        gorsel.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(62, 62, 70));

        // Düğme zeminleri: inaktif → hover → aktif gri basamakları.
        gorsel.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(52, 52, 59);
        gorsel.widgets.inactive.bg_fill = egui::Color32::from_rgb(52, 52, 59);
        gorsel.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(66, 66, 74);
        gorsel.widgets.hovered.bg_fill = egui::Color32::from_rgb(66, 66, 74);
        gorsel.widgets.active.weak_bg_fill = egui::Color32::from_rgb(44, 44, 50);
        gorsel.widgets.active.bg_fill = egui::Color32::from_rgb(44, 44, 50);

        // Dokunmatik: tıklanabilir her öğe en az 44×44 puan; egui Button yüksekliği
        // `spacing.interact_size.y`'den türetildiği için tüm düğmeler bundan yararlanır.
        stil.spacing.interact_size = egui::vec2(DOKUNMATIK_HEDEF, DOKUNMATIK_HEDEF);
        stil.spacing.button_padding = egui::vec2(12.0, 8.0);
    });
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

    #[test]
    fn tema_dokunmatik_hedefi_ve_paleti_kurar() {
        let ctx = egui::Context::default();
        tema_kur(&ctx);
        let stil = ctx.style_of(egui::Theme::Dark);
        assert_eq!(
            stil.spacing.interact_size,
            egui::vec2(DOKUNMATIK_HEDEF, DOKUNMATIK_HEDEF),
            "tıklanabilir her öğe en az 44×44 olmalı"
        );
        assert_eq!(stil.visuals.dark_mode, true);
        assert_eq!(
            stil.visuals.panel_fill,
            egui::Color32::from_rgba_unmultiplied(26, 26, 30, 235)
        );
    }
}
