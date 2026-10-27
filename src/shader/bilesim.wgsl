// HDR yolunda egui katmanını scRGB yüzeyine birleştiren tam ekran geçiş.
//
// egui, gama kodlu değerleri Rgba8Unorm ara dokuya yazar (ön çarpımlı alfa). Bu geçiş
// değerleri düz alfaya çevirir, gama uzayından doğrusala dönüştürür, SDR beyazı ölçeğiyle
// (sdr_beyaz_nits / 80) çarpar ve doğrusal ışıkta ön çarpımlı alfa ile harmanlar.

struct BilesimUniform {
    // x = SDR beyazının scRGB ölçeği (1.0 = 80 nit).
    olcek: vec4<f32>,
};

@group(0) @binding(0) var arayuz_dokusu: texture_2d<f32>;
@group(0) @binding(1) var arayuz_ornekleyici: sampler;
@group(0) @binding(2) var<uniform> ayar: BilesimUniform;

struct TepeCikti {
    @builtin(position) konum: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_ana(@builtin(vertex_index) indeks: u32) -> TepeCikti {
    // Tam ekran üçgen: (0,0), (2,0), (0,2) → NCD (-1,-1), (3,-1), (-1,3).
    var ncd = vec2<f32>(f32((indeks << 1u) & 2u), f32(indeks & 2u));
    ncd = ncd * 2.0 - 1.0;

    var cikti: TepeCikti;
    cikti.konum = vec4<f32>(ncd, 0.0, 1.0);
    // NCD y ekseni yukarı, doku UV'si y aşağı: doku üst satırı ekranın üstüne gelsin.
    cikti.uv = vec2<f32>(ncd.x, -ncd.y) * 0.5 + vec2<f32>(0.5, 0.5);
    return cikti;
}

// Gama kodlu sRGB değerini doğrusala çevirir.
fn gamadan_dogrusal(deger: vec3<f32>) -> vec3<f32> {
    let guvenli = clamp(deger, vec3<f32>(0.0), vec3<f32>(1.0));
    let kucuk = guvenli <= vec3<f32>(0.04045);
    let alt = guvenli / 12.92;
    let ust = pow((guvenli + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(ust, alt, kucuk);
}

@fragment
fn fs_ana(girdi: TepeCikti) -> @location(0) vec4<f32> {
    let ui = textureSample(arayuz_dokusu, arayuz_ornekleyici, girdi.uv);
    let alfa = clamp(ui.a, 0.0, 1.0);
    if (alfa <= 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    // Ön çarpımlı gama değerini düz alfaya çevir; ölçekleme doğrusal uzayda yapılır.
    let duz_gama = ui.rgb / alfa;
    let dogrusal = gamadan_dogrusal(duz_gama) * ayar.olcek.x;
    return vec4<f32>(dogrusal * alfa, alfa);
}
