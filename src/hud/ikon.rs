//! Lucide ikonları: tek font ailesi, tam punto boyut ve egui atlasında keskin glifler.

use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Response, Sense, Stroke, StrokeKind, Ui,
    vec2,
};
use iconflow::{Pack, Size, Style};

use super::tasarim::DOKUNMATIK_HEDEF;

const IKON_BOYUTU: f32 = 16.0;

/// Arayüzde kullanılan Lucide simgeleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ikon {
    Sol,
    Sag,
    Arti,
    Eksi,
    Carpi,
    UcNokta,
    DisaAc,
    Sirala,
    Klasor,
    Dosya,
    Ara,
    Tema,
    PanelSol,
    PanelSag,
    Yenile,
    Ayar,
    Bilgi,
    TamEkran,
    Sigdir,
    Onay,
    Uyari,
    Goz,
    Liste,
    Menu,
}

#[cfg(test)]
const IKONLAR: [Ikon; 24] = [
    Ikon::Sol,
    Ikon::Sag,
    Ikon::Arti,
    Ikon::Eksi,
    Ikon::Carpi,
    Ikon::UcNokta,
    Ikon::DisaAc,
    Ikon::Sirala,
    Ikon::Klasor,
    Ikon::Dosya,
    Ikon::Ara,
    Ikon::Tema,
    Ikon::PanelSol,
    Ikon::PanelSag,
    Ikon::Yenile,
    Ikon::Ayar,
    Ikon::Bilgi,
    Ikon::TamEkran,
    Ikon::Sigdir,
    Ikon::Onay,
    Ikon::Uyari,
    Ikon::Goz,
    Ikon::Liste,
    Ikon::Menu,
];

fn lucide_adi(ikon: Ikon) -> &'static str {
    match ikon {
        Ikon::Sol => "chevron-left",
        Ikon::Sag => "chevron-right",
        Ikon::Arti => "plus",
        Ikon::Eksi => "minus",
        Ikon::Carpi => "x",
        Ikon::UcNokta => "ellipsis-vertical",
        Ikon::DisaAc => "external-link",
        Ikon::Sirala => "arrow-up-down",
        Ikon::Klasor => "folder",
        Ikon::Dosya => "file-image",
        Ikon::Ara => "search",
        Ikon::Tema => "sun-moon",
        Ikon::PanelSol => "panel-left",
        Ikon::PanelSag => "panel-right",
        Ikon::Yenile => "refresh-cw",
        Ikon::Ayar => "settings",
        Ikon::Bilgi => "info",
        Ikon::TamEkran => "maximize",
        Ikon::Sigdir => "scan",
        Ikon::Onay => "check",
        Ikon::Uyari => "triangle-alert",
        Ikon::Goz => "eye",
        Ikon::Liste => "list",
        Ikon::Menu => "menu",
    }
}

/// Metin etiketlerine gömülebilen Lucide glifi.
pub fn karakter(ikon: Ikon) -> char {
    let kaynak = ikon_ref(ikon);
    char::from_u32(kaynak.codepoint).unwrap_or(' ')
}

fn ikon_ref(ikon: Ikon) -> iconflow::IconRef {
    iconflow::try_icon(
        Pack::Lucide,
        lucide_adi(ikon),
        Style::Regular,
        Size::Regular,
    )
    .expect("Lucide ikon adı çözümlenmeli")
}

/// Simgeyi verilen dikdörtgenin ortasına, font atlasından çizer.
pub fn ciz(painter: &egui::Painter, dikdortgen: egui::Rect, ikon: Ikon, renk: Color32) {
    let kaynak = ikon_ref(ikon);
    let Some(karakter) = char::from_u32(kaynak.codepoint) else {
        return;
    };
    let boyut = dikdortgen.width().min(dikdortgen.height()).max(IKON_BOYUTU);
    painter.text(
        dikdortgen.center(),
        Align2::CENTER_CENTER,
        karakter,
        FontId::new(boyut, FontFamily::Name(kaynak.family.into())),
        renk,
    );
}

pub fn ikon_dugme_boyut(
    ui: &mut Ui,
    ikon: Ikon,
    secili: bool,
    ipucu: &str,
    boyut: f32,
) -> Response {
    let gorsel = boyut.max(DOKUNMATIK_HEDEF);
    let (dikdortgen, yanit) = ui.allocate_exact_size(vec2(gorsel, gorsel), Sense::click());
    let vizueller = ui.visuals();
    let dolgu = if secili {
        Some(vizueller.selection.bg_fill)
    } else if yanit.hovered() || yanit.highlighted() {
        Some(vizueller.widgets.hovered.weak_bg_fill)
    } else {
        None
    };
    if let Some(dolgu) = dolgu {
        ui.painter()
            .rect_filled(dikdortgen, CornerRadius::same(7), dolgu);
    }
    if yanit.has_focus() {
        ui.painter().rect_stroke(
            dikdortgen,
            CornerRadius::same(7),
            Stroke::new(2.0, vizueller.selection.stroke.color),
            StrokeKind::Inside,
        );
    }
    let renk = if secili {
        vizueller.selection.stroke.color
    } else {
        vizueller.text_color()
    };
    ciz(
        ui.painter(),
        egui::Rect::from_center_size(dikdortgen.center(), vec2(IKON_BOYUTU, IKON_BOYUTU)),
        ikon,
        renk,
    );
    yanit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, ipucu));
    yanit.on_hover_text(ipucu)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn tum_lucide_adlari_cozulur() {
        for ikon in IKONLAR {
            let ad = lucide_adi(ikon);
            assert!(
                iconflow::try_icon(Pack::Lucide, ad, Style::Regular, Size::Regular).is_ok(),
                "{ad} Lucide paketinde bulunamadı"
            );
        }
    }

}
