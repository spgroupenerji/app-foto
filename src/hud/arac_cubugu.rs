//! Üst araç çubuğu ve alt durum çubuğu.
//!
//! Üst çubuk yalnızca en sık kullanılan işlemleri taşır; kalan her şey çubuğun sonundaki
//! "Daha fazla" menüsünde hızlı erişim listesi olarak toplanır. Simge düğmeleri font
//! glifi yerine vektörel çizilir (`ikon`), böylece bozuk glif sorunu yaşanmaz.

use crate::cekirdek::ayar::{PanelYeri, Tema};
use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;
use crate::goruntu::YuklemeAsamasi;

use super::HudSonucu;
use super::ikon::{self, Ikon};

/// Yükleme ilerleme çubuğunun genişliği (puan).
const ILERLEME_GENISLIGI: f32 = 120.0;

/// Araç çubuğu ve durum çubuğunu çizer.
pub fn ciz(ui: &mut egui::Ui, durum: &UygulamaDurumu, sonuc: &mut HudSonucu) {
    ust_cubuk(ui, durum, sonuc);
    alt_cubuk(ui, durum);
}

/// Üst çubuk: menü, en sık işlemler ve sağda "Daha fazla" hızlı erişim menüsü.
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
                    gorunum_maddeleri(ui, durum, sonuc);
                });
            });
            ui.separator();

            // En sık kullanılan işlemler doğrudan çubukta durur.
            if metin_dugme(ui, "Aç", "Görsel aç (Ctrl+O)") {
                sonuc.ekle(Eylem::DosyaAc);
            }
            if metin_dugme(ui, "Klasör", "Klasör aç (Ctrl+Shift+O)") {
                sonuc.ekle(Eylem::KlasorAc);
            }
            ui.separator();

            if ikon::ikon_dugme(ui, Ikon::Sol, "Önceki görsel (Sol ok / PageUp)") {
                sonuc.ekle(Eylem::Onceki);
            }
            if ikon::ikon_dugme(ui, Ikon::Sag, "Sonraki görsel (Sağ ok / PageDown)") {
                sonuc.ekle(Eylem::Sonraki);
            }
            ui.separator();

            // Konum ve dosya adı: uzun adlar ortadan kısaltılır, tam ad ipucunda gösterilir.
            ui.label(
                egui::RichText::new(durum.konum_metni())
                    .strong()
                    .monospace(),
            );
            let ad = durum.dosya_adi();
            ui.label(egui::RichText::new(kisalt(&ad, 48)).strong())
                .on_hover_text(&ad);
            ui.separator();

            if ikon::ikon_dugme(ui, Ikon::Eksi, "Uzaklaştır (Ctrl + -)") {
                sonuc.ekle(Eylem::Yakinlastir {
                    carpan: 1.0 / crate::girdi::klavye::TUS_ZOOM_CARPI,
                    pivot: None,
                });
            }
            ui.label(egui::RichText::new(durum.zoom_metni()).monospace().strong());
            if ikon::ikon_dugme(ui, Ikon::Arti, "Yakınlaştır (Ctrl + +)") {
                sonuc.ekle(Eylem::Yakinlastir {
                    carpan: crate::girdi::klavye::TUS_ZOOM_CARPI,
                    pivot: None,
                });
            }
            ui.separator();

            // Sağa yaslanan bölüm: yükleme göstergesi, rozetler ve "Daha fazla" menüsü.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                daha_fazla_menusu(ui, durum, sonuc);
                rozetleri_ciz(ui, durum);
                yukleme_gostergesi(ui, durum);
            });
        });
    });
}

/// Görünüm menüsünün (ve "Daha fazla" menüsünün) ortak maddeleri.
fn gorunum_maddeleri(ui: &mut egui::Ui, durum: &UygulamaDurumu, sonuc: &mut HudSonucu) {
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
        if durum.ayarlar.dosya_listesi_acik {
            "Dizin listesini gizle"
        } else {
            "Dizin listesini göster"
        },
        "L",
    ) {
        sonuc.ekle(Eylem::DosyaListesiDegistir);
    }
    if menu_maddesi(
        ui,
        if durum.ayarlar.dosya_listesi_yeri == PanelYeri::Sol {
            "Dizin listesini sağa taşı"
        } else {
            "Dizin listesini sola taşı"
        },
        "Liste kenarı",
    ) {
        sonuc.ekle(Eylem::DosyaListesiYeriDegistir);
    }
    if menu_maddesi(
        ui,
        if durum.ayarlar.bilgi_paneli_acik {
            "Bilgi panelini gizle"
        } else {
            "Bilgi panelini göster"
        },
        "I",
    ) {
        sonuc.ekle(Eylem::MetaPaneliDegistir);
    }
    ui.separator();
    ui.label(egui::RichText::new("Tema").weak());
    for (tema, etiket) in [
        (Tema::Koyu, "Koyu"),
        (Tema::Acik, "Açık"),
        (Tema::Sistem, "Sistem"),
    ] {
        let secili = durum.ayarlar.tema == tema;
        let satir = if secili {
            format!("{etiket} ✓")
        } else {
            etiket.to_string()
        };
        if menu_maddesi(ui, &satir, "Görünüm teması") {
            sonuc.ekle(Eylem::TemaSec(tema));
        }
    }
    ui.separator();
    if menu_maddesi(ui, "Ayarlar…", "Ctrl+K") {
        sonuc.ekle(Eylem::AyarPenceresi);
    }
}

/// Çubuğun sonundaki hızlı erişim menüsü: seyrek işlemler tek menüde toplanır.
fn daha_fazla_menusu(ui: &mut egui::Ui, durum: &UygulamaDurumu, sonuc: &mut HudSonucu) {
    let yanit = ikon::ikon_dugme_yanit(
        ui,
        Ikon::UcNokta,
        false,
        "Daha fazla: tüm görünüm ve dosya işlemleri",
    );
    egui::Popup::menu(&yanit).show(|ui| {
        gorunum_maddeleri(ui, durum, sonuc);
    });
}

/// HDR/ICC rozetlerini çizer (sağdan sola akışta menünün solunda durur).
fn rozetleri_ciz(ui: &mut egui::Ui, durum: &UygulamaDurumu) {
    if durum.hdr_yuzey {
        rozet(ui, "HDR", egui::Color32::from_rgb(120, 200, 140));
    }
    if durum.hdr_kaynak {
        rozet(ui, "HDR kaynak", egui::Color32::from_rgb(150, 180, 240));
    }
    if durum.icc_var() {
        rozet(ui, "ICC", egui::Color32::from_rgb(220, 190, 120));
    }
}

/// Yükleme göstergesi: okuma aşamasında % çubuğu, çözme aşamasında süreli spinner.
fn yukleme_gostergesi(ui: &mut egui::Ui, durum: &UygulamaDurumu) {
    let Some(gosterge) = &durum.yukleme else {
        return;
    };
    match gosterge.asama {
        YuklemeAsamasi::Okuma => {
            let cubuk = egui::ProgressBar::new(gosterge.oran.clamp(0.0, 1.0))
                .desired_width(ILERLEME_GENISLIGI)
                .show_percentage();
            ui.add(cubuk).on_hover_text("Görsel dosyası okunuyor");
        }
        YuklemeAsamasi::Cozme => {
            ui.spinner().on_hover_text("Görsel çözümleniyor");
            ui.label(
                egui::RichText::new(format!("{:.1} sn", gosterge.gecen_sn))
                    .monospace()
                    .weak(),
            )
            .on_hover_text("Kod çözme sürüyor; büyük dosyalar ağda daha uzun sürebilir");
        }
    }
}

/// Alt çubuk: tek satırda dosya özeti ile performans göstergesi.
fn alt_cubuk(ui: &mut egui::Ui, durum: &UygulamaDurumu) {
    egui::containers::Panel::bottom("gorsel-durum-cubugu").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            match &durum.aktif {
                Some(meta) => {
                    // Tek satır özet: ayrıntılar bilgi panelinde kalır.
                    let ozet = format!(
                        "{} · {} · {}",
                        crate::goruntu::bicim::bicim_adi(meta.bicim),
                        meta.cozunurluk_metni(),
                        meta.boyut_metni()
                    );
                    ui.label(egui::RichText::new(ozet).strong());
                    if meta.yonelim > 1 {
                        ui.label(format!(
                            "⟲ {}",
                            crate::goruntu::meta::yonelim_adi(meta.yonelim)
                        ));
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
fn metin_dugme(ui: &mut egui::Ui, etiket: &str, ipucu: &str) -> bool {
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
