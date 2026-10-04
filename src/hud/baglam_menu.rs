//! Sağ tık bağlam menüsü.

use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::HudSonucu;

/// Pencere dışına taşmayı önlemek için menü ölçüsü sınırları.
/// 12 madde × 44 px dokunmatik hedef + ayraçlar sığmalı.
const MENU_EN_COK_YUKSEKLIK: f32 = 600.0;

/// Bağlam menüsünü çizer; seçim yapıldığında veya dışarı tıklanınca kapanır.
pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let Some((x, y)) = durum.baglam_menusu else {
        return;
    };
    let ctx = ui.ctx().clone();
    let konum = egui::pos2(x, y);
    let mut kapat = false;

    let ic = egui::Area::new(egui::Id::new("gorsel-baglam-menusu"))
        .order(egui::Order::Foreground)
        .fixed_pos(konum)
        .constrain(true)
        .show(&ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(220.0);
                ui.set_max_height(MENU_EN_COK_YUKSEKLIK);
                ui.vertical(|ui| {
                    if madde(ui, "◀  Önceki görsel", "Sol ok") {
                        sonuc.ekle(Eylem::Onceki);
                        kapat = true;
                    }
                    if madde(ui, "▶  Sonraki görsel", "Sağ ok") {
                        sonuc.ekle(Eylem::Sonraki);
                        kapat = true;
                    }
                    ui.separator();
                    if madde(ui, "Sığdır", "S") {
                        sonuc.ekle(Eylem::Sigdir);
                        kapat = true;
                    }
                    if madde(ui, "%100 (gerçek boyut)", "G") {
                        sonuc.ekle(Eylem::GercekBoyut);
                        kapat = true;
                    }
                    if madde(
                        ui,
                        if durum.tam_ekran {
                            "Pencere kipine dön"
                        } else {
                            "Tam ekran"
                        },
                        "F11",
                    ) {
                        sonuc.ekle(Eylem::TamEkranDegistir);
                        kapat = true;
                    }
                    ui.separator();
                    if madde(ui, "Birlikte aç…", "Ctrl + O") {
                        sonuc.ekle(Eylem::BirlikteAc);
                        kapat = true;
                    }
                    if madde(ui, "Yeniden yükle", "R") {
                        sonuc.ekle(Eylem::YenidenYukle);
                        kapat = true;
                    }
                    ui.separator();
                    if madde(
                        ui,
                        if durum.ayarlar.dosya_listesi_acik {
                            "Dizin listesini gizle"
                        } else {
                            "Dizin listesini göster"
                        },
                        "L",
                    ) {
                        sonuc.ekle(Eylem::DosyaListesiDegistir);
                        kapat = true;
                    }
                    if madde(
                        ui,
                        if durum.ayarlar.dosya_listesi_yeri == crate::cekirdek::ayar::PanelYeri::Sol
                        {
                            "Dizin listesini sağa taşı"
                        } else {
                            "Dizin listesini sola taşı"
                        },
                        "Liste kenarı",
                    ) {
                        sonuc.ekle(Eylem::DosyaListesiYeriDegistir);
                        kapat = true;
                    }
                    if madde(
                        ui,
                        if durum.ayarlar.bilgi_paneli_acik {
                            "Bilgi panelini gizle"
                        } else {
                            "Bilgi panelini göster"
                        },
                        "I",
                    ) {
                        sonuc.ekle(Eylem::MetaPaneliDegistir);
                        kapat = true;
                    }
                    if madde(ui, "Ayarlar…", "Ctrl + K") {
                        sonuc.ekle(Eylem::AyarPenceresi);
                        kapat = true;
                    }
                });
            });
        });

    // Menü dışına tıklama menüyü kapatır.
    let menu_dikdortgeni = ic.response.rect;
    let basildi = ctx.input(|i| i.pointer.any_pressed());
    let imlec = ctx.input(|i| i.pointer.interact_pos());
    if basildi && !imlec.is_some_and(|p| menu_dikdortgeni.contains(p)) {
        kapat = true;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        kapat = true;
    }

    if kapat {
        sonuc.ekle(Eylem::BaglamMenusuKapat);
    }
}

/// Menü maddesi; tıklandıysa `true`.
fn madde(ui: &mut egui::Ui, etiket: &str, ipucu: &str) -> bool {
    ui.add(
        egui::Button::new(etiket)
            .frame(false)
            .min_size(egui::vec2(200.0, 0.0)),
    )
    .on_hover_text(ipucu)
    .clicked()
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
