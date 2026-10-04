//! Ayarlar penceresi: görünüm, ön yükleme, renk ve kabuk entegrasyonu seçenekleri.

use crate::cekirdek::ayar::{
    Filtre, OnYuklemeGenisligi, PanelYeri, SagTikDavranisi, Tema, VarsayilanZoom,
};
use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::HudSonucu;

/// Ayarlar penceresini çizer.
pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let mut acik = true;
    let ctx = ui.ctx().clone();
    egui::Window::new("Ayarlar")
        .open(&mut acik)
        .resizable(true)
        .default_width(420.0)
        .default_height(560.0)
        .show(&ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                girdi_bolumu(ui, durum, sonuc);
                on_yukleme_bolumu(ui, durum, sonuc);
                gorunum_bolumu(ui, durum, sonuc);
                arayuz_bolumu(ui, durum, sonuc);
                pencere_bolumu(ui, durum, sonuc);
                kabuk_bolumu(ui, durum, sonuc);
            });
        });

    if !acik {
        durum.ayar_penceresi = false;
    }
}

/// Fare ve klavye davranış ayarları.
fn girdi_bolumu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    ui.heading("Girdi");
    let ayar = &mut durum.ayarlar;

    ui.horizontal(|ui| {
        ui.label("Sağ tık:");
        for (deger, etiket) in [
            (SagTikDavranisi::Gezinme, "Önceki görsel"),
            (SagTikDavranisi::Menu, "Bağlam menüsü"),
        ] {
            if ui.radio_value(&mut ayar.sag_tik, deger, etiket).changed() {
                sonuc.ayar_degisti = true;
            }
        }
    });

    if ui
        .add(
            egui::Slider::new(&mut ayar.tekerlek_zoom_carpani, 1.01..=1.6)
                .text("Tekerlek yakınlaştırma adımı")
                .fixed_decimals(2),
        )
        .changed()
    {
        sonuc.ayar_degisti = true;
    }

    if ui
        .checkbox(&mut ayar.cift_tik_tam_ekran, "Çift tıklama tam ekran")
        .changed()
    {
        sonuc.ayar_degisti = true;
    }
    ui.separator();
}

/// Ön yükleme ve önbellek ayarları.
fn on_yukleme_bolumu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    ui.heading("Ön yükleme ve önbellek");
    let ayar = &mut durum.ayarlar;

    ui.horizontal_wrapped(|ui| {
        ui.label("Kayan pencere:");
        for (deger, etiket) in [
            (OnYuklemeGenisligi::Kapali, "Kapalı"),
            (OnYuklemeGenisligi::Dar, "Dar (±1)"),
            (OnYuklemeGenisligi::Normal, "Normal (±2)"),
            (OnYuklemeGenisligi::Genis, "Geniş (±3)"),
        ] {
            if ui
                .radio_value(&mut ayar.on_yukleme, deger, etiket)
                .changed()
            {
                sonuc.ayar_degisti = true;
            }
        }
    });

    if ui
        .add(
            egui::Slider::new(&mut ayar.ram_onbellek_mb, 64..=4096)
                .text("RAM önbelleği (MB)")
                .logarithmic(true),
        )
        .changed()
    {
        sonuc.ayar_degisti = true;
    }

    if ui
        .checkbox(&mut ayar.dizin_izle, "Dizin değişikliklerini izle")
        .on_hover_text("Dizine dosya eklendiğinde/silindiğinde listeyi kendiliğinden günceller")
        .changed()
    {
        sonuc.ayar_degisti = true;
    }
    ui.separator();
}

/// Görüntüleme ve renk ayarları.
fn gorunum_bolumu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    ui.heading("Görünüm ve renk");
    let ayar = &mut durum.ayarlar;

    ui.horizontal_wrapped(|ui| {
        ui.label("Ölçekleme filtresi:");
        for (deger, etiket) in [
            (Filtre::CatmullRom, "Catmull-Rom"),
            (Filtre::Bilinear, "Bilinear"),
            (Filtre::Lanczos3, "Lanczos3"),
        ] {
            if ui
                .radio_value(&mut ayar.olcek_filtresi, deger, etiket)
                .on_hover_text("Ön küçültmede kullanılan CPU (SIMD) filtresi")
                .changed()
            {
                sonuc.ayar_degisti = true;
            }
        }
    });

    ui.horizontal_wrapped(|ui| {
        ui.label("Açılışta:");
        for (deger, etiket) in [
            (VarsayilanZoom::Sigdir, "Sığdır"),
            (VarsayilanZoom::GercekBoyut, "Gerçek boyut"),
            (VarsayilanZoom::PencereGenisligi, "Pencere genişliği"),
        ] {
            if ui
                .radio_value(&mut ayar.varsayilan_zoom, deger, etiket)
                .changed()
            {
                sonuc.ayar_degisti = true;
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("Arka plan:");
        if ui.color_edit_button_srgb(&mut ayar.arka_plan).changed() {
            sonuc.ayar_degisti = true;
        }
    });

    if ui
        .checkbox(&mut ayar.hdr_etkin, "HDR çıkışı (destekleyen ekranlarda)")
        .changed()
    {
        sonuc.ayar_degisti = true;
        // Yüzey yapılandırması bir sonraki pencere oluşturmada uygulanır.
        durum.bilgi_ver("HDR ayarı pencere yeniden açıldığında etkin olur.");
    }
    ui.separator();
}

/// Tema ve panel görünürlüğü ayarları; değişiklikler anında uygulanır ve kalıcıdır.
fn arayuz_bolumu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    ui.heading("Arayüz");
    let ayar = &mut durum.ayarlar;

    ui.horizontal_wrapped(|ui| {
        ui.label("Tema:");
        for (deger, etiket) in [
            (Tema::Koyu, "Koyu"),
            (Tema::Acik, "Açık"),
            (Tema::Sistem, "Sistem"),
        ] {
            if ui
                .radio_value(&mut ayar.tema, deger, etiket)
                .on_hover_text("Sistem seçeneği Windows'un koyu/açık tercihini izler")
                .changed()
            {
                sonuc.ayar_degisti = true;
            }
        }
    });

    if ui
        .checkbox(&mut ayar.dosya_listesi_acik, "Dizin listesi paneli")
        .on_hover_text("Açık dizindeki görselleri yan panelde listeler (L)")
        .changed()
    {
        sonuc.ayar_degisti = true;
    }
    if ui
        .checkbox(&mut ayar.onizlemeler, "Dizin listesinde küçük resimler")
        .on_hover_text(
            "Liste satırlarında dosyanın küçük resmi gösterilir; kapatmak \
             ağ paylaşımlarında taramayı hızlandırır",
        )
        .changed()
    {
        sonuc.ayar_degisti = true;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Dizin listesi yeri:");
        for (deger, etiket) in [(PanelYeri::Sol, "Sol"), (PanelYeri::Sag, "Sağ")] {
            if ui
                .radio_value(&mut ayar.dosya_listesi_yeri, deger, etiket)
                .changed()
            {
                sonuc.ayar_degisti = true;
            }
        }
    });

    if ui
        .checkbox(&mut ayar.bilgi_paneli_acik, "Bilgi paneli")
        .on_hover_text("Meta veri, önbellek, ekran ve kısayol bilgileri (I)")
        .changed()
    {
        sonuc.ayar_degisti = true;
    }
    ui.separator();
}

/// Pencere ve sıralama ayarları.
fn pencere_bolumu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    ui.heading("Pencere");
    // Bilgi mesajları, `durum.ayarlar` ödüncü bittikten sonra gösterilir.
    let mut mesaj: Option<&'static str> = None;

    {
        let ayar = &mut durum.ayarlar;

        if ui
            .checkbox(&mut ayar.mica_etkin, "Windows Mica malzemesi")
            .on_hover_text("Windows 11 sistem arka plan efekti")
            .changed()
        {
            sonuc.ayar_degisti = true;
            mesaj = Some("Mica ayarı pencere yeniden açıldığında etkin olur.");
        }

        if ui
            .checkbox(
                &mut ayar.dogal_siralama,
                "Windows Gezgini ile aynı doğal sıralama",
            )
            .on_hover_text("resim2.jpg < resim10.jpg sırası (StrCmpLogicalW)")
            .changed()
        {
            sonuc.ayar_degisti = true;
            mesaj = Some("Sıralama değişikliği dizin yeniden tarandığında uygulanır.");
        }

        if ui
            .button("Görünümü yeniden yükle")
            .on_hover_text("Sıralama/filtre değişikliklerini hemen uygular")
            .clicked()
        {
            sonuc.ekle(Eylem::YenidenYukle);
        }
    }

    if let Some(metin) = mesaj {
        durum.bilgi_ver(metin);
    }
    ui.separator();
}

/// Kabuk entegrasyonu: dosya ilişkilendirmesi.
fn kabuk_bolumu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    ui.heading("Kabuk entegrasyonu");
    ui.label(
        egui::RichText::new(if durum.iliskilendirme_durumu.is_empty() {
            "Durum bilinmiyor"
        } else {
            &durum.iliskilendirme_durumu
        })
        .small()
        .weak(),
    );
    ui.add_space(4.0);

    ui.horizontal_wrapped(|ui| {
        if ui
            .button("İlişkilendirmeyi kaydet")
            .on_hover_text(
                "Desteklenen uzantıları bu uygulamaya kaydeder (yalnızca kullanıcı hesabı)",
            )
            .clicked()
        {
            sonuc.ekle(Eylem::IliskilendirmeKaydet);
        }
        if ui
            .button("Kayıtları kaldır")
            .on_hover_text("Bu uygulamanın yazdığı kayıtları siler")
            .clicked()
        {
            sonuc.ekle(Eylem::IliskilendirmeKaldir);
        }
        if ui
            .button("Varsayılan uygulamalar…")
            .on_hover_text("Windows Ayarlar → Varsayılan uygulamalar sayfasını açar")
            .clicked()
        {
            sonuc.ekle(Eylem::VarsayilanUygulamaSayfasi);
        }
    });

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Windows, varsayılan uygulamayı kullanıcı onayı olmadan değiştirmeye izin vermez; \
             kayıttan sonra uygulamayı bu sayfadan seçebilirsiniz.",
        )
        .small()
        .weak(),
    );
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::cekirdek::ayar::Ayarlar;

    #[test]
    fn ayar_penceresi_bayragi_bagimsiz() {
        let mut durum = UygulamaDurumu::yeni(Ayarlar::default());
        assert!(!durum.ayar_penceresi);
        durum.ayar_penceresi = true;
        assert!(durum.ayar_penceresi);
    }

    #[test]
    fn varsayilan_ayarlar_tutarli() {
        let ayar = Ayarlar::default();
        assert!(ayar.mica_etkin);
        assert!(ayar.dogal_siralama);
        assert!(ayar.hdr_etkin);
        assert!((ayar.tekerlek_zoom_carpani - 1.15).abs() < 1e-6);
    }
}
