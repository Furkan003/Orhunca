//! Hâl ekleri ve sözlük temelli ek çözümleme.
//!
//! Derleyici genel bir Türkçe biçimbilim çözümlemesi yapmaz: programda tanımlı
//! isimleri bilir. Bir kelimenin başındaki tanımlı ismi bulur, kalanı ek
//! tablosunda arar. Ünlü uyumu hoşgörüyle karşılanır (`5'e` de `5'a` da kabul
//! edilir); birden çok çözüm varsa tahmin yürütülmez, hata verilir.

use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Hal {
    /// -(y)I: işlem yapılan nesne → `5'i`
    Belirtme,
    /// -(y)A: hedef → `sayılara`
    Yonelme,
    /// -DAn: kaynak → `sayılardan`
    Ayrilma,
    /// -DA: yer → `listede`
    Bulunma,
    /// -(y)lA: araç → `x'le`
    Vasita,
    /// -(n)In: sahiplik → `listenin`
    Ilgi,
}

impl Hal {
    pub fn adi(self) -> &'static str {
        match self {
            Hal::Belirtme => "belirtme (-i)",
            Hal::Yonelme => "yönelme (-e)",
            Hal::Ayrilma => "ayrılma (-den)",
            Hal::Bulunma => "bulunma (-de)",
            Hal::Vasita => "vasıta (-le)",
            Hal::Ilgi => "ilgi (-in)",
        }
    }
}

/// Soyut ek tablosundan (`-(y)I`, `-(y)A`, `-DAn` ...) üretilmiş tüm biçimler.
/// Ünlü uyumu bilerek denetlenmez; biçimlendirici ileride düzeltecek.
pub fn hal_bul(ek: &str) -> Option<Hal> {
    let h = match ek {
        "ı" | "i" | "u" | "ü" | "yı" | "yi" | "yu" | "yü" | "nı" | "ni" | "nu" | "nü" => {
            Hal::Belirtme
        }
        "a" | "e" | "ya" | "ye" | "na" | "ne" => Hal::Yonelme,
        "dan" | "den" | "tan" | "ten" | "ndan" | "nden" => Hal::Ayrilma,
        "da" | "de" | "ta" | "te" | "nda" | "nde" => Hal::Bulunma,
        "la" | "le" | "yla" | "yle" => Hal::Vasita,
        "ın" | "in" | "un" | "ün" | "nın" | "nin" | "nun" | "nün" => Hal::Ilgi,
        _ => return None,
    };
    Some(h)
}

/// Ünlüyle başlayan ek aldığında son hecesindeki ünlüyü yitiren sık kelimeler
/// (ünlü düşmesi): `isim` → `ismi`, `metin` → `metni`, `şehir` → `şehre`.
const UNLU_DUSMESI: &[(&str, &str)] = &[
    ("isim", "ism"),
    ("metin", "metn"),
    ("resim", "resm"),
    ("şehir", "şehr"),
    ("fikir", "fikr"),
    ("akıl", "akl"),
    ("burun", "burn"),
    ("ağız", "ağz"),
    ("oğul", "oğl"),
    ("alın", "aln"),
    ("ömür", "ömr"),
    ("kayıt", "kayd"),
    ("hüküm", "hükm"),
    ("beyin", "beyn"),
    ("gönül", "gönl"),
    ("boyun", "boyn"),
    ("karın", "karn"),
    ("sabır", "sabr"),
    ("zihin", "zihn"),
    ("nehir", "nehr"),
    ("emir", "emr"),
    ("asır", "asr"),
    ("göğüs", "göğs"),
    ("vakit", "vakt"),
    ("devir", "devr"),
    ("nesil", "nesl"),
    ("zehir", "zehr"),
    ("keyif", "keyf"),
    ("kısım", "kısm"),
    ("şekil", "şekl"),
    ("cisim", "cism"),
    ("ilim", "ilm"),
];

/// Bir ismin ek aldığında görülebilecek gövdeleri (ünsüz yumuşaması, ünlü düşmesi).
/// `kitap` → `kitab`, `renk` → `reng`, `çocuk` → `çocuğ`, `ağaç` → `ağac`,
/// `kanat` → `kanad`, `isim` → `ism`, `ürün_ismi` için `ürün_isim` → `ürün_ism`
fn govdeler(isim: &str) -> Vec<String> {
    let mut v = vec![isim.to_string()];
    for (tam, dusmus) in UNLU_DUSMESI {
        if let Some(on) = isim.strip_suffix(tam) {
            if on.is_empty() || on.ends_with('_') {
                v.push(format!("{on}{dusmus}"));
            }
        }
    }
    let mut k: Vec<char> = isim.chars().collect();
    if k.len() >= 2 {
        let son = k.len() - 1;
        let yumusak = match k[son] {
            'p' => Some('b'),
            'ç' => Some('c'),
            't' => Some('d'),
            'k' if k[son - 1] == 'n' => Some('g'),
            'k' => Some('ğ'),
            _ => None,
        };
        if let Some(y) = yumusak {
            k[son] = y;
            v.push(k.into_iter().collect());
        }
    }
    v
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cozum {
    /// Kelime eksiz bir isim.
    Isim(String),
    /// İsim + hâl eki.
    EkliIsim(String, Hal),
    /// Birden çok olası okuma var.
    Belirsiz(Vec<String>),
    Bilinmiyor,
}

#[derive(Debug, Default, Clone)]
pub struct Sozluk {
    /// Görülebilecek gövde → asıl isim
    govdeler: HashMap<String, String>,
    isimler: BTreeSet<String>,
}

impl Sozluk {
    pub fn ekle(&mut self, isim: &str) {
        self.isimler.insert(isim.to_string());
        for g in govdeler(isim) {
            self.govdeler.entry(g).or_insert_with(|| isim.to_string());
        }
    }

    /// Tanımlı isimler (öneriler için).
    pub fn isimler(&self) -> impl Iterator<Item = &str> {
        self.isimler.iter().map(String::as_str)
    }

    /// Kesme işaretiyle yazılmış bir kökün asıl ismini verir (`kitab'ı` → `kitap`).
    pub fn asil_isim(&self, kok: &str) -> Option<&str> {
        if self.isimler.contains(kok) {
            return Some(self.isimler.get(kok).unwrap());
        }
        self.govdeler.get(kok).map(|s| s.as_str())
    }

    /// Kesme işareti olmadan yazılmış bir kelimeyi çözer.
    pub fn cozumle(&self, kelime: &str) -> Cozum {
        let mut adaylar: Vec<(String, Option<Hal>)> = Vec::new();
        if self.isimler.contains(kelime) {
            adaylar.push((kelime.to_string(), None));
        }
        for (i, _) in kelime.char_indices().skip(1) {
            let (kok, ek) = kelime.split_at(i);
            if let (Some(asil), Some(hal)) = (self.govdeler.get(kok), hal_bul(ek)) {
                let aday = (asil.clone(), Some(hal));
                if !adaylar.contains(&aday) {
                    adaylar.push(aday);
                }
            }
        }
        match adaylar.len() {
            0 => Cozum::Bilinmiyor,
            1 => {
                let (isim, hal) = adaylar.pop().unwrap();
                match hal {
                    None => Cozum::Isim(isim),
                    Some(h) => Cozum::EkliIsim(isim, h),
                }
            }
            _ => Cozum::Belirsiz(
                adaylar
                    .into_iter()
                    .map(|(isim, hal)| match hal {
                        None => format!("'{isim}' ismi"),
                        Some(_) => {
                            let ek = &kelime[govde_uzunlugu(kelime, &isim)..];
                            format!("{isim}'{ek} ({})", hal.unwrap().adi())
                        }
                    })
                    .collect(),
            ),
        }
    }
}

/// Kelimede asıl ismin (ya da yumuşamış gövdesinin) bayt uzunluğu.
fn govde_uzunlugu(kelime: &str, isim: &str) -> usize {
    govdeler(isim)
        .into_iter()
        .filter(|g| kelime.starts_with(g.as_str()))
        .map(|g| g.len())
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn sozluk(isimler: &[&str]) -> Sozluk {
        let mut s = Sozluk::default();
        for i in isimler {
            s.ekle(i);
        }
        s
    }

    #[test]
    fn hal_ekleri() {
        let s = sozluk(&["sayılar", "sayı"]);
        assert_eq!(
            s.cozumle("sayılara"),
            Cozum::EkliIsim("sayılar".into(), Hal::Yonelme)
        );
        assert_eq!(
            s.cozumle("sayılardan"),
            Cozum::EkliIsim("sayılar".into(), Hal::Ayrilma)
        );
        assert_eq!(
            s.cozumle("sayıyı"),
            Cozum::EkliIsim("sayı".into(), Hal::Belirtme)
        );
        assert_eq!(s.cozumle("sayı"), Cozum::Isim("sayı".into()));
        assert_eq!(s.cozumle("masa"), Cozum::Bilinmiyor);
    }

    #[test]
    fn cogul_ve_iyelik_ismin_parcasi() {
        let s = sozluk(&["sayılarımız"]);
        assert_eq!(
            s.cozumle("sayılarımızdan"),
            Cozum::EkliIsim("sayılarımız".into(), Hal::Ayrilma)
        );
    }

    #[test]
    fn unsuz_yumusamasi() {
        let s = sozluk(&["kitap", "renk", "çocuk"]);
        assert_eq!(
            s.cozumle("kitabı"),
            Cozum::EkliIsim("kitap".into(), Hal::Belirtme)
        );
        assert_eq!(
            s.cozumle("rengi"),
            Cozum::EkliIsim("renk".into(), Hal::Belirtme)
        );
        assert_eq!(
            s.cozumle("çocuğa"),
            Cozum::EkliIsim("çocuk".into(), Hal::Yonelme)
        );
        assert_eq!(s.asil_isim("kitab"), Some("kitap"));
    }

    #[test]
    fn unlu_dusmesi() {
        let s = sozluk(&["isim", "metin", "ürün_şehir"]);
        assert_eq!(
            s.cozumle("ismi"),
            Cozum::EkliIsim("isim".into(), Hal::Belirtme)
        );
        assert_eq!(
            s.cozumle("metne"),
            Cozum::EkliIsim("metin".into(), Hal::Yonelme)
        );
        assert_eq!(s.cozumle("ürün_şehrinden"), Cozum::Bilinmiyor);
        assert_eq!(
            s.cozumle("ürün_şehri"),
            Cozum::EkliIsim("ürün_şehir".into(), Hal::Belirtme)
        );
        assert_eq!(s.asil_isim("ism"), Some("isim"));
    }

    #[test]
    fn belirsizlikte_tahmin_yok() {
        let s = sozluk(&["kitap", "kitapla"]);
        assert!(matches!(s.cozumle("kitapla"), Cozum::Belirsiz(_)));
    }

    #[test]
    fn hosgorulu_unlu_uyumu() {
        assert_eq!(hal_bul("a"), Some(Hal::Yonelme));
        assert_eq!(hal_bul("e"), Some(Hal::Yonelme));
        assert_eq!(hal_bul("ten"), Some(Hal::Ayrilma));
        assert_eq!(hal_bul("xyz"), None);
    }
}
