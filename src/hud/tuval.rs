//! Görsel tuvali katmanları: boş durum, yükleme kartı ve yüzen yakınlaştırma pilu.

use egui::{Align2, Area, CornerRadius, Margin, Order, RichText, Sense, Stroke, vec2};

use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;
use crate::girdi::klavye::TUS_ZOOM_CARPI;
use crate::goruntu::YuklemeAsamasi;

use super::ikon::{self, Ikon};
use super::tasarim::{self, ALT_CUBUK_YUKSEKLIGI, Palet};
use super::{HudSonucu, bilesen};

pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let ctx = ui.ctx().clone();
    let p = Palet::ctx(&ctx);

    if durum.yukleniyor.is_some() || durum.yukleme.is_some() {
        yukleme_karti(&ctx, durum, p);
    } else if durum.aktif.is_none() {
        bos_ekran(&ctx, durum, sonuc, p);
    } else {
        zoom_pilu(&ctx, durum, sonuc, p);
    }
}

fn yukleme_karti(ctx: &egui::Context, durum: &UygulamaDurumu, p: Palet) {
    let dosya = durum
        .yukleniyor
        .and_then(|i| durum.indeks.get(i))
        .and_then(|y| y.file_name())
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Görsel yükleniyor".to_string());

    Area::new(egui::Id::new("gorsel-tuval-yukleme"))
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .order(Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            ui.set_width(390.0);
            bilesen::kart(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
                    ui.painter().rect_filled(
                        r,
                        CornerRadius::same(tasarim::YARICAP_XL),
                        p.vurgu_yumusak,
                    );
                    ikon::ciz(ui.painter(), r.shrink(12.0), Ikon::Yenile, p.vurgu);
                    ui.add_space(tasarim::BOSLUK_MD);
                    ui.heading("Görsel yükleniyor");
                    ui.add_space(tasarim::BOSLUK_XS);
                    ui.add(
                        egui::Label::new(
                            RichText::new(dosya).color(p.soluk).size(12.0),
                        )
                        .truncate(),
                    );
                    ui.add_space(tasarim::BOSLUK_LG);
                    if let Some(g) = durum.yukleme {
                        match g.asama {
                            YuklemeAsamasi::Okuma => {
                                ui.add(
                                    egui::ProgressBar::new(g.oran.clamp(0.0, 1.0))
                                        .desired_width(260.0)
                                        .show_percentage(),
                                );
                                ui.add_space(tasarim::BOSLUK_XS);
                                ui.label(
                                    RichText::new("Dosya okunuyor")
                                        .small()
                                        .color(p.cok_soluk),
                                );
                            }
                            YuklemeAsamasi::Cozme => {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(
                                        RichText::new(format!(
                                            "Çözümleniyor · {:.1} sn",
                                            g.gecen_sn
                                        ))
                                        .small()
                                        .color(p.soluk),
                                    );
                                });
                            }
                        }
                    } else {
                        ui.spinner();
                    }
                });
            });
        });
}

fn bos_ekran(
    ctx: &egui::Context,
    _durum: &UygulamaDurumu,
    sonuc: &mut HudSonucu,
    _p: Palet,
) {
    Area::new(egui::Id::new("gorsel-tuval-bos"))
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .order(Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            ui.set_width(460.0);
            if let Some(eylem) = bilesen::bos_durum(
                ui,
                Ikon::Dosya,
                "Görsel açık değil",
                "Bir dosya seçin, klasör açın veya görseli pencereye bırakın.",
                &[
                    ("Dosya aç…", Eylem::DosyaAc, bilesen::DugmeTuru::Birincil, Some("Ctrl O")),
                    (
                        "Klasör aç…",
                        Eylem::KlasorAc,
                        bilesen::DugmeTuru::Ikincil,
                        Some("Ctrl Shift O"),
                    ),
                ],
            ) {
                sonuc.ekle(eylem);
            }
        });
}

fn zoom_pilu(ctx: &egui::Context, durum: &UygulamaDurumu, sonuc: &mut HudSonucu, p: Palet) {
    Area::new(egui::Id::new("gorsel-yakinlastirma"))
        .anchor(
            Align2::CENTER_BOTTOM,
            vec2(0.0, -(ALT_CUBUK_YUKSEKLIGI + tasarim::BOSLUK_MD)),
        )
        .order(Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(p.panel_opak)
                .stroke(Stroke::new(1.0, p.kenar_guc))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let eksi = ikon::ikon_dugme_boyut(
                            ui,
                            Ikon::Eksi,
                            false,
                            "Uzaklaştır",
                            30.0,
                        );
                        if bilesen::tiklandi(ui, &eksi) {
                            sonuc.ekle(Eylem::Yakinlastir {
                                carpan: 1.0 / TUS_ZOOM_CARPI,
                                pivot: None,
                            });
                        }
                        ui.add_sized(
                            [52.0, 30.0],
                            egui::Label::new(
                                RichText::new(durum.zoom_metni())
                                    .monospace()
                                    .size(11.0)
                                    .color(p.metin),
                            )
                            .selectable(false),
                        );
                        let arti = ikon::ikon_dugme_boyut(
                            ui,
                            Ikon::Arti,
                            false,
                            "Yakınlaştır",
                            30.0,
                        );
                        if bilesen::tiklandi(ui, &arti) {
                            sonuc.ekle(Eylem::Yakinlastir {
                                carpan: TUS_ZOOM_CARPI,
                                pivot: None,
                            });
                        }
                        ui.separator();
                        let sigdir = egui::Button::new(
                            RichText::new("Sığdır").size(11.0).color(p.soluk),
                        )
                        .frame(false)
                        .min_size(vec2(52.0, 30.0));
                        if ui.add(sigdir).clicked() {
                            sonuc.ekle(Eylem::Sigdir);
                        }
                        let yuz = egui::Button::new(
                            RichText::new("%100").size(11.0).color(p.soluk),
                        )
                        .frame(false)
                        .min_size(vec2(46.0, 30.0));
                        if ui.add(yuz).clicked() {
                            sonuc.ekle(Eylem::GercekBoyut);
                        }
                    });
                });
        });
}
