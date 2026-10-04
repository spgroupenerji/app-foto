//! Dizin listesi paneli: açık dizindeki görseller küçük resimlerle listelenir ve
//! tıklanan satıra geçilir.
//!
//! Liste kaynağı [`UygulamaDurumu::indeks`]'tir; bu liste dizin tarayıcısı
//! (`dizin::tarayici::tara`) tarafından yalnızca desteklenen biçimlerden doldurulur.
//! Böylece panelde, oklarla gezinmede ve ön yüklemede desteklenmeyen bir dosya
//! asla görünmez. Panelin görünürlüğü, kenarı ve genişliği `Ayarlar` üzerinden kalıcıdır.
//!
//! Küçük resimler yalnızca görünür satırlar için arka planda üretilir ve RAM'de
//! sınırlı sayıda tutulur; böylece ağ paylaşımlarında da liste akıcı kalır.

use std::path::Path;

use crate::cekirdek::ayar::{LISTE_GENISLIK_ARALIGI, SiralamaTuru, SiralamaYonu};
use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

use super::HudSonucu;
use super::ikon::{self, Ikon};

/// Uzun dosya adlarının listede kısaltılma sınırı; tam ad fare ipucunda gösterilir.
const AD_UZUNLUGU: usize = 36;

/// Satır yüksekliği: küçük resim + iki satır metin (dokunmatik hedefin üzerinde).
const SATIR_YUKSEKLIGI: f32 = 64.0;

/// Satır içi küçük resim kenarı.
const KUCUK_RESIM: f32 = 52.0;

/// Satırın sağ ucundaki "birlikte aç" eylem alanının genişliği.
const EYLEM_ALANI: f32 = 48.0;

/// Seçili satırın kaydırma konumu egui geçici belleğinde tutulur.
const KONUM_BELLEGI: &str = "gorsel-dosya-listesi-konum";

/// Genişlik değişiminin "sabitleşti" bilgisinin tutulduğu geçici bellek anahtarı.
const GENISLIK_BELLEGI: &str = "gorsel-dosya-listesi-genislik";

/// Sıralama anahtarlarının Türkçe etiketleri.
const SIRALAMA_ETIKETLERI: [(SiralamaTuru, &str); 4] = [
    (SiralamaTuru::Ad, "Ada göre"),
    (SiralamaTuru::Tur, "Türe göre"),
    (SiralamaTuru::Tarih, "Tarihe göre"),
    (SiralamaTuru::Boyut, "Boyuta göre"),
];

/// Dizin listesi panelini çizer; görünürlük ve yer `Ayarlar`'dan gelir.
pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    if !durum.ayarlar.dosya_listesi_acik {
        return;
    }

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

    panel
        .resizable(true)
        .size_range(egui::Rangef::new(
            *LISTE_GENISLIK_ARALIGI.start(),
            *LISTE_GENISLIK_ARALIGI.end(),
        ))
        .default_size(durum.ayarlar.dosya_listesi_genislik)
        .show(ui, |ui| {
            // Kullanıcının sürüklediği genişlik ayarlara işlenir; sürükleme bitince
            // tek kez diske yazılır (her karede yazma engellenir).
            genisligi_tazele(ui, kimlik, durum, sonuc);

            ui.horizontal(|ui| {
                ui.heading("Dizin");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ikon::ikon_dugme(ui, Ikon::Carpi, "Listeyi kapat (L)") {
                        acik = false;
                    }
                    siralama_menusu(ui, durum, sonuc);
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
                        ui.label(
                            egui::RichText::new("Bu dizinde desteklenen görsel yok").weak(),
                        );
                        return;
                    }
                    for sira in 0..durum.indeks.len() {
                        satir_ciz(ui, durum, sira, sonuc);
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

/// Sıralama menüsü: anahtar (ad/tür/tarih/boyut) ve ön (artan/azalan) seçimi.
fn siralama_menusu(ui: &mut egui::Ui, durum: &mut UygulamaDurumu, sonuc: &mut HudSonucu) {
    let ipucu = format!(
        "Sıralama: {} ({}) — değiştirmek için tıklayın",
        tur_etiketi(durum.ayarlar.siralama_turu),
        yon_etiketi(durum.ayarlar.siralama_yonu)
    );
    let yanit = ikon::ikon_dugme_yanit(ui, Ikon::Sirala, false, &ipucu);
    egui::Popup::menu(&yanit).show(|ui| {
        ui.label(egui::RichText::new("Sırala").weak());
        for (tur, etiket) in SIRALAMA_ETIKETLERI {
            let secili = durum.ayarlar.siralama_turu == tur;
            let satir = if secili {
                format!("{etiket} ✓")
            } else {
                etiket.to_string()
            };
            if menu_maddesi(ui, &satir, "Sıralama anahtarı") {
                sonuc.ekle(Eylem::SiralaTuru(tur));
                ui.close();
            }
        }
        ui.separator();
        ui.label(egui::RichText::new("Yön").weak());
        for (yon, etiket) in [
            (SiralamaYonu::Artan, "Artan (A→Z, eski→yeni)"),
            (SiralamaYonu::Azalan, "Azalan (Z→A, yeni→eski)"),
        ] {
            let secili = durum.ayarlar.siralama_yonu == yon;
            let satir = if secili {
                format!("{etiket} ✓")
            } else {
                etiket.to_string()
            };
            if menu_maddesi(ui, &satir, "Sıralama yönü") {
                sonuc.ekle(Eylem::SiralaYonu(yon));
                ui.close();
            }
        }
    });
}

/// Tek satırı çizer: küçük resim, ad, boyut/tarih, "Yeni" rozeti ve birlikte aç eylemi.
fn satir_ciz(
    ui: &mut egui::Ui,
    durum: &mut UygulamaDurumu,
    sira: usize,
    sonuc: &mut HudSonucu,
) {
    let Some(yol) = durum.indeks.get(sira).cloned() else {
        return;
    };
    let ad = yol
        .file_name()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_else(|| yol.display().to_string());

    let (dikdortgen, yanit) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), SATIR_YUKSEKLIGI),
        egui::Sense::click(),
    );

    // Seçili/hover zemin.
    let vizueller = ui.visuals();
    let zemin = if sira == durum.konum {
        vizueller.selection.bg_fill
    } else if yanit.hovered() {
        vizueller.widgets.hovered.weak_bg_fill
    } else {
        vizueller.faint_bg_color
    };
    ui.painter()
        .add(egui::Shape::rect_filled(
            dikdortgen,
            egui::CornerRadius::same(4),
            zemin,
        ));

    // Küçük resim: üretildiyse doku, üretilmediyse biçim başlıklı yer tutucu.
    let resim_dikt = egui::Rect::from_min_size(
        egui::pos2(dikdortgen.min.x + 6.0, dikdortgen.min.y + 6.0),
        egui::vec2(KUCUK_RESIM, KUCUK_RESIM),
    );
    kucuk_resim_ciz(ui, durum, &yol, sira, resim_dikt, sonuc);

    // Metin bloğu: ad + "boyut · tarih".
    let metin_x = resim_dikt.max.x + 8.0;
    let metin_renk = if sira == durum.konum {
        vizueller.selection.stroke.color
    } else {
        vizueller.text_color()
    };
    ui.painter().text(
        egui::pos2(metin_x, dikdortgen.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        super::arac_cubugu::kisalt(&ad, AD_UZUNLUGU),
        egui::FontId::proportional(13.0),
        metin_renk,
    );
    let alt_metin = durum
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
        egui::pos2(metin_x, dikdortgen.min.y + 32.0),
        egui::Align2::LEFT_TOP,
        alt_metin,
        egui::FontId::proportional(11.0),
        vizueller.weak_text_color(),
    );

    // "Yeni" rozeti: dizindeki en yeni görsel işaretlenir.
    if durum.en_yeni_yol.as_ref().is_some_and(|y| *y == yol) {
        let rozet_renk = egui::Color32::from_rgb(96, 190, 120);
        ui.painter()
            .add(egui::Shape::rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(dikdortgen.max.x - EYLEM_ALANI - 52.0, dikdortgen.min.y + 8.0),
                    egui::vec2(46.0, 16.0),
                ),
                egui::CornerRadius::same(8),
                rozet_renk,
            ));
        ui.painter().text(
            egui::pos2(dikdortgen.max.x - EYLEM_ALANI - 29.0, dikdortgen.min.y + 16.0),
            egui::Align2::CENTER_CENTER,
            "YENİ",
            egui::FontId::proportional(10.0),
            egui::Color32::BLACK,
        );
    }

    // Satır eylemi: sol ana alan tıklaması geçiş, sağdaki ok simgesi birlikte aç.
    let eylem_dikt = egui::Rect::from_min_size(
        egui::pos2(dikdortgen.max.x - EYLEM_ALANI, dikdortgen.min.y),
        egui::vec2(EYLEM_ALANI, SATIR_YUKSEKLIGI),
    );
    ikon::ciz(
        ui.painter(),
        eylem_dikt,
        Ikon::DisaAc,
        if yanit.hovered() {
            vizueller.text_color()
        } else {
            vizueller.weak_text_color()
        },
    );

    if yanit.clicked() {
        let tik = yanit.hover_pos().unwrap_or_default();
        if eylem_dikt.contains(tik) {
            sonuc.ekle(Eylem::BirlikteAcSatir(sira));
        } else {
            sonuc.ekle(Eylem::Git(sira));
        }
    }
    if sira == durum.konum && onceki_konum(ui) != durum.konum {
        yanit.scroll_to_me(Some(egui::Align::Center));
    }
}

/// Satırdaki küçük resmi çizer; ilk kez görünen satır için arka plan üretim isteği gönderir.
///
/// İstek döngüsü: eşlemede kayıt yoksa istek gönderilir (uygulama `istendi` kaydı açar).
/// `istendi = false` ve piksel yoksa biçim küçük resim üretemiyor demektir (HDR/RAW);
/// kalıcı yer tutucu gösterilir, yeniden istek gönderilmez.
fn kucuk_resim_ciz(
    ui: &egui::Ui,
    durum: &mut UygulamaDurumu,
    yol: &Path,
    sira: usize,
    resim_dikt: egui::Rect,
    sonuc: &mut HudSonucu,
) {
    let vizueller = ui.visuals();
    let gorunur = ui.clip_rect().intersects(resim_dikt);
    if !durum.onizlemeler.contains_key(yol) {
        if durum.ayarlar.onizlemeler && gorunur {
            sonuc.ekle(Eylem::OnIzlemeIste(sira));
        }
        yer_tutucu_ciz(ui, resim_dikt, vizueller);
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
        yer_tutucu_ciz(ui, resim_dikt, vizueller);
        return;
    };
    // Küçük resmi kutuya orantılı sığdır (kenar boşlukları simetrik olur).
    let g = doku.size()[0] as f32;
    let y = doku.size()[1] as f32;
    let olcek = (resim_dikt.width() / g.max(1.0)).min(resim_dikt.height() / y.max(1.0));
    let olcu = egui::vec2(g * olcek, y * olcek);
    let hedef = egui::Rect::from_center_size(resim_dikt.center(), olcu);
    ui.painter().image(
        doku.id(),
        hedef,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

/// Küçük resim hazır olana kadar çizilen soluk yer tutucu.
fn yer_tutucu_ciz(ui: &egui::Ui, dikt: egui::Rect, vizueller: &egui::Visuals) {
    ui.painter()
        .add(egui::Shape::rect_filled(
            dikt,
            egui::CornerRadius::same(4),
            vizueller.extreme_bg_color,
        ));
    ui.painter().text(
        dikt.center(),
        egui::Align2::CENTER_CENTER,
        "···",
        egui::FontId::proportional(14.0),
        vizueller.weak_text_color(),
    );
}

/// Sıralama menüsünde kullanılan son tık öncesi konum (kaydırma merkezlemesi için).
fn onceki_konum(ui: &egui::Ui) -> usize {
    ui.ctx()
        .data(|d| {
            d.get_temp::<usize>(egui::Id::new(KONUM_BELLEGI))
                .unwrap_or(usize::MAX)
        })
}

/// Panelin ölçülen genişliğini ayara işler; sürükleme durunca tek kez kaydeder.
fn genisligi_tazele(
    ui: &egui::Ui,
    kimlik: &str,
    durum: &mut UygulamaDurumu,
    sonuc: &mut HudSonucu,
) {
    // Panel iç kenar payı (varsayılan 8 px × 2) ölçüye eklenir.
    let olculen = ui.max_rect().width() + 16.0;
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
    // Ölçü sabitlendi: sürükleme bitti, bekleyen değişiklik tek kez diske yazılır.
    let bekliyor = ui.ctx().data_mut(|d| d.remove_temp::<bool>(anahtar));
    if bekliyor == Some(true) {
        sonuc.ayar_degisti = true;
    }
}

/// Sıralama anahtarının Türkçe etiketi.
pub fn tur_etiketi(tur: SiralamaTuru) -> &'static str {
    match tur {
        SiralamaTuru::Ad => "Ad",
        SiralamaTuru::Tur => "Tür",
        SiralamaTuru::Tarih => "Tarih",
        SiralamaTuru::Boyut => "Boyut",
    }
}

/// Sıralama önünün Türkçe etiketi.
pub fn yon_etiketi(yon: SiralamaYonu) -> &'static str {
    match yon {
        SiralamaYonu::Artan => "artan",
        SiralamaYonu::Azalan => "azalan",
    }
}

/// Menü maddesi; tıklandıysa `true`.
fn menu_maddesi(ui: &mut egui::Ui, etiket: &str, ipucu: &str) -> bool {
    ui.add(
        egui::Button::new(etiket)
            .frame(false)
            .min_size(egui::vec2(200.0, 0.0)),
    )
    .on_hover_text(ipucu)
    .clicked()
}

/// UNIX zaman damgasını "YYYY-AA-GG ss:dd" biçimine, sistemin yerel saatinde çevirir.
///
/// Yerel saat dilimi saptanamazsa (az sayıda Unix ortamı) UTC'ye düşülür; liste gösterimi
/// engellenmez. Geçersiz damgada "—" gösterilir.
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
    use crate::goruntu::ONIZLEME_KENAR;

    #[test]
    fn tarih_kisa_bicimi_sabit() {
        // Biçim YYYY-AA-GG ss:dd olmalı (çözünürlük dakika; saat dilimi makineye bağlıdır).
        let metin = tarih_kisa(1_791_072_000_000);
        assert_eq!(metin.len(), 16, "gelen: {metin}");
        assert_eq!(&metin[4..5], "-");
        assert_eq!(&metin[7..8], "-");
        assert_eq!(&metin[10..11], " ");
        assert_eq!(&metin[13..14], ":");
        assert!(metin[..4].chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn tarih_kisa_yuvarlama_yapmaz() {
        // Dakika altı saniyeler görünmez biçimde atılır.
        assert_eq!(tarih_kisa(0), tarih_kisa(59_999));
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
        assert_eq!(tur_etiketi(SiralamaTuru::Tarih), "Tarih");
        assert_eq!(yon_etiketi(SiralamaYonu::Artan), "artan");
        assert_eq!(yon_etiketi(SiralamaYonu::Azalan), "azalan");
    }

    #[test]
    fn onizleme_sabitleri_uyumlu() {
        assert!(ONIZLEME_KENAR >= KUCUK_RESIM as u32, "2× netlik için küçük resim kenarı, üretim kenarını geçmemeli");
        assert!(SATIR_YUKSEKLIGI >= super::super::DOKUNMATIK_HEDEF);
    }

    #[test]
    fn yol_damgasi_olmayan_dosyada_sifir_uretir() {
        let bilgi = crate::dizin::tarayici::DosyaBilgi::yol_damgasi(Path::new(
            r"C:\kesinlikle\yok\olmayan_98765.jpg",
        ));
        assert_eq!(bilgi.boyut, 0);
        assert_eq!(bilgi.tarih_ms, 0);
    }
}
