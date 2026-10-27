//! Geçici bildirimler: sağ altta ikonlu, kapatılabilir toast.

use egui::{Align2, Area, CornerRadius, Margin, Order, RichText, Stroke, vec2};

use crate::cekirdek::durum::UygulamaDurumu;

use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet};
use super::bilesen;

pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu) {
    let Some(mesaj) = durum.mesaj.as_ref() else {
        return;
    };
    let metin = mesaj.metin.clone();
    let ipucu = mesaj.ipucu.clone();
    let onemli = mesaj.onemli;
    let kalan = mesaj.kalan_oran();
    let ctx = ui.ctx().clone();
    let p = Palet::ctx(&ctx);
    let mut kapat = false;

    Area::new(egui::Id::new("gorsel-bildirim"))
        .anchor(
            Align2::RIGHT_BOTTOM,
            vec2(
                -tasarim::BOSLUK_LG,
                -(tasarim::ALT_CUBUK_YUKSEKLIGI + tasarim::BOSLUK_MD),
            ),
        )
        .order(Order::Foreground)
        .interactable(true)
        .show(&ctx, |ui| {
            egui::Frame::NONE
                .fill(p.panel_opak)
                .stroke(Stroke::new(
                    1.0,
                    if onemli { p.tehlike } else { p.kenar_guc },
                ))
                .corner_radius(CornerRadius::same(tasarim::YARICAP_XL))
                .inner_margin(Margin::same(tasarim::BOSLUK_MD as i8))
                .show(ui, |ui| {
                    ui.set_width(360.0);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(26.0, 26.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            r,
                            CornerRadius::same(tasarim::YARICAP_LG),
                            if onemli {
                                egui::Color32::from_rgba_unmultiplied(240, 122, 122, 26)
                            } else {
                                p.vurgu_yumusak
                            },
                        );
                        ikon::ciz(
                            ui.painter(),
                            r.shrink(6.0),
                            if onemli { Ikon::Uyari } else { Ikon::Onay },
                            if onemli { p.tehlike } else { p.vurgu },
                        );
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&metin).size(12.0).strong());
                            if let Some(ipucu) = &ipucu {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(ipucu).size(11.0).color(p.soluk),
                                    )
                                    .truncate(),
                                );
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            let kapat_yanit = ikon::ikon_dugme_boyut(
                                ui,
                                Ikon::Carpi,
                                false,
                                "Bildirimi kapat",
                                28.0,
                            );
                            if bilesen::tiklandi(ui, &kapat_yanit) {
                                kapat = true;
                            }
                        });
                    });
                    ui.add_space(tasarim::BOSLUK_XS);
                    let (cubuk, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), 2.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(
                        cubuk,
                        CornerRadius::same(1),
                        p.yuzey_3,
                    );
                    if kalan > 0.0 {
                        let dolu = egui::Rect::from_min_size(
                            cubuk.min,
                            vec2(cubuk.width() * kalan, cubuk.height()),
                        );
                        ui.painter().rect_filled(
                            dolu,
                            CornerRadius::same(1),
                            if onemli { p.tehlike } else { p.vurgu },
                        );
                    }
                });
        });

    if kapat {
        durum.mesaj = None;
    }
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
