# SP GROUP Görsel Görüntüleyici

egui arayüz çerçevesi ve wgpu grafik altyapısıyla geliştirilen, çok iş parçacıklı ve GPU hızlandırmalı mimariye sahip Windows görsel görüntüleyicisi.

## Özellikler

- 17 görüntü biçimi ailesinin tanınması ve 45 dosya uzantısının desteklenmesi; JPEG, PNG, WebP, GIF, TIFF, BMP, JPEG XL, SVG, OpenEXR, Radiance HDR, QOI, DDS, PNM ve kamera RAW biçimlerinin (CR2, CR3, NEF, ARW, DNG vb.) çözülmesi
- Biçim tespitinin dosya içeriği imzasıyla (magic bytes) yapılması; uzantı bilgisine güvenilmemesi
- wgpu tabanlı GPU hızlandırmalı render; HDR ekranlarda HDR çıkışı ve SDR ekranlara ton haritalama
- Gömülü ICC renk profillerinin lcms2 ile dönüştürülmesi; EXIF yönelim bilgisinin uygulanması
- Catmull-Rom filtreyle keskin ölçekleme; Ctrl+tekerlek ile imleç odaklı akıcı yakınlaştırma, sığdırma ve yüzde yüz gerçek boyut
- Kayan pencere ile komşu görsellerin ön yüklenmesi; ayarlanabilir RAM önbelleği (64 MB–8 GB) ve VRAM komşu doku desteğiyle akıcı gezinme
- Ok tuşları, PageUp/PageDown, Home/End ve tıklama bölgeleriyle hızlı gezinme; sürükle-bırak ile dosya ve klasör açma
- Windows Gezgini ile aynı doğal sıralama; dizin değişikliklerinin canlı izlenmesi; tek örnek (single instance) davranışı
- Meta veri paneli (biçim, çözünürlük, renk profili, önbellek ve aygıt bilgisi), ayarlar penceresi, bağlam menüsü ve tam ekran kipi
- Kurumsal koyu tema; WCAG 2.5.5 ve Apple HIG ile uyumlu 44 px dokunma hedefleri

## Hazır Uygulama

`release` klasöründeki `app-foto_v<YYYYMMDD>saat<HHmm>.exe` dosyası kurulum gerektirmez ve doğrudan çalıştırılır. Klasörde yalnızca en güncel sürüm bulundurulur; her derlemede önceki sürümler otomatik olarak kaldırılır. Dosya adındaki damga, derleme anını gösterir ve uygulama içindeki sürüm bilgisiyle aynıdır.

## Kaynaktan Derleme

Gereksinimler: Rust (MSVC hedefi) ve Visual Studio 2022 C++ Build Tools (Windows SDK ile birlikte).

```powershell
powershell -ExecutionPolicy Bypass -File vendor\kurulum.ps1
```

Betik ön koşulları denetler, sürüm damgasını üretir, uygulamayı derler, çıktıyı `release\app-foto_v<YYYYMMDD>saat<HHmm>.exe` adıyla kopyalar, önceki kopyaları kaldırır ve 20 kontrolü kapsayan kendi kendini test çalıştırır. Bağımlılıklar ilk derlemede `Cargo.lock` sürümleriyle otomatik iner; sonraki derlemeler artımlıdır.

Elle derlemede (`cargo build --release`) ikili ara ürün olarak `target\release\gorsel.exe` altında üretilir; depoya girmez. Takip edilen yayın kopyası yalnızca damgalı adla `release\` altında oluşturulur.

### Yayına uygun derleme (yol damgası)

Yayın ikilisi, panic konumlarında derleme makinesinin kullanıcı yolunu sızdırmamak için `--remap-path-prefix` damgasıyla derlenir. Damga makine-yerel `%USERPROFILE%\.cargo\config.toml` içinde tanımlanır (repoya girmez):

```toml
[build]
rustflags = [
    "--remap-path-prefix=C:\\Users\\<KULLANICI>=/derleme",
    "--remap-path-prefix=C:\\<PROJE-KOKU>=gorsel",
]
```

`build.rs`, damgasız yayın derlemesinde uyarı üretir; damgasız ikili repoya commit'lenmez.

## Geliştirici Kurulumu (Yeni Bilgisayar)

```powershell
git clone https://github.com/spgroupenerji/app-foto.git
cd app-foto
powershell -ExecutionPolicy Bypass -File vendor\kurulum.ps1
```

Betik ön koşulları denetler, bağımlılıkları indirir ve uygulamayı damgalı yayın kopyasıyla birlikte üretir. Cargo derleme önbelleği (`target/`) makineye ve yola özgüdür; depoya dahil edilmez ve kopyalanamaz.

## Güvenlik

- Uygulama yalnızca kullanıcı tarafından açılan yerel dosyaları işler; arka planda ağ isteği yapmaz, telemetri toplamaz ve kullanıcı verisi iletmez. Kod tabanında hiçbir ağ istemcisi bağımlılığı bulunmaz.
- Ağ paylaşımından (UNC yolu) açılan dosyalarda kullanıcı, performans etkisi hakkında önceden bilgilendirilir.
- Biçim tespiti dosya içeriği imzasıyla yapılır; bozuk, eksik veya aşırı boyutlu dosyalar kullanıcıya anlaşılır hata mesajlarıyla karşılanır, uygulama çalışmaya devam eder.
- Görüntü çözme ve önbellekleme işlemleri, arayüz iş parçacığından bağımsız bir işçi havuzunda yürütülür; sorunlu dosyalar arayüzü kilitlemez.
- Yayın ikilisinde panic konumlarındaki makine yolları `--remap-path-prefix` damgasıyla maskelenir (bkz. *Yayına uygun derleme*).
- Ayarlar ve pencere yerleşimi `%APPDATA%\Gorsel`, günlük ile meta veri önbelleği `%LOCALAPPDATA%\Gorsel` altında, yalnızca yerel kullanıcı dizininde saklanır.
- Dosya ilişkilendirmesi yalnızca kullanıcı hesabı kapsamındaki kayıt defterine (HKCU) yazılır; yönetici hakları gerektirmez.
- Günlük dosyası yalnızca teknik tanılama bilgisi içerir; kimlik bilgisi veya gizli içerik yazılmaz.

## Sürümlandırma

Her derleme, derleme anını gösteren `vYYYYMMDDHHMM` (yıl-ay-gün-saat-dakika, yerel saat) damgasını otomatik alır. Damga `vendor/kurulum.ps1` tarafından tek kez üretilir; hem ikiliye `DERLEME_SURUMU` olarak gömülür hem de yayın dosyasının adını oluşturur, böylece dosya adı ile uygulama sürümü daima aynıdır. Sürüm bilgisi `gorsel --surum` komutuyla ve kendi kendini test raporunda görüntülenir.

## Depo Yapısı

| Dizin | İçerik |
| --- | --- |
| `src/` | Uygulama kaynak kodu (çekirdek, görüntü, GPU, arayüz ve kabuk katmanları) |
| `assets/` | Uygulama ikonu |
| `vendor/` | Derleme ve kurulum betiği (`kurulum.ps1`) |
| `docs/` | Belgeler |
| `release/` | Güncel sürümlü uygulama çalıştırıcısı |

## Lisans

Uygulama bağımlılıklarının lisans bilgileri `Cargo.toml` ve `Cargo.lock` kayıtları üzerinden izlenebilir; yeniden dağıtım sırasında ilgili lisans yükümlülükleri dikkate alınmalıdır.

---

© 2026 SP GROUP ENERJİ SAN. VE Tİ. LTD. ŞTİ. — info@spgroupenerji.com
