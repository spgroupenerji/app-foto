//! Kayan pencere ön yükleme: aktif görselin komşuları, gezinme yönüne öncelik verilerek
//! arka planda çözülür ve RAM önbelleğine konur.
//!
//! Amaç, ağ paylaşımlarında bile kullanıcı bir sonraki görsele geçtiğinde bekleme
//! yaşamamasıdır: çözümleme sırası "önce gidilen yön" mantığıyla belirlenir.

use crate::cekirdek::ayar::OnYuklemeGenisligi;

/// Gezinme yönü; ön yükleme önceliğini belirler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yon {
    Ileri,
    Geri,
    Yok,
}

/// İki konum arasındaki geçişten yönü çıkarır.
pub fn yon_belirle(onceki: usize, yeni: usize) -> Yon {
    match yeni.cmp(&onceki) {
        std::cmp::Ordering::Greater => Yon::Ileri,
        std::cmp::Ordering::Less => Yon::Geri,
        std::cmp::Ordering::Equal => Yon::Yok,
    }
}

/// Ön yüklenecek indeksleri üretir.
///
/// Sıra: önce gidilen yöndeki komşular (yakından uzağa), sonra diğer yön.
/// Dizin sınırları dışına çıkılmaz ve aktif görsel listeye dahil edilmez.
pub fn plan(konum: usize, adet: usize, genislik: OnYuklemeGenisligi, yon: Yon) -> Vec<usize> {
    let komsu = genislik.komsu_sayisi();
    if komsu == 0 || adet == 0 || konum >= adet {
        return Vec::new();
    }
    let (onceki_yon, sonraki_yon) = match yon {
        Yon::Geri => (Geri::Once, Geri::Sonra),
        Yon::Ileri | Yon::Yok => (Geri::Sonra, Geri::Once),
    };
    let mut plan = Vec::with_capacity(komsu * 2);
    plan.extend(yondeki(konum, adet, komsu, onceki_yon));
    plan.extend(yondeki(konum, adet, komsu, sonraki_yon));
    plan
}

/// Yön seçimi (aynı mantığı iki kez yazmamak için).
#[derive(Clone, Copy)]
enum Geri {
    Once,
    Sonra,
}

fn yondeki(konum: usize, adet: usize, komsu: usize, yon: Geri) -> Vec<usize> {
    let mut liste = Vec::with_capacity(komsu);
    for adim in 1..=komsu {
        let hedef = match yon {
            Geri::Once => konum.checked_sub(adim),
            Geri::Sonra => konum.checked_add(adim).filter(|i| *i < adet),
        };
        if let Some(i) = hedef {
            if i < adet {
                liste.push(i);
            }
        }
    }
    liste
}

/// Bellekte (RAM önbelleği ve VRAM) tutulması gereken indeksler: aktif görsel ve komşuları.
pub fn tutulacak_indeksler(konum: usize, adet: usize, genislik: OnYuklemeGenisligi) -> Vec<usize> {
    let mut liste = vec![konum];
    liste.extend(plan(konum, adet, genislik, Yon::Yok));
    liste.retain(|i| *i < adet);
    liste.dedup();
    liste.sort_unstable();
    liste
}

// Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
#[allow(dead_code)]
/// Ön yükleme planındaki indekslerin kaçının hâlâ çözülmesi gerektiğini sayar.
pub fn eksik_sayisi(plan: &[usize], hazir: impl Fn(usize) -> bool) -> usize {
    plan.iter().filter(|i| !hazir(**i)).count()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn yon_dogru_cikarilir() {
        assert_eq!(yon_belirle(3, 4), Yon::Ileri);
        assert_eq!(yon_belirle(4, 3), Yon::Geri);
        assert_eq!(yon_belirle(3, 3), Yon::Yok);
    }

    #[test]
    fn ileri_yonde_once_sonraki_komsular() {
        let p = plan(5, 20, OnYuklemeGenisligi::Normal, Yon::Ileri);
        assert_eq!(p, vec![6, 7, 4, 3]);
    }

    #[test]
    fn geri_yonde_once_onceki_komsular() {
        let p = plan(5, 20, OnYuklemeGenisligi::Normal, Yon::Geri);
        assert_eq!(p, vec![4, 3, 6, 7]);
    }

    #[test]
    fn yon_yoksa_ileri_oncelikli() {
        let p = plan(5, 20, OnYuklemeGenisligi::Dar, Yon::Yok);
        assert_eq!(p, vec![6, 4]);
    }

    #[test]
    fn kapali_genislik_plan_uretmez() {
        assert!(plan(5, 20, OnYuklemeGenisligi::Kapali, Yon::Ileri).is_empty());
    }

    #[test]
    fn sinirlarda_tasma_olmaz() {
        // İlk görselde geri yön boş, yalnızca ileri komşular.
        let bas = plan(0, 10, OnYuklemeGenisligi::Normal, Yon::Geri);
        assert_eq!(bas, vec![1, 2]);
        // Son görselde ileri yön boş.
        let son = plan(9, 10, OnYuklemeGenisligi::Normal, Yon::Ileri);
        assert_eq!(son, vec![8, 7]);
    }

    #[test]
    fn tek_elemanli_listede_plan_bos() {
        assert!(plan(0, 1, OnYuklemeGenisligi::Genis, Yon::Ileri).is_empty());
        assert!(plan(0, 0, OnYuklemeGenisligi::Genis, Yon::Ileri).is_empty());
    }

    #[test]
    fn genis_ayar_daha_cok_komsu_ister() {
        let dar = plan(10, 50, OnYuklemeGenisligi::Dar, Yon::Ileri);
        let genis = plan(10, 50, OnYuklemeGenisligi::Genis, Yon::Ileri);
        assert_eq!(dar.len(), 2);
        assert_eq!(genis.len(), 6);
        assert!(genis.contains(&11) && genis.contains(&13));
    }

    #[test]
    fn tutulacaklar_aktifi_ve_komsulari_icerir() {
        let t = tutulacak_indeksler(5, 20, OnYuklemeGenisligi::Dar);
        assert!(t.contains(&5));
        assert!(t.contains(&6));
        assert!(t.contains(&4));
        assert_eq!(t.len(), 3);
        // Sıralı ve tekrarsız.
        let mut sirali = t.clone();
        sirali.sort_unstable();
        assert_eq!(t, sirali);
    }

    #[test]
    fn tutulacaklar_sinirda_dogru() {
        let t = tutulacak_indeksler(0, 3, OnYuklemeGenisligi::Normal);
        assert_eq!(t, vec![0, 1, 2]);
    }

    #[test]
    fn eksik_sayisi_dogru() {
        let p = vec![1, 2, 3];
        assert_eq!(eksik_sayisi(&p, |_| false), 3);
        assert_eq!(eksik_sayisi(&p, |i| i == 2), 2);
        assert_eq!(eksik_sayisi(&p, |_| true), 0);
    }
}
