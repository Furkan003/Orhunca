//! Ayrıştırıcı: sözcükleri söz dizimi ağacına çevirir.
//!
//! Türkçe cümle yapısı: bağımsız değişkenler hâl ekleriyle işaretlenir, fiil sona
//! gelir. `5'i sayılara ekle.` ile `sayılara 5'i ekle.` aynı anlamdadır.

use crate::agac::*;
use crate::ekler::{hal_bul, Cozum, Hal, Sozluk};
use crate::hata::{Hata, Konum, Sonuc};
use crate::sablon::{Parca, Sablon};
use crate::sozcuk::{sozcukle, Sozcuk, Tok};
use std::collections::HashSet;
use std::rc::Rc;

/// İsim olarak kullanılamayan kelimeler.
const AYRILMIS: &[&str] = &[
    "eğer",
    "değilse",
    "ise",
    "her",
    "için",
    "kadar",
    "işlev",
    "kullan",
    "sabit",
    "fiil",
    "döndür",
    "dur",
    "sürdür",
    "ve",
    "veya",
    "değil",
    "doğru",
    "yanlış",
    "olduğu",
    "sürece",
    "iken",
    "yaz",
    "ekle",
    "sırala",
    "çıkar",
    "ekran",
    "ekrana",
    "uzunluğu",
];

const FIILLER: &[&str] = &["yaz", "ekle", "sırala", "çıkar"];

/// Model nesneleri için fiiller: `ürün'ü kaydet.`, `ürün'ü sil.` Ayrılmış değildir;
/// yalnızca cümlenin sonunda fiil sayılırlar (`sil(liste, 0)` bir işlev çağrısıdır).
const MODEL_FIILLERI: &[&str] = &["kaydet", "sil"];

/// Web yolu tanımlayan kelimeler ve HTTP yöntemleri: `al "/ürünler":`
const ROTA_YONTEMLERI: &[(&str, &str)] = &[
    ("al", "GET"),
    ("gönder", "POST"),
    ("koy", "PUT"),
    ("sil", "DELETE"),
];

/// Yerleşik modellerin tanımlandığı sanal dosyanın sırası.
pub const YERLESIK_DOSYA: usize = usize::MAX;

/// Web çatısının yerleşik modelleri; her programa eklenir. Çalışma zamanı
/// alanları adlarıyla bulur.
const YERLESIK_MODELLER: &str = "\
model İstek:
    yöntem: metin
    yol: metin
    sorgu: sözlük<metin, metin>
    form: sözlük<metin, metin>
    parametreler: sözlük<metin, metin>
    gövde: metin
    başlıklar: sözlük<metin, metin>
    çerezler: sözlük<metin, metin>
    oturum: sözlük<metin, metin>
    dosyalar: sözlük<metin, YüklenenDosya>

model Yanıt:
    durum: sayı = 200
    tür: metin = \"text/html; charset=utf-8\"
    gövde: metin
    konum: metin
    başlıklar: sözlük<metin, metin>
    çerezler: sözlük<metin, metin>

model YüklenenDosya:
    ad: metin
    tür: metin
    yol: metin
    boyut: sayı
";

/// Görünüm işlevlerinde çıktı parçalarının toplandığı liste (kullanıcı bu adı yazamaz).
const CIKTI: &str = "@çıktı";

/// Yerleşik işlevler: `uzunluk(x)`, `metin(x)`, `sayı(x)`, `ondalık(x)`, `yuvarla(x)`, `oku()`.
pub const YERLESIK: &[&str] = &["uzunluk", "metin", "sayı", "ondalık", "yuvarla", "oku"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum YuklemKoku {
    Buyuk,
    Kucuk,
    Esit,
    Degil,
}

/// `büyükse`, `küçükken`, `eşit` gibi yüklem kelimelerini ayırır.
fn yuklem(kelime: &str) -> Option<(YuklemKoku, &str)> {
    for (kok, tur) in [
        ("büyük", YuklemKoku::Buyuk),
        ("küçük", YuklemKoku::Kucuk),
        ("eşit", YuklemKoku::Esit),
        ("değil", YuklemKoku::Degil),
    ] {
        if let Some(ek) = kelime.strip_prefix(kok) {
            if matches!(ek, "" | "se" | "sa" | "ken") {
                return Some((tur, ek));
            }
        }
    }
    None
}

/// `uzunluğu`, `uzunluğunu`, `uzunluğuna` ...: eki varsa onu da döndürür.
fn uzunluk_kelimesi(k: &str) -> Option<Option<Hal>> {
    let ek = k.strip_prefix("uzunluğu")?;
    if ek.is_empty() {
        return Some(None);
    }
    hal_bul(ek).filter(|_| ek.starts_with('n')).map(Some)
}

fn ayrilmis_mi(k: &str) -> bool {
    AYRILMIS.contains(&k) || yuklem(k).is_some() || uzunluk_kelimesi(k).is_some()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KosulTuru {
    Eger,
    Surece,
}

/// Tüm dosyalarda ortak tanımlar.
#[derive(Default)]
struct Tanimlar {
    /// Tanımlı isimler (değişkenler, parametreler, işlevler): ek çözümlemesi için.
    sozluk: Sozluk,
    /// Kullanıcının `fiil` ile tanımladığı fiiller.
    fiiller: HashSet<String>,
    /// `model` ile tanımlanan (ve yerleşik) modellerin adları.
    modeller: HashSet<String>,
    /// Tüm modellerin alan adları (ve seçenek değerleri): `ürün.fiyatı` gibi
    /// ekli yazımı çözmek için.
    alanlar: Sozluk,
    /// `seçenek Renk: ...` ile tanımlanan seçenek türleri ve değerleri.
    secenekler: std::collections::HashMap<String, Vec<String>>,
}

pub struct Ayristirici {
    sozcukler: Vec<Sozcuk>,
    poz: usize,
    t: Rc<Tanimlar>,
    /// İfade içinde yakalanan hâl eki: eki taşıyan öğe ifadeyi bitirir ve ek
    /// tüm ifadeye ait sayılır (`a + b'yi yaz` → `(a + b)'yi yaz`).
    yakalanan: Option<(Hal, Konum)>,
    /// `arayüz:` ya da `bileşen` gövdesi ayrıştırılıyor: arayüz öğeleri yazılabilir.
    arayuzde: bool,
}

#[cfg(test)]
pub fn ayristir(sozcukler: Vec<Sozcuk>) -> Sonuc<Program> {
    ayristir_cok(vec![sozcukler], Vec::new())
}

/// İlk dosya ana programdır; diğerleri `kullan` ile eklenen kütüphanelerdir ve
/// yalnızca tanım (işlev, fiil, sabit, model, web yolu) içerebilir. İsim sözlüğü
/// ve fiiller tüm dosyalarda ortaktır. Görünümler (`.ohchtml`) işlevlere çevrilir.
pub fn ayristir_cok(mut dosyalar: Vec<Vec<Sozcuk>>, sablonlar: Vec<Sablon>) -> Sonuc<Program> {
    let mut yerlesik = sozcukle(YERLESIK_MODELLER).expect("yerleşik modeller");
    for s in yerlesik.iter_mut() {
        s.konum.dosya = YERLESIK_DOSYA;
    }
    dosyalar.push(yerlesik);
    let mut hepsi: Vec<Sozcuk> = dosyalar.iter().flatten().cloned().collect();
    for s in &sablonlar {
        hepsi.extend(s.tum_sozcukler());
    }
    derinlik_denetle(&hepsi)?;
    let mut t = sozluk_kur(&hepsi);
    if !sablonlar.is_empty() {
        for ad in ["model", "içerik", "başlık"] {
            t.sozluk.ekle(ad);
        }
        for s in &sablonlar {
            for ad in s.parametre_adlari() {
                t.sozluk.ekle(&ad);
            }
        }
    }
    let t = Rc::new(t);
    let mut program = Program::default();
    for (i, sozcukler) in dosyalar.into_iter().enumerate() {
        let mut a = Ayristirici::yeni(&t, sozcukler);
        a.program(&mut program, i > 0)?;
    }
    for s in sablonlar {
        program.islevler.push(sablon_islevi(&t, s)?);
    }
    Ok(program)
}

/// İç içe parantez/köşeli parantez derinliği, blok derinliği ve tek deyimdeki işleç
/// sayısı için üst sınırlar: ayrıştırıcı, denetçi ve kod üreticiler özyinelemelidir;
/// aşırı derin girdi yığını taşırıp derleyiciyi (ve onu barındıran Stüdyo'yu) düşürmesin.
pub const EN_COK_PARANTEZ: usize = 128;
pub const EN_COK_BLOK: usize = 64;
pub const EN_COK_ISLEC: usize = 1000;

fn derinlik_denetle(sozcukler: &[Sozcuk]) -> Sonuc<()> {
    let mut parantez = 0usize;
    let mut blok = 0usize;
    let mut islec = 0usize;
    for s in sozcukler {
        match &s.tok {
            Tok::Op("(" | "[" | "{") => {
                parantez += 1;
                if parantez > EN_COK_PARANTEZ {
                    return Err(Hata::yeni(
                        s.konum,
                        format!("çok fazla iç içe parantez (en çok {EN_COK_PARANTEZ})"),
                    )
                    .ipucu("ifadeyi ara değişkenlere bölün"));
                }
            }
            Tok::Op(")" | "]" | "}") => parantez = parantez.saturating_sub(1),
            Tok::Op("+" | "-" | "*" | "/" | "//" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">=") => {
                islec += 1
            }
            Tok::Kelime(k) if k == "ve" || k == "veya" || k == "değil" => islec += 1,
            Tok::Girinti => {
                blok += 1;
                islec = 0;
                if blok > EN_COK_BLOK {
                    return Err(Hata::yeni(
                        s.konum,
                        format!("çok fazla iç içe blok (en çok {EN_COK_BLOK})"),
                    )
                    .ipucu("iç kısımları ayrı işlevlere taşıyın"));
                }
            }
            Tok::Cikinti => {
                blok = blok.saturating_sub(1);
                islec = 0;
            }
            Tok::YeniSatir | Tok::Son => {
                islec = 0;
                parantez = 0;
            }
            _ => {}
        }
        if islec > EN_COK_ISLEC {
            return Err(Hata::yeni(
                s.konum,
                format!("ifade çok uzun (bir deyimde en çok {EN_COK_ISLEC} işlem)"),
            )
            .ipucu("ifadeyi birkaç satıra ve ara değişkenlere bölün"));
        }
    }
    Ok(())
}

/// Bir dosyadaki `kullan "yol.ohc"` satırlarının yollarını verir.
pub fn kullanilanlar(sozcukler: &[Sozcuk]) -> Vec<(String, Konum)> {
    sozcukler
        .windows(2)
        .filter_map(|w| match (&w[0].tok, &w[1].tok) {
            (Tok::Kelime(k), Tok::Metin(m)) if k == "kullan" => Some((m.clone(), w[1].konum)),
            _ => None,
        })
        .collect()
}

/// Programdaki tüm tanımlı isimleri (atama hedefleri, döngü değişkenleri,
/// işlevler ve parametreleri) toplayarak ek çözümleme sözlüğünü kurar. Kullanıcı
/// fiillerini, modelleri ve model alanlarını da toplar.
fn sozluk_kur(s: &[Sozcuk]) -> Tanimlar {
    let mut t = Tanimlar::default();
    let sozluk = &mut t.sozluk;
    sozluk.ekle("pi");
    t.alanlar.ekle("kimlik");
    let fiiller = &mut t.fiiller;
    let kelime = |i: usize| match s.get(i).map(|s| &s.tok) {
        Some(Tok::Kelime(k)) if !ayrilmis_mi(k) => Some(k.as_str()),
        _ => None,
    };
    let satir_basi = |i: usize| {
        i == 0
            || matches!(
                s[i - 1].tok,
                Tok::YeniSatir | Tok::Girinti | Tok::Cikinti | Tok::Son
            )
    };
    // Model bloğunun içinde miyiz (girinti derinliği)?
    let mut model_derinligi = 0usize;
    for i in 0..s.len() {
        match s[i].tok {
            Tok::Girinti if model_derinligi > 0 => model_derinligi += 1,
            // Model bloğunun bittiği çıkıntı
            Tok::Cikinti if model_derinligi == 2 => model_derinligi = 0,
            Tok::Cikinti if model_derinligi > 2 => model_derinligi -= 1,
            Tok::Son => model_derinligi = 0,
            _ => {}
        }
        // `seçenek Renk: kırmızı, yeşil` → tür adı ve değerleri (değerler ekli yazılabilir)
        if satir_basi(i) && s[i].tok == Tok::Kelime("seçenek".into()) {
            if let (Some(ad), Some(Tok::Op(":"))) = (kelime(i + 1), s.get(i + 2).map(|s| &s.tok)) {
                let mut degerler = Vec::new();
                // Tek satırda (`: a, b`) ya da alt satırlardaki blokta
                let blok = matches!(s.get(i + 3).map(|s| &s.tok), Some(Tok::YeniSatir));
                let mut derinlik = 0;
                for x in &s[i + 3..] {
                    match &x.tok {
                        Tok::Girinti => derinlik += 1,
                        Tok::Cikinti if derinlik <= 1 => break,
                        Tok::Cikinti => derinlik -= 1,
                        Tok::YeniSatir if !blok => break,
                        Tok::Son => break,
                        Tok::Kelime(k) => {
                            t.alanlar.ekle(k);
                            degerler.push(k.clone());
                        }
                        _ => {}
                    }
                }
                t.secenekler.insert(ad.to_string(), degerler);
                continue;
            }
        }
        if satir_basi(i) && s[i].tok == Tok::Kelime("model".into()) {
            if let (Some(ad), Some(Tok::Op(":"))) = (kelime(i + 1), s.get(i + 2).map(|s| &s.tok)) {
                t.modeller.insert(ad.to_string());
                model_derinligi = 1;
                continue;
            }
        }
        if model_derinligi == 2 && satir_basi(i) {
            if let (Some(alan), Some(Tok::Op(":"))) = (kelime(i), s.get(i + 1).map(|s| &s.tok)) {
                t.alanlar.ekle(alan);
            }
            continue;
        }
        if model_derinligi > 1 {
            continue;
        }
        // Web yolu: `al "/ürünler/{kimlik: sayı}":` → istek ve kimlik tanımlıdır.
        if satir_basi(i) {
            if let (Tok::Kelime(k), Some(Tok::Metin(kalip)), Some(Tok::Op(":"))) = (
                &s[i].tok,
                s.get(i + 1).map(|s| &s.tok),
                s.get(i + 2).map(|s| &s.tok),
            ) {
                if ROTA_YONTEMLERI.iter().any(|(y, _)| y == k) {
                    sozluk.ekle("istek");
                    if let Ok((_, parametreler)) = rota_kalibi(kalip, s[i + 1].konum) {
                        for (p, _) in parametreler {
                            sozluk.ekle(&p);
                        }
                    }
                }
            }
        }
        // `işler: liste<metin> = []`: tipi yazılmış değişken tanımı
        if satir_basi(i) && s.get(i + 1).map(|s| &s.tok) == Some(&Tok::Op(":")) {
            if let (Some(k), Some(Tok::Kelime(_))) = (kelime(i), s.get(i + 2).map(|s| &s.tok)) {
                let satirda_esittir = s[i + 2..]
                    .iter()
                    .take_while(|t| !matches!(t.tok, Tok::YeniSatir | Tok::Son))
                    .any(|t| t.tok == Tok::Op("="));
                if satirda_esittir {
                    sozluk.ekle(k);
                }
            }
        }
        let onceki_her = i > 0 && s[i - 1].tok == Tok::Kelime("her".into());
        if let Some(k) = kelime(i) {
            let sonraki = s.get(i + 1).map(|s| &s.tok);
            // `stok: sayı = 0` (alan varsayılanı) bir atama değildir.
            let tip_sonrasi = i > 0 && s[i - 1].tok == Tok::Op(":");
            if (matches!(sonraki, Some(Tok::Op("=" | "+=" | "-="))) && !tip_sonrasi) || onceki_her {
                sozluk.ekle(k);
            }
        }
        // `yakala hata:` → hata değişkeni
        if satir_basi(i) && s[i].tok == Tok::Kelime("yakala".into()) {
            if let (Some(k), Some(Tok::Op(":"))) = (kelime(i + 1), s.get(i + 2).map(|s| &s.tok)) {
                sozluk.ekle(k);
            }
        }
        if s[i].tok == Tok::Kelime("sabit".into()) || s[i].tok == Tok::Kelime("durum".into()) {
            if let Some(k) = kelime(i + 1) {
                sozluk.ekle(k);
            }
        }
        if s[i].tok == Tok::Kelime("işlev".into()) || s[i].tok == Tok::Kelime("bileşen".into()) {
            if let Some(ad) = kelime(i + 1) {
                sozluk.ekle(ad);
            }
            // Parametreler: parantez içinde virgülle ayrılmış parçaların ilk kelimesi.
            let mut j = i + 2;
            let mut parca_basi = true;
            if s.get(j).map(|s| &s.tok) == Some(&Tok::Op("(")) {
                j += 1;
                while let Some(t) = s.get(j) {
                    match &t.tok {
                        Tok::Op(")") | Tok::YeniSatir | Tok::Son => break,
                        Tok::Op(",") => parca_basi = true,
                        Tok::Kelime(_) if parca_basi => {
                            if let Some(k) = kelime(j) {
                                sozluk.ekle(k);
                            }
                            parca_basi = false;
                        }
                        _ => parca_basi = false,
                    }
                    j += 1;
                }
            }
        }
        // `fiil sayı'yı (b: ondalık)'ya böl -> ondalık:`
        // Ekli kelimeler ve parantez içindeki ilk kelimeler parametre, ekiz son kelime fiil adıdır.
        if s[i].tok == Tok::Kelime("fiil".into()) {
            let mut j = i + 1;
            let mut derinlik = 0;
            let mut ad = None;
            while let Some(t) = s.get(j) {
                match &t.tok {
                    Tok::Op("(") => derinlik += 1,
                    Tok::Op(")") => derinlik -= 1,
                    Tok::Op(":") if derinlik == 0 => break,
                    Tok::Op("->") | Tok::YeniSatir | Tok::Son => break,
                    Tok::Kelime(_) => {
                        if let Some(k) = kelime(j) {
                            let parantez_basi = s[j - 1].tok == Tok::Op("(");
                            let ekli = matches!(s.get(j + 1).map(|s| &s.tok), Some(Tok::Ek(_)));
                            if parantez_basi || (derinlik == 0 && ekli) {
                                sozluk.ekle(k);
                            } else if derinlik == 0 {
                                ad = Some(k.to_string());
                            }
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if let Some(ad) = ad {
                fiiller.insert(ad);
            }
        }
    }
    t
}

/// `"/ürünler/{kimlik: sayı}"` → (`/ürünler/{kimlik:sayı}`, [(kimlik, sayı)])
fn rota_kalibi(kalip: &str, konum: Konum) -> Sonuc<(String, Vec<(String, Tip)>)> {
    if !kalip.starts_with('/') {
        return Err(Hata::yeni(konum, "web yolu '/' ile başlamalı").ipucu("al \"/ürünler\":"));
    }
    let mut parametreler: Vec<(String, Tip)> = Vec::new();
    let mut parcalar = Vec::new();
    for parca in kalip.split('/').skip(1) {
        if let Some(ic) = parca.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
            let (ad, tip) = match ic.split_once(':') {
                Some((a, t)) => (a.trim(), t.trim()),
                None => (ic.trim(), "metin"),
            };
            let gecerli = ad
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
                && ad.chars().all(|c| c.is_alphanumeric() || c == '_');
            if !gecerli || ad == "istek" {
                return Err(Hata::yeni(
                    konum,
                    format!("'{ad}' geçerli bir yol parametresi adı değil"),
                ));
            }
            let tip = match tip {
                "metin" => Tip::Metin,
                "sayı" => Tip::Sayi,
                t => {
                    return Err(Hata::yeni(
                        konum,
                        format!("yol parametresinin tipi metin ya da sayı olmalı, '{t}' bulundu"),
                    ))
                }
            };
            if parametreler.iter().any(|(p, _)| p == ad) {
                return Err(Hata::yeni(
                    konum,
                    format!("'{ad}' parametresi yolda iki kez geçiyor"),
                ));
            }
            parcalar.push(if tip == Tip::Sayi {
                format!("{{{ad}:sayı}}")
            } else {
                format!("{{{ad}}}")
            });
            parametreler.push((ad.to_string(), tip));
        } else if parca.contains(['{', '}']) {
            return Err(
                Hata::yeni(konum, "yol parametresi yolun bütün bir parçası olmalı")
                    .ipucu("al \"/ürünler/{kimlik: sayı}\":"),
            );
        } else {
            parcalar.push(parca.to_string());
        }
    }
    while parcalar.len() > 1 && parcalar.last().is_some_and(|p| p.is_empty()) {
        parcalar.pop();
    }
    Ok((format!("/{}", parcalar.join("/")), parametreler))
}

impl Ayristirici {
    fn yeni(t: &Rc<Tanimlar>, sozcukler: Vec<Sozcuk>) -> Self {
        Ayristirici {
            sozcukler,
            poz: 0,
            t: Rc::clone(t),
            yakalanan: None,
            arayuzde: false,
        }
    }

    // ---------- yardımcılar ----------

    fn bak(&self) -> &Tok {
        &self.sozcukler[self.poz].tok
    }

    fn bak_n(&self, n: usize) -> &Tok {
        let i = (self.poz + n).min(self.sozcukler.len() - 1);
        &self.sozcukler[i].tok
    }

    fn konum(&self) -> Konum {
        self.sozcukler[self.poz].konum
    }

    fn ilerle(&mut self) -> Sozcuk {
        let s = self.sozcukler[self.poz].clone();
        if self.poz < self.sozcukler.len() - 1 {
            self.poz += 1;
        }
        s
    }

    fn op_mu(&self, op: &str) -> bool {
        matches!(self.bak(), Tok::Op(o) if *o == op)
    }

    fn kelime_mi(&self, k: &str) -> bool {
        matches!(self.bak(), Tok::Kelime(w) if w == k)
    }

    fn bekle_op(&mut self, op: &str, ne: &str) -> Sonuc<()> {
        if self.op_mu(op) {
            self.ilerle();
            Ok(())
        } else {
            Err(self.beklenmeyen(&format!("'{op}' ({ne})")))
        }
    }

    fn bekle_kelime(&mut self, k: &str) -> Sonuc<()> {
        if self.kelime_mi(k) {
            self.ilerle();
            Ok(())
        } else {
            Err(self.beklenmeyen(&format!("'{k}'")))
        }
    }

    fn beklenmeyen(&self, beklenen: &str) -> Hata {
        Hata::yeni(
            self.konum(),
            format!("{beklenen} bekleniyordu, {} bulundu", tok_adi(self.bak())),
        )
    }

    fn deyim_sonu_mu(&self) -> bool {
        matches!(self.bak(), Tok::YeniSatir | Tok::Son | Tok::Cikinti) || self.op_mu(".")
    }

    fn deyim_bitir(&mut self) -> Sonuc<()> {
        if self.op_mu(".") {
            self.ilerle();
        }
        match self.bak() {
            Tok::YeniSatir => {
                self.ilerle();
                Ok(())
            }
            Tok::Son | Tok::Cikinti => Ok(()),
            _ => Err(self.beklenmeyen("satır sonu")),
        }
    }

    // ---------- program ve bloklar ----------

    fn program(&mut self, p: &mut Program, kutuphane: bool) -> Sonuc<()> {
        while *self.bak() != Tok::Son {
            match self.bak() {
                Tok::YeniSatir => {
                    self.ilerle();
                }
                Tok::Girinti => return Err(Hata::yeni(self.konum(), "beklenmeyen girinti")),
                _ if self.kelime_mi("işlev") => p.islevler.push(self.islev()?),
                _ if self.kutuphane_basi_mi() => p.islevler.extend(self.kutuphane_blogu()?),
                _ if self.bilesen_basi_mi() => p.islevler.push(self.islev()?),
                _ if self.arayuz_basi_mi() && !kutuphane => {
                    let f = self.arayuz_blogu()?;
                    if p.islevler.iter().any(|x| x.ad == ARAYUZ_ISLEVI) {
                        return Err(Hata::yeni(f.konum, "programda yalnızca bir 'arayüz:' bloğu olabilir")
                            .ipucu("arayüzün parçalarını 'bileşen' olarak tanımlayıp çağırın"));
                    }
                    p.islevler.push(f);
                }
                _ if self.durum_basi_mi() => {
                    let d = self.durum()?;
                    if p.durumlar.iter().any(|x| x.ad == d.ad) {
                        return Err(Hata::yeni(
                            d.konum,
                            format!("'{}' durumu iki kez tanımlanmış", d.ad),
                        ));
                    }
                    p.durumlar.push(d);
                }
                _ if self.kelime_mi("fiil") => p.islevler.push(self.fiil_tanimi()?),
                _ if self.model_basi_mi() => {
                    let m = self.model_tanimi()?;
                    if p.modeller.iter().any(|x| x.ad == m.ad) {
                        return Err(Hata::yeni(
                            m.konum,
                            format!("'{}' modeli iki kez tanımlanmış", m.ad),
                        ));
                    }
                    p.modeller.push(m);
                }
                _ if self.rota_basi_mi() => p.islevler.push(self.rota()?),
                _ if self.secenek_basi_mi() => {
                    let t = self.secenek_tanimi()?;
                    if p.secenekler.iter().any(|x| x.ad == t.ad) {
                        return Err(Hata::yeni(
                            t.konum,
                            format!("'{}' seçenek türü iki kez tanımlanmış", t.ad),
                        ));
                    }
                    p.secenekler.push(t);
                }
                _ if self.kelime_mi("kullan") => {
                    self.ilerle();
                    if !matches!(self.bak(), Tok::Metin(_)) {
                        return Err(self.beklenmeyen("dosya yolu (ör. kullan \"araçlar.ohc\")"));
                    }
                    self.ilerle();
                    self.deyim_bitir()?;
                }
                _ if self.kelime_mi("sabit") => {
                    self.ilerle();
                    let konum = self.konum();
                    let ad = self.isim_adi("sabit adı")?;
                    self.bekle_op("=", "sabitin değeri")?;
                    let deger = self.duz_ifade()?;
                    self.deyim_bitir()?;
                    if p.sabitler.iter().any(|(a, _)| *a == ad) {
                        return Err(Hata::yeni(
                            konum,
                            format!("'{ad}' sabiti iki kez tanımlanmış"),
                        ));
                    }
                    p.sabitler.push((ad, deger));
                }
                _ if kutuphane => {
                    return Err(Hata::yeni(
                        self.konum(),
                        "kütüphane dosyalarında yalnızca tanımlar (işlev, fiil, sabit, model, seçenek, web yolu, bileşen, durum) olabilir",
                    ))
                }
                _ => p.ana.push(self.deyim()?),
            }
        }
        Ok(())
    }

    fn blok(&mut self) -> Sonuc<Vec<Deyim>> {
        self.bekle_op(":", "bloğun başında")?;
        if *self.bak() != Tok::YeniSatir {
            return Err(self.beklenmeyen("':' sonrasında yeni satır"));
        }
        self.ilerle();
        if *self.bak() != Tok::Girinti {
            return Err(Hata::yeni(
                self.konum(),
                "blok boş olamaz; içeriden başlayan satırlar bekleniyordu",
            ));
        }
        self.ilerle();
        let mut govde = Vec::new();
        while !matches!(self.bak(), Tok::Cikinti | Tok::Son) {
            if self.kelime_mi("işlev")
                || self.kutuphane_basi_mi()
                || self.kelime_mi("fiil")
                || self.model_basi_mi()
                || self.rota_basi_mi()
                || self.bilesen_basi_mi()
                || self.arayuz_basi_mi()
                || self.durum_basi_mi()
                || self.secenek_basi_mi()
            {
                return Err(Hata::yeni(
                    self.konum(),
                    "işlevler, fiiller, modeller, seçenekler, web yolları, bileşenler, durumlar ve arayüz yalnızca en dış düzeyde tanımlanabilir",
                ));
            }
            govde.push(self.deyim()?);
        }
        if *self.bak() == Tok::Cikinti {
            self.ilerle();
        }
        Ok(govde)
    }

    /// `kütüphane "m":` satırı mı? (C kütüphanesindeki işlevler)
    fn kutuphane_basi_mi(&self) -> bool {
        self.kelime_mi("kütüphane")
            && matches!(self.bak_n(1), Tok::Metin(_))
            && *self.bak_n(2) == Tok::Op(":")
    }

    /// ```text
    /// kütüphane "m":
    ///     işlev cbrt(x: ondalık) -> ondalık
    /// ```
    fn kutuphane_blogu(&mut self) -> Sonuc<Vec<Islev>> {
        self.ilerle();
        let kk = self.konum();
        let Tok::Metin(kutuphane) = self.ilerle().tok else {
            unreachable!()
        };
        if kutuphane.trim().is_empty() {
            return Err(Hata::yeni(kk, "kütüphane adı boş olamaz").ipucu("kütüphane \"m\":"));
        }
        self.bekle_op(":", "kütüphane adından sonra")?;
        if *self.bak() != Tok::YeniSatir {
            return Err(self.beklenmeyen("':' sonrasında yeni satır"));
        }
        self.ilerle();
        if *self.bak() != Tok::Girinti {
            return Err(Hata::yeni(
                self.konum(),
                "kütüphane bloğunda C işlevlerinin imzaları yazılmalı",
            )
            .ipucu("    işlev cbrt(x: ondalık) -> ondalık"));
        }
        self.ilerle();
        let mut islevler = Vec::new();
        while !matches!(self.bak(), Tok::Cikinti | Tok::Son) {
            if *self.bak() == Tok::YeniSatir {
                self.ilerle();
                continue;
            }
            if !self.kelime_mi("işlev") {
                return Err(Hata::yeni(
                    self.konum(),
                    "kütüphane bloğunda yalnızca işlev imzaları olabilir",
                )
                .ipucu("işlev cbrt(x: ondalık) -> ondalık"));
            }
            self.ilerle();
            let konum = self.konum();
            let ad = self.isim_adi("C işlevinin adı")?;
            if YERLESIK.contains(&ad.as_str()) {
                return Err(Hata::yeni(
                    konum,
                    format!("'{ad}' yerleşik bir işlevin adı"),
                ));
            }
            self.bekle_op("(", "parametre listesi")?;
            let mut parametreler = Vec::new();
            let mut tipler = Vec::new();
            while !self.op_mu(")") {
                let p = self.isim_adi("parametre adı")?;
                self.bekle_op(":", "C işlevinin parametre tipi (ör. x: ondalık)")?;
                let t = self.c_tipi(false)?;
                parametreler.push((p, t.orhunca()));
                tipler.push(t);
                if !self.op_mu(")") {
                    self.bekle_op(",", "parametreler arasında")?;
                }
            }
            self.ilerle();
            let donus = if self.op_mu("->") {
                self.ilerle();
                self.c_tipi(true)?
            } else {
                CTip::Yok
            };
            if self.op_mu(":") {
                return Err(Hata::yeni(
                    self.konum(),
                    "C işlevinin gövdesi yazılmaz; yalnızca imzası yazılır",
                ));
            }
            self.deyim_bitir()?;
            islevler.push(Islev {
                ad,
                parametreler,
                haller: Vec::new(),
                donus: Some(donus.orhunca()),
                govde: Vec::new(),
                konum,
                yereller: Vec::new(),
                rota: None,
                arayuz: false,
                dis: Some(DisIslev {
                    kutuphane: kutuphane.clone(),
                    tipler,
                    donus,
                }),
            });
        }
        if *self.bak() == Tok::Cikinti {
            self.ilerle();
        }
        Ok(islevler)
    }

    /// C işlevlerinde kullanılabilen tipler: sayı, sayı32, ondalık, mantık, metin.
    fn c_tipi(&mut self, donus: bool) -> Sonuc<CTip> {
        let konum = self.konum();
        let ad = match self.ilerle().tok {
            Tok::Kelime(k) => k,
            t => {
                return Err(Hata::yeni(
                    konum,
                    format!("tip adı bekleniyordu, {} bulundu", tok_adi(&t)),
                ))
            }
        };
        Ok(match ad.as_str() {
            "sayı" => CTip::Sayi,
            "sayı32" => CTip::Sayi32,
            "ondalık" => CTip::Ondalik,
            "mantık" => CTip::Mantik,
            "metin" => CTip::Metin,
            "yok" if donus => CTip::Yok,
            _ => {
                return Err(Hata::yeni(
                    konum,
                    format!("'{ad}' C işlevlerinde kullanılamaz"),
                )
                .ipucu("C işlevlerinin tipleri: sayı (int64), sayı32 (int), ondalık (double), mantık (bool), metin (char *)"))
            }
        })
    }

    /// `seçenek Renk:` satırı mı?
    fn secenek_basi_mi(&self) -> bool {
        self.kelime_mi("seçenek")
            && matches!(self.bak_n(1), Tok::Kelime(_))
            && *self.bak_n(2) == Tok::Op(":")
    }

    /// `seçenek Renk: kırmızı, yeşil, mavi` ya da değerler alt satırlarda.
    fn secenek_tanimi(&mut self) -> Sonuc<SecenekTanimi> {
        self.bekle_kelime("seçenek")?;
        let konum = self.konum();
        let ad = self.isim_adi("seçenek türünün adı")?;
        if self.t.modeller.contains(&ad) {
            return Err(Hata::yeni(konum, format!("'{ad}' bir modelin adı")));
        }
        self.bekle_op(":", "seçenek türünün adından sonra")?;
        let mut degerler: Vec<String> = Vec::new();
        let deger_ekle = |a: &mut Self, degerler: &mut Vec<String>| -> Sonuc<()> {
            let k = a.konum();
            let d = a.isim_adi("seçenek değeri")?;
            if degerler.contains(&d) {
                return Err(Hata::yeni(k, format!("'{d}' değeri iki kez yazılmış")));
            }
            degerler.push(d);
            Ok(())
        };
        if *self.bak() == Tok::YeniSatir {
            self.ilerle();
            if *self.bak() != Tok::Girinti {
                return Err(
                    Hata::yeni(self.konum(), "seçenek türünün değerleri yazılmalı")
                        .ipucu("seçenek Renk: kırmızı, yeşil, mavi"),
                );
            }
            self.ilerle();
            while !matches!(self.bak(), Tok::Cikinti | Tok::Son) {
                if *self.bak() == Tok::YeniSatir {
                    self.ilerle();
                    continue;
                }
                deger_ekle(self, &mut degerler)?;
                if self.op_mu(",") {
                    self.ilerle();
                } else if !matches!(self.bak(), Tok::YeniSatir | Tok::Cikinti | Tok::Son) {
                    return Err(self.beklenmeyen("',' ya da satır sonu"));
                }
            }
            if *self.bak() == Tok::Cikinti {
                self.ilerle();
            }
        } else {
            loop {
                deger_ekle(self, &mut degerler)?;
                if !self.op_mu(",") {
                    break;
                }
                self.ilerle();
            }
            self.deyim_bitir()?;
        }
        Ok(SecenekTanimi {
            ad,
            degerler,
            konum,
        })
    }

    /// `arayüz:` satırı mı?
    fn arayuz_basi_mi(&self) -> bool {
        self.kelime_mi("arayüz") && *self.bak_n(1) == Tok::Op(":")
    }

    /// `bileşen Kart(başlık: metin):` satırı mı?
    fn bilesen_basi_mi(&self) -> bool {
        self.kelime_mi("bileşen")
            && matches!(self.bak_n(1), Tok::Kelime(_))
            && *self.bak_n(2) == Tok::Op("(")
    }

    /// `durum sayaç = 0` ya da `durum işler: liste<metin> = []` satırı mı?
    fn durum_basi_mi(&self) -> bool {
        self.kelime_mi("durum")
            && matches!(self.bak_n(1), Tok::Kelime(_))
            && matches!(self.bak_n(2), Tok::Op("=") | Tok::Op(":"))
    }

    fn durum(&mut self) -> Sonuc<Durum> {
        self.bekle_kelime("durum")?;
        let konum = self.konum();
        let ad = self.isim_adi("durum adı")?;
        let tip = if self.op_mu(":") {
            self.ilerle();
            Some(self.tip()?)
        } else {
            None
        };
        self.bekle_op("=", "durumun ilk değeri")?;
        let deger = self.duz_ifade()?;
        self.deyim_bitir()?;
        Ok(Durum {
            ad,
            tip,
            deger,
            konum,
        })
    }

    /// `arayüz:` bloğu: programın arayüzünü çizen işlev.
    fn arayuz_blogu(&mut self) -> Sonuc<Islev> {
        let konum = self.konum();
        self.ilerle();
        self.arayuzde = true;
        let govde = self.blok();
        self.arayuzde = false;
        Ok(Islev {
            ad: ARAYUZ_ISLEVI.into(),
            parametreler: Vec::new(),
            haller: Vec::new(),
            donus: Some(Tip::Bos),
            govde: govde?,
            konum,
            yereller: Vec::new(),
            rota: None,
            arayuz: true,
            dis: None,
        })
    }

    /// Arayüz öğesi satırı mı? (`düğme(...)`, `satır:`)
    fn oge_basi_mi(&self) -> bool {
        self.arayuzde
            && matches!(self.bak(), Tok::Kelime(k) if crate::arayuz::oge(k).is_some())
            && matches!(self.bak_n(1), Tok::Op("(") | Tok::Op(":"))
    }

    /// ```text
    /// düğme("Artır", renk: "yeşil") tıklanınca:
    ///     sayaç += 1
    /// satır:
    ///     yazı("a")
    /// ```
    fn oge(&mut self) -> Sonuc<Deyim> {
        let konum = self.konum();
        let Tok::Kelime(ad) = self.ilerle().tok else {
            unreachable!()
        };
        let mut argumanlar = Vec::new();
        let mut secenekler: Vec<(String, Ifade)> = Vec::new();
        if self.op_mu("(") {
            self.ilerle();
            while !self.op_mu(")") {
                if let (Tok::Kelime(s), Tok::Op(":")) = (self.bak().clone(), self.bak_n(1)) {
                    let sk = self.konum();
                    self.ilerle();
                    self.ilerle();
                    let d = self.duz_ifade()?;
                    if secenekler.iter().any(|(a, _)| *a == s) {
                        return Err(Hata::yeni(sk, format!("'{s}' seçeneği iki kez verilmiş")));
                    }
                    secenekler.push((s, d));
                } else {
                    if !secenekler.is_empty() {
                        return Err(Hata::yeni(
                            self.konum(),
                            "adlı seçenekler (renk: ...) değerlerden sonra yazılır",
                        ));
                    }
                    argumanlar.push(self.duz_ifade()?);
                }
                if !self.op_mu(")") {
                    self.bekle_op(",", "değerler arasında")?;
                }
            }
            self.ilerle();
        }
        let mut olay = None;
        let mut cocuklar = Vec::new();
        if let Tok::Kelime(k) = self.bak().clone() {
            if !crate::arayuz::olay_mi(&k) {
                return Err(Hata::yeni(self.konum(), format!("'{k}' bir olay değil"))
                    .ipucu("olaylar: tıklanınca, değişince, gönderilince, çalınca, her_karede"));
            }
            let ok = self.konum();
            self.ilerle();
            // Olay bloğu sıradan koddur: içinde öğe yazılmaz.
            self.arayuzde = false;
            let govde = self.blok();
            self.arayuzde = true;
            olay = Some(Olay {
                ad: k,
                govde: govde?,
                yakalananlar: Vec::new(),
                yereller: Vec::new(),
                konum: ok,
            });
        } else if self.op_mu(":") {
            cocuklar = self.blok()?;
        } else {
            self.deyim_bitir()?;
        }
        Ok(Deyim::Oge(Box::new(Oge {
            ad,
            argumanlar,
            secenekler,
            cocuklar,
            olay,
            baglama: None,
            konum,
        })))
    }

    /// `model Ürün:` satırı mı? (`model` ayrılmış değildir; değişken adı olabilir.)
    fn model_basi_mi(&self) -> bool {
        self.kelime_mi("model")
            && matches!(self.bak_n(1), Tok::Kelime(_))
            && *self.bak_n(2) == Tok::Op(":")
    }

    /// `al "/ürünler":` satırı mı?
    fn rota_basi_mi(&self) -> bool {
        matches!(self.bak(), Tok::Kelime(k) if ROTA_YONTEMLERI.iter().any(|(y, _)| y == k))
            && matches!(self.bak_n(1), Tok::Metin(_))
            && *self.bak_n(2) == Tok::Op(":")
    }

    /// ```text
    /// model Ürün:
    ///     ad: metin, zorunlu, en_fazla 100
    ///     fiyat: ondalık, en_az 0
    ///     stok: sayı = 0
    /// ```
    fn model_tanimi(&mut self) -> Sonuc<Model> {
        let konum = self.konum();
        self.bekle_kelime("model")?;
        let akonum = self.konum();
        let ad = self.isim_adi("model adı")?;
        if akonum.dosya != YERLESIK_DOSYA
            && ["İstek", "Yanıt", "YüklenenDosya"].contains(&ad.as_str())
        {
            return Err(Hata::yeni(
                akonum,
                format!("'{ad}' yerleşik bir model; başka bir ad seçin"),
            ));
        }
        if YERLESIK.contains(&ad.as_str()) || crate::yerlesik::bul(&ad).is_some() {
            return Err(Hata::yeni(
                akonum,
                format!("'{ad}' yerleşik bir işlevin adı"),
            ));
        }
        if ["sayı", "ondalık", "metin", "mantık", "liste", "sözlük"].contains(&ad.as_str()) {
            return Err(Hata::yeni(akonum, format!("'{ad}' bir tip adı")));
        }
        self.bekle_op(":", "model adından sonra")?;
        if *self.bak() != Tok::YeniSatir {
            return Err(self.beklenmeyen("':' sonrasında yeni satır"));
        }
        self.ilerle();
        if *self.bak() != Tok::Girinti {
            return Err(Hata::yeni(
                self.konum(),
                "model boş olamaz; içeriden başlayan alan satırları bekleniyordu",
            )
            .ipucu("model Ürün:\n    ad: metin"));
        }
        self.ilerle();
        let mut alanlar: Vec<AlanTanimi> = Vec::new();
        while !matches!(self.bak(), Tok::Cikinti | Tok::Son) {
            let fk = self.konum();
            let alan = self.isim_adi("alan adı")?;
            if alanlar.iter().any(|a| a.ad == alan) {
                return Err(Hata::yeni(
                    fk,
                    format!("'{alan}' alanı iki kez tanımlanmış"),
                ));
            }
            self.bekle_op(":", "alan adından sonra tipi (ör. ad: metin)")?;
            let tip = self.tip()?;
            let varsayilan = if self.op_mu("=") {
                self.ilerle();
                Some(self.duz_ifade()?)
            } else {
                None
            };
            let mut a = AlanTanimi {
                ad: alan,
                tip,
                varsayilan,
                zorunlu: false,
                en_az: None,
                en_fazla: None,
                etiket: None,
                e_posta: false,
                secenekler: Vec::new(),
                ic_model: None,
                dongusel: false,
                konum: fk,
            };
            while self.op_mu(",") {
                self.ilerle();
                let nk = self.konum();
                let nitelik = match self.bak() {
                    Tok::Kelime(k) => k.clone(),
                    _ => String::new(),
                };
                match nitelik.as_str() {
                    "zorunlu" => {
                        self.ilerle();
                        a.zorunlu = true;
                    }
                    "birincil" if a.ad == "kimlik" => {
                        self.ilerle();
                    }
                    "e_posta" => {
                        self.ilerle();
                        a.e_posta = true;
                    }
                    "etiket" => {
                        self.ilerle();
                        let Tok::Metin(m) = self.bak().clone() else {
                            return Err(self.beklenmeyen("etiket metni (ör. etiket \"E-posta\")"));
                        };
                        if m.is_empty() || m.contains(['\t', '\n']) {
                            return Err(Hata::yeni(self.konum(), "etiket boş olamaz ve satır sonu içeremez"));
                        }
                        self.ilerle();
                        a.etiket = Some(m);
                    }
                    "birincil" => {
                        return Err(Hata::yeni(
                            nk,
                            "'birincil' yalnızca kimlik alanında kullanılır",
                        ))
                    }
                    "en_az" | "en_fazla" => {
                        self.ilerle();
                        let eksi = self.op_mu("-");
                        if eksi {
                            self.ilerle();
                        }
                        let n = match self.bak() {
                            Tok::Sayi(n) => *n as f64,
                            Tok::Ondalik(n) => *n,
                            _ => {
                                return Err(self.beklenmeyen(&format!("'{nitelik}' için bir sayı")))
                            }
                        };
                        self.ilerle();
                        let n = if eksi { -n } else { n };
                        if nitelik == "en_az" {
                            a.en_az = Some(n);
                        } else {
                            a.en_fazla = Some(n);
                        }
                    }
                    _ => {
                        return Err(Hata::yeni(nk, "bilinmeyen alan niteliği").ipucu(
                            "nitelikler: zorunlu, en_az 1, en_fazla 100, e_posta, etiket \"Görünen ad\"",
                        ))
                    }
                }
            }
            self.deyim_bitir()?;
            alanlar.push(a);
        }
        if *self.bak() == Tok::Cikinti {
            self.ilerle();
        }
        // Her modelin ilk alanı kimliktir; yazılmamışsa eklenir.
        match alanlar.iter().position(|a| a.ad == "kimlik") {
            Some(i) => {
                let k = alanlar.remove(i);
                if k.tip != Tip::Sayi || k.varsayilan.is_some() {
                    return Err(Hata::yeni(
                        k.konum,
                        "kimlik alanı varsayılan değeri olmayan bir sayı olmalı",
                    ));
                }
                alanlar.insert(0, k);
            }
            None => alanlar.insert(
                0,
                AlanTanimi {
                    ad: "kimlik".into(),
                    tip: Tip::Sayi,
                    varsayilan: None,
                    zorunlu: false,
                    en_az: None,
                    en_fazla: None,
                    etiket: None,
                    e_posta: false,
                    secenekler: Vec::new(),
                    ic_model: None,
                    dongusel: false,
                    konum,
                },
            ),
        }
        Ok(Model {
            ad,
            alanlar,
            konum: akonum,
        })
    }

    /// `al "/ürünler/{kimlik: sayı}":` — gövde, `istek` parametresi alan ve `Yanıt`
    /// döndüren bir işleve çevrilir. Yol parametreleri yerel değişken olur.
    fn rota(&mut self) -> Sonuc<Islev> {
        let konum = self.konum();
        let Tok::Kelime(y) = self.ilerle().tok else {
            unreachable!()
        };
        let yontem = ROTA_YONTEMLERI.iter().find(|(t, _)| *t == y).unwrap().1;
        let kkonum = self.konum();
        let Tok::Metin(kalip) = self.ilerle().tok else {
            unreachable!()
        };
        let (kalip, parametreler) = rota_kalibi(&kalip, kkonum)?;
        let govde = self.blok()?;
        let mut tum = Vec::new();
        for (ad, tip) in parametreler {
            // ad = istek.parametreler["ad"]   (sayı ise sayı(...))
            let e = |tur| Ifade::yeni(tur, kkonum);
            let istek = e(IfadeTuru::Isim("istek".into()));
            let p = e(IfadeTuru::Alan(Box::new(istek), "parametreler".into(), 0));
            let mut deger = e(IfadeTuru::Indeks(
                Box::new(p),
                Box::new(e(IfadeTuru::Metin(ad.clone()))),
            ));
            if tip == Tip::Sayi {
                deger = e(IfadeTuru::Cagri("sayı".into(), vec![deger]));
            }
            tum.push(Deyim::Atama {
                hedef: ad,
                tip: None,
                deger,
                konum: kkonum,
            });
        }
        tum.extend(govde);
        Ok(Islev {
            ad: format!("{yontem} {kalip}"),
            parametreler: vec![("istek".into(), Tip::Model("İstek".into()))],
            haller: Vec::new(),
            donus: Some(Tip::Model("Yanıt".into())),
            govde: tum,
            konum,
            yereller: Vec::new(),
            arayuz: false,
            dis: None,
            rota: Some(Rota {
                yontem: yontem.into(),
                kalip,
            }),
        })
    }

    /// `işlev ad(...) -> tip:` ya da `bileşen Ad(...):` (arayüz parçası)
    fn islev(&mut self) -> Sonuc<Islev> {
        let konum = self.konum();
        let bilesen = self.kelime_mi("bileşen");
        self.ilerle();
        let ad = self.isim_adi(if bilesen {
            "bileşen adı"
        } else {
            "işlev adı"
        })?;
        if bilesen && crate::arayuz::oge(&ad).is_some() {
            return Err(Hata::yeni(
                konum,
                format!("'{ad}' yerleşik bir arayüz öğesinin adı"),
            ));
        }
        if YERLESIK.contains(&ad.as_str()) {
            return Err(Hata::yeni(
                konum,
                format!("'{ad}' yerleşik bir işlevin adı"),
            ));
        }
        self.bekle_op("(", "parametre listesi")?;
        let mut parametreler = Vec::new();
        while !self.op_mu(")") {
            let p = self.isim_adi("parametre adı")?;
            let tip = if self.op_mu(":") {
                self.ilerle();
                self.tip()?
            } else {
                // Tipi denetçi çıkarır (ilk çağrıdan; yoksa sayı).
                Tip::Bilinmeyen
            };
            parametreler.push((p, tip));
            if !self.op_mu(")") {
                self.bekle_op(",", "parametreler arasında")?;
            }
        }
        self.ilerle();
        let donus = if bilesen {
            Some(Tip::Bos)
        } else if self.op_mu("->") {
            self.ilerle();
            Some(self.tip()?)
        } else {
            None
        };
        self.arayuzde = bilesen;
        let govde = self.blok();
        self.arayuzde = false;
        Ok(Islev {
            ad,
            parametreler,
            haller: Vec::new(),
            donus,
            govde: govde?,
            konum,
            yereller: Vec::new(),
            arayuz: bilesen,
            dis: None,
            rota: None,
        })
    }

    /// `fiil sayı'yı karele:` ya da `fiil (a: ondalık)'yı b'ye böl -> ondalık:`
    fn fiil_tanimi(&mut self) -> Sonuc<Islev> {
        let konum = self.konum();
        self.bekle_kelime("fiil")?;
        let mut parametreler = Vec::new();
        let mut haller: Vec<Hal> = Vec::new();
        loop {
            let pkonum = self.konum();
            let (ad, tip) = if self.op_mu("(") {
                self.ilerle();
                let ad = self.isim_adi("parametre adı")?;
                let tip = if self.op_mu(":") {
                    self.ilerle();
                    self.tip()?
                } else {
                    Tip::Bilinmeyen
                };
                self.bekle_op(")", "parametrenin sonunda")?;
                (ad, tip)
            } else if matches!(self.bak_n(1), Tok::Ek(_)) {
                (self.isim_adi("parametre adı")?, Tip::Bilinmeyen)
            } else {
                break;
            };
            let Tok::Ek(ek) = self.bak().clone() else {
                return Err(self.beklenmeyen("parametrenin hâl eki (ör. sayı'yı)"));
            };
            let ekonum = self.konum();
            self.ilerle();
            let hal =
                hal_bul(&ek).ok_or_else(|| Hata::yeni(ekonum, format!("tanınmayan ek '{ek}'")))?;
            if hal == Hal::Ilgi {
                return Err(Hata::yeni(
                    ekonum,
                    "fiil parametresi ilgi hâlinde (-in) olamaz",
                ));
            }
            if haller.contains(&hal) {
                return Err(Hata::yeni(
                    ekonum,
                    format!("bu fiilde zaten {} hâlinde bir parametre var", hal.adi()),
                )
                .ipucu("her parametre farklı bir hâl eki almalı; çağrıda rolleri ekler belirler"));
            }
            if parametreler.iter().any(|(p, _)| *p == ad) {
                return Err(Hata::yeni(
                    pkonum,
                    format!("'{ad}' parametresi iki kez yazılmış"),
                ));
            }
            parametreler.push((ad, tip));
            haller.push(hal);
        }
        let akonum = self.konum();
        let ad = match self.bak().clone() {
            Tok::Kelime(k) if self.t.fiiller.contains(&k) => {
                self.ilerle();
                k
            }
            Tok::Kelime(k) if FIILLER.contains(&k.as_str()) => {
                return Err(Hata::yeni(
                    akonum,
                    format!("'{k}' yerleşik bir fiil, yeniden tanımlanamaz"),
                ))
            }
            Tok::Kelime(k) if !ayrilmis_mi(&k) => {
                return Err(
                    Hata::yeni(akonum, format!("'{k}' parametresinin bir hâl eki olmalı"))
                        .ipucu(format!("{k}'i ya da (x: sayı)'yı biçiminde yazın")),
                )
            }
            _ => return Err(self.beklenmeyen("fiil adı")),
        };
        if YERLESIK.contains(&ad.as_str()) {
            return Err(Hata::yeni(
                akonum,
                format!("'{ad}' yerleşik bir işlevin adı"),
            ));
        }
        let donus = if self.op_mu("->") {
            self.ilerle();
            Some(self.tip()?)
        } else {
            None
        };
        let govde = self.blok()?;
        Ok(Islev {
            ad,
            parametreler,
            haller,
            donus,
            govde,
            konum,
            yereller: Vec::new(),
            arayuz: false,
            dis: None,
            rota: None,
        })
    }

    fn isim_adi(&mut self, ne: &str) -> Sonuc<String> {
        match self.bak().clone() {
            Tok::Kelime(k) if !ayrilmis_mi(&k) => {
                self.ilerle();
                Ok(k)
            }
            Tok::Kelime(k) => Err(Hata::yeni(
                self.konum(),
                format!("'{k}' ayrılmış bir kelime, {ne} olarak kullanılamaz"),
            )),
            _ => Err(self.beklenmeyen(ne)),
        }
    }

    fn tip(&mut self) -> Sonuc<Tip> {
        let konum = self.konum();
        let ad = match self.ilerle().tok {
            Tok::Kelime(k) => k,
            t => {
                return Err(Hata::yeni(
                    konum,
                    format!("tip adı bekleniyordu, {} bulundu", tok_adi(&t)),
                ))
            }
        };
        Ok(match ad.as_str() {
            "sayı" => Tip::Sayi,
            "ondalık" => Tip::Ondalik,
            "metin" => Tip::Metin,
            "mantık" => Tip::Mantik,
            "liste" => {
                if self.op_mu("<") {
                    self.ilerle();
                    let ic = self.tip()?;
                    self.bekle_op(">", "liste tipinin sonunda")?;
                    Tip::Liste(Box::new(ic))
                } else {
                    Tip::Liste(Box::new(Tip::Sayi))
                }
            }
            "sözlük" => {
                self.bekle_op("<", "sözlük tipi: sözlük<metin, sayı>")?;
                let a = self.tip()?;
                self.bekle_op(",", "sözlük tipinde anahtar ile değer arasında")?;
                let d = self.tip()?;
                self.bekle_op(">", "sözlük tipinin sonunda")?;
                Tip::Sozluk(Box::new(a), Box::new(d))
            }
            _ if self.t.modeller.contains(&ad) => Tip::Model(ad),
            _ if self.t.secenekler.contains_key(&ad) => Tip::Secenek(ad),
            _ => {
                return Err(Hata::yeni(konum, format!("bilinmeyen tip '{ad}'")).ipucu(
                    "tipler: sayı, ondalık, metin, mantık, liste<sayı>, sözlük<metin, sayı>, bir model ya da seçenek adı",
                ))
            }
        })
    }

    // ---------- deyimler ----------

    /// Tanımsız isim hatası; başka dillerden gelen kelimeler için Orhunca karşılığı,
    /// yanlış yazılmış isimler için "bunu mu demek istediniz?" önerisi.
    fn tanimsiz(&self, k: &str, konum: Konum) -> Hata {
        let h = Hata::yeni(konum, format!("tanımsız isim '{k}'"))
            .ipucu("değişkeni önce tanımlayın (ör. x = 5) ya da ekini kesme işaretiyle ayırın");
        if let Some(oneri) = crate::oneriler::yabanci(k) {
            return h.oneri(oneri);
        }
        let adaylar = self
            .t
            .sozluk
            .isimler()
            .chain(self.t.fiiller.iter().map(String::as_str))
            .chain(AYRILMIS.iter().copied())
            .chain(crate::yerlesik::YERLESIKLER.iter().map(|y| y.ad));
        match crate::oneriler::benzer(k, adaylar) {
            Some(b) => h.oneri(format!("bunu mu demek istediniz: {b}")),
            None => h,
        }
    }

    /// `yaz("merhaba")`, `print(x)`: başka dillerdeki yazdırma alışkanlığı.
    fn yazdirma_aliskanligi(&self) -> Option<Hata> {
        let Tok::Kelime(k) = self.bak() else {
            return None;
        };
        let yabanci = matches!(
            k.as_str(),
            "print" | "println" | "printf" | "puts" | "echo" | "yazdır" | "yazdir"
        );
        if k != "yaz" && !yabanci {
            return None;
        }
        if !matches!(
            self.bak_n(1),
            Tok::Op("(") | Tok::Metin(_) | Tok::Sayi(_) | Tok::Ondalik(_) | Tok::Kelime(_)
        ) {
            return None;
        }
        let son = self.sozcukler[self.poz..]
            .iter()
            .position(|z| {
                matches!(
                    z.tok,
                    Tok::YeniSatir | Tok::Son | Tok::Girinti | Tok::Cikinti
                )
            })
            .map(|n| self.poz + n)
            .unwrap_or(self.sozcukler.len());
        let mut arguman = &self.sozcukler[self.poz + 1..son];
        if matches!(arguman.first().map(|z| &z.tok), Some(Tok::Op("(")))
            && matches!(arguman.last().map(|z| &z.tok), Some(Tok::Op(")")))
        {
            arguman = &arguman[1..arguman.len() - 1];
        }
        let mut metin = sozcuk_metni(arguman);
        if metin.is_empty() {
            return None;
        }
        if arguman.len() > 1 && !matches!(arguman.last().map(|z| &z.tok), Some(Tok::Op(")"))) {
            metin = format!("({metin})");
        }
        let ek = crate::bicimlendirici::ek_oner(&metin, crate::ekler::Hal::Belirtme);
        Some(
            Hata::yeni(
                self.konum(),
                "Orhunca'da ekrana yazmak bir cümledir: önce değer, sonra fiil",
            )
            .ipucu(format!("şöyle yazın: {metin}'{ek} yaz.")),
        )
    }

    fn deyim(&mut self) -> Sonuc<Deyim> {
        let konum = self.konum();
        if let Some(h) = self.yazdirma_aliskanligi() {
            return Err(h);
        }
        if self.oge_basi_mi() {
            return self.oge();
        }
        if self.kelime_mi("eğer") {
            self.ilerle();
            return self.eger();
        }
        if self.kelime_mi("değilse") {
            return Err(Hata::yeni(
                konum,
                "'değilse' bir 'eğer' bloğundan hemen sonra gelmeli",
            ));
        }
        if self.kelime_mi("her") {
            return self.her();
        }
        if self.kelime_mi("dene") && *self.bak_n(1) == Tok::Op(":") {
            return self.dene();
        }
        if self.yakala_basi_mi() {
            return Err(Hata::yeni(
                konum,
                "'yakala' bir 'dene' bloğundan hemen sonra gelmeli",
            ));
        }
        if self.kelime_mi("döndür") {
            self.ilerle();
            let deger = if self.deyim_sonu_mu() {
                None
            } else {
                Some(self.duz_ifade()?)
            };
            self.deyim_bitir()?;
            return Ok(Deyim::Dondur(deger, konum));
        }
        if self.kelime_mi("dur") {
            self.ilerle();
            self.deyim_bitir()?;
            return Ok(Deyim::Dur(konum));
        }
        if self.kelime_mi("sürdür") {
            self.ilerle();
            self.deyim_bitir()?;
            return Ok(Deyim::Surdur(konum));
        }
        if let Some(d) = self.atama()? {
            return Ok(d);
        }
        if self.surece_satiri_mi() {
            let kosul = self.kosul(KosulTuru::Surece)?;
            let govde = self.blok()?;
            return Ok(Deyim::Surece { kosul, govde });
        }
        self.cumle()
    }

    /// `x = ...`, `x += ...`, `liste[i] = ...`
    fn atama(&mut self) -> Sonuc<Option<Deyim>> {
        let konum = self.konum();
        let Tok::Kelime(ad) = self.bak().clone() else {
            return Ok(None);
        };
        if ayrilmis_mi(&ad) {
            return Ok(None);
        }
        match self.bak_n(1) {
            Tok::Op(op @ ("=" | "+=" | "-=")) => {
                let op = *op;
                self.ilerle();
                self.ilerle();
                let mut deger = self.duz_ifade()?;
                if op != "=" {
                    let ikili = if op == "+=" {
                        IkiliOp::Topla
                    } else {
                        IkiliOp::Cikar
                    };
                    let sol = Ifade::yeni(IfadeTuru::Isim(ad.clone()), konum);
                    deger = Ifade::yeni(
                        IfadeTuru::Ikili(ikili, Box::new(sol), Box::new(deger)),
                        konum,
                    );
                }
                self.deyim_bitir()?;
                Ok(Some(Deyim::Atama {
                    hedef: ad,
                    tip: None,
                    deger,
                    konum,
                }))
            }
            Tok::Op("[") | Tok::Uye => self.uye_atamasi(konum),
            Tok::Op(":") if self.tip_bildirimi_mi() => {
                // `işler: liste<metin> = []`
                self.ilerle();
                self.ilerle();
                let tip = self.tip()?;
                self.bekle_op("=", "değişkenin ilk değeri")?;
                let deger = self.duz_ifade()?;
                self.deyim_bitir()?;
                Ok(Some(Deyim::Atama {
                    hedef: ad,
                    tip: Some(tip),
                    deger,
                    konum,
                }))
            }
            _ => Ok(None),
        }
    }

    /// `ad: tip = değer` satırı mı? (`:` sonrasında bir kelime ve satırda `=` var)
    fn tip_bildirimi_mi(&self) -> bool {
        if !matches!(self.bak_n(2), Tok::Kelime(_)) {
            return false;
        }
        let mut i = self.poz + 2;
        while !matches!(self.sozcukler[i].tok, Tok::YeniSatir | Tok::Son) {
            if self.sozcukler[i].tok == Tok::Op("=") {
                return true;
            }
            i += 1;
        }
        false
    }

    /// `ürün.fiyat = 12.5`, `ürün.stok += 1`, `ürün.etiketler[0] = "yeni"`
    fn uye_atamasi(&mut self, konum: Konum) -> Sonuc<Option<Deyim>> {
        let mut i = self.poz;
        let mut derinlik = 0;
        let mut op = None;
        while !matches!(self.sozcukler[i].tok, Tok::YeniSatir | Tok::Son) {
            match self.sozcukler[i].tok {
                Tok::Op("[") | Tok::Op("(") => derinlik += 1,
                Tok::Op("]") | Tok::Op(")") => derinlik -= 1,
                Tok::Op(o) if derinlik == 0 && matches!(o, "=" | "+=" | "-=") => {
                    op = Some(o);
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        let Some(op) = op else {
            return Ok(None);
        };
        self.yakalanan = None;
        let hedef = self.sonek()?;
        if let Some((_, k)) = self.yakalanan.take() {
            return Err(Hata::yeni(k, "atamanın sol tarafı ek almaz"));
        }
        if !self.op_mu(op) {
            return Err(self.beklenmeyen(&format!("'{op}'")));
        }
        self.ilerle();
        let mut deger = self.duz_ifade()?;
        self.deyim_bitir()?;
        match hedef.tur {
            IfadeTuru::Alan(nesne, alan, _) => {
                if op != "=" {
                    let ikili = if op == "+=" {
                        IkiliOp::Topla
                    } else {
                        IkiliOp::Cikar
                    };
                    let sol =
                        Ifade::yeni(IfadeTuru::Alan(nesne.clone(), alan.clone(), 0), hedef.konum);
                    deger = Ifade::yeni(
                        IfadeTuru::Ikili(ikili, Box::new(sol), Box::new(deger)),
                        konum,
                    );
                }
                Ok(Some(Deyim::AlanAtama {
                    nesne: *nesne,
                    alan,
                    sira: 0,
                    deger,
                    konum,
                }))
            }
            IfadeTuru::Indeks(liste, indeks) => {
                if op != "=" {
                    // `sayım[a] += 1` → `sayım[a] = sayım[a] + 1`
                    let ikili = if op == "+=" {
                        IkiliOp::Topla
                    } else {
                        IkiliOp::Cikar
                    };
                    let sol = Ifade::yeni(
                        IfadeTuru::Indeks(liste.clone(), indeks.clone()),
                        hedef.konum,
                    );
                    deger = Ifade::yeni(
                        IfadeTuru::Ikili(ikili, Box::new(sol), Box::new(deger)),
                        konum,
                    );
                }
                Ok(Some(Deyim::IndeksAtama {
                    liste: *liste,
                    indeks: *indeks,
                    deger,
                }))
            }
            _ => Err(Hata::yeni(
                konum,
                "atamanın sol tarafı bir değişken, liste öğesi ya da model alanı olmalı",
            )),
        }
    }

    /// Satır `... olduğu sürece:` ya da `... küçükken:` ile mi bitiyor?
    fn surece_satiri_mi(&self) -> bool {
        let mut i = self.poz;
        while !matches!(self.sozcukler[i].tok, Tok::YeniSatir | Tok::Son) {
            i += 1;
        }
        if i < self.poz + 2 || self.sozcukler[i - 1].tok != Tok::Op(":") {
            return false;
        }
        match &self.sozcukler[i - 2].tok {
            Tok::Kelime(k) => {
                k == "sürece" || k == "iken" || yuklem(k).is_some_and(|(_, ek)| ek == "ken")
            }
            _ => false,
        }
    }

    fn eger(&mut self) -> Sonuc<Deyim> {
        let kosul = self.kosul(KosulTuru::Eger)?;
        let govde = self.blok()?;
        let mut degilse = Vec::new();
        if self.kelime_mi("değilse") {
            self.ilerle();
            if self.kelime_mi("eğer") {
                self.ilerle();
                degilse.push(self.eger()?);
            } else {
                degilse = self.blok()?;
            }
        }
        Ok(Deyim::Eger {
            kosul,
            govde,
            degilse,
        })
    }

    /// `yakala:` ya da `yakala hata:` satırı mı?
    fn yakala_basi_mi(&self) -> bool {
        self.kelime_mi("yakala")
            && (*self.bak_n(1) == Tok::Op(":")
                || (matches!(self.bak_n(1), Tok::Kelime(_)) && *self.bak_n(2) == Tok::Op(":")))
    }

    /// `dene:` + blok, ardından `yakala hata:` (ya da `yakala:`) + blok
    fn dene(&mut self) -> Sonuc<Deyim> {
        let konum = self.konum();
        self.bekle_kelime("dene")?;
        let govde = self.blok()?;
        if !self.yakala_basi_mi() {
            return Err(Hata::yeni(
                self.konum(),
                "'dene' bloğundan sonra bir 'yakala' bloğu gelmeli",
            )
            .ipucu("dene:\n    ...\nyakala hata:\n    hata'yı yaz."));
        }
        self.ilerle();
        let degisken = if self.op_mu(":") {
            None
        } else {
            Some(self.isim_adi("hata değişkeni")?)
        };
        let yakala = self.blok()?;
        Ok(Deyim::Dene {
            govde,
            degisken,
            yakala,
            konum,
        })
    }

    /// `her i için 1'den 10'a kadar:` ya da `her sayı için sayılardan:`
    fn her(&mut self) -> Sonuc<Deyim> {
        let konum = self.konum();
        self.bekle_kelime("her")?;
        let degisken = self.isim_adi("döngü değişkeni")?;
        self.bekle_kelime("için")?;
        let (kaynak, hal, hal_konum) = self.ekli_ifade()?;
        if hal != Some(Hal::Ayrilma) {
            return Err(
                Hata::yeni(hal_konum, "döngünün kaynağı ayrılma hâlinde (-den) olmalı")
                    .ipucu("her sayı için sayılardan:  ya da  her i için 1'den 10'a kadar:"),
            );
        }
        if self.op_mu(":") {
            let govde = self.blok()?;
            return Ok(Deyim::HerListe {
                degisken,
                liste: kaynak,
                govde,
                konum,
            });
        }
        let (son, hal, hal_konum) = self.ekli_ifade()?;
        if hal != Some(Hal::Yonelme) {
            return Err(
                Hata::yeni(hal_konum, "aralığın sonu yönelme hâlinde (-e) olmalı")
                    .ipucu("her i için 1'den 10'a kadar:"),
            );
        }
        self.bekle_kelime("kadar")?;
        let govde = self.blok()?;
        Ok(Deyim::HerAralik {
            degisken,
            bas: kaynak,
            son,
            govde,
            konum,
        })
    }

    /// Fiille biten cümle: `x'i ekrana yaz.`, `5'i sayılara ekle.`, `sayıları sırala.`
    fn cumle(&mut self) -> Sonuc<Deyim> {
        let konum = self.konum();
        let mut ogeler: Vec<(Ifade, Option<Hal>, Konum)> = Vec::new();
        let mut fiil: Option<(String, Konum)> = None;
        loop {
            if let Tok::Kelime(k) = self.bak() {
                let kullanici = self.t.fiiller.contains(k) && *self.bak_n(1) != Tok::Op("(");
                let model_fiili = MODEL_FIILLERI.contains(&k.as_str())
                    && matches!(
                        self.bak_n(1),
                        Tok::YeniSatir | Tok::Son | Tok::Cikinti | Tok::Op(".")
                    );
                if FIILLER.contains(&k.as_str()) || kullanici || model_fiili {
                    fiil = Some((k.clone(), self.konum()));
                    self.ilerle();
                    break;
                }
                if k == "ekrana" {
                    self.ilerle();
                    continue;
                }
                if k == "ekran"
                    && matches!(self.bak_n(1), Tok::Ek(e) if hal_bul(e) == Some(Hal::Yonelme))
                {
                    self.ilerle();
                    self.ilerle();
                    continue;
                }
            }
            if self.deyim_sonu_mu() || self.op_mu(":") {
                break;
            }
            // Cümlenin son kelimesi tanınmıyorsa büyük olasılıkla bilinmeyen bir fiildir.
            if let Tok::Kelime(k) = self.bak() {
                let sonda = matches!(
                    self.bak_n(1),
                    Tok::YeniSatir | Tok::Son | Tok::Cikinti | Tok::Op(".")
                );
                if sonda && !ayrilmis_mi(k) && self.t.sozluk.cozumle(k) == Cozum::Bilinmiyor {
                    let mut fiiller: Vec<&str> = FIILLER.to_vec();
                    fiiller.extend(MODEL_FIILLERI);
                    let mut kullanici: Vec<&str> =
                        self.t.fiiller.iter().map(|s| s.as_str()).collect();
                    kullanici.sort();
                    fiiller.extend(kullanici);
                    return Err(
                        Hata::yeni(self.konum(), format!("bilinmeyen fiil '{k}'")).ipucu(format!(
                            "kullanılabilen fiiller: {}. Yeni fiil tanımlamak için: fiil x'i {k}:",
                            fiiller.join(", ")
                        )),
                    );
                }
            }
            ogeler.push(self.ekli_ifade()?);
        }
        if let (Tok::Ek(ek), Some((f, _))) = (self.bak(), &fiil) {
            return Err(Hata::yeni(
                self.konum(),
                format!("fiilin sonucu doğrudan ek alamaz ('{ek})"),
            )
            .ipucu(format!(
                "fiil cümlesini parantez içine alın: (... {f})'{ek}"
            )));
        }
        if self.op_mu(":") {
            return Err(Hata::yeni(self.konum(), "bu satır bir blok başlatamaz")
                .ipucu("koşul için 'eğer', döngü için 'her' ya da '... olduğu sürece:' kullanın"));
        }
        self.deyim_bitir()?;

        let bul = |ogeler: &mut Vec<(Ifade, Option<Hal>, Konum)>, hal: Hal| {
            ogeler
                .iter()
                .position(|o| o.1 == Some(hal))
                .map(|i| ogeler.remove(i).0)
        };

        let Some((fiil, fiil_konum)) = fiil else {
            if ogeler.len() == 1 && ogeler[0].1.is_none() {
                let (ifade, _, _) = ogeler.pop().unwrap();
                if matches!(ifade.tur, IfadeTuru::Cagri(..) | IfadeTuru::Metod(..)) {
                    return Ok(Deyim::IfadeDeyimi(ifade));
                }
            }
            return Err(Hata::yeni(konum, "cümle bir fiille bitmeli")
                .ipucu("ör. x'i ekrana yaz.  5'i sayılara ekle.  sayıları sırala."));
        };

        if self.t.fiiller.contains(&fiil) {
            let mut arg = Vec::new();
            for (ifade, hal, k) in ogeler {
                let Some(hal) = hal else {
                    return Err(Hata::yeni(
                        k,
                        format!("bu öğe ek almamış; '{fiil}' fiili için rolü belli değil"),
                    ));
                };
                arg.push((hal, ifade));
            }
            return Ok(Deyim::IfadeDeyimi(Ifade::yeni(
                IfadeTuru::FiilCagri(fiil, arg),
                fiil_konum,
            )));
        }

        let sonuc = match fiil.as_str() {
            "yaz" => {
                let Some(deger) = bul(&mut ogeler, Hal::Belirtme) else {
                    return Err(Hata::yeni(
                        fiil_konum,
                        "'yaz' neyin yazılacağını belirtme hâlinde (-i) bekler",
                    )
                    .ipucu("x'i ekrana yaz.  \"Merhaba\"'yı yaz."));
                };
                // `metni "notlar.txt"'ye yaz.` → dosyaya yazar
                match bul(&mut ogeler, Hal::Yonelme) {
                    Some(yol) => Deyim::DosyayaYaz { deger, yol },
                    None => Deyim::Yaz(deger),
                }
            }
            "çıkar" => {
                let oge = bul(&mut ogeler, Hal::Belirtme);
                let liste = bul(&mut ogeler, Hal::Ayrilma);
                match (oge, liste) {
                    (Some(oge), Some(liste)) => Deyim::Cikar { oge, liste },
                    _ => {
                        return Err(Hata::yeni(
                            fiil_konum,
                            "'çıkar' bir öğe (-i) ve bir liste (-den) bekler",
                        )
                        .ipucu("5'i sayılardan çıkar."))
                    }
                }
            }
            "ekle" => {
                let oge = bul(&mut ogeler, Hal::Belirtme);
                let liste = bul(&mut ogeler, Hal::Yonelme);
                match (oge, liste) {
                    (Some(oge), Some(liste)) => Deyim::Ekle { oge, liste },
                    _ => {
                        return Err(Hata::yeni(
                            fiil_konum,
                            "'ekle' bir öğe (-i) ve bir liste (-e) bekler",
                        )
                        .ipucu("5'i sayılara ekle."))
                    }
                }
            }
            "kaydet" | "sil" => {
                let Some(nesne) = bul(&mut ogeler, Hal::Belirtme) else {
                    return Err(Hata::yeni(
                        fiil_konum,
                        format!("'{fiil}' bir model nesnesini belirtme hâlinde (-i) bekler"),
                    )
                    .ipucu(format!("ürün'ü {fiil}.")));
                };
                Deyim::IfadeDeyimi(Ifade::yeni(
                    IfadeTuru::Metod(Box::new(nesne), fiil.clone(), Vec::new()),
                    fiil_konum,
                ))
            }
            "sırala" => {
                let Some(liste) = bul(&mut ogeler, Hal::Belirtme) else {
                    return Err(Hata::yeni(
                        fiil_konum,
                        "'sırala' belirtme hâlinde (-i) bir liste bekler",
                    )
                    .ipucu("sayıları sırala."));
                };
                Deyim::Sirala(liste)
            }
            _ => unreachable!(),
        };
        if let Some((_, hal, k)) = ogeler.first() {
            let neden = match hal {
                Some(h) => format!("{} hâlindeki bu öğe '{fiil}' fiiliyle kullanılmaz", h.adi()),
                None => format!("bu öğe ek almamış; '{fiil}' fiili için rolü belli değil"),
            };
            return Err(Hata::yeni(*k, neden));
        }
        Ok(sonuc)
    }

    // ---------- koşullar ----------

    /// `x 4'ten büyükse`, `a b'ye eşit değilse`, `x > 3 ve y < 2 ise`,
    /// `x 10'dan küçük olduğu sürece`, `x 10'dan küçükken`
    fn kosul(&mut self, tur: KosulTuru) -> Sonuc<Ifade> {
        // ve/veya: 've' daha sıkı bağlanır.
        let mut veya_parcalari = Vec::new();
        let mut ve_parcalari = vec![self.kosul_parcasi(tur)?];
        loop {
            if self.kelime_mi("ve") {
                self.ilerle();
                ve_parcalari.push(self.kosul_parcasi(tur)?);
            } else if self.kelime_mi("veya") {
                self.ilerle();
                veya_parcalari.push(std::mem::take(&mut ve_parcalari));
                ve_parcalari.push(self.kosul_parcasi(tur)?);
            } else {
                break;
            }
        }
        veya_parcalari.push(ve_parcalari);
        let birlestir = |parcalar: Vec<Ifade>, op: IkiliOp| {
            parcalar
                .into_iter()
                .reduce(|a, b| {
                    let k = a.konum;
                    Ifade::yeni(IfadeTuru::Ikili(op, Box::new(a), Box::new(b)), k)
                })
                .unwrap()
        };
        let ifade = birlestir(
            veya_parcalari
                .into_iter()
                .map(|p| birlestir(p, IkiliOp::Ve))
                .collect(),
            IkiliOp::Veya,
        );

        match tur {
            KosulTuru::Eger => {
                if self.kelime_mi("ise") {
                    self.ilerle();
                }
            }
            KosulTuru::Surece => {
                if self.kelime_mi("olduğu") {
                    self.ilerle();
                    self.bekle_kelime("sürece")?;
                } else if self.kelime_mi("iken") || self.kelime_mi("sürece") {
                    self.ilerle();
                }
            }
        }
        if !self.op_mu(":") {
            return Err(self.beklenmeyen("koşulun sonunda ':'"));
        }
        Ok(ifade)
    }

    fn kosul_sonu_mu(&self) -> bool {
        self.op_mu(":")
            || ["ise", "olduğu", "iken", "sürece", "ve", "veya"]
                .iter()
                .any(|k| self.kelime_mi(k))
    }

    fn satirda_fiil_var(&self) -> bool {
        self.sozcukler[self.poz..]
            .iter()
            .take_while(|s| !matches!(s.tok, Tok::YeniSatir | Tok::Son))
            .any(|s| matches!(&s.tok, Tok::Kelime(k) if self.t.fiiller.contains(k)))
    }

    fn kosul_parcasi(&mut self, tur: KosulTuru) -> Sonuc<Ifade> {
        let konum = self.konum();
        self.yakalanan = None;
        let mut sol = self.degil_ifadesi()?;
        // `eğer 5'i karele 20'den büyükse:` — koşul bir fiil çağrısıyla başlıyor.
        if let Some((h, k)) = self.yakalanan {
            if self.satirda_fiil_var() {
                self.yakalanan = None;
                sol = self.fiil_tamamla(sol, h, k)?;
            }
        }
        if let Some((h, k)) = self.yakalanan.take() {
            return Err(Hata::yeni(
                k,
                format!("koşulun ilk öğesi ek almamalı ({} bulundu)", h.adi()),
            )
            .ipucu("karşılaştırma iki değer ister: x 3'e eşitse, x 4'ten büyükse"));
        }
        if self.kosul_sonu_mu() {
            return Ok(sol);
        }
        // `bitti değilse`
        if let Tok::Kelime(k) = self.bak() {
            if let Some((YuklemKoku::Degil, ek)) = yuklem(k) {
                let ek = ek.to_string();
                self.ilerle();
                self.yuklem_eki_denetle(tur, &ek, konum)?;
                return Ok(Ifade::yeni(
                    IfadeTuru::Tekli(TekliOp::Degil, Box::new(sol)),
                    konum,
                ));
            }
        }
        let (sag, hal, hal_konum) = self.ekli_ifade()?;
        let ykonum = self.konum();
        let Tok::Kelime(k) = self.bak().clone() else {
            return Err(self.beklenmeyen("'büyük', 'küçük' ya da 'eşit'"));
        };
        let Some((kok, ek)) = yuklem(&k) else {
            return Err(self.beklenmeyen("'büyük', 'küçük' ya da 'eşit'"));
        };
        let mut ek = ek.to_string();
        self.ilerle();

        let mut op = match (kok, hal) {
            (YuklemKoku::Buyuk, Some(Hal::Ayrilma)) => IkiliOp::Buyuk,
            (YuklemKoku::Kucuk, Some(Hal::Ayrilma)) => IkiliOp::Kucuk,
            (YuklemKoku::Esit, Some(Hal::Yonelme)) => IkiliOp::Esit,
            (YuklemKoku::Buyuk | YuklemKoku::Kucuk, _) => {
                return Err(Hata::yeni(
                    hal_konum,
                    "karşılaştırılan değer ayrılma hâlinde (-den) olmalı",
                )
                .ipucu("x 4'ten büyükse"))
            }
            (YuklemKoku::Esit, _) => {
                return Err(Hata::yeni(
                    hal_konum,
                    "eşitlikte karşılaştırılan değer yönelme hâlinde (-e) olmalı",
                )
                .ipucu("x 4'e eşitse"))
            }
            (YuklemKoku::Degil, _) => {
                return Err(Hata::yeni(ykonum, "'değil' burada beklenmiyordu"))
            }
        };

        // `büyük veya eşitse`, `eşit değilse`
        if ek.is_empty() {
            if let (IkiliOp::Buyuk | IkiliOp::Kucuk, true) = (op, self.kelime_mi("veya")) {
                if let Tok::Kelime(k2) = self.bak_n(1).clone() {
                    if let Some((YuklemKoku::Esit, ek2)) = yuklem(&k2) {
                        self.ilerle();
                        self.ilerle();
                        op = if op == IkiliOp::Buyuk {
                            IkiliOp::BuyukEsit
                        } else {
                            IkiliOp::KucukEsit
                        };
                        ek = ek2.to_string();
                    }
                }
            }
            if let Tok::Kelime(k2) = self.bak().clone() {
                if let Some((YuklemKoku::Degil, ek2)) = yuklem(&k2) {
                    self.ilerle();
                    op = match op {
                        IkiliOp::Esit => IkiliOp::EsitDegil,
                        IkiliOp::Buyuk => IkiliOp::KucukEsit,
                        IkiliOp::Kucuk => IkiliOp::BuyukEsit,
                        IkiliOp::BuyukEsit => IkiliOp::Kucuk,
                        IkiliOp::KucukEsit => IkiliOp::Buyuk,
                        o => o,
                    };
                    ek = ek2.to_string();
                }
            }
        }
        self.yuklem_eki_denetle(tur, &ek, ykonum)?;
        Ok(Ifade::yeni(
            IfadeTuru::Ikili(op, Box::new(sol), Box::new(sag)),
            konum,
        ))
    }

    fn yuklem_eki_denetle(&self, tur: KosulTuru, ek: &str, konum: Konum) -> Sonuc<()> {
        match (tur, ek) {
            (KosulTuru::Eger, "ken") => Err(Hata::yeni(
                konum,
                "'-ken' eki döngüler içindir; koşulda '-se' kullanın",
            )
            .ipucu("x 4'ten büyükse:")),
            (KosulTuru::Surece, "se" | "sa") => Err(Hata::yeni(
                konum,
                "'-se' eki koşullar içindir; döngüde '-ken' ya da 'olduğu sürece' kullanın",
            )
            .ipucu("x 10'dan küçükken:")),
            _ => Ok(()),
        }
    }

    // ---------- ifadeler ----------

    /// Ek almaması gereken ifade (atama sağ tarafı, parametreler...).
    /// Ekli bir öğeyle başlayan ifade yalnızca bir kullanıcı fiili çağrısı
    /// olabilir: `y = 5'i karele`, `y = a'yı b'ye böl`.
    fn duz_ifade(&mut self) -> Sonuc<Ifade> {
        let (ifade, hal, konum) = self.ekli_ifade()?;
        match hal {
            None => Ok(ifade),
            Some(h) => self.fiil_tamamla(ifade, h, konum),
        }
    }

    /// İlk ekli öğesi okunmuş bir fiil cümlesini fiile kadar okur.
    fn fiil_tamamla(&mut self, ilk: Ifade, hal: Hal, konum: Konum) -> Sonuc<Ifade> {
        let mut arg = vec![(hal, ilk)];
        loop {
            if let Tok::Kelime(k) = self.bak().clone() {
                if self.t.fiiller.contains(&k) {
                    let fk = self.konum();
                    self.ilerle();
                    if let Tok::Ek(ek) = self.bak() {
                        return Err(Hata::yeni(
                            self.konum(),
                            format!("fiilin sonucu doğrudan ek alamaz ('{ek})"),
                        )
                        .ipucu(format!(
                            "fiil cümlesini parantez içine alın: (... {k})'{ek}"
                        )));
                    }
                    return Ok(Ifade::yeni(IfadeTuru::FiilCagri(k, arg), fk));
                }
            }
            let devam = matches!(
                self.bak(),
                Tok::Kelime(_) | Tok::Sayi(_) | Tok::Ondalik(_) | Tok::Metin(_)
            ) || self.op_mu("(")
                || self.op_mu("[")
                || self.op_mu("-");
            if !devam || matches!(self.bak(), Tok::Kelime(k) if ayrilmis_mi(k)) {
                let mut h = Hata::yeni(
                    konum,
                    format!("burada ek beklenmiyordu ({} bulundu)", hal.adi()),
                );
                if !self.t.fiiller.is_empty() || arg.len() > 1 {
                    h = h.ipucu("bir fiil çağırıyorsanız cümle fiille bitmeli: y = 5'i karele");
                }
                return Err(h);
            }
            let (ifade, h, k) = self.ekli_ifade()?;
            let Some(h) = h else {
                return Err(Hata::yeni(
                    k,
                    "fiil cümlesindeki her öğe bir hâl eki almalı",
                ));
            };
            arg.push((h, ifade));
        }
    }

    /// Sonu bir hâl eki taşıyabilen ifade.
    fn ekli_ifade(&mut self) -> Sonuc<(Ifade, Option<Hal>, Konum)> {
        let onceki = self.yakalanan.take();
        let konum = self.konum();
        let ifade = self.veya_ifadesi()?;
        let yakalanan = std::mem::replace(&mut self.yakalanan, onceki);
        Ok(match yakalanan {
            Some((h, k)) => (ifade, Some(h), k),
            None => (ifade, None, konum),
        })
    }

    fn ikili(&mut self, op: IkiliOp, sol: Ifade, sag: Ifade) -> Ifade {
        let k = sol.konum;
        Ifade::yeni(IfadeTuru::Ikili(op, Box::new(sol), Box::new(sag)), k)
    }

    fn veya_ifadesi(&mut self) -> Sonuc<Ifade> {
        let mut sol = self.ve_ifadesi()?;
        while self.yakalanan.is_none() && self.kelime_mi("veya") && !self.yuklem_veya_mi() {
            self.ilerle();
            let sag = self.ve_ifadesi()?;
            sol = self.ikili(IkiliOp::Veya, sol, sag);
        }
        Ok(sol)
    }

    /// `büyük veya eşit` içindeki `veya` mantıksal 'veya' değildir.
    fn yuklem_veya_mi(&self) -> bool {
        matches!(self.bak_n(1), Tok::Kelime(k) if yuklem(k).is_some())
    }

    fn ve_ifadesi(&mut self) -> Sonuc<Ifade> {
        let mut sol = self.degil_ifadesi()?;
        while self.yakalanan.is_none() && self.kelime_mi("ve") {
            self.ilerle();
            let sag = self.degil_ifadesi()?;
            sol = self.ikili(IkiliOp::Ve, sol, sag);
        }
        Ok(sol)
    }

    fn degil_ifadesi(&mut self) -> Sonuc<Ifade> {
        if self.kelime_mi("değil") {
            let k = self.konum();
            self.ilerle();
            let ic = self.degil_ifadesi()?;
            return Ok(Ifade::yeni(
                IfadeTuru::Tekli(TekliOp::Degil, Box::new(ic)),
                k,
            ));
        }
        self.karsilastirma()
    }

    fn karsilastirma(&mut self) -> Sonuc<Ifade> {
        let mut sol = self.toplama()?;
        while self.yakalanan.is_none() {
            let op = match self.bak() {
                Tok::Op("==") => IkiliOp::Esit,
                Tok::Op("!=") => IkiliOp::EsitDegil,
                Tok::Op("<") => IkiliOp::Kucuk,
                Tok::Op(">") => IkiliOp::Buyuk,
                Tok::Op("<=") => IkiliOp::KucukEsit,
                Tok::Op(">=") => IkiliOp::BuyukEsit,
                _ => break,
            };
            self.ilerle();
            let sag = self.toplama()?;
            sol = self.ikili(op, sol, sag);
        }
        Ok(sol)
    }

    fn toplama(&mut self) -> Sonuc<Ifade> {
        let mut sol = self.carpma()?;
        while self.yakalanan.is_none() {
            let op = match self.bak() {
                Tok::Op("+") => IkiliOp::Topla,
                Tok::Op("-") => IkiliOp::Cikar,
                _ => break,
            };
            self.ilerle();
            let sag = self.carpma()?;
            sol = self.ikili(op, sol, sag);
        }
        Ok(sol)
    }

    fn carpma(&mut self) -> Sonuc<Ifade> {
        let mut sol = self.tekli()?;
        while self.yakalanan.is_none() {
            let op = match self.bak() {
                Tok::Op("*") => IkiliOp::Carp,
                Tok::Op("/") => IkiliOp::Bol,
                Tok::Op("//") => IkiliOp::TamBol,
                Tok::Op("%") => IkiliOp::Mod,
                _ => break,
            };
            self.ilerle();
            let sag = self.tekli()?;
            sol = self.ikili(op, sol, sag);
        }
        Ok(sol)
    }

    fn tekli(&mut self) -> Sonuc<Ifade> {
        if self.op_mu("-") {
            let k = self.konum();
            self.ilerle();
            let ic = self.tekli()?;
            match ic.tur {
                IfadeTuru::Sayi(n) => return Ok(Ifade::yeni(IfadeTuru::Sayi(-n), k)),
                IfadeTuru::Ondalik(n) => return Ok(Ifade::yeni(IfadeTuru::Ondalik(-n), k)),
                _ => {}
            }
            return Ok(Ifade::yeni(
                IfadeTuru::Tekli(TekliOp::Eksi, Box::new(ic)),
                k,
            ));
        }
        self.sonek()
    }

    /// Temel öğe + indeks + üye erişimi + hâl eki.
    fn sonek(&mut self) -> Sonuc<Ifade> {
        let mut ifade = self.temel()?;
        while self.yakalanan.is_none() {
            if self.op_mu("[") {
                let k = self.konum();
                self.ilerle();
                let i = self.duz_ifade()?;
                self.bekle_op("]", "indeksin sonunda")?;
                ifade = Ifade::yeni(IfadeTuru::Indeks(Box::new(ifade), Box::new(i)), k);
            } else if *self.bak() == Tok::Uye {
                ifade = self.uye(ifade)?;
            } else {
                break;
            }
        }
        if let Tok::Ek(ek) = self.bak().clone() {
            let k = self.konum();
            if self.yakalanan.is_some() {
                return Err(Hata::yeni(k, "bu kelime zaten bir ek almış"));
            }
            self.ilerle();
            let Some(hal) = hal_bul(&ek) else {
                return Err(Hata::yeni(k, format!("tanınmayan ek '{ek}'"))
                    .ipucu("kullanılabilen hâl ekleri: -i, -e, -den, -de, -le, -in"));
            };
            self.yakalanan = Some((hal, k));
        }
        // `listenin uzunluğu`, `listenin uzunluğunu yaz`
        if let Some((Hal::Ilgi, k)) = self.yakalanan {
            let uzunluk = match self.bak() {
                Tok::Kelime(w) => uzunluk_kelimesi(w),
                _ => None,
            };
            let Some(ek) = uzunluk else {
                return Err(
                    Hata::yeni(k, "ilgi hâlinden (-in) sonra 'uzunluğu' bekleniyordu")
                        .ipucu("sayıların uzunluğu"),
                );
            };
            let ek_konum = self.konum();
            self.ilerle();
            self.yakalanan = ek.map(|h| (h, ek_konum));
            let kk = ifade.konum;
            ifade = Ifade::yeni(IfadeTuru::Cagri("uzunluk".into(), vec![ifade]), kk);
            if let Tok::Ek(e) = self.bak().clone() {
                // `sayıların uzunluğu'nu yaz`
                if self.yakalanan.is_some() {
                    return Err(Hata::yeni(self.konum(), "bu kelime zaten bir ek almış"));
                }
                let k = self.konum();
                self.ilerle();
                let hal =
                    hal_bul(&e).ok_or_else(|| Hata::yeni(k, format!("tanınmayan ek '{e}'")))?;
                self.yakalanan = Some((hal, k));
            }
        }
        Ok(ifade)
    }

    /// `nesne.alan`, `nesne.yöntem(...)`; alan adı ek almış olabilir: `ürün.fiyatı`
    fn uye(&mut self, nesne: Ifade) -> Sonuc<Ifade> {
        self.ilerle(); // .
        let konum = self.konum();
        let Tok::Kelime(ad) = self.bak().clone() else {
            return Err(self.beklenmeyen("noktadan sonra alan ya da yöntem adı"));
        };
        self.ilerle();
        if self.op_mu("(") {
            self.ilerle();
            let mut arg = Vec::new();
            while !self.op_mu(")") {
                arg.push(self.duz_ifade()?);
                if !self.op_mu(")") {
                    self.bekle_op(",", "bağımsız değişkenler arasında")?;
                }
            }
            self.ilerle();
            return Ok(Ifade::yeni(
                IfadeTuru::Metod(Box::new(nesne), ad, arg),
                konum,
            ));
        }
        // Seçenek değeri adıyla yazılmışsa ek çözümlemesi yapılmaz: `Durum.yolda`
        let secenek_degeri = match &nesne.tur {
            IfadeTuru::ModelAdi(m) => self.t.secenekler.get(m).is_some_and(|d| d.contains(&ad)),
            _ => false,
        };
        let alan = if secenek_degeri || matches!(self.bak(), Tok::Ek(_)) {
            self.t.alanlar.asil_isim(&ad).unwrap_or(&ad).to_string()
        } else {
            match self.t.alanlar.cozumle(&ad) {
                Cozum::Isim(a) => a,
                Cozum::EkliIsim(a, hal) => {
                    self.yakalanan = Some((hal, konum));
                    a
                }
                Cozum::Belirsiz(adaylar) => {
                    return Err(Hata::yeni(
                        konum,
                        format!("'{ad}' belirsiz: {}", adaylar.join(" ya da ")),
                    )
                    .ipucu("eki kesme işaretiyle ayırın: ürün.ad'ı"))
                }
                Cozum::Bilinmiyor => ad,
            }
        };
        Ok(Ifade::yeni(
            IfadeTuru::Alan(Box::new(nesne), alan, 0),
            konum,
        ))
    }

    /// `Ürün(ad: "Kalem", fiyat: 12.5)`
    fn kurucu(&mut self, model: String, konum: Konum) -> Sonuc<Ifade> {
        self.ilerle(); // (
        let mut arg: Vec<(String, Ifade)> = Vec::new();
        while !self.op_mu(")") {
            let ak = self.konum();
            let alan = match (self.bak().clone(), self.bak_n(1)) {
                (Tok::Kelime(a), Tok::Op(":")) => a,
                _ => {
                    return Err(
                        Hata::yeni(ak, "model kurucusunda değerler alan adıyla verilir")
                            .ipucu(format!("{model}(ad: \"Kalem\", fiyat: 12.5)")),
                    )
                }
            };
            self.ilerle();
            self.ilerle();
            let d = self.duz_ifade()?;
            if arg.iter().any(|(a, _)| *a == alan) {
                return Err(Hata::yeni(ak, format!("'{alan}' alanı iki kez verilmiş")));
            }
            arg.push((alan, d));
            if !self.op_mu(")") {
                self.bekle_op(",", "alanlar arasında")?;
            }
        }
        self.ilerle();
        Ok(Ifade::yeni(IfadeTuru::Kurucu(model, arg), konum))
    }

    fn temel(&mut self) -> Sonuc<Ifade> {
        let konum = self.konum();
        match self.bak().clone() {
            Tok::Sayi(n) => {
                self.ilerle();
                Ok(Ifade::yeni(IfadeTuru::Sayi(n), konum))
            }
            Tok::Ondalik(n) => {
                self.ilerle();
                Ok(Ifade::yeni(IfadeTuru::Ondalik(n), konum))
            }
            Tok::Metin(m) => {
                self.ilerle();
                Ok(Ifade::yeni(IfadeTuru::Metin(m), konum))
            }
            Tok::Op("(") => {
                self.ilerle();
                let i = self.duz_ifade()?;
                self.bekle_op(")", "kapanış parantezi")?;
                Ok(i)
            }
            Tok::Op("[") => {
                self.ilerle();
                let mut ogeler = Vec::new();
                while !self.op_mu("]") {
                    ogeler.push(self.duz_ifade()?);
                    if !self.op_mu("]") {
                        self.bekle_op(",", "liste öğeleri arasında")?;
                    }
                }
                self.ilerle();
                Ok(Ifade::yeni(IfadeTuru::Liste(ogeler), konum))
            }
            Tok::Op("{") => {
                self.ilerle();
                let mut ciftler = Vec::new();
                while !self.op_mu("}") {
                    let a = self.duz_ifade()?;
                    self.bekle_op(":", "anahtar ile değer arasında")?;
                    let d = self.duz_ifade()?;
                    ciftler.push((a, d));
                    if !self.op_mu("}") {
                        self.bekle_op(",", "sözlük öğeleri arasında")?;
                    }
                }
                self.ilerle();
                Ok(Ifade::yeni(IfadeTuru::Sozluk(ciftler), konum))
            }
            Tok::Kelime(k) => self.kelime(k, konum),
            t => Err(Hata::yeni(
                konum,
                format!("ifade bekleniyordu, {} bulundu", tok_adi(&t)),
            )),
        }
    }

    fn kelime(&mut self, k: String, konum: Konum) -> Sonuc<Ifade> {
        match k.as_str() {
            "doğru" | "yanlış" => {
                self.ilerle();
                return Ok(Ifade::yeni(IfadeTuru::Mantik(k == "doğru"), konum));
            }
            _ if uzunluk_kelimesi(&k).is_some() => {
                return Err(Hata::yeni(
                    konum,
                    "'uzunluğu' ilgi hâlindeki bir isimden sonra gelmeli",
                )
                .ipucu("sayıların uzunluğu"))
            }
            _ => {}
        }
        if ayrilmis_mi(&k) {
            return Err(Hata::yeni(konum, format!("'{k}' burada beklenmiyordu")));
        }
        self.ilerle();

        // Model: kurucu `Ürün(...)` ya da yöntem alıcısı `Ürün.hepsi()`
        if self.t.modeller.contains(&k) {
            if self.op_mu("(") {
                return self.kurucu(k, konum);
            }
            if *self.bak() == Tok::Uye {
                return Ok(Ifade::yeni(IfadeTuru::ModelAdi(k), konum));
            }
            return Err(
                Hata::yeni(konum, format!("'{k}' bir model adı")).ipucu(format!(
                    "yeni bir nesne için: {k}(alan: değer)  ya da kayıtlar için: {k}.hepsi()"
                )),
            );
        }

        // Seçenek türü: değer `Renk.kırmızı`, değerler `Renk.hepsi()`, çevirme `Renk("mavi")`
        if self.t.secenekler.contains_key(&k) && !self.op_mu("(") {
            if *self.bak() == Tok::Uye {
                return Ok(Ifade::yeni(IfadeTuru::ModelAdi(k), konum));
            }
            return Err(
                Hata::yeni(konum, format!("'{k}' bir seçenek türü")).ipucu(format!(
                    "bir değeri için: {k}.‹değer›  ·  hepsi için: {k}.hepsi()  ·  metinden: {k}(m)"
                )),
            );
        }

        // İşlev çağrısı
        if self.op_mu("(") {
            self.ilerle();
            let mut arg = Vec::new();
            while !self.op_mu(")") {
                arg.push(self.duz_ifade()?);
                if !self.op_mu(")") {
                    self.bekle_op(",", "bağımsız değişkenler arasında")?;
                }
            }
            self.ilerle();
            return Ok(Ifade::yeni(IfadeTuru::Cagri(k, arg), konum));
        }

        // Bağımsız değişkensiz kullanıcı fiili: `y = zar_at`
        if self.t.fiiller.contains(&k) {
            return Ok(Ifade::yeni(IfadeTuru::FiilCagri(k, Vec::new()), konum));
        }

        // Kesme işaretiyle yazılmış ek: kök tanımlı bir isim olmalı.
        if matches!(self.bak(), Tok::Ek(_)) {
            let isim = self
                .t
                .sozluk
                .asil_isim(&k)
                .map(str::to_string)
                .ok_or_else(|| self.tanimsiz(&k, konum))?;
            return Ok(Ifade::yeni(IfadeTuru::Isim(isim), konum));
        }

        match self.t.sozluk.cozumle(&k) {
            Cozum::Isim(isim) => Ok(Ifade::yeni(IfadeTuru::Isim(isim), konum)),
            Cozum::EkliIsim(isim, hal) => {
                self.yakalanan = Some((hal, konum));
                Ok(Ifade::yeni(IfadeTuru::Isim(isim), konum))
            }
            Cozum::Belirsiz(adaylar) => Err(Hata::yeni(
                konum,
                format!("'{k}' belirsiz: {}", adaylar.join(" ya da ")),
            )
            .ipucu("ekli kullanımı kesme işaretiyle ayırın (ör. kitap'la)")),
            Cozum::Bilinmiyor => Err(self.tanimsiz(&k, konum)),
        }
    }
}

// ---------- görünümler (.ohchtml) ----------

impl Ayristirici {
    /// Görünümdeki bir kod parçasının sonu.
    fn parca_sonu(&mut self) -> Sonuc<()> {
        if *self.bak() == Tok::YeniSatir {
            self.ilerle();
        }
        if *self.bak() != Tok::Son {
            return Err(self.beklenmeyen("kod parçasının sonu"));
        }
        Ok(())
    }

    /// `@her ürün için model'den {` ya da `@her i için 1'den 5'e kadar {` başlığı.
    fn sablon_her(&mut self, konum: Konum, govde: Vec<Deyim>) -> Sonuc<Deyim> {
        let degisken = self.isim_adi("döngü değişkeni")?;
        self.bekle_kelime("için")?;
        let (kaynak, hal, hal_konum) = self.ekli_ifade()?;
        if hal != Some(Hal::Ayrilma) {
            return Err(
                Hata::yeni(hal_konum, "döngünün kaynağı ayrılma hâlinde (-den) olmalı")
                    .ipucu("@her ürün için ürünler'den {"),
            );
        }
        if matches!(self.bak(), Tok::YeniSatir | Tok::Son) {
            self.parca_sonu()?;
            return Ok(Deyim::HerListe {
                degisken,
                liste: kaynak,
                govde,
                konum,
            });
        }
        let (son, hal, hal_konum) = self.ekli_ifade()?;
        if hal != Some(Hal::Yonelme) {
            return Err(
                Hata::yeni(hal_konum, "aralığın sonu yönelme hâlinde (-e) olmalı")
                    .ipucu("@her i için 1'den 5'e kadar {"),
            );
        }
        self.bekle_kelime("kadar")?;
        self.parca_sonu()?;
        Ok(Deyim::HerAralik {
            degisken,
            bas: kaynak,
            son,
            govde,
            konum,
        })
    }
}

fn isim(ad: &str, konum: Konum) -> Ifade {
    Ifade::yeni(IfadeTuru::Isim(ad.into()), konum)
}

/// Görünüm çıktısına bir parça ekleyen deyim.
fn cikti_ekle(oge: Ifade) -> Deyim {
    let k = oge.konum;
    Deyim::Ekle {
        oge,
        liste: isim(CIKTI, k),
    }
}

fn parcalari_indir(
    t: &Rc<Tanimlar>,
    parcalar: Vec<Parca>,
    govde: &mut Vec<Deyim>,
    konum: Konum,
) -> Sonuc<()> {
    for p in parcalar {
        match p {
            Parca::Metin(m) => govde.push(cikti_ekle(Ifade::yeni(IfadeTuru::Metin(m), konum))),
            Parca::Cikti { sozcukler, ham } => {
                let mut a = Ayristirici::yeni(t, sozcukler);
                let e = a.duz_ifade()?;
                a.parca_sonu()?;
                // Başka bir görünümün çıktısı zaten HTML'dir.
                let gorunum = matches!(&e.tur, IfadeTuru::Cagri(ad, _) if ad == "görünüm");
                let k = e.konum;
                let ad = if ham || gorunum { "metin" } else { "kaçır" };
                govde.push(cikti_ekle(Ifade::yeni(
                    IfadeTuru::Cagri(ad.into(), vec![e]),
                    k,
                )));
            }
            Parca::Eger {
                kosul,
                govde: g,
                degilse,
            } => {
                let mut a = Ayristirici::yeni(t, kosul);
                let kosul = a.kosul(KosulTuru::Eger)?;
                a.bekle_op(":", "koşulun sonunda")?;
                a.parca_sonu()?;
                let mut evet = Vec::new();
                parcalari_indir(t, g, &mut evet, konum)?;
                let mut hayir = Vec::new();
                parcalari_indir(t, degilse, &mut hayir, konum)?;
                govde.push(Deyim::Eger {
                    kosul,
                    govde: evet,
                    degilse: hayir,
                });
            }
            Parca::Her {
                baslik,
                govde: g,
                konum: hk,
            } => {
                let mut ic = Vec::new();
                parcalari_indir(t, g, &mut ic, konum)?;
                let mut a = Ayristirici::yeni(t, baslik);
                govde.push(a.sablon_her(hk, ic)?);
            }
            Parca::Icerik => govde.push(cikti_ekle(isim("içerik", konum))),
        }
    }
    Ok(())
}

/// Bir görünümü metin döndüren `görünüm:<ad>` işlevine çevirir.
fn sablon_islevi(t: &Rc<Tanimlar>, s: Sablon) -> Sonuc<Islev> {
    let konum = s.konum;
    let mut parametreler: Vec<(String, Tip)> = Vec::new();
    if let Some(sozcukler) = s.model {
        let mut a = Ayristirici::yeni(t, sozcukler);
        if a.op_mu("(") {
            // @model (ürün: Ürün, hatalar: liste<metin>)
            a.ilerle();
            while !a.op_mu(")") {
                let pk = a.konum();
                let ad = a.isim_adi("değer adı")?;
                a.bekle_op(":", "değer adından sonra tipi")?;
                let tip = a.tip()?;
                if parametreler.iter().any(|(p, _)| *p == ad) {
                    return Err(Hata::yeni(pk, format!("'{ad}' iki kez yazılmış")));
                }
                parametreler.push((ad, tip));
                if !a.op_mu(")") {
                    a.bekle_op(",", "değerler arasında")?;
                }
            }
            a.ilerle();
        } else {
            parametreler.push(("model".to_string(), a.tip()?));
        }
        a.parca_sonu()?;
    }
    if s.icerik_var {
        parametreler.push(("içerik".to_string(), Tip::Metin));
        parametreler.push(("başlık".to_string(), Tip::Metin));
    }
    let mut govde = vec![Deyim::Atama {
        hedef: CIKTI.into(),
        tip: None,
        deger: Ifade::yeni(IfadeTuru::Liste(Vec::new()), konum),
        konum,
    }];
    parcalari_indir(t, s.parcalar, &mut govde, konum)?;
    let sonuc = Ifade::yeni(
        IfadeTuru::Cagri(
            "birleştir".into(),
            vec![
                isim(CIKTI, konum),
                Ifade::yeni(IfadeTuru::Metin(String::new()), konum),
            ],
        ),
        konum,
    );
    let donen = match s.duzen {
        Some((duzen, dk)) => {
            let baslik = match s.baslik {
                Some(sozcukler) => {
                    let mut a = Ayristirici::yeni(t, sozcukler);
                    let e = a.duz_ifade()?;
                    a.parca_sonu()?;
                    e
                }
                None => Ifade::yeni(IfadeTuru::Metin(String::new()), dk),
            };
            Ifade::yeni(
                IfadeTuru::Cagri(format!("görünüm:{duzen}"), vec![sonuc, baslik]),
                dk,
            )
        }
        None => {
            if let Some(b) = s.baslik.as_ref().and_then(|b| b.first()) {
                return Err(Hata::yeni(
                    b.konum,
                    "@başlık yalnızca bir @düzen kullanan görünümde anlamlıdır",
                ));
            }
            sonuc
        }
    };
    govde.push(Deyim::Dondur(Some(donen), konum));
    Ok(Islev {
        ad: format!("görünüm:{}", s.ad),
        parametreler,
        haller: Vec::new(),
        donus: Some(Tip::Metin),
        govde,
        konum,
        yereller: Vec::new(),
        rota: None,
        arayuz: false,
        dis: None,
    })
}

/// Satırdaki sözcüklerin kaynak metni (öneriler için yaklaşık yeniden yazım).
fn sozcuk_metni(sozcukler: &[Sozcuk]) -> String {
    let mut s = String::new();
    for (i, z) in sozcukler.iter().enumerate() {
        let parca = match &z.tok {
            Tok::Sayi(n) => n.to_string(),
            Tok::Ondalik(f) => f.to_string(),
            Tok::Metin(m) => format!("{m:?}"),
            Tok::Kelime(k) => k.clone(),
            Tok::Ek(e) => format!("'{e}"),
            Tok::Op(o) => o.to_string(),
            Tok::Uye => ".".into(),
            _ => continue,
        };
        let bitisik = matches!(z.tok, Tok::Ek(_) | Tok::Uye)
            || matches!(z.tok, Tok::Op(")" | "]" | "," | ":"))
            || i > 0 && matches!(sozcukler[i - 1].tok, Tok::Op("(" | "[") | Tok::Uye)
            || i > 0
                && matches!(z.tok, Tok::Op("("))
                && matches!(sozcukler[i - 1].tok, Tok::Kelime(_));
        if !s.is_empty() && !bitisik {
            s.push(' ');
        }
        if matches!(z.tok, Tok::Op(",")) {
            s.push_str(", ");
            continue;
        }
        s.push_str(&parca);
    }
    s.replace(",  ", ", ").trim().to_string()
}

fn tok_adi(t: &Tok) -> String {
    match t {
        Tok::Sayi(n) => format!("'{n}' sayısı"),
        Tok::Ondalik(n) => format!("'{n}' sayısı"),
        Tok::Metin(m) => format!("\"{m}\" metni"),
        Tok::Kelime(k) => format!("'{k}'"),
        Tok::Ek(e) => format!("'{e} eki"),
        Tok::Op(o) => format!("'{o}'"),
        Tok::Uye => "'.'".into(),
        Tok::YeniSatir => "satır sonu".into(),
        Tok::Girinti => "girinti".into(),
        Tok::Cikinti => "blok sonu".into(),
        Tok::Son => "dosya sonu".into(),
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::sozcuk::sozcukle;

    fn ayr(k: &str) -> Program {
        ayristir(sozcukle(k).unwrap()).unwrap()
    }

    fn hata(k: &str) -> String {
        ayristir(sozcukle(k).unwrap()).unwrap_err().mesaj
    }

    #[test]
    fn dene_yakala() {
        let p = ayr("dene:\n    x = 1\nyakala hata:\n    hata'yı yaz.\ndene:\n    x = 2\nyakala:\n    x = 3\n");
        assert!(matches!(&p.ana[0], Deyim::Dene { degisken: Some(d), .. } if d == "hata"));
        assert!(matches!(&p.ana[1], Deyim::Dene { degisken: None, .. }));
        assert!(hata("dene:\n    x = 1\nx = 2\n").contains("'yakala' bloğu gelmeli"));
        assert!(hata("yakala h:\n    x = 1\n").contains("'dene' bloğundan hemen sonra"));
        // `dene` ve `yakala` ayrılmış değildir: değişken adı olabilirler.
        ayr("dene = 1\nyakala = dene + 1\nyakala'yı yaz.\n");
    }

    #[test]
    fn parametre_sirasi_onemsiz() {
        let a = ayr("sayılar = []\n5'i sayılara ekle.\nsayılara 5'i ekle.\n");
        assert!(matches!(a.ana[1], Deyim::Ekle { .. }));
        assert!(matches!(a.ana[2], Deyim::Ekle { .. }));
    }

    #[test]
    fn turkce_kosul() {
        let a = ayr("x = 5\neğer x 4'ten büyükse:\n    x'i yaz.\n");
        let Deyim::Eger { kosul, .. } = &a.ana[1] else {
            panic!()
        };
        assert!(matches!(kosul.tur, IfadeTuru::Ikili(IkiliOp::Buyuk, ..)));
    }

    #[test]
    fn esit_degil() {
        let a = ayr("x = 5\neğer x 4'e eşit değilse:\n    x'i yaz.\n");
        let Deyim::Eger { kosul, .. } = &a.ana[1] else {
            panic!()
        };
        assert!(matches!(
            kosul.tur,
            IfadeTuru::Ikili(IkiliOp::EsitDegil, ..)
        ));
    }

    #[test]
    fn surece_dongusu() {
        let a = ayr(
            "x = 0\nx 10'dan küçük olduğu sürece:\n    x += 1\nx 20'den küçükken:\n    x += 1\n",
        );
        assert!(matches!(a.ana[1], Deyim::Surece { .. }));
        assert!(matches!(a.ana[2], Deyim::Surece { .. }));
    }

    #[test]
    fn ek_tum_ifadeye_ait() {
        let a = ayr("a = 1\nb = 2\na + b'yi yaz.\n");
        let Deyim::Yaz(i) = &a.ana[2] else { panic!() };
        assert!(matches!(i.tur, IfadeTuru::Ikili(IkiliOp::Topla, ..)));
    }

    #[test]
    fn yanlis_hal() {
        assert!(hata("x = 1\neğer x 4'e büyükse:\n    x'i yaz.\n").contains("ayrılma"));
        assert!(hata("x = 1\nx'e yaz.\n").contains("belirtme"));
    }

    #[test]
    fn uzunlugu_ekli() {
        let a = ayr("l = [1]\nlistenin = 0\nl'nin uzunluğunu yaz.\n");
        let Deyim::Yaz(i) = &a.ana[2] else { panic!() };
        assert!(matches!(&i.tur, IfadeTuru::Cagri(ad, _) if ad == "uzunluk"));
    }

    #[test]
    fn fiil_tanimi_ve_cagrisi() {
        let a =
            ayr("fiil x'i (y: ondalık)'ye böl -> ondalık:\n    döndür x / y\nz = 2.0'ye 5'i böl\n");
        assert_eq!(a.islevler[0].ad, "böl");
        assert_eq!(a.islevler[0].haller, vec![Hal::Belirtme, Hal::Yonelme]);
        assert_eq!(a.islevler[0].parametreler[1].1, Tip::Ondalik);
        let Deyim::Atama { deger, .. } = &a.ana[0] else {
            panic!()
        };
        let IfadeTuru::FiilCagri(ad, arg) = &deger.tur else {
            panic!()
        };
        assert_eq!(ad, "böl");
        assert_eq!(arg[0].0, Hal::Yonelme);
    }

    #[test]
    fn tanimsiz_isim() {
        assert!(hata("masayı yaz.\n").contains("tanımsız"));
    }
}
