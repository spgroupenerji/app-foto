# Dokümanlar

Projenin sürdürülebilirlik notları bu klasörde tutulur.

## Sürüm numaralandırma kuralı

Uygulamanın her derlemesi `build.rs` tarafından otomatik numaralandırılır:

- Biçim: `vYYYYMMDDHHMM` — YılAyGünSaatDakika, yerel saat (örnek: `v202610040939`).
- Derleme anında üretilir, ikiliye `DERLEME_SURUMU` olarak gömülür.
- `gorsel --surum` ve `gorsel --dogrula` bu numarayı gösterir.
- Kural hem bu dosyada hem `AGENTS.md`'de kayıtlıdır; her derleme uygular.

## Yayın akışı

1. `cargo build --release` → `target/release/gorsel.exe`
2. Kopyala: `cp target/release/gorsel.exe release/gorsel.exe` — yayın kopyası takiptedir
3. `cargo run -- --dogrula` — 20 kontrolün tamamı geçmeli (çıkış kodu 0)
4. Değişen `release/gorsel.exe` commit'lenir.

**Yeni bilgisayarda** `git clone` sonrası tek komut: `powershell -ExecutionPolicy
Bypass -File kurulum.ps1` — araçları denetler, derler, kopyalar ve doğrular.
Bağımlılıklar `Cargo.lock` sürümleriyle ilk derlemede otomatik iner.

Derleme çıktıları yerel `target/` altındadır; `cargo clean` yalnızca `target/` siler.
`build.rs`, taze ikiliye rağmen eski/eksik `release/gorsel.exe` konusunda uyarır.

## Yayına uygun derleme (yol damgası)

Yayın ikilisi, panic konumlarında derleyici makinesinin kullanıcı yolunu sızdırmamak
için `--remap-path-prefix` damgasıyla derlenir. Damga makine-yerel olarak
`%USERPROFILE%\.cargo\config.toml` içinde tanımlanır (repoya girmez); kendi kök yolunuzu
yer tutucu yerine yazın:

```toml
[build]
rustflags = [
    "--remap-path-prefix=C:\\Users\\<KULLANICI>=/derleme",
    "--remap-path-prefix=C:\\<PROJE-KOKU>=gorsel",
]
```

`build.rs`, damgasız yayın derlemesinde uyarı üretir; damgasız ikili repoya commit'lenmez.
