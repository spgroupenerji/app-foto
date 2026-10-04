//! HUD (baş üstü arayüz) katmanı: araç çubuğu, meta veri paneli, bağlam menüsü,
//! ayarlar penceresi ve geçici bildirimler. Tüm arayüz Türkçedir.

pub mod arac_cubugu;
pub mod ayar_pencere;
pub mod baglam_menu;
pub mod bildirim;
pub mod dosya_listesi;
pub mod meta_panel;

use crate::cekirdek::ayar::Tema;
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

/// Kurumsal paletin panel zeminleri: hem koyu hem açık varyant, egui temalarına yazılır.
const KOYU_PANEL_FILI: [u8; 4] = [26, 26, 30, 235];
const ACIK_PANEL_FILI: [u8; 4] = [243, 243, 246, 235];

/// RGBA dizisini yarı saydam renge çevirir (sabit üretimi için yardımcı).
fn renk(rgba: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// Koyu tema paleti: nötr kurumsal gri ölçek (zinc), araç çubuğu yarı saydam kalır.
fn koyu_palet() -> egui::Visuals {
    let mut gorsel = egui::Visuals::dark();
    gorsel.panel_fill = renk(KOYU_PANEL_FILI);
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
    gorsel
}

/// Açık tema paleti: aynı gri ölçeğin aydınlık ucu.
fn acik_palet() -> egui::Visuals {
    let mut gorsel = egui::Visuals::light();
    gorsel.panel_fill = renk(ACIK_PANEL_FILI);
    gorsel.window_fill = egui::Color32::from_rgba_unmultiplied(248, 248, 250, 245);
    gorsel.extreme_bg_color = egui::Color32::from_rgb(233, 233, 238);
    gorsel.faint_bg_color = egui::Color32::from_rgb(224, 224, 230);
    gorsel.override_text_color = Some(egui::Color32::from_rgb(35, 35, 40));
    gorsel.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(198, 198, 206));

    gorsel.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(215, 215, 222);
    gorsel.widgets.inactive.bg_fill = egui::Color32::from_rgb(215, 215, 222);
    gorsel.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(199, 199, 208);
    gorsel.widgets.hovered.bg_fill = egui::Color32::from_rgb(199, 199, 208);
    gorsel.widgets.active.weak_bg_fill = egui::Color32::from_rgb(208, 208, 216);
    gorsel.widgets.active.bg_fill = egui::Color32::from_rgb(208, 208, 216);
    gorsel
}

/// Tema tercihini ve kurumsal paleti uygular.
///
/// Paletler her iki egui temasına da yazılır; böylece koyu/açık/sistem geçişi
/// anında ve doğru paletle gerçekleşir. Dokunmatik hedefler tüm temalara ortaktır.
pub fn tema_uygula(ctx: &egui::Context, tema: Tema) {
    ctx.set_theme(match tema {
        Tema::Koyu => egui::ThemePreference::Dark,
        Tema::Acik => egui::ThemePreference::Light,
        Tema::Sistem => egui::ThemePreference::System,
    });
    ctx.set_visuals_of(egui::Theme::Dark, koyu_palet());
    ctx.set_visuals_of(egui::Theme::Light, acik_palet());
    ctx.all_styles_mut(|stil| {
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
    dosya_listesi::ciz(ui, durum, &mut sonuc);
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
    fn tema_iki_paleti_ve_dokunmatik_hedefi_kurar() {
        let ctx = egui::Context::default();
        tema_uygula(&ctx, Tema::Koyu);
        let koyu = ctx.style_of(egui::Theme::Dark);
        assert!(koyu.visuals.dark_mode);
        assert_eq!(koyu.visuals.panel_fill, renk(KOYU_PANEL_FILI));
        assert_eq!(
            koyu.spacing.interact_size,
            egui::vec2(DOKUNMATIK_HEDEF, DOKUNMATIK_HEDEF),
            "tıklanabilir her öğe en az 44×44 olmalı"
        );

        tema_uygula(&ctx, Tema::Acik);
        let acik = ctx.style_of(egui::Theme::Light);
        assert!(!acik.visuals.dark_mode);
        assert_eq!(acik.visuals.panel_fill, renk(ACIK_PANEL_FILI));
        assert_eq!(
            acik.spacing.interact_size,
            egui::vec2(DOKUNMATIK_HEDEF, DOKUNMATIK_HEDEF)
        );

        // Sistem tercihi: iki palet de kurulu kalır, egui OS tercihini izler.
        tema_uygula(&ctx, Tema::Sistem);
        assert_eq!(
            ctx.style_of(egui::Theme::Dark).visuals.panel_fill,
            renk(KOYU_PANEL_FILI)
        );
        assert_eq!(
            ctx.style_of(egui::Theme::Light).visuals.panel_fill,
            renk(ACIK_PANEL_FILI)
        );
    }
}
