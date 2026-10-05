//! Yeni proje şablonları. `yakinda` dolu olanlar henüz desteklenmeyen proje
//! türleridir; Stüdyo onları "yakında" olarak gösterir.
//!
//! Web şablonlarının dosyaları `sablon_dosyalari/<kimlik>/` altındadır ve ikili
//! dosyaya gömülür; `<kimlik>.sade/` aynı dosyaların örneksiz (sade) hâlidir.
//! Dosyalardaki `«ad»` proje adıyla değiştirilir.

mod gomulu {
    include!(concat!(env!("OUT_DIR"), "/sablon_dosyalari.rs"));
}

fn gomulu_dosya(yol: &str) -> Option<&'static str> {
    gomulu::DOSYALAR
        .iter()
        .find(|(y, _)| *y == yol)
        .and_then(|(_, b)| std::str::from_utf8(b).ok())
}

/// Web şablonu mu? (Stüdyo, çalıştırınca canlı önizlemeyi açar.)
pub fn web_mi(s: &Sablon) -> bool {
    s.kategoriler.contains(&"Web")
}

pub struct Sablon {
    pub kimlik: &'static str,
    pub ad: &'static str,
    pub aciklama: &'static str,
    pub simge: &'static str,
    pub kategoriler: &'static [&'static str],
    pub etiketler: &'static [&'static str],
    /// Desteklenmiyorsa hangi aşamada geleceği.
    pub yakinda: Option<&'static str>,
    /// Oluşturulacak dosyaların listesi (proje dosyası `{ad}.ohcproj` olarak gösterilir).
    pub dosyalar: &'static [&'static str],
    /// Projenin giriş dosyası.
    pub giris: &'static str,
}

pub const SABLONLAR: &[Sablon] = &[
    Sablon {
        kimlik: "konsol",
        ad: "Konsol Uygulaması",
        aciklama: "Komut satırında çalışan, klavyeden okuyup ekrana yazan uygulama.",
        simge: "terminal",
        kategoriler: &["Konsol"],
        etiketler: &["Orhunca", "Linux · Windows", "Konsol"],
        yakinda: None,
        dosyalar: &["ana.ohc", "{ad}.ohcproj", "BENİOKU.md"],
        giris: "ana.ohc",
    },
    Sablon {
        kimlik: "sayi_tahmin",
        ad: "Sayı Tahmin Oyunu",
        aciklama: "Bilgisayarın tuttuğu sayıyı bulmaya çalıştığınız küçük bir oyun.",
        simge: "sports_esports",
        kategoriler: &["Konsol"],
        etiketler: &["Orhunca", "Konsol", "Oyun"],
        yakinda: None,
        dosyalar: &["oyun.ohc", "{ad}.ohcproj", "BENİOKU.md"],
        giris: "oyun.ohc",
    },
    Sablon {
        kimlik: "kutuphane",
        ad: "Kütüphane",
        aciklama: "Başka projelerin kullan ile alabileceği fiil ve işlevler, testleriyle.",
        simge: "deployed_code",
        kategoriler: &["Kütüphane"],
        etiketler: &["Orhunca", "Tüm platformlar", "Kütüphane"],
        yakinda: None,
        dosyalar: &[
            "kutuphane.ohc",
            "testler/kutuphane_testi.ohc",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "testler/kutuphane_testi.ohc",
    },
    Sablon {
        kimlik: "bos_web",
        ad: "Boş Web Sayfası",
        aciklama: "Tek bir sayfa ve stil dosyasıyla sıfırdan başlayın.",
        simge: "draft",
        kategoriler: &["Web"],
        etiketler: &["Orhunca", "Web", "Başlangıç"],
        yakinda: None,
        dosyalar: &[
            "sunucu.ohc",
            "görünümler/anasayfa.ohchtml",
            "statik/stil.css",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "sunucu.ohc",
    },
    Sablon {
        kimlik: "web_sitesi",
        ad: "Web Sitesi",
        aciklama: "Ortak düzenli çok sayfalı site ve kaydedilen iletişim formu.",
        simge: "language",
        kategoriler: &["Web"],
        etiketler: &["Orhunca", "Web", "Sayfalar"],
        yakinda: None,
        dosyalar: &[
            "sunucu.ohc",
            "görünümler/düzen.ohchtml",
            "görünümler/anasayfa.ohchtml",
            "görünümler/hakkında.ohchtml",
            "görünümler/iletişim.ohchtml",
            "görünümler/teşekkürler.ohchtml",
            "statik/stil.css",
            "statik/logo.svg",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "sunucu.ohc",
    },
    Sablon {
        kimlik: "web_uyg",
        ad: "Web Uygulaması",
        aciklama: "JSON API'li, sayfası yenilenmeden çalışan etkileşimli uygulama.",
        simge: "web",
        kategoriler: &["Web"],
        etiketler: &["Orhunca", "Web", "JavaScript"],
        yakinda: None,
        dosyalar: &[
            "sunucu.ohc",
            "statik/index.html",
            "statik/uygulama.js",
            "statik/stil.css",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "sunucu.ohc",
    },
    Sablon {
        kimlik: "acilis",
        ad: "Açılış Sayfası",
        aciklama: "Hazır bölümlerle ürün veya etkinlik tanıtım sayfası.",
        simge: "rocket_launch",
        kategoriler: &["Web"],
        etiketler: &["Orhunca", "Web", "Pazarlama"],
        yakinda: None,
        dosyalar: &[
            "sunucu.ohc",
            "görünümler/sayfa.ohchtml",
            "görünümler/bölümler/kahraman.ohchtml",
            "görünümler/bölümler/özellikler.ohchtml",
            "görünümler/bölümler/kayıt_formu.ohchtml",
            "statik/stil.css",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "sunucu.ohc",
    },
    Sablon {
        kimlik: "web_api",
        ad: "Web API",
        aciklama: "JSON döndüren REST uç noktaları sunan sunucu uygulaması.",
        simge: "dns",
        kategoriler: &["Web", "Sunucu"],
        etiketler: &["Orhunca", "Sunucu", "REST"],
        yakinda: None,
        dosyalar: &[
            "sunucu.ohc",
            "yollar/hava.ohc",
            "yollar/kullanıcılar.ohc",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "sunucu.ohc",
    },
    Sablon {
        kimlik: "tam_yigin",
        ad: "Tam Yığın Uygulama",
        aciklama: "Sayfalar, sunucu ve veri deposu tek projede: ürün yönetimi.",
        simge: "stacks",
        kategoriler: &["Web", "Sunucu"],
        etiketler: &["Orhunca", "Web", "Sunucu", "Veri"],
        yakinda: None,
        dosyalar: &[
            "sunucu.ohc",
            "modeller/ürün.ohc",
            "yollar/ürünler.ohc",
            "görünümler/düzen.ohchtml",
            "görünümler/ürünler.ohchtml",
            "görünümler/ürün_formu.ohchtml",
            "statik/stil.css",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "sunucu.ohc",
    },
    Sablon {
        kimlik: "arayuz",
        ad: "Arayüz Uygulaması",
        aciklama: "Türkçe arayüz diliyle düğmeli, listeli bir uygulama; tarayıcıda çalışır.",
        simge: "desktop_windows",
        kategoriler: &["Masaüstü", "Web"],
        etiketler: &["Orhunca", "WebAssembly", "Arayüz"],
        yakinda: None,
        dosyalar: &["uygulama.ohc", "{ad}.ohcproj", "BENİOKU.md"],
        giris: "uygulama.ohc",
    },
    Sablon {
        kimlik: "oyun",
        ad: "2B Oyun",
        aciklama: "Klavye, fare ve dokunmayla oynanan bir oyun: çizim, hareket, puan ve ses.",
        simge: "sports_esports",
        kategoriler: &["Oyun", "Masaüstü"],
        etiketler: &["Orhunca", "WebAssembly", "Oyun", "Telefon"],
        yakinda: None,
        dosyalar: &["oyun.ohc", "{ad}.ohcproj", "BENİOKU.md"],
        giris: "oyun.ohc",
    },
];

/// Türkçe arayüz diliyle yazılan (WebAssembly'ye derlenen) proje şablonu mu?
pub fn arayuz_mu(s: &Sablon) -> bool {
    matches!(s.kimlik, "arayuz" | "oyun")
}

pub fn bul(kimlik: &str) -> Option<&'static Sablon> {
    SABLONLAR.iter().find(|s| s.kimlik == kimlik)
}

/// Şablonun bir dosyasının içeriği. `ornek` kapalıysa en sade hâli üretilir.
pub fn icerik(sablon: &Sablon, dosya: &str, ad: &str, ornek: bool) -> String {
    if dosya == "{ad}.ohcproj" {
        return format!(
            "ad = \"{ad}\"\nsürüm = \"0.1.0\"\ngiriş = \"{}\"\nşablon = \"{}\"\n",
            sablon.giris, sablon.kimlik
        );
    }
    if dosya == "BENİOKU.md" && arayuz_mu(sablon) {
        return format!(
            "# {ad}\n\nTürkçe arayüz diliyle yazılmış bir Orhunca uygulaması. Program WebAssembly'ye \
             derlenir ve tarayıcıda çalışır.\n\n\
             ## Çalıştırma\n\n```\norhunca çalıştır\n```\n\n\
             Uygulama tarayıcıda açılır. Orhunca Stüdyo'da **F5** uygulamayı canlı önizlemede \
             gösterir; kaydettiğinizde yenilenir. Tek dosyalık bir sayfa üretmek için:\n\n\
             ```\norhunca derle uygulama.ohc --hedef web\n```\n\n\
             ## Arayüz dili\n\n\
             - `durum ad = değer` — uygulamanın değişkenleri; bir olaydan sonra arayüz yeniden çizilir\n\
             - `arayüz:` — ekranda görünenler: `başlık(...)`, `yazı(...)`, `düğme(...)`, `giriş(...)`, \
             `satır:`, `sütun:`, `kart:` …\n\
             - `düğme(\"Ekle\") tıklanınca:` — olay bloğu\n\
             - `bileşen Ad(...):` — arayüzün yeniden kullanılan parçaları\n"
        );
    }
    if dosya == "BENİOKU.md" && web_mi(sablon) {
        return format!(
            "# {ad}\n\n{} şablonuyla oluşturulmuş bir Orhunca web projesi.\n\n\
             ## Çalıştırma\n\n```\norhunca çalıştır\n```\n\n\
             Sonra tarayıcıda http://localhost:3000 adresini açın. Orhunca Stüdyo'da **F5** \
             sunucuyu başlatır ve sayfayı canlı önizlemede gösterir; kaydettiğinizde yenilenir.\n\n\
             ## Klasörler\n\n\
             - `sunucu.ohc` — yollar: `al \"/adres\":` ve `gönder \"/adres\":`\n\
             - `görünümler/` — `.ohchtml` sayfaları (`görünüm(\"ad\", değer)` ile kullanılır)\n\
             - `statik/` — olduğu gibi sunulan dosyalar (CSS, resim, JavaScript)\n\
             - `veri/` — `kaydet` ile saklanan model kayıtları (JSON)\n",
            sablon.ad
        );
    }
    if dosya == "BENİOKU.md" {
        return format!(
            "# {ad}\n\n{} şablonuyla oluşturulmuş bir Orhunca projesi.\n\n\
             ## Çalıştırma\n\n```\norhunca çalıştır\n```\n\n\
             Orhunca Stüdyo'da **F5** tuşu da projeyi derleyip çalıştırır.\n",
            sablon.ad
        );
    }
    match (sablon.kimlik, dosya, ornek) {
        ("oyun", "oyun.ohc", true) => {
            include_str!("../../örnekler/oyunlar/top_yakala.ohc").replace("Top Yakala", ad)
        }
        ("oyun", "oyun.ohc", false) => format!(
            "# {ad}: ok tuşlarıyla daireyi hareket ettirin\n\
             durum x = 240.0\n\
             durum y = 160.0\n\
             \n\
             arayüz:\n    \
                 oyun_alanı(480, 320) her_karede:\n        \
                     eğer tuş_basılı(\"sol\") ise:\n            \
                         x -= 4\n        \
                     eğer tuş_basılı(\"sağ\") ise:\n            \
                         x += 4\n        \
                     eğer tuş_basılı(\"yukarı\") ise:\n            \
                         y -= 4\n        \
                     eğer tuş_basılı(\"aşağı\") ise:\n            \
                         y += 4\n        \
                     temizle(\"#101820\")\n        \
                     daire(x, y, 20, \"turuncu\")\n"
        ),
        ("konsol", "ana.ohc", true) => format!(
            "# {ad} — Orhunca konsol uygulaması\n\
             # Çalıştırmak için F5'e basın ya da terminalde: orhunca çalıştır\n\
             \n\
             \"Adınız nedir?\"'i yaz.\n\
             ad = kırp(oku())\n\
             eğer ad \"\"'ye eşitse:\n    \
                 ad = \"dünya\"\n\
             \n\
             her i için 1'den 3'e kadar:\n    \
                 \"Merhaba \" + ad + \"! \" + i'yi yaz.\n"
        ),
        ("konsol", "ana.ohc", false) => "\"Merhaba, dünya!\"'yı ekrana yaz.\n".to_string(),
        ("arayuz", "uygulama.ohc", true) => format!(
            "# {ad} — Türkçe arayüz diliyle yapılacaklar listesi
# Çalıştırmak için F5'e basın: uygulama canlı önizlemede açılır.

model İş:
    ad: metin
    bitti: mantık

durum yeni_iş = \"\"
durum işler: liste<İş> = []
durum süzgeç = \"Hepsi\"

işlev iş_ekle():
    ad = kırp(yeni_iş)
    eğer ad \"\"'ye eşit değilse:
        İş(ad: ad, bitti: yanlış)'ı işler'e ekle.
        yeni_iş = \"\"

işlev kalan_sayısı() -> sayı:
    n = 0
    her iş için işler'den:
        eğer değil iş.bitti ise:
            n += 1
    döndür n

bileşen İş_Satırı(iş: İş, sıra: sayı):
    kart(iç_boşluk: 10):
        satır:
            onay_kutusu(iş.bitti, iş.ad)
            boşluk()
            düğme(\"Sil\", arka: \"kırmızı\") tıklanınca:
                sil(işler, sıra)

arayüz:
    başlık(\"Yapılacaklar\")
    satır:
        giriş(yeni_iş, \"Ne yapılacak?\") gönderilince:
            iş_ekle()
        düğme(\"Ekle\", etkin: kırp(yeni_iş) != \"\") tıklanınca:
            iş_ekle()
    seçim(süzgeç, [\"Hepsi\", \"Kalanlar\", \"Bitenler\"])
    her i için 0'dan uzunluk(işler) - 1'e kadar:
        iş = işler[i]
        göster = süzgeç == \"Hepsi\" veya (süzgeç == \"Kalanlar\" ve değil iş.bitti) veya (süzgeç == \"Bitenler\" ve iş.bitti)
        eğer göster ise:
            İş_Satırı(iş, i)
    eğer uzunluk(işler) 0'a eşitse:
        yazı(\"Henüz iş yok. Yukarıya yazıp Enter'a basın.\", renk: \"gri\")
    değilse:
        yazı(kalan_sayısı() + \" iş kaldı\", kalın: doğru)
"
        ),
        ("arayuz", "uygulama.ohc", false) => "\
durum sayaç = 0

arayüz:
    başlık(\"Sayaç: \" + sayaç)
    düğme(\"Artır\") tıklanınca:
        sayaç += 1
"
        .to_string(),
        ("sayi_tahmin", "oyun.ohc", true) => "\
# Sayı tahmin oyunu: bilgisayar 1 ile 100 arasında bir sayı tutar.
sabit EN_KÜÇÜK = 1
sabit EN_BÜYÜK = 100

gizli = rastgele(EN_KÜÇÜK, EN_BÜYÜK)
deneme = 0
bulundu = yanlış

\"1 ile 100 arasında bir sayı tuttum. Tahmin et!\"'i yaz.
değil bulundu olduğu sürece:
    girdi = kırp(oku())
    eğer değil sayı_mı(girdi) ise:
        \"Lütfen bir sayı yazın.\"'ı yaz.
        sürdür
    tahmin = sayı(girdi)
    deneme += 1
    eğer tahmin gizli'den küçükse:
        \"Daha büyük bir sayı dene.\"'yi yaz.
    değilse eğer tahmin gizli'den büyükse:
        \"Daha küçük bir sayı dene.\"'yi yaz.
    değilse:
        bulundu = doğru

\"Tebrikler! \" + deneme + \" denemede buldun.\"'u yaz.
"
        .to_string(),
        ("sayi_tahmin", "oyun.ohc", false) => "\
gizli = rastgele(1, 100)
\"Tuttuğum sayı: \" + gizli'yi yaz.
"
        .to_string(),
        ("kutuphane", "kutuphane.ohc", true) => format!(
            "# {ad} kütüphanesi: başka projeler `kullan \"kutuphane.ohc\"` ile kullanabilir.\n\
             sabit SÜRÜM = \"0.1.0\"\n\
             \n\
             fiil sayı'yı karele:\n    \
                 döndür sayı * sayı\n\
             \n\
             işlev ortalama(sayılar: liste<sayı>) -> ondalık:\n    \
                 eğer uzunluk(sayılar) 0'a eşitse:\n        \
                     döndür 0.0\n    \
                 döndür toplam(sayılar) / uzunluk(sayılar)\n"
        ),
        ("kutuphane", "kutuphane.ohc", false) => {
            "fiil sayı'yı karele:\n    döndür sayı * sayı\n".to_string()
        }
        ("kutuphane", "testler/kutuphane_testi.ohc", true) => "\
kullan \"../kutuphane.ohc\"

fiil (ad: metin)'i (koşul: mantık)'la doğrula:
    eğer koşul ise:
        \"✓ \" + ad'ı yaz.
    değilse:
        \"✗ \" + ad'ı yaz.

kare = 4'ü karele
\"karele\"'yi (kare == 16)'yla doğrula.
\"ortalama\"'yı (ortalama([2, 4, 6]) == 4.0)'la doğrula.
\"boş liste\"'yi (ortalama([]) == 0.0)'la doğrula.
"
        .to_string(),
        ("kutuphane", "testler/kutuphane_testi.ohc", false) => {
            "kullan \"../kutuphane.ohc\"\n\n(4'ü karele)'yi yaz.\n".to_string()
        }
        (kimlik, _, _) => {
            let sade = format!("{kimlik}.sade/{dosya}");
            let tam = format!("{kimlik}/{dosya}");
            let icerik = (!ornek)
                .then(|| gomulu_dosya(&sade))
                .flatten()
                .or_else(|| gomulu_dosya(&tam))
                .unwrap_or_default();
            icerik.replace("«ad»", ad)
        }
    }
}
