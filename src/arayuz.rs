//! Türkçe arayüz dilinin öğeleri: `arayüz:` bloğunda kullanılan `başlık`,
//! `düğme`, `giriş`, `satır` ... Ayrıştırıcı, denetçi, kod üretici ve dil
//! sunucusu bu tabloyu ortak kullanır. Öğelerin tarayıcıdaki karşılıkları
//! (HTML ve stil) `runtime/wasm/orhunca.js` içindedir.

/// Bir öğenin konumsal değerinin beklenen tipi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beklenen {
    /// Her tip: metne çevrilerek gösterilir.
    Herhangi,
    Metin,
    Sayi,
    /// Sayı ya da ondalık
    Sayisal,
    /// `liste<metin>` (seçim kutusunun seçenekleri)
    MetinListesi,
    /// Herhangi bir liste (iç içe de olabilir); JSON olarak gönderilir
    Veri,
    /// Durum değişkenine bağlanan değer: kullanıcı değiştirince değişken güncellenir.
    Bag(BagTuru),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BagTuru {
    /// metin, sayı ya da ondalık (giriş kutusu, seçim)
    Yazi,
    /// yalnızca metin (çok satırlı yazı alanı)
    Metin,
    /// mantık (onay kutusu)
    Mantik,
    /// sayı ya da ondalık (kaydırıcı)
    Sayisal,
}

pub struct OgeTanimi {
    pub ad: &'static str,
    /// Konumsal değerler: tarayıcı tarafındaki adı ve beklenen tip.
    pub degerler: &'static [(&'static str, Beklenen)],
    /// Zorunlu konumsal değer sayısı
    pub zorunlu: usize,
    /// İçine başka öğeler alır mı (`satır:`)
    pub kapsayici: bool,
    pub olaylar: &'static [&'static str],
    pub aciklama: &'static str,
    pub ornek: &'static str,
}

use Beklenen::*;

pub const OLAYLAR: &[(&str, &str)] = &[
    ("tıklanınca", "öğeye tıklanınca"),
    (
        "değişince",
        "kullanıcı değeri değiştirince (bağlı değişken güncellendikten sonra)",
    ),
    ("gönderilince", "giriş kutusunda Enter'a basılınca"),
    ("çalınca", "zamanlayıcının süresi her dolduğunda"),
    (
        "her_karede",
        "oyun alanında saniyede yaklaşık 60 kez (oyun döngüsü)",
    ),
];

const TIK: &[&str] = &["tıklanınca"];
const DEGISINCE: &[&str] = &["değişince"];

pub const OGELER: &[OgeTanimi] = &[
    OgeTanimi {
        ad: "başlık",
        degerler: &[("metin", Herhangi)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: TIK,
        aciklama: "Büyük başlık",
        ornek: "başlık(\"Hoş geldiniz\")",
    },
    OgeTanimi {
        ad: "alt_başlık",
        degerler: &[("metin", Herhangi)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: TIK,
        aciklama: "Bölüm başlığı",
        ornek: "alt_başlık(\"Ayarlar\")",
    },
    OgeTanimi {
        ad: "yazı",
        degerler: &[("metin", Herhangi)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: TIK,
        aciklama: "Bir paragraf yazı; her tip metne çevrilir",
        ornek: "yazı(\"Toplam: \" + toplam)",
    },
    OgeTanimi {
        ad: "düğme",
        degerler: &[("metin", Herhangi)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: TIK,
        aciklama: "Düğme",
        ornek: "düğme(\"Kaydet\") tıklanınca:",
    },
    OgeTanimi {
        ad: "bağlantı",
        degerler: &[("metin", Herhangi), ("adres", Metin)],
        zorunlu: 2,
        kapsayici: false,
        olaylar: TIK,
        aciklama: "Başka bir sayfaya bağlantı (yeni sekmede açılır)",
        ornek: "bağlantı(\"Orhunca\", \"https://github.com/furkan003/orhunca\")",
    },
    OgeTanimi {
        ad: "resim",
        degerler: &[("adres", Metin), ("açıklama", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: TIK,
        aciklama: "Resim",
        ornek: "resim(\"kedi.png\", genişlik: 200)",
    },
    OgeTanimi {
        ad: "ayraç",
        degerler: &[],
        zorunlu: 0,
        kapsayici: false,
        olaylar: &[],
        aciklama: "Yatay ayırma çizgisi",
        ornek: "ayraç()",
    },
    OgeTanimi {
        ad: "boşluk",
        degerler: &[],
        zorunlu: 0,
        kapsayici: false,
        olaylar: &[],
        aciklama: "Satır ya da sütunda kalan yeri dolduran boşluk",
        ornek: "boşluk()",
    },
    OgeTanimi {
        ad: "giriş",
        degerler: &[("değer", Bag(BagTuru::Yazi)), ("yer_tutucu", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: &["değişince", "gönderilince"],
        aciklama: "Tek satırlık giriş kutusu; değeri bir durum değişkenine bağlıdır",
        ornek: "giriş(ad, \"Adınız\")",
    },
    OgeTanimi {
        ad: "metin_alanı",
        degerler: &[("değer", Bag(BagTuru::Metin)), ("yer_tutucu", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: &["değişince"],
        aciklama: "Çok satırlı yazı alanı",
        ornek: "metin_alanı(not, \"Notunuz\")",
    },
    OgeTanimi {
        ad: "onay_kutusu",
        degerler: &[("değer", Bag(BagTuru::Mantik)), ("metin", Herhangi)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: &["değişince"],
        aciklama: "Onay kutusu; mantık tipindeki bir durum değişkenine bağlıdır",
        ornek: "onay_kutusu(karanlık, \"Karanlık tema\")",
    },
    OgeTanimi {
        ad: "seçim",
        degerler: &[("değer", Bag(BagTuru::Yazi)), ("seçenekler", MetinListesi)],
        zorunlu: 2,
        kapsayici: false,
        olaylar: &["değişince"],
        aciklama: "Açılır seçim kutusu",
        ornek: "seçim(şehir, [\"Ankara\", \"İstanbul\", \"İzmir\"])",
    },
    OgeTanimi {
        ad: "kaydırıcı",
        degerler: &[
            ("değer", Bag(BagTuru::Sayisal)),
            ("en_az", Sayisal),
            ("en_fazla", Sayisal),
            ("adım", Sayisal),
        ],
        zorunlu: 3,
        kapsayici: false,
        olaylar: &["değişince"],
        aciklama: "Kaydırıcı; sayı ya da ondalık bir durum değişkenine bağlıdır",
        ornek: "kaydırıcı(ses, 0, 100)",
    },
    OgeTanimi {
        ad: "ilerleme",
        degerler: &[("değer", Sayisal), ("en_fazla", Sayisal)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: &[],
        aciklama: "İlerleme çubuğu (varsayılan en fazla 100)",
        ornek: "ilerleme(yüzde)",
    },
    OgeTanimi {
        ad: "tablo",
        degerler: &[("satırlar", Veri)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: &[],
        aciklama: "Tablo; ilk satır başlıktır (her satır bir liste)",
        ornek: "tablo([[\"Ad\", \"Not\"], [\"Ayşe\", \"90\"]])",
    },
    OgeTanimi {
        ad: "grafik",
        degerler: &[("değerler", Veri), ("etiketler", MetinListesi), ("tür", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: &[],
        aciklama: "Grafik: tür \"çubuk\" (varsayılan), \"çizgi\" ya da \"pasta\"",
        ornek: "grafik([12, 19, 7], [\"Ocak\", \"Şubat\", \"Mart\"], \"çubuk\")",
    },
    OgeTanimi {
        ad: "sekmeler",
        degerler: &[("değer", Bag(BagTuru::Yazi)), ("seçenekler", MetinListesi)],
        zorunlu: 2,
        kapsayici: false,
        olaylar: &["değişince"],
        aciklama: "Sekme çubuğu; seçili sekmenin adı bir durum değişkenine bağlıdır",
        ornek: "sekmeler(sekme, [\"Genel\", \"Ayarlar\"])",
    },
    OgeTanimi {
        ad: "iletişim_kutusu",
        degerler: &[("açık", Bag(BagTuru::Mantik)), ("başlık", Herhangi)],
        zorunlu: 1,
        kapsayici: true,
        olaylar: &["değişince"],
        aciklama: "Sayfanın üstünde açılan pencere; mantık değişkeni doğruyken görünür, kapatılınca yanlış olur",
        ornek: "iletişim_kutusu(açık, \"Emin misiniz?\"):",
    },
    OgeTanimi {
        ad: "satır",
        degerler: &[],
        zorunlu: 0,
        kapsayici: true,
        olaylar: TIK,
        aciklama: "İçindekileri yan yana dizer",
        ornek: "satır:",
    },
    OgeTanimi {
        ad: "sütun",
        degerler: &[],
        zorunlu: 0,
        kapsayici: true,
        olaylar: TIK,
        aciklama: "İçindekileri alt alta dizer",
        ornek: "sütun:",
    },
    OgeTanimi {
        ad: "kart",
        degerler: &[],
        zorunlu: 0,
        kapsayici: true,
        olaylar: TIK,
        aciklama: "Kenarlıklı, gölgeli kutu",
        ornek: "kart:",
    },
    OgeTanimi {
        ad: "kutu",
        degerler: &[],
        zorunlu: 0,
        kapsayici: true,
        olaylar: TIK,
        aciklama: "Süssüz kapsayıcı (seçeneklerle biçimlenir)",
        ornek: "kutu(arka: \"açık_gri\", iç_boşluk: 12):",
    },
    OgeTanimi {
        ad: "ızgara",
        degerler: &[("sütun", Sayi)],
        zorunlu: 1,
        kapsayici: true,
        olaylar: TIK,
        aciklama: "İçindekileri verilen sütun sayısıyla ızgaraya dizer",
        ornek: "ızgara(3):",
    },
    OgeTanimi {
        ad: "zamanlayıcı",
        degerler: &[("süre", Sayisal)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: &["çalınca"],
        aciklama: "Görünmez; her `süre` saniyede bir `çalınca:` bloğunu çalıştırır",
        ornek: "zamanlayıcı(1) çalınca:",
    },
    OgeTanimi {
        ad: "kamera",
        degerler: &[("değer", Bag(BagTuru::Yazi)), ("metin", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: DEGISINCE,
        aciklama: "Fotoğraf çekme düğmesi; çekilen fotoğraf (resim adresi) değişkene yazılır",
        ornek: "kamera(fotoğraf, \"Fotoğraf çek\") değişince:",
    },
    OgeTanimi {
        ad: "karekod_okuyucu",
        degerler: &[("değer", Bag(BagTuru::Yazi)), ("metin", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: DEGISINCE,
        aciklama: "Kamerayla karekod (QR) ya da barkod okur; okunan metin değişkene yazılır",
        ornek: "karekod_okuyucu(kod, \"Karekod okut\") değişince:",
    },
    OgeTanimi {
        ad: "konum",
        degerler: &[("değer", Bag(BagTuru::Yazi)), ("metin", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: DEGISINCE,
        aciklama: "Konum düğmesi; \"enlem,boylam\" değişkene yazılır (izin istenir)",
        ornek: "konum(yer, \"Konumumu bul\") değişince:",
    },
    OgeTanimi {
        ad: "dosya_seç",
        degerler: &[("değer", Bag(BagTuru::Yazi)), ("metin", Metin)],
        zorunlu: 1,
        kapsayici: false,
        olaylar: DEGISINCE,
        aciklama: "Dosya seçme düğmesi; metin dosyasının içeriği (resim ise resim adresi) değişkene yazılır",
        ornek: "dosya_seç(içerik, \"Dosya seç\", tür: \".csv,.txt\") değişince:",
    },
    OgeTanimi {
        ad: "oyun_alanı",
        degerler: &[("genişlik", Sayi), ("yükseklik", Sayi)],
        zorunlu: 2,
        kapsayici: false,
        olaylar: &["her_karede", "tıklanınca"],
        aciklama: "Oyunlar için çizim alanı: `her_karede:` bloğu saniyede yaklaşık 60 kez çalışır; içinde daire, dikdörtgen, yazı_çiz ... ile çizilir",
        ornek: "oyun_alanı(480, 320) her_karede:",
    },
];

/// Bütün öğelerde kullanılabilen adlı seçenekler: ad, beklenen tip, açıklama.
pub const SECENEKLER: &[(&str, Beklenen, &str)] = &[
    ("renk", Metin, "yazı rengi: \"kırmızı\", \"#3366ff\""),
    ("arka", Metin, "arka plan rengi"),
    ("boyut", Sayisal, "yazı boyutu (piksel)"),
    ("kalın", Herhangi, "kalın yazı (doğru/yanlış)"),
    ("eğik", Herhangi, "eğik yazı (doğru/yanlış)"),
    (
        "hizala",
        Metin,
        "\"sol\", \"orta\", \"sağ\" (kapsayıcılarda içindekiler)",
    ),
    ("genişlik", Herhangi, "genişlik: piksel ya da \"50%\""),
    ("yükseklik", Herhangi, "yükseklik: piksel ya da \"50%\""),
    (
        "boşluk",
        Sayisal,
        "kapsayıcıda öğeler arası boşluk (piksel)",
    ),
    (
        "iç_boşluk",
        Sayisal,
        "kenarlarla içerik arası boşluk (piksel)",
    ),
    ("köşe", Sayisal, "köşe yuvarlaklığı (piksel)"),
    ("kenarlık", Metin, "kenarlık rengi"),
    ("sınıf", Metin, "eklenen CSS sınıfı"),
    ("ipucu", Herhangi, "üzerine gelince görünen yazı"),
    ("etkin", Herhangi, "yanlış ise kullanılamaz (düğme, giriş)"),
    ("gizli", Herhangi, "doğru ise görünmez"),
    (
        "tür",
        Metin,
        "giriş kutusunun türü: \"şifre\", \"e_posta\", \"tarih\", \"renk\"",
    ),
];

/// Mantık beklenen seçenekler
pub const MANTIK_SECENEKLERI: &[&str] = &["kalın", "eğik", "etkin", "gizli"];

pub fn oge(ad: &str) -> Option<&'static OgeTanimi> {
    OGELER.iter().find(|o| o.ad == ad)
}

pub fn olay_mi(k: &str) -> bool {
    OLAYLAR.iter().any(|(o, _)| *o == k)
}

pub fn secenek(ad: &str) -> Option<(Beklenen, &'static str)> {
    SECENEKLER
        .iter()
        .find(|(a, _, _)| *a == ad)
        .map(|(_, b, a)| (*b, *a))
}
