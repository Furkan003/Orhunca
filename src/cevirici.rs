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
        yontemler: Vec::new(),
        yontemde: false,
        adsizlar: Default::default(),
        karsiliksiz: BTreeSet::new(),
        notlar: BTreeSet::new(),
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
    for n in &c.notlar {
        let isaret = if dil == Dil::Python { "#" } else { "//" };
        bas.push(s(&format!("{isaret} Not: {n}")));
        bas.push(s(""));
    }
    // Yardımcılar birbirini kullanabilir (orhunca_metin → orhunca_ondalık_metni).
    if dil == Dil::JavaScript && c.yardimci.contains("orhunca_metin") {
        c.yardimci.insert("orhunca_ondalık_metni");
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
        (Dil::Python, "orhunca_c") => {
            "def orhunca_c(kütüphane, ad, tipler, dönüş, *argümanlar):\n    \"\"\"C kütüphanesindeki işlevi çağırır (Orhunca'daki kütüphane bloğu).\"\"\"\n    c = {\"sayı\": ctypes.c_int64, \"sayı32\": ctypes.c_int, \"ondalık\": ctypes.c_double, \"mantık\": ctypes.c_bool, \"metin\": ctypes.c_char_p, \"yok\": None}\n    yol = kütüphane if \".\" in kütüphane or \"/\" in kütüphane else ctypes.util.find_library(kütüphane)\n    f = getattr(ctypes.CDLL(yol), ad)\n    f.argtypes = [c[t] for t in tipler]\n    f.restype = c[dönüş]\n    sonuç = f(*[a.encode() if isinstance(a, str) else a for a in argümanlar])\n    return (sonuç or b\"\").decode() if dönüş == \"metin\" else sonuç"
        }
        // Orhunca'nın davranışını birebir taklit eden yardımcılar (çeviri aynı sonucu versin)
        (Dil::Python, "orhunca_metin") => {
            "def orhunca_metin(x, iç=False):\n    \"\"\"Değeri Orhunca'nın yazdığı gibi metne çevirir: doğru/yanlış, 2.0, [\"a\", 1].\"\"\"\n    if isinstance(x, bool):\n        return \"doğru\" if x else \"yanlış\"\n    if isinstance(x, float):\n        m = f\"{x:.15g}\"\n        return m if any(h in m for h in \".eni\") else m + \".0\"\n    if isinstance(x, str):\n        return f'\"{x}\"' if iç else x\n    if isinstance(x, list):\n        return \"[\" + \", \".join(orhunca_metin(o, True) for o in x) + \"]\"\n    if isinstance(x, dict):\n        return \"{\" + \", \".join(orhunca_metin(a, True) + \": \" + orhunca_metin(d, True) for a, d in x.items()) + \"}\"\n    return str(x)"
        }
        (Dil::Python, "orhunca_büyük_harf") => {
            "def orhunca_büyük_harf(m):\n    # Türkçe: i → İ, ı → I\n    return m.replace(\"i\", \"İ\").replace(\"ı\", \"I\").upper()"
        }
        (Dil::Python, "orhunca_sıra") => {
            "def orhunca_sıra(m):\n    # Türk alfabesine göre sıralama anahtarı: büyük ve küçük harf aynı yerde\n    abc = \"abcçdefgğhıijklmnoöpqrsştuüvwxyz\"\n    ABC = \"ABCÇDEFGĞHIİJKLMNOÖPQRSŞTUÜVWXYZ\"\n    def sıra(c):\n        i = abc.find(c) if c in abc else ABC.find(c)\n        if i >= 0:\n            return 1000 + i\n        return ord(c) if ord(c) < 0x80 else 100000 + ord(c)\n    return (tuple(sıra(c) for c in m), m)"
        }
        (Dil::Python, "orhunca_küçük_harf") => {
            "def orhunca_küçük_harf(m):\n    # Türkçe: I → ı, İ → i\n    return m.replace(\"I\", \"ı\").replace(\"İ\", \"i\").lower()"
        }
        (Dil::Python, "orhunca_yuvarla") => {
            "def orhunca_yuvarla(x, basamak=0):\n    # Yarımlar sıfırdan uzağa yuvarlanır: 2.5 → 3, -2.5 → -3 (Python'un round'u çifte yuvarlar)\n    k = 10 ** basamak\n    y = math.floor(abs(x) * k + 0.5) / k\n    y = -y if x < 0 else y\n    return int(y) if basamak == 0 else y"
        }
        (Dil::Python, "orhunca_parça") => {
            "def orhunca_parça(x, baş, uzunluk):\n    baş = max(baş, 0)  # Orhunca'da negatif başlangıç 0 sayılır\n    return x[baş:baş + uzunluk]"
        }
        (Dil::Python, "orhunca_böl") => {
            "def orhunca_böl(m, ayraç):\n    # Boş ayraç: boşluklardan böler\n    return m.split() if ayraç == \"\" else m.split(ayraç)"
        }
        (Dil::Python, "orhunca_değiştir") => {
            "def orhunca_değiştir(m, aranan, yeni):\n    return m if aranan == \"\" else m.replace(aranan, yeni)"
        }
        (Dil::Python, "orhunca_ondalık") => {
            "def orhunca_ondalık(m):\n    d = float(m.strip().replace(\",\", \".\"))  # \"3,5\" de kabul edilir\n    if not math.isfinite(d):\n        raise ValueError(f\"'{m}' bir ondalık sayı değil\")\n    return d"
        }
        (Dil::JavaScript, "orhunca_metin") => {
            "// Değeri Orhunca'nın yazdığı gibi metne çevirir: doğru/yanlış, 2.0, [\"a\", 1]\nfunction orhunca_metin(x, ondalık = false, iç = false) {\n  if (typeof x === \"boolean\") return x ? \"doğru\" : \"yanlış\";\n  if (typeof x === \"number\") return ondalık ? orhunca_ondalık_metni(x) : String(x);\n  if (typeof x === \"string\") return iç ? `\"${x}\"` : x;\n  if (Array.isArray(x)) return \"[\" + x.map(o => orhunca_metin(o, ondalık, true)).join(\", \") + \"]\";\n  if (x && typeof x === \"object\")\n    return \"{\" + Object.entries(x).map(([a, d]) => orhunca_metin(a, false, true) + \": \" + orhunca_metin(d, ondalık, true)).join(\", \") + \"}\";\n  return String(x);\n}"
        }
        (Dil::JavaScript, "orhunca_ondalık_metni") => {
            "// Ondalık sayıyı Orhunca gibi (15 anlamlı basamak) yazar: 2 → \"2.0\", 0.1 + 0.2 → \"0.3\"\nfunction orhunca_ondalık_metni(x) {\n  if (!Number.isFinite(x)) return Number.isNaN(x) ? \"nan\" : x > 0 ? \"inf\" : \"-inf\";\n  if (x === 0) return Object.is(x, -0) ? \"-0.0\" : \"0.0\";\n  const [g, us] = x.toExponential(14).split(\"e\"), u = Number(us);\n  let m = u < -4 || u >= 15\n    ? g.replace(/\\.?0+$/, \"\") + \"e\" + (u < 0 ? \"-\" : \"+\") + String(Math.abs(u)).padStart(2, \"0\")\n    : x.toFixed(14 - u).replace(/(\\.\\d*?)0+$/, \"$1\").replace(/\\.$/, \"\");\n  return /[.eni]/.test(m) ? m : m + \".0\";\n}"
        }
        (Dil::JavaScript, "orhunca_yuvarla") => {
            "// Yarımlar sıfırdan uzağa yuvarlanır: 2.5 → 3, -2.5 → -3\nfunction orhunca_yuvarla(x, basamak = 0) {\n  const k = 10 ** basamak, y = Math.floor(Math.abs(x) * k + 0.5) / k;\n  return x < 0 ? -y : y;\n}"
        }
        (Dil::JavaScript, "orhunca_kalan") => {
            "// Kalanın işareti bölenle aynıdır: -5 % 2 → 1 (JavaScript'in % işleci -1 verir)\nfunction orhunca_kalan(a, b) {\n  const k = a % b;\n  return k !== 0 && (k < 0) !== (b < 0) ? k + b : k;\n}"
        }
        (Dil::JavaScript, "orhunca_parça") => {
            "// Harf harf (emoji tek harf sayılır); negatif başlangıç 0 sayılır\nfunction orhunca_parça(x, baş, uzunluk) {\n  baş = Math.max(baş, 0);\n  return typeof x === \"string\" ? [...x].slice(baş, baş + uzunluk).join(\"\") : x.slice(baş, baş + uzunluk);\n}"
        }
        (Dil::JavaScript, "orhunca_bul") => {
            "// Sıra harf olarak sayılır (emoji tek harf)\nfunction orhunca_bul(x, aranan) {\n  const i = x.indexOf(aranan);\n  return typeof x === \"string\" && i > 0 ? [...x.slice(0, i)].length : i;\n}"
        }
        (Dil::JavaScript, "orhunca_ondalık") => {
            "function orhunca_ondalık(m) {\n  const d = Number(m.trim().replace(\",\", \".\")); // \"3,5\" de kabul edilir\n  if (m.trim() === \"\" || !Number.isFinite(d)) throw new Error(`'${m}' bir ondalık sayı değil`);\n  return d;\n}"
        }
        (Dil::JavaScript, "orhunca_sayı") => {
            "function orhunca_sayı(m) {\n  if (!/^\\s*[-+]?\\d+\\s*$/.test(m)) throw new Error(`'${m}' bir sayı değil`);\n  return parseInt(m, 10);\n}"
        }
        (Dil::JavaScript, "orhunca_böl") => {
            "// Boş ayraç: boşluklardan böler\nfunction orhunca_böl(m, ayraç) {\n  return ayraç === \"\" ? m.split(/\\s+/).filter(p => p !== \"\") : m.split(ayraç);\n}"
        }
        (Dil::JavaScript, "orhunca_değiştir") => {
            "function orhunca_değiştir(m, aranan, yeni) {\n  return aranan === \"\" ? m : m.replaceAll(aranan, yeni);\n}"
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
    /// Çevirinin başına yazılacak uyarılar (ör. JavaScript'te büyük tamsayı sınırı)
    notlar: BTreeSet<&'static str>,
    /// JavaScript: tanımlanmış değişkenler (ilk atamada `let`)
    kapsam: Vec<HashSet<String>>,
    kaynak: Option<usize>,
    /// Seçenek türleri ve değerleri (`Renk.hepsi()`)
    secenekler: Vec<(String, Vec<String>)>,
    /// Çevrilen modelin adı (kendine dönen alan başta boştur)
    model_adi: String,
    /// Model işlevleri (`Kitap.özet`) sınıfın yöntemleri olarak yazılır.
    yontemler: Vec<Islev>,
    /// Bir yöntemin içindeyiz: `bu` → self / this
    yontemde: bool,
    /// Adsız işlevlerin (`‹adsız›N`) parametresi ve gövdesi: çağrıldıkları yere dilin
    /// kendi biçimiyle yazılırlar (bkz. src/adsiz.rs).
    adsizlar: std::collections::HashMap<String, (String, Ifade)>,
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
        if self.yontemde && a == MODEL_NESNESI {
            return if self.py() { "self" } else { "this" }.into();
        }
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
        for f in p
            .islevler
            .iter()
            .filter(|f| f.ad.starts_with(crate::adsiz::ANAHTAR_ONEKI))
        {
            if let (Some((a, _)), [Deyim::Dondur(Some(g), _)]) =
                (f.parametreler.first(), f.govde.as_slice())
            {
                self.adsizlar.insert(f.ad.clone(), (a.clone(), g.clone()));
            }
        }
        self.yontemler = p
            .islevler
            .iter()
            .filter(|f| f.ad.contains('.') && !f.ad.starts_with('‹'))
            .cloned()
            .collect();
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
            if f.ad.starts_with(ON_EK)
                || f.ad.starts_with('‹')
                || f.ad.contains('.')
                || f.konum.dosya >= p.dosyalar.len()
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
            self.girinti -= 1;
            self.yontemleri_yaz(&m.ad);
            self.girinti -= 1;
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
            self.yontemleri_yaz(&m.ad);
            self.blok_kapat();
        }
        self.bos_satir();
    }

    /// Modelin işlevleri sınıfın yöntemleri olur; ilk parametre (`bu`) self/this'tir.
    fn yontemleri_yaz(&mut self, model: &str) {
        let on_ek = format!("{model}.");
        let yontemler: Vec<Islev> = self
            .yontemler
            .iter()
            .filter(|f| f.ad.starts_with(&on_ek))
            .cloned()
            .collect();
        for f in yontemler {
            self.bos_satir();
            self.konum(f.konum);
            let kisa = &f.ad[on_ek.len()..];
            let mut parametreler: Vec<String> = f.parametreler[1..]
                .iter()
                .map(|(a, _)| self.ad(a))
                .collect();
            if self.py() {
                parametreler.insert(0, "self".into());
                self.blok_ac(format!(
                    "def {}({})",
                    self.ad(kisa),
                    parametreler.join(", ")
                ));
            } else {
                self.blok_ac(format!("{}({})", self.ad(kisa), parametreler.join(", ")));
                self.kapsam
                    .push(f.parametreler.iter().map(|(a, _)| a.clone()).collect());
            }
            self.yontemde = true;
            self.govde(&f.govde);
            self.yontemde = false;
            if !self.py() {
                self.kapsam.pop();
            }
            self.blok_kapat();
        }
    }

    fn islev(&mut self, f: &Islev) {
        self.konum(f.konum);
        let parametreler: Vec<String> = f.parametreler.iter().map(|(a, _)| self.ad(a)).collect();
        if let Some(d) = &f.dis {
            // C kütüphanesindeki işlev: Python'da ctypes ile çağrılır; JavaScript'te karşılığı yok.
            if self.py() {
                self.ithal.insert("ctypes");
                self.ithal.insert("ctypes.util");
                self.yardimci.insert("orhunca_c");
                let tipler: Vec<String> =
                    d.tipler.iter().map(|t| format!("{:?}", t.adi())).collect();
                self.blok_ac(format!(
                    "def {}({})",
                    self.ad(&f.ad),
                    parametreler.join(", ")
                ));
                let mut argumanlar = vec![
                    format!("{:?}", d.kutuphane),
                    format!("{:?}", f.ad),
                    format!("[{}]", tipler.join(", ")),
                    format!("{:?}", d.donus.adi()),
                ];
                argumanlar.extend(parametreler);
                self.yaz(format!("return orhunca_c({})", argumanlar.join(", ")));
            } else {
                self.notlar.insert(
                    "C kütüphanesi işlevleri (kütüphane bloğu) JavaScript'te çağrılamaz; çağrılınca hata verir.",
                );
                self.blok_ac(format!(
                    "function {}({})",
                    self.ad(&f.ad),
                    parametreler.join(", ")
                ));
                self.yaz(format!(
                    "throw new Error({});",
                    metin_sabiti(&format!(
                        "'{}' bir C işlevi; JavaScript'te çağrılamaz",
                        f.ad
                    ))
                ));
            }
            self.blok_kapat();
            self.bos_satir();
            return;
        }
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
                let v = if e.tip == Tip::Sayi {
                    self.ifade(e)
                } else {
                    self.metne(e)
                };
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
                if self.py() && matches!(&e.tip, Tip::Liste(t) if t.metin_gibi()) {
                    self.yardimci.insert("orhunca_sıra");
                    self.yaz(format!("{l}.sort(key=orhunca_sıra)"));
                } else if self.py() {
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
            // Arayüz programında ertelenen blok; çeviride sırayla çalışır.
            Deyim::ArkaPlan {
                govde,
                bitince,
                olay,
                ..
            } => match olay {
                Some(o) => self.govde_ana(&o.govde),
                None => {
                    self.govde_ana(govde);
                    self.govde_ana(bitince);
                }
            },
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

    fn ifade_p(&mut self, e: &Ifade) -> (String, u8) {
        let py = self.py();
        match &e.tur {
            IfadeTuru::Sayi(n) => {
                if !py && n.unsigned_abs() > (1u64 << 53) {
                    self.notlar.insert(
                        "JavaScript'te 9007199254740992'den (2^53) büyük tamsayılar hassasiyet \
                         kaybeder; Orhunca 64 bit tamsayı kullanır. Gerekirse BigInt kullanın.",
                    );
                }
                (n.to_string(), if *n < 0 { 12 } else { P_ATOM })
            }
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
            IfadeTuru::Adsiz(p, g) => {
                let g = self.ifade(g);
                let p: Vec<String> = p.iter().map(|a| self.ad(a)).collect();
                if self.py() {
                    (format!("lambda {}: {g}", p.join(", ")), 0)
                } else {
                    (format!("({}) => {g}", p.join(", ")), 0)
                }
            }
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

    /// İfadeyi Orhunca'nın yazdığı biçimde metne çevirir (yazdırma ve metin birleştirme):
    /// metin ve tamsayı olduğu gibi kalır; doğru/yanlış, ondalık, liste ve sözlük yardımcıyla.
    fn metne(&mut self, e: &Ifade) -> String {
        match &e.tip {
            Tip::Metin | Tip::Secenek(_) => self.ifade(e),
            Tip::Sayi if self.py() => format!("str({})", self.ifade(e)),
            Tip::Sayi => format!("String({})", self.ifade(e)),
            Tip::Ondalik if !self.py() => {
                self.yardimci.insert("orhunca_ondalık_metni");
                format!("orhunca_ondalık_metni({})", self.ifade(e))
            }
            Tip::Model(_) | Tip::Bos | Tip::Bilinmeyen => self.ifade(e),
            t => {
                self.yardimci.insert("orhunca_metin");
                let v = self.ifade(e);
                if self.py() {
                    format!("orhunca_metin({v})")
                } else if ondalik_icerir(t) {
                    format!("orhunca_metin({v}, true)")
                } else {
                    format!("orhunca_metin({v})")
                }
            }
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
        if op == Mod && !py {
            // JavaScript'te kalanın işareti bölünenle aynıdır; Orhunca'da bölenle.
            self.yardimci.insert("orhunca_kalan");
            return (
                format!("orhunca_kalan({}, {})", self.ifade(a), self.ifade(b)),
                P_ATOM,
            );
        }
        if op == TamBol && !py {
            return (
                format!("Math.floor({} / {})", self.ifade(a), self.oncelikli(b, 11)),
                P_ATOM,
            );
        }
        // Metin birleştirme: diğer taraf Orhunca'nın yazdığı gibi metne çevrilir
        // (Python'da str() gerekir; doğru/yanlış ve ondalık her iki dilde de farklı yazılır).
        if op == Topla && tip.metin_gibi() {
            let taraf = |c: &mut Self, e: &Ifade, p: u8| match (&e.tip, py) {
                (t, _) if t.metin_gibi() => c.oncelikli(e, p),
                (Tip::Sayi, true) => format!("str({})", c.ifade(e)),
                (Tip::Sayi, false) => c.oncelikli(e, p),
                _ => c.metne(e),
            };
            let sol = taraf(self, a, p);
            let sag = taraf(self, b, p + 1);
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

    fn adsiz_cagrisi(&mut self, tur: &str, liste: &Ifade, p: &str, g: &Ifade) -> (String, u8) {
        let l = self.oncelikli(liste, P_ATOM);
        let p = self.ad(p);
        let metin_anahtari = g.tip.metin_gibi();
        let g = self.ifade(g);
        let m = if self.py() {
            match tur {
                "süz" => format!("[{p} for {p} in {l} if {g}]"),
                "dönüştür" => format!("[{g} for {p} in {l}]"),
                "biri_mi" => format!("any({g} for {p} in {l})"),
                "hepsi_mi" => format!("all({g} for {p} in {l})"),
                _ if metin_anahtari => {
                    self.yardimci.insert("orhunca_sıra");
                    format!("{l}.sort(key=lambda {p}: orhunca_sıra({g}))")
                }
                _ => format!("{l}.sort(key=lambda {p}: {g})"),
            }
        } else {
            let f = format!("({p}) => {g}");
            match tur {
                "süz" => format!("{l}.filter({f})"),
                "dönüştür" => format!("{l}.map({f})"),
                "biri_mi" => format!("{l}.some({f})"),
                "hepsi_mi" => format!("{l}.every({f})"),
                _ if metin_anahtari => {
                    format!("{l}.sort((a, b) => (({f})(a)).localeCompare(({f})(b), \"tr\"))")
                }
                _ => format!("{l}.sort((a, b) => ({f})(a) - ({f})(b))"),
            }
        };
        (m, P_ATOM)
    }

    fn cagri(&mut self, ad: &str, arg: &[Ifade]) -> (String, u8) {
        if ad == SECENEK_CEVIR {
            // Seçenek değeri metin olarak taşınır: Renk("mavi") → "mavi"
            return self.ifade_p(&arg[0]);
        }
        // süz(l, işlev(x) -> ...) ve benzerleri: dilin kendi biçimi
        if let Some((tur, sira)) = crate::adsiz::sargi(ad) {
            let anahtar = format!("{}{sira}", crate::adsiz::ANAHTAR_ONEKI);
            if let Some((p, g)) = self.adsizlar.get(&anahtar).cloned() {
                return self.adsiz_cagrisi(tur, &arg[0], &p, &g);
            }
        }
        // Model işlevi: Kitap.özet(k, x) → k.özet(x)
        if let (Some((_, kisa)), false) = (ad.split_once('.'), ad.starts_with('‹')) {
            let alici = self.oncelikli(&arg[0], P_ATOM);
            let a: Vec<String> = arg[1..].iter().map(|x| self.ifade(x)).collect();
            return (
                format!("{alici}.{}({})", self.ad(kisa), a.join(", ")),
                P_ATOM,
            );
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
                ("metin", 1) if t0 == Tip::Sayi => atom(format!("str({})", a[0])),
                ("metin", 1) if t0.metin_gibi() => (a[0].clone(), self.ifade_p(&arg[0]).1),
                ("metin", 1) => atom(self.metne(&arg[0])),
                ("sayı", 1) => atom(format!("int({})", a[0])),
                ("ondalık", 1) if t0 == Tip::Sayi => (a[0].clone(), self.ifade_p(&arg[0]).1),
                ("ondalık", 1) => {
                    ithal(self, "math");
                    self.yardimci.insert("orhunca_ondalık");
                    atom(format!("orhunca_ondalık({})", a[0]))
                }
                ("yuvarla", 1 | 2) => {
                    ithal(self, "math");
                    self.yardimci.insert("orhunca_yuvarla");
                    atom(format!("orhunca_yuvarla({})", a.join(", ")))
                }
                ("büyük_harf", 1) => {
                    self.yardimci.insert("orhunca_büyük_harf");
                    atom(format!("orhunca_büyük_harf({})", a[0]))
                }
                ("küçük_harf", 1) => {
                    self.yardimci.insert("orhunca_küçük_harf");
                    atom(format!("orhunca_küçük_harf({})", a[0]))
                }
                ("kırp", 1) => atom(format!("{}.strip()", alici(self, 0))),
                ("parça", 3) if negatif_olamaz(&arg[1]) => {
                    atom(format!("{}[{}:{} + {}]", alici(self, 0), a[1], a[1], a[2]))
                }
                ("parça", 3) => {
                    self.yardimci.insert("orhunca_parça");
                    atom(format!("orhunca_parça({})", a.join(", ")))
                }
                ("böl", 2) if bos_olmayan_metin(&arg[1]) => {
                    atom(format!("{}.split({})", alici(self, 0), a[1]))
                }
                ("böl", 2) => {
                    self.yardimci.insert("orhunca_böl");
                    atom(format!("orhunca_böl({}, {})", a[0], a[1]))
                }
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
                ("değiştir", 3) if bos_olmayan_metin(&arg[1]) => {
                    atom(format!("{}.replace({}, {})", alici(self, 0), a[1], a[2]))
                }
                ("değiştir", 3) => {
                    self.yardimci.insert("orhunca_değiştir");
                    atom(format!("orhunca_değiştir({})", a.join(", ")))
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
                // Metinde harf sayısı: emoji gibi karakterler tek harf sayılır (Orhunca gibi).
                ("uzunluk", 1) if t0 == Tip::Metin => atom(format!("[...{}].length", a[0])),
                ("uzunluk", 1) => atom(format!("{}.length", alici(self, 0))),
                ("metin", 1) if t0 == Tip::Sayi => atom(format!("String({})", a[0])),
                ("metin", 1) if t0.metin_gibi() => (a[0].clone(), self.ifade_p(&arg[0]).1),
                ("metin", 1) => atom(self.metne(&arg[0])),
                ("sayı", 1) => {
                    self.yardimci.insert("orhunca_sayı");
                    atom(format!("orhunca_sayı({})", a[0]))
                }
                ("ondalık", 1) if t0 == Tip::Sayi => (a[0].clone(), self.ifade_p(&arg[0]).1),
                ("ondalık", 1) => {
                    self.yardimci.insert("orhunca_ondalık");
                    atom(format!("orhunca_ondalık({})", a[0]))
                }
                ("yuvarla", 1 | 2) => {
                    self.yardimci.insert("orhunca_yuvarla");
                    atom(format!("orhunca_yuvarla({})", a.join(", ")))
                }
                ("büyük_harf", 1) => {
                    atom(format!("{}.toLocaleUpperCase(\"tr\")", alici(self, 0)))
                }
                ("küçük_harf", 1) => {
                    atom(format!("{}.toLocaleLowerCase(\"tr\")", alici(self, 0)))
                }
                ("kırp", 1) => atom(format!("{}.trim()", alici(self, 0))),
                ("parça", 3) if t0 != Tip::Metin && negatif_olamaz(&arg[1]) => atom(format!(
                    "{}.slice({}, {} + {})",
                    alici(self, 0),
                    a[1],
                    a[1],
                    a[2]
                )),
                ("parça", 3) => {
                    self.yardimci.insert("orhunca_parça");
                    atom(format!("orhunca_parça({})", a.join(", ")))
                }
                ("böl", 2) if bos_olmayan_metin(&arg[1]) => {
                    atom(format!("{}.split({})", alici(self, 0), a[1]))
                }
                ("böl", 2) => {
                    self.yardimci.insert("orhunca_böl");
                    atom(format!("orhunca_böl({}, {})", a[0], a[1]))
                }
                ("birleştir", 2) => atom(format!("{}.join({})", alici(self, 0), a[1])),
                ("içerir", 2) if sozluk => {
                    (format!("{} in {}", self.oncelikli(&arg[1], 7), a[0]), 6)
                }
                ("içerir", 2) => {
                    atom(format!("{}.includes({})", liste_ya_da_metin(self, 0), a[1]))
                }
                ("bul", 2) if t0 == Tip::Metin => {
                    self.yardimci.insert("orhunca_bul");
                    atom(format!("orhunca_bul({}, {})", a[0], a[1]))
                }
                ("bul", 2) => atom(format!("{}.indexOf({})", liste_ya_da_metin(self, 0), a[1])),
                ("değiştir", 3) if bos_olmayan_metin(&arg[1]) => {
                    atom(format!("{}.replaceAll({}, {})", alici(self, 0), a[1], a[2]))
                }
                ("değiştir", 3) => {
                    self.yardimci.insert("orhunca_değiştir");
                    atom(format!("orhunca_değiştir({})", a.join(", ")))
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

/// Değerin (iç içe) ondalık içerip içermediği: JavaScript'te 2.0 ile 2 ayırt edilemediği
/// için yazdırma yardımcısına bildirilir.
fn ondalik_icerir(t: &Tip) -> bool {
    match t {
        Tip::Ondalik => true,
        Tip::Liste(o) => ondalik_icerir(o),
        Tip::Sozluk(_, d) => ondalik_icerir(d),
        _ => false,
    }
}

/// Sıfır ya da pozitif bir tamsayı sabiti mi (parça başlangıcı doğrudan dilimlenebilir).
fn negatif_olamaz(e: &Ifade) -> bool {
    matches!(e.tur, IfadeTuru::Sayi(n) if n >= 0)
}

/// Boş olmayan bir metin sabiti mi (böl/değiştir doğrudan çevrilebilir).
fn bos_olmayan_metin(e: &Ifade) -> bool {
    matches!(&e.tur, IfadeTuru::Metin(m) if !m.is_empty())
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
