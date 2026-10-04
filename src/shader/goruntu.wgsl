// 4-örneklemeli Catmull-Rom yeniden örnekleme ve ton haritalama gölgelendiricisi.
//
// Görüntü, pencereye tek bir tam ekran üçgen olarak çizilir; yeniden örnekleme GPU'da
// yapılır. Klasik 2B bicubic 16 doku okuması gerektirirken Catmull-Rom ağırlıkları
// donanımın çift doğrusal (bilinear) filtreleme birimlerine gömülür: bir eksende iki
// ardışık tekselin ağırlıklı ortalaması, donanımın ürettiği (1-t)*T0 + t*T1 değeriyle
// aynı olduğundan örnek konumu w1/(w0+w1) kadar kaydırılır ve 2 okuma yeter.
// İki eksende 2 × 2 = 4 doku okuması.

struct GoruntuUniform {
    // Kaynak dokunun piksel ölçüsü.
    doku_boyutu: vec2<f32>,
    _dolgu: vec2<f32>,
    // Görüntü merkezinin pencere içindeki yeri (0-1 aralığında, xy) ve pencere ölçüsü (piksel, zw).
    yerlesim: vec4<f32>,
    // Görüntünün pencereye göre yarı ölçüsü (0-1 aralığında, xy).
    donusum: vec4<f32>,
    // x = tepe parlaklık (nit), y = SDR beyaz referansı (nit),
    // z = HDR kaynak (1 ise ton haritalama uygulanır), w = gama kodlaması (1 ise shader sRGB kodlar).
    ton: vec4<f32>,
    // Arka plan rengi (doğrusal RGBA).
    arka_plan: vec4<f32>,
};

@group(0) @binding(0) var<uniform> ayar: GoruntuUniform;
@group(0) @binding(1) var goruntu_dokusu: texture_2d<f32>;
@group(0) @binding(2) var goruntu_ornekleyici: sampler;

struct TepeCikti {
    @builtin(position) konum: vec4<f32>,
    // Pencere içi konum (piksel, sol üst köşe orijin, y aşağı).
    @location(0) pencere_konumu: vec2<f32>,
};

@vertex
fn vs_ana(@builtin(vertex_index) indeks: u32) -> TepeCikti {
    // Tam ekran üçgen: (0,0), (2,0), (0,2) → NCD (-1,-1), (3,-1), (-1,3).
    var ncd = vec2<f32>(f32((indeks << 1u) & 2u), f32(indeks & 2u));
    ncd = ncd * 2.0 - 1.0;

    var cikti: TepeCikti;
    cikti.konum = vec4<f32>(ncd, 0.0, 1.0);
    // NCD y ekseni yukarı, pencere pikseli y ekseni aşağı: bu yüzden ters çevrilir.
    cikti.pencere_konumu = vec2<f32>(ncd.x, -ncd.y) * 0.5 * ayar.yerlesim.zw
        + ayar.yerlesim.zw * 0.5;
    return cikti;
}

// Catmull-Rom ağırlıkları (gerilim 0,5); f, örnek konumunun teksel içi kesridir.
fn agirliklar(f: f32) -> vec4<f32> {
    let f2 = f * f;
    let f3 = f2 * f;
    return vec4<f32>(
        -0.5 * f3 + f2 - 0.5 * f,
        1.5 * f3 - 2.5 * f2 + 1.0,
        -1.5 * f3 + 2.0 * f2 + 0.5 * f,
        0.5 * f3 - 0.5 * f2,
    );
}

// Bir eksende 2 örnek için konum (xy, 0-1 aralığı) ve birleşik ağırlık (zw) üretir.
// w0+w1 veya w2+w3 sıfıra yakınsa bölme yapılmaz; NaN üretmemek için koşullu seçim kullanılır.
fn eksen_ornekleri(w: vec4<f32>, taban: f32, teksel: f32) -> vec4<f32> {
    let dis_a = w.x + w.y;
    let dis_b = w.z + w.w;
    let guvenli_a = select(0.0, w.y / dis_a, abs(dis_a) > 1e-6);
    let guvenli_b = select(0.0, w.w / dis_b, abs(dis_b) > 1e-6);
    // T0 = taban-1, T1 = taban, T2 = taban+1, T3 = taban+2 teksel merkezleridir.
    let konum_a = (taban - 1.0 + guvenli_a) * teksel;
    let konum_b = (taban + 1.0 + guvenli_b) * teksel;
    return vec4<f32>(konum_a, konum_b, dis_a, dis_b);
}

// 4 doku okumasıyla Catmull-Rom örneklemesi.
fn catmull_rom_4(uv: vec2<f32>) -> vec4<f32> {
    let boyut = ayar.doku_boyutu;
    let ornek_konumu = uv * boyut;
    let taban = floor(ornek_konumu - vec2<f32>(0.5, 0.5)) + vec2<f32>(0.5, 0.5);
    let f = ornek_konumu - taban;

    let wx = eksen_ornekleri(agirliklar(f.x), taban.x, 1.0 / boyut.x);
    let wy = eksen_ornekleri(agirliklar(f.y), taban.y, 1.0 / boyut.y);

    let a0 = textureSampleLevel(goruntu_dokusu, goruntu_ornekleyici, vec2<f32>(wx.x, wy.x), 0.0);
    let a1 = textureSampleLevel(goruntu_dokusu, goruntu_ornekleyici, vec2<f32>(wx.y, wy.x), 0.0);
    let b0 = textureSampleLevel(goruntu_dokusu, goruntu_ornekleyici, vec2<f32>(wx.x, wy.y), 0.0);
    let b1 = textureSampleLevel(goruntu_dokusu, goruntu_ornekleyici, vec2<f32>(wx.y, wy.y), 0.0);

    return a0 * (wx.z * wy.z) + a1 * (wx.w * wy.z) + b0 * (wx.z * wy.w) + b1 * (wx.w * wy.w);
}

// Doğrusal değeri sRGB kodlamasına çevirir.
fn dogrusaldan_srgb(deger: vec3<f32>) -> vec3<f32> {
    let guvenli = max(deger, vec3<f32>(0.0));
    let kucuk = guvenli <= vec3<f32>(0.0031308);
    let alt = guvenli * 12.92;
    let ust = 1.055 * pow(guvenli, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(ust, alt, kucuk);
}

// HDR içeriği ekranın bildirdiği tepe parlaklığa göre sıkıştırır (Reinhard + beyaz noktası).
fn ton_haritala(renk: vec3<f32>, tepe_nits: f32, beyaz_nits: f32) -> vec3<f32> {
    let tepe = max(tepe_nits, 1.0);
    let beyaz = clamp(beyaz_nits, 1.0, tepe);
    let olcekli = max(renk, vec3<f32>(0.0)) * tepe;
    let sikistirilmis = olcekli / (olcekli + vec3<f32>(beyaz));
    return sikistirilmis * (beyaz / tepe);
}

@fragment
fn fs_ana(girdi: TepeCikti) -> @location(0) vec4<f32> {
    // Pencere pikselinden görüntü UV'sine: yerel 0-1 aralığı, sol üst köşe orijin.
    let merkez_piksel = ayar.yerlesim.xy * ayar.yerlesim.zw;
    let yarim_olcu_piksel = ayar.donusum.xy * ayar.yerlesim.zw;
    let yerel = (girdi.pencere_konumu - merkez_piksel) / (yarim_olcu_piksel * 2.0)
        + vec2<f32>(0.5, 0.5);

    // Görüntü dikdörtgeninin dışı arka plan rengini alır.
    if (yerel.x < 0.0 || yerel.x > 1.0 || yerel.y < 0.0 || yerel.y > 1.0) {
        return ayar.arka_plan;
    }

    var renk = catmull_rom_4(yerel);
    let alfa = renk.a;

    if (ayar.ton.z > 0.5) {
        renk = vec4<f32>(ton_haritala(renk.rgb, ayar.ton.x, ayar.ton.y), alfa);
    }

    // Hedef format sRGB değilse (float yüzey) değerler shader içinde sRGB kodlanır;
    // egui aynı hedefte aynı kodlamayı uyguladığı için HUD ile görüntü tutarlı kalır.
    if (ayar.ton.w > 0.5) {
        renk = vec4<f32>(dogrusaldan_srgb(renk.rgb), alfa);
    }

    return renk;
}
