//! Sınırlı bellek bütçesiyle çalışan son kullanılan (LRU) önbelleği.
//!
//! Hem RAM'de çözülmüş görsel tamponları hem de meta veri kayıtları için kullanılır.
//! Erişim sırası `VecDeque` üzerinde tutulur; aynı anahtar birden çok kez kuyruğa
//! girebilir, tahliye sırasında yalnızca en güncel kayıt geçerli sayılır.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

struct Kayit<V> {
    deger: V,
    boyut: u64,
    son_erisim: u64,
}

/// Bayt bütçesiyle sınırlandırılmış LRU önbelleği.
pub struct Onbellek<K, V> {
    esik_bayt: u64,
    kullanilan_bayt: u64,
    sayac: u64,
    kayitlar: HashMap<K, Kayit<V>>,
    sira: VecDeque<(K, u64)>,
    boyut_olcer: fn(&V) -> u64,
}

impl<K, V> Onbellek<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Verilen bayt bütçesi ve boyut ölçer ile boş önbellek kurar.
    pub fn yeni(esik_bayt: u64, boyut_olcer: fn(&V) -> u64) -> Self {
        Self {
            esik_bayt,
            kullanilan_bayt: 0,
            sayac: 0,
            kayitlar: HashMap::new(),
            sira: VecDeque::new(),
            boyut_olcer,
        }
    }

    // Katman API'si: HUD'a bağlanması Faz 2 kapsamında.
    #[allow(dead_code)]
    pub fn esik_bayt(&self) -> u64 {
        self.esik_bayt
    }

    pub fn kullanilan_bayt(&self) -> u64 {
        self.kullanilan_bayt
    }

    pub fn adet(&self) -> usize {
        self.kayitlar.len()
    }

    pub fn icerir(&self, anahtar: &K) -> bool {
        self.kayitlar.contains_key(anahtar)
    }

    /// Kaydı okur ve erişim sırasını günceller.
    pub fn al(&mut self, anahtar: &K) -> Option<&V> {
        let sayac = self.sayac;
        match self.kayitlar.get_mut(anahtar) {
            Some(kayit) => {
                self.sayac += 1;
                kayit.son_erisim = sayac;
                self.sira.push_back((anahtar.clone(), sayac));
                Some(&kayit.deger)
            }
            None => None,
        }
    }

    /// Kaydı ekler; bütçe aşılırsa en eski kayıtlar tahliye edilir.
    pub fn koy(&mut self, anahtar: K, deger: V) {
        let boyut = (self.boyut_olcer)(&deger);
        if boyut > self.esik_bayt {
            // Tek başına bütçeyi aşan kayıt önbelleğe alınmaz.
            return;
        }
        self.sayac += 1;
        let son_erisim = self.sayac;
        if let Some(eski) = self.kayitlar.insert(
            anahtar.clone(),
            Kayit {
                deger,
                boyut,
                son_erisim,
            },
        ) {
            self.kullanilan_bayt = self.kullanilan_bayt.saturating_sub(eski.boyut);
        }
        self.kullanilan_bayt += boyut;
        self.sira.push_back((anahtar, son_erisim));
        self.tahliye_et();
    }

    /// Kaydı kaldırır ve varsa döndürür.
    pub fn kaldir(&mut self, anahtar: &K) -> Option<V> {
        match self.kayitlar.remove(anahtar) {
            Some(kayit) => {
                self.kullanilan_bayt = self.kullanilan_bayt.saturating_sub(kayit.boyut);
                Some(kayit.deger)
            }
            None => None,
        }
    }

    /// Verilen yüklem ile eşleşen tüm kayıtları tahliye eder (ör. yön değişince uzak komşular).
    pub fn suz_ve_tahliye<F: Fn(&K) -> bool>(&mut self, tut: F) -> usize {
        let atilacak: Vec<K> = self
            .kayitlar
            .keys()
            .filter(|k| !tut(k))
            .cloned()
            .collect();
        let sayi = atilacak.len();
        for anahtar in atilacak {
            self.kaldir(&anahtar);
        }
        sayi
    }

    pub fn temizle(&mut self) {
        self.kayitlar.clear();
        self.sira.clear();
        self.kullanilan_bayt = 0;
    }

    /// Bütçe aşıldığı sürece en eski kaydı tahliye eder.
    fn tahliye_et(&mut self) {
        while self.kullanilan_bayt > self.esik_bayt {
            let Some((anahtar, damga)) = self.sira.pop_front() else {
                break;
            };
            let guncel = self
                .kayitlar
                .get(&anahtar)
                .is_some_and(|k| k.son_erisim == damga);
            if guncel {
                self.kaldir(&anahtar);
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    // Önbellek, `fn(&V) -> u64` imzası bekler; V = Vec<u8> olduğu için
    // dilim yerine referans alınır.
    #[allow(clippy::ptr_arg)]
    fn bayt_boyu(v: &Vec<u8>) -> u64 {
        v.len() as u64
    }

    fn doldur(adet: usize) -> Vec<u8> {
        vec![7u8; adet]
    }

    #[test]
    fn butce_asildiginda_en_eski_tahliye_edilir() {
        let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(300, bayt_boyu);
        for i in 0..3 {
            onbellek.koy(i, doldur(100));
        }
        assert_eq!(onbellek.adet(), 3);
        assert_eq!(onbellek.kullanilan_bayt(), 300);

        onbellek.koy(3, doldur(100));
        assert_eq!(onbellek.adet(), 3, "bütçe korunmalı");
        assert!(!onbellek.icerir(&0), "en eski kayıt düşmeli");
        assert!(onbellek.icerir(&3));
    }

    #[test]
    fn erisim_sirasi_yeniler() {
        let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(300, bayt_boyu);
        for i in 0..3 {
            onbellek.koy(i, doldur(100));
        }
        // 0 numaralı kaydı tazele.
        assert!(onbellek.al(&0).is_some());
        onbellek.koy(3, doldur(100));
        assert!(onbellek.icerir(&0), "tazelenen kayıt korunmalı");
        assert!(!onbellek.icerir(&1), "sıradaki en eski düşmeli");
    }

    #[test]
    fn butceyi_asan_tek_kayit_alinmaz() {
        let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(50, bayt_boyu);
        onbellek.koy(1, doldur(100));
        assert_eq!(onbellek.adet(), 0);
        assert_eq!(onbellek.kullanilan_bayt(), 0);
    }

    #[test]
    fn ayni_anahtar_yeniden_konunca_bayt_sismaz() {
        let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(1000, bayt_boyu);
        onbellek.koy(1, doldur(100));
        onbellek.koy(1, doldur(200));
        assert_eq!(onbellek.adet(), 1);
        assert_eq!(onbellek.kullanilan_bayt(), 200);
    }

    #[test]
    fn kaldirma_butceyi_dusurur() {
        let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(1000, bayt_boyu);
        onbellek.koy(1, doldur(300));
        assert!(onbellek.kaldir(&1).is_some());
        assert_eq!(onbellek.kullanilan_bayt(), 0);
        assert!(onbellek.kaldir(&1).is_none());
    }

    #[test]
    fn suzme_istenmeyenleri_atlar() {
        let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(10_000, bayt_boyu);
        for i in 0..10 {
            onbellek.koy(i, doldur(10));
        }
        // Yalnızca 4-7 arası indeksler kalsın.
        let atilan = onbellek.suz_ve_tahliye(|k| (4..=7).contains(k));
        assert_eq!(atilan, 6);
        assert_eq!(onbellek.adet(), 4);
        assert_eq!(onbellek.kullanilan_bayt(), 40);
    }

    #[test]
    fn temizle_her_seyi_siler() {
        let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(10_000, bayt_boyu);
        for i in 0..5 {
            onbellek.koy(i, doldur(10));
        }
        onbellek.temizle();
        assert_eq!(onbellek.adet(), 0);
        assert_eq!(onbellek.kullanilan_bayt(), 0);
        assert!(onbellek.al(&0).is_none());
    }
}
