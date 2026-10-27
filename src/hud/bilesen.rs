//! Ortak arayüz bileşenleri: düğme, rozet, bölüm başlığı, bilgi satırı, kart, iskelet.

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Margin, Response, RichText, Sense, Stroke,
    StrokeKind, TextStyle, Ui, Vec2, pos2, vec2,
};

use crate::girdi::Eylem;

use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DugmeTuru {
    Birincil,
    Ikincil,
    Hayalet,
    Tehlike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RozetTuru {
    Uyari,
    Bilgi,
}

/// Özel çizilen düğme; klavye aktivasyonu için [`tiklandi`] ile birlikte kullanılır.
pub fn dugme(
    ui: &mut Ui,
    etiket: &str,
    tur: DugmeTuru,
    ikon: Option<Ikon>,
    kisayol: Option<&str>,
) -> Response {
    let p = Palet::ui(ui);
    let metin_rengi = match tur {
        DugmeTuru::Birincil => p.vurgu_metin,
        DugmeTuru::Tehlike => p.tehlike,
        _ => p.metin,
    };
    let font = TextStyle::Button.resolve(ui.style());
    let metin = ui
        .painter()
        .layout_no_wrap(etiket.to_owned(), font, metin_rengi);
    let kisayol = kisayol.map(|k| {
        ui.painter().layout_no_wrap(
            k.to_owned(),
            FontId::new(10.0, FontFamily::Monospace),
            p.soluk,
        )
    });
    let ikon_alani = if ikon.is_some() { 17.0 } else { 0.0 };
    let aralik = if ikon.is_some() { 7.0 } else { 0.0 };
    let kisayol_alani = kisayol
        .as_ref()
        .map_or(0.0, |g| g.size().x + 20.0);
    let yukseklik = 36.0;
    let genislik = (tasarim::BOSLUK_MD * 2.0
        + ikon_alani
        + aralik
        + metin.size().x
        + kisayol_alani)
        .max(tasarim::DOKUNMATIK_HEDEF);
    let (rect, yanit) = ui.allocate_exact_size(vec2(genislik, yukseklik), Sense::click());
    let etkin = yanit.hovered() || yanit.has_focus();
    let (dolgu, kenar) = match tur {
        DugmeTuru::Birincil => (
            if etkin { p.vurgu_guc } else { p.vurgu },
            Color32::TRANSPARENT,
        ),
        DugmeTuru::Tehlike => (
            if etkin {
                Color32::from_rgba_unmultiplied(
                    p.tehlike.r(),
                    p.tehlike.g(),
                    p.tehlike.b(),
                    28,
                )
            } else {
                Color32::TRANSPARENT
            },
            if etkin { p.tehlike } else { p.kenar },
        ),
        DugmeTuru::Ikincil => (
            if etkin { p.yuzey_2 } else { p.yuzey },
            if etkin { p.kenar_guc } else { p.kenar },
        ),
        DugmeTuru::Hayalet => (
            if etkin { p.yuzey_2 } else { Color32::TRANSPARENT },
            Color32::TRANSPARENT,
        ),
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(tasarim::YARICAP_LG), dolgu);
    if kenar != Color32::TRANSPARENT {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(tasarim::YARICAP_LG),
            Stroke::new(1.0, kenar),
            StrokeKind::Inside,
        );
    }
    if yanit.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(tasarim::YARICAP_LG + 1),
            Stroke::new(2.0, p.vurgu),
            StrokeKind::Outside,
        );
    }

    let mut x = rect.left() + tasarim::BOSLUK_MD;
    if let Some(ikon) = ikon {
        let ikon_rect = egui::Rect::from_center_size(
            pos2(x + 8.5, rect.center().y),
            Vec2::splat(16.0),
        );
        ikon::ciz(ui.painter(), ikon_rect, ikon, metin_rengi);
        x += ikon_alani + aralik;
    }
    ui.painter().galley(
        pos2(x, rect.center().y - metin.size().y / 2.0),
        metin,
        metin_rengi,
    );
    if let Some(kisayol) = kisayol {
        let kw = kisayol.size().x + tasarim::BOSLUK_SM * 2.0;
        let krect = egui::Rect::from_min_size(
            pos2(rect.right() - tasarim::BOSLUK_MD - kw, rect.center().y - 9.0),
            vec2(kw, 18.0),
        );
        ui.painter()
            .rect_filled(krect, CornerRadius::same(tasarim::YARICAP_SM), p.yuzey_2);
        ui.painter().galley(
            pos2(krect.left() + tasarim::BOSLUK_SM, krect.center().y - kisayol.size().y / 2.0),
            kisayol,
            p.soluk,
        );
    }

    yanit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, etiket));
    yanit
}

/// Düğme yanıtının klavye veya fareyle tetiklenip tetiklenmediği.
pub fn tiklandi(ui: &Ui, yanit: &Response) -> bool {
    yanit.clicked()
        || (yanit.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)))
}

pub fn kbd(ui: &mut Ui, metin: &str) -> Response {
    let p = Palet::ui(ui);
    egui::Frame::NONE
        .fill(p.yuzey_2)
        .stroke(Stroke::new(1.0, p.kenar))
        .corner_radius(CornerRadius::same(tasarim::YARICAP_SM))
        .inner_margin(Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(
                RichText::new(metin)
                    .monospace()
                    .size(10.0)
                    .color(p.soluk),
            );
        })
        .response
}

pub fn rozet(ui: &mut Ui, metin: &str, tur: RozetTuru) -> Response {
    let p = Palet::ui(ui);
    let (metin_rengi, dolgu) = match tur {
        RozetTuru::Uyari => (p.uyari, Color32::from_rgba_unmultiplied(230, 180, 80, 24)),
        RozetTuru::Bilgi => (p.bilgi, Color32::from_rgba_unmultiplied(123, 184, 255, 22)),
    };
    egui::Frame::NONE
        .fill(dolgu)
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(metin).size(10.5).strong().color(metin_rengi));
        })
        .response
}

pub fn bolum_basligi(ui: &mut Ui, baslik: &str, aciklama: Option<&str>) {
    let p = Palet::ui(ui);
    ui.label(
        RichText::new(baslik.to_uppercase())
            .text_style(TextStyle::Name("bolum".into()))
            .color(p.cok_soluk),
    );
    if let Some(aciklama) = aciklama {
        ui.label(RichText::new(aciklama).small().color(p.soluk));
    }
}

pub fn bilgi_satiri(ui: &mut Ui, etiket: &str, deger: &str) {
    let p = Palet::ui(ui);
    ui.horizontal(|ui| {
        ui.label(RichText::new(etiket).small().color(p.soluk));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(deger).small().color(p.metin)).truncate());
        });
    });
}

pub fn kart<R>(ui: &mut Ui, ekle: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let p = Palet::ui(ui);
    egui::Frame::NONE
        .fill(p.panel_opak)
        .stroke(Stroke::new(1.0, p.kenar))
        .corner_radius(CornerRadius::same(tasarim::YARICAP_XL))
        .inner_margin(Margin::same(tasarim::BOSLUK_XL as i8))
        .show(ui, ekle)
}

/// shadcn tarzı anahtar; değiştiyse `true` döner.
pub fn anahtar(ui: &mut Ui, acik: &mut bool, etiket: &str) -> bool {
    let p = Palet::ui(ui);
    let (rect, yanit) = ui.allocate_exact_size(vec2(38.0, 22.0), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, *acik, 0.14);
    let dolgu = renk_karistir(p.yuzey_3, p.vurgu, t);
    ui.painter()
        .rect_filled(rect, CornerRadius::same(11), dolgu);
    let x = rect.left() + 2.0 + (rect.width() - 22.0) * t;
    ui.painter().circle_filled(
        pos2(x + 9.0, rect.center().y),
        9.0,
        if t > 0.5 { p.vurgu_metin } else { p.panel_opak },
    );
    if yanit.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(13),
            Stroke::new(2.0, p.vurgu),
            StrokeKind::Outside,
        );
    }
    let tiklandi = tiklandi(ui, &yanit);
    if tiklandi {
        *acik = !*acik;
    }
    yanit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Checkbox, true, etiket));
    tiklandi
}

/// Segmentli seçim; değiştiyse `true` döner.
pub fn segment<T: Copy + PartialEq>(
    ui: &mut Ui,
    secenekler: &[(T, &str)],
    secili: &mut T,
) -> bool {
    let p = Palet::ui(ui);
    let mut degisti = false;
    egui::Frame::NONE
        .fill(p.yuzey_2)
        .stroke(Stroke::new(1.0, p.kenar))
        .corner_radius(CornerRadius::same(tasarim::YARICAP_LG))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (deger, etiket) in secenekler {
                    let sec = *secili == *deger;
                    let yanit = ui.add(
                        egui::Button::new(
                            RichText::new(*etiket)
                                .size(11.0)
                                .color(if sec { p.metin } else { p.soluk }),
                        )
                        .frame(false)
                        .fill(if sec { p.yuzey } else { Color32::TRANSPARENT })
                        .corner_radius(CornerRadius::same(tasarim::YARICAP_MD))
                        .min_size(vec2(52.0, 28.0)),
                    );
                    if yanit.clicked() {
                        *secili = *deger;
                        degisti = true;
                    }
                }
            });
        });
    degisti
}

fn renk_karistir(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let k = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t) as u8;
    Color32::from_rgb(
        k(a.r(), b.r()),
        k(a.g(), b.g()),
        k(a.b(), b.b()),
    )
}

/// Boş durum kartı; tıklanan eylemi döndürür.
pub fn bos_durum(
    ui: &mut Ui,
    ikon: Ikon,
    baslik: &str,
    aciklama: &str,
    dugmeler: &[(&str, Eylem, DugmeTuru, Option<&str>)],
) -> Option<Eylem> {
    let mut secilen = None;
    ui.vertical_centered(|ui| {
        kart(ui, |ui| {
            ui.set_max_width(390.0);
            ui.vertical_centered(|ui| {
                let p = Palet::ui(ui);
                let (r, _) = ui.allocate_exact_size(vec2(52.0, 52.0), Sense::hover());
                ui.painter().rect_filled(
                    r,
                    CornerRadius::same(tasarim::YARICAP_XL),
                    p.vurgu_yumusak,
                );
                ikon::ciz(ui.painter(), r.shrink(12.0), ikon, p.vurgu);
                ui.add_space(tasarim::BOSLUK_MD);
                ui.heading(baslik);
                ui.add_space(tasarim::BOSLUK_XS);
                ui.label(RichText::new(aciklama).color(p.soluk).size(12.0));
                ui.add_space(tasarim::BOSLUK_LG);
                ui.horizontal(|ui| {
                    for (etiket, eylem, tur, kisayol) in dugmeler {
                        let yanit = dugme(ui, etiket, *tur, None, *kisayol);
                        if tiklandi(ui, &yanit) {
                            secilen = Some(*eylem);
                        }
                    }
                });
            });
        });
    });
    secilen
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dugme_turleri_ayrisir() {
        assert_ne!(DugmeTuru::Birincil, DugmeTuru::Ikincil);
        assert_ne!(DugmeTuru::Hayalet, DugmeTuru::Tehlike);
    }
}
