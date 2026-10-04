# Dokümanlar

Projenin sürdürülebilirlik notları bu klasörde tutulur.

## Sürüm numaralandırma kuralı

Uygulamanın her derlemesi `build.rs` tarafından otomatik numaralandırılır:

- Biçim: `vYYYYMMDDHHMM` — YılAyGünSaatDakika, yerel saat (örnek: `v202610040939`).
- Derleme anında üretilir, ikiliye `DERLEME_SURUMU` olarak gömülür.
- `gorsel --surum` ve `gorsel --dogrula` bu numarayı gösterir.
- Yayın dosyasının adı aynı damgadan türetilir:
  `app-foto_v<YYYYMMDD>saat<HHmm>.exe` (örnek: `app-foto_v20261004saat0939.exe`).
- `vendor/kurulum.ps1` damgayı `DERLEME_DAMGASI` ortam değişkeniyle derlemeye verir;
  böylece dosya adı ile gömülü sürüm daima aynıdır.
- Kural hem bu dosyada hem `AGENTS.md`'de kayıtlıdır; her derleme uygular.

## Yayın akışı

1. `powershell -ExecutionPolicy Bypass -File vendor/kurulum.ps1` — damga üretir,
   derler ve `release/app-foto_v<YYYYMMDD>saat<HHmm>.exe` adıyla kopyalar; eski
   yayın kopyalarını siler.
2. Betik kendi kendini test eder (`--dogrula`) — 20 kontrolün tamamı geçmeli
   (çıkış kodu 0).
3. `release/` altındaki yeni damgalı kopya (ve eski kopyanın silinmesi) commit'lenir.

`cargo build --release` çıktısı `target/release/gorsel.exe` ara üründür; repoya
girmez. Takip edilen yayın kopyası yalnızca damgalı adla `release/` altında durur.

**Yeni bilgisayarda** `git clone` sonrası tek komut: `powershell -ExecutionPolicy
Bypass -File vendor/kurulum.ps1` — araçları denetler, derler, damgalı kopyayı üretir
ve doğrular. Bağımlılıklar `Cargo.lock` sürümleriyle ilk derlemede otomatik iner.

Derleme çıktıları yerel `target/` altındadır; `cargo clean` yalnızca `target/` siler.
`build.rs`, target/'taki ikili taze olduğu hâlde `release/` altında ondan yeni bir
`app-foto_v*.exe` kopyası yoksa uyarır.

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
