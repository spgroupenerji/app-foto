//! Dizin listesi paneli: açık dizindeki görselleri sanallaştırılmış satırlarla listeler.
//!
//! Küçük resimler yalnızca görünen satırlar için arka planda üretilir; liste 1.000+ dosyada
//! da yalnızca görünür satırları çizer. Panel genişliği ve satır yoğunluğu kalıcıdır.

use std::path::Path;

use egui::{
    Color32, CornerRadius, FontId, Rect, RichText, Sense, pos2, vec2,
};

use crate::cekirdek::ayar::{LISTE_GENISLIK_ARALIGI, SiralamaTuru, SiralamaYonu};
use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet};
use super::HudSonucu;

const AD_UZUNLUGU: usize = 36;
const EYLEM_ALANI: f32 = 40.0;
const KONUM_BELLEGI: &str = "gorsel-dosya-listesi-konum";
const GENISLIK_BELLEGI: &str = "gorsel-dosya-listesi-genislik";

const SIRALAMA_ETIKETLERI: [(SiralamaTuru, &str); 4] = [
    (SiralamaTuru::Ad, "Ada göre"),
    (SiralamaTuru::Tur, "Türe göre"),
    (SiralamaTuru::Tarih, "Tarihe göre"),
    (SiralamaTuru::Boyut, "Boyuta göre"),
];

pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    if !durum.ayarlar.dosya_listesi_acik {
        return;
    }
    let p = Palet::ui(ui);
    let mut acik = true;
    let kimlik = match durum.ayarlar.dosya_listesi_yeri {
        crate::cekirdek::ayar::PanelYeri::Sol => "gorsel-dosya-listesi-sol",
        crate::cekirdek::ayar::PanelYeri::Sag => "gorsel-dosya-listesi-sag",
    };
    let panel = if durum.ayarlar.dosya_listesi_yeri == crate::cekirdek::ayar::PanelYeri::Sag {
        egui::containers::Panel::right(kimlik)
    } else {
        egui::containers::Panel::left(kimlik)
    };

    let panel_yaniti = panel
        .resizable(true)
        .size_range(egui::Rangef::new(
            *LISTE_GENISLIK_ARALIGI.start(),
            *LISTE_GENISLIK_ARALIGI.end(),
        ))
        .default_size(durum.ayarlar.dosya_listesi_genislik)
        .show(ui, |ui| {
            let satir_yuksekligi = if durum.ayarlar.liste_yogunlugu
                == crate::cekirdek::ayar::Yogunluk::Kompakt
            {
                tasarim::SATIR_KOMPAKT
            } else {
                tasarim::SATIR_RAHAT
            };

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("DİZİN")
                            .text_style(egui::TextStyle::Name("bolum".into()))
                            .color(p.cok_soluk),
                    );
                    let dizin = durum
                        .indeks
                        .first()
                        .and_then(|y| y.parent())
                        .and_then(|d| d.file_name())
                        .map(|a| a.to_string_lossy().into_owned())
                        .map(|ad| format!("{} · {} görsel", ad, durum.indeks.len()))
                        .unwrap_or_else(|| format!("{} görsel", durum.indeks.len()));
                    ui.add(
                        egui::Label::new(RichText::new(dizin).size(12.0).color(p.soluk))
                            .truncate(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let kapat = ikon::ikon_dugme_boyut(
                        ui,
                        Ikon::Carpi,
                        false,
                        "Dizin listesini kapat (L)",
                        32.0,
                    );
                    if super::bilesen::tiklandi(ui, &kapat) {
                        acik = false;
                    }
                    siralama_menusu(ui, durum, sonuc);
                });
            });

            ui.add_space(tasarim::BOSLUK_XS);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · {}",
                        tur_etiketi(durum.ayarlar.siralama_turu),
                        yon_etiketi(durum.ayarlar.siralama_yonu)
                    ))
                    .size(11.0)
                    .color(p.cok_soluk),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let yogunluk = super::bilesen::dugme(
                        ui,
                        durum.ayarlar.liste_yogunlugu.etiket(),
                        super::bilesen::DugmeTuru::Hayalet,
                        None,
                        None,
                    );
                    if super::bilesen::tiklandi(ui, &yogunluk) {
                        sonuc.ekle(Eylem::YogunlukDegistir);
                    }
                });
            });
            ui.separator();

            if durum.indeks.is_empty() {
                ui.add_space(tasarim::BOSLUK_XL);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Bu dizinde desteklenen görsel yok")
                            .size(12.0)
                            .color(p.soluk),
                    );
                });
            } else {
                let onceki = ui
                    .data(|d| d.get_temp::<usize>(egui::Id::new(KONUM_BELLEGI)))
                    .unwrap_or(usize::MAX);
                let mut kaydirma = egui::ScrollArea::vertical().auto_shrink([false, false]);
                if onceki != durum.konum {
                    let gorunur = ui.available_height().max(satir_yuksekligi);
                    let hedef =
                        (durum.konum as f32 * satir_yuksekligi - gorunur * 0.42).max(0.0);
                    kaydirma = kaydirma.vertical_scroll_offset(hedef);
                    ui.data_mut(|d| {
                        d.insert_temp(egui::Id::new(KONUM_BELLEGI), durum.konum)
                    });
                }
                kaydirma.show_rows(
                    ui,
                    satir_yuksekligi,
                    durum.indeks.len(),
                    |ui, aralik| {
                        for sira in aralik {
                            satir_ciz(ui, durum, sira, satir_yuksekligi, sonuc, p);
                        }
                    },
                );
            }

            ui.add_space(tasarim::BOSLUK_XS);
            ui.separator();
            let toplam: u64 = durum.dosya_bilgileri.iter().map(|b| b.boyut).sum();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} görsel", durum.indeks.len()))
                        .size(10.5)
                        .color(p.cok_soluk),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(crate::goruntu::meta::bayt_bicimle(toplam))
                            .size(10.5)
                            .color(p.cok_soluk),
                    );
                });
            });
        });
    genisligi_kaydet(
        ui,
        durum,
        sonuc,
        panel_yaniti.response.rect.width(),
        kimlik,
    );

    if !acik {
        durum.ayarlar.dosya_listesi_acik = false;
        sonuc.ayar_degisti = true;
    }
}

fn siralama_menusu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let ipucu = format!(
        "Sıralama: {} ({}) — değiştirmek için tıklayın",
        tur_etiketi(durum.ayarlar.siralama_turu),
        yon_etiketi(durum.ayarlar.siralama_yonu)
    );
    let yanit = ikon::ikon_dugme_boyut(ui, Ikon::Sirala, false, &ipucu, 32.0);
    egui::Popup::menu(&yanit).show(|ui| {
        let p = Palet::ui(ui);
        ui.label(
            RichText::new("SIRALA")
                .text_style(egui::TextStyle::Name("bolum".into()))
                .color(p.cok_soluk),
        );
        for (tur, etiket) in SIRALAMA_ETIKETLERI {
            let secili = durum.ayarlar.siralama_turu == tur;
            if menu_maddesi(ui, etiket, secili) {
                sonuc.ekle(Eylem::SiralaTuru(tur));
                ui.close();
            }
        }
        ui.separator();
        ui.label(
            RichText::new("YÖN")
                .text_style(egui::TextStyle::Name("bolum".into()))
                .color(p.cok_soluk),
        );
        for (yon, etiket) in [
            (SiralamaYonu::Artan, "Artan (A→Z, eski→yeni)"),
            (SiralamaYonu::Azalan, "Azalan (Z→A, yeni→eski)"),
        ] {
            let secili = durum.ayarlar.siralama_yonu == yon;
            if menu_maddesi(ui, etiket, secili) {
                sonuc.ekle(Eylem::SiralaYonu(yon));
                ui.close();
            }
        }
    });
}

fn menu_maddesi(ui: &mut egui::Ui, etiket: &str, secili: bool) -> bool {
    let p = Palet::ui(ui);
    let metin = if secili {
        format!("{etiket}  ✓")
    } else {
        etiket.to_string()
    };
    ui.add(
        egui::Button::new(
            RichText::new(metin)
                .size(12.0)
                .color(if secili { p.vurgu } else { p.metin }),
        )
        .frame(false)
        .min_size(vec2(210.0, 30.0)),
    )
    .clicked()
}

fn satir_ciz(
    ui: &mut egui::Ui,
    durum: &mut UygulamaDurumu,
    sira: usize,
    satir_yuksekligi: f32,
    sonuc: &mut HudSonucu,
    p: Palet,
) {
    let Some(yol) = durum.indeks.get(sira).cloned() else {
        return;
    };
    let ad = yol
        .file_name()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_else(|| yol.display().to_string());
    let (rect, yanit) = ui.allocate_exact_size(
        vec2(ui.available_width(), satir_yuksekligi),
        Sense::click(),
    );
    let secili = sira == durum.konum;
    let zemin = if secili {
        p.secim
    } else if yanit.hovered() {
        p.yuzey_2
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(
        rect.shrink2(vec2(2.0, 1.0)),
        CornerRadius::same(tasarim::YARICAP_LG),
        zemin,
    );
    if secili {
        ui.painter().rect_filled(
            Rect::from_min_size(
                pos2(rect.left() + 2.0, rect.top() + 10.0),
                vec2(2.0, rect.height() - 20.0),
            ),
            CornerRadius::same(2),
            p.vurgu,
        );
    }

    let kucuk = if satir_yuksekligi <= tasarim::SATIR_KOMPAKT {
        tasarim::KUCUK_RESIM_KOMPAKT
    } else {
        tasarim::KUCUK_RESIM_RAHAT
    };
    let resim = Rect::from_min_size(
        pos2(
            rect.left() + tasarim::BOSLUK_SM + 2.0,
            rect.center().y - kucuk / 2.0,
        ),
        vec2(kucuk, kucuk),
    );
    kucuk_resim_ciz(ui, durum, &yol, sira, resim, sonuc, p);

    let metin_x = resim.right() + tasarim::BOSLUK_SM;
    let metin_rengi = p.metin;
    ui.painter().text(
        pos2(metin_x, rect.top() + 10.0),
        egui::Align2::LEFT_TOP,
        super::arac_cubugu::kisalt(&ad, AD_UZUNLUGU),
        FontId::proportional(12.0),
        metin_rengi,
    );
    let alt = durum
        .bilgi(sira)
        .map(|b| {
            format!(
                "{} · {}",
                crate::goruntu::meta::bayt_bicimle(b.boyut),
                tarih_kisa(b.tarih_ms)
            )
        })
        .unwrap_or_default();
    ui.painter().text(
        pos2(metin_x, rect.top() + satir_yuksekligi - 20.0),
        egui::Align2::LEFT_TOP,
        alt,
        FontId::proportional(10.5),
        p.cok_soluk,
    );

    if durum.en_yeni_yol.as_ref().is_some_and(|y| *y == yol) {
        let rozet = Rect::from_min_size(
            pos2(rect.right() - EYLEM_ALANI - 50.0, rect.center().y - 8.0),
            vec2(42.0, 16.0),
        );
        ui.painter()
            .rect_filled(rozet, CornerRadius::same(5), p.vurgu);
        ui.painter().text(
            rozet.center(),
            egui::Align2::CENTER_CENTER,
            "YENİ",
            FontId::proportional(9.0),
            p.vurgu_metin,
        );
    }

    let eylem = Rect::from_min_size(
        pos2(rect.right() - EYLEM_ALANI, rect.top()),
        vec2(EYLEM_ALANI, satir_yuksekligi),
    );
    ikon::ciz(
        ui.painter(),
        eylem.shrink(10.0),
        Ikon::DisaAc,
        if yanit.hovered() { p.metin } else { p.cok_soluk },
    );

    let yanit = yanit.on_hover_text(&ad);
    if yanit.clicked() {
        let tik = yanit.hover_pos().unwrap_or_default();
        if eylem.contains(tik) {
            sonuc.ekle(Eylem::BirlikteAcSatir(sira));
        } else {
            sonuc.ekle(Eylem::Git(sira));
        }
    }
}

fn kucuk_resim_ciz(
    ui: &egui::Ui,
    durum: &mut UygulamaDurumu,
    yol: &Path,
    sira: usize,
    resim: Rect,
    sonuc: &mut HudSonucu,
    p: Palet,
) {
    if !durum.onizlemeler.contains_key(yol) {
        if durum.ayarlar.onizlemeler && ui.clip_rect().intersects(resim) {
            sonuc.ekle(Eylem::OnIzlemeIste(sira));
        }
        yer_tutucu_ciz(ui, resim, p);
        return;
    }
    let dugum = durum
        .onizlemeler
        .get_mut(yol)
        .expect("varlığı yukarıda denetlendi");
    if dugum.doku.is_none() {
        if let Some((rgba, g, y)) = &dugum.piksel {
            let im =
                egui::ColorImage::from_rgba_unmultiplied([*g as usize, *y as usize], rgba);
            let ad = yol
                .file_name()
                .map(|a| a.to_string_lossy().into_owned())
                .unwrap_or_default();
            dugum.doku = Some(ui.ctx().load_texture(ad, im, egui::TextureOptions::LINEAR));
        }
    }
    let Some(doku) = &dugum.doku else {
        yer_tutucu_ciz(ui, resim, p);
        return;
    };
    let g = doku.size()[0] as f32;
    let y = doku.size()[1] as f32;
    let olcek = (resim.width() / g.max(1.0)).min(resim.height() / y.max(1.0));
    let hedef = Rect::from_center_size(resim.center(), vec2(g * olcek, y * olcek));
    ui.painter().rect_filled(
        resim,
        CornerRadius::same(tasarim::YARICAP_MD),
        p.yuzey_3,
    );
    ui.painter().image(
        doku.id(),
        hedef,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

fn yer_tutucu_ciz(ui: &egui::Ui, rect: Rect, p: Palet) {
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(tasarim::YARICAP_MD),
        p.yuzey_3,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "···",
        FontId::proportional(13.0),
        p.cok_soluk,
    );
}

fn genisligi_kaydet(
    ui: &egui::Ui,
    durum: &mut UygulamaDurumu,
    sonuc: &mut HudSonucu,
    olculen: f32,
    kimlik: &str,
) {
    let degisti = (olculen - durum.ayarlar.dosya_listesi_genislik).abs() > 0.5;
    let anahtar = egui::Id::new(GENISLIK_BELLEGI).with(kimlik);
    if degisti {
        durum.ayarlar.dosya_listesi_genislik = olculen.clamp(
            *LISTE_GENISLIK_ARALIGI.start(),
            *LISTE_GENISLIK_ARALIGI.end(),
        );
        ui.ctx().data_mut(|d| d.insert_temp(anahtar, true));
        return;
    }
    if ui.ctx().data_mut(|d| d.remove_temp::<bool>(anahtar)) == Some(true) {
        sonuc.ayar_degisti = true;
    }
}

pub fn tur_etiketi(tur: SiralamaTuru) -> &'static str {
    match tur {
        SiralamaTuru::Ad => "Ad",
        SiralamaTuru::Tur => "Tür",
        SiralamaTuru::Tarih => "Tarih",
        SiralamaTuru::Boyut => "Boyut",
    }
}

pub fn yon_etiketi(yon: SiralamaYonu) -> &'static str {
    match yon {
        SiralamaYonu::Artan => "artan",
        SiralamaYonu::Azalan => "azalan",
    }
}

fn tarih_kisa(ms: u64) -> String {
    let Ok(t) = time::OffsetDateTime::from_unix_timestamp((ms / 1000) as i64) else {
        return "—".to_string();
    };
    let t = t.to_offset(
        time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC),
    );
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute()
    )
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn tarih_kisa_bicimi_sabit() {
        let metin = tarih_kisa(1_791_072_000_000);
        assert_eq!(metin.len(), 16, "gelen: {metin}");
        assert_eq!(&metin[4..5], "-");
        assert_eq!(&metin[7..8], "-");
        assert_eq!(&metin[10..11], " ");
        assert_eq!(&metin[13..14], ":");
    }

    #[test]
    fn tarih_kisa_gecersiz_damgada_tire_gosterir() {
        assert_eq!(tarih_kisa(u64::MAX / 2), "—");
    }

    #[test]
    fn siralama_etiketleri_dolu() {
        for (tur, etiket) in SIRALAMA_ETIKETLERI {
            assert!(!etiket.is_empty());
            assert!(!tur_etiketi(tur).is_empty());
        }
        assert_eq!(tur_etiketi(SiralamaTuru::Ad), "Ad");
        assert_eq!(yon_etiketi(SiralamaYonu::Artan), "artan");
    }
}
