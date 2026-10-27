//! Üst komut çubuğu ve alt durum çubuğu.
//!
//! Üst çubuk marka, menüler, hızlı işlemler, arama ve "Daha fazla" menüsünü taşır;
//! dosya adı ve konum alt durum çubuğunda gösterilir. Tüm simgeler vektördür.

use egui::{
    Align, Align2, CornerRadius, FontFamily, FontId, Rect, RichText, Sense, vec2,
};

use crate::cekirdek::ayar::{PanelYeri, Tema};
use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;
use crate::girdi::klavye::TUS_ZOOM_CARPI;
use crate::goruntu::YuklemeAsamasi;

use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet, UST_CUBUK_YUKSEKLIGI};
use super::{HudSonucu, bilesen};

const ILERLEME_GENISLIGI: f32 = 104.0;

pub fn ciz(ui: &mut egui::Ui, durum: &UygulamaDurumu, sonuc: &mut HudSonucu) {
    let p = Palet::ui(ui);
    egui::containers::Panel::top("gorsel-arac-cubugu")
        .frame(egui::Frame::NONE.fill(p.panel_filli))
        .show(ui, |ui| {
            ui.set_min_height(UST_CUBUK_YUKSEKLIGI);
            let dar = ui.available_width() < 1800.0;
            ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                sol_grup(ui, durum, sonuc, p, dar);
                ui.add_space(tasarim::BOSLUK_SM);
                sag_grup(ui, durum, sonuc, dar);
            });
            ui.painter().hline(
                ui.max_rect().x_range(),
                ui.max_rect().bottom() - 0.5,
                egui::Stroke::new(1.0, p.kenar),
            );
        });

    egui::containers::Panel::bottom("gorsel-durum-cubugu")
        .frame(egui::Frame::NONE.fill(p.panel_filli))
        .show(ui, |ui| {
            ui.set_min_height(tasarim::ALT_CUBUK_YUKSEKLIGI);
            alt_icerik(ui, durum);
            ui.painter().hline(
                ui.max_rect().x_range(),
                ui.max_rect().top() + 0.5,
                egui::Stroke::new(1.0, p.kenar),
            );
        });
}

fn sol_grup(
    ui: &mut egui::Ui,
    durum: &UygulamaDurumu,
    sonuc: &mut HudSonucu,
    p: Palet,
    dar: bool,
) {
    ui.spacing_mut().item_spacing.x = tasarim::BOSLUK_SM;
    let (logo_rect, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
    ui.painter().rect_filled(
        logo_rect,
        CornerRadius::same(tasarim::YARICAP_LG),
        p.vurgu_yumusak,
    );
    ikon::ciz(ui.painter(), logo_rect.shrink(7.0), Ikon::Dosya, p.vurgu);
    if !dar {
        ui.label(RichText::new("Görsel").size(14.0).strong());
        ui.menu_button(
            RichText::new("Dosya").size(12.0).color(p.soluk),
            |ui| dosya_maddeleri(ui, sonuc),
        );
        ui.menu_button(
            RichText::new("Görünüm").size(12.0).color(p.soluk),
            |ui| gorunum_maddeleri(ui, durum, sonuc),
        );
    } else {
        ui.menu_button(
            RichText::new(format!("{} Menü", ikon::karakter(Ikon::Menu)))
                .size(12.0)
                .color(p.soluk),
            |ui| {
                dosya_maddeleri(ui, sonuc);
                ui.separator();
                gorunum_maddeleri(ui, durum, sonuc);
            },
        );
    }
    ui.separator();
    let ac = bilesen::dugme(
        ui,
        "Aç",
        bilesen::DugmeTuru::Ikincil,
        Some(Ikon::Klasor),
        if dar { None } else { Some("Ctrl O") },
    );
    if bilesen::tiklandi(ui, &ac) {
        sonuc.ekle(Eylem::DosyaAc);
    }
    if !dar {
        let klasor = ikon::ikon_dugme_boyut(
            ui,
            Ikon::Klasor,
            false,
            "Klasör aç (Ctrl+Shift+O)",
            38.0,
        );
        if bilesen::tiklandi(ui, &klasor) {
            sonuc.ekle(Eylem::KlasorAc);
        }
    }
}

fn sag_grup(
    ui: &mut egui::Ui,
    durum: &UygulamaDurumu,
    sonuc: &mut HudSonucu,
    dar: bool,
) {
    let p = Palet::ui(ui);
    let eksi = ikon::ikon_dugme_boyut(ui, Ikon::Eksi, false, "Uzaklaştır", 32.0);
    if bilesen::tiklandi(ui, &eksi) {
        sonuc.ekle(Eylem::Yakinlastir {
            carpan: 1.0 / TUS_ZOOM_CARPI,
            pivot: None,
        });
    }
    ui.add_sized(
        [52.0, 44.0],
        egui::Label::new(
            RichText::new(durum.zoom_metni())
                .monospace()
                .size(11.0)
                .color(p.metin),
        )
        .selectable(false),
    );
    let arti = ikon::ikon_dugme_boyut(ui, Ikon::Arti, false, "Yakınlaştır", 32.0);
    if bilesen::tiklandi(ui, &arti) {
        sonuc.ekle(Eylem::Yakinlastir {
            carpan: TUS_ZOOM_CARPI,
            pivot: None,
        });
    }
    let sigdir = ikon::ikon_dugme_boyut(ui, Ikon::Sigdir, false, "Sığdır (S)", 32.0);
    if bilesen::tiklandi(ui, &sigdir) {
        sonuc.ekle(Eylem::Sigdir);
    }
    ui.separator();
    if !dar && durum.yukleme.is_some() {
        yukleme_gostergesi(ui, durum);
    }
    if dar {
        let komut = ikon::ikon_dugme_boyut(ui, Ikon::Ara, false, "Komut paleti (Ctrl P)", 38.0);
        if bilesen::tiklandi(ui, &komut) {
            sonuc.ekle(Eylem::KomutPaleti);
        }
    } else {
        let komut = bilesen::dugme(
            ui,
            "Komut",
            bilesen::DugmeTuru::Ikincil,
            Some(Ikon::Ara),
            Some("Ctrl P"),
        );
        if bilesen::tiklandi(ui, &komut) {
            sonuc.ekle(Eylem::KomutPaleti);
        }
        let tema = ikon::ikon_dugme_boyut(
            ui,
            Ikon::Tema,
            false,
            tema_ipucu(durum.ayarlar.tema),
            38.0,
        );
        if bilesen::tiklandi(ui, &tema) {
            sonuc.ekle(Eylem::TemaSec(tema_sirasi(durum.ayarlar.tema)));
        }
        let liste = ikon::ikon_dugme_boyut(
            ui,
            Ikon::PanelSol,
            durum.ayarlar.dosya_listesi_acik,
            if durum.ayarlar.dosya_listesi_acik {
                "Dizin listesini gizle (L)"
            } else {
                "Dizin listesini göster (L)"
            },
            38.0,
        );
        if bilesen::tiklandi(ui, &liste) {
            sonuc.ekle(Eylem::DosyaListesiDegistir);
        }
        let bilgi = ikon::ikon_dugme_boyut(
            ui,
            Ikon::PanelSag,
            durum.ayarlar.bilgi_paneli_acik,
            if durum.ayarlar.bilgi_paneli_acik {
                "Bilgi panelini gizle (I)"
            } else {
                "Bilgi panelini göster (I)"
            },
            38.0,
        );
        if bilesen::tiklandi(ui, &bilgi) {
            sonuc.ekle(Eylem::MetaPaneliDegistir);
        }
    }
    let daha = ikon::ikon_dugme_boyut(ui, Ikon::UcNokta, false, "Daha fazla", 38.0);
    egui::Popup::menu(&daha).show(|ui| gorunum_maddeleri(ui, durum, sonuc));
}

fn dosya_maddeleri(ui: &mut egui::Ui, sonuc: &mut HudSonucu) {
    if menu_satiri(ui, Some(Ikon::Dosya), "Aç…", "Ctrl O", false) {
        sonuc.ekle(Eylem::DosyaAc);
    }
    if menu_satiri(ui, Some(Ikon::Klasor), "Klasör Aç…", "Ctrl Shift O", false) {
        sonuc.ekle(Eylem::KlasorAc);
    }
    ui.separator();
    if menu_satiri(ui, Some(Ikon::DisaAc), "Birlikte aç…", "Ctrl E", false) {
        sonuc.ekle(Eylem::BirlikteAc);
    }
    if menu_satiri(ui, Some(Ikon::Yenile), "Yeniden yükle", "R", false) {
        sonuc.ekle(Eylem::YenidenYukle);
    }
    ui.separator();
    if menu_satiri(ui, Some(Ikon::Carpi), "Kapat", "Ctrl W", false) {
        sonuc.ekle(Eylem::Kapat);
    }
}

fn gorunum_maddeleri(ui: &mut egui::Ui, durum: &UygulamaDurumu, sonuc: &mut HudSonucu) {
    if menu_satiri(ui, Some(Ikon::Sigdir), "Sığdır", "S", false) {
        sonuc.ekle(Eylem::Sigdir);
    }
    if menu_satiri(ui, Some(Ikon::Goz), "Gerçek boyut", "G", false) {
        sonuc.ekle(Eylem::GercekBoyut);
    }
    if menu_satiri(ui, Some(Ikon::TamEkran), "Tam ekran", "F11", false) {
        sonuc.ekle(Eylem::TamEkranDegistir);
    }
    ui.separator();
    if menu_satiri(
        ui,
        Some(Ikon::PanelSol),
        if durum.ayarlar.dosya_listesi_acik {
            "Dizin listesini gizle"
        } else {
            "Dizin listesini göster"
        },
        "L",
        durum.ayarlar.dosya_listesi_acik,
    ) {
        sonuc.ekle(Eylem::DosyaListesiDegistir);
    }
    if menu_satiri(
        ui,
        Some(Ikon::PanelSag),
        if durum.ayarlar.dosya_listesi_yeri == PanelYeri::Sol {
            "Dizin listesini sağa taşı"
        } else {
            "Dizin listesini sola taşı"
        },
        "",
        false,
    ) {
        sonuc.ekle(Eylem::DosyaListesiYeriDegistir);
    }
    if menu_satiri(
        ui,
        Some(Ikon::Bilgi),
        if durum.ayarlar.bilgi_paneli_acik {
            "Bilgi panelini gizle"
        } else {
            "Bilgi panelini göster"
        },
        "I",
        durum.ayarlar.bilgi_paneli_acik,
    ) {
        sonuc.ekle(Eylem::MetaPaneliDegistir);
    }
    if menu_satiri(
        ui,
        Some(Ikon::Sirala),
        "Kompakt satırlar",
        "",
        durum.ayarlar.liste_yogunlugu == crate::cekirdek::ayar::Yogunluk::Kompakt,
    ) {
        sonuc.ekle(Eylem::YogunlukDegistir);
    }
    ui.separator();
    for (tema, etiket) in [
        (Tema::Koyu, "Koyu tema"),
        (Tema::Acik, "Açık tema"),
        (Tema::Sistem, "Sistem teması"),
    ] {
        if menu_satiri(
            ui,
            Some(Ikon::Tema),
            etiket,
            "",
            durum.ayarlar.tema == tema,
        ) {
            sonuc.ekle(Eylem::TemaSec(tema));
        }
    }
    ui.separator();
    if menu_satiri(ui, Some(Ikon::Ayar), "Ayarlar…", "Ctrl K", false) {
        sonuc.ekle(Eylem::AyarPenceresi);
    }
}

fn menu_satiri(
    ui: &mut egui::Ui,
    ikon: Option<Ikon>,
    etiket: &str,
    kisayol: &str,
    secili: bool,
) -> bool {
    let p = Palet::ui(ui);
    let (rect, yanit) = ui.allocate_exact_size(vec2(232.0, 32.0), Sense::click());
    if yanit.hovered() || yanit.has_focus() {
        ui.painter().rect_filled(
            rect.shrink2(vec2(4.0, 1.0)),
            CornerRadius::same(tasarim::YARICAP_MD),
            p.yuzey_2,
        );
    }
    let renk = if secili { p.vurgu } else { p.metin };
    let mut x = rect.left() + 12.0;
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
            egui::pos2(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            kisayol,
            FontId::new(10.0, FontFamily::Monospace),
            p.cok_soluk,
        );
    }
    let tiklandi = bilesen::tiklandi(ui, &yanit);
    if tiklandi {
        ui.close();
    }
    tiklandi
}

fn yukleme_gostergesi(ui: &mut egui::Ui, durum: &UygulamaDurumu) {
    let Some(g) = &durum.yukleme else {
        return;
    };
    let p = Palet::ui(ui);
    match g.asama {
        YuklemeAsamasi::Okuma => {
            ui.add(
                egui::ProgressBar::new(g.oran.clamp(0.0, 1.0))
                    .desired_width(ILERLEME_GENISLIGI)
                    .fill(p.vurgu)
                    .show_percentage(),
            );
        }
        YuklemeAsamasi::Cozme => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(format!("{:.1} sn", g.gecen_sn))
                        .monospace()
                        .size(11.0)
                        .color(p.soluk),
                );
            });
        }
    }
}

fn alt_icerik(ui: &mut egui::Ui, durum: &UygulamaDurumu) {
    let p = Palet::ui(ui);
    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
        if let Some(meta) = &durum.aktif {
            ui.label(
                RichText::new(durum.dosya_adi())
                    .size(11.0)
                    .color(p.metin),
            );
            ui.label(
                RichText::new(format!(
                    "{} · {} · {}",
                    crate::goruntu::bicim::bicim_adi(meta.bicim),
                    meta.cozunurluk_metni(),
                    meta.boyut_metni()
                ))
                .size(11.0)
                .color(p.soluk),
            );
            if meta.yonelim > 1 {
                ui.label(
                    RichText::new(format!(
                        "Yönelim: {}",
                        crate::goruntu::meta::yonelim_adi(meta.yonelim)
                    ))
                    .size(11.0)
                    .color(p.cok_soluk),
                );
            }
        } else {
            ui.label(
                RichText::new("Görsel yükleniyor…")
                    .size(11.0)
                    .color(p.cok_soluk),
            );
        }
        if durum.hdr_yuzey {
            bilesen::rozet(ui, "HDR yüzey", bilesen::RozetTuru::Bilgi);
        }
        if durum.hdr_kaynak {
            bilesen::rozet(ui, "HDR kaynak", bilesen::RozetTuru::Bilgi);
        }
        if durum.icc_var() {
            bilesen::rozet(ui, "ICC", bilesen::RozetTuru::Uyari);
        }
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(&durum.adaptor_bilgisi)
                    .size(11.0)
                    .color(p.cok_soluk),
            );
            ui.label(
                RichText::new(format!("{:.1} ms", durum.kare_suresi_ms))
                    .monospace()
                    .size(11.0)
                    .color(p.cok_soluk),
            );
            ui.label(
                RichText::new(durum.zoom_metni())
                    .monospace()
                    .size(11.0)
                    .color(p.soluk),
            );
            ui.label(
                RichText::new(durum.konum_metni())
                    .monospace()
                    .size(11.0)
                    .color(p.cok_soluk),
            );
        });
    });
}

fn tema_sirasi(tema: Tema) -> Tema {
    match tema {
        Tema::Koyu => Tema::Acik,
        Tema::Acik => Tema::Sistem,
        Tema::Sistem => Tema::Koyu,
    }
}

fn tema_ipucu(tema: Tema) -> &'static str {
    match tema {
        Tema::Koyu => "Tema: Koyu — açığa geç",
        Tema::Acik => "Tema: Açık — sisteme geç",
        Tema::Sistem => "Tema: Sistem — koyuya geç",
    }
}

/// Uzun metni ortadan kısaltır: "cok_uzun_dosya_adi.jpg" → "cok_uzun…adi.jpg".
pub fn kisalt(metin: &str, en_cok: usize) -> String {
    let karakter_sayisi = metin.chars().count();
    if karakter_sayisi <= en_cok {
        return metin.to_string();
    }
    let bas = en_cok / 2 - 1;
    let son = en_cok - bas - 1;
    let on: String = metin.chars().take(bas).collect();
    let arka: String = metin.chars().skip(karakter_sayisi - son).collect();
    format!("{on}…{arka}")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kisa_metin_degismez() {
        assert_eq!(kisalt("resim.jpg", 48), "resim.jpg");
        assert_eq!(kisalt("", 10), "");
    }

    #[test]
    fn uzun_metin_ortadan_kisalir() {
        let uzun = "cok_uzun_bir_dosya_adi_ile_ornek_goruntu_2026.jpg";
        let kisa = kisalt(uzun, 20);
        assert_eq!(kisa.chars().count(), 20);
        assert!(kisa.contains('…'));
        assert!(kisa.starts_with("cok_uzun"));
        assert!(kisa.ends_with(".jpg"));
    }

    #[test]
    fn turkce_karakterler_bozulmaz() {
        let metin = "ığüşöç_çok_uzun_dosya_adı_örneği.png";
        let kisa = kisalt(metin, 15);
        assert_eq!(kisa.chars().count(), 15);
        assert!(kisa.contains('ı') || kisa.contains('ğ') || kisa.contains('ü'));
    }

    #[test]
    fn tema_sirasi_dongusel() {
        assert_eq!(tema_sirasi(Tema::Koyu), Tema::Acik);
        assert_eq!(tema_sirasi(Tema::Acik), Tema::Sistem);
        assert_eq!(tema_sirasi(Tema::Sistem), Tema::Koyu);
    }
}
