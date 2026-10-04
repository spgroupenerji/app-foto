//! Derleme zamanı kaynakları: EXE ikonu ve derleme sürüm numarası.
//!
//! Sürüm kuralı (bkz. docs/README.md): her derlemede `vYYYYMMDDHHMM` (yerel saat,
//! YılAyGünSaatDakika) üretilip `DERLEME_SURUMU` ortam değişkeniyle ikiliye gömülür;
//! `--surum` ve `--dogrula` gösterir. rerun-if yönergesi bilinçli olarak yazılmaz:
//! öntanımlı davranışla paketteki her değişiklik bu betiği yeniden çalıştırır ve
//! damga tazelenir.

fn main() {
    println!("cargo:rustc-env=DERLEME_SURUMU={}", derleme_surumu());

    // Yayın ikilisi yol damgasıyla derlenmezse panic konumları derleyici makinesinin
    // kullanıcı yolunu sızdırır; damga makine-yerel CARGO_HOME config'inden gelir
    // (kural: docs/README.md).
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        let damgali = std::env::var("RUSTFLAGS")
            .unwrap_or_default()
            .contains("remap-path-prefix")
            || std::env::var("CARGO_ENCODED_RUSTFLAGS")
                .unwrap_or_default()
                .contains("remap-path-prefix");
        if !damgali {
            println!(
                "cargo:warning=Yayın derlemesi yol damgası (remap-path-prefix) olmadan \
                 derleniyor: docs/README.md'deki yayına uygun derleme komutunu kullanın"
            );
        }

        // Yayın kopyası tazeliği: target/'taki ikili taze ama release/'te ondan yeni
        // damgalı kopya yoksa kopyalama adımı atlanmıştır (kural: docs/README.md).
        let derleme_ikilisi = std::path::Path::new("target/release/gorsel.exe");
        if !yayin_kopyasi_taze_mi(derleme_ikilisi) && derleme_ikilisi.exists() {
            println!(
                "cargo:warning=release/ altında taze app-foto_v*.exe kopyası yok: \
                 vendor/kurulum.ps1 ile yayın kopyasını yenileyin"
            );
        }
    }

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut kaynak = winresource::WindowsResource::new();
        kaynak.set_icon("assets/gorsel.ico");
        kaynak
            .compile()
            .expect("Windows ikon kaynağı derlenemedi (Windows SDK kurulu mu?)");
    }
}

/// target/'taki ikili, release/'teki en yeni `app-foto_v*.exe` kopyasından taze mi.
/// Damgalı kopya hiç yoksa `false` döner.
fn yayin_kopyasi_taze_mi(ikili: &std::path::Path) -> bool {
    let Ok(ikili_tarihi) = ikili.metadata().and_then(|m| m.modified()) else {
        return false;
    };
    std::fs::read_dir("release")
        .ok()
        .and_then(|okunan| {
            okunan
                .filter_map(|giris| giris.ok())
                .filter(|giris| {
                    giris
                        .file_name()
                        .to_string_lossy()
                        .starts_with("app-foto_v")
                })
                .filter_map(|giris| giris.metadata().ok())
                .filter_map(|meta| meta.modified().ok())
                .max()
        })
        .is_some_and(|kopya_tarihi| ikili_tarihi <= kopya_tarihi)
}

/// vYYYYMMDDHHMM biçiminde derleme damgası (yerel saat; alınamazsa UTC).
///
/// `DERLEME_DAMGASI` ortam değişkeni geçerli bir damga taşıyorsa (vendor/kurulum.ps1
/// koyar) onu kullanır; böylece yayın dosyasının adı ile ikiliye gömülen sürüm tek
/// damgadan türetilir.
fn derleme_surumu() -> String {
    if let Ok(damga) = std::env::var("DERLEME_DAMGASI") {
        if damga.len() == 12 && damga.bytes().all(|b| b.is_ascii_digit()) {
            return format!("v{damga}");
        }
    }
    let simdi =
        time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    format!(
        "v{:04}{:02}{:02}{:02}{:02}",
        simdi.year(),
        u8::from(simdi.month()),
        simdi.day(),
        simdi.hour(),
        simdi.minute()
    )
}
