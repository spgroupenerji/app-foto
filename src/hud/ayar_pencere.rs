//! Ayarlar: ortalanmış modal, sol bölüm navigasyonu ve shadcn tarzı kontroller.

use egui::{Align, CornerRadius, Margin, RichText, Stroke, vec2};

use crate::cekirdek::ayar::{
    Ayarlar, Filtre, OnYuklemeGenisligi, PanelYeri, SagTikDavranisi, Tema, VarsayilanZoom,
    Yogunluk,
};
use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::bilesen::{self, DugmeTuru};
use super::ikon::{self, Ikon};
use super::tasarim::{self, Palet};
use super::HudSonucu;

const SEKMELER: [&str; 5] = ["Genel", "Görünüm", "Performans", "Kısayollar", "Hakkında"];
const SEKME_BELLEGI: &str = "gorsel-ayar-sekmesi";

pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let ctx = ui.ctx().clone();
    let p = Palet::ui(ui);
    let mut kapat = false;
    let mut mesaj: Option<String> = None;

    let modal = egui::Modal::new(egui::Id::new("gorsel-ayarlar"))
        .backdrop_color(egui::Color32::from_black_alpha(120))
        .frame(
            egui::Frame::NONE
                .fill(p.panel_opak)
                .stroke(Stroke::new(1.0, p.kenar_guc))
                .corner_radius(CornerRadius::same(tasarim::YARICAP_XL))
                .inner_margin(Margin::same(0)),
        )
        .show(&ctx, |ui| {
            ui.set_width(720.0);
            ui.set_height(520.0);
            ui.horizontal(|ui| {
                ui.add_space(tasarim::BOSLUK_LG);
                ui.vertical(|ui| {
                    ui.add_space(tasarim::BOSLUK_MD);
                    ui.label(
                        RichText::new("TERCİHLER")
                            .text_style(egui::TextStyle::Name("bolum".into()))
                            .color(p.cok_soluk),
                    );
                    ui.label(
                        RichText::new("Ayarlar")
                            .text_style(egui::TextStyle::Name("buyuk".into())),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let kapat_yanit =
                        ikon::ikon_dugme_boyut(ui, Ikon::Carpi, false, "Ayarları kapat", 32.0);
                    if bilesen::tiklandi(ui, &kapat_yanit) {
                        kapat = true;
                    }
                });
            });
            ui.add_space(tasarim::BOSLUK_SM);
            ui.separator();

            let mut sekme = ui
                .data(|d| d.get_temp::<usize>(egui::Id::new(SEKME_BELLEGI)))
                .unwrap_or(0)
                .min(SEKMELER.len() - 1);
            let iliskilendirme = durum.iliskilendirme_durumu.clone();
            let adaptor = durum.adaptor_bilgisi.clone();
            ui.horizontal(|ui| {
                ui.add_space(tasarim::BOSLUK_SM);
                ui.vertical(|ui| {
                    ui.set_width(160.0);
                    ui.add_space(tasarim::BOSLUK_SM);
                    for (sira, ad) in SEKMELER.iter().enumerate() {
                        let secili = sira == sekme;
                        let yanit = ui.add(
                            egui::Button::new(
                                RichText::new(*ad).size(12.0).color(if secili {
                                    p.vurgu
                                } else {
                                    p.soluk
                                }),
                            )
                            .frame(false)
                            .fill(if secili { p.vurgu_yumusak } else { egui::Color32::TRANSPARENT })
                            .corner_radius(CornerRadius::same(tasarim::YARICAP_LG))
                            .min_size(vec2(152.0, 34.0)),
                        );
                        if yanit.clicked() {
                            sekme = sira;
                            ui.data_mut(|d| {
                                d.insert_temp(egui::Id::new(SEKME_BELLEGI), sira)
                            });
                        }
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(tasarim::BOSLUK_MD);
                        ui.set_width(500.0);
                        match sekme {
                            0 => genel(
                                ui,
                                &mut durum.ayarlar,
                                &iliskilendirme,
                                sonuc,
                                &mut mesaj,
                            ),
                            1 => gorunum(ui, &mut durum.ayarlar, sonuc, &mut mesaj),
                            2 => performans(ui, &mut durum.ayarlar, sonuc),
                            3 => kisayollar(ui),
                            _ => hakkinda(ui, &adaptor),
                        }
                        ui.add_space(tasarim::BOSLUK_MD);
                    });
            });

            ui.separator();
            ui.horizontal(|ui| {
                ui.add_space(tasarim::BOSLUK_MD);
                let sifirla = bilesen::dugme(
                    ui,
                    "Varsayılanlara dön",
                    DugmeTuru::Hayalet,
                    None,
                    None,
                );
                if bilesen::tiklandi(ui, &sifirla) {
                    durum.ayarlar = Ayarlar::default();
                    sonuc.ayar_degisti = true;
                    mesaj = Some("Ayarlar varsayılan değerlere döndürüldü.".to_string());
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let tamam = bilesen::dugme(ui, "Tamam", DugmeTuru::Birincil, None, None);
                    if bilesen::tiklandi(ui, &tamam) {
                        kapat = true;
                    }
                    let vazgec = bilesen::dugme(ui, "Vazgeç", DugmeTuru::Ikincil, None, None);
                    if bilesen::tiklandi(ui, &vazgec) {
                        kapat = true;
                    }
                });
                ui.add_space(tasarim::BOSLUK_MD);
            });
            ui.add_space(tasarim::BOSLUK_SM);
        });

    if kapat || modal.should_close() {
        durum.ayar_penceresi = false;
    }
    if let Some(mesaj) = mesaj {
        durum.bilgi_ver(mesaj);
    }
}

fn genel(
    ui: &mut egui::Ui,
    ayar: &mut Ayarlar,
    iliskilendirme: &str,
    sonuc: &mut HudSonucu,
    mesaj: &mut Option<String>,
) {
    bolum(ui, "GÖRÜNÜM");
    ayar_satiri(ui, "Tema", "Koyu, açık veya Windows sistem tercihi.", |ui| {
        if bilesen::segment(
            ui,
            &[(Tema::Koyu, "Koyu"), (Tema::Acik, "Açık"), (Tema::Sistem, "Sistem")],
            &mut ayar.tema,
        ) {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Dizin listesi", "Küçük resimli dizin paneli (L).", |ui| {
        if bilesen::anahtar(ui, &mut ayar.dosya_listesi_acik, "Dizin listesi") {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Bilgi paneli", "Meta veri, renk ve sistem bilgileri (I).", |ui| {
        if bilesen::anahtar(ui, &mut ayar.bilgi_paneli_acik, "Bilgi paneli") {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Küçük resimler", "Yalnızca görünen satırlar için üretilir.", |ui| {
        if bilesen::anahtar(ui, &mut ayar.onizlemeler, "Küçük resimler") {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Satır yoğunluğu", "Rahat veya kompakt dizin listesi.", |ui| {
        if bilesen::segment(
            ui,
            &[(Yogunluk::Rahat, "Rahat"), (Yogunluk::Kompakt, "Kompakt")],
            &mut ayar.liste_yogunlugu,
        ) {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Hareketi azalt", "Panel ve bildirim geçişlerini anında yapar.", |ui| {
        if bilesen::anahtar(ui, &mut ayar.hareket_azalt, "Hareketi azalt") {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Mica malzemesi", "Windows 11 sistem arka planı; yeniden açılışta etkin olur.", |ui| {
        if bilesen::anahtar(ui, &mut ayar.mica_etkin, "Mica malzemesi") {
            sonuc.ayar_degisti = true;
            *mesaj = Some("Mica ayarı pencere yeniden açıldığında etkin olur.".to_string());
        }
    });
    bolum(ui, "KABUK");
    ayar_satiri(ui, "Dosya ilişkilendirmesi", iliskilendirme, |ui| {
        ui.horizontal(|ui| {
            let kaydet = bilesen::dugme(ui, "Kaydet", DugmeTuru::Ikincil, None, None);
            if bilesen::tiklandi(ui, &kaydet) {
                sonuc.ekle(Eylem::IliskilendirmeKaydet);
            }
            let kaldir = bilesen::dugme(ui, "Kaldır", DugmeTuru::Tehlike, None, None);
            if bilesen::tiklandi(ui, &kaldir) {
                sonuc.ekle(Eylem::IliskilendirmeKaldir);
            }
            let sayfa = bilesen::dugme(ui, "Varsayılanlar…", DugmeTuru::Hayalet, None, None);
            if bilesen::tiklandi(ui, &sayfa) {
                sonuc.ekle(Eylem::VarsayilanUygulamaSayfasi);
            }
        });
    });
}

fn gorunum(
    ui: &mut egui::Ui,
    ayar: &mut Ayarlar,
    sonuc: &mut HudSonucu,
    mesaj: &mut Option<String>,
) {
    bolum(ui, "GÖRÜNTÜ");
    ayar_satiri(ui, "Ölçekleme filtresi", "Ön küçültmede kullanılan CPU filtresi.", |ui| {
        if bilesen::segment(
            ui,
            &[
                (Filtre::CatmullRom, "Catmull"),
                (Filtre::Bilinear, "Bilinear"),
                (Filtre::Lanczos3, "Lanczos3"),
            ],
            &mut ayar.olcek_filtresi,
        ) {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Açılışta", "Yeni görselde varsayılan yakınlaştırma.", |ui| {
        if bilesen::segment(
            ui,
            &[
                (VarsayilanZoom::Sigdir, "Sığdır"),
                (VarsayilanZoom::GercekBoyut, "Gerçek"),
                (VarsayilanZoom::PencereGenisligi, "Genişlik"),
            ],
            &mut ayar.varsayilan_zoom,
        ) {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Arka plan", "Görsel alanının arkasındaki renk.", |ui| {
        if ui.color_edit_button_srgb(&mut ayar.arka_plan).changed() {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "HDR çıkışı", "Destekleyen ekranlarda scRGB; yeniden açılışta etkin olur.", |ui| {
        if bilesen::anahtar(ui, &mut ayar.hdr_etkin, "HDR çıkışı") {
            sonuc.ayar_degisti = true;
            *mesaj = Some("HDR ayarı pencere yeniden açıldığında etkin olur.".to_string());
        }
    });
    bolum(ui, "GİRDİ");
    ayar_satiri(ui, "Sağ tık", "Gezinme veya bağlam menüsü.", |ui| {
        if bilesen::segment(
            ui,
            &[(SagTikDavranisi::Gezinme, "Gezin"), (SagTikDavranisi::Menu, "Menü")],
            &mut ayar.sag_tik,
        ) {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Çift tıklama", "Pencereyi tam ekran kipine alır.", |ui| {
        if bilesen::anahtar(ui, &mut ayar.cift_tik_tam_ekran, "Çift tık tam ekran") {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Tekerlek adımı", "Ctrl + tekerlek yakınlaştırma çarpanı.", |ui| {
        if ui
            .add(
                egui::Slider::new(&mut ayar.tekerlek_zoom_carpani, 1.01..=1.6)
                    .fixed_decimals(2),
            )
            .changed()
        {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Dizin paneli kenarı", "Panelin sol veya sağ kenarı.", |ui| {
        if bilesen::segment(
            ui,
            &[(PanelYeri::Sol, "Sol"), (PanelYeri::Sag, "Sağ")],
            &mut ayar.dosya_listesi_yeri,
        ) {
            sonuc.ayar_degisti = true;
        }
    });
}

fn performans(ui: &mut egui::Ui, ayar: &mut Ayarlar, sonuc: &mut HudSonucu) {
    bolum(ui, "ÖNBELLEK");
    ayar_satiri(ui, "RAM önbelleği", "64 MB – 8 GB arası; büyük değer daha akıcı gezinme.", |ui| {
        if ui
            .add(
                egui::Slider::new(&mut ayar.ram_onbellek_mb, 64..=4096)
                    .logarithmic(true)
                    .suffix(" MB"),
            )
            .changed()
        {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Ön yükleme", "Aktif görselin çevresinde hazır tutulacak komşular.", |ui| {
        if bilesen::segment(
            ui,
            &[
                (OnYuklemeGenisligi::Kapali, "Kapalı"),
                (OnYuklemeGenisligi::Dar, "±1"),
                (OnYuklemeGenisligi::Normal, "±2"),
                (OnYuklemeGenisligi::Genis, "±3"),
            ],
            &mut ayar.on_yukleme,
        ) {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Dizin izleme", "Dosya eklendiğinde/silindiğinde listeyi tazeler.", |ui| {
        if bilesen::anahtar(ui, &mut ayar.dizin_izle, "Dizin izleme") {
            sonuc.ayar_degisti = true;
        }
    });
    ayar_satiri(ui, "Doğal sıralama", "Windows Gezgini ile aynı resim2 < resim10 sırası.", |ui| {
        if bilesen::anahtar(ui, &mut ayar.dogal_siralama, "Doğal sıralama") {
            sonuc.ayar_degisti = true;
        }
    });
    ui.add_space(tasarim::BOSLUK_MD);
    let yenile = bilesen::dugme(ui, "Görünümü yeniden yükle", DugmeTuru::Ikincil, Some(Ikon::Yenile), None);
    if bilesen::tiklandi(ui, &yenile) {
        sonuc.ekle(Eylem::YenidenYukle);
    }
}

fn kisayollar(ui: &mut egui::Ui) {
    bolum(ui, "KLAVYE VE FARE");
    for (tus, aciklama) in super::meta_panel::KISAYOLLAR {
        bilesen::bilgi_satiri(ui, tus, aciklama);
    }
}

fn hakkinda(ui: &mut egui::Ui, adaptor: &str) {
    let p = Palet::ui(ui);
    bolum(ui, "UYGULAMA");
    bilesen::bilgi_satiri(ui, "Sürüm", env!("DERLEME_SURUMU"));
    bilesen::bilgi_satiri(ui, "Motor", "wgpu + egui (scRGB HDR)");
    bilesen::bilgi_satiri(ui, "Grafik aygıtı", adaptor);
    ui.add_space(tasarim::BOSLUK_MD);
    bolum(ui, "VERİ");
    bilesen::bilgi_satiri(ui, "Ayarlar", r"%APPDATA%\Gorsel");
    bilesen::bilgi_satiri(ui, "Önbellek ve günlük", r"%LOCALAPPDATA%\Gorsel");
    ui.add_space(tasarim::BOSLUK_MD);
    ui.label(
        RichText::new(
            "Uygulama yalnızca yerel dosyaları işler; ağ isteği yapmaz, telemetri toplamaz. \
             Bağımlılık lisansları Cargo.lock üzerinden izlenir.",
        )
        .size(11.0)
        .color(p.soluk),
    );
}

fn bolum(ui: &mut egui::Ui, baslik: &str) {
    ui.add_space(tasarim::BOSLUK_MD);
    ui.label(
        RichText::new(baslik)
            .text_style(egui::TextStyle::Name("bolum".into()))
            .color(Palet::ui(ui).cok_soluk),
    );
    ui.add_space(tasarim::BOSLUK_XS);
}

fn ayar_satiri(
    ui: &mut egui::Ui,
    ad: &str,
    aciklama: &str,
    kontrol: impl FnOnce(&mut egui::Ui),
) {
    let p = Palet::ui(ui);
    let genislik = ui.available_width();
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            vec2((genislik - 230.0).max(180.0), 44.0),
            egui::Layout::top_down(Align::Min),
            |ui| {
                ui.label(RichText::new(ad).size(12.0).strong().color(p.metin));
                ui.add(
                    egui::Label::new(RichText::new(aciklama).size(11.0).color(p.soluk))
                        .truncate(),
                );
            },
        );
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            kontrol(ui);
        });
    });
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilan_ayarlar_tutarli() {
        let ayar = Ayarlar::default();
        assert!(ayar.mica_etkin);
        assert!(ayar.dogal_siralama);
        assert!(ayar.hdr_etkin);
        assert_eq!(ayar.liste_yogunlugu, Yogunluk::Rahat);
        assert!(!ayar.hareket_azalt);
        assert!((ayar.tekerlek_zoom_carpani - 1.15).abs() < 1e-6);
    }

    #[test]
    fn sekme_listesi_dolu() {
        assert_eq!(SEKMELER.len(), 5);
    }
}
