//! Üst araç çubuğu ve alt durum çubuğu.

use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::HudSonucu;

/// Araç çubuğu ve durum çubuğunu çizer.
pub fn ciz(ui: &mut egui::Ui, durum: &UygulamaDurumu, sonuc: &mut HudSonucu) {
    ust_cubuk(ui, durum, sonuc);
    alt_cubuk(ui, durum);
}

/// Üst çubuk: gezinme, yakınlaştırma, dosya işlemleri ve görünüm düğmeleri.
fn ust_cubuk(ui: &mut egui::Ui, durum: &UygulamaDurumu, sonuc: &mut HudSonucu) {
    egui::containers::Panel::top("gorsel-arac-cubugu").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            // Menü çubuğu: keşfedilebilir giriş noktası (dosya/klasör açma, görünüm).
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Dosya", |ui| {
                    if menu_maddesi(ui, "Aç…", "Ctrl+O") {
                        sonuc.ekle(Eylem::DosyaAc);
                    }
                    if menu_maddesi(ui, "Klasör Aç…", "Ctrl+Shift+O") {
                        sonuc.ekle(Eylem::KlasorAc);
                    }
                    ui.separator();
                    if menu_maddesi(ui, "Birlikte aç…", "Ctrl+E") {
                        sonuc.ekle(Eylem::BirlikteAc);
                    }
                    if menu_maddesi(ui, "Yeniden yükle", "R") {
                        sonuc.ekle(Eylem::YenidenYukle);
                    }
                    ui.separator();
                    if menu_maddesi(ui, "Kapat", "Ctrl+W") {
                        sonuc.ekle(Eylem::Kapat);
                    }
                });
                ui.menu_button("Görünüm", |ui| {
                    if menu_maddesi(ui, "Sığdır", "S") {
                        sonuc.ekle(Eylem::Sigdir);
                    }
                    if menu_maddesi(ui, "Gerçek boyut", "G") {
                        sonuc.ekle(Eylem::GercekBoyut);
                    }
                    ui.separator();
                    if menu_maddesi(ui, "Tam ekran", "F11") {
                        sonuc.ekle(Eylem::TamEkranDegistir);
                    }
                    if menu_maddesi(
                        ui,
                        if durum.meta_panel_acik {
                            "Bilgi panelini gizle"
                        } else {
                            "Bilgi panelini göster"
                        },
                        "I",
                    ) {
                        sonuc.ekle(Eylem::MetaPaneliDegistir);
                    }
                    ui.separator();
                    if menu_maddesi(ui, "Ayarlar…", "Ctrl+K") {
                        sonuc.ekle(Eylem::AyarPenceresi);
                    }
                });
            });
            ui.separator();

            // En sık kullanılan iki işlem doğrudan araç çubuğunda durur.
            if duz_dugme(ui, "Aç", "Görsel aç (Ctrl+O)") {
                sonuc.ekle(Eylem::DosyaAc);
            }
            if duz_dugme(ui, "Klasör", "Klasör aç (Ctrl+Shift+O)") {
                sonuc.ekle(Eylem::KlasorAc);
            }
            ui.separator();

            if duz_dugme(ui, "◀", "Önceki görsel (Sol ok / PageUp)") {
                sonuc.ekle(Eylem::Onceki);
            }
            if duz_dugme(ui, "▶", "Sonraki görsel (Sağ ok / PageDown)") {
                sonuc.ekle(Eylem::Sonraki);
            }
            ui.separator();

            ui.label(
                egui::RichText::new(durum.konum_metni())
                    .strong()
                    .monospace(),
            );
            ui.separator();

            // Dosya adı: uzun adlar ortadan kısaltılır, tam ad ipucunda gösterilir.
            let ad = durum.dosya_adi();
            ui.label(egui::RichText::new(kisalt(&ad, 48)).strong())
                .on_hover_text(&ad);
            ui.separator();

            if duz_dugme(ui, "−", "Uzaklaştır (Ctrl + -)") {
                sonuc.ekle(Eylem::Yakinlastir {
                    carpan: 1.0 / crate::girdi::klavye::TUS_ZOOM_CARPI,
                    pivot: None,
                });
            }
            ui.label(egui::RichText::new(durum.zoom_metni()).monospace().strong());
            if duz_dugme(ui, "+", "Yakınlaştır (Ctrl + +)") {
                sonuc.ekle(Eylem::Yakinlastir {
                    carpan: crate::girdi::klavye::TUS_ZOOM_CARPI,
                    pivot: None,
                });
            }
            if duz_dugme(ui, "Sığdır", "Görseli pencereye sığdır (S)") {
                sonuc.ekle(Eylem::Sigdir);
            }
            if duz_dugme(ui, "%100", "Gerçek boyut (G)") {
                sonuc.ekle(Eylem::GercekBoyut);
            }
            ui.separator();

            if duz_dugme(ui, "⟳", "Yeniden yükle (R)") {
                sonuc.ekle(Eylem::YenidenYukle);
            }
            if duz_dugme(ui, "Birlikte aç", "Harici uygulamada aç (Ctrl+E)") {
                sonuc.ekle(Eylem::BirlikteAc);
            }
            if duz_dugme(
                ui,
                if durum.tam_ekran {
                    "Pencere"
                } else {
                    "Tam ekran"
                },
                "Tam ekran (F11 / F / çift tık)",
            ) {
                sonuc.ekle(Eylem::TamEkranDegistir);
            }
            if duz_dugme(ui, "Ayarlar", "Ayarlar penceresi (Ctrl + K)") {
                sonuc.ekle(Eylem::AyarPenceresi);
            }

            // Sağa yaslanan rozetler.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if durum.hdr_yuzey {
                    rozet(ui, "HDR", egui::Color32::from_rgb(120, 200, 140));
                }
                if durum.hdr_kaynak {
                    rozet(ui, "HDR kaynak", egui::Color32::from_rgb(150, 180, 240));
                }
                if durum.icc_var() {
                    rozet(ui, "ICC", egui::Color32::from_rgb(220, 190, 120));
                }
                if durum.yukleniyor.is_some() {
                    ui.spinner();
                }
            });
        });
    });
}

/// Alt çubuk: dosya ve görüntü bilgileri ile performans göstergesi.
fn alt_cubuk(ui: &mut egui::Ui, durum: &UygulamaDurumu) {
    egui::containers::Panel::bottom("gorsel-durum-cubugu").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            match &durum.aktif {
                Some(meta) => {
                    ui.label(
                        egui::RichText::new(crate::goruntu::bicim::bicim_adi(meta.bicim)).strong(),
                    );
                    ui.label(meta.cozunurluk_metni());
                    ui.label(meta.megapiksel_metni());
                    ui.label(meta.boyut_metni());
                    if meta.yonelim > 1 {
                        ui.label(format!(
                            "⟲ {}",
                            crate::goruntu::meta::yonelim_adi(meta.yonelim)
                        ));
                    }
                    if meta.ham_genislik != meta.genislik {
                        ui.label(format!("ham {}×{}", meta.ham_genislik, meta.ham_yukseklik));
                    }
                }
                None => {
                    ui.label(egui::RichText::new("Görsel yükleniyor…").weak());
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.1} ms", durum.kare_suresi_ms))
                        .monospace()
                        .weak(),
                )
                .on_hover_text("Son karenin çizim süresi");
                ui.label(egui::RichText::new(durum.konum_metni()).monospace().weak());
                ui.label(egui::RichText::new(&durum.adaptor_bilgisi).weak())
                    .on_hover_text("Grafik aygıtı");
            });
        });
    });
}

/// Menü maddesi; tıklandıysa `true` döner ve menüyü kapatır.
fn menu_maddesi(ui: &mut egui::Ui, etiket: &str, ipucu: &str) -> bool {
    let tiklandi = ui
        .add(
            egui::Button::new(etiket)
                .frame(false)
                .min_size(egui::vec2(180.0, 0.0)),
        )
        .on_hover_text(ipucu)
        .clicked();
    if tiklandi {
        ui.close();
    }
    tiklandi
}

/// Simgesiz, kenarlıksız düğme; en az 44×44 dokunmatik hedef taşır.
fn duz_dugme(ui: &mut egui::Ui, etiket: &str, ipucu: &str) -> bool {
    ui.add(
        egui::Button::new(etiket)
            .frame(false)
            .min_size(egui::vec2(super::DOKUNMATIK_HEDEF, super::DOKUNMATIK_HEDEF)),
    )
    .on_hover_text(ipucu)
    .clicked()
}

/// Renkli, küçük bilgi rozeti.
fn rozet(ui: &mut egui::Ui, metin: &str, renk: egui::Color32) {
    ui.label(egui::RichText::new(metin).small().strong().color(renk));
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
}
