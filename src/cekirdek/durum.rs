//! Uygulama durumu: açık dosya listesi, aktif görsel, görünüm (zoom/kaydırma) ve
//! kullanıcıya gösterilen geçici durum/hata mesajları.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::ayar::Ayarlar;
use super::hata::GorselHatasi;
use crate::girdi::bolge::{Gorunum, Pivot};

/// Durum ve hata mesajlarının ekranda kalma süresi.
pub const MESAJ_SURESI: Duration = Duration::from_secs(5);

/// Kullanıcıya gösterilecek geçici mesaj.
#[derive(Debug, Clone)]
pub struct Mesaj {
    pub metin: String,
    pub ipucu: Option<String>,
    pub olusturma: Instant,
    pub onemli: bool,
}

impl Mesaj {
    pub fn suresi_gecti(&self) -> bool {
        self.olusturma.elapsed() > MESAJ_SURESI
    }

    /// Kalan gösterim oranı (1,0 = yeni, 0,0 = süresi doldu).
    pub fn kalan_oran(&self) -> f32 {
        let gecen = self.olusturma.elapsed().as_secs_f32();
        (1.0 - gecen / MESAJ_SURESI.as_secs_f32()).clamp(0.0, 1.0)
    }
}

/// Uygulamanın tüm değişken durumu.
pub struct UygulamaDurumu {
    pub ayarlar: Ayarlar,
    /// Açık dizindeki desteklenen görseller, Windows Gezgini sırasında.
    pub indeks: Vec<PathBuf>,
    /// Aktif görselin indeksteki yeri.
    pub konum: usize,
    pub gorunum: Gorunum,
    /// Aktif görselin meta verisi (yüklendikten sonra dolar).
    pub aktif: Option<crate::goruntu::meta::MetaBilgi>,
    /// Yüklenmekte olan görselin indeksteki yeri.
    pub yukleniyor: Option<usize>,
    /// Görsel yüksek dinamik aralıklı mı (HUD rozeti için).
    pub hdr_kaynak: bool,
    /// Takas zinciri HDR mı.
    pub hdr_yuzey: bool,
    pub mesaj: Option<Mesaj>,
    /// Ayarlar penceresi açık mı.
    pub ayar_penceresi: bool,
    /// Pencere tam ekran kipinde mi.
    pub tam_ekran: bool,
    /// Bağlam menüsü konumu (ekran pikseli) — açık değilse `None`.
    pub baglam_menusu: Option<(f32, f32)>,
    /// Grafik adaptörü bilgisi (HUD'da gösterilir).
    pub adaptor_bilgisi: String,
    /// Arka plan işçilerine gönderilen istek nesli; eski yanıtlar bu sayede atılır.
    pub nesil: u64,
    /// Son çizilen karenin süresi (milisaniye) — performans göstergesi.
    pub kare_suresi_ms: f32,
    /// Dosya ilişkilendirmesinin kayıt defterindeki durum (HUD'da gösterilir).
    pub iliskilendirme_durumu: String,
    /// Yerel meta veri veritabanındaki kayıt sayısı (None = veritabanı kapalı).
    pub meta_kayit_sayisi: Option<u64>,
    /// Etkin VRAM komşu dokusu sayısı (ön yükleme ayarından türetilir).
    pub vram_komsu: usize,
}

impl UygulamaDurumu {
    pub fn yeni(ayarlar: Ayarlar) -> Self {
        Self {
            ayarlar,
            indeks: Vec::new(),
            konum: 0,
            gorunum: Gorunum::default(),
            aktif: None,
            yukleniyor: None,
            hdr_kaynak: false,
            hdr_yuzey: false,
            mesaj: None,
            ayar_penceresi: false,
            tam_ekran: false,
            baglam_menusu: None,
            adaptor_bilgisi: String::new(),
            nesil: 0,
            kare_suresi_ms: 0.0,
            iliskilendirme_durumu: String::new(),
            meta_kayit_sayisi: None,
            vram_komsu: 0,
        }
    }

    /// Aktif görselin gömülü ICC profili var mı.
    pub fn icc_var(&self) -> bool {
        self.aktif.as_ref().is_some_and(|m| m.icc_var)
    }

    pub fn aktif_yol(&self) -> Option<&Path> {
        self.indeks.get(self.konum).map(PathBuf::as_path)
    }

    pub fn dosya_adi(&self) -> String {
        self.aktif_yol()
            .and_then(|y| y.file_name())
            .map(|a| a.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Görsel açık değil".to_string())
    }

    /// Dizin listesini değiştirir ve verilen yolu aktif yapar.
    ///
    /// Yol listede yoksa ilk görsele düşülür; liste boşsa konum 0'da kalır.
    pub fn indeksi_kur(&mut self, yeni: Vec<PathBuf>, odak: Option<&Path>) {
        self.indeks = yeni;
        self.konum = odak
            .and_then(|y| self.indeks.iter().position(|p| p == y))
            .unwrap_or(0);
        self.nesil = self.nesil.wrapping_add(1);
        self.aktif = None;
        self.yukleniyor = None;
    }

    /// Bir sonraki görsele geçer; liste sonundaysa `false` döner.
    pub fn sonraki(&mut self) -> bool {
        if self.konum + 1 < self.indeks.len() {
            self.konum += 1;
            self.gecis_hazirla();
            true
        } else {
            false
        }
    }

    /// Bir önceki görsele geçer; liste başındaysa `false` döner.
    pub fn onceki(&mut self) -> bool {
        if self.konum > 0 {
            self.konum -= 1;
            self.gecis_hazirla();
            true
        } else {
            false
        }
    }

    /// Belirtilen konuma gider (sınırlar içinde kırpılır).
    pub fn git(&mut self, hedef: usize) {
        if self.indeks.is_empty() {
            return;
        }
        let yeni = hedef.min(self.indeks.len() - 1);
        if yeni != self.konum {
            self.konum = yeni;
            self.gecis_hazirla();
        }
    }

    /// İlk görsele gider.
    pub fn basa_don(&mut self) {
        self.git(0);
    }

    /// Son görsele gider.
    pub fn sona_git(&mut self) {
        if !self.indeks.is_empty() {
            self.git(self.indeks.len() - 1);
        }
    }

    /// Görsel değişiminde görünümü varsayılana döndürür ve meta veriyi temizler.
    fn gecis_hazirla(&mut self) {
        self.aktif = None;
        self.yukleniyor = None;
        self.gorunum = Gorunum::default();
        self.hdr_kaynak = false;
        self.nesil = self.nesil.wrapping_add(1);
    }

    /// Yeni görsel yüklendiğinde görünümü ayarlara göre oturtur.
    pub fn gorunumu_oturt(&mut self, goruntu: (u32, u32), pencere: (u32, u32)) {
        let (gw, gh) = (goruntu.0, goruntu.1);
        let (pw, ph) = (f64::from(pencere.0.max(1)), f64::from(pencere.1.max(1)));
        self.gorunum = match self.ayarlar.varsayilan_zoom {
            super::ayar::VarsayilanZoom::Sigdir => Gorunum::sigdir(gw, gh, pw, ph),
            super::ayar::VarsayilanZoom::GercekBoyut => Gorunum::gercek_boyut(),
            super::ayar::VarsayilanZoom::PencereGenisligi => Gorunum::pencere_genisligi(gw, pw),
        };
    }

    /// Görünümü pencereye sığdırır.
    pub fn sigdir(&mut self, goruntu: (u32, u32), pencere: (u32, u32)) {
        self.gorunum = Gorunum::sigdir(
            goruntu.0,
            goruntu.1,
            f64::from(pencere.0.max(1)),
            f64::from(pencere.1.max(1)),
        );
    }

    /// Pivot odaklı yakınlaştırma uygular.
    pub fn yakinlastir(
        &mut self,
        carpan: f64,
        pivot: Pivot,
        pencere: (u32, u32),
        goruntu: (u32, u32),
    ) {
        let (pw, ph) = (f64::from(pencere.0.max(1)), f64::from(pencere.1.max(1)));
        self.gorunum.pivot_ile_olcekle(carpan, pivot, pw, ph);
        self.gorunum.sinirla(goruntu.0, goruntu.1, pw, ph);
    }

    /// Bilgilendirme mesajı gösterir.
    pub fn bilgi_ver(&mut self, metin: impl Into<String>) {
        self.mesaj = Some(Mesaj {
            metin: metin.into(),
            ipucu: None,
            olusturma: Instant::now(),
            onemli: false,
        });
    }

    /// Hata mesajını eylem ipucuyla birlikte gösterir.
    pub fn hata_ver(&mut self, hata: &GorselHatasi) {
        self.mesaj = Some(Mesaj {
            metin: hata.to_string(),
            ipucu: Some(hata.eylem_ipucu().to_string()),
            olusturma: Instant::now(),
            onemli: true,
        });
    }

    /// Süresi dolmuş mesajı temizler.
    pub fn mesaji_tazele(&mut self) {
        if self.mesaj.as_ref().is_some_and(Mesaj::suresi_gecti) {
            self.mesaj = None;
        }
    }

    /// HUD'da gösterilecek konum metni: "3 / 128".
    pub fn konum_metni(&self) -> String {
        if self.indeks.is_empty() {
            "0 / 0".to_string()
        } else {
            format!("{} / {}", self.konum + 1, self.indeks.len())
        }
    }

    /// Yakınlaştırma oranı metni: "%150".
    pub fn zoom_metni(&self) -> String {
        format!("%{:.0}", self.gorunum.olcek * 100.0)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn durum(adet: usize) -> UygulamaDurumu {
        let mut d = UygulamaDurumu::yeni(Ayarlar::default());
        let liste: Vec<PathBuf> = (0..adet)
            .map(|i| PathBuf::from(format!("r{i}.jpg")))
            .collect();
        d.indeksi_kur(liste, None);
        d
    }

    #[test]
    fn gezinme_sinirlarda_durur() {
        let mut d = durum(3);
        assert_eq!(d.konum, 0);
        assert!(!d.onceki(), "ilk görselden öncesi yok");
        assert!(d.sonraki());
        assert!(d.sonraki());
        assert_eq!(d.konum, 2);
        assert!(!d.sonraki(), "son görselden sonrası yok");
    }

    #[test]
    fn basa_ve_sona_gitme() {
        let mut d = durum(5);
        d.sona_git();
        assert_eq!(d.konum, 4);
        d.basa_don();
        assert_eq!(d.konum, 0);
    }

    #[test]
    fn git_sinir_disi_kirpilir() {
        let mut d = durum(3);
        d.git(99);
        assert_eq!(d.konum, 2);
        d.git(0);
        assert_eq!(d.konum, 0);
    }

    #[test]
    fn bos_listede_gezinme_guvenli() {
        let mut d = durum(0);
        assert!(!d.sonraki());
        assert!(!d.onceki());
        d.sona_git();
        assert_eq!(d.konum, 0);
        assert_eq!(d.konum_metni(), "0 / 0");
        assert!(d.aktif_yol().is_none());
        assert_eq!(d.dosya_adi(), "Görsel açık değil");
    }

    #[test]
    fn indeks_kurma_odagi_bulur() {
        let mut d = durum(4);
        let hedef = PathBuf::from("r2.jpg");
        d.indeksi_kur(
            vec![
                PathBuf::from("r0.jpg"),
                PathBuf::from("r1.jpg"),
                hedef.clone(),
                PathBuf::from("r3.jpg"),
            ],
            Some(&hedef),
        );
        assert_eq!(d.konum, 2);
        // Listede olmayan odak ilk görsele düşer.
        d.indeksi_kur(vec![PathBuf::from("a.jpg")], Some(&hedef));
        assert_eq!(d.konum, 0);
    }

    #[test]
    fn nesil_her_geciste_artar() {
        let mut d = durum(3);
        let ilk = d.nesil;
        d.sonraki();
        assert!(d.nesil > ilk, "görsel değişince nesil artmalı");
        let ikinci = d.nesil;
        d.indeksi_kur(vec![PathBuf::from("x.jpg")], None);
        assert!(d.nesil > ikinci);
    }

    #[test]
    fn gecis_meta_ve_gorunumu_sifirlar() {
        let mut d = durum(3);
        d.gorunum.olcek = 4.0;
        d.hdr_kaynak = true;
        d.sonraki();
        assert_eq!(d.gorunum.olcek, 1.0, "görünüm sıfırlanmalı");
        assert!(!d.hdr_kaynak);
        assert!(d.aktif.is_none());
    }

    #[test]
    fn zoom_metni_yuzde_gosterir() {
        let mut d = durum(1);
        assert_eq!(d.zoom_metni(), "%100");
        d.gorunum.olcek = 1.5;
        assert_eq!(d.zoom_metni(), "%150");
    }

    #[test]
    fn varsayilan_zoom_sigdir_uygulanir() {
        let mut d = durum(1);
        d.gorunumu_oturt((4000, 2000), (1000, 1000));
        assert!((d.gorunum.olcek - 0.25).abs() < 1e-9);
    }

    #[test]
    fn pivot_zoom_sinirlanir() {
        let mut d = durum(1);
        for _ in 0..50 {
            d.yakinlastir(
                2.0,
                Pivot {
                    ekran_x: 100.0,
                    ekran_y: 100.0,
                },
                (800, 600),
                (200, 200),
            );
        }
        assert!(d.gorunum.olcek <= crate::girdi::bolge::EN_COK_OLCEK);
    }

    #[test]
    fn hata_mesaji_ipucu_tasir() {
        let mut d = durum(1);
        d.hata_ver(&GorselHatasi::BozukVeri("test".into()));
        let mesaj = d.mesaj.expect("mesaj olmalı");
        assert!(mesaj.onemli);
        assert!(mesaj.ipucu.is_some());
        assert!(mesaj.metin.contains("test"));
    }

    #[test]
    fn mesaj_suresi_dolunca_temizlenir() {
        let mut d = durum(1);
        d.bilgi_ver("merhaba");
        assert!(d.mesaj.is_some());
        // Süresi dolmuş mesajı taklit et.
        if let Some(m) = d.mesaj.as_mut() {
            m.olusturma = Instant::now() - MESAJ_SURESI - Duration::from_secs(1);
        }
        assert!(d.mesaj.as_ref().unwrap().suresi_gecti());
        d.mesaji_tazele();
        assert!(d.mesaj.is_none());
    }
}
