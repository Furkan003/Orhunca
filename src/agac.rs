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
    /// Seçenek türü (numaralandırma): `seçenek Renk: kırmızı, yeşil, mavi`.
    /// Çalışma zamanında değer, seçeneğin adıdır (metin).
    Secenek(String),
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
            Tip::Metin | Tip::Secenek(_) => 1,
            Tip::Mantik => 2,
            Tip::Ondalik => 3,
            Tip::Liste(t) => 4 + 8 * t.kod(),
            Tip::Sozluk(a, d) => 5 + 8 * (a.metin_gibi() as i64 + 2 * d.kod()),
            Tip::Model(_) => 6,
            _ => 0,
        }
    }

    /// Tipin içindeki (liste öğesi, sözlük değeri olarak) modelin adı.
    pub fn ic_model(&self) -> Option<&str> {
        match self {
            Tip::Model(m) => Some(m),
            Tip::Liste(t) => t.ic_model(),
            Tip::Sozluk(a, d) => a.ic_model().or(d.ic_model()),
            _ => None,
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

    /// Çalışma zamanında metin olarak taşınan tip mi (metin ya da seçenek)?
    pub fn metin_gibi(&self) -> bool {
        matches!(self, Tip::Metin | Tip::Secenek(_))
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
            Tip::Model(ad) | Tip::Secenek(ad) => write!(f, "{ad}"),
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
    /// Adsız işlev: `işlev(x) -> x * 2`. Yalnızca `süz`, `dönüştür`, `sırala`, `biri_mi`,
    /// `hepsi_mi` çağrılarında kullanılır; denetçiden önce `adsiz::indir` bu çağrıları
    /// üretilmiş işlevlere çevirir (bkz. src/adsiz.rs).
    Adsiz(Vec<String>, Box<Ifade>),
}

#[derive(Debug, Clone)]
pub enum Deyim {
    Atama {
        hedef: String,
        /// Tipi yazılmış tanım: `liste: liste<metin> = []`
        tip: Option<Tip>,
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
    /// Arayüz öğesi (yalnızca `arayüz:` ve `bileşen` gövdelerinde): `düğme("Artır") tıklanınca:`
    Oge(Box<Oge>),
    /// `dene:` ... `yakala hata:` ...: gövdede çalışma hatası olursa yakala bloğu
    /// çalışır; `degisken` hatanın mesajını (metin) alır.
    Dene {
        govde: Vec<Deyim>,
        degisken: Option<String>,
        yakala: Vec<Deyim>,
        konum: Konum,
    },
}

impl Deyim {
    /// Deyimin kaynaktaki yeri (hata ayıklayıcının durduğu satır).
    pub fn konum(&self) -> Option<crate::hata::Konum> {
        Some(match self {
            Deyim::Atama { konum, .. }
            | Deyim::AlanAtama { konum, .. }
            | Deyim::HerAralik { konum, .. }
            | Deyim::HerListe { konum, .. }
            | Deyim::Dene { konum, .. }
            | Deyim::Dondur(_, konum)
            | Deyim::Dur(konum)
            | Deyim::Surdur(konum) => *konum,
            Deyim::IndeksAtama { liste: e, .. }
            | Deyim::Yaz(e)
            | Deyim::Ekle { oge: e, .. }
            | Deyim::Cikar { oge: e, .. }
            | Deyim::DosyayaYaz { deger: e, .. }
            | Deyim::Sirala(e)
            | Deyim::Eger { kosul: e, .. }
            | Deyim::Surece { kosul: e, .. }
            | Deyim::IfadeDeyimi(e) => e.konum,
            Deyim::Oge(_) => return None,
        })
    }
}

/// Bir gövdede adı geçen değişkenler (okunan, yazılan, döngü ve hata
/// değişkenleri); iç içe bloklar dahil, ilk geçiş sırasıyla.
pub fn gecen_adlar(govde: &[Deyim]) -> Vec<String> {
    fn ekle(adlar: &mut Vec<String>, ad: &str) {
        if !adlar.iter().any(|a| a == ad) {
            adlar.push(ad.to_string());
        }
    }
    fn ifade(e: &Ifade, adlar: &mut Vec<String>) {
        match &e.tur {
            IfadeTuru::Isim(ad) => ekle(adlar, ad),
            IfadeTuru::Liste(l) | IfadeTuru::Cagri(_, l) => l.iter().for_each(|x| ifade(x, adlar)),
            IfadeTuru::Sozluk(c) => c.iter().for_each(|(a, d)| {
                ifade(a, adlar);
                ifade(d, adlar)
            }),
            IfadeTuru::Ikili(_, a, b) | IfadeTuru::Indeks(a, b) => {
                ifade(a, adlar);
                ifade(b, adlar)
            }
            IfadeTuru::Tekli(_, a) | IfadeTuru::Alan(a, ..) => ifade(a, adlar),
            IfadeTuru::FiilCagri(_, l) => l.iter().for_each(|(_, x)| ifade(x, adlar)),
            IfadeTuru::Kurucu(_, l) => l.iter().for_each(|(_, x)| ifade(x, adlar)),
            IfadeTuru::Metod(a, _, l) => {
                ifade(a, adlar);
                l.iter().for_each(|x| ifade(x, adlar))
            }
            IfadeTuru::Adsiz(p, g) => {
                let mut ic = Vec::new();
                ifade(g, &mut ic);
                for a in ic.iter().filter(|a| !p.contains(a)) {
                    ekle(adlar, a);
                }
            }
            IfadeTuru::Sayi(_)
            | IfadeTuru::Ondalik(_)
            | IfadeTuru::Metin(_)
            | IfadeTuru::Mantik(_)
            | IfadeTuru::ModelAdi(_) => {}
        }
    }
    fn blok(govde: &[Deyim], adlar: &mut Vec<String>) {
        for d in govde {
            match d {
                Deyim::Atama { hedef, deger, .. } => {
                    ifade(deger, adlar);
                    ekle(adlar, hedef)
                }
                Deyim::IndeksAtama {
                    liste,
                    indeks,
                    deger,
                } => [liste, indeks, deger].iter().for_each(|x| ifade(x, adlar)),
                Deyim::AlanAtama { nesne, deger, .. } => {
                    ifade(nesne, adlar);
                    ifade(deger, adlar)
                }
                Deyim::Yaz(e) | Deyim::Sirala(e) | Deyim::IfadeDeyimi(e) => ifade(e, adlar),
                Deyim::Ekle { oge, liste } | Deyim::Cikar { oge, liste } => {
                    ifade(oge, adlar);
                    ifade(liste, adlar)
                }
                Deyim::DosyayaYaz { deger, yol } => {
                    ifade(deger, adlar);
                    ifade(yol, adlar)
                }
                Deyim::Eger {
                    kosul,
                    govde,
                    degilse,
                } => {
                    ifade(kosul, adlar);
                    blok(govde, adlar);
                    blok(degilse, adlar)
                }
                Deyim::Surece { kosul, govde } => {
                    ifade(kosul, adlar);
                    blok(govde, adlar)
                }
                Deyim::HerAralik {
                    degisken,
                    bas,
                    son,
                    govde,
                    ..
                } => {
                    ifade(bas, adlar);
                    ifade(son, adlar);
                    ekle(adlar, degisken);
                    blok(govde, adlar)
                }
                Deyim::HerListe {
                    degisken,
                    liste,
                    govde,
                    ..
                } => {
                    ifade(liste, adlar);
                    ekle(adlar, degisken);
                    blok(govde, adlar)
                }
                Deyim::Dondur(e, _) => e.iter().for_each(|x| ifade(x, adlar)),
                Deyim::Dur(_) | Deyim::Surdur(_) => {}
                Deyim::Oge(o) => {
                    o.argumanlar.iter().for_each(|x| ifade(x, adlar));
                    o.secenekler.iter().for_each(|(_, x)| ifade(x, adlar));
                    blok(&o.cocuklar, adlar);
                    for olay in o.olay.iter().chain(&o.baglama) {
                        for (ad, _) in &olay.yakalananlar {
                            ekle(adlar, ad);
                        }
                    }
                }
                Deyim::Dene {
                    govde,
                    degisken,
                    yakala,
                    ..
                } => {
                    blok(govde, adlar);
                    if let Some(d) = degisken {
                        ekle(adlar, d);
                    }
                    blok(yakala, adlar)
                }
            }
        }
    }
    let mut adlar = Vec::new();
    blok(govde, &mut adlar);
    adlar
}

/// Gövdedeki her ifadeyi (alt ifadeler dahil, içten dışa) gezer.
pub fn ifadeleri_gez(govde: &mut [Deyim], f: &mut dyn FnMut(&mut Ifade)) {
    fn ifade(e: &mut Ifade, f: &mut dyn FnMut(&mut Ifade)) {
        match &mut e.tur {
            IfadeTuru::Liste(l) | IfadeTuru::Cagri(_, l) => l.iter_mut().for_each(|x| ifade(x, f)),
            IfadeTuru::Sozluk(c) => c.iter_mut().for_each(|(a, d)| {
                ifade(a, f);
                ifade(d, f)
            }),
            IfadeTuru::Ikili(_, a, b) | IfadeTuru::Indeks(a, b) => {
                ifade(a, f);
                ifade(b, f)
            }
            IfadeTuru::Tekli(_, a) | IfadeTuru::Alan(a, ..) => ifade(a, f),
            IfadeTuru::FiilCagri(_, l) => l.iter_mut().for_each(|(_, x)| ifade(x, f)),
            IfadeTuru::Kurucu(_, l) => l.iter_mut().for_each(|(_, x)| ifade(x, f)),
            IfadeTuru::Metod(a, _, l) => {
                ifade(a, f);
                l.iter_mut().for_each(|x| ifade(x, f))
            }
            _ => {}
        }
        f(e);
    }
    for d in govde {
        match d {
            Deyim::Atama { deger, .. }
            | Deyim::Yaz(deger)
            | Deyim::Sirala(deger)
            | Deyim::IfadeDeyimi(deger) => ifade(deger, f),
            Deyim::IndeksAtama {
                liste,
                indeks,
                deger,
            } => {
                ifade(liste, f);
                ifade(indeks, f);
                ifade(deger, f)
            }
            Deyim::AlanAtama { nesne, deger, .. } => {
                ifade(nesne, f);
                ifade(deger, f)
            }
            Deyim::Ekle { oge, liste } | Deyim::Cikar { oge, liste } => {
                ifade(oge, f);
                ifade(liste, f)
            }
            Deyim::DosyayaYaz { deger, yol } => {
                ifade(deger, f);
                ifade(yol, f)
            }
            Deyim::Eger {
                kosul,
                govde,
                degilse,
            } => {
                ifade(kosul, f);
                ifadeleri_gez(govde, f);
                ifadeleri_gez(degilse, f)
            }
            Deyim::Surece { kosul, govde } => {
                ifade(kosul, f);
                ifadeleri_gez(govde, f)
            }
            Deyim::HerAralik {
                bas, son, govde, ..
            } => {
                ifade(bas, f);
                ifade(son, f);
                ifadeleri_gez(govde, f)
            }
            Deyim::HerListe { liste, govde, .. } => {
                ifade(liste, f);
                ifadeleri_gez(govde, f)
            }
            Deyim::Dondur(e, _) => {
                if let Some(e) = e {
                    ifade(e, f)
                }
            }
            Deyim::Dur(_) | Deyim::Surdur(_) => {}
            Deyim::Oge(o) => {
                o.argumanlar.iter_mut().for_each(|x| ifade(x, f));
                o.secenekler.iter_mut().for_each(|(_, x)| ifade(x, f));
                ifadeleri_gez(&mut o.cocuklar, f);
                if let Some(olay) = &mut o.olay {
                    ifadeleri_gez(&mut olay.govde, f);
                }
            }
            Deyim::Dene { govde, yakala, .. } => {
                ifadeleri_gez(govde, f);
                ifadeleri_gez(yakala, f)
            }
        }
    }
}

/// Gövdede (iç içe bloklar dahil) bir `dene:` bloğu var mı?
pub fn dene_var(govde: &[Deyim]) -> bool {
    govde.iter().any(|d| match d {
        Deyim::Dene { .. } => true,
        Deyim::Eger { govde, degilse, .. } => dene_var(govde) || dene_var(degilse),
        Deyim::Surece { govde, .. }
        | Deyim::HerAralik { govde, .. }
        | Deyim::HerListe { govde, .. } => dene_var(govde),
        Deyim::Oge(o) => {
            dene_var(&o.cocuklar) || o.olay.iter().chain(&o.baglama).any(|x| dene_var(&x.govde))
        }
        _ => false,
    })
}

/// Bir arayüz öğesi: `yazı("Merhaba", renk: "mavi")`, `satır:` + çocuklar,
/// `düğme("Artır") tıklanınca:` + olay bloğu, `giriş(ad)` (durum değişkenine bağlı).
#[derive(Debug, Clone)]
pub struct Oge {
    /// `düğme`, `satır`, `giriş` ...
    pub ad: String,
    pub argumanlar: Vec<Ifade>,
    /// Adlı seçenekler: `renk: "mavi"`, `boyut: 24`
    pub secenekler: Vec<(String, Ifade)>,
    /// Kapsayıcı öğelerin (`satır:`, `kart:`) içindeki öğeler
    pub cocuklar: Vec<Deyim>,
    pub olay: Option<Olay>,
    /// Denetçi doldurur: değeri bir durum değişkenine bağlanan öğelerde (giriş,
    /// onay kutusu, seçim, kaydırıcı) kullanıcı değeri değiştirince çalışan blok:
    /// `hedef = ‹değer›` ve varsa `değişince:` bloğu.
    pub baglama: Option<Olay>,
    pub konum: Konum,
}

/// Olay bloğu: öğe çizilirken çevredeki yerel değişkenlerin o anki değerleri
/// yakalanır; olay olunca blok bu değerlerle çalışır.
#[derive(Debug, Clone)]
pub struct Olay {
    /// `tıklanınca`, `değişince`, `gönderilince`, `çalınca`
    pub ad: String,
    pub govde: Vec<Deyim>,
    /// Denetçi doldurur: bloğun kullandığı, çevreleyen işlevin yerel değişkenleri.
    pub yakalananlar: Vec<(String, Tip)>,
    /// Denetçi doldurur: bloğun kendi yerel değişkenleri.
    pub yereller: Vec<(String, Tip)>,
    pub konum: Konum,
}

/// `durum sayaç = 0`: arayüz programlarında programın her yerinden görülen ve
/// değiştirilebilen değişken. Bir olaydan sonra arayüz yeniden çizilir.
#[derive(Debug, Clone)]
pub struct Durum {
    pub ad: String,
    pub tip: Option<Tip>,
    pub deger: Ifade,
    pub konum: Konum,
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
    /// `arayüz:` bloğu ya da `bileşen`: gövdesinde arayüz öğeleri olabilir.
    pub arayuz: bool,
    /// `kütüphane "m":` bloğundaki C işlevi: gövdesi yoktur, çağrılınca C kütüphanesindeki
    /// aynı adlı işlev çağrılır.
    pub dis: Option<DisIslev>,
}

/// C kütüphanesindeki bir işlevin bilgileri (FFI).
#[derive(Debug, Clone)]
pub struct DisIslev {
    /// `kütüphane "m":` → "m" (libm); tam dosya adı da olabilir ("libcurl.so.4").
    pub kutuphane: String,
    /// Parametrelerin C karşılıkları
    pub tipler: Vec<CTip>,
    pub donus: CTip,
}

/// Orhunca tipinin C'deki karşılığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CTip {
    /// `sayı`: int64_t (long long)
    Sayi,
    /// `sayı32`: int (32 bit)
    Sayi32,
    /// `ondalık`: double
    Ondalik,
    /// `mantık`: bool
    Mantik,
    /// `metin`: const char * (UTF-8, NUL ile biter)
    Metin,
    /// dönüş yok (void)
    Yok,
}

impl CTip {
    pub fn adi(self) -> &'static str {
        match self {
            CTip::Sayi => "sayı",
            CTip::Sayi32 => "sayı32",
            CTip::Ondalik => "ondalık",
            CTip::Mantik => "mantık",
            CTip::Metin => "metin",
            CTip::Yok => "yok",
        }
    }

    pub fn orhunca(self) -> Tip {
        match self {
            CTip::Sayi | CTip::Sayi32 => Tip::Sayi,
            CTip::Ondalik => Tip::Ondalik,
            CTip::Mantik => Tip::Mantik,
            CTip::Metin => Tip::Metin,
            CTip::Yok => Tip::Bos,
        }
    }
}

/// Programın arayüzünü çizen işlevin adı (`arayüz:` bloğu).
pub const ARAYUZ_ISLEVI: &str = "arayüz";

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
    /// Seçenek türündeki alanın geçerli değerleri (denetçi doldurur)
    pub secenekler: Vec<String>,
    /// Alan (ya da liste/sözlük öğesi) bir modelse onun adı (denetçi doldurur)
    pub ic_model: Option<String>,
    /// Model alanı kendi modeline geri dönen bir zincirde: varsayılanı boş (denetçi doldurur)
    pub dongusel: bool,
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
            // Boş argümanlı kurucu: modelin varsayılan nesnesi (kod üretici kurar)
            Tip::Model(m) if !self.dongusel => IfadeTuru::Kurucu(m.clone(), Vec::new()),
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
    /// Kurallar bit alanıdır: 1 zorunlu, 2 e-posta biçimi. Seçenek alanlarında
    /// yedinci sütun geçerli değerlerdir (`|` ile ayrılmış).
    pub fn tanim_metni(&self) -> String {
        let mut s = self.ad.clone();
        s.push('\n');
        let sinir = |x: Option<f64>| x.map(|x| x.to_string()).unwrap_or_default();
        for a in &self.alanlar {
            s.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                a.ad,
                a.tip.kod(),
                a.zorunlu as u8 | (a.e_posta as u8) << 1,
                sinir(a.en_az),
                sinir(a.en_fazla),
                a.etiket.as_deref().unwrap_or_default()
            ));
            if !a.secenekler.is_empty() {
                s.push('\t');
                s.push_str(&a.secenekler.join("|"));
            } else if let Some(m) = &a.ic_model {
                s.push_str("\t@");
                s.push_str(m);
            }
            s.push('\n');
        }
        s
    }
}

/// `seçenek Renk: kırmızı, yeşil, mavi`
#[derive(Debug, Clone)]
pub struct SecenekTanimi {
    pub ad: String,
    pub degerler: Vec<String>,
    pub konum: Konum,
}

/// Seçenek türüne çevirme işlevinin (denetçinin ürettiği) adı:
/// `Renk("mavi")` → `‹seçenek›("mavi", "kırmızı|yeşil|mavi", "Renk")`.
pub const SECENEK_CEVIR: &str = "‹seçenek›";

/// Model işlevlerinde nesnenin kendisi: `bu.ad`. Model işlevi `Kitap.özet` adlı ve ilk
/// parametresi `bu` olan sıradan bir işlevdir; `k.özet()` çağrısı `Kitap.özet(k)` olur.
pub const MODEL_NESNESI: &str = "bu";

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub islevler: Vec<Islev>,
    pub modeller: Vec<Model>,
    pub secenekler: Vec<SecenekTanimi>,
    /// `sabit PI = 3.14159`: her yerden görülebilen değişmez değerler.
    pub sabitler: Vec<(String, Ifade)>,
    /// `durum sayaç = 0`: arayüz programlarının değişkenleri.
    pub durumlar: Vec<Durum>,
    pub ana: Vec<Deyim>,
    pub ana_yereller: Vec<(String, Tip)>,
    /// Derlemedeki kaynak dosyaların yolları (`Konum::dosya` sırasıyla).
    pub dosyalar: Vec<String>,
}

impl Program {
    /// Arayüz programı mı (`arayüz:` bloğu ya da `durum` değişkenleri var)?
    /// Böyle programlar yalnızca WebAssembly'ye (tarayıcı) derlenir.
    pub fn arayuz_programi(&self) -> bool {
        !self.durumlar.is_empty() || self.islevler.iter().any(|f| f.arayuz)
    }
}
