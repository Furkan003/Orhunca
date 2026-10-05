//! Orhunca programının Python ya da JavaScript karşılığı (öğrenme amaçlı):
//! `orhunca çevir dosya.ohc --dil python`. Her çıktı satırı, geldiği Orhunca
//! satırını bilir; Stüdyo iki dili yan yana gösterir.
//!
//! Tip denetiminden geçmiş ağaç çevrilir (fiil çağrıları sıralanmış, tipler belli).
//! Arayüz bölümleri, web yolları ve model kayıt işlemleri Orhunca'ya özeldir;
//! bunlar için açıklama satırı yazılır.

use crate::agac::SECENEK_CEVIR;
use crate::agac::*;
use crate::on_kutuphane::{HATA_SATIRDA, ON_EK, YERLESIK_ON_EK};
use std::collections::{BTreeSet, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dil {
    Python,
    JavaScript,
}

impl Dil {
    pub fn coz(ad: &str) -> Option<Dil> {
        match ad.to_lowercase().as_str() {
            "python" | "py" => Some(Dil::Python),
            "javascript" | "js" => Some(Dil::JavaScript),
            _ => None,
        }
    }
}

/// Bir çıktı satırı ve geldiği Orhunca satırı (ana dosyada; 1'den başlar).
#[derive(Clone, Debug)]
pub struct Satir {
    pub kaynak: Option<usize>,
    pub metin: String,
}

pub fn cevir(p: &Program, dil: Dil) -> Vec<Satir> {
    let mut c = Cevirici {
        dil,
        satirlar: Vec::new(),
        girinti: 0,
        ithal: BTreeSet::new(),
        yardimci: BTreeSet::new(),
        kapsam: vec![HashSet::new()],
        kaynak: None,
        secenekler: p
            .secenekler
            .iter()
            .map(|s| (s.ad.clone(), s.degerler.clone()))
            .collect(),
        model_adi: String::new(),
        karsiliksiz: BTreeSet::new(),
        alan_varsayilan: Default::default(),
    };
    c.program(p);
    // Gerekli içe aktarmalar ve yardımcı işlevler en başa
    let mut bas = Vec::new();
    let s = |m: &str| Satir {
        kaynak: None,
        metin: m.to_string(),
    };
    match dil {
        Dil::Python => {
            for m in &c.ithal {
                bas.push(s(&format!("import {m}")));
            }
        }
        Dil::JavaScript => {
            if c.ithal.contains("fs") {
                bas.push(s("const fs = require(\"fs\");"));
            }
        }
    }
    if !c.ithal.is_empty() {
        bas.push(s(""));
    }
    if !c.karsiliksiz.is_empty() {
        let isaret = if dil == Dil::Python { "#" } else { "//" };
        let liste: Vec<&str> = c.karsiliksiz.iter().map(String::as_str).collect();
        bas.push(s(&format!(
            "{isaret} Not: {} Orhunca'ya özeldir; bu dilde karşılığını kendiniz yazmalısınız.",
            liste.join(", ")
        )));
        bas.push(s(""));
    }
    for y in &c.yardimci {
        for satir in yardimci_kodu(dil, y).lines() {
            bas.push(s(satir));
        }
        bas.push(s(""));
    }
    bas.extend(c.satirlar);
    while bas.last().is_some_and(|s| s.metin.is_empty()) {
        bas.pop();
    }
    bas
}

/// Çeviriyi düz metin olarak verir.
pub fn metin(satirlar: &[Satir]) -> String {
    let mut m: String = satirlar
        .iter()
        .map(|s| s.metin.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    m.push('\n');
    m
}

fn yardimci_kodu(dil: Dil, ad: &str) -> &'static str {
    match (dil, ad) {
        (Dil::Python, "bul") => {
            "def bul(x, aranan):\n    if isinstance(x, str):\n        return x.find(aranan)\n    return x.index(aranan) if aranan in x else -1"
        }
        (Dil::Python, "dosya_sil") => {
            "def dosya_sil(yol):\n    if os.path.exists(yol):\n        os.remove(yol)\n        return True\n    return False"
        }
        (Dil::Python, "sayı_mı") => {
            "def sayı_mı(m):\n    try:\n        int(m)\n        return True\n    except ValueError:\n        return False"
        }
        (Dil::Python, "ondalık_mı") => {
            "def ondalık_mı(m):\n    try:\n        float(m)\n        return True\n    except ValueError:\n        return False"
        }
        (Dil::JavaScript, "sayı_mı") => "function sayı_mı(m) {\n  return /^\\s*[-+]?\\d+\\s*$/.test(m);\n}",
        (Dil::JavaScript, "ondalık_mı") => {
            "function ondalık_mı(m) {\n  return m.trim() !== \"\" && !isNaN(Number(m));\n}"
        }
        (Dil::JavaScript, "rastgele") => {
            "function rastgele(a, b) {\n  return a + Math.floor(Math.random() * (b - a + 1));\n}"
        }
        _ => "",
    }
}

const PYTHON_AYRILMIS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield", "None",
    "True", "False", "print", "input", "len", "str", "int", "float", "list", "dict", "range",
    "sum", "max", "min", "type", "open",
];
const JS_AYRILMIS: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "await",
    "of",
    "Math",
    "console",
    "String",
    "Number",
    "Object",
    "JSON",
    "prompt",
];

struct Cevirici {
    dil: Dil,
    satirlar: Vec<Satir>,
    girinti: usize,
    ithal: BTreeSet<&'static str>,
    yardimci: BTreeSet<&'static str>,
    /// JavaScript: tanımlanmış değişkenler (ilk atamada `let`)
    kapsam: Vec<HashSet<String>>,
    kaynak: Option<usize>,
    /// Seçenek türleri ve değerleri (`Renk.hepsi()`)
    secenekler: Vec<(String, Vec<String>)>,
    /// Çevrilen modelin adı (kendine dönen alan başta boştur)
    model_adi: String,
    /// Karşılığı olmayan (Orhunca'ya özel) yerleşikler ve yöntemler
    karsiliksiz: BTreeSet<String>,
    /// Modellerin alanlarının varsayılan değerleri (çevrilmiş): kurucuda yazılmayanlar atlanır
    alan_varsayilan: std::collections::HashMap<(String, String), String>,
}

/// Ön kütüphanede çağıranın satırını son argüman olarak alan işlevler
const SATIRLI: &[&str] = &[
    "tekrarla",
    "gün_ekle",
    "gün_farkı",
    "haftanın_günü",
    "tarih_yazısı",
    "eşleşir",
    "desen_bul",
    "eşleşmeler",
    "desen_değiştir",
    "desen_böl",
];

/// İşlem önceliği (büyük olan daha sıkı bağlar)
const P_ATOM: u8 = 20;

impl Cevirici {
    fn py(&self) -> bool {
        self.dil == Dil::Python
    }

    fn yaz(&mut self, m: impl Into<String>) {
        let girinti = if self.py() { "    " } else { "  " };
        self.satirlar.push(Satir {
            kaynak: self.kaynak,
            metin: format!("{}{}", girinti.repeat(self.girinti), m.into()),
        });
    }

    fn bos_satir(&mut self) {
        if self.satirlar.last().is_some_and(|s| !s.metin.is_empty()) {
            self.satirlar.push(Satir {
                kaynak: None,
                metin: String::new(),
            });
        }
    }

    fn yorum(&mut self, m: &str) {
        let isaret = if self.py() { "#" } else { "//" };
        self.yaz(format!("{isaret} {m}"));
    }

    fn konum(&mut self, k: crate::hata::Konum) {
        self.kaynak = (k.dosya == 0).then_some(k.satir);
    }

    fn ad(&self, a: &str) -> String {
        let a = a
            .trim_start_matches(ON_EK)
            .trim_start_matches(YERLESIK_ON_EK);
        let ayrilmis = if self.py() {
            PYTHON_AYRILMIS
        } else {
            JS_AYRILMIS
        };
        if ayrilmis.contains(&a) {
            format!("{a}_")
        } else {
            a.to_string()
        }
    }

    fn blok_ac(&mut self, bas: String) {
        if self.py() {
            self.yaz(format!("{bas}:"));
        } else {
            self.yaz(format!("{bas} {{"));
        }
        self.girinti += 1;
    }

    fn blok_kapat(&mut self) {
        self.girinti -= 1;
        if !self.py() {
            let k = self.kaynak;
            self.kaynak = None;
            self.yaz("}");
            self.kaynak = k;
        }
    }

    fn govde(&mut self, g: &[Deyim]) {
        let once = self.satirlar.len();
        for d in g {
            self.deyim(d);
        }
        if self.py() && self.satirlar.len() == once {
            self.yaz("pass");
        }
    }

    // -----------------------------------------------------------------------
    // Program
    // -----------------------------------------------------------------------

    fn program(&mut self, p: &Program) {
        for (ad, deger) in &p.sabitler {
            self.konum(deger.konum);
            let d = self.ifade(deger);
            if self.py() {
                self.yaz(format!("{} = {d}", self.ad(ad)));
            } else {
                self.yaz(format!("const {} = {d};", self.ad(ad)));
                self.kapsam[0].insert(ad.clone());
            }
        }
        if !p.sabitler.is_empty() {
            self.bos_satir();
        }
        for s in &p.secenekler {
            self.konum(s.konum);
            if self.py() {
                self.blok_ac(format!("class {}", s.ad));
                for d in &s.degerler {
                    self.yaz(format!("{} = {:?}", self.ad(d), d));
                }
                self.girinti -= 1;
            } else {
                let ic: Vec<String> = s
                    .degerler
                    .iter()
                    .map(|d| format!("{d}: {}", metin_sabiti(d)))
                    .collect();
                self.yaz(format!(
                    "const {} = Object.freeze({{ {} }});",
                    s.ad,
                    ic.join(", ")
                ));
            }
            self.bos_satir();
        }
        // Yerleşik modeller (İstek, Yanıt...) programın dosyalarında tanımlı değildir
        for m in p
            .modeller
            .iter()
            .filter(|m| m.konum.dosya < p.dosyalar.len())
        {
            self.model(m);
        }
        for d in &p.durumlar {
            self.konum(d.konum);
            let v = self.ifade(&d.deger);
            if self.py() {
                self.yaz(format!("{} = {v}", self.ad(&d.ad)));
            } else {
                self.yaz(format!("let {} = {v};", self.ad(&d.ad)));
                self.kapsam[0].insert(d.ad.clone());
            }
        }
        let mut ozel = false;
        for f in &p.islevler {
            if f.ad.starts_with(ON_EK) || f.ad.starts_with('‹') || f.konum.dosya >= p.dosyalar.len()
            {
                continue;
            }
            if f.arayuz || f.rota.is_some() {
                ozel = true;
                continue;
            }
            self.islev(f);
        }
        if ozel {
            self.kaynak = None;
            self.yorum("Bu programın arayüz ya da web bölümü Orhunca'ya özeldir ve çevrilmedi.");
            if self.py() {
                self.yorum("Python'da arayüz için tkinter, web için Flask kullanılır.");
            } else {
                self.yorum("JavaScript'te arayüz için HTML ve DOM, web için Express kullanılır.");
            }
            self.bos_satir();
        }
        self.govde_ana(&p.ana);
    }

    fn govde_ana(&mut self, g: &[Deyim]) {
        for d in g {
            self.deyim(d);
        }
    }

    fn varsayilan(&self, t: &Tip) -> String {
        match t {
            Tip::Sayi => "0".into(),
            Tip::Ondalik => "0.0".into(),
            Tip::Metin => "\"\"".into(),
            Tip::Mantik => self.mantik(false),
            Tip::Liste(_) => "[]".into(),
            Tip::Sozluk(..) => "{}".into(),
            // İç model başta varsayılan bir nesnedir; kendi modeline dönen alan boştur
            Tip::Model(m) if *m != self.model_adi => {
                if self.py() {
                    format!("{m}()")
                } else {
                    format!("new {m}()")
                }
            }
            Tip::Secenek(s) => self
                .secenekler
                .iter()
                .find(|(a, _)| a == s)
                .and_then(|(_, d)| d.first())
                .map(|d| metin_sabiti(d))
                .unwrap_or_else(|| "\"\"".into()),
            _ => self.bos(),
        }
    }

    fn bos(&self) -> String {
        if self.py() { "None" } else { "null" }.into()
    }

    fn mantik(&self, d: bool) -> String {
        match (self.py(), d) {
            (true, true) => "True",
            (true, false) => "False",
            (false, true) => "true",
            (false, false) => "false",
        }
        .into()
    }

    fn model(&mut self, m: &Model) {
        self.konum(m.konum);
        self.model_adi = m.ad.clone();
        let alanlar: Vec<(String, String, bool)> = m
            .alanlar
            .iter()
            .map(|a| {
                let v = match &a.varsayilan {
                    Some(e) => self.ifade(e),
                    None => self.varsayilan(&a.tip),
                };
                let degisken = matches!(a.tip, Tip::Liste(_) | Tip::Sozluk(..) | Tip::Model(_));
                self.alan_varsayilan
                    .insert((m.ad.clone(), a.ad.clone()), v.clone());
                (self.ad(&a.ad), v, degisken)
            })
            .collect();
        if self.py() {
            self.blok_ac(format!("class {}", m.ad));
            // Liste, sözlük ve model varsayılanları her nesneye ayrı oluşturulur
            let parametreler: Vec<String> = alanlar
                .iter()
                .map(|(a, v, d)| {
                    if *d {
                        format!("{a}=None")
                    } else {
                        format!("{a}={v}")
                    }
                })
                .collect();
            self.blok_ac(format!("def __init__(self, {})", parametreler.join(", ")));
            for (a, v, d) in &alanlar {
                if *d && v != "None" {
                    self.yaz(format!("self.{a} = {v} if {a} is None else {a}"));
                } else {
                    self.yaz(format!("self.{a} = {a}"));
                }
            }
            self.girinti -= 2;
        } else {
            self.blok_ac(format!("class {}", m.ad));
            let parametreler: Vec<String> = alanlar
                .iter()
                .map(|(a, v, _)| format!("{a} = {v}"))
                .collect();
            self.blok_ac(format!(
                "constructor({{ {} }} = {{}})",
                parametreler.join(", ")
            ));
            for (a, _, _) in &alanlar {
                self.yaz(format!("this.{a} = {a};"));
            }
            self.blok_kapat();
            self.blok_kapat();
        }
        self.bos_satir();
    }

    fn islev(&mut self, f: &Islev) {
        self.konum(f.konum);
        let parametreler: Vec<String> = f.parametreler.iter().map(|(a, _)| self.ad(a)).collect();
        if self.py() {
            self.blok_ac(format!(
                "def {}({})",
                self.ad(&f.ad),
                parametreler.join(", ")
            ));
        } else {
            self.blok_ac(format!(
                "function {}({})",
                self.ad(&f.ad),
                parametreler.join(", ")
            ));
            self.kapsam
                .push(f.parametreler.iter().map(|(a, _)| a.clone()).collect());
        }
        self.govde(&f.govde);
        if !self.py() {
            self.kapsam.pop();
        }
        self.blok_kapat();
        self.bos_satir();
    }

    // -----------------------------------------------------------------------
    // Deyimler
    // -----------------------------------------------------------------------

    fn noktali(&self, m: String) -> String {
        if self.py() {
            m
        } else {
            m + ";"
        }
    }

    fn tanimli(&self, ad: &str) -> bool {
        self.kapsam.iter().any(|k| k.contains(ad))
    }

    fn deyim(&mut self, d: &Deyim) {
        if let Some(k) = d.konum() {
            self.konum(k);
        }
        match d {
            Deyim::Atama { hedef, deger, .. } => {
                let v = self.ifade(deger);
                let a = self.ad(hedef);
                if self.py() {
                    self.yaz(format!("{a} = {v}"));
                } else if self.tanimli(hedef) {
                    self.yaz(format!("{a} = {v};"));
                } else {
                    self.kapsam.last_mut().unwrap().insert(hedef.clone());
                    self.yaz(format!("let {a} = {v};"));
                }
            }
            Deyim::IndeksAtama {
                liste,
                indeks,
                deger,
            } => {
                let m = format!(
                    "{}[{}] = {}",
                    self.oncelikli(liste, P_ATOM),
                    self.ifade(indeks),
                    self.ifade(deger)
                );
                let m = self.noktali(m);
                self.yaz(m);
            }
            Deyim::AlanAtama {
                nesne, alan, deger, ..
            } => {
                let m = format!(
                    "{}.{} = {}",
                    self.oncelikli(nesne, P_ATOM),
                    self.ad(alan),
                    self.ifade(deger)
                );
                let m = self.noktali(m);
                self.yaz(m);
            }
            Deyim::Yaz(e) => {
                let v = self.ifade(e);
                if self.py() {
                    self.yaz(format!("print({v})"));
                } else {
                    self.yaz(format!("console.log({v});"));
                }
            }
            Deyim::Ekle { oge, liste } => {
                let m = format!(
                    "{}.{}({})",
                    self.oncelikli(liste, P_ATOM),
                    if self.py() { "append" } else { "push" },
                    self.ifade(oge)
                );
                let m = self.noktali(m);
                self.yaz(m);
            }
            Deyim::Cikar { oge, liste } => {
                let l = self.oncelikli(liste, P_ATOM);
                let o = self.ifade(oge);
                if self.py() {
                    self.yaz(format!("{l}.remove({o})"));
                } else {
                    self.yaz(format!("{l}.splice({l}.indexOf({o}), 1);"));
                }
            }
            Deyim::DosyayaYaz { deger, yol } => {
                let v = self.metne(deger);
                let y = self.ifade(yol);
                if self.py() {
                    self.yaz(format!("open({y}, \"w\", encoding=\"utf-8\").write({v})"));
                } else {
                    self.ithal.insert("fs");
                    self.yaz(format!("fs.writeFileSync({y}, {v});"));
                }
            }
            Deyim::Sirala(e) => {
                let l = self.oncelikli(e, P_ATOM);
                if self.py() {
                    self.yaz(format!("{l}.sort()"));
                } else if matches!(&e.tip, Tip::Liste(t) if t.sayisal()) {
                    self.yaz(format!("{l}.sort((a, b) => a - b);"));
                } else {
                    self.yaz(format!("{l}.sort((a, b) => a.localeCompare(b, \"tr\"));"));
                }
            }
            Deyim::Eger {
                kosul,
                govde,
                degilse,
            } => self.eger(kosul, govde, degilse, false),
            Deyim::Surece { kosul, govde } => {
                let k = self.ifade(kosul);
                if self.py() {
                    self.blok_ac(format!("while {k}"));
                } else {
                    self.blok_ac(format!("while ({k})"));
                }
                self.govde(govde);
                self.blok_kapat();
            }
            Deyim::HerAralik {
                degisken,
                bas,
                son,
                govde,
                ..
            } => {
                let a = self.ad(degisken);
                let b = self.ifade(bas);
                if self.py() {
                    let s = match &son.tur {
                        IfadeTuru::Sayi(n) => (n + 1).to_string(),
                        _ => format!("{} + 1", self.oncelikli(son, 6)),
                    };
                    self.blok_ac(format!("for {a} in range({b}, {s})"));
                } else {
                    let s = self.ifade(son);
                    self.blok_ac(format!("for (let {a} = {b}; {a} <= {s}; {a}++)"));
                    self.kapsam.push([degisken.clone()].into());
                }
                self.govde(govde);
                if !self.py() {
                    self.kapsam.pop();
                }
                self.blok_kapat();
            }
            Deyim::HerListe {
                degisken,
                liste,
                govde,
                ..
            } => {
                let a = self.ad(degisken);
                let l = self.ifade(liste);
                if self.py() {
                    self.blok_ac(format!("for {a} in {l}"));
                } else {
                    let l = if matches!(liste.tip, Tip::Sozluk(..)) {
                        format!("Object.keys({l})")
                    } else {
                        l
                    };
                    self.blok_ac(format!("for (const {a} of {l})"));
                    self.kapsam.push([degisken.clone()].into());
                }
                self.govde(govde);
                if !self.py() {
                    self.kapsam.pop();
                }
                self.blok_kapat();
            }
            Deyim::Dondur(e, _) => {
                let m = match e {
                    Some(e) => format!("return {}", self.ifade(e)),
                    None => "return".into(),
                };
                let m = self.noktali(m);
                self.yaz(m);
            }
            Deyim::Dur(_) => {
                let m = self.noktali("break".into());
                self.yaz(m);
            }
            Deyim::Surdur(_) => {
                let m = self.noktali("continue".into());
                self.yaz(m);
            }
            Deyim::IfadeDeyimi(e) => {
                let m = self.ifade(e);
                let m = self.noktali(m);
                self.yaz(m);
            }
            Deyim::Oge(_) => {}
            Deyim::Dene {
                govde,
                degisken,
                yakala,
                ..
            } => {
                if self.py() {
                    self.blok_ac("try".into());
                    self.govde(govde);
                    self.girinti -= 1;
                    match degisken {
                        Some(d) => {
                            self.blok_ac("except Exception as hata_".into());
                            self.yaz(format!("{} = str(hata_)", self.ad(d)));
                        }
                        None => self.blok_ac("except Exception".into()),
                    }
                    self.govde(yakala);
                    self.girinti -= 1;
                } else {
                    self.blok_ac("try".into());
                    self.kapsam.push(HashSet::new());
                    self.govde(govde);
                    self.kapsam.pop();
                    self.girinti -= 1;
                    match degisken {
                        Some(d) => {
                            self.yaz("} catch (hata_) {");
                            self.girinti += 1;
                            self.kapsam.push([d.clone()].into());
                            self.yaz(format!("const {} = hata_.message;", self.ad(d)));
                        }
                        None => {
                            self.yaz("} catch {");
                            self.girinti += 1;
                            self.kapsam.push(HashSet::new());
                        }
                    }
                    self.govde(yakala);
                    self.kapsam.pop();
                    self.blok_kapat();
                }
            }
        }
    }

    fn eger(&mut self, kosul: &Ifade, govde: &[Deyim], degilse: &[Deyim], devam: bool) {
        let k = self.ifade(kosul);
        if self.py() {
            self.blok_ac(format!("{} {k}", if devam { "elif" } else { "if" }));
        } else if devam {
            self.girinti -= 1;
            self.yaz(format!("}} else if ({k}) {{"));
            self.girinti += 1;
        } else {
            self.blok_ac(format!("if ({k})"));
        }
        if !self.py() {
            self.kapsam.push(HashSet::new());
        }
        self.govde(govde);
        if !self.py() {
            self.kapsam.pop();
        }
        // değilse eğer → elif
        if let [Deyim::Eger {
            kosul,
            govde,
            degilse,
        }] = degilse
        {
            if let Some(k) = kosul.konum.dosya.eq(&0).then_some(kosul.konum.satir) {
                self.kaynak = Some(k);
            }
            if self.py() {
                self.girinti -= 1;
            }
            self.eger(kosul, govde, degilse, true);
            return;
        }
        if !degilse.is_empty() {
            if self.py() {
                self.girinti -= 1;
                self.blok_ac("else".into());
            } else {
                self.girinti -= 1;
                self.yaz("} else {");
                self.girinti += 1;
                self.kapsam.push(HashSet::new());
            }
            self.govde(degilse);
            if !self.py() {
                self.kapsam.pop();
            }
        }
        if self.py() {
            self.girinti -= 1;
        } else {
            self.blok_kapat();
        }
    }

    // -----------------------------------------------------------------------
    // İfadeler
    // -----------------------------------------------------------------------

    fn ifade(&mut self, e: &Ifade) -> String {
        self.ifade_p(e).0
    }

    /// Önceliği `en_az`dan düşükse parantez içinde
    fn oncelikli(&mut self, e: &Ifade, en_az: u8) -> String {
        let (m, p) = self.ifade_p(e);
        if p < en_az {
            format!("({m})")
        } else {
            m
        }
    }

    /// Metin olmayan değer metne çevrilir (Python'da `"a" + 5` hatadır)
    fn metne(&mut self, e: &Ifade) -> String {
        if e.tip.metin_gibi() {
            self.ifade(e)
        } else if self.py() {
            format!("str({})", self.ifade(e))
        } else {
            format!("String({})", self.ifade(e))
        }
    }

    fn ifade_p(&mut self, e: &Ifade) -> (String, u8) {
        let py = self.py();
        match &e.tur {
            IfadeTuru::Sayi(n) => (n.to_string(), if *n < 0 { 12 } else { P_ATOM }),
            IfadeTuru::Ondalik(x) => {
                let mut m = format!("{x}");
                if !m.contains(['.', 'e', 'E', 'i', 'N']) {
                    m.push_str(".0");
                }
                (m, P_ATOM)
            }
            IfadeTuru::Metin(m) => (metin_sabiti(m), P_ATOM),
            IfadeTuru::Mantik(d) => (self.mantik(*d), P_ATOM),
            IfadeTuru::Isim(a) => (self.ad(a), P_ATOM),
            IfadeTuru::ModelAdi(a) => (a.clone(), P_ATOM),
            IfadeTuru::Liste(l) => {
                let ic: Vec<String> = l.iter().map(|x| self.ifade(x)).collect();
                (format!("[{}]", ic.join(", ")), P_ATOM)
            }
            IfadeTuru::Sozluk(c) => {
                let ic: Vec<String> = c
                    .iter()
                    .map(|(a, d)| {
                        let a = if !py && !matches!(a.tur, IfadeTuru::Metin(_) | IfadeTuru::Sayi(_))
                        {
                            format!("[{}]", self.ifade(a))
                        } else {
                            self.ifade(a)
                        };
                        format!("{a}: {}", self.ifade(d))
                    })
                    .collect();
                (format!("{{{}}}", ic.join(", ")), P_ATOM)
            }
            IfadeTuru::Tekli(TekliOp::Eksi, a) => (format!("-{}", self.oncelikli(a, 12)), 12),
            IfadeTuru::Tekli(TekliOp::Degil, a) => {
                if py {
                    (format!("not {}", self.oncelikli(a, 3)), 3)
                } else {
                    (format!("!{}", self.oncelikli(a, 12)), 12)
                }
            }
            IfadeTuru::Ikili(op, a, b) => self.ikili(*op, a, b, &e.tip),
            IfadeTuru::Indeks(l, i) => (
                format!("{}[{}]", self.oncelikli(l, P_ATOM), self.ifade(i)),
                P_ATOM,
            ),
            IfadeTuru::Alan(n, a, _) => (
                format!("{}.{}", self.oncelikli(n, P_ATOM), self.ad(a)),
                P_ATOM,
            ),
            IfadeTuru::Kurucu(m, alanlar) => {
                // Varsayılan değerindeki alanlar yazılmaz (denetçi hepsini doldurur)
                let ic: Vec<String> = alanlar
                    .iter()
                    .filter_map(|(a, d)| {
                        let v = self.ifade(d);
                        if self.alan_varsayilan.get(&(m.clone(), a.clone())) == Some(&v) {
                            return None;
                        }
                        Some(if py {
                            format!("{}={v}", self.ad(a))
                        } else {
                            format!("{}: {v}", self.ad(a))
                        })
                    })
                    .collect();
                if py {
                    (format!("{m}({})", ic.join(", ")), P_ATOM)
                } else {
                    (format!("new {m}({{ {} }})", ic.join(", ")), P_ATOM)
                }
            }
            IfadeTuru::Metod(n, ad, arg)
                if ad == "hepsi"
                    && matches!(&n.tur, IfadeTuru::ModelAdi(m) if self.secenekler.iter().any(|(a, _)| a == m)) =>
            {
                let IfadeTuru::ModelAdi(m) = &n.tur else {
                    unreachable!()
                };
                let d = &self.secenekler.iter().find(|(a, _)| a == m).unwrap().1;
                let ic: Vec<String> = d.iter().map(|x| metin_sabiti(x)).collect();
                (format!("[{}]", ic.join(", ")), P_ATOM)
            }
            IfadeTuru::Metod(n, ad, arg) => {
                if matches!(
                    ad.as_str(),
                    "kaydet"
                        | "sil"
                        | "hepsi"
                        | "bul"
                        | "var_mı"
                        | "formdan"
                        | "json"
                        | "hatalar"
                        | "geçerli_mi"
                ) {
                    self.karsiliksiz.insert(format!(".{ad}()"));
                }
                let a: Vec<String> = arg.iter().map(|x| self.ifade(x)).collect();
                (
                    format!(
                        "{}.{}({})",
                        self.oncelikli(n, P_ATOM),
                        self.ad(ad),
                        a.join(", ")
                    ),
                    P_ATOM,
                )
            }
            IfadeTuru::FiilCagri(ad, arg) => {
                let a: Vec<String> = arg.iter().map(|(_, x)| self.ifade(x)).collect();
                (format!("{}({})", self.ad(ad), a.join(", ")), P_ATOM)
            }
            IfadeTuru::Cagri(ad, arg) => self.cagri(ad, arg),
        }
    }

    fn ikili(&mut self, op: IkiliOp, a: &Ifade, b: &Ifade, tip: &Tip) -> (String, u8) {
        use IkiliOp::*;
        let py = self.py();
        let (isaret, p): (&str, u8) = match op {
            Veya => (if py { "or" } else { "||" }, 1),
            Ve => (if py { "and" } else { "&&" }, 2),
            Esit => (if py { "==" } else { "===" }, 5),
            EsitDegil => (if py { "!=" } else { "!==" }, 5),
            Kucuk => ("<", 6),
            Buyuk => (">", 6),
            KucukEsit => ("<=", 6),
            BuyukEsit => (">=", 6),
            Topla => ("+", 8),
            Cikar => ("-", 8),
            Carp => ("*", 10),
            Bol => ("/", 10),
            TamBol => ("//", 10),
            Mod => ("%", 10),
        };
        if op == TamBol && !py {
            return (
                format!("Math.floor({} / {})", self.ifade(a), self.oncelikli(b, 11)),
                P_ATOM,
            );
        }
        // Metin birleştirme: Python'da diğer taraf metne çevrilmeli
        if op == Topla && tip.metin_gibi() && py {
            let sol = if a.tip.metin_gibi() {
                self.oncelikli(a, p)
            } else {
                format!("str({})", self.ifade(a))
            };
            let sag = if b.tip.metin_gibi() {
                self.oncelikli(b, p + 1)
            } else {
                format!("str({})", self.ifade(b))
            };
            return (format!("{sol} + {sag}"), p);
        }
        // Python'da karşılaştırmalar zincirlenir; iç içe karşılaştırma paranteze alınır
        let sag_p = if matches!(p, 5 | 6) { 7 } else { p + 1 };
        let sol_p = if matches!(p, 5 | 6) { 7 } else { p };
        (
            format!(
                "{} {isaret} {}",
                self.oncelikli(a, sol_p),
                self.oncelikli(b, sag_p)
            ),
            p,
        )
    }

    fn cagri(&mut self, ad: &str, arg: &[Ifade]) -> (String, u8) {
        if ad == SECENEK_CEVIR {
            // Seçenek değeri metin olarak taşınır: Renk("mavi") → "mavi"
            return self.ifade_p(&arg[0]);
        }
        let on_kutuphane = ad.starts_with(ON_EK);
        let ad = ad
            .trim_start_matches(ON_EK)
            .trim_start_matches(YERLESIK_ON_EK);
        let arg = if on_kutuphane && SATIRLI.contains(&ad) && !arg.is_empty() {
            &arg[..arg.len() - 1]
        } else {
            arg
        };
        let ad = match ad {
            "metin_bul" => "bul",
            "metin_içerir" => "içerir",
            "metin_ters" => "ters",
            HATA_SATIRDA => "hata_ver",
            a => a,
        };
        let py = self.py();
        let t0 = arg.first().map(|a| a.tip.clone()).unwrap_or(Tip::Bos);
        let a: Vec<String> = arg.iter().map(|x| self.ifade(x)).collect();
        let alici = |c: &mut Self, i: usize| c.oncelikli(&arg[i], P_ATOM);
        let atom = |m: String| (m, P_ATOM);
        let ithal = |c: &mut Self, m: &'static str| {
            c.ithal.insert(m);
        };
        let sozluk = matches!(t0, Tip::Sozluk(..));
        if py {
            match (ad, a.len()) {
                ("uzunluk", 1) => atom(format!("len({})", a[0])),
                ("metin", 1) => atom(format!("str({})", a[0])),
                ("sayı", 1) => atom(format!("int({})", a[0])),
                ("ondalık", 1) if t0 == Tip::Sayi => (a[0].clone(), self.ifade_p(&arg[0]).1),
                ("ondalık", 1) => atom(format!("float({}.replace(\",\", \".\"))", alici(self, 0))),
                ("yuvarla", 1) => atom(format!("round({})", a[0])),
                ("yuvarla", 2) => atom(format!("round({}, {})", a[0], a[1])),
                ("büyük_harf", 1) => atom(format!("{}.upper()", alici(self, 0))),
                ("küçük_harf", 1) => atom(format!("{}.lower()", alici(self, 0))),
                ("kırp", 1) => atom(format!("{}.strip()", alici(self, 0))),
                ("parça", 3) => atom(format!("{}[{}:{} + {}]", alici(self, 0), a[1], a[1], a[2])),
                ("böl", 2) => atom(format!("{}.split({})", alici(self, 0), a[1])),
                ("birleştir", 2) => atom(format!(
                    "{}.join({})",
                    self.oncelikli(&arg[1], P_ATOM),
                    a[0]
                )),
                ("içerir", 2) => (
                    format!(
                        "{} in {}",
                        self.oncelikli(&arg[1], 7),
                        self.oncelikli(&arg[0], 7)
                    ),
                    5,
                ),
                ("bul", 2) => {
                    if t0 == Tip::Metin {
                        atom(format!("{}.find({})", alici(self, 0), a[1]))
                    } else {
                        self.yardimci.insert("bul");
                        atom(format!("bul({}, {})", a[0], a[1]))
                    }
                }
                ("değiştir", 3) => {
                    atom(format!("{}.replace({}, {})", alici(self, 0), a[1], a[2]))
                }
                ("başlar", 2) => atom(format!("{}.startswith({})", alici(self, 0), a[1])),
                ("biter", 2) => atom(format!("{}.endswith({})", alici(self, 0), a[1])),
                ("tekrarla", 2) => (
                    format!("{} * {}", alici(self, 0), self.oncelikli(&arg[1], 11)),
                    10,
                ),
                ("harfler", 1) => atom(format!("list({})", a[0])),
                ("satırlar", 1) => atom(format!("{}.splitlines()", alici(self, 0))),
                ("ters", 1) => atom(format!("{}[::-1]", alici(self, 0))),
                ("kod", 1) => atom(format!("ord({})", a[0])),
                ("karakter", 1) => atom(format!("chr({})", a[0])),
                ("kodlar", 1) => atom(format!("[ord(h) for h in {}]", a[0])),
                ("kodlardan", 1) => atom(format!("\"\".join(chr(k) for k in {})", a[0])),
                ("sil", 2) if sozluk => atom(format!("{}.pop({})", alici(self, 0), a[1])),
                ("sil", 2) => atom(format!("{}.pop({})", alici(self, 0), a[1])),
                ("kopya", 1) => atom(format!("{}.copy()", alici(self, 0))),
                ("karıştır", 1) => {
                    ithal(self, "random");
                    atom(format!("random.shuffle({})", a[0]))
                }
                ("en_büyük", _) => atom(format!("max({})", a.join(", "))),
                ("en_küçük", _) => atom(format!("min({})", a.join(", "))),
                ("toplam", 1) => atom(format!("sum({})", a[0])),
                ("anahtarlar", 1) => atom(format!("list({}.keys())", alici(self, 0))),
                ("değerler", 1) => atom(format!("list({}.values())", alici(self, 0))),
                ("karekök", 1) => {
                    ithal(self, "math");
                    atom(format!("math.sqrt({})", a[0]))
                }
                ("üs", 2) => (
                    format!(
                        "{} ** {}",
                        self.oncelikli(&arg[0], 15),
                        self.oncelikli(&arg[1], 14)
                    ),
                    14,
                ),
                ("mutlak", 1) => atom(format!("abs({})", a[0])),
                ("sinüs" | "kosinüs" | "tanjant", 1) => {
                    ithal(self, "math");
                    let f = match ad {
                        "sinüs" => "sin",
                        "kosinüs" => "cos",
                        _ => "tan",
                    };
                    atom(format!("math.{f}({})", a[0]))
                }
                ("logaritma", 1) => {
                    ithal(self, "math");
                    atom(format!("math.log({})", a[0]))
                }
                ("logaritma", 2) => {
                    ithal(self, "math");
                    atom(format!("math.log({}, {})", a[0], a[1]))
                }
                ("rastgele", 0) => {
                    ithal(self, "random");
                    atom("random.random()".into())
                }
                ("rastgele", 2) => {
                    ithal(self, "random");
                    atom(format!("random.randint({}, {})", a[0], a[1]))
                }
                ("zaman", 0) => {
                    ithal(self, "time");
                    atom("time.time()".into())
                }
                ("bekle", 1) => {
                    ithal(self, "time");
                    atom(format!("time.sleep({})", a[0]))
                }
                ("oku", 0) => atom("input()".into()),
                ("argümanlar", 0) => {
                    ithal(self, "sys");
                    atom("sys.argv[1:]".into())
                }
                ("ortam", 1) => {
                    ithal(self, "os");
                    atom(format!("os.environ.get({}, \"\")", a[0]))
                }
                ("çık", 1) => {
                    ithal(self, "sys");
                    atom(format!("sys.exit({})", a[0]))
                }
                ("hata_ver", _) => atom(format!("raise Exception({})", a[0])),
                ("dosya_oku", 1) => atom(format!("open({}, encoding=\"utf-8\").read()", a[0])),
                ("dosyaya_yaz", 2) => atom(format!(
                    "open({}, \"w\", encoding=\"utf-8\").write({})",
                    a[0], a[1]
                )),
                ("dosyaya_ekle", 2) => atom(format!(
                    "open({}, \"a\", encoding=\"utf-8\").write({})",
                    a[0], a[1]
                )),
                ("dosya_var", 1) => {
                    ithal(self, "os");
                    atom(format!("os.path.exists({})", a[0]))
                }
                ("dosya_sil", 1) => {
                    ithal(self, "os");
                    self.yardimci.insert("dosya_sil");
                    atom(format!("dosya_sil({})", a[0]))
                }
                ("bugün", 0) => {
                    ithal(self, "datetime");
                    atom("datetime.date.today().isoformat()".into())
                }
                ("saat", 0) => {
                    ithal(self, "datetime");
                    atom("datetime.datetime.now().strftime(\"%H:%M\")".into())
                }
                ("gün_ekle", 2) => {
                    ithal(self, "datetime");
                    atom(format!(
                        "(datetime.date.fromisoformat({}) + datetime.timedelta(days={})).isoformat()",
                        a[0], a[1]
                    ))
                }
                ("gün_farkı", 2) => {
                    ithal(self, "datetime");
                    atom(format!(
                        "(datetime.date.fromisoformat({}) - datetime.date.fromisoformat({})).days",
                        a[1], a[0]
                    ))
                }
                ("eşleşir", 2) => {
                    ithal(self, "re");
                    (format!("re.fullmatch({}, {}) is not None", a[1], a[0]), 5)
                }
                ("eşleşmeler", 2) => {
                    ithal(self, "re");
                    atom(format!("re.findall({}, {})", a[1], a[0]))
                }
                ("desen_değiştir", 3) => {
                    ithal(self, "re");
                    atom(format!("re.sub({}, {}, {})", a[1], a[2], a[0]))
                }
                ("desen_böl", 2) => {
                    ithal(self, "re");
                    atom(format!("re.split({}, {})", a[1], a[0]))
                }
                ("json", 1) => {
                    ithal(self, "json");
                    atom(format!("json.dumps({}, ensure_ascii=False)", a[0]))
                }
                ("sayı_mı" | "ondalık_mı", 1) => {
                    self.yardimci.insert(if ad == "sayı_mı" {
                        "sayı_mı"
                    } else {
                        "ondalık_mı"
                    });
                    atom(format!("{ad}({})", a[0]))
                }
                ("boş_mu", 1) => (format!("{} is None", self.oncelikli(&arg[0], 7)), 5),
                _ => self.karsiliksiz_cagri(ad, &a),
            }
        } else {
            let liste_ya_da_metin = |c: &mut Self, i: usize| c.oncelikli(&arg[i], P_ATOM);
            match (ad, a.len()) {
                ("uzunluk", 1) if sozluk => atom(format!("Object.keys({}).length", a[0])),
                ("uzunluk", 1) => atom(format!("{}.length", alici(self, 0))),
                ("metin", 1) => atom(format!("String({})", a[0])),
                ("sayı", 1) => atom(format!("parseInt({})", a[0])),
                ("ondalık", 1) if t0 == Tip::Sayi => (a[0].clone(), self.ifade_p(&arg[0]).1),
                ("ondalık", 1) => atom(format!("parseFloat({})", a[0])),
                ("yuvarla", 1) => atom(format!("Math.round({})", a[0])),
                ("yuvarla", 2) => atom(format!("Number({}.toFixed({}))", alici(self, 0), a[1])),
                ("büyük_harf", 1) => {
                    atom(format!("{}.toLocaleUpperCase(\"tr\")", alici(self, 0)))
                }
                ("küçük_harf", 1) => {
                    atom(format!("{}.toLocaleLowerCase(\"tr\")", alici(self, 0)))
                }
                ("kırp", 1) => atom(format!("{}.trim()", alici(self, 0))),
                ("parça", 3) => atom(format!(
                    "{}.slice({}, {} + {})",
                    alici(self, 0),
                    a[1],
                    a[1],
                    a[2]
                )),
                ("böl", 2) => atom(format!("{}.split({})", alici(self, 0), a[1])),
                ("birleştir", 2) => atom(format!("{}.join({})", alici(self, 0), a[1])),
                ("içerir", 2) if sozluk => {
                    (format!("{} in {}", self.oncelikli(&arg[1], 7), a[0]), 6)
                }
                ("içerir", 2) => {
                    atom(format!("{}.includes({})", liste_ya_da_metin(self, 0), a[1]))
                }
                ("bul", 2) => atom(format!("{}.indexOf({})", liste_ya_da_metin(self, 0), a[1])),
                ("değiştir", 3) => {
                    atom(format!("{}.replaceAll({}, {})", alici(self, 0), a[1], a[2]))
                }
                ("başlar", 2) => atom(format!("{}.startsWith({})", alici(self, 0), a[1])),
                ("biter", 2) => atom(format!("{}.endsWith({})", alici(self, 0), a[1])),
                ("tekrarla", 2) => atom(format!("{}.repeat({})", alici(self, 0), a[1])),
                ("harfler", 1) => atom(format!("[...{}]", alici(self, 0))),
                ("satırlar", 1) => atom(format!("{}.split(\"\\n\")", alici(self, 0))),
                ("ters", 1) if t0 == Tip::Metin => {
                    atom(format!("[...{}].reverse().join(\"\")", alici(self, 0)))
                }
                ("ters", 1) => atom(format!("[...{}].reverse()", alici(self, 0))),
                ("kod", 1) => atom(format!("{}.codePointAt(0)", alici(self, 0))),
                ("karakter", 1) => atom(format!("String.fromCodePoint({})", a[0])),
                ("sil", 2) if sozluk => atom(format!("delete {}[{}]", alici(self, 0), a[1])),
                ("sil", 2) => atom(format!("{}.splice({}, 1)[0]", alici(self, 0), a[1])),
                ("kopya", 1) if sozluk => atom(format!("{{ ...{} }}", a[0])),
                ("kopya", 1) => atom(format!("[...{}]", a[0])),
                ("en_büyük", 1) => atom(format!("Math.max(...{})", a[0])),
                ("en_küçük", 1) => atom(format!("Math.min(...{})", a[0])),
                ("en_büyük", _) => atom(format!("Math.max({})", a.join(", "))),
                ("en_küçük", _) => atom(format!("Math.min({})", a.join(", "))),
                ("dosya_sil", 1) => {
                    self.ithal.insert("fs");
                    atom(format!("fs.rmSync({}, {{ force: true }})", a[0]))
                }
                ("toplam", 1) => atom(format!("{}.reduce((a, b) => a + b, 0)", alici(self, 0))),
                ("anahtarlar", 1) => atom(format!("Object.keys({})", a[0])),
                ("değerler", 1) => atom(format!("Object.values({})", a[0])),
                ("karekök", 1) => atom(format!("Math.sqrt({})", a[0])),
                ("üs", 2) => (
                    format!(
                        "{} ** {}",
                        self.oncelikli(&arg[0], 15),
                        self.oncelikli(&arg[1], 14)
                    ),
                    14,
                ),
                ("mutlak", 1) => atom(format!("Math.abs({})", a[0])),
                ("sinüs", 1) => atom(format!("Math.sin({})", a[0])),
                ("kosinüs", 1) => atom(format!("Math.cos({})", a[0])),
                ("tanjant", 1) => atom(format!("Math.tan({})", a[0])),
                ("logaritma", 1) => atom(format!("Math.log({})", a[0])),
                ("logaritma", 2) => atom(format!("Math.log({}) / Math.log({})", a[0], a[1])),
                ("rastgele", 0) => atom("Math.random()".into()),
                ("rastgele", 2) => {
                    self.yardimci.insert("rastgele");
                    atom(format!("rastgele({}, {})", a[0], a[1]))
                }
                ("zaman", 0) => atom("Date.now() / 1000".into()),
                ("oku", 0) => atom("prompt(\"\")".into()),
                ("argümanlar", 0) => atom("process.argv.slice(2)".into()),
                ("ortam", 1) => atom(format!("(process.env[{}] ?? \"\")", a[0])),
                ("çık", 1) => atom(format!("process.exit({})", a[0])),
                ("hata_ver", _) => atom(format!("throw new Error({})", a[0])),
                ("dosya_oku", 1) => {
                    self.ithal.insert("fs");
                    atom(format!("fs.readFileSync({}, \"utf8\")", a[0]))
                }
                ("dosyaya_yaz", 2) => {
                    self.ithal.insert("fs");
                    atom(format!("fs.writeFileSync({}, {})", a[0], a[1]))
                }
                ("dosyaya_ekle", 2) => {
                    self.ithal.insert("fs");
                    atom(format!("fs.appendFileSync({}, {})", a[0], a[1]))
                }
                ("dosya_var", 1) => {
                    self.ithal.insert("fs");
                    atom(format!("fs.existsSync({})", a[0]))
                }
                ("json", 1) => atom(format!("JSON.stringify({})", a[0])),
                ("sayı_mı" | "ondalık_mı", 1) => {
                    self.yardimci.insert(if ad == "sayı_mı" {
                        "sayı_mı"
                    } else {
                        "ondalık_mı"
                    });
                    atom(format!("{ad}({})", a[0]))
                }
                ("boş_mu", 1) => (format!("{} == null", self.oncelikli(&arg[0], 7)), 5),
                _ => self.karsiliksiz_cagri(ad, &a),
            }
        }
    }

    /// Kullanıcının işlevi ya da karşılığı olmayan bir yerleşik
    fn karsiliksiz_cagri(&mut self, ad: &str, a: &[String]) -> (String, u8) {
        if crate::yerlesik::bul(ad).is_some() {
            self.karsiliksiz.insert(format!("{ad}()"));
        }
        (format!("{}({})", self.ad(ad), a.join(", ")), P_ATOM)
    }
}

/// Çift tırnaklı metin sabiti (Python ve JavaScript'te aynı kaçışlar)
fn metin_sabiti(m: &str) -> String {
    let mut s = String::from("\"");
    for c in m.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\t' => s.push_str("\\t"),
            '\r' => s.push_str("\\r"),
            c => s.push(c),
        }
    }
    s.push('"');
    s
}
