//! Vektör araç simgeleri: font gliflerine güvenmeyen, çizim tabanlı ikonlar.
//!
//! Lucide çizgi diline yakın 1,75 px strok kullanılır; her simge tema rengini izler.
//! Dokunmatik hedef varsayılanı 44×44, kompakt araç çubuğu için 32×32 varyantı vardır.

use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};

const CIZGI: f32 = 1.75;
const YARICAP_ORANI: f32 = 0.24;

/// Çizim tabanlı araç simgeleri.
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
}

pub fn ikon_dugme_boyut(
    ui: &mut Ui,
    ikon: Ikon,
    secili: bool,
    ipucu: &str,
    boyut: f32,
) -> Response {
    let (dikdortgen, yanit) = ui.allocate_exact_size(vec2(boyut, boyut), Sense::click());
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
    ciz(ui.painter(), dikdortgen, ikon, renk);
    yanit.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, ipucu)
    });
    yanit.on_hover_text(ipucu)
}

/// Simgeyi verilen dikdörtgenin ortasına çizer.
pub fn ciz(painter: &egui::Painter, dikdortgen: Rect, ikon: Ikon, renk: Color32) {
    let merkez = dikdortgen.center();
    let yaricap = dikdortgen.width().min(dikdortgen.height()) * YARICAP_ORANI;
    let cizgi = Stroke::new(CIZGI, renk);
    let k = |x: f32, y: f32| pos2(merkez.x + x * yaricap, merkez.y + y * yaricap);

    let sekiller: Vec<Shape> = match ikon {
        Ikon::Sol => vec![Shape::line(
            vec![k(0.35, -0.65), k(-0.35, 0.0), k(0.35, 0.65)],
            cizgi,
        )],
        Ikon::Sag => vec![Shape::line(
            vec![k(-0.35, -0.65), k(0.35, 0.0), k(-0.35, 0.65)],
            cizgi,
        )],
        Ikon::Arti => vec![
            Shape::line(vec![k(0.0, -0.7), k(0.0, 0.7)], cizgi),
            Shape::line(vec![k(-0.7, 0.0), k(0.7, 0.0)], cizgi),
        ],
        Ikon::Eksi => vec![Shape::line(vec![k(-0.7, 0.0), k(0.7, 0.0)], cizgi)],
        Ikon::Carpi => vec![
            Shape::line(vec![k(-0.6, -0.6), k(0.6, 0.6)], cizgi),
            Shape::line(vec![k(-0.6, 0.6), k(0.6, -0.6)], cizgi),
        ],
        Ikon::UcNokta => vec![
            Shape::circle_filled(k(0.0, -0.6), CIZGI.max(1.6), renk),
            Shape::circle_filled(merkez, CIZGI.max(1.6), renk),
            Shape::circle_filled(k(0.0, 0.6), CIZGI.max(1.6), renk),
        ],
        Ikon::DisaAc => vec![
            Shape::line(
                vec![
                    k(-0.2, -0.85),
                    k(-0.85, -0.85),
                    k(-0.85, 0.85),
                    k(0.85, 0.85),
                    k(0.85, 0.2),
                ],
                cizgi,
            ),
            Shape::line(vec![k(-0.05, 0.05), k(0.8, -0.8)], cizgi),
            Shape::line(vec![k(0.25, -0.85), k(0.85, -0.85), k(0.85, -0.25)], cizgi),
        ],
        Ikon::Sirala => vec![
            yatay(&k, -0.55, 1.0, cizgi),
            yatay(&k, 0.0, 0.62, cizgi),
            yatay(&k, 0.55, 0.28, cizgi),
        ],
        Ikon::Klasor => vec![
            Shape::line(
                vec![
                    k(-0.85, -0.55),
                    k(-0.85, 0.65),
                    k(0.85, 0.65),
                    k(0.85, -0.25),
                    k(-0.1, -0.25),
                    k(-0.35, -0.55),
                    k(-0.85, -0.55),
                ],
                cizgi,
            ),
        ],
        Ikon::Dosya => vec![
            Shape::line(
                vec![
                    k(-0.55, -0.85),
                    k(0.25, -0.85),
                    k(0.65, -0.45),
                    k(0.65, 0.85),
                    k(-0.55, 0.85),
                    k(-0.55, -0.85),
                ],
                cizgi,
            ),
            Shape::line(vec![k(0.25, -0.85), k(0.25, -0.45), k(0.65, -0.45)], cizgi),
        ],
        Ikon::Ara => vec![
            Shape::circle_stroke(k(-0.15, -0.15), yaricap * 0.55, cizgi),
            Shape::line(vec![k(0.3, 0.3), k(0.85, 0.85)], cizgi),
        ],
        Ikon::Tema => vec![
            Shape::circle_stroke(merkez, yaricap * 0.65, cizgi),
            Shape::line(vec![k(0.0, -0.95), k(0.0, -0.65)], cizgi),
            Shape::line(vec![k(0.0, 0.65), k(0.0, 0.95)], cizgi),
            Shape::line(vec![k(-0.95, 0.0), k(-0.65, 0.0)], cizgi),
            Shape::line(vec![k(0.65, 0.0), k(0.95, 0.0)], cizgi),
        ],
        Ikon::PanelSol | Ikon::PanelSag => {
            let sol = ikon == Ikon::PanelSol;
            vec![
                Shape::rect_stroke(
                    Rect::from_center_size(merkez, vec2(yaricap * 1.8, yaricap * 1.55)),
                    CornerRadius::same(2),
                    cizgi,
                    StrokeKind::Inside,
                ),
                Shape::line(
                    vec![
                        k(if sol { -0.25 } else { 0.25 }, -0.75),
                        k(if sol { -0.25 } else { 0.25 }, 0.75),
                    ],
                    cizgi,
                ),
            ]
        }
        Ikon::Yenile => {
            let r = yaricap * 0.68;
            let mut noktalar = Vec::new();
            for i in 0..=18 {
                let a = -2.4 + (i as f32 / 18.0) * 4.6;
                noktalar.push(pos2(merkez.x + a.cos() * r, merkez.y + a.sin() * r));
            }
            vec![
                Shape::line(noktalar, cizgi),
                Shape::line(
                    vec![k(0.35, -0.85), k(0.75, -0.6), k(0.4, -0.25)],
                    cizgi,
                ),
            ]
        }
        Ikon::Ayar => vec![
            Shape::line(vec![k(-0.85, -0.55), k(0.85, -0.55)], cizgi),
            Shape::line(vec![k(-0.85, 0.0), k(0.85, 0.0)], cizgi),
            Shape::line(vec![k(-0.85, 0.55), k(0.85, 0.55)], cizgi),
            Shape::circle_filled(k(-0.2, -0.55), yaricap * 0.18, renk),
            Shape::circle_filled(k(0.35, 0.0), yaricap * 0.18, renk),
            Shape::circle_filled(k(-0.45, 0.55), yaricap * 0.18, renk),
        ],
        Ikon::Bilgi => vec![
            Shape::circle_stroke(merkez, yaricap * 0.78, cizgi),
            Shape::line(vec![k(0.0, -0.15), k(0.0, 0.45)], cizgi),
            Shape::circle_filled(k(0.0, -0.48), yaricap * 0.1, renk),
        ],
        Ikon::TamEkran | Ikon::Sigdir => {
            let d = if ikon == Ikon::Sigdir { 0.55 } else { 0.9 };
            vec![
                Shape::line(vec![k(-d, -0.35), k(-d, -d), k(-0.35, -d)], cizgi),
                Shape::line(vec![k(0.35, -d), k(d, -d), k(d, -0.35)], cizgi),
                Shape::line(vec![k(-d, 0.35), k(-d, d), k(-0.35, d)], cizgi),
                Shape::line(vec![k(0.35, d), k(d, d), k(d, 0.35)], cizgi),
            ]
        }
        Ikon::Onay => vec![Shape::line(
            vec![k(-0.7, 0.0), k(-0.2, 0.55), k(0.75, -0.6)],
            cizgi,
        )],
        Ikon::Uyari => vec![
            Shape::line(
                vec![k(0.0, -0.85), k(-0.85, 0.75), k(0.85, 0.75), k(0.0, -0.85)],
                cizgi,
            ),
            Shape::line(vec![k(0.0, -0.3), k(0.0, 0.3)], cizgi),
            Shape::circle_filled(k(0.0, 0.55), yaricap * 0.1, renk),
        ],
        Ikon::Goz => vec![
            Shape::line(
                vec![
                    k(-0.85, 0.0),
                    k(-0.45, -0.45),
                    k(0.0, -0.6),
                    k(0.45, -0.45),
                    k(0.85, 0.0),
                    k(0.45, 0.45),
                    k(0.0, 0.6),
                    k(-0.45, 0.45),
                    k(-0.85, 0.0),
                ],
                cizgi,
            ),
            Shape::circle_stroke(merkez, yaricap * 0.22, cizgi),
        ],
    };
    painter.extend(sekiller);
}

fn yatay(k: &impl Fn(f32, f32) -> Pos2, y: f32, oran: f32, cizgi: Stroke) -> Shape {
    Shape::line(vec![k(-0.85 * oran, y), k(0.85 * oran, y)], cizgi)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn tum_simgeler_cizilebilir() {
        let painter = egui::Painter::new(
            Default::default(),
            egui::LayerId::new(egui::Order::Background, egui::Id::new("ikon-test")),
            Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0)),
        );
        for ikon in [
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
        ] {
            ciz(
                &painter,
                Rect::from_min_size(pos2(0.0, 0.0), vec2(44.0, 44.0)),
                ikon,
                Color32::WHITE,
            );
        }
    }
}
