//! Erişilebilirlik sınaması: `orhunca erişilebilirlik [klasör|dosya]`.
//!
//! Ekran okuyucu kullananların ve düşük görenlerin takılacağı yaygın sorunları arar:
//! açıklamasız resimler, etiketsiz giriş kutuları, yazısız düğme ve bağlantılar, dili
//! belirtilmemiş sayfalar ve okunması zor renk çiftleri. `.ohc` arayüz programları ile
//! `.ohchtml` / `.html` sayfalarına bakar. Kesin bir denetim değil; uyarıları bir göz gezdirin.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Bulgu {
    pub dosya: PathBuf,
    pub satir: usize,
    pub mesaj: String,
}

fn bulgu(dosya: &Path, satir: usize, mesaj: impl Into<String>) -> Bulgu {
    Bulgu {
        dosya: dosya.to_path_buf(),
        satir,
        mesaj: mesaj.into(),
    }
}

/// `ad(` çağrısının parantez içindeki değerleri (en üst düzeydeki virgüllerden bölünmüş).
fn cagri_degerleri(satir: &str, ad: &str) -> Option<Vec<String>> {
    let bas = satir.trim_start();
    let ic = bas.strip_prefix(ad)?.strip_prefix('(')?;
    let mut derinlik = 0;
    let mut tirnak = false;
    let mut parca = String::new();
    let mut sonuc = Vec::new();
    for c in ic.chars() {
        match c {
            '"' => tirnak = !tirnak,
            '(' | '[' | '{' if !tirnak => derinlik += 1,
            ')' if !tirnak && derinlik == 0 => {
                if !parca.trim().is_empty() {
                    sonuc.push(parca.trim().to_string());
                }
                return Some(sonuc);
            }
            ')' | ']' | '}' if !tirnak => derinlik -= 1,
            ',' if !tirnak && derinlik == 0 => {
                sonuc.push(parca.trim().to_string());
                parca.clear();
                continue;
            }
            _ => {}
        }
        parca.push(c);
    }
    None
}

/// Konumsal (adsız) değerler ve `ad: değer` seçenekleri.
fn ayir(degerler: &[String]) -> (Vec<&str>, Vec<(&str, &str)>) {
    let mut konumsal = Vec::new();
    let mut secenek = Vec::new();
    for d in degerler {
        match d.split_once(':') {
            Some((a, v)) if !a.contains('"') && !a.trim().is_empty() && !a.contains(' ') => {
                secenek.push((a.trim(), v.trim()))
            }
            _ => konumsal.push(d.as_str()),
        }
    }
    (konumsal, secenek)
}

const RENKLER: &[(&str, &str)] = &[
    ("kırmızı", "#e5484d"),
    ("yeşil", "#30a46c"),
    ("mavi", "#3e63dd"),
    ("sarı", "#e2a336"),
    ("turuncu", "#f76b15"),
    ("mor", "#8e4ec6"),
    ("pembe", "#e93d82"),
    ("gri", "#8b8d98"),
    ("siyah", "#111113"),
    ("beyaz", "#ffffff"),
    ("lacivert", "#1e3a8a"),
    ("kahverengi", "#8d5a3b"),
];

fn renk(d: &str) -> Option<[f64; 3]> {
    let d = d.trim().trim_matches('"');
    let hex = RENKLER
        .iter()
        .find(|(a, _)| *a == d)
        .map(|(_, h)| *h)
        .unwrap_or(d);
    let h = hex.strip_prefix('#')?;
    let h: String = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        6 => h.to_string(),
        _ => return None,
    };
    let b = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16)
            .ok()
            .map(|x| x as f64 / 255.0)
    };
    Some([b(0)?, b(2)?, b(4)?])
}

/// WCAG bağıl parlaklık karşıtlığı (1–21).
fn karsitlik(a: [f64; 3], b: [f64; 3]) -> f64 {
    let l = |c: [f64; 3]| {
        let k = |x: f64| {
            if x <= 0.03928 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * k(c[0]) + 0.7152 * k(c[1]) + 0.0722 * k(c[2])
    };
    let (x, y) = (l(a), l(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn bos_metin(d: Option<&&str>) -> bool {
    matches!(d.map(|s| s.trim()), Some("\"\""))
}

/// Arayüz programı (`.ohc`) satırları.
pub fn ohc_denetle(dosya: &Path, kaynak: &str) -> Vec<Bulgu> {
    let mut b = Vec::new();
    for (i, satir) in kaynak.lines().enumerate() {
        let n = i + 1;
        if satir.trim_start().starts_with('#') {
            continue;
        }
        if let Some(d) = cagri_degerleri(satir, "resim") {
            let (k, s) = ayir(&d);
            let aciklama = k
                .get(1)
                .copied()
                .or_else(|| s.iter().find(|(a, _)| *a == "açıklama").map(|(_, v)| *v));
            if aciklama.is_none_or(|a| a == "\"\"") {
                b.push(bulgu(
                    dosya,
                    n,
                    "resim açıklamasız: ekran okuyucular için ikinci değer olarak ne gösterdiğini yazın, ör. resim(\"kedi.png\", \"Uyuyan kedi\")",
                ));
            }
        }
        for ad in ["giriş", "metin_alanı"] {
            if let Some(d) = cagri_degerleri(satir, ad) {
                let (k, s) = ayir(&d);
                let etiketli = k.get(1).is_some_and(|y| *y != "\"\"")
                    || s.iter().any(|(a, _)| *a == "yer_tutucu" || *a == "etiket");
                if !etiketli {
                    b.push(bulgu(
                        dosya,
                        n,
                        format!("{ad} kutusunun ne için olduğu yazmıyor: ikinci değer olarak bir yer tutucu verin, ör. {ad}(ad, \"Adınız\")"),
                    ));
                }
            }
        }
        for ad in ["düğme", "bağlantı"] {
            if let Some(d) = cagri_degerleri(satir, ad) {
                let (k, _) = ayir(&d);
                if bos_metin(k.first()) {
                    b.push(bulgu(
                        dosya,
                        n,
                        format!("{ad} yazısız: ekran okuyucu ne işe yaradığını söyleyemez"),
                    ));
                }
            }
        }
        // renk: ve arka: birlikte verilmişse karşıtlık
        let tum: Vec<String> = [
            "başlık",
            "alt_başlık",
            "yazı",
            "düğme",
            "bağlantı",
            "kart",
            "kutu",
        ]
        .iter()
        .find_map(|a| cagri_degerleri(satir, a))
        .unwrap_or_default();
        let (_, s) = ayir(&tum);
        let bul = |ad: &str| s.iter().find(|(a, _)| *a == ad).and_then(|(_, v)| renk(v));
        if let (Some(on), Some(arka)) = (bul("renk"), bul("arka")) {
            let k = karsitlik(on, arka);
            if k < 4.5 {
                b.push(bulgu(
                    dosya,
                    n,
                    format!("yazı ile arka plan arasındaki karşıtlık düşük ({k:.1}:1, en az 4.5:1 olmalı): renkleri koyulaştırın ya da açın"),
                ));
            }
        }
    }
    b
}

/// HTML etiketinin bir özniteliği var mı (değeri boş değil)?
fn oznitelik<'a>(etiket: &'a str, ad: &str) -> Option<&'a str> {
    let kucuk = etiket.to_ascii_lowercase();
    let mut ara = 0;
    while let Some(i) = kucuk[ara..].find(ad) {
        let bas = ara + i;
        ara = bas + ad.len();
        let once = kucuk[..bas].chars().last();
        if !matches!(once, Some(' ' | '\n' | '\t')) {
            continue;
        }
        let kalan = etiket[ara..].trim_start();
        let Some(v) = kalan.strip_prefix('=') else {
            return Some("");
        };
        let v = v.trim_start();
        let (t, ic) = match v.chars().next() {
            Some(q @ ('"' | '\'')) => (q, &v[1..]),
            _ => return Some(v.split([' ', '>']).next().unwrap_or("")),
        };
        return ic.split(t).next();
    }
    None
}

/// `.ohchtml` / `.html` sayfası.
pub fn html_denetle(dosya: &Path, kaynak: &str) -> Vec<Bulgu> {
    let mut b = Vec::new();
    let kucuk = kaynak.to_ascii_lowercase();
    let satir_no = |i: usize| kaynak[..i].matches('\n').count() + 1;
    if kucuk.contains("<html")
        && oznitelik(&kaynak[kucuk.find("<html").unwrap()..], "lang").is_none_or(str::is_empty)
    {
        b.push(bulgu(
            dosya,
            satir_no(kucuk.find("<html").unwrap()),
            "<html> etiketinde dil yok: <html lang=\"tr\"> yazın",
        ));
    }
    let mut i = 0;
    while let Some(k) = kucuk[i..].find('<') {
        let bas = i + k;
        let Some(son) = kucuk[bas..].find('>') else {
            break;
        };
        let etiket = &kaynak[bas..bas + son + 1];
        let ad: String = kucuk[bas + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        i = bas + son + 1;
        match ad.as_str() {
            "img" if oznitelik(etiket, "alt").is_none() => {
                b.push(bulgu(dosya, satir_no(bas), "<img> etiketinde alt yok: ne gösterdiğini alt=\"...\" ile yazın (süsse alt=\"\")"));
            }
            "input" | "select" | "textarea" => {
                let tur = oznitelik(etiket, "type").unwrap_or("").to_lowercase();
                if matches!(
                    tur.as_str(),
                    "hidden" | "submit" | "button" | "reset" | "image"
                ) {
                    continue;
                }
                let aria = oznitelik(etiket, "aria-label").is_some_and(|v| !v.is_empty())
                    || oznitelik(etiket, "aria-labelledby").is_some();
                // <label> içinde mi ya da for= ile bağlı mı?
                let once = &kucuk[..bas];
                let label_ici = once
                    .rfind("<label")
                    .is_some_and(|l| once.rfind("</label").is_none_or(|s| s < l));
                let for_ile = oznitelik(etiket, "id").is_some_and(|id| {
                    !id.is_empty()
                        && (kucuk.contains(&format!("for=\"{}\"", id.to_ascii_lowercase()))
                            || kucuk.contains(&format!("for='{}'", id.to_ascii_lowercase())))
                });
                if !aria && !label_ici && !for_ile {
                    b.push(bulgu(dosya, satir_no(bas), format!("<{ad}> etiketsiz: <label>Adınız <{ad} ...></label> ya da aria-label kullanın")));
                }
            }
            "button" | "a" => {
                let kapanis = format!("</{ad}");
                if let Some(s) = kucuk[i..].find(&kapanis) {
                    let ic = &kucuk[i..i + s];
                    let yazi: String = {
                        let mut d = 0;
                        ic.chars()
                            .filter(|c| {
                                match c {
                                    '<' => d += 1,
                                    '>' => {
                                        d -= 1;
                                        return false;
                                    }
                                    _ => {}
                                }
                                d == 0
                            })
                            .collect()
                    };
                    let resim_alt = ic.contains("alt=\"") && !ic.contains("alt=\"\"");
                    if yazi.trim().is_empty()
                        && !resim_alt
                        && oznitelik(etiket, "aria-label").is_none_or(str::is_empty)
                        && oznitelik(etiket, "title").is_none_or(str::is_empty)
                    {
                        b.push(bulgu(
                            dosya,
                            satir_no(bas),
                            format!("<{ad}> yazısız: içine yazı ya da aria-label ekleyin"),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    b
}

fn dosyalari_topla(yol: &Path, liste: &mut Vec<PathBuf>) {
    if yol.is_file() {
        liste.push(yol.to_path_buf());
        return;
    }
    let Ok(okunan) = std::fs::read_dir(yol) else {
        return;
    };
    let mut girdiler: Vec<_> = okunan.flatten().map(|g| g.path()).collect();
    girdiler.sort();
    for g in girdiler {
        let ad = g
            .file_name()
            .map(|a| a.to_string_lossy().to_string())
            .unwrap_or_default();
        if ad.starts_with('.')
            || matches!(
                ad.as_str(),
                "veri" | "kütüphaneler" | "target" | "node_modules" | "dağıtım"
            )
        {
            continue;
        }
        if g.is_dir() {
            dosyalari_topla(&g, liste);
        } else if matches!(
            g.extension().and_then(|e| e.to_str()),
            Some("ohc" | "ohchtml" | "html")
        ) {
            liste.push(g);
        }
    }
}

pub fn denetle(yol: &Path) -> Vec<Bulgu> {
    let mut dosyalar = Vec::new();
    dosyalari_topla(yol, &mut dosyalar);
    let mut b = Vec::new();
    for d in dosyalar {
        let Ok(k) = std::fs::read_to_string(&d) else {
            continue;
        };
        if d.extension().is_some_and(|e| e == "ohc") {
            b.extend(ohc_denetle(&d, &k));
        } else {
            b.extend(html_denetle(&d, &k));
        }
    }
    b
}

/// `orhunca erişilebilirlik [yol] [--json]`. Bulgu varsa başarısız döner.
pub fn komut(args: &[String]) -> Result<bool, String> {
    let json = args.iter().any(|a| a == "--json");
    let yol = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if !yol.exists() {
        return Err(format!("{} bulunamadı", yol.display()));
    }
    let b = denetle(&yol);
    if json {
        let l: Vec<_> = b
            .iter()
            .map(|x| serde_json::json!({ "dosya": x.dosya, "satir": x.satir, "mesaj": x.mesaj }))
            .collect();
        println!("{}", serde_json::Value::Array(l));
    } else if b.is_empty() {
        println!("Erişilebilirlik: sorun bulunmadı.");
    } else {
        for x in &b {
            println!("{}:{}: {}", x.dosya.display(), x.satir, x.mesaj);
        }
        println!("\n{} erişilebilirlik uyarısı.", b.len());
    }
    Ok(b.is_empty())
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    fn mesajlar(b: Vec<Bulgu>) -> Vec<String> {
        b.into_iter().map(|x| x.mesaj).collect()
    }

    #[test]
    fn arayuz_programi() {
        let p = Path::new("a.ohc");
        let k = "arayüz:\n    resim(\"a.png\")\n    resim(\"b.png\", \"Kedi\")\n    giriş(ad)\n    giriş(ad, \"Adınız\")\n    düğme(\"\")\n    yazı(\"x\", renk: \"sarı\", arka: \"beyaz\")\n    yazı(\"x\", renk: \"siyah\", arka: \"beyaz\")\n";
        let m = mesajlar(ohc_denetle(p, k));
        assert_eq!(m.len(), 4, "{m:?}");
        assert!(m[0].starts_with("resim açıklamasız"));
        assert!(m[1].starts_with("giriş kutusunun"));
        assert!(m[2].starts_with("düğme yazısız"));
        assert!(m[3].contains("karşıtlık düşük"));
    }

    #[test]
    fn html_sayfasi() {
        let p = Path::new("a.ohchtml");
        let k = "<html>\n<img src=a.png>\n<img src=b.png alt=\"\">\n<label>Ad <input name=ad></label>\n<input name=soyad>\n<input type=hidden name=x>\n<button></button>\n<a href=/>Ana sayfa</a>\n";
        let m = mesajlar(html_denetle(p, k));
        assert_eq!(m.len(), 4, "{m:?}");
        assert!(m[0].contains("dil yok"));
        assert!(m[1].contains("<img>"));
        assert!(m[2].contains("<input> etiketsiz"));
        assert!(m[3].contains("<button> yazısız"));
    }

    #[test]
    fn karsitlik_degerleri() {
        let k = karsitlik(renk("siyah").unwrap(), renk("beyaz").unwrap());
        assert!(k > 18.0);
        assert!((karsitlik(renk("#fff").unwrap(), renk("#ffffff").unwrap()) - 1.0).abs() < 1e-9);
    }
}
