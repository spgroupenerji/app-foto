# Görsel Görüntüleyici — yeni bilgisayar kurulum ve derleme betiği.
#
# Kullanım:
#   powershell -ExecutionPolicy Bypass -File kurulum.ps1
#
# Sıra: araç denetimi → derleme (target/) → yayın kopyası (release/) → kendi kendini test.

$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust bulunamadı. https://rustup.rs adresinden rustup'ı (MSVC hedefiyle) kurup yeniden çalıştırın."
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Warning "git bulunamadı; sürüm takibi için https://git-scm.com adresinden kurun."
}

cargo build --release
if ($LASTEXITCODE -ne 0) {
    throw "Derleme başarısız. Visual Studio 2022 Build Tools ve Windows SDK kurulu mu? (bkz. docs/README.md)"
}

New-Item -ItemType Directory -Force -Path release | Out-Null
Copy-Item target/release/gorsel.exe release/gorsel.exe -Force
Write-Host "Yayın kopyası tazelendi: release/gorsel.exe"

& .\release\gorsel.exe --dogrula
if ($LASTEXITCODE -eq 0) {
    Write-Host "Kurulum tamamlandı. Rapor: %LOCALAPPDATA%\Gorsel\dogrulama-raporu.txt"
} else {
    throw "Kendi kendini test başarısız oldu; raporu inceleyin."
}
