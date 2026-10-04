//! Yeni proje şablonları. `hazir: false` olanlar henüz desteklenmeyen proje
//! türleridir (web, masaüstü); Stüdyo onları "yakında" olarak gösterir.

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
        yakinda: Some("Aşama 6"),
        dosyalar: &["index.ohchtml", "stil.css", "{ad}.ohcproj"],
        giris: "index.ohchtml",
    },
    Sablon {
        kimlik: "web_sitesi",
        ad: "Web Sitesi",
        aciklama: "Çok sayfalı, yönlendirmeli statik web sitesi.",
        simge: "language",
        kategoriler: &["Web"],
        etiketler: &["Orhunca", "Web", "Statik"],
        yakinda: Some("Aşama 6"),
        dosyalar: &[
            "sayfalar/anasayfa.ohchtml",
            "sayfalar/hakkinda.ohchtml",
            "sayfalar/iletisim.ohchtml",
            "bilesenler/ustbilgi.ohchtml",
            "stiller/ana.css",
            "varliklar/logo.svg",
            "{ad}.ohcproj",
            "BENİOKU.md",
        ],
        giris: "sayfalar/anasayfa.ohchtml",
    },
    Sablon {
        kimlik: "web_uyg",
        ad: "Web Uygulaması",
        aciklama: "Bileşen tabanlı, etkileşimli tek sayfa uygulaması.",
        simge: "web",
        kategoriler: &["Web"],
        etiketler: &["Orhunca", "Web", "Bileşenler"],
        yakinda: Some("Aşama 6"),
        dosyalar: &[
            "kaynak/uygulama.ohc",
            "kaynak/bilesenler/sayac.ohc",
            "kaynak/bilesenler/liste.ohc",
            "kaynak/durum.ohc",
            "stiller/tema.css",
            "{ad}.ohcproj",
        ],
        giris: "kaynak/uygulama.ohc",
    },
    Sablon {
        kimlik: "acilis",
        ad: "Açılış Sayfası",
        aciklama: "Hazır bölümlerle ürün veya etkinlik tanıtım sayfası.",
        simge: "rocket_launch",
        kategoriler: &["Web"],
        etiketler: &["Orhunca", "Web", "Pazarlama"],
        yakinda: Some("Aşama 6"),
        dosyalar: &[
            "sayfa.ohchtml",
            "bolumler/kahraman.ohchtml",
            "bolumler/ozellikler.ohchtml",
            "bolumler/iletisim_formu.ohchtml",
            "stiller/ana.css",
            "{ad}.ohcproj",
        ],
        giris: "sayfa.ohchtml",
    },
    Sablon {
        kimlik: "web_api",
        ad: "Web API",
        aciklama: "REST uç noktaları sunan sunucu uygulaması.",
        simge: "dns",
        kategoriler: &["Web", "Sunucu"],
        etiketler: &["Orhunca", "Sunucu", "REST"],
        yakinda: Some("Aşama 6"),
        dosyalar: &[
            "sunucu.ohc",
            "yollar/hava.ohc",
            "yollar/kullanicilar.ohc",
            "veri/ornek.json",
            "{ad}.ohcproj",
        ],
        giris: "sunucu.ohc",
    },
    Sablon {
        kimlik: "tam_yigin",
        ad: "Tam Yığın Uygulama",
        aciklama: "Ön yüz, sunucu ve veritabanı tek projede.",
        simge: "stacks",
        kategoriler: &["Web", "Sunucu"],
        etiketler: &["Orhunca", "Web", "Sunucu", "Veritabanı"],
        yakinda: Some("Aşama 6"),
        dosyalar: &[
            "istemci/anasayfa.ohchtml",
            "istemci/bilesenler/kart.ohchtml",
            "sunucu/sunucu.ohc",
            "sunucu/yollar/urunler.ohc",
            "veritabani/sema.ohc",
            "{ad}.ohcproj",
        ],
        giris: "sunucu/sunucu.ohc",
    },
    Sablon {
        kimlik: "masaustu",
        ad: "Masaüstü Uygulaması",
        aciklama: "Pencere tabanlı masaüstü uygulaması oluşturun.",
        simge: "desktop_windows",
        kategoriler: &["Masaüstü"],
        etiketler: &["Orhunca", "Windows", "Masaüstü"],
        yakinda: Some("Aşama 8"),
        dosyalar: &["pencere.ohc", "arayuz/ana_ekran.ohc", "{ad}.ohcproj"],
        giris: "pencere.ohc",
    },
];

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
    if dosya == "BENİOKU.md" {
        return format!(
            "# {ad}\n\n{} şablonuyla oluşturulmuş bir Orhunca projesi.\n\n\
             ## Çalıştırma\n\n```\norhunca çalıştır\n```\n\n\
             Orhunca Stüdyo'da **F5** tuşu da projeyi derleyip çalıştırır.\n",
            sablon.ad
        );
    }
    match (sablon.kimlik, dosya, ornek) {
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
        _ => String::new(),
    }
}
