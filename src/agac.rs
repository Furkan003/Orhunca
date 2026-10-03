//! Söz dizimi ağacı (AST).

use crate::hata::Konum;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tip {
    Sayi,
    Metin,
    Mantik,
    Liste(Box<Tip>),
    /// Değer döndürmeyen işlev.
    Bos,
    /// Henüz bilinmiyor (ör. boş liste `[]`).
    Bilinmeyen,
}

impl Tip {
    /// Çalışma zamanının yazdırma/sıralama için kullandığı tip kodu.
    pub fn kod(&self) -> i64 {
        match self {
            Tip::Metin => 1,
            Tip::Mantik => 2,
            Tip::Liste(t) => 3 + 4 * t.kod(),
            _ => 0,
        }
    }

    /// `liste<?>` ile `liste<sayı>` gibi tipleri birleştirir.
    pub fn birlestir(&self, diger: &Tip) -> Option<Tip> {
        match (self, diger) {
            (Tip::Bilinmeyen, t) | (t, Tip::Bilinmeyen) => Some(t.clone()),
            (Tip::Liste(a), Tip::Liste(b)) => a.birlestir(b).map(|t| Tip::Liste(Box::new(t))),
            (a, b) if a == b => Some(a.clone()),
            _ => None,
        }
    }
}

impl fmt::Display for Tip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tip::Sayi => write!(f, "sayı"),
            Tip::Metin => write!(f, "metin"),
            Tip::Mantik => write!(f, "mantık"),
            Tip::Liste(t) => write!(f, "liste<{t}>"),
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
    Bol,
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
    Metin(String),
    Mantik(bool),
    Isim(String),
    Liste(Vec<Ifade>),
    Ikili(IkiliOp, Box<Ifade>, Box<Ifade>),
    Tekli(TekliOp, Box<Ifade>),
    Cagri(String, Vec<Ifade>),
    Indeks(Box<Ifade>, Box<Ifade>),
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
    Yaz(Ifade),
    Ekle {
        oge: Ifade,
        liste: Ifade,
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
    /// `None`: dönüş tipi gövdeden çıkarılır.
    pub donus: Option<Tip>,
    pub govde: Vec<Deyim>,
    pub konum: Konum,
    /// Tip denetçisi doldurur: parametreler dahil tüm yerel değişkenler.
    pub yereller: Vec<(String, Tip)>,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub islevler: Vec<Islev>,
    pub ana: Vec<Deyim>,
    pub ana_yereller: Vec<(String, Tip)>,
}
