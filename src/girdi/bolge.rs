//! Pencere alanının yatay bölgelerine göre tıklama eşlemesi.
//!
//! Kullanıcı sözleşmesi: sol üçte bir = önceki görsel, sağ üçte bir = sonraki görsel,
//! orta üçte bir = serbest kaydırma (pan) alanı.

/// Pencerenin yatay bölgesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bolge {
    Sol,
    Orta,
    Sag,
}

/// Yatay konumu üçte birlik dilimlere göre sınıflandırır.
///
/// Sıfır genişlikte (pencere henüz boyutlanmamışken) veya imleç alan dışındayken
/// güvenli varsayılan olan [`Bolge::Orta`] döner.
pub fn bolge_belirle(x: f64, genislik: f64) -> Bolge {
    if !genislik.is_finite() || genislik <= 0.0 || !x.is_finite() {
        return Bolge::Orta;
    }
    let uc = genislik / 3.0;
    if x < uc {
        Bolge::Sol
    } else if x >= uc * 2.0 {
        Bolge::Sag
    } else {
        Bolge::Orta
    }
}

/// Yakınlaştırma odağı: ekran koordinatını görüntü uzayına bağlar.
///
/// `Ctrl + tekerlek` sırasında imlecin altındaki pikselin zoom sonrası aynı yerde
/// kalmasını sağlayan dönüşümü üretir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pivot {
    /// İmlecin pencere içi konumu (fiziksel piksel).
    pub ekran_x: f64,
    pub ekran_y: f64,
}

/// Görüntünün pencere içindeki yerleşimi: ölçek ve merkez kaydırması.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gorunum {
    /// Görüntü pikseli başına düşen ekran pikseli (1.0 = %100).
    pub olcek: f64,
    /// Görüntü merkezinin pencere merkezine göre kaydırması, görüntü pikseli cinsinden.
    pub kaydirma_x: f64,
    pub kaydirma_y: f64,
}

impl Default for Gorunum {
    fn default() -> Self {
        Self {
            olcek: 1.0,
            kaydirma_x: 0.0,
            kaydirma_y: 0.0,
        }
    }
}

/// Sınırlar: aşırı zoom'da taşma ve sıfıra bölme hatalarını engeller.
pub const EN_AZ_OLCEK: f64 = 0.01;
pub const EN_COK_OLCEK: f64 = 64.0;

impl Gorunum {
    /// Görüntüyü pencereye tam sığdıran görünüm.
    pub fn sigdir(goruntu_w: u32, goruntu_h: u32, pencere_w: f64, pencere_h: f64) -> Self {
        let olcek = sigdirma_olcegi(goruntu_w, goruntu_h, pencere_w, pencere_h);
        Self {
            olcek,
            kaydirma_x: 0.0,
            kaydirma_y: 0.0,
        }
    }

    /// %100 ölçek, merkezlenmiş görünüm.
    pub fn gercek_boyut() -> Self {
        Self::default()
    }

    /// Görüntüyü pencere genişliğine sığdıran görünüm.
    pub fn pencere_genisligi(goruntu_w: u32, pencere_w: f64) -> Self {
        if goruntu_w == 0 || pencere_w <= 0.0 {
            return Self::default();
        }
        Self {
            olcek: (pencere_w / f64::from(goruntu_w)).clamp(EN_AZ_OLCEK, EN_COK_OLCEK),
            kaydirma_x: 0.0,
            kaydirma_y: 0.0,
        }
    }

    /// Verilen pivot noktası sabit kalacak biçimde ölçeği çarpar.
    pub fn pivot_ile_olcekle(&mut self, carpan: f64, pivot: Pivot, pencere_w: f64, pencere_h: f64) {
        if !carpan.is_finite() || carpan <= 0.0 {
            return;
        }
        let yeni = (self.olcek * carpan).clamp(EN_AZ_OLCEK, EN_COK_OLCEK);
        let gercek = yeni / self.olcek;
        if (gercek - 1.0).abs() < f64::EPSILON {
            return;
        }
        // Pivotun pencere merkezine göre konumu (ekran pikseli).
        let px = pivot.ekran_x - pencere_w / 2.0;
        let py = pivot.ekran_y - pencere_h / 2.0;
        // Pivotun görüntü uzayındaki karşılığı zoom öncesi hesaplanır.
        let gx = px / self.olcek - self.kaydirma_x;
        let gy = py / self.olcek - self.kaydirma_y;
        self.olcek = yeni;
        // Zoom sonrası aynı pivotta kalması için kaydırma düzeltilir.
        self.kaydirma_x = px / self.olcek - gx;
        self.kaydirma_y = py / self.olcek - gy;
    }

    /// Görüntüyü pencere içinde tutar; görüntü pencereden küçükse merkezler.
    pub fn sinirla(&mut self, goruntu_w: u32, goruntu_h: u32, pencere_w: f64, pencere_h: f64) {
        let gw = f64::from(goruntu_w) * self.olcek;
        let gh = f64::from(goruntu_h) * self.olcek;
        let pay_x = (pencere_w - gw) / 2.0;
        let pay_y = (pencere_h - gh) / 2.0;

        // Görüntü merkezinin pencere merkezinden sapabileceği en büyük miktar.
        let sapma_x = (gw - pencere_w).max(0.0) / 2.0;
        let sapma_y = (gh - pencere_h).max(0.0) / 2.0;
        let gt_x = (sapma_x / self.olcek).max(0.0);
        let gt_y = (sapma_y / self.olcek).max(0.0);
        let _ = (pay_x, pay_y);
        self.kaydirma_x = self.kaydirma_x.clamp(-gt_x, gt_x);
        self.kaydirma_y = self.kaydirma_y.clamp(-gt_y, gt_y);
    }

    /// Kaydırmayı görüntü pikseli cinsinden uygular.
    pub fn kaydir(&mut self, dx_ekran: f64, dy_ekran: f64) {
        if self.olcek <= 0.0 {
            return;
        }
        self.kaydirma_x += dx_ekran / self.olcek;
        self.kaydirma_y += dy_ekran / self.olcek;
    }
}

/// Görüntüyü pencereye sığdıran ölçeği hesaplar (büyütme yapmaz: üst sınır 1.0).
pub fn sigdirma_olcegi(goruntu_w: u32, goruntu_h: u32, pencere_w: f64, pencere_h: f64) -> f64 {
    if goruntu_w == 0 || goruntu_h == 0 || pencere_w <= 0.0 || pencere_h <= 0.0 {
        return 1.0;
    }
    let yatay = pencere_w / f64::from(goruntu_w);
    let dikey = pencere_h / f64::from(goruntu_h);
    yatay.min(dikey).min(1.0).clamp(EN_AZ_OLCEK, EN_COK_OLCEK)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn uc_bolge_dogru_siniflanir() {
        let w = 300.0;
        assert_eq!(bolge_belirle(0.0, w), Bolge::Sol);
        assert_eq!(bolge_belirle(99.9, w), Bolge::Sol);
        assert_eq!(bolge_belirle(100.0, w), Bolge::Orta);
        assert_eq!(bolge_belirle(199.9, w), Bolge::Orta);
        assert_eq!(bolge_belirle(200.0, w), Bolge::Sag);
        assert_eq!(bolge_belirle(299.0, w), Bolge::Sag);
    }

    #[test]
    fn gecersiz_genislik_orta_dondurur() {
        assert_eq!(bolge_belirle(10.0, 0.0), Bolge::Orta);
        assert_eq!(bolge_belirle(f64::NAN, 100.0), Bolge::Orta);
        assert_eq!(bolge_belirle(10.0, f64::INFINITY), Bolge::Orta);
    }

    #[test]
    fn sigdirma_buyutmez() {
        // Küçük görsel büyütülmez.
        assert_eq!(sigdirma_olcegi(100, 100, 800.0, 600.0), 1.0);
        // Büyük görsel sığdırılır.
        let o = sigdirma_olcegi(1600, 1200, 800.0, 600.0);
        assert!((o - 0.5).abs() < 1e-9, "ölçek 0.5 olmalı, gelen {o}");
    }

    #[test]
    fn pivot_zoom_imleci_sabit_tutar() {
        let mut g = Gorunum::default();
        let pencere_w = 800.0;
        let pencere_h = 600.0;
        let pivot = Pivot {
            ekran_x: 600.0,
            ekran_y: 150.0,
        };
        // Pivotun zoom öncesi görüntü uzayı koordinatı.
        let px = pivot.ekran_x - pencere_w / 2.0;
        let py = pivot.ekran_y - pencere_h / 2.0;
        let oncesi_x = px / g.olcek - g.kaydirma_x;
        let oncesi_y = py / g.olcek - g.kaydirma_y;

        g.pivot_ile_olcekle(2.0, pivot, pencere_w, pencere_h);

        // Zoom sonrası aynı ekran noktasına karşılık gelen görüntü koordinatı aynı olmalı.
        let sonrasi_x = px / g.olcek - g.kaydirma_x;
        let sonrasi_y = py / g.olcek - g.kaydirma_y;
        assert!((oncesi_x - sonrasi_x).abs() < 1e-9);
        assert!((oncesi_y - sonrasi_y).abs() < 1e-9);
    }

    #[test]
    fn olcek_sinirlari_asilmaz() {
        let mut g = Gorunum::default();
        let p = Pivot {
            ekran_x: 0.0,
            ekran_y: 0.0,
        };
        for _ in 0..200 {
            g.pivot_ile_olcekle(2.0, p, 800.0, 600.0);
        }
        assert!(g.olcek <= EN_COK_OLCEK);
        for _ in 0..400 {
            g.pivot_ile_olcekle(0.5, p, 800.0, 600.0);
        }
        assert!(g.olcek >= EN_AZ_OLCEK);
    }

    #[test]
    fn kucuk_goruntu_merkezde_kalir() {
        let mut g = Gorunum {
            olcek: 1.0,
            kaydirma_x: 500.0,
            kaydirma_y: 500.0,
        };
        g.sinirla(100, 100, 800.0, 600.0);
        assert_eq!(g.kaydirma_x, 0.0);
        assert_eq!(g.kaydirma_y, 0.0);
    }

    #[test]
    fn buyuk_goruntu_sinirlanir() {
        let mut g = Gorunum {
            olcek: 1.0,
            kaydirma_x: 10_000.0,
            kaydirma_y: -10_000.0,
        };
        g.sinirla(1000, 1000, 800.0, 600.0);
        assert_eq!(g.kaydirma_x, 100.0);
        assert_eq!(g.kaydirma_y, -200.0);
    }
}
