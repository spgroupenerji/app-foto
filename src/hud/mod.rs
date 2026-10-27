//! HUD (baş üstü arayüz) katmanı: araç çubuğu, paneller, komut paleti, modallar ve bildirimler.
//! Tüm arayüz Türkçedir; renk ve ölçüler `tasarim` modülünden gelir.

pub mod arac_cubugu;
pub mod ayar_pencere;
pub mod baglam_menu;
pub mod bilesen;
pub mod bildirim;
pub mod dosya_listesi;
pub mod ikon;
pub mod komut_paleti;
pub mod meta_panel;
pub mod tasarim;
pub mod tuval;

use crate::cekirdek::durum::UygulamaDurumu;
use crate::girdi::Eylem;

pub use tasarim::tema_uygula;

/// HUD'un bir karede ürettiği sonuç.
#[derive(Debug, Default)]
pub struct HudSonucu {
    /// Kullanıcının tetiklediği eylemler (sırayla uygulanır).
    pub eylemler: Vec<Eylem>,
    /// Ayarlar değişti: diske yazılmalı.
    pub ayar_degisti: bool,
}

impl HudSonucu {
    /// Tek bir eylem ekler.
    pub fn ekle(&mut self, eylem: Eylem) {
        if eylem != Eylem::Yok {
            self.eylemler.push(eylem);
        }
    }
}

/// Tüm HUD katmanını çizer.
pub fn ciz(ui: &mut egui::Ui, durum: &mut UygulamaDurumu) -> HudSonucu {
    let mut sonuc = HudSonucu::default();
    arac_cubugu::ciz(ui, durum, &mut sonuc);
    dosya_listesi::ciz(ui, durum, &mut sonuc);
    meta_panel::ciz(ui, durum, &mut sonuc);
    tuval::ciz(ui, durum, &mut sonuc);

    if durum.komut_paleti {
        komut_paleti::ciz(ui, durum, &mut sonuc);
    }
    if durum.ayar_penceresi {
        ayar_pencere::ciz(ui, durum, &mut sonuc);
    }
    if durum.baglam_menusu.is_some() {
        baglam_menu::ciz(ui, durum, &mut sonuc);
    }
    bildirim::ciz(ui, durum);
    sonuc
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::cekirdek::ayar::Tema;

    #[test]
    fn sonuc_eylem_biriktirir() {
        let mut s = HudSonucu::default();
        s.ekle(Eylem::Yok);
        assert!(s.eylemler.is_empty(), "Yok eylemi eklenmez");
        s.ekle(Eylem::Sonraki);
        assert_eq!(s.eylemler, vec![Eylem::Sonraki]);
    }

    #[test]
    fn tema_iki_paleti_ve_dokunmatik_hedefi_kurar() {
        let ctx = egui::Context::default();
        tema_uygula(&ctx, Tema::Koyu, false);
        let koyu = ctx.style_of(egui::Theme::Dark);
        assert!(koyu.visuals.dark_mode);
        assert_eq!(
            koyu.spacing.interact_size,
            egui::vec2(
                tasarim::DOKUNMATIK_HEDEF,
                tasarim::DOKUNMATIK_HEDEF
            ),
            "tıklanabilir her öğe en az 44×44 olmalı"
        );

        tema_uygula(&ctx, Tema::Acik, false);
        let acik = ctx.style_of(egui::Theme::Light);
        assert!(!acik.visuals.dark_mode);
        assert_eq!(
            acik.spacing.interact_size,
            egui::vec2(
                tasarim::DOKUNMATIK_HEDEF,
                tasarim::DOKUNMATIK_HEDEF
            )
        );
    }
}
