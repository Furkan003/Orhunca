//! Söz dizimi ağacı (AST).

use crate::ekler::Hal;
use crate::hata::Konum;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tip {
    Sayi,
    Ondalik,
    Metin,
    Mantik,
    Liste(Box<Tip>),
    /// Anahtarları sayı ya da metin olan eşleme: `{"elma": 5}`
    Sozluk(Box<Tip>, Box<Tip>),
    /// Kullanıcı tanımlı ya da yerleşik model: `Ürün`, `İstek`, `Yanıt`.
    Model(String),
    /// Değer döndürmeyen işlev.
    Bos,
    /// Henüz bilinmiyor (ör. boş liste `[]`).
    Bilinmeyen,
}

impl Tip {
    /// Çalışma zamanının yazdırma/sıralama için kullandığı tip kodu:
    /// 0 sayı, 1 metin, 2 mantık, 3 ondalık, 4 + 8*öğe liste,
    /// 5 + 8*(anahtar + 2*değer) sözlük (anahtar: 0 sayı, 1 metin), 6 model
    /// (modelin tanımı nesnenin kendisinde durur).
    pub fn kod(&self) -> i64 {
        match self {
            Tip::Metin => 1,
            Tip::Mantik => 2,
            Tip::Ondalik => 3,
            Tip::Liste(t) => 4 + 8 * t.kod(),
            Tip::Sozluk(a, d) => 5 + 8 * ((**a == Tip::Metin) as i64 + 2 * d.kod()),
            Tip::Model(_) => 6,
            _ => 0,
        }
    }

    /// Tipin içinde (liste öğesi, sözlük değeri olarak) bir model var mı?
    pub fn model_icerir(&self) -> bool {
        match self {
            Tip::Model(_) => true,
            Tip::Liste(t) => t.model_icerir(),
            Tip::Sozluk(a, d) => a.model_icerir() || d.model_icerir(),
            _ => false,
        }
    }

    /// Liste öğesinin ya da sözlük anahtarının tip kodu.
    pub fn ic_kod(&self) -> i64 {
        match self {
            Tip::Liste(t) | Tip::Sozluk(t, _) => t.kod(),
            _ => 0,
        }
    }

    /// Bu tipte bir yere `t` tipinde bir değer konabilir mi? Sayılar ondalığa
    /// kendiliğinden çevrilir.
    pub fn kabul_eder(&self, t: &Tip) -> bool {
        (*self == Tip::Ondalik && *t == Tip::Sayi) || self.birlestir(t).is_some()
    }

    pub fn sayisal(&self) -> bool {
        matches!(self, Tip::Sayi | Tip::Ondalik)
    }

    /// `liste<?>` ile `liste<sayı>` gibi tipleri birleştirir.
    pub fn birlestir(&self, diger: &Tip) -> Option<Tip> {
        match (self, diger) {
            (Tip::Bilinmeyen, t) | (t, Tip::Bilinmeyen) => Some(t.clone()),
            (Tip::Liste(a), Tip::Liste(b)) => a.birlestir(b).map(|t| Tip::Liste(Box::new(t))),
            (Tip::Sozluk(a, b), Tip::Sozluk(c, d)) => Some(Tip::Sozluk(
                Box::new(a.birlestir(c)?),
                Box::new(b.birlestir(d)?),
            )),
            (a, b) if a == b => Some(a.clone()),
            _ => None,
        }
    }
}

impl fmt::Display for Tip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tip::Sayi => write!(f, "sayı"),
            Tip::Ondalik => write!(f, "ondalık"),
            Tip::Metin => write!(f, "metin"),
            Tip::Mantik => write!(f, "mantık"),
            Tip::Liste(t) => write!(f, "liste<{t}>"),
            Tip::Sozluk(a, d) => write!(f, "sözlük<{a}, {d}>"),
            Tip::Model(ad) => write!(f, "{ad}"),
            Tip::Bos => write!(f, "boş"),
            Tip::Bilinmeyen => write!(f, "?"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IkiliOp {
    Topla,
    Cikar,
    Carp,
    /// `/`: her zaman ondalık sonuç verir (7 / 2 = 3.5).
    Bol,
    /// `//`: tam bölme (7 // 2 = 3).
    TamBol,
    Mod,
    Esit,
    EsitDegil,
    Kucuk,
    Buyuk,
    KucukEsit,
    BuyukEsit,
    Ve,
    Veya,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TekliOp {
    Eksi,
    Degil,
}

#[derive(Debug, Clone)]
pub struct Ifade {
    pub tur: IfadeTuru,
    pub konum: Konum,
    /// Tip denetçisi tarafından doldurulur.
    pub tip: Tip,
}

impl Ifade {
    pub fn yeni(tur: IfadeTuru, konum: Konum) -> Self {
        Ifade {
            tur,
            konum,
            tip: Tip::Bilinmeyen,
        }
    }
}

#[derive(Debug, Clone)]
pub enum IfadeTuru {
    Sayi(i64),
    Ondalik(f64),
    Metin(String),
    Mantik(bool),
    Isim(String),
    Liste(Vec<Ifade>),
    Sozluk(Vec<(Ifade, Ifade)>),
    Ikili(IkiliOp, Box<Ifade>, Box<Ifade>),
    Tekli(TekliOp, Box<Ifade>),
    Cagri(String, Vec<Ifade>),
    /// Kullanıcı tanımlı fiil çağrısı: `5'i karele`. Bağımsız değişkenler hâl
    /// ekleriyle eşleştirilir; denetçi bunu sıralı bir `Cagri`ya çevirir.
    FiilCagri(String, Vec<(Hal, Ifade)>),
    Indeks(Box<Ifade>, Box<Ifade>),
    /// `Ürün(ad: "Kalem", fiyat: 12.5)`. Denetçiden sonra modelin tüm alanlarını
    /// tanım sırasıyla içerir (verilmeyenler varsayılan değerleriyle).
    Kurucu(String, Vec<(String, Ifade)>),
    /// `ürün.ad`. Üçüncü değer alanın sırasıdır (0: kimlik); denetçi doldurur.
    Alan(Box<Ifade>, String, usize),
    /// `ürün.kaydet()`, `Ürün.hepsi()`
    Metod(Box<Ifade>, String, Vec<Ifade>),
    /// Yalnızca yöntem çağrısının alıcısı olabilen model adı: `Ürün.hepsi()`.
    ModelAdi(String),
}

#[derive(Debug, Clone)]
pub enum Deyim {
    Atama {
        hedef: String,
        deger: Ifade,
        konum: Konum,
    },
    IndeksAtama {
        liste: Ifade,
        indeks: Ifade,
        deger: Ifade,
    },
    /// `ürün.fiyat = 12.5`; `sira` denetçi tarafından doldurulur.
    AlanAtama {
        nesne: Ifade,
        alan: String,
        sira: usize,
        deger: Ifade,
        konum: Konum,
    },
    Yaz(Ifade),
    Ekle {
        oge: Ifade,
        liste: Ifade,
    },
    /// `x'i listeden çıkar.`
    Cikar {
        oge: Ifade,
        liste: Ifade,
    },
    /// `metni "dosya.txt"'ye yaz.`
    DosyayaYaz {
        deger: Ifade,
        yol: Ifade,
    },
    Sirala(Ifade),
    Eger {
        kosul: Ifade,
        govde: Vec<Deyim>,
        degilse: Vec<Deyim>,
    },
    Surece {
        kosul: Ifade,
        govde: Vec<Deyim>,
    },
    HerAralik {
        degisken: String,
        bas: Ifade,
        son: Ifade,
        govde: Vec<Deyim>,
        konum: Konum,
    },
    HerListe {
        degisken: String,
        liste: Ifade,
        govde: Vec<Deyim>,
        konum: Konum,
    },
    Dondur(Option<Ifade>, Konum),
    Dur(Konum),
    Surdur(Konum),
    IfadeDeyimi(Ifade),
}

#[derive(Debug, Clone)]
pub struct Islev {
    pub ad: String,
    pub parametreler: Vec<(String, Tip)>,
    /// Fiillerde her parametrenin hâl eki (`fiil sayı'yı karele:`); işlevlerde boş.
    pub haller: Vec<Hal>,
    /// `None`: dönüş tipi gövdeden çıkarılır.
    pub donus: Option<Tip>,
    pub govde: Vec<Deyim>,
    pub konum: Konum,
    /// Tip denetçisi doldurur: parametreler dahil tüm yerel değişkenler.
    pub yereller: Vec<(String, Tip)>,
    /// Web yolu ise yöntemi ve kalıbı: `al "/ürünler/{kimlik: sayı}":`
    pub rota: Option<Rota>,
}

#[derive(Debug, Clone)]
pub struct Rota {
    /// GET, POST, PUT, DELETE
    pub yontem: String,
    /// Çalışma zamanının eşleştirdiği kalıp: `/ürünler/{kimlik:sayı}`
    pub kalip: String,
}

/// `model Ürün:` tanımı. İlk alan her zaman `kimlik: sayı`dır.
#[derive(Debug, Clone)]
pub struct Model {
    pub ad: String,
    pub alanlar: Vec<AlanTanimi>,
    pub konum: Konum,
}

#[derive(Debug, Clone)]
pub struct AlanTanimi {
    pub ad: String,
    pub tip: Tip,
    pub varsayilan: Option<Ifade>,
    pub zorunlu: bool,
    pub en_az: Option<f64>,
    pub en_fazla: Option<f64>,
    /// Hata mesajlarında alan adı yerine gösterilen ad: `etiket "E-posta"`
    pub etiket: Option<String>,
    /// Değerin biçimi: `e_posta`
    pub e_posta: bool,
    pub konum: Konum,
}

impl AlanTanimi {
    /// Alanın yeni bir nesnedeki ilk değeri: tanımdaki varsayılan ya da tipin sıfır değeri.
    pub fn ilk_deger(&self) -> Ifade {
        if let Some(v) = &self.varsayilan {
            return v.clone();
        }
        let tur = match &self.tip {
            Tip::Ondalik => IfadeTuru::Ondalik(0.0),
            Tip::Metin => IfadeTuru::Metin(String::new()),
            Tip::Mantik => IfadeTuru::Mantik(false),
            Tip::Liste(_) => IfadeTuru::Liste(Vec::new()),
            Tip::Sozluk(..) => IfadeTuru::Sozluk(Vec::new()),
            _ => IfadeTuru::Sayi(0),
        };
        Ifade {
            tur,
            konum: self.konum,
            tip: self.tip.clone(),
        }
    }
}

impl Model {
    pub fn alan(&self, ad: &str) -> Option<(usize, &AlanTanimi)> {
        self.alanlar.iter().enumerate().find(|(_, a)| a.ad == ad)
    }

    /// Çalışma zamanının okuduğu tanım metni: ilk satırda modelin adı, sonra her
    /// alan için `ad<TAB>tip kodu<TAB>kurallar<TAB>en az<TAB>en fazla<TAB>etiket`.
    /// Kurallar bit alanıdır: 1 zorunlu, 2 e-posta biçimi.
    pub fn tanim_metni(&self) -> String {
        let mut s = self.ad.clone();
        s.push('\n');
        let sinir = |x: Option<f64>| x.map(|x| x.to_string()).unwrap_or_default();
        for a in &self.alanlar {
            s.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                a.ad,
                a.tip.kod(),
                a.zorunlu as u8 | (a.e_posta as u8) << 1,
                sinir(a.en_az),
                sinir(a.en_fazla),
                a.etiket.as_deref().unwrap_or_default()
            ));
        }
        s
    }
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub islevler: Vec<Islev>,
    pub modeller: Vec<Model>,
    /// `sabit PI = 3.14159`: her yerden görülebilen değişmez değerler.
    pub sabitler: Vec<(String, Ifade)>,
    pub ana: Vec<Deyim>,
    pub ana_yereller: Vec<(String, Tip)>,
}
