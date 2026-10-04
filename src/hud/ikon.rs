//! Vektör araç simgeleri: font gliflerine güvenmeyen, çizim tabanlı ikonlar.
//!
//! Sistem fontlarında ☰ 🌙 ⋮ gibi glifler bozuk kutu (tofu) olarak görünebildiği için
//! tüm simgeler egui çizimcisiyle vektörel çizilir; böylece her temada ve her Windows
//! sürümünde aynı modern görünüm garanti edilir. Dokunmatik hedef 44×44 korunur.

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

use super::DOKUNMATIK_HEDEF;

/// Varsayılan simge çizgi kalınlığı.
const CIZGI: f32 = 1.8;

/// Simge çizim kenar payı: simge yarıçapı, hedefin bu oranındadır.
const YARICAP_ORANI: f32 = 0.22;

/// Çizim tabanlı araç simgeleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ikon {
    /// Önceki görsel (sol chevron).
    Sol,
    /// Sonraki görsel (sağ chevron).
    Sag,
    /// Yakınlaştır.
    Arti,
    /// Uzaklaştır.
    Eksi,
    /// Kapat.
    Carpi,
    /// "Daha fazla" taşma menüsü (dikey üç nokta).
    UcNokta,
    /// Harici uygulamada aç (dışarı ok).
    DisaAc,
    /// Sıralama (kısalan çizgiler).
    Sirala,
}

/// Simgeli 44×44 düğme; tıklandıysa `true` döner.
pub fn ikon_dugme(ui: &mut Ui, ikon: Ikon, ipucu: &str) -> bool {
    ikon_dugme_durumlu(ui, ikon, false, ipucu)
}

/// Seçili durumu gösteren simgeli düğme (panel anahtarları için).
pub fn ikon_dugme_durumlu(ui: &mut Ui, ikon: Ikon, secili: bool, ipucu: &str) -> bool {
    ikon_dugme_yanit(ui, ikon, secili, ipucu).clicked()
}

/// Simgeli düğmenin yanıtını döndürür; açılır menüler için çapa olarak kullanılır.
pub fn ikon_dugme_yanit(ui: &mut Ui, ikon: Ikon, secili: bool, ipucu: &str) -> Response {
    let (dikdortgen, yanit) = ui.allocate_exact_size(
        vec2(DOKUNMATIK_HEDEF, DOKUNMATIK_HEDEF),
        Sense::click(),
    );
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
            .add(Shape::rect_filled(dikdortgen, CornerRadius::same(6), dolgu));
    }
    let renk = if secili {
        vizueller.selection.stroke.color
    } else {
        vizueller.text_color()
    };
    ciz(ui.painter(), dikdortgen, ikon, renk);
    yanit.on_hover_text(ipucu)
}

/// Simgeyi verilen dikdörtgenin ortasına çizer (satır eylemleri ve boş durumlar için).
pub fn ciz(painter: &egui::Painter, dikdortgen: Rect, ikon: Ikon, renk: Color32) {
    let merkez = dikdortgen.center();
    let yaricap = dikdortgen.width().min(dikdortgen.height()) * YARICAP_ORANI;
    let cizgi = Stroke::new(CIZGI, renk);
    // Birim koordinattan (−1..=1) simge koordinatına ölçekler.
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
    };
    painter.extend(sekiller);
}

/// Belirtilen oranda genişlikte, ortalanmış yatay çizgi.
fn yatay(
    k: &impl Fn(f32, f32) -> Pos2,
    y: f32,
    oran: f32,
    cizgi: Stroke,
) -> Shape {
    Shape::line(vec![k(-0.85 * oran, y), k(0.85 * oran, y)], cizgi)
}
