//! Komut paleti: tüm eylemleri gruplu, aranabilir ve klavyeyle gezilebilir sunar.

use egui::{
    Align, Align2, Color32, CornerRadius, FontFamily, FontId, Margin, Response, RichText, Sense,
    Stroke, Ui, Vec2, pos2, vec2,
};

use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet};
use super::{HudSonucu, bilesen};

pub struct Komut {
    pub etiket: &'static str,
    pub ipucu: &'static str,
    pub grup: &'static str,
    pub ikon: Ikon,
    pub kisayol: &'static str,
    pub eylem: Eylem,
}

pub const KOMUTLAR: &[Komut] = &[
    Komut {
        etiket: "Dosya aç…",
        ipucu: "Bir görsel dosyası seç",
        grup: "Dosya",
        ikon: Ikon::Dosya,
        kisayol: "Ctrl O",
        eylem: Eylem::DosyaAc,
    },
    Komut {
        etiket: "Klasör aç…",
        ipucu: "Bir klasördeki görselleri listele",
        grup: "Dosya",
        ikon: Ikon::Klasor,
        kisayol: "Ctrl Shift O",
        eylem: Eylem::KlasorAc,
    },
    Komut {
        etiket: "Birlikte aç…",
        ipucu: "Görseli harici bir uygulamada aç",
        grup: "Dosya",
        ikon: Ikon::DisaAc,
        kisayol: "Ctrl E",
        eylem: Eylem::BirlikteAc,
    },
    Komut {
        etiket: "Yeniden yükle",
        ipucu: "Görseli ve meta veriyi tazele",
        grup: "Dosya",
        ikon: Ikon::Yenile,
        kisayol: "R",
        eylem: Eylem::YenidenYukle,
    },
    Komut {
        etiket: "Sığdır",
        ipucu: "Görseli pencereye sığdır",
        grup: "Görünüm",
        ikon: Ikon::Sigdir,
        kisayol: "S",
        eylem: Eylem::Sigdir,
    },
    Komut {
        etiket: "Gerçek boyut (%100)",
        ipucu: "Görseli piksel piksel göster",
        grup: "Görünüm",
        ikon: Ikon::Goz,
        kisayol: "G",
        eylem: Eylem::GercekBoyut,
    },
    Komut {
        etiket: "Tam ekran",
        ipucu: "Pencereyi tam ekran kipine al",
        grup: "Görünüm",
        ikon: Ikon::TamEkran,
        kisayol: "F11",
        eylem: Eylem::TamEkranDegistir,
    },
    Komut {
        etiket: "Satır yoğunluğunu değiştir",
        ipucu: "Dizin listesini rahat veya kompakt göster",
        grup: "Görünüm",
        ikon: Ikon::Sirala,
        kisayol: "",
        eylem: Eylem::YogunlukDegistir,
    },
    Komut {
        etiket: "Dizin listesini aç / kapat",
        ipucu: "Sol dizin panelini göster veya gizle",
        grup: "Paneller",
        ikon: Ikon::PanelSol,
        kisayol: "L",
        eylem: Eylem::DosyaListesiDegistir,
    },
    Komut {
        etiket: "Bilgi panelini aç / kapat",
        ipucu: "Meta veri ve sistem bilgilerini göster",
        grup: "Paneller",
        ikon: Ikon::PanelSag,
        kisayol: "I",
        eylem: Eylem::MetaPaneliDegistir,
    },
    Komut {
        etiket: "Dizin panelini kenara taşı",
        ipucu: "Paneli sol veya sağ kenara al",
        grup: "Paneller",
        ikon: Ikon::PanelSag,
        kisayol: "",
        eylem: Eylem::DosyaListesiYeriDegistir,
    },
    Komut {
        etiket: "Ayarlar…",
        ipucu: "Görünüm, performans ve kabuk seçenekleri",
        grup: "Uygulama",
        ikon: Ikon::Ayar,
        kisayol: "Ctrl K",
        eylem: Eylem::AyarPenceresi,
    },
];

pub fn komutlari_filtrele(arama: &str) -> Vec<&'static Komut> {
    let sorgu = arama.trim().to_lowercase();
    KOMUTLAR
        .iter()
        .filter(|komut| {
            sorgu.is_empty()
                || komut.etiket.to_lowercase().contains(&sorgu)
                || komut.ipucu.to_lowercase().contains(&sorgu)
        })
        .collect()
}

pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let ctx = ui.ctx().clone();
    let p = Palet::ctx(&ctx);
    let mut arama = std::mem::take(&mut durum.komut_arama);
    let mut secili = durum.komut_secili;
    let mut kapat = false;
    let mut secilen: Option<Eylem> = None;

    let modal = egui::Modal::new(egui::Id::new("gorsel-komut-paleti"))
        .backdrop_color(Color32::from_black_alpha(120))
        .frame(
            egui::Frame::NONE
                .fill(p.panel_opak)
                .stroke(Stroke::new(1.0, p.kenar_guc))
                .corner_radius(CornerRadius::same(tasarim::YARICAP_XL))
                .inner_margin(Margin::same(0)),
        )
        .show(&ctx, |ui| {
            ui.set_width(640.0);
            ui.add_space(tasarim::BOSLUK_SM);
            let arama_id = egui::Id::new("gorsel-komut-arama");
            ui.memory_mut(|m| m.request_focus(arama_id));

            let asagi = ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
            });
            let yukari = ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
            });
            let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                kapat = true;
            }

            ui.horizontal(|ui| {
                ui.add_space(tasarim::BOSLUK_MD);
                let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                ikon::ciz(ui.painter(), r, Ikon::Ara, p.soluk);
                let yanit = ui.add(
                    egui::TextEdit::singleline(&mut arama)
                        .id(arama_id)
                        .hint_text("Komut ara…")
                        .frame(egui::Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Body),
                );
                if yanit.changed() {
                    secili = 0;
                }
                let kapat_yanit = ikon::ikon_dugme_boyut(ui, Ikon::Carpi, false, "Kapat", 32.0);
                if bilesen::tiklandi(ui, &kapat_yanit) {
                    kapat = true;
                }
                ui.add_space(tasarim::BOSLUK_SM);
            });
            ui.add_space(tasarim::BOSLUK_SM);
            ui.separator();

            let gorunur = komutlari_filtrele(&arama);
            if gorunur.is_empty() {
                ui.add_space(tasarim::BOSLUK_XL);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Eşleşen komut yok")
                            .color(p.soluk)
                            .size(12.0),
                    );
                });
                ui.add_space(tasarim::BOSLUK_XL);
            } else {
                secili = secili.min(gorunur.len() - 1);
                if asagi {
                    secili = (secili + 1).min(gorunur.len() - 1);
                }
                if yukari {
                    secili = secili.saturating_sub(1);
                }
                let mut son_grup = "";
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (sira, komut) in gorunur.iter().enumerate() {
                            if komut.grup != son_grup {
                                son_grup = komut.grup;
                                ui.add_space(tasarim::BOSLUK_XS);
                                ui.label(
                                    RichText::new(komut.grup.to_uppercase())
                                        .text_style(egui::TextStyle::Name("bolum".into()))
                                        .color(p.cok_soluk),
                                );
                                ui.add_space(tasarim::BOSLUK_XS);
                            }
                            let yanit = komut_satiri(
                                ui,
                                komut,
                                sira == secili,
                                p,
                            );
                            if yanit.clicked() || (sira == secili && enter) {
                                secilen = Some(komut.eylem);
                            }
                            if sira == secili && asagi {
                                yanit.scroll_to_me(Some(Align::Center));
                            }
                        }
                    });
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.add_space(tasarim::BOSLUK_MD);
                bilesen::kbd(ui, "↑");
                bilesen::kbd(ui, "↓");
                ui.label(RichText::new("gez").small().color(p.cok_soluk));
                ui.add_space(tasarim::BOSLUK_SM);
                bilesen::kbd(ui, "Enter");
                ui.label(RichText::new("çalıştır").small().color(p.cok_soluk));
                ui.add_space(tasarim::BOSLUK_SM);
                bilesen::kbd(ui, "Esc");
                ui.label(RichText::new("kapat").small().color(p.cok_soluk));
            });
            ui.add_space(tasarim::BOSLUK_SM);
        });

    durum.komut_arama = arama;
    durum.komut_secili = secili;

    if let Some(eylem) = secilen {
        sonuc.ekle(eylem);
        sonuc.ekle(Eylem::KomutPaletiKapat);
    } else if kapat || modal.should_close() {
        sonuc.ekle(Eylem::KomutPaletiKapat);
    }
}

fn komut_satiri(ui: &mut Ui, komut: &Komut, secili: bool, p: Palet) -> Response {
    let (rect, yanit) = ui.allocate_exact_size(
        vec2(ui.available_width(), 40.0),
        Sense::click(),
    );
    if secili || yanit.hovered() {
        let dolgu = if secili {
            p.vurgu_yumusak
        } else {
            p.yuzey_2
        };
        ui.painter().rect_filled(
            rect.shrink2(vec2(4.0, 2.0)),
            CornerRadius::same(tasarim::YARICAP_LG),
            dolgu,
        );
    }
    let ikon_rect = egui::Rect::from_center_size(
        pos2(rect.left() + 22.0, rect.center().y),
        Vec2::splat(16.0),
    );
    ikon::ciz(
        ui.painter(),
        ikon_rect,
        komut.ikon,
        if secili { p.vurgu } else { p.soluk },
    );
    ui.painter().text(
        pos2(rect.left() + 42.0, rect.center().y),
        Align2::LEFT_CENTER,
        komut.etiket,
        FontId::new(13.0, FontFamily::Proportional),
        p.metin,
    );
    if !komut.kisayol.is_empty() {
        let g = ui.painter().layout_no_wrap(
            komut.kisayol.to_owned(),
            FontId::new(10.0, FontFamily::Monospace),
            p.soluk,
        );
        let kw = g.size().x + 12.0;
        let krect = egui::Rect::from_center_size(
            pos2(rect.right() - 12.0 - kw / 2.0, rect.center().y),
            vec2(kw, 18.0),
        );
        ui.painter()
            .rect_filled(krect, CornerRadius::same(tasarim::YARICAP_SM), p.yuzey_2);
        ui.painter().galley(
            pos2(krect.left() + 6.0, rect.center().y - g.size().y / 2.0),
            g,
            p.soluk,
        );
    }
    yanit.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, komut.etiket)
    });
    yanit
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bos_sorgu_tum_komutlari_dondurur() {
        assert_eq!(komutlari_filtrele("").len(), KOMUTLAR.len());
        assert_eq!(komutlari_filtrele("   ").len(), KOMUTLAR.len());
    }

    #[test]
    fn komut_adi_ve_ipucu_ile_filtreler() {
        let sonuc = komutlari_filtrele("sığdır");
        assert_eq!(sonuc.len(), 1);
        assert_eq!(sonuc[0].eylem, Eylem::Sigdir);

        let sonuc = komutlari_filtrele("meta veri");
        assert!(sonuc.iter().any(|k| k.eylem == Eylem::MetaPaneliDegistir));
    }

    #[test]
    fn eslesmeyen_sorgu_bos_doner() {
        assert!(komutlari_filtrele("kesinlikle-olmayan-komut").is_empty());
    }
}
