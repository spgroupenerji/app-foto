//! Uygulama: pencere olay döngüsü, arka plan işçileriyle eşgüdüm, önbellek/ön yükleme
//! yönetimi ve kare çizimi.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::cekirdek::ayar::{Ayarlar, PanelYeri, Tema};
use crate::cekirdek::durum::UygulamaDurumu;
use crate::dizin::izleyici::{DizinIzleyici, DizinOlayi};
use crate::gio::isci::{IsciHavuzu, Istek, Yanit};
use crate::gio::onbellek::Onbellek;
use crate::gio::oneyukleme::{self, Yon};
use crate::gio::veritabani::{MetaKayit, MetaVeritabani};
use crate::girdi::bolge::{self, Pivot};
use crate::girdi::{Eylem, fare, klavye};
use crate::goruntu::IslenmisGoruntu;
use crate::gpu::Gpu;
use crate::gpu::boru::uniform_olustur;
use crate::gpu::cizim::{Cizim, EguiKare};
use crate::gpu::doku::GoruntuDokusu;
use crate::gpu::yuzey::Yuzey;
use crate::hud;
use crate::kabuk::dosya_sec::DiyalogTuru;
use crate::pencere::{self, mica, tam_ekran};

/// Yerleşim kaydı yoksa kullanılan pencere ölçüsü.
pub const VARSAYILAN_OLCU: (u32, u32) = (1280, 800);

/// İki sol tıklamanın çift tıklama sayılması için azami aralık.
pub const CIFT_TIK_ARALIGI: std::time::Duration = std::time::Duration::from_millis(400);

/// Uygulama başlatma seçenekleri.
#[derive(Debug, Clone)]
pub struct Baslatma {
    /// Komut satırından gelen görsel yolu.
    pub baslangic_yolu: Option<PathBuf>,
    /// RAM önbelleği sınırı (bayt).
    pub onbellek_bayt: u64,
}

/// Ana uygulama nesnesi.
pub struct Uygulama {
    ayarlar: Ayarlar,
    durum: UygulamaDurumu,
    isci: IsciHavuzu,
    onbellek: Onbellek<PathBuf, IslenmisGoruntu>,
    /// Komut satırından/sürükle-bırakla gelen odak dosyası.
    baslangic_yolu: Option<PathBuf>,
    /// Açık dizin (tarama, yeniden tarama ve izleyici bu dizini kullanır).
    son_dizin: Option<PathBuf>,
    /// Kullanıcının istediği fakat henüz gösterilmemiş sistem diyalogu.
    bekleyen_diyalog: Option<DiyalogTuru>,
    /// Menüden çıkış istendi mi (olay döngüsü `about_to_wait` içinde kapanır).
    cikis_istendi: bool,
    /// Paneller/tema gibi ayar alanları değişti: bir sonraki karede diske yazılır.
    ayar_degisti: bool,
    /// egui bağlamına uygulanan son tema; farklıysa bir sonraki karede yeniden uygulanır.
    tema_uygulandi: Option<Tema>,

    // Pencereye bağlı bileşenler (resumed ile kurulur).
    gpu: Option<Gpu>,
    pencere: Option<Arc<Window>>,
    yuzey: Option<Yuzey>,
    cizim: Option<Cizim>,
    doku: Option<GoruntuDokusu>,
    egui_baglam: egui::Context,
    egui_winit: Option<egui_winit::State>,

    // Dizin izleyici (notify) ve meta veri önbelleği: ikisi de isteğe bağlıdır,
    // kurulamazlarsa uygulama özelliksiz ama çalışır durumda devam eder.
    dizin_izleyici: Option<(DizinIzleyici, std::sync::mpsc::Receiver<DizinOlayi>)>,
    veritabani: Option<MetaVeritabani>,

    // Etkileşim durumu.
    ctrl: bool,
    shift: bool,
    surukleniyor: bool,
    son_imlec: (f64, f64),
    /// Son sol tıklamanın zamanı (çift tıklama tespiti için).
    son_tik: Option<Instant>,
    son_yon: Yon,
    kirli: bool,
}

impl Uygulama {
    /// Uygulamayı kurar; GPU ve pencere `resumed` olayında oluşturulur.
    pub fn yeni(ayarlar: Ayarlar, baslatma: Baslatma) -> Self {
        let onbellek_bayt = baslatma.onbellek_bayt.max(64 * 1024 * 1024);
        let isci = IsciHavuzu::baslat(ayarlar.clone());
        log::info!("işçi havuzu {} iplikle kuruldu", isci.isci_sayisi());

        let mut durum = UygulamaDurumu::yeni(ayarlar.clone());
        durum.vram_komsu = ayarlar.on_yukleme.vram_komsu();

        Self {
            durum,
            ayarlar,
            isci,
            onbellek: Onbellek::yeni(onbellek_bayt, |g: &IslenmisGoruntu| g.veri.len() as u64 * 2),
            baslangic_yolu: baslatma.baslangic_yolu,
            son_dizin: None,
            bekleyen_diyalog: None,
            cikis_istendi: false,
            ayar_degisti: false,
            tema_uygulandi: None,
            gpu: None,
            pencere: None,
            yuzey: None,
            cizim: None,
            doku: None,
            egui_baglam: egui::Context::default(),
            egui_winit: None,
            dizin_izleyici: None,
            veritabani: MetaVeritabani::varsayilan().ok(),
            ctrl: false,
            shift: false,
            surukleniyor: false,
            son_imlec: (0.0, 0.0),
            son_tik: None,
            son_yon: Yon::Yok,
            kirli: true,
        }
    }

    /// Pencere ölçüsü (yüzey yoksa varsayılan).
    fn pencere_olcusu(&self) -> (u32, u32) {
        self.yuzey
            .as_ref()
            .map(Yuzey::olcu)
            .unwrap_or(VARSAYILAN_OLCU)
    }

    /// Ağ paylaşımından açılan görselde kullanıcıyı bir kez bilgilendirir.
    fn ag_uyarisini_bildir(&mut self, yol: &std::path::Path) {
        if let Some(uyari) = crate::gio::yol::ag_uyarisi(yol) {
            self.durum.bilgi_ver(uyari);
        }
    }

    /// Aktif görselin yolunu döndürür.
    fn aktif_yol(&self) -> Option<PathBuf> {
        self.durum.aktif_yol().map(|p| p.to_path_buf())
    }

    /// Pencereyi, GPU yüzeyini, çizim katmanını ve egui durumunu kurar.
    fn pencere_kur(&mut self, olay_dongusu: &ActiveEventLoop) -> Result<(), String> {
        if self.pencere.is_some() {
            return Ok(());
        }

        let gpu = Gpu::yeni().map_err(|k| k.to_string())?;
        self.durum.adaptor_bilgisi = gpu.adaptor_bilgisi.clone();

        let ozellikler = WindowAttributes::default()
            .with_title(self.baslik_metni())
            .with_window_icon(pencere::ikon::pencere_ikonu())
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(VARSAYILAN_OLCU.0),
                f64::from(VARSAYILAN_OLCU.1),
            ))
            .with_min_inner_size(winit::dpi::LogicalSize::new(320.0, 240.0))
            // Yerleşim uygulanana kadar görünmez: pencere sıçraması olmaz.
            .with_visible(false);

        let pencere = Arc::new(
            olay_dongusu
                .create_window(ozellikler)
                .map_err(|k| format!("pencere oluşturulamadı: {k}"))?,
        );

        let ic_olcu = pencere.inner_size();
        let yuzey = Yuzey::yeni(
            &gpu,
            Arc::clone(&pencere),
            (ic_olcu.width, ic_olcu.height),
            self.ayarlar.hdr_etkin,
        )
        .map_err(|k| k.to_string())?;

        let cizim = Cizim::yeni(&gpu, yuzey.format());

        // Kayıtlı pencere yerleşimi varsa uygula, sonra pencereyi görünür kıl.
        if let Err(k) = pencere::yerlesim::yukle(&pencere) {
            log::warn!("pencere yerleşimi yüklenemedi: {k}");
        }
        pencere.set_visible(true);

        let malzeme = mica::Malzeme::ayardan(self.ayarlar.mica_etkin);
        if malzeme != mica::Malzeme::Yok {
            let uygulandi = mica::uygula(&pencere, malzeme, true);
            if !uygulandi {
                log::info!("Mica malzemesi uygulanamadı; düz arka plan kullanılıyor");
            }
        }

        let mut egui_winit = egui_winit::State::new(
            self.egui_baglam.clone(),
            egui::ViewportId::ROOT,
            &*pencere,
            Some(pencere.scale_factor() as f32),
            None,
            Some(gpu.device.limits().max_texture_dimension_2d as usize),
        );
        let _ = &mut egui_winit;

        self.durum.hdr_yuzey = yuzey.hdr_aktif();
        log::info!(
            "yüzey: {:?}, HDR {}, gama kodlaması {}",
            yuzey.format(),
            if yuzey.hdr_aktif() {
                "açık"
            } else {
                "kapalı"
            },
            if yuzey.gama_kodlamasi_shaderda() {
                "shader'da"
            } else {
                "donanımda"
            }
        );
        self.gpu = Some(gpu);
        self.egui_winit = Some(egui_winit);
        self.cizim = Some(cizim);
        self.yuzey = Some(yuzey);
        self.pencere = Some(pencere);
        Ok(())
    }

    /// Başlangıç dizinini taratır ve odağı verilen dosyaya verir.
    fn dizini_yukle(&mut self) {
        // Komut satırından KLASÖR de verilebilir: bu durumda klasör kipine geçilir.
        // (Aksi halde klasör "dosya" sanılıp üst dizini taranır ve yanlış dizin açılır.)
        if let Some(yol) = self.baslangic_yolu.clone() {
            if yol.is_dir() {
                self.baslangic_yolu = None;
                self.son_dizin = Some(yol);
                self.baslik_guncelle();
            } else if !yol.exists() {
                self.durum
                    .hata_ver(&crate::cekirdek::hata::GorselHatasi::Bulunamadi(yol));
                return;
            }
        }

        // Tarama dizini: açık klasör, yoksa odak dosyanın klasörü.
        let dizin = self.son_dizin.clone().or_else(|| {
            self.baslangic_yolu
                .as_ref()
                .and_then(|p| dosyanin_dizini(p))
        });
        let Some(dizin) = dizin else {
            self.durum.bilgi_ver(
                "Görsel açmak için Dosya ▸ Aç… kullanın, dosyayı pencereye bırakın \
                 ya da dosyayı komut satırından verin.",
            );
            return;
        };
        self.son_dizin = Some(dizin.clone());
        self.nesli_arttir();
        self.tarama_iste(dizin, self.baslangic_yolu.clone());
    }

    /// Klasör tarama isteğini kuyruğa yazar.
    fn tarama_iste(&self, dizin: PathBuf, odak: Option<PathBuf>) {
        self.isci.gonder(Istek::Tara {
            nesil: self.durum.nesil,
            dizin,
            odak,
        });
    }

    /// Açık dosyayı değiştirir (komut satırı, sürükle-bırak, diyalog veya ikincil süreç).
    pub fn yolu_ac(&mut self, yol: PathBuf) {
        self.son_dizin = dosyanin_dizini(&yol);
        self.baslangic_yolu = Some(yol);
        self.baslik_guncelle();
        self.dizini_yukle();
        self.kirli = true;
    }

    /// Bir klasörü açar: içindeki görseller doğal sırada listelenir ve ilki gösterilir.
    pub fn klasoru_ac(&mut self, dizin: PathBuf) {
        self.baslangic_yolu = None;
        self.son_dizin = Some(dizin);
        self.baslik_guncelle();
        self.dizini_yukle();
        self.kirli = true;
    }

    /// Pencere başlığını günceller (dosya adı, yoksa klasör adı).
    fn baslik_guncelle(&mut self) {
        let baslik = self.baslik_metni();
        if let Some(pencere) = self.pencere.as_ref() {
            pencere.set_title(&baslik);
        }
    }

    /// Pencere başlığı metni.
    pub fn baslik_metni(&self) -> String {
        const UYGULAMA: &str = "Görsel Görüntüleyici";
        if let Some(ad) = self.baslangic_yolu.as_ref().and_then(|p| p.file_name()) {
            return format!("{} — {UYGULAMA}", ad.to_string_lossy());
        }
        if let Some(ad) = self.son_dizin.as_ref().and_then(|p| p.file_name()) {
            return format!("{} (klasör) — {UYGULAMA}", ad.to_string_lossy());
        }
        UYGULAMA.to_string()
    }

    /// Aktif görselin çözülmesini ister; önbellekte varsa doğrudan kullanır.
    fn cozme_iste(&mut self) {
        let Some(yol) = self.aktif_yol() else {
            return;
        };
        if let Some(g) = self.onbellek.al(&yol) {
            let g = g.clone();
            self.goruntuyu_uygula(&g, false);
            return;
        }
        self.durum.yukleniyor = Some(self.durum.konum);
        let hedef = Some(self.pencere_olcusu());
        if !self.isci.gonder(Istek::Coz {
            nesil: self.durum.nesil,
            sira: self.durum.konum,
            yol,
            hedef,
            on_yukleme: false,
        }) {
            self.durum.yukleniyor = None;
            self.durum
                .hata_ver(&crate::cekirdek::hata::GorselHatasi::Gio(
                    "arka plan işçisi kullanılamıyor".into(),
                ));
        }
    }

    /// İşlenmiş görüntüyü duruma ve GPU'ya uygular.
    fn goruntuyu_uygula(&mut self, g: &IslenmisGoruntu, on_yukleme: bool) {
        if !on_yukleme {
            // Aktif görsel yüklendiğinde tanılama günlüğü (ön yükleme gürültü yapmaz).
            log::info!(
                "görsel yüklendi: {} ({}×{}{}{})",
                g.meta.dosya_adi,
                g.meta.genislik,
                g.meta.yukseklik,
                if g.meta.icc_uygulandi { ", ICC" } else { "" },
                if g.meta.kucultuldu {
                    ", ön küçültme"
                } else {
                    ""
                }
            );
            self.durum.hdr_kaynak = g.hdr;
            self.durum.aktif = Some(g.meta.clone());
            self.durum.yukleniyor = None;
            self.durum
                .gorunumu_oturt((g.genislik, g.yukseklik), self.pencere_olcusu());
        }
        self.doku_yukle(g);
        self.durum.meta_kayit_sayisi = self.meta_kayit_sayisi();
        self.durum.vram_komsu = self.ayarlar.on_yukleme.vram_komsu();
        self.kirli = true;
    }

    /// Görüntüyü VRAM'e yükler.
    fn doku_yukle(&mut self, g: &IslenmisGoruntu) {
        let sonuc = {
            let (Some(gpu), Some(cizim)) = (self.gpu.as_ref(), self.cizim.as_ref()) else {
                return;
            };
            GoruntuDokusu::yeni(
                gpu,
                &g.veri,
                g.genislik,
                g.yukseklik,
                &cizim.boru.yerlesim,
                &cizim.boru.uniform_tamponu,
                &cizim.boru.ornekleyici,
            )
        };
        match sonuc {
            Ok(doku) => self.doku = Some(doku),
            Err(k) => self.durum.hata_ver(&k),
        }
    }

    /// Arka plan işçilerinden gelen yanıtları işler.
    fn yanitlari_isle(&mut self) {
        // Ödünç çakışmasını önlemek için tutulacak yollar önbelleğe dokunmadan hesaplanır.
        let zaten_tutulacak = self.tutulacak_yollar();
        for yanit in self.isci.yanitlari_topla() {
            match yanit {
                Yanit::Tarandi { nesil, odak, sonuc } => {
                    if nesil != self.durum.nesil {
                        continue;
                    }
                    match sonuc {
                        Ok(liste) => {
                            if let Some(y) = odak.as_ref() {
                                self.ag_uyarisini_bildir(y);
                            }
                            if liste.is_empty() {
                                self.durum.hata_ver(
                                    &crate::cekirdek::hata::GorselHatasi::DesteklenmeyenBicim(
                                        "bu dizinde desteklenen görsel yok".into(),
                                    ),
                                );
                            }
                            self.durum.indeksi_kur(liste, odak.as_deref());
                            if let Some(dizin) = self.son_dizin.clone() {
                                self.izleyiciyi_kur(&dizin);
                            }
                            self.cozme_iste();
                        }
                        Err(k) => self.durum.hata_ver(&k),
                    }
                }
                Yanit::Cozuldu {
                    nesil,
                    sira,
                    on_yukleme,
                    sonuc,
                } => {
                    if nesil != self.durum.nesil {
                        continue;
                    }
                    match sonuc {
                        Ok(g) => {
                            let yol = g.meta.yol.clone();
                            let aktif_mi = !on_yukleme && sira == self.durum.konum;
                            if aktif_mi {
                                self.goruntuyu_uygula(&g, false);
                            }
                            self.meta_kaydet(&g);
                            self.onbellek.koy(yol, g);
                        }
                        Err(k) => {
                            if on_yukleme {
                                log::debug!("ön yükleme başarısız: {k}");
                            } else {
                                self.durum.yukleniyor = None;
                                self.durum.hata_ver(&k);
                            }
                        }
                    }
                }
            }
        }
        self.onbellek
            .suz_ve_tahliye(|yol| zaten_tutulacak.contains(yol));
    }

    /// Dizin izleyiciyi kurar; ayar kapalıysa veya kurulum başarısızsa sessizce devam eder.
    fn izleyiciyi_kur(&mut self, dizin: &std::path::Path) {
        if !self.ayarlar.dizin_izle {
            self.dizin_izleyici = None;
            return;
        }
        // Zaten bu dizin izleniyorsa yeniden kurulmaz.
        if self
            .dizin_izleyici
            .as_ref()
            .is_some_and(|(izleyici, _)| izleyici.dizin() == dizin)
        {
            return;
        }
        match DizinIzleyici::baslat(dizin) {
            Ok(cift) => self.dizin_izleyici = Some(cift),
            Err(k) => {
                log::warn!("dizin izleyici kurulamadı: {k}");
                self.dizin_izleyici = None;
            }
        }
    }

    /// Dizin değişiklik olaylarını işler; liste gerçekten değiştiyse yeniden tarar.
    fn izleyici_olaylarini_isle(&mut self) {
        let Some((_, alici)) = self.dizin_izleyici.as_ref() else {
            return;
        };
        let mut yeniden_tara = false;
        while let Ok(olay) = alici.try_recv() {
            match olay {
                DizinOlayi::Eklendi(_) | DizinOlayi::Silindi(_) | DizinOlayi::TopluDegisim => {
                    yeniden_tara = true;
                }
            }
        }
        if yeniden_tara {
            log::debug!("dizin değişti, liste yeniden taranıyor");
            self.dizini_yeniden_tara();
        }
    }

    /// Açık dizini, aktif görsel odağını koruyarak yeniden tarar.
    fn dizini_yeniden_tara(&mut self) {
        let Some(dizin) = self.son_dizin.clone() else {
            return;
        };
        self.nesli_arttir();
        self.tarama_iste(dizin, self.baslangic_yolu.clone());
    }

    /// Çözülen görselin meta verisini yerel veritabanına yazar (hata uygulamayı durdurmaz).
    fn meta_kaydet(&self, g: &IslenmisGoruntu) {
        let Some(veritabani) = self.veritabani.as_ref() else {
            return;
        };
        let (boyut, degisim_ms) =
            crate::gio::okuyucu::dosya_damgasi(&g.meta.yol).unwrap_or((g.meta.dosya_boyutu, 0));
        let kayit = MetaKayit {
            genislik: g.meta.genislik,
            yukseklik: g.meta.yukseklik,
            yonelim: g.meta.yonelim,
            dosya_boyutu: boyut,
            degisim_ms,
        };
        if let Err(k) = veritabani.koy(&g.meta.yol, kayit) {
            log::debug!("meta veri yazılamadı: {k}");
        }
    }

    /// Veritabanındaki kayıt sayısı (HUD'da gösterilir).
    fn meta_kayit_sayisi(&self) -> Option<u64> {
        self.veritabani.as_ref()?.kayit_sayisi().ok()
    }

    /// RAM önbelleğinde tutulacak yollar: aktif görsel ve kayan pencere komşuları.
    ///
    /// Küme, önbellek değiştirilmeden ÖNCE hesaplanır; böylece aynı anda hem önbellek
    /// (değiştirilebilir) hem de durum (paylaşılan) ödünç alınmaz.
    fn tutulacak_yollar(&self) -> HashSet<PathBuf> {
        oneyukleme::tutulacak_indeksler(
            self.durum.konum,
            self.durum.indeks.len(),
            self.ayarlar.on_yukleme,
        )
        .iter()
        .filter_map(|i| self.durum.indeks.get(*i).cloned())
        .collect()
    }

    /// Kayan pencere ön yüklemesini besler (kuyruk boşken).
    fn on_yuklemeyi_besle(&mut self) {
        if !self.isci.kuyruk_bos() || self.durum.yukleniyor.is_some() {
            return;
        }
        let plan = oneyukleme::plan(
            self.durum.konum,
            self.durum.indeks.len(),
            self.ayarlar.on_yukleme,
            self.son_yon,
        );
        let hedef = Some(self.pencere_olcusu());
        for sira in plan {
            let Some(yol) = self.durum.indeks.get(sira).cloned() else {
                continue;
            };
            if self.onbellek.icerir(&yol) {
                continue;
            }
            self.isci.gonder(Istek::Coz {
                nesil: self.durum.nesil,
                sira,
                yol,
                hedef,
                on_yukleme: true,
            });
        }
    }

    /// Kullanıcı eylemini uygular.
    fn eylem_uygula(&mut self, eylem: Eylem) {
        let (ww, wh) = self.pencere_olcusu();
        let goruntu = self
            .doku
            .as_ref()
            .map(|d| (d.genislik, d.yukseklik))
            .unwrap_or((0, 0));

        match eylem {
            Eylem::Onceki => {
                let onceki = self.durum.konum;
                if self.durum.onceki() {
                    self.son_yon = oneyukleme::yon_belirle(onceki, self.durum.konum);
                    self.cozme_iste();
                }
            }
            Eylem::Sonraki => {
                let onceki = self.durum.konum;
                if self.durum.sonraki() {
                    self.son_yon = oneyukleme::yon_belirle(onceki, self.durum.konum);
                    self.cozme_iste();
                }
            }
            Eylem::Ilk => {
                if self.durum.konum != 0 {
                    self.son_yon = Yon::Geri;
                    self.durum.basa_don();
                    self.cozme_iste();
                }
            }
            Eylem::Son => {
                if self.durum.konum + 1 != self.durum.indeks.len() {
                    self.son_yon = Yon::Ileri;
                    self.durum.sona_git();
                    self.cozme_iste();
                }
            }
            Eylem::Yakinlastir { carpan, pivot } => {
                let p = pivot.unwrap_or(Pivot {
                    ekran_x: f64::from(ww) / 2.0,
                    ekran_y: f64::from(wh) / 2.0,
                });
                self.durum.yakinlastir(carpan, p, (ww, wh), goruntu);
            }
            Eylem::Sigdir => self.durum.sigdir(goruntu, (ww, wh)),
            Eylem::GercekBoyut => {
                self.durum.gorunum = crate::girdi::bolge::Gorunum::gercek_boyut();
            }
            Eylem::KaydirmaBaslat => self.surukleniyor = true,
            Eylem::KaydirmaBitir => self.surukleniyor = false,
            Eylem::Surukle { dx, dy } => {
                if self.surukleniyor {
                    self.durum.gorunum.kaydir(dx, dy);
                    self.durum.gorunum.sinirla(
                        goruntu.0,
                        goruntu.1,
                        f64::from(ww.max(1)),
                        f64::from(wh.max(1)),
                    );
                }
            }
            Eylem::TamEkranDegistir => {
                if let Some(pencere) = self.pencere.as_ref() {
                    let hedef = !tam_ekran::tam_ekranda(pencere);
                    self.durum.tam_ekran = tam_ekran::degistir(pencere, hedef);
                }
            }
            Eylem::DosyaAc => self.bekleyen_diyalog = Some(DiyalogTuru::Dosya),
            Eylem::KlasorAc => self.bekleyen_diyalog = Some(DiyalogTuru::Klasor),
            Eylem::BirlikteAc => self.birlikte_ac(),
            Eylem::IliskilendirmeKaydet => self.iliskilendirme_kaydet(),
            Eylem::IliskilendirmeKaldir => self.iliskilendirme_kaldir(),
            Eylem::VarsayilanUygulamaSayfasi => {
                match crate::kabuk::iliskilendirme::varsayilan_uygulama_sayfasini_ac() {
                    Ok(()) => self
                        .durum
                        .bilgi_ver("Windows varsayılan uygulamalar sayfası açıldı."),
                    Err(k) => self.durum.hata_ver(&k),
                }
            }
            Eylem::YenidenYukle => {
                self.onbellek.temizle();
                self.nesli_arttir();
                self.cozme_iste();
                self.durum.bilgi_ver("Görsel yeniden yüklendi.");
            }
            Eylem::AyarPenceresi => self.durum.ayar_penceresi = !self.durum.ayar_penceresi,
            Eylem::MetaPaneliDegistir => {
                self.ayarlar.bilgi_paneli_acik = !self.ayarlar.bilgi_paneli_acik;
                self.ayar_degisti = true;
            }
            Eylem::Git(sira) => {
                let onceki = self.durum.konum;
                self.durum.git(sira);
                if self.durum.konum != onceki {
                    self.son_yon = oneyukleme::yon_belirle(onceki, self.durum.konum);
                    self.cozme_iste();
                }
            }
            Eylem::DosyaListesiDegistir => {
                self.ayarlar.dosya_listesi_acik = !self.ayarlar.dosya_listesi_acik;
                self.ayar_degisti = true;
            }
            Eylem::DosyaListesiYeriDegistir => {
                self.ayarlar.dosya_listesi_yeri = match self.ayarlar.dosya_listesi_yeri {
                    PanelYeri::Sol => PanelYeri::Sag,
                    PanelYeri::Sag => PanelYeri::Sol,
                };
                self.ayar_degisti = true;
            }
            Eylem::TemaSec(tema) => {
                if self.ayarlar.tema != tema {
                    self.ayarlar.tema = tema;
                    self.ayar_degisti = true;
                }
            }
            Eylem::BaglamMenusu { x, y } => self.durum.baglam_menusu = Some((x as f32, y as f32)),
            Eylem::BaglamMenusuKapat => self.durum.baglam_menusu = None,
            Eylem::Kapat => {
                // Olay döngüsü yalnızca `about_to_wait` içinde kapatılabilir; bayrak kullanılır.
                self.cikis_istendi = true;
            }
            Eylem::Yok => {}
        }
        self.kirli = true;
    }

    /// Nesli ilerletir: uçuştaki eski istekler geçersiz olur.
    fn nesli_arttir(&mut self) {
        self.durum.nesil = self.durum.nesil.wrapping_add(1);
        self.isci.nesli_guncelle(self.durum.nesil);
    }

    /// Aktif görseli harici bir uygulamada açar.
    fn birlikte_ac(&mut self) {
        let Some(yol) = self.aktif_yol() else {
            return;
        };
        match crate::kabuk::birlikte_ac::birlikte_ac(&yol) {
            Ok(()) => log::info!("birlikte aç: {}", yol.display()),
            Err(k) => self.durum.hata_ver(&k),
        }
    }

    /// Dosya ilişkilendirmesini kaydeder.
    fn iliskilendirme_kaydet(&mut self) {
        let Some(exe) = std::env::current_exe().ok() else {
            self.durum
                .hata_ver(&crate::cekirdek::hata::GorselHatasi::Kabuk(
                    "uygulama yolu belirlenemedi".into(),
                ));
            return;
        };
        match crate::kabuk::iliskilendirme::iliskilendirmeyi_kaydet(&exe) {
            Ok(()) => {
                self.durum.iliskilendirme_durumu = iliskilendirme_metni();
                self.durum.bilgi_ver(
                    "Dosya ilişkilendirmesi kaydedildi; varsayılan uygulama sayfasından seçebilirsiniz.",
                );
            }
            Err(k) => self.durum.hata_ver(&k),
        }
    }

    /// Dosya ilişkilendirme kayıtlarını siler.
    fn iliskilendirme_kaldir(&mut self) {
        match crate::kabuk::iliskilendirme::iliskilendirmeyi_kaldir() {
            Ok(()) => {
                self.durum.iliskilendirme_durumu = iliskilendirme_metni();
                self.durum
                    .bilgi_ver("Dosya ilişkilendirme kayıtları silindi.");
            }
            Err(k) => self.durum.hata_ver(&k),
        }
    }

    /// Bekleyen sistem diyalogunu gösterir.
    ///
    /// Diyalog modal olduğu ve kendi ileti döngüsünü çalıştırdığı için kare çiziminin
    /// DIŞINDA, `about_to_wait` içinde çağrılır; aksi halde çizim geri çağrısı içinde
    /// iç içe ileti döngüsü oluşur.
    fn bekleyen_diyalogu_isle(&mut self) {
        let Some(tur) = self.bekleyen_diyalog.take() else {
            return;
        };
        // Başlangıç klasörü: açık görselin/klasörün bulunduğu yer.
        let baslangic = self.son_dizin.clone();
        let secim = match crate::kabuk::dosya_sec::sec(tur, baslangic.as_deref()) {
            Ok(secim) => secim,
            Err(k) => {
                self.durum.hata_ver(&k);
                return;
            }
        };
        match (tur, secim) {
            // Kullanıcı iptal etti: sessiz kal.
            (_, None) => {}
            (DiyalogTuru::Dosya, Some(yol)) => self.dosya_birakildi(yol),
            (DiyalogTuru::Klasor, Some(yol)) => {
                self.klasoru_ac(yol);
                self.durum
                    .bilgi_ver("Klasör açıldı; ilk görsel gösteriliyor.");
            }
        }
        self.yuzeyi_tazele();
    }

    /// Modal diyalog sonrası takas zincirini tazeler ve yeniden çizim ister.
    fn yuzeyi_tazele(&mut self) {
        if let (Some(gpu), Some(yuzey)) = (self.gpu.as_ref(), self.yuzey.as_mut()) {
            yuzey.zorla_yeniden_yapilandir(gpu);
        }
        self.kirli = true;
        if let Some(pencere) = self.pencere.as_ref() {
            pencere.request_redraw();
        }
    }

    /// Pencereyi kapatır ve yerleşimi kaydeder.
    fn kapat(&mut self) {
        if let Some(pencere) = self.pencere.as_ref() {
            if let Err(k) = pencere::yerlesim::kaydet(pencere) {
                log::warn!("pencere yerleşimi kaydedilemedi: {k}");
            }
        }
        if let Err(k) = self.ayarlar.kaydet() {
            log::warn!("ayarlar kaydedilemedi: {k}");
        }
    }

    /// Bir kareyi çizer.
    ///
    /// Ödünçler dar kapsamlarda tutulur: `cizim` yalnızca gerçek çizim anında değiştirilebilir
    /// olarak alınır, böylece eylem uygulama (kendisi de `&mut self` ister) arada çalışabilir.
    fn cerceve_ciz(&mut self) {
        let baslangic = Instant::now();
        if self.gpu.is_none()
            || self.pencere.is_none()
            || self.yuzey.is_none()
            || self.cizim.is_none()
        {
            return;
        }
        let (ww, wh) = match self.yuzey.as_ref() {
            Some(yuzey) => yuzey.olcu(),
            None => return,
        };

        let ham_girdi = {
            let (Some(pencere), Some(egui_winit)) =
                (self.pencere.as_ref(), self.egui_winit.as_mut())
            else {
                return;
            };
            egui_winit.take_egui_input(pencere)
        };

        // egui bağlamı Arc tabanlı bir tutamaçtır: klonlamak ucuzdur ve ödünç çakışmasını önler.
        let ctx = self.egui_baglam.clone();
        // Tema tercihi değiştiyse (ilk kare dahil) palet ve tercih yeniden uygulanır.
        if self.tema_uygulandi != Some(self.ayarlar.tema) {
            hud::tema_uygula(&ctx, self.ayarlar.tema);
            self.tema_uygulandi = Some(self.ayarlar.tema);
            self.kirli = true;
        }
        let durum = &mut self.durum;
        let mut hud_sonuc = hud::HudSonucu::default();
        // egui 0.36 kök arayüzü `Ui` üzerinden çalıştırır; paneller bu kökün içine yerleşir.
        let tam_cikti = ctx.run_ui(ham_girdi, |ui| {
            hud_sonuc = hud::ciz(ui, durum);
        });

        if let (Some(pencere), Some(s)) = (self.pencere.as_ref(), self.egui_winit.as_mut()) {
            s.handle_platform_output(pencere, tam_cikti.platform_output.clone());
        }

        let egui::FullOutput {
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = tam_cikti;

        let hud::HudSonucu {
            eylemler,
            ayar_degisti: hud_ayari_degisti,
        } = hud_sonuc;
        for eylem in eylemler {
            self.eylem_uygula(eylem);
        }
        // Paneller ve tema (`self.ayar_degisti`) ile ayar penceresi (`hud_ayari_degisti`)
        // aynı kayıt yolunu paylaşır; değişiklik anında diske yazılır.
        if hud_ayari_degisti || self.ayar_degisti {
            self.ayar_degisti = false;
            if let Err(k) = self.ayarlar.kaydet() {
                log::warn!("ayarlar kaydedilemedi: {k}");
            }
        }

        let (dw, dh) = self
            .doku
            .as_ref()
            .map(|d| (d.genislik, d.yukseklik))
            .unwrap_or((1, 1));
        let uniform = {
            let Some(yuzey) = self.yuzey.as_ref() else {
                return;
            };
            uniform_olustur(
                (dw, dh),
                &self.durum.gorunum,
                (ww, wh),
                &yuzey.hdr,
                self.durum.hdr_kaynak,
                yuzey.gama_kodlamasi_shaderda(),
                self.ayarlar.arka_plan,
            )
        };

        let kare_durumu = {
            let (Some(gpu), Some(yuzey), Some(cizim)) =
                (self.gpu.as_ref(), self.yuzey.as_ref(), self.cizim.as_mut())
            else {
                return;
            };
            cizim.boru.uniform_yaz(gpu, &uniform);

            let ekran = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [ww, wh],
                pixels_per_point,
            };
            let cizim_isleri = ctx.tessellate(shapes, pixels_per_point);
            let egui_kare = EguiKare {
                doku_deltalari: textures_delta,
                cizim_isleri,
                ekran,
            };
            cizim.kare_ciz(gpu, yuzey, self.doku.as_ref(), &uniform, Some(egui_kare))
        };

        if kare_durumu.yeniden_yapilandirma_gerekli() {
            if let (Some(gpu), Some(yuzey)) = (self.gpu.as_ref(), self.yuzey.as_mut()) {
                yuzey.yeniden_boyutla(gpu, ww, wh);
            }
        }
        if let Some(metin) = kare_durumu.hata_metni() {
            log::warn!("{metin}");
        }

        self.durum.kare_suresi_ms = baslangic.elapsed().as_secs_f32() * 1000.0;
        self.durum.mesaji_tazele();
        self.kirli = false;
    }

    /// Dışarıdan gelen yol: görsel dosyası, klasör veya desteklenmeyen dosya.
    ///
    /// Klasör verilirse içindeki görseller listelenir ve ilki gösterilir; bu yol hem
    /// komut satırı, hem sürükle-bırak, hem de ikincil süreç istekleri için geçerlidir.
    fn dosya_birakildi(&mut self, yol: PathBuf) {
        if yol.is_dir() {
            self.klasoru_ac(yol);
            self.durum
                .bilgi_ver("Klasör açıldı; ilk görsel gösteriliyor.");
        } else if crate::dizin::tarayici::gorsel_adayi_mi(&yol) {
            self.yolu_ac(yol);
        } else {
            self.durum
                .hata_ver(&crate::cekirdek::hata::GorselHatasi::DesteklenmeyenBicim(
                    format!("{} desteklenen bir görsel değil", yol.display()),
                ));
        }
    }
}

/// Bir dosya yolunun klasörünü döndürür; yol yalnızca dosya adıysa `None`.
pub fn dosyanin_dizini(yol: &std::path::Path) -> Option<PathBuf> {
    yol.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
}

/// İlişkilendirme durumunu Türkçe metne çevirir.
pub fn iliskilendirme_metni() -> String {
    use crate::kabuk::iliskilendirme::Durum;
    match crate::kabuk::iliskilendirme::durum() {
        Durum::Kayitli => "Kayıtlı".to_string(),
        Durum::Kismi => "Kısmen kayıtlı".to_string(),
        Durum::KayitliDegil => "Kayıtlı değil".to_string(),
    }
}

impl ApplicationHandler<PathBuf> for Uygulama {
    fn resumed(&mut self, olay_dongusu: &ActiveEventLoop) {
        if let Err(k) = self.pencere_kur(olay_dongusu) {
            log::error!("pencere kurulamadı: {k}");
            self.durum.mesaj = Some(crate::cekirdek::durum::Mesaj {
                metin: format!("Pencere kurulamadı: {k}"),
                ipucu: Some("Grafik sürücüsünü güncelleyip tekrar deneyin.".into()),
                olusturma: Instant::now(),
                onemli: true,
            });
            return;
        }
        self.durum.iliskilendirme_durumu = iliskilendirme_metni();
        self.dizini_yukle();
        self.kirli = true;
    }

    fn window_event(
        &mut self,
        olay_dongusu: &ActiveEventLoop,
        _pencere_kimligi: WindowId,
        olay: WindowEvent,
    ) {
        // egui önce görsün: tükettiği olaylar uygulamaya iletilmez.
        if let (Some(egui_winit), Some(pencere)) = (self.egui_winit.as_mut(), self.pencere.as_ref())
        {
            let yanit = egui_winit.on_window_event(pencere, &olay);
            if yanit.repaint {
                self.kirli = true;
            }
            if yanit.consumed {
                return;
            }
        }

        match olay {
            WindowEvent::CloseRequested => {
                self.kapat();
                olay_dongusu.exit();
            }
            WindowEvent::Resized(olcu) => {
                if let (Some(gpu), Some(yuzey)) = (self.gpu.as_ref(), self.yuzey.as_mut()) {
                    yuzey.yeniden_boyutla(gpu, olcu.width, olcu.height);
                }
                self.kirli = true;
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(pencere), Some(gpu), Some(yuzey)) = (
                    self.pencere.as_ref(),
                    self.gpu.as_ref(),
                    self.yuzey.as_mut(),
                ) {
                    let olcu = pencere.inner_size();
                    yuzey.yeniden_boyutla(gpu, olcu.width, olcu.height);
                }
                self.kirli = true;
            }
            WindowEvent::RedrawRequested => self.cerceve_ciz(),
            WindowEvent::DroppedFile(yol) => self.dosya_birakildi(yol),
            WindowEvent::ModifiersChanged(modifier) => {
                let durum = modifier.state();
                self.ctrl = durum.control_key();
                self.shift = durum.shift_key();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let onceki = self.son_imlec;
                self.son_imlec = (position.x, position.y);
                if self.surukleniyor {
                    self.eylem_uygula(Eylem::Surukle {
                        dx: position.x - onceki.0,
                        dy: position.y - onceki.1,
                    });
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    let (ww, _) = self.pencere_olcusu();
                    match button {
                        MouseButton::Left => {
                            // Çift tıklama: pencere sistemleri tık sayısını vermez,
                            // bu yüzden iki basış arasındaki süre ölçülür.
                            let simdi = Instant::now();
                            let cift = self.son_tik.is_some_and(|onceki| {
                                simdi.duration_since(onceki) < CIFT_TIK_ARALIGI
                            });
                            if cift {
                                self.son_tik = None;
                                let eylem = fare::cift_tik_eylemi(self.ayarlar.cift_tik_tam_ekran);
                                self.eylem_uygula(eylem);
                            } else {
                                self.son_tik = Some(simdi);
                                let bolge = bolge::bolge_belirle(self.son_imlec.0, f64::from(ww));
                                let eylem = fare::sol_tik_eylemi(bolge);
                                self.eylem_uygula(eylem);
                            }
                        }
                        MouseButton::Right => {
                            let gezinme = self.ayarlar.sag_tik
                                == crate::cekirdek::ayar::SagTikDavranisi::Gezinme;
                            let eylem =
                                fare::sag_tik_eylemi(gezinme, self.son_imlec.0, self.son_imlec.1);
                            self.eylem_uygula(eylem);
                        }
                        MouseButton::Middle => self.eylem_uygula(Eylem::KaydirmaBaslat),
                        _ => {}
                    }
                } else {
                    match button {
                        MouseButton::Left | MouseButton::Middle => {
                            self.eylem_uygula(Eylem::KaydirmaBitir);
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let pivot = Pivot {
                    ekran_x: self.son_imlec.0,
                    ekran_y: self.son_imlec.1,
                };
                let eylem = fare::tekerlek_eylemi(
                    delta,
                    self.ctrl,
                    pivot,
                    self.ayarlar.tekerlek_zoom_carpani,
                );
                self.eylem_uygula(eylem);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if let PhysicalKey::Code(kod) = event.physical_key {
                    // Çift tıklama benzeri hızlı gezinme için tekrar sayısı yok sayılır.
                    if kod == KeyCode::KeyI {
                        self.eylem_uygula(Eylem::MetaPaneliDegistir);
                        return;
                    }
                    let eylem = klavye::tus_eylemi(kod, self.ctrl, self.shift);
                    self.eylem_uygula(eylem);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, olay_dongusu: &ActiveEventLoop) {
        if self.cikis_istendi {
            self.kapat();
            olay_dongusu.exit();
            return;
        }

        self.yanitlari_isle();
        self.izleyici_olaylarini_isle();
        self.on_yuklemeyi_besle();
        self.bekleyen_diyalogu_isle();

        if self.pencere.is_none() {
            return;
        }
        if self.kirli || self.durum.mesaj.is_some() {
            if let Some(pencere) = self.pencere.as_ref() {
                pencere.request_redraw();
            }
        }
        let _ = olay_dongusu;
    }

    fn exiting(&mut self, _olay_dongusu: &ActiveEventLoop) {
        self.kapat();
    }

    /// İkincil süreçten gelen "şu dosyayı aç" isteği.
    fn user_event(&mut self, _olay_dongusu: &ActiveEventLoop, yol: PathBuf) {
        self.dosya_birakildi(yol);
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn uygulama(yol: Option<PathBuf>) -> Uygulama {
        Uygulama::yeni(
            Ayarlar::default(),
            Baslatma {
                baslangic_yolu: yol,
                onbellek_bayt: 64 * 1024 * 1024,
            },
        )
    }

    #[test]
    fn baslik_dosya_adi_tasir() {
        let u = uygulama(Some(PathBuf::from(r"C:\resimler\deniz.jpg")));
        let baslik = u.baslik_metni();
        assert!(baslik.starts_with("deniz.jpg"), "gelen: {baslik}");
        assert!(baslik.contains("Görsel Görüntüleyici"));
    }

    #[test]
    fn baslik_yol_yoksa_yalnizca_uygulama_adi() {
        assert_eq!(uygulama(None).baslik_metni(), "Görsel Görüntüleyici");
    }

    #[test]
    fn baslik_klasor_acilinca_klasoru_gosterir() {
        let mut u = uygulama(None);
        u.klasoru_ac(PathBuf::from(r"C:\resimler\tatil"));
        let baslik = u.baslik_metni();
        assert!(baslik.starts_with("tatil"), "gelen: {baslik}");
        assert!(baslik.contains("klasör"));
    }

    #[test]
    fn klasor_acma_dosya_odagini_temizler() {
        let mut u = uygulama(Some(PathBuf::from(r"C:\resimler\a.jpg")));
        u.klasoru_ac(PathBuf::from(r"C:\baska"));
        assert_eq!(u.son_dizin, Some(PathBuf::from(r"C:\baska")));
        assert!(
            u.baslangic_yolu.is_none(),
            "klasör açılınca dosya odağı temizlenmeli (ilk görsel gösterilir)"
        );
    }

    #[test]
    fn yolu_acma_dizini_dosyadan_turetir() {
        let mut u = uygulama(None);
        u.yolu_ac(PathBuf::from(r"C:\resimler\deniz.jpg"));
        assert_eq!(u.son_dizin, Some(PathBuf::from(r"C:\resimler")));
        assert_eq!(
            u.baslangic_yolu,
            Some(PathBuf::from(r"C:\resimler\deniz.jpg"))
        );
    }

    #[test]
    fn klasor_yolu_klasor_kipine_gecer() {
        // Regresyon: klasör yolu "dosya" sanılıp üst dizini taranmamalı.
        let dizin = std::env::temp_dir();
        let mut u = uygulama(Some(dizin.clone()));
        u.dizini_yukle();
        assert!(
            u.baslangic_yolu.is_none(),
            "klasör yolu dosya odağı olarak kalmamalı"
        );
        assert_eq!(u.son_dizin.as_deref(), Some(dizin.as_path()));
    }

    #[test]
    fn olmayan_yol_hata_verir_ve_taramaz() {
        let mut u = uygulama(Some(PathBuf::from(r"C:\kesinlikle\yokoyle.jpg")));
        u.dizini_yukle();
        assert!(u.son_dizin.is_none(), "olmayan yol için tarama yapılmamalı");
        assert!(u.durum.mesaj.as_ref().is_some_and(|m| m.onemli));
    }

    #[test]
    fn dosyanin_dizini_yol_ayirir() {
        assert_eq!(
            dosyanin_dizini(std::path::Path::new(r"C:\a\b.jpg")),
            Some(PathBuf::from(r"C:\a"))
        );
        assert_eq!(dosyanin_dizini(std::path::Path::new("b.jpg")), None);
    }
}
