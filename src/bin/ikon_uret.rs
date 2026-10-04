//! assets/favikon_SP-GROUP_gorsel.png → assets/gorsel.ico üretimi.
//!
//! ICO kapsayıcısı el ile kurulur (başlık + dizin girdileri + PNG kareler);
//! Windows Vista ve üzeri ICO içinde PNG sıkıştırması yerel olarak destekler.

use std::fs;
use std::path::PathBuf;

/// EXE kaynağına gömülecek kare boyları (piksel).
const BOYUTLAR: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

fn main() {
    let kok = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kaynak_yolu = kok.join("assets/favikon_SP-GROUP_gorsel.png");
    let hedef_yolu = kok.join("assets/gorsel.ico");

    let kaynak = image::open(&kaynak_yolu).unwrap_or_else(|k| panic!("favikon okunamadı: {k}"));
    let kenar = kaynak.width().max(kaynak.height());
    println!("kaynak: {}x{}", kaynak.width(), kaynak.height());

    let mut kareler: Vec<(u32, Vec<u8>)> = Vec::new();
    for boyut in BOYUTLAR {
        if boyut > kenar {
            break;
        }
        let kare = kaynak.resize_exact(boyut, boyut, image::imageops::FilterType::Lanczos3);
        let mut png = Vec::new();
        kare.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap_or_else(|k| panic!("{boyut}px kare PNG kodlanamadı: {k}"));
        kareler.push((boyut, png));
    }
    assert!(kareler.len() >= 2, "kaynak ikon çok küçük: {kenar}px");

    let mut ico = Vec::new();
    ico.extend_from_slice(&[0u8, 0, 1, 0]); // ayrılmış=0, tür=1 (ikon)
    ico.extend_from_slice(&(kareler.len() as u16).to_le_bytes());
    let mut ofset = (6 + kareler.len() * 16) as u32;
    for (boyut, png) in &kareler {
        let kenar_bayt = if *boyut == 256 { 0 } else { *boyut as u8 };
        ico.extend_from_slice(&[kenar_bayt, kenar_bayt, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes()); // düzlem sayısı
        ico.extend_from_slice(&32u16.to_le_bytes()); // bit derinliği
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&ofset.to_le_bytes());
        ofset += png.len() as u32;
    }
    for (_, png) in &kareler {
        ico.extend_from_slice(png);
    }

    fs::write(&hedef_yolu, &ico).unwrap_or_else(|k| panic!("ico yazılamadı: {k}"));
    println!(
        "{} -> {} ({} kare, {} bayt)",
        kaynak_yolu.display(),
        hedef_yolu.display(),
        kareler.len(),
        ico.len()
    );
}
