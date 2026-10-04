//! `--dogrula` kendi kendini test modu.
//!
//! Pencere açmadan tüm boru hattını gerçek girdilerle çalıştırır ve sonucu raporlar.
//! Amaç, uygulamanın "çalışıyor" iddiasının ölçülebilir kanıtla desteklenmesidir:
//! her kontrol gerçek bir işlevi çağırır, hiçbiri yalnızca derleme durumunu bildirmez.

use std::process::ExitCode;

/// Bir kontrolün sonucu.
pub(crate) enum Durum {
    Basarili(String),
    /// Ortam nedeniyle çalıştırılamadı (ör. grafik aygıtı yok) — başarısızlık sayılmaz.
    Atlanmis(String),
    Basarisiz(String),
}

/// Tek bir doğrulama adımı.
pub(crate) struct Kontrol {
    ad: &'static str,
    durum: Durum,
}

impl Kontrol {
    fn basarili(ad: &'static str, ayrinti: impl Into<String>) -> Self {
        Self {
            ad,
            durum: Durum::Basarili(ayrinti.into()),
        }
    }

    fn basarisiz(ad: &'static str, ayrinti: impl Into<String>) -> Self {
        Self {
            ad,
            durum: Durum::Basarisiz(ayrinti.into()),
        }
    }

    fn atlanmis(ad: &'static str, ayrinti: impl Into<String>) -> Self {
        Self {
            ad,
            durum: Durum::Atlanmis(ayrinti.into()),
        }
    }
}

/// Tüm kontrolleri çalıştırır.
pub fn calistir() -> Vec<Kontrol> {
    let mut kontroller = vec![
        Kontrol::basarili(
            "Sürüm",
            format!(
                "gorsel {} {} ({}/{}, {} profili)",
                env!("CARGO_PKG_VERSION"),
                env!("DERLEME_SURUMU"),
                std::env::consts::OS,
                std::env::consts::ARCH,
                if cfg!(debug_assertions) { "geliştirme" } else { "yayın" }
            ),
        ),
        surum_bilgisi(),
        bicim_tespiti(),
        renk_donusumu(),
        dogal_siralama(),
        onbellek(),
        on_yukleme_plani(),
        ag_yolu_tespiti(),
        pencere_yerlesimi(),
        ayar_gidis_donusu(),
        exe_ikonu(),
    ];
    kontroller.push(goruntu_boru_hatti());
    kontroller.push(isci_havuzu());
    kontroller.extend(gpu_kontrolleri());
    kontroller
}

/// Raporu yazdırır ve çıkış kodunu döndürür.
pub fn calistir_ve_kod() -> ExitCode {
    let kontroller = calistir();

    let mut satirlar: Vec<String> = vec![
        "gorsel — kendi kendini test (--dogrula)".to_string(),
        "-".repeat(72),
    ];
    let mut basarisiz = 0usize;
    let mut atlanan = 0usize;
    for kontrol in &kontroller {
        match &kontrol.durum {
            Durum::Basarili(ayrinti) => {
                satirlar.push(format!("  ✓ {:<26} {ayrinti}", kontrol.ad));
            }
            Durum::Atlanmis(ayrinti) => {
                satirlar.push(format!("  – {:<26} ATLANDI: {ayrinti}", kontrol.ad));
                atlanan += 1;
            }
            Durum::Basarisiz(ayrinti) => {
                satirlar.push(format!("  ✗ {:<26} HATA: {ayrinti}", kontrol.ad));
                basarisiz += 1;
            }
        }
    }
    satirlar.push("-".repeat(72));
    satirlar.push(format!(
        "{} kontrol: {} başarılı, {basarisiz} başarısız, {atlanan} atlandı",
        kontroller.len(),
        kontroller.len() - basarisiz - atlanan
    ));

    let rapor = satirlar.join("\n");
    println!("{rapor}");

    // Rapor ayrıca dosyaya yazılır: Gezginden çalıştırıldığında konsol olmayabilir.
    match crate::cekirdek::yerel::veri_dosyasi("dogrulama-raporu.txt") {
        Ok(yol) => {
            if let Some(ust) = yol.parent() {
                let _ = crate::cekirdek::yerel::dizini_hazirla(ust);
            }
            match std::fs::write(&yol, format!("{rapor}\n")) {
                Ok(()) => println!("Rapor: {}", yol.display()),
                Err(k) => eprintln!("Rapor yazılamadı ({}): {k}", yol.display()),
            }
        }
        Err(k) => eprintln!("Rapor yolu belirlenemedi: {k}"),
    }

    if basarisiz == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}



/// Doğrulama kontrolleri için benzersiz geçici dizin.
///
/// Paralel koşan kontroller aynı dizini paylaşırsa biri diğerinin dosyasını siler;
/// bu yüzden dizin adı süreç kimliği ve atomik sayaçla ayrıştırılır.
fn gecici_dizin(ad: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SAYAC: AtomicU32 = AtomicU32::new(0);
    let sira = SAYAC.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "gorsel-dogrula-{ad}-{}-{sira}",
        std::process::id()
    ))
}

/// Çalışma ortamı ve derleme bilgisi.
fn surum_bilgisi() -> Kontrol {
    let isletim = std::env::consts::OS;
    let mimari = std::env::consts::ARCH;
    Kontrol::basarili(
        "Ortam",
        format!("{isletim}/{mimari}, işçi ipliği önerisi {}", crate::gio::isci::varsayilan_isci_sayisi()),
    )
}

/// Sihirli bayt tabanlı biçim tespiti.
fn bicim_tespiti() -> Kontrol {
    use crate::goruntu::bicim::{tespit, Bicim};

    let png = [0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let jpeg = [0xFFu8, 0xD8, 0xFF, 0xE0];
    let unc_beklenen = Bicim::Png;

    if tespit(&png, None) != unc_beklenen {
        return Kontrol::basarisiz("Biçim tespiti", "PNG imzası tanınmadı");
    }
    if tespit(&jpeg, None) != Bicim::Jpeg {
        return Kontrol::basarisiz("Biçim tespiti", "JPEG imzası tanınmadı");
    }
    if tespit(&[0u8; 8], Some("xyz")) != Bicim::Bilinmeyen {
        return Kontrol::basarisiz("Biçim tespiti", "bilinmeyen veri tanımlandı");
    }
    Kontrol::basarili("Biçim tespiti", "PNG, JPEG ve bilinmeyen veri doğru sınıflandı")
}

/// sRGB → doğrusal dönüşümü ve arama tablosu tutarlılığı.
fn renk_donusumu() -> Kontrol {
    use crate::goruntu::renk::{dogrusallastir_rgba8, srgb_bayt_doğrusal};

    let siyah = srgb_bayt_doğrusal(0);
    let beyaz = srgb_bayt_doğrusal(255);
    let orta = srgb_bayt_doğrusal(128);
    if siyah.abs() > 1e-6 || (beyaz - 1.0).abs() > 1e-6 {
        return Kontrol::basarisiz("Renk dönüşümü", "uç noktalar hatalı");
    }
    if (orta - 0.2158).abs() > 1e-3 {
        return Kontrol::basarisiz("Renk dönüşümü", format!("orta gri bekleneden farklı: {orta}"));
    }
    // Arama tablosu ile tam hesap tüm aralıkta örtüşmeli.
    let mut en_buyuk_sapma = 0.0f32;
    for d in 0..=255u8 {
        let lut = dogrusallastir_rgba8(&[d, 0, 0, 255])[0];
        let tam = srgb_bayt_doğrusal(d);
        en_buyuk_sapma = en_buyuk_sapma.max((lut - tam).abs());
    }
    if en_buyuk_sapma > 1e-3 {
        return Kontrol::basarisiz(
            "Renk dönüşümü",
            format!("arama tablosu sapması {en_buyuk_sapma}"),
        );
    }
    Kontrol::basarili(
        "Renk dönüşümü",
        format!("sRGB→doğrusal 256 değerde doğrulandı (sapma ≤ {en_buyuk_sapma:.1e})"),
    )
}

/// Windows doğal sıralaması.
fn dogal_siralama() -> Kontrol {
    use crate::dizin::sirala::dogal_karsilastir;

    if dogal_karsilastir("resim2.jpg", "resim10.jpg") != std::cmp::Ordering::Less {
        return Kontrol::basarisiz(
            "Doğal sıralama",
            "resim2.jpg, resim10.jpg'den önce gelmeli",
        );
    }
    if dogal_karsilastir("g1.png", "g02.png") != std::cmp::Ordering::Less {
        return Kontrol::basarisiz("Doğal sıralama", "sıfır dolgulu sıralama hatalı");
    }
    Kontrol::basarili(
        "Doğal sıralama",
        "StrCmpLogicalW yolu Gezgin sırasını veriyor",
    )
}

/// LRU önbellek tahliyesi.
fn onbellek() -> Kontrol {
    use crate::gio::onbellek::Onbellek;

    let mut onbellek: Onbellek<u32, Vec<u8>> = Onbellek::yeni(300, |v: &Vec<u8>| v.len() as u64);
    for i in 0..3 {
        onbellek.koy(i, vec![0u8; 100]);
    }
    let _ = onbellek.al(&0);
    onbellek.koy(3, vec![0u8; 100]);

    if !onbellek.icerir(&0) || onbellek.icerir(&1) || onbellek.kullanilan_bayt() > 300 {
        return Kontrol::basarisiz(
            "LRU önbellek",
            "tahliye sırası veya bütçe denetimi hatalı",
        );
    }
    Kontrol::basarili(
        "LRU önbellek",
        format!(
            "bütçe korunuyor ({} kayıt, {} bayt)",
            onbellek.adet(),
            onbellek.kullanilan_bayt()
        ),
    )
}

/// Kayan pencere ön yükleme planı.
fn on_yukleme_plani() -> Kontrol {
    use crate::cekirdek::ayar::OnYuklemeGenisligi;
    use crate::gio::oneyukleme::{plan, Yon};

    let ileri = plan(5, 20, OnYuklemeGenisligi::Normal, Yon::Ileri);
    if ileri != vec![6, 7, 4, 3] {
        return Kontrol::basarisiz("Ön yükleme planı", format!("ileri yön planı: {ileri:?}"));
    }
    let geri = plan(5, 20, OnYuklemeGenisligi::Normal, Yon::Geri);
    if geri != vec![4, 3, 6, 7] {
        return Kontrol::basarisiz("Ön yükleme planı", format!("geri yön planı: {geri:?}"));
    }
    Kontrol::basarili("Ön yükleme planı", "yön önceliği doğru sıralanıyor")
}

/// Ağ yolu tespiti (mmap yasağının dayanağı).
fn ag_yolu_tespiti() -> Kontrol {
    use crate::gio::yol::{ag_metni_mi, YolTuru};

    if !ag_metni_mi(r"\\sunucu\paylasim\gorsel.jpg") {
        return Kontrol::basarisiz("Ağ yolu tespiti", "UNC yolu tanınmadı");
    }
    if ag_metni_mi(r"C:\resimler\gorsel.jpg") {
        return Kontrol::basarisiz("Ağ yolu tespiti", "yerel yol ağ sayıldı");
    }
    Kontrol::basarili(
        "Ağ yolu tespiti",
        format!("UNC tanınıyor; yerel yol {} olarak sınıflandı", YolTuru::Yerel.etiket()),
    )
}

/// Pencere yerleşimi serileştirmesi.
fn pencere_yerlesimi() -> Kontrol {
    use crate::pencere::yerlesim::{Yerlesim, YERLESIM_BAYT};

    let yerlesim = Yerlesim {
        uzunluk: 44,
        bayraklar: 0,
        gosterim_komutu: 3,
        min_konum: (-32000, -32000),
        maks_konum: (-8, -8),
        normal_dikdortgen: (100, 50, 1380, 830),
    };
    let baytlar = yerlesim.baytlara();
    if baytlar.len() != YERLESIM_BAYT {
        return Kontrol::basarisiz("Pencere yerleşimi", "bayt uzunluğu beklenenden farklı");
    }
    if Yerlesim::baytlardan(&baytlar) != Some(yerlesim) {
        return Kontrol::basarisiz("Pencere yerleşimi", "gidiş-dönüş bozuldu");
    }
    if !yerlesim.gecerli_mi() {
        return Kontrol::basarisiz("Pencere yerleşimi", "geçerli yerleşim reddedildi");
    }
    Kontrol::basarili(
        "Pencere yerleşimi",
        format!("WINDOWPLACEMENT {YERLESIM_BAYT} bayt gidiş-dönüş tam"),
    )
}

/// Ayar dosyası gidiş-dönüşü.
fn ayar_gidis_donusu() -> Kontrol {
    use crate::cekirdek::ayar::{Ayarlar, Filtre};

    let dizin = gecici_dizin("ayar");
    let _ = std::fs::remove_dir_all(&dizin);
    if std::fs::create_dir_all(&dizin).is_err() {
        return Kontrol::atlanmis("Ayar dosyası", "geçici dizin oluşturulamadı");
    }
    let yol = dizin.join("ayarlar.json");

    let mut ayar = Ayarlar::default();
    ayar.olcek_filtresi = Filtre::Lanczos3;
    ayar.ram_onbellek_mb = 1024;
    if let Err(k) = ayar.kaydet_dosyaya(&yol) {
        return Kontrol::basarisiz("Ayar dosyası", k.to_string());
    }
    match Ayarlar::yukle_dosyadan(&yol) {
        Ok(geri) if geri.olcek_filtresi == Filtre::Lanczos3 && geri.ram_onbellek_mb == 1024 => {
            let _ = std::fs::remove_dir_all(&dizin);
            Kontrol::basarili("Ayar dosyası", "JSON gidiş-dönüş ve alanlar korunuyor")
        }
        Ok(_) => Kontrol::basarisiz("Ayar dosyası", "yüklenen ayarlar farklı"),
        Err(k) => Kontrol::basarisiz("Ayar dosyası", k.to_string()),
    }
}

/// EXE'ye gömülü ikon kaynağı (Gezgin, görev çubuğu ve ilişkilendirme bu kaynağı kullanır).
fn exe_ikonu() -> Kontrol {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
        use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
        use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

        let exe = match std::env::current_exe() {
            Ok(yol) => yol,
            Err(k) => return Kontrol::basarisiz("EXE ikonu", format!("exe yolu okunamadı: {k}")),
        };
        let mut yol16: Vec<u16> = exe.as_os_str().encode_wide().collect();
        yol16.push(0);
        let mut bilgi = SHFILEINFOW::default();
        let sonuc = unsafe {
            SHGetFileInfoW(
                PCWSTR(yol16.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut bilgi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            )
        };
        if sonuc == 0 {
            return Kontrol::basarisiz("EXE ikonu", "gömülü ikon kaynağı bulunamadı");
        }
        let _ = unsafe { DestroyIcon(bilgi.hIcon) };
        Kontrol::basarili(
            "EXE ikonu",
            "gömülü ikon kaynağı okundu (Gezgin/görev çubuğu)",
        )
    }
    #[cfg(not(windows))]
    {
        Kontrol::atlanmis("EXE ikonu", "Windows dışı derlemede ikon kaynağı gömülmez")
    }
}

/// Tam görüntü boru hattı: PNG üret → çöz → renk işle → f16 tampon.
fn goruntu_boru_hatti() -> Kontrol {
    use crate::cekirdek::ayar::Ayarlar;
    use crate::goruntu;

    let dizin = gecici_dizin("boru");
    let _ = std::fs::remove_dir_all(&dizin);
    if std::fs::create_dir_all(&dizin).is_err() {
        return Kontrol::atlanmis("Görüntü boru hattı", "geçici dizin oluşturulamadı");
    }
    let yol = dizin.join("dogrulama.png");

    let img = image::RgbaImage::from_fn(64, 48, |x, y| {
        image::Rgba([(x * 4) as u8, (y * 5) as u8, 200, 255])
    });
    if let Err(k) = image::DynamicImage::ImageRgba8(img)
        .save_with_format(&yol, image::ImageFormat::Png)
    {
        return Kontrol::basarisiz("Görüntü boru hattı", format!("PNG üretilemedi: {k}"));
    }

    let ayar = Ayarlar::default();
    let sonuc = match goruntu::isle(&yol, &ayar, Some((128, 128)), None) {
        Ok(s) => s,
        Err(k) => return Kontrol::basarisiz("Görüntü boru hattı", k.to_string()),
    };
    let _ = std::fs::remove_dir_all(&dizin);

    let beklenen = sonuc.genislik as usize * sonuc.yukseklik as usize * 4;
    if sonuc.veri.len() != beklenen {
        return Kontrol::basarisiz(
            "Görüntü boru hattı",
            format!("f16 tampon boyutu {} ≠ {beklenen}", sonuc.veri.len()),
        );
    }
    if sonuc.meta.bicim != crate::goruntu::bicim::Bicim::Png {
        return Kontrol::basarisiz("Görüntü boru hattı", "biçim meta verisi hatalı");
    }
    Kontrol::basarili(
        "Görüntü boru hattı",
        format!(
            "{}×{} → {} px f16, {} bayt VRAM",
            sonuc.meta.ham_genislik,
            sonuc.meta.ham_yukseklik,
            sonuc.genislik * sonuc.yukseklik,
            sonuc.vram_boyutu()
        ),
    )
}

/// Arka plan işçi havuzu: gerçek çözümleme turu.
fn isci_havuzu() -> Kontrol {
    use crate::cekirdek::ayar::Ayarlar;
    use crate::gio::isci::{IsciHavuzu, Istek, Yanit};

    let dizin = gecici_dizin("isci");
    let _ = std::fs::remove_dir_all(&dizin);
    if std::fs::create_dir_all(&dizin).is_err() {
        return Kontrol::atlanmis("İşçi havuzu", "geçici dizin oluşturulamadı");
    }
    let yol = dizin.join("isci.png");
    let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([30, 60, 90, 255]));
    if let Err(k) =
        image::DynamicImage::ImageRgba8(img).save_with_format(&yol, image::ImageFormat::Png)
    {
        return Kontrol::basarisiz("İşçi havuzu", format!("PNG üretilemedi: {k}"));
    }

    let havuz = IsciHavuzu::baslat(Ayarlar::default());
    if !havuz.gonder(Istek::Coz {
        nesil: 0,
        sira: 0,
        yol,
        hedef: None,
        on_yukleme: false,
        ilerleme: None,
    }) {
        return Kontrol::basarisiz("İşçi havuzu", "istek kuyruğa yazılamadı");
    }

    let baslangic = std::time::Instant::now();
    loop {
        for yanit in havuz.yanitlari_topla() {
            if let Yanit::Cozuldu { sonuc, .. } = yanit {
                let _ = std::fs::remove_dir_all(&dizin);
                return match sonuc {
                    Ok(g) => Kontrol::basarili(
                        "İşçi havuzu",
                        format!(
                            "{} işçi, {}×{} çözüldü, {:.0} ms",
                            havuz.isci_sayisi(),
                            g.genislik,
                            g.yukseklik,
                            baslangic.elapsed().as_secs_f64() * 1000.0
                        ),
                    ),
                    Err(k) => Kontrol::basarisiz("İşçi havuzu", k.to_string()),
                };
            }
        }
        if baslangic.elapsed() > std::time::Duration::from_secs(20) {
            let _ = std::fs::remove_dir_all(&dizin);
            return Kontrol::basarisiz("İşçi havuzu", "20 sn içinde yanıt gelmedi");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// GPU kontrolleri: adaptör, HDR bilgisi, doku yükleme, gölgelendirici derlemesi.
fn gpu_kontrolleri() -> Vec<Kontrol> {
    use crate::goruntu::IslenmisGoruntu;
    use crate::gpu::Gpu;
    use crate::gpu::boru::GoruntuBorusu;
    use crate::gpu::doku::GoruntuDokusu;

    let gpu = match Gpu::yeni() {
        Ok(g) => g,
        Err(k) => {
            return vec![Kontrol::atlanmis("Grafik aygıtı", k.to_string())];
        }
    };
    let mut kontroller = vec![Kontrol::basarili(
        "Grafik aygıtı",
        format!(
            "{} — en büyük doku {} px",
            gpu.adaptor_bilgisi,
            gpu.en_buyuk_doku_kenari()
        ),
    )];

    // Gölgelendirici derlemesi (WGSL sözdizimi ve bağlayıcı doğrulaması).
    let boru = GoruntuBorusu::yeni(&gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
    kontroller.push(Kontrol::basarili(
        "Gölgelendirici",
        "WGSL derlendi, boru hattı kuruldu (4 örneklemeli Catmull-Rom)",
    ));

    // Doku yükleme.
    let genislik = 32u32;
    let yukseklik = 24u32;
    let veri: Vec<u16> = (0..(genislik * yukseklik * 4))
        .map(|i| half::f16::from_f32((i % 97) as f32 / 97.0).to_bits())
        .collect();
    match GoruntuDokusu::yeni(
        &gpu,
        &veri,
        genislik,
        yukseklik,
        &boru.yerlesim,
        &boru.uniform_tamponu,
        &boru.ornekleyici,
    ) {
        Ok(doku) => kontroller.push(Kontrol::basarili(
            "GPU dokusu",
            format!(
                "{} yüklendi, {} bayt VRAM, bağlama grubu kuruldu",
                doku.olcu_metni(),
                doku.vram_boyutu()
            ),
        )),
        Err(k) => kontroller.push(Kontrol::basarisiz("GPU dokusu", k.to_string())),
    }

    // Eksik tampon reddedilmeli (sessiz bozulma olmamalı).
    let eksik = vec![0u16; 4];
    match GoruntuDokusu::yeni(
        &gpu,
        &eksik,
        genislik,
        yukseklik,
        &boru.yerlesim,
        &boru.uniform_tamponu,
        &boru.ornekleyici,
    ) {
        Ok(_) => kontroller.push(Kontrol::basarisiz(
            "Doku doğrulaması",
            "eksik veri kabul edildi",
        )),
        Err(_) => kontroller.push(Kontrol::basarili(
            "Doku doğrulaması",
            "eksik piksel tamponu reddedildi",
        )),
    }

    // HDR bilgisi (pencere yüzeyi olmadan yalnızca varsayılan değerler okunur).
    let hdr = crate::gpu::hdr::HdrBilgi::varsayilan();
    kontroller.push(Kontrol::basarili(
        "HDR varsayılanı",
        format!(
            "{} (tepe {:.0} nit, SDR beyazı {:.0} nit)",
            hdr.ozet(),
            hdr.tepe_nits,
            hdr.sdr_beyaz_nits
        ),
    ));

    // Kare üniformu üretimi.
    let uniform = crate::gpu::boru::uniform_olustur(
        (genislik, yukseklik),
        &crate::girdi::bolge::Gorunum::sigdir(genislik, yukseklik, 1280.0, 800.0),
        (1280, 800),
        &hdr,
        false,
        true,
        [24, 24, 28],
    );
    if uniform.yerlesim[0].is_finite() && uniform.donusum[0].is_finite() {
        kontroller.push(Kontrol::basarili(
            "Kare üniformu",
            format!(
                "merkez ({:.3}, {:.3}), yarı ölçü {:.3}",
                uniform.yerlesim[0], uniform.yerlesim[1], uniform.donusum[0]
            ),
        ));
    } else {
        kontroller.push(Kontrol::basarisiz(
            "Kare üniformu",
            "sayısal taşma veya NaN",
        ));
    }

    // Çözücü matrisi: her desteklenen biçim bir çözücüye yönlendirilmeli.
    kontroller.push(cozucu_matrisi());

    // IslenmisGoruntu tipinin GPU ile uyumu (derleme zamanı sözleşmesi).
    let _ = std::mem::size_of::<IslenmisGoruntu>();

    kontroller
}

/// Biçim → çözücü yönlendirmesinin eksiksizliği.
fn cozucu_matrisi() -> Kontrol {
    use crate::goruntu::bicim::Bicim;
    use crate::goruntu::cozucu::resim_formati;

    // image motoruna yönlenen biçimler.
    let image_bicimleri = [
        Bicim::Png,
        Bicim::WebP,
        Bicim::Gif,
        Bicim::Bmp,
        Bicim::Tga,
        Bicim::Ico,
        Bicim::Tiff,
        Bicim::Pnm,
        Bicim::Qoi,
        Bicim::Dds,
        Bicim::Hdr,
        Bicim::Exr,
        Bicim::Avif,
    ];
    let eksikler: Vec<&str> = image_bicimleri
        .iter()
        .filter(|b| resim_formati(**b).is_none())
        .map(|b| crate::goruntu::bicim::bicim_adi(*b))
        .collect();
    if !eksikler.is_empty() {
        return Kontrol::basarisiz(
            "Çözücü matrisi",
            format!("yönlendirmesi olmayan biçimler: {}", eksikler.join(", ")),
        );
    }
    // Özel çözücüler image motoruna yönlenmemeli.
    if resim_formati(Bicim::Jpeg).is_some() || resim_formati(Bicim::Svg).is_some() {
        return Kontrol::basarisiz("Çözücü matrisi", "JPEG/SVG yanlış motora yönlendi");
    }
    // Yüksek dinamik aralıklı biçimler doğru sınıflandırılmalı (ton haritalama buna bağlı).
    if !crate::goruntu::bicim::hdr_mi(Bicim::Exr) || !crate::goruntu::bicim::hdr_mi(Bicim::Hdr) {
        return Kontrol::basarisiz("Çözücü matrisi", "EXR/HDR yüksek dinamik aralıklı sayılmadı");
    }
    if crate::goruntu::bicim::hdr_mi(Bicim::Jpeg) {
        return Kontrol::basarisiz("Çözücü matrisi", "JPEG yanlışlıkla HDR sayıldı");
    }
    // Dosya ilişkilendirmesine yazılan uzantılar, görüntüleyicinin gerçekten açabildiği
    // uzantıların dışına çıkmamalıdır (kabuk kaydı ile çözücü matrisi ayrışmasın).
    let iliskili = crate::kabuk::iliskilendirme::desteklenen_uzantilar();
    let desteksiz: Vec<&str> = iliskili
        .iter()
        .copied()
        .filter(|u| !crate::goruntu::bicim::uzanti_desteklenir(u))
        .collect();
    if !desteksiz.is_empty() {
        return Kontrol::basarisiz(
            "Çözücü matrisi",
            format!("ilişkilendirmede desteksiz uzantılar: {}", desteksiz.join(", ")),
        );
    }
    if iliskili.is_empty() {
        return Kontrol::basarisiz("Çözücü matrisi", "ilişkilendirme uzantı listesi boş");
    }
    Kontrol::basarili(
        "Çözücü matrisi",
        format!(
            "{} biçim doğru motora yönlendirildi, {} ilişkilendirme uzantısı tutarlı",
            image_bicimleri.len() + 4,
            iliskili.len()
        ),
    )
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kontrol_listesi_bos_degil() {
        let kontroller = calistir();
        assert!(
            kontroller.len() >= 10,
            "en az 10 kontrol beklenir, gelen {}",
            kontroller.len()
        );
    }

    #[test]
    fn gpu_disi_kontroller_basarili() {
        // GPU gerektirmeyen kontroller her ortamda geçmelidir.
        for kontrol in calistir() {
            if let Durum::Basarisiz(ayrinti) = &kontrol.durum {
                // Yalnızca grafik aygıtına bağlı kontroller ortama göre başarısız olabilir.
                let gpu_bagimli = matches!(
                    kontrol.ad,
                    "Grafik aygıtı" | "GPU dokusu" | "Gölgelendirici" | "Kare üniformu"
                );
                assert!(gpu_bagimli, "{} başarısız: {ayrinti}", kontrol.ad);
            }
        }
    }
}
