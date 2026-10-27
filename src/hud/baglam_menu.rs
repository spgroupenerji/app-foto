//! Sağ tık bağlam menüsü: ikonlu, kısayol ipuçlu gruplu eylemler.

use egui::{
    Align2, Area, CornerRadius, FontFamily, FontId, Margin, Order, Rect, Sense, Stroke,
    vec2,
};

use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet};
use super::HudSonucu;

pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let Some((x, y)) = durum.baglam_menusu else {
        return;
    };
    let ctx = ui.ctx().clone();
    let p = Palet::ctx(&ctx);
    let mut kapat = false;

    let ic = Area::new(egui::Id::new("gorsel-baglam-menusu"))
        .order(Order::Foreground)
        .fixed_pos(egui::pos2(x, y))
        .constrain(true)
        .show(&ctx, |ui| {
            egui::Frame::NONE
                .fill(p.panel_opak)
                .stroke(Stroke::new(1.0, p.kenar_guc))
                .corner_radius(CornerRadius::same(tasarim::YARICAP_XL))
                .inner_margin(Margin::same(4))
                .show(ui, |ui| {
                    ui.set_max_width(232.0);
                    ui.vertical(|ui| {
                        if madde(ui, Some(Ikon::Sol), "Önceki görsel", "Sol ok", false, p) {
                            sonuc.ekle(Eylem::Onceki);
                            kapat = true;
                        }
                        if madde(ui, Some(Ikon::Sag), "Sonraki görsel", "Sağ ok", false, p) {
                            sonuc.ekle(Eylem::Sonraki);
                            kapat = true;
                        }
                        ui.separator();
                        if madde(ui, Some(Ikon::Sigdir), "Sığdır", "S", false, p) {
                            sonuc.ekle(Eylem::Sigdir);
                            kapat = true;
                        }
                        if madde(ui, Some(Ikon::Goz), "Gerçek boyut", "G", false, p) {
                            sonuc.ekle(Eylem::GercekBoyut);
                            kapat = true;
                        }
                        if madde(
                            ui,
                            Some(Ikon::TamEkran),
                            if durum.tam_ekran {
                                "Pencere kipine dön"
                            } else {
                                "Tam ekran"
                            },
                            "F11",
                            durum.tam_ekran,
                            p,
                        ) {
                            sonuc.ekle(Eylem::TamEkranDegistir);
                            kapat = true;
                        }
                        ui.separator();
                        if madde(ui, Some(Ikon::DisaAc), "Birlikte aç…", "Ctrl E", false, p) {
                            sonuc.ekle(Eylem::BirlikteAc);
                            kapat = true;
                        }
                        if madde(ui, Some(Ikon::Yenile), "Yeniden yükle", "R", false, p) {
                            sonuc.ekle(Eylem::YenidenYukle);
                            kapat = true;
                        }
                        ui.separator();
                        if madde(
                            ui,
                            Some(Ikon::PanelSol),
                            if durum.ayarlar.dosya_listesi_acik {
                                "Dizin listesini gizle"
                            } else {
                                "Dizin listesini göster"
                            },
                            "L",
                            durum.ayarlar.dosya_listesi_acik,
                            p,
                        ) {
                            sonuc.ekle(Eylem::DosyaListesiDegistir);
                            kapat = true;
                        }
                        if madde(
                            ui,
                            Some(Ikon::PanelSag),
                            if durum.ayarlar.bilgi_paneli_acik {
                                "Bilgi panelini gizle"
                            } else {
                                "Bilgi panelini göster"
                            },
                            "I",
                            durum.ayarlar.bilgi_paneli_acik,
                            p,
                        ) {
                            sonuc.ekle(Eylem::MetaPaneliDegistir);
                            kapat = true;
                        }
                        ui.separator();
                        if madde(ui, Some(Ikon::Ayar), "Ayarlar…", "Ctrl K", false, p) {
                            sonuc.ekle(Eylem::AyarPenceresi);
                            kapat = true;
                        }
                    });
                });
        });

    let menu_rect = ic.response.rect;
    let basildi = ctx.input(|i| i.pointer.any_pressed());
    let imlec = ctx.input(|i| i.pointer.interact_pos());
    if basildi && !imlec.is_some_and(|p| menu_rect.contains(p)) {
        kapat = true;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        kapat = true;
    }
    if kapat {
        sonuc.ekle(Eylem::BaglamMenusuKapat);
    }
}

fn madde(
    ui: &mut egui::Ui,
    ikon: Option<Ikon>,
    etiket: &str,
    kisayol: &str,
    secili: bool,
    p: Palet,
) -> bool {
    let (rect, yanit) = ui.allocate_exact_size(vec2(224.0, 32.0), Sense::click());
    if yanit.hovered() || yanit.has_focus() {
        ui.painter().rect_filled(
            rect.shrink2(vec2(2.0, 1.0)),
            CornerRadius::same(tasarim::YARICAP_MD),
            p.yuzey_2,
        );
    }
    let renk = if secili { p.vurgu } else { p.metin };
    let mut x = rect.left() + 10.0;
    if let Some(ikon) = ikon {
        let ikon_rect = Rect::from_center_size(
            egui::pos2(x + 7.0, rect.center().y),
            vec2(14.0, 14.0),
        );
        ikon::ciz(ui.painter(), ikon_rect, ikon, renk);
        x += 24.0;
    }
    ui.painter().text(
        egui::pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        etiket,
        FontId::proportional(12.0),
        renk,
    );
    if !kisayol.is_empty() {
        ui.painter().text(
            egui::pos2(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            kisayol,
            FontId::new(10.0, FontFamily::Monospace),
            p.cok_soluk,
        );
    }
    super::bilesen::tiklandi(ui, &yanit)
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::cekirdek::ayar::Ayarlar;

    #[test]
    fn menu_konumu_yoksa_kapali_kabul_edilir() {
        let durum = UygulamaDurumu::yeni(Ayarlar::default());
        assert!(durum.baglam_menusu.is_none());
    }

    #[test]
    fn menu_acilip_kapanabilir() {
        let mut durum = UygulamaDurumu::yeni(Ayarlar::default());
        durum.baglam_menusu = Some((100.0, 100.0));
        assert!(durum.baglam_menusu.is_some());
        durum.baglam_menusu = None;
        assert!(durum.baglam_menusu.is_none());
    }
}
