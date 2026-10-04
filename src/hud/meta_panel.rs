//! Meta veri paneli: dosya, görüntü, renk, önbellek ve aygıt bilgileri.

use crate::cekirdek::ayar::BILGI_GENISLIK_ARALIGI;
use crate::cekirdek::durum::UygulamaDurumu;

use super::HudSonucu;

/// Sürüklenme bittikten sonra kaydedilecek genişlik bilgisinin egui anahtarı.
const GENISLIK_BELLEGI: &str = "gorsel-meta-paneli-genislik";

/// Sağ tarafta açılıp kapanan meta veri panelini çizer; durumu `Ayarlar`'da kalıcıdır.
pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    if !durum.ayarlar.bilgi_paneli_acik {
        return;
    }
    let mut acik = true;
    egui::containers::Panel::right("gorsel-meta-paneli")
        .resizable(true)
        .size_range(egui::Rangef::new(
            *BILGI_GENISLIK_ARALIGI.start(),
            *BILGI_GENISLIK_ARALIGI.end(),
        ))
        .default_size(durum.ayarlar.bilgi_paneli_genislik)
        .show(ui, |ui| {
            // Genişlik ayarı sürükleme bitince tek kez diske yazılır.
            genisligi_tazele(ui, durum, sonuc);
            ui.horizontal(|ui| {
                ui.heading("Bilgi");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if super::ikon::ikon_dugme(
                        ui,
                        super::ikon::Ikon::Carpi,
                        "Paneli kapat (I)",
                    ) {
                        acik = false;
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                match &durum.aktif {
                    Some(meta) => {
                        satir(ui, "Dosya", &meta.dosya_adi);
                        satir(ui, "Biçim", crate::goruntu::bicim::bicim_adi(meta.bicim));
                        satir(ui, "Çözünürlük", &meta.cozunurluk_metni());
                        satir(ui, "Megapiksel", &meta.megapiksel_metni());
                        satir(ui, "Boyut", &meta.boyut_metni());
                        satir(
                            ui,
                            "Yönelim",
                            crate::goruntu::meta::yonelim_adi(meta.yonelim),
                        );
                        satir(
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
                        satir(
                            ui,
                            "Dinamik aralık",
                            if meta.hdr { "HDR (doğrusal)" } else { "SDR" },
                        );
                        if meta.kucultuldu {
                            satir(
                                ui,
                                "Ön küçültme",
                                &format!(
                                    "{}×{} → {}×{}",
                                    meta.ham_genislik,
                                    meta.ham_yukseklik,
                                    meta.genislik,
                                    meta.yukseklik
                                ),
                            );
                        }
                        satir(ui, "Yakınlaştırma", &durum.zoom_metni());
                        satir(ui, "Dizin", &durum.konum_metni());
                    }
                    None => {
                        ui.label(egui::RichText::new("Meta veri bekleniyor…").weak());
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Önbellek").strong());
                satir(
                    ui,
                    "RAM önbelleği",
                    &format!("{} MB sınır", durum.ayarlar.ram_onbellek_mb),
                );
                satir(
                    ui,
                    "VRAM komşu dokusu",
                    &match durum.vram_komsu {
                        0 => "Kapalı".to_string(),
                        n => format!("{n} komşu"),
                    },
                );
                satir(
                    ui,
                    "Meta kaydı",
                    &match durum.meta_kayit_sayisi {
                        Some(n) => format!("{n} kayıt"),
                        None => "Veritabanı kapalı".to_string(),
                    },
                );

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Ekran").strong());
                satir(
                    ui,
                    "HDR yüzey",
                    if durum.hdr_yuzey { "Açık" } else { "Kapalı" },
                );
                satir(
                    ui,
                    "Ton haritalama",
                    if durum.hdr_kaynak && durum.hdr_yuzey {
                        "Etkin"
                    } else if durum.hdr_kaynak {
                        "SDR ekrana sıkıştırılıyor"
                    } else {
                        "Gerekmiyor"
                    },
                );
                satir(ui, "Grafik aygıtı", &durum.adaptor_bilgisi);
                satir(
                    ui,
                    "Kare süresi",
                    &format!("{:.1} ms", durum.kare_suresi_ms),
                );

                ui.add_space(8.0);
                ui.separator();
                // Kısayol listesi uzundur: varsayılan kapalı katlanır bölümde tutulur,
                // böylece panelde bilgi satırları ön planda kalır.
                egui::CollapsingHeader::new("Kısayollar")
                    .id_salt("gorsel-kisayollar")
                    .default_open(false)
                    .show(ui, |ui| {
                        for (tus, aciklama) in KISAYOLLAR {
                            satir(ui, tus, aciklama);
                        }
                    });

                ui.add_space(8.0);
            });
        });

    if !acik {
        durum.ayarlar.bilgi_paneli_acik = false;
        sonuc.ayar_degisti = true;
    }
}

/// Panelin ölçülen genişliğini ayara işler; sürükleme durunca tek kez kaydeder.
fn genisligi_tazele(ui: &egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let olculen = ui.max_rect().width() + 16.0;
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
    let bekliyor = ui.ctx().data_mut(|d| d.remove_temp::<bool>(anahtar));
    if bekliyor == Some(true) {
        sonuc.ayar_degisti = true;
    }
}

/// İki sütunlu bilgi satırı.
fn satir(ui: &mut egui::Ui, etiket: &str, deger: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(etiket).weak().small());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(deger).small())
                .on_hover_text(deger);
        });
    });
}

/// Panelde gösterilen klavye ve fare kısayolları.
const KISAYOLLAR: [(&str, &str); 13] = [
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
];

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kisayol_listesi_dolu() {
        assert_eq!(KISAYOLLAR.len(), 13);
        for (tus, aciklama) in KISAYOLLAR {
            assert!(!tus.is_empty());
            assert!(!aciklama.is_empty());
        }
    }
}
