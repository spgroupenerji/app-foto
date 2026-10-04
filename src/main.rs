//! gorsel — Windows için yüksek başarımlı, GPU hızlandırmalı görsel görüntüleyici.
//!
//! Kullanım:
//!   gorsel [DOSYA]      görseli aç (dizin taranır, komşular ön yüklenir)
//!   gorsel --dogrula    pencere açmadan kendi kendini test et
//!   gorsel --surum      sürüm bilgisini yazdır
//!

// Yayın derlemesinde konsol penceresi açılmaz (GUI alt sistemi); geliştirme derlemesinde
// günlükleri görebilmek için konsol korunur.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Clippy: aşağıdaki pedantik kurallar bilinçli olarak kapalıdır.
// - `collapsible_if`: platforma özgü `#[cfg]` bloklarında iç içe `if let` zinciri,
//   tek satırda birleştirilmiş koşuldan daha okunurdur ve hata ayıklaması kolaydır.
// - `needless_return`: `#[cfg]` dalları arasında erken dönüş, bloğun son ifadesine
//   göre niyeti daha açık gösterir; üretilen kod aynıdır.
// - `field_reassign_with_default`: ayar testlerinde alan alan kurulum okunurluğu artırır.
// - `chunks_exact`/`manual_range_patterns`/`needless_range_loop`: piksel tamponlarında
//   RGBA grupları ve kanal indeksleri, `as_chunks` desteklemesinden daha okunurdur.
#![allow(
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::field_reassign_with_default,
    clippy::chunks_exact_to_as_chunks,
    clippy::manual_range_patterns,
    clippy::needless_range_loop
)]

mod cekirdek;
mod dizin;
mod dogrulama;
mod gio;
mod girdi;
mod goruntu;
mod gpu;
mod hud;
mod kabuk;
mod pencere;
mod uygulama;

use std::path::PathBuf;
use std::process::ExitCode;

use cekirdek::ayar::Ayarlar;
use uygulama::{Baslatma, Uygulama};

fn main() -> ExitCode {
    gunlugu_kur();

    let argumanlar: Vec<String> = std::env::args().skip(1).collect();

    // Yalnızca bilgi amaçlı bayraklar terminal çıktısı üretir; GUI kipinde bunlar için
    // üst sürecin konsoluna bağlanılır (aksi halde çıktı hiçbir yere gitmez).
    if argumanlar.iter().any(|a| {
        matches!(
            a.as_str(),
            "--dogrula" | "--surum" | "-v" | "--yardim" | "-h"
        )
    }) {
        konsola_baglan();
    }

    if argumanlar.iter().any(|a| a == "--dogrula") {
        return dogrulama::calistir_ve_kod();
    }
    if argumanlar.iter().any(|a| a == "--surum" || a == "-v") {
        println!(
            "gorsel {} {}",
            env!("CARGO_PKG_VERSION"),
            env!("DERLEME_SURUMU")
        );
        return ExitCode::SUCCESS;
    }
    if argumanlar.iter().any(|a| a == "--yardim" || a == "-h") {
        yardim_yazdir();
        return ExitCode::SUCCESS;
    }

    let yol = argumanlar
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from);

    // Tek örnek: çalışan bir ana süreç varsa yolu ona iletip çıkılır.
    if let Some(y) = yol.as_ref() {
        if kabuk::tekornek::yolu_ana_surece_gonder(y) {
            log::info!("yol çalışan örneğe iletildi");
            return ExitCode::SUCCESS;
        }
    }

    match baslat(yol) {
        Ok(()) => ExitCode::SUCCESS,
        Err(k) => {
            eprintln!("Görsel Görüntüleyici başlatılamadı: {k}");
            ExitCode::FAILURE
        }
    }
}

/// Ayarları yükler, tek örnek kanalını dinler ve olay döngüsünü başlatır.
/// Günlükleri hem dosyaya hem (bağlıysa) konsola yazan yazıcı.
///
/// Yayın derlemesi GUI alt sisteminde çalıştığı için konsol çıktısı kaybolur; bu yüzden
/// günlükler `%LOCALAPPDATA%\Gorsel\gorsel.log` dosyasına da yazılır. Böylece kullanıcı
/// konsol açmadan da tanılama bilgisine ulaşır.
struct IkiliYazici {
    dosya: std::fs::File,
}

impl std::io::Write for IkiliYazici {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Konsol bağlı değilse bu yazma sessizce başarısız olur; dosya yeterlidir.
        let _ = std::io::stderr().write_all(buf);
        self.dosya.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stderr().flush();
        self.dosya.flush()
    }
}

/// Günlük dosyasını açar; aşırı büyümüşse sıfırlar.
fn gunluk_dosyasi() -> Option<std::fs::File> {
    const EN_COK_BAYT: u64 = 4 * 1024 * 1024;
    let yol = cekirdek::yerel::veri_dosyasi("gorsel.log").ok()?;
    if let Some(ust) = yol.parent() {
        let _ = cekirdek::yerel::dizini_hazirla(ust);
    }
    let buyuk = std::fs::metadata(&yol)
        .map(|m| m.len() > EN_COK_BAYT)
        .unwrap_or(false);
    if buyuk {
        let _ = std::fs::remove_file(&yol);
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&yol)
        .ok()
}

/// env_logger'ı dosya + konsol hedefiyle kurar.
fn gunlugu_kur() {
    let mut kurucu =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    kurucu.format_timestamp_secs();
    if let Some(dosya) = gunluk_dosyasi() {
        kurucu.target(env_logger::Target::Pipe(Box::new(IkiliYazici { dosya })));
    }
    kurucu.init();
}

/// Yayın derlemesinde (GUI alt sistemi) üst sürecin konsoluna bağlanır.
///
/// `CONOUT$` açılıp standart tutamaçlara yerleştirilir; Rust'ın standart çıktısı ilk
/// yazımda bu tutamacı okur. Konsol yoksa (ör. Gezginden çalıştırma) sessizce döner.
///
/// Yalnızca konsolu bulunmayan GUI derlemesinde çağrılır: geliştirme derlemesinde
/// çağrılsaydı `SetStdHandle` boru/redirect hedefini ezer ve çıktı kaybolurdu.
#[cfg(all(windows, not(debug_assertions)))]
fn konsola_baglan() {
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    };
    use windows::core::PCWSTR;

    let ad: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        let Ok(tutamac) = CreateFileW(
            PCWSTR::from_raw(ad.as_ptr()),
            0x4000_0000, // GENERIC_WRITE
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        ) else {
            return;
        };
        let _ = SetStdHandle(STD_OUTPUT_HANDLE, tutamac);
        let _ = SetStdHandle(STD_ERROR_HANDLE, tutamac);
    }
}

#[cfg(not(all(windows, not(debug_assertions))))]
fn konsola_baglan() {}

fn baslat(yol: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Dosya/klasör diyalogları COM ister: tek iş parçacıklı daire (STA) açılır.
    kabuk::dosya_sec::com_hazirla();

    // Bozuk ayar dosyası uygulamayı engellemez; varsayılanlara düşülür.
    let ayarlar = match Ayarlar::yukle() {
        Ok(a) => a,
        Err(k) => {
            log::warn!("ayarlar yüklenemedi, varsayılanlar kullanılıyor: {k}");
            Ayarlar::default()
        }
    };

    let olay_dongusu = winit::event_loop::EventLoop::<PathBuf>::with_user_event().build()?;

    // İkincil süreçlerden gelen "dosya aç" istekleri olay döngüsüne aktarılır.
    let _dinleyici = match kabuk::tekornek::BoruDinleyici::baslat() {
        Ok((dinleyici, alici)) => {
            let vekil = olay_dongusu.create_proxy();
            std::thread::Builder::new()
                .name("gorsel-boru".to_string())
                .spawn(move || {
                    while let Ok(aktarilan) = alici.recv() {
                        if vekil.send_event(aktarilan).is_err() {
                            break;
                        }
                    }
                })?;
            Some(dinleyici)
        }
        Err(k) => {
            // Kanal kurulamazsa uygulama tek örnek olmadan çalışmaya devam eder.
            log::warn!("tek örnek kanalı kurulamadı: {k}");
            None
        }
    };

    let onbellek_bayt = ayarlar.ram_siniri_bayt();
    let mut uygulama = Uygulama::yeni(
        ayarlar,
        Baslatma {
            baslangic_yolu: yol,
            onbellek_bayt,
        },
    );

    olay_dongusu.run_app(&mut uygulama)?;
    Ok(())
}

/// Komut satırı yardımı.
fn yardim_yazdir() {
    // Ham dize: yol ayırıcıları ve yüzde işaretleri kaçış gerektirmez.
    let metin = r#"gorsel — Windows görsel görüntüleyici

KULLANIM:
  gorsel [DOSYA|KLASOR]   görseli veya klasörü aç
  gorsel --dogrula        kendi kendini test et (pencere açmaz)
  gorsel --surum          sürüm bilgisi
  gorsel --yardim         bu metin

KISAYOLLAR:
  Sol/Sağ ok, PageUp/PageDown  önceki/sonraki görsel
  Home/End                     ilk/son görsel
  Ctrl + O                     dosya aç
  Ctrl + Shift + O             klasör aç
  Ctrl + E                     birlikte aç
  Ctrl + tekerlek              imleç odaklı yakınlaştırma
  Sol üçte bir / sağ üçte bir  tıklama ile gezinme
  S / G                        sığdır / gerçek boyut
  F11 / F / çift tık           tam ekran
  Ctrl + K                     ayarlar
  I                            bilgi panelini aç/kapat
  L                            dizin listesini aç/kapat

NOT: Uygulama GUI alt sisteminde çalışır, konsol penceresi açılmaz.
     --dogrula raporu her durumda %LOCALAPPDATA%\Gorsel\dogrulama-raporu.txt
     dosyasına, günlükler %LOCALAPPDATA%\Gorsel\gorsel.log dosyasına yazılır."#;
    println!("{metin}");
}
