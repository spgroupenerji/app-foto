//! Bilgi paneli: dosya, görüntü, renk ve sistem sekmeleri.

use egui::{RichText, Stroke};

use crate::cekirdek::ayar::BILGI_GENISLIK_ARALIGI;
use crate::cekirdek::durum::UygulamaDurumu;

use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet};
use super::HudSonucu;

const GENISLIK_BELLEGI: &str = "gorsel-meta-paneli-genislik";
const SEKME_BELLEGI: &str = "gorsel-bilgi-sekmesi";

const SEKMELER: [&str; 4] = ["Dosya", "Görüntü", "Renk", "Sistem"];

pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    if !durum.ayarlar.bilgi_paneli_acik {
        return;
    }
    let p = Palet::ui(ui);
    let mut acik = true;
    let panel_yaniti = egui::containers::Panel::right("gorsel-meta-paneli")
        .resizable(true)
        .size_range(egui::Rangef::new(
            *BILGI_GENISLIK_ARALIGI.start(),
            *BILGI_GENISLIK_ARALIGI.end(),
        ))
        .default_size(durum.ayarlar.bilgi_paneli_genislik)
        .show(ui, |ui| {
            let dosya = durum
                .aktif
                .as_ref()
                .map(|m| m.dosya_adi.clone())
                .unwrap_or_else(|| "Meta veri bekleniyor…".to_string());

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("BİLGİ")
                            .text_style(egui::TextStyle::Name("bolum".into()))
                            .color(p.cok_soluk),
                    );
                    ui.add(
                        egui::Label::new(RichText::new(&dosya).size(12.0).color(p.soluk))
                            .truncate(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let yanit = ikon::ikon_dugme_boyut(
                        ui,
                        Ikon::Carpi,
                        false,
                        "Bilgi panelini kapat (I)",
                        32.0,
                    );
                    if super::bilesen::tiklandi(ui, &yanit) {
                        acik = false;
                    }
                });
            });

            let mut sekme = ui
                .data(|d| d.get_temp::<usize>(egui::Id::new(SEKME_BELLEGI)))
                .unwrap_or(0)
                .min(SEKMELER.len() - 1);
            ui.add_space(tasarim::BOSLUK_SM);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = tasarim::BOSLUK_XS;
                for (sira, ad) in SEKMELER.iter().enumerate() {
                    let secili = sira == sekme;
                    let yanit = ui.add(
                        egui::Button::new(
                            RichText::new(*ad)
                                .size(11.0)
                                .color(if secili { p.metin } else { p.soluk }),
                        )
                        .frame(false)
                        .min_size(egui::vec2(52.0, 30.0)),
                    );
                    if secili {
                        ui.painter().line_segment(
                            [
                                yanit.rect.left_bottom() + egui::vec2(7.0, 0.0),
                                yanit.rect.right_bottom() + egui::vec2(-7.0, 0.0),
                            ],
                            Stroke::new(2.0, p.vurgu),
                        );
                    }
                    if yanit.clicked() {
                        sekme = sira;
                        ui.data_mut(|d| d.insert_temp(egui::Id::new(SEKME_BELLEGI), sira));
                    }
                }
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(tasarim::BOSLUK_SM);
                    match sekme {
                        0 => dosya_sekmesi(ui, durum, p),
                        1 => goruntu_sekmesi(ui, durum, p),
                        2 => renk_sekmesi(ui, durum, p),
                        _ => sistem_sekmesi(ui, durum, p),
                    }
                });

            ui.add_space(tasarim::BOSLUK_XS);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{:.1} ms kare", durum.kare_suresi_ms))
                        .monospace()
                        .size(10.5)
                        .color(p.cok_soluk),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&durum.adaptor_bilgisi)
                                .size(10.5)
                                .color(p.cok_soluk),
                        )
                        .truncate(),
                    );
                });
            });
        });
    genisligi_kaydet(ui, durum, sonuc, panel_yaniti.response.rect.width());

    if !acik {
        durum.ayarlar.bilgi_paneli_acik = false;
        sonuc.ayar_degisti = true;
    }
}

fn dosya_sekmesi(ui: &mut egui::Ui, durum: &UygulamaDurumu, _p: Palet) {
    if let Some(meta) = &durum.aktif {
        super::bilesen::bilgi_satiri(ui, "Biçim", crate::goruntu::bicim::bicim_adi(meta.bicim));
        super::bilesen::bilgi_satiri(ui, "Çözünürlük", &meta.cozunurluk_metni());
        super::bilesen::bilgi_satiri(ui, "Megapiksel", &meta.megapiksel_metni());
        super::bilesen::bilgi_satiri(ui, "Boyut", &meta.boyut_metni());
        super::bilesen::bilgi_satiri(
            ui,
            "Yönelim",
            crate::goruntu::meta::yonelim_adi(meta.yonelim),
        );
        super::bilesen::bilgi_satiri(
            ui,
            "Renk profili",
            if meta.icc_uygulandi {
                "Gömülü ICC dönüştürüldü"
            } else if meta.icc_var {
                "Gömülü ICC (dönüşüm gerekmedi)"
            } else {
                "sRGB varsayıldı"
            },
        );
        if meta.kucultuldu {
            super::bilesen::bilgi_satiri(
                ui,
                "Ön küçültme",
                &format!(
                    "{}×{} → {}×{}",
                    meta.ham_genislik, meta.ham_yukseklik, meta.genislik, meta.yukseklik
                ),
            );
        }
    } else {
        ui.label(
            RichText::new("Görsel yüklendiğinde meta veri burada görünür.")
                .size(12.0)
                .color(Palet::ui(ui).soluk),
        );
    }
    ui.add_space(tasarim::BOSLUK_MD);
    super::bilesen::bolum_basligi(ui, "Önbellek", None);
    super::bilesen::bilgi_satiri(
        ui,
        "RAM sınırı",
        &format!("{} MB", durum.ayarlar.ram_onbellek_mb),
    );
    super::bilesen::bilgi_satiri(
        ui,
        "VRAM komşusu",
        &match durum.vram_komsu {
            0 => "Kapalı".to_string(),
            n => format!("{n} komşu"),
        },
    );
    super::bilesen::bilgi_satiri(
        ui,
        "Meta kaydı",
        &match durum.meta_kayit_sayisi {
            Some(n) => format!("{n} kayıt"),
            None => "Veritabanı kapalı".to_string(),
        },
    );
}

fn goruntu_sekmesi(ui: &mut egui::Ui, durum: &UygulamaDurumu, p: Palet) {
    if let Some(meta) = &durum.aktif {
        super::bilesen::bilgi_satiri(
            ui,
            "Dinamik aralık",
            if meta.hdr { "HDR (doğrusal)" } else { "SDR" },
        );
    }
    super::bilesen::bilgi_satiri(
        ui,
        "Ölçek filtresi",
        match durum.ayarlar.olcek_filtresi {
            crate::cekirdek::ayar::Filtre::CatmullRom => "Catmull-Rom",
            crate::cekirdek::ayar::Filtre::Bilinear => "Bilinear",
            crate::cekirdek::ayar::Filtre::Lanczos3 => "Lanczos3",
        },
    );
    super::bilesen::bilgi_satiri(ui, "Yakınlaştırma", &durum.zoom_metni());
    super::bilesen::bilgi_satiri(
        ui,
        "Varsayılan",
        match durum.ayarlar.varsayilan_zoom {
            crate::cekirdek::ayar::VarsayilanZoom::Sigdir => "Sığdır",
            crate::cekirdek::ayar::VarsayilanZoom::GercekBoyut => "Gerçek boyut",
            crate::cekirdek::ayar::VarsayilanZoom::PencereGenisligi => "Pencere genişliği",
        },
    );
    ui.add_space(tasarim::BOSLUK_MD);
    super::bilesen::bolum_basligi(ui, "Başarım", None);
    super::bilesen::bilgi_satiri(
        ui,
        "Kare süresi",
        &format!("{:.1} ms", durum.kare_suresi_ms),
    );
    super::bilesen::bilgi_satiri(
        ui,
        "Ön yükleme",
        match durum.ayarlar.on_yukleme {
            crate::cekirdek::ayar::OnYuklemeGenisligi::Kapali => "Kapalı",
            crate::cekirdek::ayar::OnYuklemeGenisligi::Dar => "Dar (±1)",
            crate::cekirdek::ayar::OnYuklemeGenisligi::Normal => "Normal (±2)",
            crate::cekirdek::ayar::OnYuklemeGenisligi::Genis => "Geniş (±3)",
        },
    );
    let _ = p;
}

fn renk_sekmesi(ui: &mut egui::Ui, durum: &UygulamaDurumu, _p: Palet) {
    if let Some(meta) = &durum.aktif {
        super::bilesen::bilgi_satiri(
            ui,
            "ICC profili",
            if meta.icc_var { "Gömülü" } else { "Yok (sRGB)" },
        );
        super::bilesen::bilgi_satiri(ui, "Renk uzayı", "sRGB çalışma uzayı");
        super::bilesen::bilgi_satiri(
            ui,
            "Ton haritalama",
            if meta.hdr && durum.hdr_yuzey {
                "Etkin (HDR → ekran)"
            } else if meta.hdr {
                "SDR ekrana sıkıştırılıyor"
            } else {
                "Gerekmiyor"
            },
        );
    }
    super::bilesen::bilgi_satiri(
        ui,
        "Arka plan",
        &format!(
            "#{:02X}{:02X}{:02X}",
            durum.ayarlar.arka_plan[0],
            durum.ayarlar.arka_plan[1],
            durum.ayarlar.arka_plan[2]
        ),
    );
    ui.add_space(tasarim::BOSLUK_MD);
    super::bilesen::bolum_basligi(ui, "Ekran", None);
    super::bilesen::bilgi_satiri(
        ui,
        "HDR yüzey",
        if durum.hdr_yuzey { "Açık" } else { "Kapalı" },
    );
    super::bilesen::bilgi_satiri(
        ui,
        "HDR kaynak",
        if durum.hdr_kaynak { "Evet" } else { "Hayır" },
    );
}

fn sistem_sekmesi(ui: &mut egui::Ui, durum: &UygulamaDurumu, _p: Palet) {
    super::bilesen::bilgi_satiri(ui, "Grafik aygıtı", &durum.adaptor_bilgisi);
    super::bilesen::bilgi_satiri(
        ui,
        "Kare süresi",
        &format!("{:.1} ms", durum.kare_suresi_ms),
    );
    super::bilesen::bilgi_satiri(
        ui,
        "Meta kaydı",
        &match durum.meta_kayit_sayisi {
            Some(n) => format!("{n} kayıt"),
            None => "Veritabanı kapalı".to_string(),
        },
    );
    super::bilesen::bilgi_satiri(
        ui,
        "İlişkilendirme",
        if durum.iliskilendirme_durumu.is_empty() {
            "Bilinmiyor"
        } else {
            &durum.iliskilendirme_durumu
        },
    );
    ui.add_space(tasarim::BOSLUK_MD);
    egui::CollapsingHeader::new(RichText::new("Kısayollar").size(12.0))
        .id_salt("gorsel-kisayollar")
        .default_open(false)
        .show(ui, |ui| {
            for (tus, aciklama) in KISAYOLLAR {
                super::bilesen::bilgi_satiri(ui, tus, aciklama);
            }
        });
}

fn genisligi_kaydet(
    ui: &egui::Ui,
    durum: &mut UygulamaDurumu,
    sonuc: &mut HudSonucu,
    olculen: f32,
) {
    let degisti = (olculen - durum.ayarlar.bilgi_paneli_genislik).abs() > 0.5;
    let anahtar = egui::Id::new(GENISLIK_BELLEGI);
    if degisti {
        durum.ayarlar.bilgi_paneli_genislik = olculen.clamp(
            *BILGI_GENISLIK_ARALIGI.start(),
            *BILGI_GENISLIK_ARALIGI.end(),
        );
        ui.ctx().data_mut(|d| d.insert_temp(anahtar, true));
        return;
    }
    if ui.ctx().data_mut(|d| d.remove_temp::<bool>(anahtar)) == Some(true) {
        sonuc.ayar_degisti = true;
    }
}

pub const KISAYOLLAR: [(&str, &str); 14] = [
    ("Sol / Sağ ok", "Önceki / sonraki görsel"),
    ("PageUp / PageDown", "Önceki / sonraki görsel"),
    ("Home / End", "İlk / son görsel"),
    ("Ctrl + tekerlek", "İmleç odaklı yakınlaştırma"),
    ("Tekerlek", "Gezinme"),
    ("Sol üçte bir tık", "Önceki görsel"),
    ("Sağ üçte bir tık", "Sonraki görsel"),
    ("Orta üçte bir sürükle", "Görseli kaydır"),
    ("Çift tık", "Tam ekran"),
    ("S / G", "Sığdır / gerçek boyut"),
    ("Ctrl + O", "Dosya aç"),
    ("Ctrl + Shift + O", "Klasör aç"),
    ("Ctrl + E", "Birlikte aç"),
    ("Ctrl + P", "Komut paleti"),
];

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kisayol_listesi_dolu() {
        assert_eq!(KISAYOLLAR.len(), 14);
        for (tus, aciklama) in KISAYOLLAR {
            assert!(!tus.is_empty());
            assert!(!aciklama.is_empty());
        }
    }
}
