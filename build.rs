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
        let damgali = std::env::var("RUSTFLAGS").unwrap_or_default()
            .contains("remap-path-prefix")
            || std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default()
                .contains("remap-path-prefix");
        if !damgali {
            println!(
                "cargo:warning=Yayın derlemesi yol damgası (remap-path-prefix) olmadan \
                 derleniyor: docs/README.md'deki yayına uygun derleme komutunu kullanın"
            );
        }

        // Yayın kopyası tazeliği: target/'taki ikili, release/ kopyasından yeniyse
        // kopyalama adımı atlanmıştır (kural: docs/README.md).
        let yayin = std::path::Path::new("release/gorsel.exe");
        let derleme_ikilisi = std::path::Path::new("target/release/gorsel.exe");
        let kopya_taze = yayin
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                derleme_ikilisi
                    .metadata()
                    .and_then(|y| y.modified())
                    .ok()
                    .map(|y| y <= t)
            })
            .unwrap_or(false);
        if !kopya_taze && derleme_ikilisi.exists() {
            println!(
                "cargo:warning=release/gorsel.exe eski veya yok: \
                 target/release/gorsel.exe'den kopyalayın"
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

/// vYYYYMMDDHHMM biçiminde derleme damgası (yerel saat; alınamazsa UTC).
fn derleme_surumu() -> String {
    let simdi = time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    format!(
        "v{:04}{:02}{:02}{:02}{:02}",
        simdi.year(),
        u8::from(simdi.month()),
        simdi.day(),
        simdi.hour(),
        simdi.minute()
    )
}
