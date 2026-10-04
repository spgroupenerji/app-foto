//! Dizin listesi paneli: açık dizindeki görseller yan panelde listelenir ve
//! tıklanan satıra geçilir.
//!
//! Liste kaynağı [`UygulamaDurumu::indeks`]'tir; bu liste dizin tarayıcısı
//! (`dizin::tarayici::tara`) tarafından yalnızca desteklenen biçimlerden doldurulur.
//! Böylece panelde, oklarla gezinmede ve ön yüklemede desteklenmeyen bir dosya
//! asla görünmez. Panelin görünürlüğü ve kenarı `Ayarlar` üzerinden kalıcıdır.

use crate::cekirdek::ayar::PanelYeri;
use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::HudSonucu;

/// Uzun dosya adlarının listede kısaltılma sınırı; tam ad fare ipucunda gösterilir.
const AD_UZUNLUGU: usize = 36;

/// Seçili satırın kaydırma konumu egui geçici belleğinde tutulur.
const KONUM_BELLEGI: &str = "gorsel-dosya-listesi-konum";

/// Dizin listesi panelini çizer; görünürlük ve yer `Ayarlar`'dan gelir.
pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    if !durum.ayarlar.dosya_listesi_acik {
        return;
    }

    let mut acik = true;
    let kimlik = match durum.ayarlar.dosya_listesi_yeri {
        PanelYeri::Sol => "gorsel-dosya-listesi-sol",
        PanelYeri::Sag => "gorsel-dosya-listesi-sag",
    };
    let panel = if durum.ayarlar.dosya_listesi_yeri == PanelYeri::Sag {
        egui::containers::Panel::right(kimlik)
    } else {
        egui::containers::Panel::left(kimlik)
    };

    panel.resizable(true).default_size(220.0).show(ui, |ui| {
        ui.set_min_width(160.0);
        ui.set_max_width(340.0);
        ui.horizontal(|ui| {
            ui.heading("Dizin");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new("✕")
                            .small()
                            .min_size(egui::vec2(super::DOKUNMATIK_HEDEF, 0.0)),
                    )
                    .on_hover_text("Listeyi kapat (L)")
                    .clicked()
                {
                    acik = false;
                }
            });
        });
        ui.label(
            egui::RichText::new(durum.konum_metni())
                .weak()
                .small()
                .monospace(),
        );
        ui.separator();

        // Seçim değiştiğinde seçili satır merkeze kaydırılır; kullanıcı kendi
        // kaydırışında zorlanmaz (önceki konum kareler arası egui belleğinde).
        let onceki_konum: usize = ui.ctx().data(|d| {
            d.get_temp(egui::Id::new(KONUM_BELLEGI))
                .unwrap_or(usize::MAX)
        });

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                if durum.indeks.is_empty() {
                    ui.label(egui::RichText::new("Bu dizinde desteklenen görsel yok").weak());
                    return;
                }
                for (sira, yol) in durum.indeks.iter().enumerate() {
                    let ad = dosya_adi(yol);
                    let etiket =
                        egui::RichText::new(super::arac_cubugu::kisalt(&ad, AD_UZUNLUGU)).small();
                    let yanit = ui
                        .add_sized(
                            egui::vec2(ui.available_width(), super::DOKUNMATIK_HEDEF),
                            egui::Button::new(etiket).selected(sira == durum.konum),
                        )
                        .on_hover_text(&ad);
                    if yanit.clicked() {
                        sonuc.ekle(Eylem::Git(sira));
                    }
                    if sira == durum.konum && onceki_konum != durum.konum {
                        yanit.scroll_to_me(Some(egui::Align::Center));
                    }
                }
            });

        if onceki_konum != durum.konum {
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new(KONUM_BELLEGI), durum.konum));
        }
    });

    if !acik {
        durum.ayarlar.dosya_listesi_acik = false;
        sonuc.ayar_degisti = true;
    }
}

/// Yolun yalnızca dosya adı; ad çözülemezse tam yol gösterilir.
fn dosya_adi(yol: &std::path::Path) -> String {
    yol.file_name()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_else(|| yol.display().to_string())
}
