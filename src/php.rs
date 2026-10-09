//! Orhunca programını PHP'ye çevirir: `orhunca yayınla --php`.
//!
//! Yalnızca PHP ve MySQL sunan paylaşımlı barındırmalar (cPanel, Plesk) için: çıktı klasörü
//! (`cikti/php/`) olduğu gibi `public_html`e yüklenir. Çeviri, tip denetiminden geçmiş ağaçtan
//! yapılır; ön kütüphane (Orhunca ile yazılmış standart işlevler: düzenli ifadeler, tarihler,
//! CSV...) de çevrilir, böylece bu işlevler yerel derlemeyle birebir aynı çalışır. Temel
//! işlevler, modellerin kaydı (PDO ile MySQL ya da SQLite) ve web yolları PHP çalışma
//! zamanındadır (runtime/php/orhunca.php).

use crate::agac::*;
use crate::on_kutuphane::{HATA_SATIRDA, ON_EK, YERLESIK_ON_EK};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// PHP çalışma zamanı (orhunca/calisma.php olarak yazılır)
pub const CALISMA_ZAMANI: &str = include_str!("../runtime/php/orhunca.php");

const GOVDE_ON_EK: &str = "görünüm:";

/// Yerleşik işlevlerin çalışma zamanındaki karşılıkları
const YERLESIKLER: &[(&str, &str)] = &[
    ("uzunluk", "o_uzunluk"),
    ("metin", "o_metin"),
    ("sayı", "o_sayi"),
    ("ondalık", "o_ondalik"),
    ("yuvarla", "o_yuvarla"),
    ("sayı_mı", "o_sayi_mi"),
    ("ondalık_mı", "o_ondalik_mi"),
    ("parça", "o_parca"),
    ("birleştir", "o_birlestir"),
    ("içerir", "o_icerir"),
    ("bul", "o_bul"),
    ("harfler", "o_harfler"),
    ("kodlar", "o_kodlar"),
    ("kodlardan", "o_kodlardan"),
    ("kod", "o_kod"),
    ("karakter", "o_karakter"),
    ("sil", "o_sil"),
    ("ters", "o_ters"),
    ("karıştır", "o_karistir"),
    ("kopya", "o_kopya"),
    ("en_büyük", "o_en_buyuk"),
    ("en_küçük", "o_en_kucuk"),
    ("toplam", "o_toplam"),
    ("anahtarlar", "o_anahtarlar"),
    ("değerler", "o_degerler"),
    ("dosya_oku", "o_dosya_oku"),
    ("dosyaya_yaz", "o_dosyaya_yaz"),
    ("dosyaya_ekle", "o_dosyaya_ekle"),
    ("dosya_var", "o_dosya_var"),
    ("dosya_sil", "o_dosya_sil"),
    ("dosya_taşı", "o_dosya_tasi"),
    ("karekök", "o_karekok"),
    ("üs", "o_us"),
    ("mutlak", "o_mutlak"),
    ("sinüs", "o_sinus"),
    ("kosinüs", "o_kosinus"),
    ("tanjant", "o_tanjant"),
    ("logaritma", "o_logaritma"),
    ("rastgele", "o_rastgele"),
    ("zaman", "o_zaman"),
    ("tarih", "o_tarih"),
    ("bekle", "o_bekle"),
    ("oku", "o_oku"),
    ("argümanlar", "o_argumanlar"),
    ("ortam", "o_ortam"),
    ("çık", "o_cik"),
    ("boş_mu", "o_bos_mu"),
    ("hata_ver", "o_hata_ver"),
    ("http_al", "o_http_al"),
    ("http_gönder", "o_http_gonder"),
    ("http_iste", "o_http_iste"),
    ("json", "o_json"),
    ("para", "o_para"),
    ("ham", "o_ham"),
    ("sun", "o_sun"),
    ("titret", "o_etkisiz"),
    ("paylaş", "o_etkisiz"),
    ("bildirim_gönder", "o_etkisiz"),
    ("tema", "o_etkisiz"),
];

/// Bir PHP satırı ve geldiği Orhunca satırı
struct Satir {
    metin: String,
    kaynak: Option<usize>,
}

struct Uretici<'a> {
    p: &'a Program,
    satirlar: Vec<Satir>,
    girinti: usize,
    kaynak: Option<usize>,
    /// Programdaki işlevlerin adları → PHP adı
    islevler: HashMap<String, String>,
    sabitler: HashSet<String>,
    secenekler: HashMap<String, Vec<String>>,
    modeller: HashMap<String, &'a Model>,
    /// Çevrilen işlevin yerel değişken tipleri
    yereller: HashMap<String, Tip>,
    donus: Option<Tip>,
    sayac: usize,
}

/// PHP'de geçerli ad: harf, rakam, _ ve ASCII dışı karakterler kalır.
fn temiz(ad: &str) -> String {
    let mut s: String = ad
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    s
}

/// PHP'nin yerleşik işlevleri ve sınıflarıyla çakışmasın: yalnızca ASCII harfli adlara önek.
fn onekli(on_ek: &str, ad: &str) -> String {
    let t = temiz(ad);
    if t.is_ascii() {
        format!("{on_ek}{t}")
    } else {
        t
    }
}

fn degisken(ad: &str) -> String {
    let t = temiz(ad.trim_start_matches('@'));
    let t = if ad.starts_with('@') {
        format!("ohc_{t}")
    } else {
        t
    };
    match t.as_str() {
        "this" | "GLOBALS" | "_GET" | "_POST" | "_SERVER" | "_COOKIE" | "_FILES" | "_ENV"
        | "_REQUEST" | "_SESSION" => format!("${t}_"),
        _ => format!("${t}"),
    }
}

/// PHP metin sabiti (çift tırnak)
fn metin_sabiti(m: &str) -> String {
    let mut s = String::from("\"");
    for c in m.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '$' => s.push_str("\\$"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(s, "\\x{:02x}", c as u32);
            }
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

/// Model alanının çalışma zamanındaki tip tanımı
fn tip_tanimi(t: &Tip, secenekler: &HashMap<String, Vec<String>>) -> String {
    match t {
        Tip::Sayi => "'s'".into(),
        Tip::Ondalik => "'o'".into(),
        Tip::Metin => "'m'".into(),
        Tip::Mantik => "'b'".into(),
        Tip::Secenek(s) => {
            let d: Vec<String> = secenekler
                .get(s)
                .map(|l| l.iter().map(|x| metin_sabiti(x)).collect())
                .unwrap_or_default();
            format!("['e', [{}]]", d.join(", "))
        }
        Tip::Liste(i) => format!("['l', {}]", tip_tanimi(i, secenekler)),
        Tip::Sozluk(a, d) => format!(
            "['d', {}, {}]",
            tip_tanimi(a, secenekler),
            tip_tanimi(d, secenekler)
        ),
        Tip::Model(m) => format!("['M', {}]", metin_sabiti(&sinif_adi(m))),
        _ => "'m'".into(),
    }
}

/// Modelin PHP sınıf adı (İstek, Yanıt, YüklenenDosya olduğu gibi kalır)
fn sinif_adi(m: &str) -> String {
    onekli("M_", m)
}

impl<'a> Uretici<'a> {
    fn yaz(&mut self, m: impl AsRef<str>) {
        self.satirlar.push(Satir {
            metin: format!("{}{}", "    ".repeat(self.girinti), m.as_ref()),
            kaynak: self.kaynak,
        });
    }

    fn konum(&mut self, k: crate::hata::Konum) {
        self.kaynak = (k.dosya < self.p.dosyalar.len()).then_some(k.satir);
    }

    fn islev_adi(&self, ad: &str) -> String {
        if let Some(r) = ad.strip_prefix(ON_EK) {
            format!("öz_{}", temiz(r))
        } else if let Some(r) = ad.strip_prefix(GOVDE_ON_EK) {
            format!("görünüm_{}", temiz(r))
        } else {
            onekli("f_", ad)
        }
    }

    // -----------------------------------------------------------------------
    // Program
    // -----------------------------------------------------------------------

    fn program(&mut self) -> Result<(), String> {
        let p = self.p;
        for s in &p.secenekler {
            self.secenekler.insert(s.ad.clone(), s.degerler.clone());
        }
        for m in &p.modeller {
            self.modeller.insert(m.ad.clone(), m);
        }
        let mut yol_sayisi = 0;
        for f in &p.islevler {
            if f.arayuz {
                return Err(
                    "arayüz programları PHP'ye çevrilemez; tarayıcıda çalışırlar \
                            (orhunca derle --hedef web)"
                        .into(),
                );
            }
            let ad = if f.rota.is_some() {
                yol_sayisi += 1;
                format!("yol_{yol_sayisi}")
            } else {
                self.islev_adi(&f.ad)
            };
            self.islevler.insert(f.ad.clone(), ad);
        }
        let web = yol_sayisi > 0;
        self.yaz(format!("o_baslat({});", if web { "true" } else { "false" }));
        self.yaz("");

        // Modeller (İstek, Yanıt ve YüklenenDosya dahil)
        for m in &p.modeller {
            self.model(m);
        }
        // İşlevler (ön kütüphane ve görünümler dahil)
        for f in &p.islevler {
            self.islev(f)?;
        }
        // Sabitler ve ana program
        self.kaynak = None;
        for (ad, deger) in &p.sabitler {
            self.sabitler.insert(ad.clone());
            self.konum(deger.konum);
            let v = self.ifade(deger)?;
            self.yaz(format!("$GLOBALS[{}] = {v};", metin_sabiti(ad)));
        }
        for f in &p.islevler {
            if let Some(r) = &f.rota {
                let ad = self.islevler[&f.ad].clone();
                self.yaz(format!(
                    "o_yol({}, {}, {});",
                    metin_sabiti(&r.yontem),
                    metin_sabiti(&r.kalip),
                    metin_sabiti(&ad)
                ));
            }
        }
        self.yereller = p.ana_yereller.iter().cloned().collect();
        self.donus = None;
        for d in &p.ana {
            self.deyim(d)?;
        }
        Ok(())
    }

    fn model(&mut self, m: &Model) {
        self.konum(m.konum);
        let sinif = sinif_adi(&m.ad);
        self.yaz(format!("final class {sinif} extends OModel"));
        self.yaz("{");
        self.girinti += 1;
        self.yaz(format!("const AD = {};", metin_sabiti(&m.ad)));
        self.yaz("const ALANLAR = [");
        self.girinti += 1;
        for a in &m.alanlar {
            let mut kurallar = vec![format!("'t' => {}", tip_tanimi(&a.tip, &self.secenekler))];
            if a.zorunlu {
                kurallar.push("'zorunlu' => true".into());
            }
            if let Some(x) = a.en_az {
                kurallar.push(format!("'en_az' => {x:?}"));
            }
            if let Some(x) = a.en_fazla {
                kurallar.push(format!("'en_fazla' => {x:?}"));
            }
            if let Some(e) = &a.etiket {
                kurallar.push(format!("'etiket' => {}", metin_sabiti(e)));
            }
            if a.e_posta {
                kurallar.push("'e_posta' => true".into());
            }
            self.yaz(format!(
                "{} => [{}],",
                metin_sabiti(&a.ad),
                kurallar.join(", ")
            ));
        }
        self.girinti -= 1;
        self.yaz("];");
        for a in &m.alanlar {
            self.yaz(format!("public ${};", temiz(&a.ad)));
        }
        let parametreler: Vec<String> = m
            .alanlar
            .iter()
            .map(|a| format!("${}", temiz(&a.ad)))
            .collect();
        self.yaz(format!(
            "public function __construct({})",
            parametreler.join(", ")
        ));
        self.yaz("{");
        self.girinti += 1;
        for a in &m.alanlar {
            let t = temiz(&a.ad);
            self.yaz(format!("$this->{t} = ${t};"));
        }
        self.girinti -= 1;
        self.yaz("}");
        // Varsayılan değerlerle yeni nesne (her çağrıda yeni liste ve iç nesneler)
        let degerler: Vec<String> = m.alanlar.iter().map(|a| self.ilk_deger(a)).collect();
        self.yaz("public static function yeni(): static");
        self.yaz("{");
        self.girinti += 1;
        self.yaz(format!("return new static({});", degerler.join(", ")));
        self.girinti -= 1;
        self.yaz("}");
        self.girinti -= 1;
        self.yaz("}");
        self.yaz("");
    }

    fn ilk_deger(&mut self, a: &AlanTanimi) -> String {
        match (&a.tip, &a.varsayilan) {
            (Tip::Model(_), None) if a.dongusel => "null".into(),
            (Tip::Secenek(s), None) => self
                .secenekler
                .get(s)
                .and_then(|l| l.first())
                .map(|d| metin_sabiti(d))
                .unwrap_or_else(|| "\"\"".into()),
            _ => {
                let e = a.ilk_deger();
                self.tipli(&e, &a.tip).unwrap_or_else(|_| "null".into())
            }
        }
    }

    fn islev(&mut self, f: &Islev) -> Result<(), String> {
        self.konum(f.konum);
        let ad = self.islevler[&f.ad].clone();
        let parametreler: Vec<String> = f.parametreler.iter().map(|(a, _)| degisken(a)).collect();
        self.yaz(format!("function {ad}({})", parametreler.join(", ")));
        self.yaz("{");
        self.girinti += 1;
        if f.dis.is_some() {
            self.yaz(format!(
                "o_hata({});",
                metin_sabiti(&format!(
                    "'{}' bir C kütüphanesi işlevi; PHP'de çağrılamaz",
                    f.ad
                ))
            ));
        } else {
            self.yereller = f.yereller.iter().cloned().collect();
            self.donus = f.donus.clone();
            if !f.ad.starts_with(ON_EK) {
                self.yaz("$ohc_derinlik = new ODerinlik();");
            }
            for d in &f.govde {
                self.deyim(d)?;
            }
        }
        self.girinti -= 1;
        self.kaynak = None;
        self.yaz("}");
        self.yaz("");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Deyimler
    // -----------------------------------------------------------------------

    fn govde(&mut self, g: &[Deyim]) -> Result<(), String> {
        self.girinti += 1;
        for d in g {
            self.deyim(d)?;
        }
        self.girinti -= 1;
        Ok(())
    }

    fn deyim(&mut self, d: &Deyim) -> Result<(), String> {
        if let Some(k) = d.konum() {
            self.konum(k);
        }
        match d {
            Deyim::Atama { hedef, deger, .. } => {
                let t = self
                    .yereller
                    .get(hedef)
                    .cloned()
                    .unwrap_or(deger.tip.clone());
                let v = self.tipli(deger, &t)?;
                self.yaz(format!("{} = {v};", self.isim(hedef)));
            }
            Deyim::IndeksAtama {
                liste,
                indeks,
                deger,
            } => {
                let oge = match &liste.tip {
                    Tip::Liste(t) => (**t).clone(),
                    Tip::Sozluk(_, t) => (**t).clone(),
                    _ => deger.tip.clone(),
                };
                let l = self.ifade(liste)?;
                let i = self.ifade(indeks)?;
                let v = self.tipli(deger, &oge)?;
                self.yaz(format!("{l}[{i}] = {v};"));
            }
            Deyim::AlanAtama {
                nesne, alan, deger, ..
            } => {
                let t = self
                    .alan_tipi(&nesne.tip, alan)
                    .unwrap_or(deger.tip.clone());
                let n = self.ifade(nesne)?;
                let v = self.tipli(deger, &t)?;
                self.yaz(format!("{n}->{} = {v};", temiz(alan)));
            }
            Deyim::Yaz(e) => {
                let v = self.ifade(e)?;
                self.yaz(format!("o_yaz({v});"));
            }
            Deyim::Ekle { oge, liste } => {
                let t = match &liste.tip {
                    Tip::Liste(t) => (**t).clone(),
                    _ => oge.tip.clone(),
                };
                let l = self.ifade(liste)?;
                let v = self.tipli(oge, &t)?;
                self.yaz(format!("o_ekle({l}, {v});"));
            }
            Deyim::Cikar { oge, liste } => {
                let l = self.ifade(liste)?;
                let v = self.ifade(oge)?;
                self.yaz(format!("o_cikar({l}, {v});"));
            }
            Deyim::DosyayaYaz { deger, yol } => {
                let v = self.ifade(deger)?;
                let y = self.ifade(yol)?;
                self.yaz(format!("o_dosyaya_yaz({y}, {v});"));
            }
            Deyim::Sirala(e) => {
                let l = self.ifade(e)?;
                self.yaz(format!("o_sirala({l});"));
            }
            Deyim::Eger {
                kosul,
                govde,
                degilse,
            } => self.eger(kosul, govde, degilse, false)?,
            Deyim::Surece { kosul, govde } => {
                let k = self.ifade(kosul)?;
                self.yaz(format!("while ({k}) {{"));
                self.govde(govde)?;
                self.kapat();
            }
            Deyim::HerAralik {
                degisken: dg,
                bas,
                son,
                govde,
                ..
            } => {
                let a = degisken(dg);
                let b = self.ifade(bas)?;
                let s = self.ifade(son)?;
                self.sayac += 1;
                let sinir = format!("$ohc_son{}", self.sayac);
                self.yaz(format!(
                    "for ({a} = {b}, {sinir} = {s}; {a} <= {sinir}; {a}++) {{"
                ));
                self.govde(govde)?;
                self.kapat();
            }
            Deyim::HerListe {
                degisken: dg,
                liste,
                govde,
                ..
            } => {
                let a = degisken(dg);
                let l = self.ifade(liste)?;
                let l = if liste.tip == Tip::Metin {
                    format!("o_harfler({l})")
                } else {
                    l
                };
                self.yaz(format!("foreach ({l} as {a}) {{"));
                self.govde(govde)?;
                self.kapat();
            }
            Deyim::Dondur(e, _) => match e {
                Some(e) => {
                    let t = self.donus.clone().unwrap_or(e.tip.clone());
                    let v = self.tipli(e, &t)?;
                    self.yaz(format!("return {v};"));
                }
                None => self.yaz("return;"),
            },
            Deyim::Dur(_) => self.yaz("break;"),
            Deyim::Surdur(_) => self.yaz("continue;"),
            Deyim::IfadeDeyimi(e) => {
                let v = self.ifade(e)?;
                self.yaz(format!("{v};"));
            }
            Deyim::Oge(_) => {
                return Err("arayüz öğeleri PHP'ye çevrilemez".into());
            }
            Deyim::ArkaPlan { .. } => return Err("arka planda bloğu denetlenmemiş".into()),
            Deyim::Dene {
                govde,
                degisken: dg,
                yakala,
                ..
            } => {
                self.yaz("try {");
                self.govde(govde)?;
                self.yaz("} catch (\\Throwable $ohc_hata) {");
                if let Some(dg) = dg {
                    self.girinti += 1;
                    self.yaz(format!("{} = o_yakalanan($ohc_hata);", degisken(dg)));
                    self.girinti -= 1;
                }
                self.govde(yakala)?;
                self.kapat();
            }
        }
        Ok(())
    }

    fn kapat(&mut self) {
        let k = self.kaynak.take();
        self.yaz("}");
        self.kaynak = k;
    }

    fn eger(
        &mut self,
        kosul: &Ifade,
        govde: &[Deyim],
        degilse: &[Deyim],
        devam: bool,
    ) -> Result<(), String> {
        let k = self.ifade(kosul)?;
        if devam {
            self.yaz(format!("}} elseif ({k}) {{"));
        } else {
            self.yaz(format!("if ({k}) {{"));
        }
        self.govde(govde)?;
        if let [Deyim::Eger {
            kosul,
            govde,
            degilse,
        }] = degilse
        {
            self.konum(kosul.konum);
            return self.eger(kosul, govde, degilse, true);
        }
        if !degilse.is_empty() {
            self.yaz("} else {");
            self.govde(degilse)?;
        }
        self.kapat();
        Ok(())
    }

    // -----------------------------------------------------------------------
    // İfadeler
    // -----------------------------------------------------------------------

    fn isim(&self, ad: &str) -> String {
        if self.sabitler.contains(ad) && !self.yereller.contains_key(ad) {
            format!("$GLOBALS[{}]", metin_sabiti(ad))
        } else {
            degisken(ad)
        }
    }

    fn alan_tipi(&self, t: &Tip, alan: &str) -> Option<Tip> {
        let Tip::Model(m) = t else { return None };
        self.modeller
            .get(m)?
            .alanlar
            .iter()
            .find(|a| a.ad == alan)
            .map(|a| a.tip.clone())
    }

    /// Hedef tipe göre: sayı değer ondalık bir yere giderse ondalığa çevrilir (Orhunca 5.0 yazar).
    fn tipli(&mut self, e: &Ifade, hedef: &Tip) -> Result<String, String> {
        match (hedef, &e.tip, &e.tur) {
            // Kendi modeline dönen alanın ilk değeri (yerelde boş işaretçi)
            (Tip::Model(_), _, IfadeTuru::Sayi(0)) => Ok("null".into()),
            (Tip::Ondalik, Tip::Sayi, IfadeTuru::Sayi(n)) => Ok(format!("{n}.0")),
            (Tip::Ondalik, Tip::Sayi, _) => Ok(format!("(float)({})", self.ifade(e)?)),
            (Tip::Liste(i), _, IfadeTuru::Liste(l)) => {
                let ic: Result<Vec<String>, String> = l.iter().map(|x| self.tipli(x, i)).collect();
                Ok(format!("o_liste({})", ic?.join(", ")))
            }
            (Tip::Sozluk(a, d), _, IfadeTuru::Sozluk(c)) => {
                let mut ic = Vec::new();
                for (k, v) in c {
                    ic.push(format!("[{}, {}]", self.tipli(k, a)?, self.tipli(v, d)?));
                }
                Ok(format!("OSozluk::yap([{}])", ic.join(", ")))
            }
            _ => self.ifade(e),
        }
    }

    fn metne(&mut self, e: &Ifade) -> Result<String, String> {
        if e.tip.metin_gibi() {
            self.ifade(e)
        } else {
            Ok(format!("o_metin({})", self.ifade(e)?))
        }
    }

    fn ifade(&mut self, e: &Ifade) -> Result<String, String> {
        Ok(match &e.tur {
            IfadeTuru::Sayi(n) => {
                if *n == i64::MIN {
                    "PHP_INT_MIN".into()
                } else if *n < 0 {
                    format!("({n})")
                } else {
                    n.to_string()
                }
            }
            IfadeTuru::Ondalik(x) => {
                if x.is_nan() {
                    "NAN".into()
                } else if x.is_infinite() {
                    if *x > 0.0 { "INF" } else { "(-INF)" }.into()
                } else {
                    let m = format!("{x:?}");
                    if *x < 0.0 {
                        format!("({m})")
                    } else {
                        m
                    }
                }
            }
            IfadeTuru::Metin(m) => metin_sabiti(m),
            IfadeTuru::Mantik(d) => if *d { "true" } else { "false" }.into(),
            IfadeTuru::Isim(a) => self.isim(a),
            IfadeTuru::ModelAdi(a) => metin_sabiti(&sinif_adi(a)),
            IfadeTuru::Adsiz(..) => return Err("adsız işlev indirilmemiş".into()),
            IfadeTuru::Liste(l) => {
                let ic = match &e.tip {
                    Tip::Liste(i) => {
                        let i = (**i).clone();
                        let r: Result<Vec<String>, String> =
                            l.iter().map(|x| self.tipli(x, &i)).collect();
                        r?
                    }
                    _ => {
                        let r: Result<Vec<String>, String> =
                            l.iter().map(|x| self.ifade(x)).collect();
                        r?
                    }
                };
                format!("o_liste({})", ic.join(", "))
            }
            IfadeTuru::Sozluk(_) => self.tipli(e, &e.tip.clone())?,
            IfadeTuru::Tekli(TekliOp::Eksi, a) => format!("(-{})", self.ifade(a)?),
            IfadeTuru::Tekli(TekliOp::Degil, a) => format!("(!{})", self.ifade(a)?),
            IfadeTuru::Ikili(op, a, b) => self.ikili(*op, a, b, &e.tip)?,
            IfadeTuru::Indeks(l, i) => {
                let lv = self.ifade(l)?;
                let iv = self.ifade(i)?;
                if l.tip == Tip::Metin {
                    format!("o_harf({lv}, {iv})")
                } else {
                    format!("{lv}[{iv}]")
                }
            }
            IfadeTuru::Alan(n, a, _) => {
                if let (IfadeTuru::ModelAdi(s), Tip::Secenek(_)) = (&n.tur, &e.tip) {
                    if self.secenekler.contains_key(s) {
                        return Ok(metin_sabiti(a));
                    }
                }
                format!("{}->{}", self.ifade(n)?, temiz(a))
            }
            IfadeTuru::Kurucu(m, alanlar) => {
                let sinif = sinif_adi(m);
                if alanlar.is_empty() {
                    return Ok(format!("{sinif}::yeni()"));
                }
                let Some(model) = self.modeller.get(m).copied() else {
                    return Err(format!("'{m}' modeli bulunamadı"));
                };
                let mut args = Vec::new();
                for a in &model.alanlar {
                    let v = match alanlar.iter().find(|(ad, _)| *ad == a.ad) {
                        Some((_, d)) => self.tipli(d, &a.tip)?,
                        None => self.ilk_deger(a),
                    };
                    args.push(v);
                }
                format!("new {sinif}({})", args.join(", "))
            }
            IfadeTuru::Metod(n, ad, arg) => self.metod(n, ad, arg)?,
            IfadeTuru::FiilCagri(ad, arg) => {
                let a: Vec<Ifade> = arg.iter().map(|(_, x)| x.clone()).collect();
                self.cagri(ad, &a)?
            }
            IfadeTuru::Cagri(ad, arg) if ad == SECENEK_CEVIR => {
                let tur = match &e.tip {
                    Tip::Secenek(t) => t.clone(),
                    _ => String::new(),
                };
                let d: Vec<String> = self
                    .secenekler
                    .get(&tur)
                    .map(|l| l.iter().map(|x| metin_sabiti(x)).collect())
                    .unwrap_or_default();
                format!(
                    "o_secenek({}, [{}], {})",
                    self.ifade(&arg[0])?,
                    d.join(", "),
                    metin_sabiti(&tur)
                )
            }
            IfadeTuru::Cagri(ad, arg) => self.cagri(ad, arg)?,
        })
    }

    fn sayisal(t: &Tip) -> bool {
        matches!(t, Tip::Sayi | Tip::Ondalik)
    }

    fn ikili(&mut self, op: IkiliOp, a: &Ifade, b: &Ifade, tip: &Tip) -> Result<String, String> {
        use IkiliOp::*;
        if op == Topla && tip.metin_gibi() {
            return Ok(format!("({} . {})", self.metne(a)?, self.metne(b)?));
        }
        if op == Topla && matches!(tip, Tip::Liste(_)) {
            let x = self.ifade(a)?;
            let y = self.ifade(b)?;
            return Ok(format!("new OListe(array_merge({x}->o, {y}->o))"));
        }
        let x = self.ifade(a)?;
        let y = self.ifade(b)?;
        Ok(match op {
            Topla => format!("({x} + {y})"),
            Cikar => format!("({x} - {y})"),
            Carp => format!("({x} * {y})"),
            Bol => format!("o_bol({x}, {y})"),
            TamBol => format!("o_tambol({x}, {y})"),
            Mod => format!("o_kalan({x}, {y})"),
            Ve => format!("({x} && {y})"),
            Veya => format!("({x} || {y})"),
            Esit | EsitDegil => {
                let ayni_basit = a.tip == b.tip
                    && matches!(
                        a.tip,
                        Tip::Sayi | Tip::Ondalik | Tip::Metin | Tip::Mantik | Tip::Secenek(_)
                    );
                let m = if ayni_basit || matches!(a.tip, Tip::Model(_)) {
                    format!("({x} === {y})")
                } else if Self::sayisal(&a.tip) && Self::sayisal(&b.tip) {
                    format!("({x} == {y})")
                } else {
                    format!("o_esit({x}, {y})")
                };
                if op == Esit {
                    m
                } else {
                    format!("(!{m})")
                }
            }
            Kucuk | Buyuk | KucukEsit | BuyukEsit => {
                let isaret = match op {
                    Kucuk => "<",
                    Buyuk => ">",
                    KucukEsit => "<=",
                    _ => ">=",
                };
                if a.tip.metin_gibi() {
                    format!("(o_kars({x}, {y}) {isaret} 0)")
                } else {
                    format!("({x} {isaret} {y})")
                }
            }
        })
    }

    fn metod(&mut self, n: &Ifade, ad: &str, arg: &[Ifade]) -> Result<String, String> {
        let a: Result<Vec<String>, String> = arg.iter().map(|x| self.ifade(x)).collect();
        let a = a?;
        if let IfadeTuru::ModelAdi(m) = &n.tur {
            if let Some(d) = self.secenekler.get(m) {
                if ad == "hepsi" {
                    let ic: Vec<String> = d.iter().map(|x| metin_sabiti(x)).collect();
                    return Ok(format!("o_liste({})", ic.join(", ")));
                }
            }
            let s = metin_sabiti(&sinif_adi(m));
            return Ok(match ad {
                "hepsi" => format!("o_model_hepsi({s})"),
                "bul" => format!("o_model_bul({s}, {})", a[0]),
                "var_mı" => format!("o_model_var_mi({s}, {})", a[0]),
                "sil" => format!("o_model_sil_kimlik({s}, {})", a[0]),
                "formdan" => format!("o_model_formdan({s}, {})", a[0]),
                _ => return Err(format!("'{m}.{ad}' PHP'ye çevrilemiyor")),
            });
        }
        let x = self.ifade(n)?;
        Ok(match ad {
            "kaydet" => format!("o_model_kaydet({x})"),
            "sil" => format!("o_model_sil({x})"),
            "geçerli_mi" => format!("o_model_gecerli_mi({x})"),
            "hatalar" => format!("o_model_hatalar({x})"),
            "json" => format!("o_json({x})"),
            _ => return Err(format!("'.{ad}()' yöntemi PHP'ye çevrilemiyor")),
        })
    }

    fn cagri(&mut self, ad: &str, arg: &[Ifade]) -> Result<String, String> {
        if ad == HATA_SATIRDA {
            return Ok(format!(
                "o_hata_satirda({}, {})",
                self.ifade(&arg[0])?,
                self.ifade(&arg[1])?
            ));
        }
        // Programdaki (ön kütüphane, görünüm ya da kullanıcının) işlevi
        if let Some(f) = self.p.islevler.iter().find(|f| f.ad == ad) {
            let php = self.islevler[ad].clone();
            let mut a = Vec::new();
            for (i, x) in arg.iter().enumerate() {
                let t = f
                    .parametreler
                    .get(i)
                    .map(|(_, t)| t.clone())
                    .unwrap_or(x.tip.clone());
                a.push(self.tipli(x, &t)?);
            }
            return Ok(format!("{php}({})", a.join(", ")));
        }
        let yerlesik = ad.trim_start_matches(YERLESIK_ON_EK);
        let a: Result<Vec<String>, String> = arg.iter().map(|x| self.ifade(x)).collect();
        let a = a?;
        // ondalık(sayı) ve metin(metin) dönüşümleri
        if yerlesik == "ondalık" && arg.first().is_some_and(|x| x.tip == Tip::Sayi) {
            return Ok(format!("(float)({})", a[0]));
        }
        match YERLESIKLER.iter().find(|(o, _)| *o == yerlesik) {
            Some((_, php)) => Ok(format!("{php}({})", a.join(", "))),
            None => Err(format!(
                "'{yerlesik}' işlevi PHP'ye çevrilemiyor (yalnızca Orhunca'nın kendi sunucusunda ya da tarayıcıda çalışır)"
            )),
        }
    }
}

/// Programı PHP'ye çevirir: index.php içeriği.
pub fn cevir(p: &Program) -> Result<String, String> {
    let mut u = Uretici {
        p,
        satirlar: Vec::new(),
        girinti: 0,
        kaynak: None,
        islevler: HashMap::new(),
        sabitler: HashSet::new(),
        secenekler: HashMap::new(),
        modeller: HashMap::new(),
        yereller: HashMap::new(),
        donus: None,
        sayac: 0,
    };
    u.program()?;
    let bas = "<?php
// Orhunca programının PHP çevirisi (orhunca yayınla --php). Bu dosyayı değiştirmeyin:
// Orhunca kaynağını değiştirip yeniden üretin. Veritabanı ayarları: orhunca/ayarlar.php
define('OHC_KOK', __DIR__);
define('OHC_PROGRAM', __FILE__);
// PHP'nin yerleşik sunucusu (php -S): var olan dosyalar olduğu gibi sunulur.
if (PHP_SAPI === 'cli-server') {
    $ohc_yol = rawurldecode(parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH));
    if (preg_match('#^/(orhunca|veri)(/|$)#', $ohc_yol)) {
        http_response_code(403);
        exit;
    }
    if ($ohc_yol !== '/' && is_file(__DIR__ . $ohc_yol)) {
        return false;
    }
}
require __DIR__ . '/orhunca/calisma.php';

";
    let bas_satir = bas.lines().count();
    let mut govde = String::new();
    let mut tablo = Vec::new();
    for (i, s) in u.satirlar.iter().enumerate() {
        govde.push_str(&s.metin);
        govde.push('\n');
        if let Some(k) = s.kaynak {
            tablo.push(format!("{} => {k}", bas_satir + i + 1));
        }
    }
    Ok(format!(
        "{bas}{govde}\n// PHP satırı => Orhunca satırı (çalışma hatalarında gösterilir)\nfunction ohc_satirlar(): array\n{{\n    return [{}];\n}}\n",
        tablo.join(", ")
    ))
}

const HTACCESS: &str = "# orhunca yayınla --php tarafından üretildi.
# Var olan dosyalar (statik/ klasöründen gelenler) olduğu gibi sunulur, gerisi index.php'ye gider.
Options -Indexes
DirectoryIndex index.php
RewriteEngine On
RewriteRule ^(orhunca|veri)(/|$) - [F,L]
RewriteCond %{REQUEST_FILENAME} -f
RewriteRule ^ - [L]
RewriteRule ^ index.php [L,QSA]
";

const KAPALI: &str = "Require all denied\n";

const AYARLAR: &str = "<?php
// Veritabanı ayarları. Barındırma panelinizde (cPanel → MySQL Veritabanları) bir veritabanı
// ve kullanıcı oluşturup bilgileri buraya yazın. Boş bırakılırsa kayıtlar
// veri/orhunca.sqlite dosyasında (SQLite) tutulur. Tablolar kendiliğinden oluşturulur.
return [
    'sunucu' => 'localhost',
    'veritabani' => '',
    'kullanici' => '',
    'sifre' => '',
];
";

/// `cikti/php/` klasörünü hazırlar. `veriyle` verilirse projedeki `veri/*.json` kayıtları
/// da kopyalanır (tablolar ilk açılışta bunlardan doldurulur); varsayılan olarak
/// bilgisayardaki deneme verisi yayına gitmez.
pub fn hazirla(giris: &Path, veriyle: bool) -> Result<PathBuf, String> {
    let kok = crate::derleme::proje_koku(giris);
    let program = crate::derleme::yukle(giris).map_err(|h| h.metin)?;
    if program.arayuz_programi() {
        return Err(
            "bu bir arayüz programı; sunucu gerekmez\nipucu: orhunca derle --hedef web \
             ile tek bir .html dosyası üretip barındırmaya yükleyin"
                .into(),
        );
    }
    let index = cevir(&program)?;
    let cikti = kok.join("cikti").join("php");
    // Önceki çıktının ayarları ve verisi korunur.
    let ayarlar = std::fs::read_to_string(cikti.join("orhunca/ayarlar.php")).ok();
    let veri_yedek = cikti.join("veri");
    let gecici_veri = kok.join("cikti").join(".php-veri");
    if veri_yedek.exists() {
        let _ = std::fs::remove_dir_all(&gecici_veri);
        std::fs::rename(&veri_yedek, &gecici_veri).map_err(|e| e.to_string())?;
    }
    if cikti.exists() {
        std::fs::remove_dir_all(&cikti)
            .map_err(|e| format!("'{}' silinemedi: {e}", cikti.display()))?;
    }
    std::fs::create_dir_all(cikti.join("orhunca")).map_err(|e| e.to_string())?;
    if gecici_veri.exists() {
        std::fs::rename(&gecici_veri, cikti.join("veri")).map_err(|e| e.to_string())?;
    } else {
        std::fs::create_dir_all(cikti.join("veri")).map_err(|e| e.to_string())?;
    }
    let statik = kok.join("statik");
    if statik.is_dir() {
        crate::yayinla::klasor_kopyala(&statik, &cikti)?;
    }
    // Orhunca'nın kendi sunucusunun kayıtları: tablolar ilk açılışta bunlardan doldurulur.
    if veriyle {
        crate::cgi::json_kayitlari_kopyala(&kok.join("veri"), &cikti.join("veri"))?;
    }
    let yaz = |ad: &str, icerik: &str| {
        std::fs::write(cikti.join(ad), icerik).map_err(|e| format!("{ad} yazılamadı: {e}"))
    };
    yaz("index.php", &index)?;
    yaz(".htaccess", HTACCESS)?;
    yaz("orhunca/calisma.php", CALISMA_ZAMANI)?;
    yaz("orhunca/ayarlar.php", ayarlar.as_deref().unwrap_or(AYARLAR))?;
    yaz("orhunca/.htaccess", KAPALI)?;
    yaz("veri/.htaccess", KAPALI)?;
    Ok(cikti)
}
