# Görsel Görüntüleyici — yeni bilgisayar kurulum ve derleme betiği.
#
# Kullanım:
#   powershell -ExecutionPolicy Bypass -File vendor/kurulum.ps1
#
# Sıra: araç denetimi → derleme (target/) → damgalı yayın kopyası (release/) → kendi kendini test.

$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust bulunamadı. https://rustup.rs adresinden rustup'ı (MSVC hedefiyle) kurup yeniden çalıştırın."
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Warning "git bulunamadı; sürüm takibi için https://git-scm.com adresinden kurun."
}

# Sürüm damgası bir kez üretilir: hem ikiliye gömülür hem yayın dosyasının adını verir
# (kural: docs/README.md — vYYYYMMDDHHMM). build.rs her derlemede yeniden çalışsın diye
# damgası tazelenir; aksi halde tam önbellekli derlemede eski damga kalır.
(Get-Item build.rs).LastWriteTime = Get-Date
$damga = Get-Date -Format yyyyMMddHHmm
$env:DERLEME_DAMGASI = $damga

cargo build --release
if ($LASTEXITCODE -ne 0) {
    throw "Derleme başarısız. Visual Studio 2022 Build Tools ve Windows SDK kurulu mu? (bkz. docs/README.md)"
}

$yayinAdi = "app-foto_v$($damga.Substring(0, 8))saat$($damga.Substring(8, 4)).exe"
New-Item -ItemType Directory -Force -Path release | Out-Null
# release/ her zaman tek taze kopya taşır: eski adlar temizlenir.
Remove-Item release/gorsel.exe, release/app-foto_v*.exe -Force -ErrorAction SilentlyContinue
Copy-Item target/release/gorsel.exe "release/$yayinAdi" -Force
Write-Host "Yayın kopyası tazelendi: release/$yayinAdi"

& ".\release\$yayinAdi" --dogrula
if ($LASTEXITCODE -eq 0) {
    Write-Host "Kurulum tamamlandı. Rapor: %LOCALAPPDATA%\Gorsel\dogrulama-raporu.txt"
} else {
    throw "Kendi kendini test başarısız oldu; raporu inceleyin."
}
