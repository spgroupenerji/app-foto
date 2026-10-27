//! Tasarım sistemi: shadcn v4 token'larının egui karşılığı, tipografi, tema ve ortak ölçüler.
//!
//! Renkler semantik rolle kullanılır; bileşenler doğrudan hex değeri taşımaz. Koyu ve
//! açık palet aynı ölçeği paylaşır; vurgu rengi tek bir kurumsal zümrüt tondur.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, TextStyle};

use crate::cekirdek::ayar::Tema;

pub const BOSLUK_XS: f32 = 4.0;
pub const BOSLUK_SM: f32 = 8.0;
pub const BOSLUK_MD: f32 = 12.0;
pub const BOSLUK_LG: f32 = 16.0;
pub const BOSLUK_XL: f32 = 24.0;

pub const DOKUNMATIK_HEDEF: f32 = 44.0;
pub const UST_CUBUK_YUKSEKLIGI: f32 = 56.0;
pub const ALT_CUBUK_YUKSEKLIGI: f32 = 28.0;
pub const SATIR_RAHAT: f32 = 64.0;
pub const SATIR_KOMPAKT: f32 = 48.0;
pub const KUCUK_RESIM_RAHAT: f32 = 52.0;
pub const KUCUK_RESIM_KOMPAKT: f32 = 36.0;
pub const YARICAP_SM: u8 = 4;
pub const YARICAP_MD: u8 = 6;
pub const YARICAP_LG: u8 = 8;
pub const YARICAP_XL: u8 = 12;

/// Seçili temaya göre anlamlı renkler.
#[derive(Debug, Clone, Copy)]
pub struct Palet {
    pub panel: Color32,
    pub panel_opak: Color32,
    pub panel_filli: Color32,
    pub yuzey: Color32,
    pub yuzey_2: Color32,
    pub yuzey_3: Color32,
    pub kenar: Color32,
    pub kenar_guc: Color32,
    pub metin: Color32,
    pub soluk: Color32,
    pub cok_soluk: Color32,
    pub vurgu: Color32,
    pub vurgu_guc: Color32,
    pub vurgu_yumusak: Color32,
    pub vurgu_metin: Color32,
    pub tehlike: Color32,
    pub uyari: Color32,
    pub bilgi: Color32,
    pub secim: Color32,
    pub golge: Color32,
    pub metin_kutusu: Color32,
    pub kod_zemin: Color32,
}

impl Palet {
    pub fn sec(koyu: bool) -> Self {
        if koyu {
            Self {
                panel: Color32::from_rgb(18, 19, 23),
                panel_opak: Color32::from_rgb(22, 23, 27),
                panel_filli: Color32::from_rgba_unmultiplied(18, 19, 23, 235),
                yuzey: Color32::from_rgb(26, 27, 32),
                yuzey_2: Color32::from_rgb(34, 36, 42),
                yuzey_3: Color32::from_rgb(43, 46, 53),
                kenar: Color32::from_rgb(42, 45, 52),
                kenar_guc: Color32::from_rgb(58, 62, 71),
                metin: Color32::from_rgb(236, 238, 242),
                soluk: Color32::from_rgb(155, 160, 170),
                cok_soluk: Color32::from_rgb(111, 116, 128),
                vurgu: Color32::from_rgb(76, 195, 138),
                vurgu_guc: Color32::from_rgb(106, 215, 157),
                vurgu_yumusak: Color32::from_rgba_unmultiplied(76, 195, 138, 28),
                vurgu_metin: Color32::from_rgb(7, 19, 13),
                tehlike: Color32::from_rgb(240, 122, 122),
                uyari: Color32::from_rgb(230, 180, 80),
                bilgi: Color32::from_rgb(123, 184, 255),
                secim: Color32::from_rgba_unmultiplied(76, 195, 138, 34),
                golge: Color32::from_black_alpha(150),
                metin_kutusu: Color32::from_rgb(14, 15, 18),
                kod_zemin: Color32::from_rgb(31, 33, 39),
            }
        } else {
            Self {
                panel: Color32::from_rgb(251, 251, 252),
                panel_opak: Color32::from_rgb(255, 255, 255),
                panel_filli: Color32::from_rgba_unmultiplied(251, 251, 252, 238),
                yuzey: Color32::from_rgb(255, 255, 255),
                yuzey_2: Color32::from_rgb(240, 241, 243),
                yuzey_3: Color32::from_rgb(228, 230, 233),
                kenar: Color32::from_rgb(217, 219, 224),
                kenar_guc: Color32::from_rgb(196, 199, 206),
                metin: Color32::from_rgb(23, 25, 30),
                soluk: Color32::from_rgb(93, 98, 108),
                cok_soluk: Color32::from_rgb(133, 138, 148),
                vurgu: Color32::from_rgb(21, 122, 85),
                vurgu_guc: Color32::from_rgb(15, 98, 67),
                vurgu_yumusak: Color32::from_rgba_unmultiplied(21, 122, 85, 22),
                vurgu_metin: Color32::WHITE,
                tehlike: Color32::from_rgb(184, 58, 58),
                uyari: Color32::from_rgb(138, 91, 0),
                bilgi: Color32::from_rgb(31, 95, 158),
                secim: Color32::from_rgba_unmultiplied(21, 122, 85, 28),
                golge: Color32::from_black_alpha(45),
                metin_kutusu: Color32::from_rgb(255, 255, 255),
                kod_zemin: Color32::from_rgb(240, 241, 243),
            }
        }
    }

    pub fn ui(ui: &egui::Ui) -> Self {
        Self::sec(ui.visuals().dark_mode)
    }

    pub fn ctx(ctx: &egui::Context) -> Self {
        Self::sec(matches!(ctx.theme(), egui::Theme::Dark))
    }
}

/// Sistem fontlarını yükler; dosya yoksa egui varsayılanları korunur.
///
/// Windows'ta Segoe UI gövde, Segoe UI Semibold başlık, Consolas sayısal metin için
/// kullanılır. Lisans gereği fontlar pakete gömülmez, sistemden okunur.
pub fn fontlari_yukle(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let varsayilan_oransal = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let varsayilan_mono = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let mut oransal = Vec::new();
    if let Some(bytes) = sistem_fontu("segoeui.ttf") {
        fonts.font_data.insert(
            "segoe-ui".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        oransal.push("segoe-ui".to_owned());
    }
    oransal.extend(varsayilan_oransal.clone());
    fonts
        .families
        .insert(FontFamily::Proportional, oransal.clone());

    let mut baslik = Vec::new();
    if let Some(bytes) = sistem_fontu("seguisb.ttf") {
        fonts.font_data.insert(
            "segoe-ui-yari-kalin".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        baslik.push("segoe-ui-yari-kalin".to_owned());
    }
    baslik.extend(oransal);
    fonts
        .families
        .insert(FontFamily::Name("baslik".into()), baslik);

    let mut mono = Vec::new();
    if let Some(bytes) = sistem_fontu("consola.ttf") {
        fonts
            .font_data
            .insert("consolas".to_owned(), Arc::new(FontData::from_owned(bytes)));
        mono.push("consolas".to_owned());
    }
    mono.extend(varsayilan_mono);
    fonts
        .families
        .insert(FontFamily::Monospace, mono);

    ctx.set_fonts(fonts);
}

fn sistem_fontu(dosya: &str) -> Option<Vec<u8>> {
    let kok = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    std::fs::read(kok.join("Fonts").join(dosya)).ok()
}

/// Tema tercihini, paleti, tipografiyi ve dokunmatik ölçüleri uygular.
pub fn tema_uygula(ctx: &egui::Context, tema: Tema, hareket_azalt: bool) {
    ctx.set_theme(match tema {
        Tema::Koyu => egui::ThemePreference::Dark,
        Tema::Acik => egui::ThemePreference::Light,
        Tema::Sistem => egui::ThemePreference::System,
    });
    ctx.set_visuals_of(egui::Theme::Dark, gorseller(true));
    ctx.set_visuals_of(egui::Theme::Light, gorseller(false));

    ctx.all_styles_mut(|stil| {
        stil.spacing.interact_size = egui::vec2(DOKUNMATIK_HEDEF, DOKUNMATIK_HEDEF);
        stil.spacing.button_padding = egui::vec2(BOSLUK_MD, BOSLUK_SM);
        stil.spacing.item_spacing = egui::vec2(BOSLUK_SM, BOSLUK_MD / 2.0);
        stil.spacing.window_margin = egui::Margin::same(BOSLUK_MD as i8);
        stil.spacing.menu_margin = egui::Margin::same(BOSLUK_SM as i8);
        stil.spacing.indent = BOSLUK_LG;
        stil.spacing.slider_width = 160.0;
        stil.spacing.combo_width = 180.0;
        stil.spacing.text_edit_width = 220.0;
        stil.spacing.tooltip_width = 320.0;
        stil.spacing.menu_width = 240.0;
        stil.spacing.extra_text_line_spacing = 2.0;
        stil.text_styles = metin_stilleri();
        stil.animation_time = if hareket_azalt { 0.0 } else { 0.16 };
        stil.visuals.window_highlight_topmost = false;
        stil.visuals.striped = false;
        stil.visuals.slider_trailing_fill = true;
        stil.visuals.interact_cursor = None;
        stil.visuals.image_loading_spinners = true;
        stil.visuals.button_frame = true;
        stil.visuals.collapsing_header_frame = false;
        stil.visuals.indent_has_left_vline = false;
    });
}

fn metin_stilleri() -> BTreeMap<TextStyle, FontId> {
    let baslik = FontFamily::Name("baslik".into());
    [
        (TextStyle::Heading, FontId::new(20.0, baslik.clone())),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(11.5, FontFamily::Monospace)),
        (
            TextStyle::Name("bolum".into()),
            FontId::new(11.0, baslik.clone()),
        ),
        (
            TextStyle::Name("buyuk".into()),
            FontId::new(24.0, baslik.clone()),
        ),
        (
            TextStyle::Name("sayi".into()),
            FontId::new(12.0, FontFamily::Monospace),
        ),
    ]
    .into()
}

fn gorseller(koyu: bool) -> egui::Visuals {
    let p = Palet::sec(koyu);
    let mut g = if koyu {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    g.panel_fill = p.panel_filli;
    g.window_fill = p.panel_opak;
    g.window_stroke = egui::Stroke::new(1.0, p.kenar);
    g.window_corner_radius = CornerRadius::same(YARICAP_XL);
    g.menu_corner_radius = CornerRadius::same(YARICAP_LG);
    g.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 30,
        spread: 0,
        color: p.golge,
    };
    g.popup_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: p.golge,
    };
    g.extreme_bg_color = p.metin_kutusu;
    g.text_edit_bg_color = Some(p.metin_kutusu);
    g.faint_bg_color = p.yuzey_2;
    g.code_bg_color = p.kod_zemin;
    g.override_text_color = Some(p.metin);
    g.weak_text_color = Some(p.soluk);
    g.hyperlink_color = p.vurgu;
    g.warn_fg_color = p.uyari;
    g.error_fg_color = p.tehlike;
    g.selection.bg_fill = p.secim;
    g.selection.stroke = egui::Stroke::new(1.0, p.metin);
    g.widgets.noninteractive.bg_fill = p.panel;
    g.widgets.noninteractive.weak_bg_fill = p.panel;
    g.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.kenar);
    g.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.metin);
    g.widgets.noninteractive.corner_radius = CornerRadius::same(YARICAP_LG);
    g.widgets.inactive.bg_fill = p.yuzey;
    g.widgets.inactive.weak_bg_fill = p.yuzey;
    g.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.kenar);
    g.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, p.metin);
    g.widgets.inactive.corner_radius = CornerRadius::same(YARICAP_LG);
    g.widgets.hovered.bg_fill = p.yuzey_2;
    g.widgets.hovered.weak_bg_fill = p.yuzey_2;
    g.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.kenar_guc);
    g.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, p.metin);
    g.widgets.hovered.corner_radius = CornerRadius::same(YARICAP_LG);
    g.widgets.active.bg_fill = p.yuzey_3;
    g.widgets.active.weak_bg_fill = p.yuzey_3;
    g.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.vurgu);
    g.widgets.active.fg_stroke = egui::Stroke::new(1.0, p.metin);
    g.widgets.active.corner_radius = CornerRadius::same(YARICAP_LG);
    g.widgets.open = g.widgets.active;
    g
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn iki_palet_ayni_rolleri_doldurur() {
        for koyu in [true, false] {
            let p = Palet::sec(koyu);
            assert_ne!(p.panel, p.metin, "panel ve metin ayrışmalı");
            assert_ne!(p.vurgu, p.vurgu_metin, "vurgu üzerindeki metin okunmalı");
            assert_ne!(p.kenar, p.panel, "kenar panelden ayrılmalı");
        }
    }

    #[test]
    fn tema_dokunmatik_hedefi_kurar() {
        let ctx = egui::Context::default();
        tema_uygula(&ctx, Tema::Koyu, false);
        let stil = ctx.style_of(egui::Theme::Dark);
        assert!(stil.visuals.dark_mode);
        assert_eq!(
            stil.spacing.interact_size,
            egui::vec2(DOKUNMATIK_HEDEF, DOKUNMATIK_HEDEF)
        );
        assert!(stil.animation_time > 0.0);

        tema_uygula(&ctx, Tema::Koyu, true);
        assert_eq!(
            ctx.style_of(egui::Theme::Dark).animation_time,
            0.0,
            "hareketi azalt animasyonu kapatmalı"
        );
    }

    #[test]
    fn metin_stilleri_turkce_baslik_fontunu_kullanir() {
        let stiller = metin_stilleri();
        let baslik = stiller.get(&TextStyle::Heading).expect("başlık stili");
        assert_eq!(baslik.family, FontFamily::Name("baslik".into()));
        assert!(stiller.contains_key(&TextStyle::Body));
        assert!(stiller.contains_key(&TextStyle::Monospace));
    }
}
