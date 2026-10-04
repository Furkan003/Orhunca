//! Dil sunucusu (Language Server Protocol): `orhunca dil-sunucusu`
//!
//! VS Code ve LSP destekleyen diğer düzenleyiciler stdin/stdout üzerinden bağlanır.
//! Desteklenenler: tanılar (hatalar ve yazım uyarıları), üzerine gelince açıklama,
//! tamamlama, biçimlendirme, tanıma gitme ve belge simgeleri.

use crate::agac::{Program, Tip};
use crate::{bicimlendirici, derleme, yerlesik};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// JSON-RPC iletişimi
// ---------------------------------------------------------------------------

fn mesaj_oku(okuyucu: &mut impl BufRead) -> Option<Value> {
    let mut uzunluk = None;
    loop {
        let mut satir = String::new();
        if okuyucu.read_line(&mut satir).ok()? == 0 {
            return None;
        }
        let satir = satir.trim_end();
        if satir.is_empty() {
            break;
        }
        if let Some(d) = satir.strip_prefix("Content-Length:") {
            uzunluk = d.trim().parse::<usize>().ok();
        }
    }
    let mut govde = vec![0; uzunluk?];
    okuyucu.read_exact(&mut govde).ok()?;
    serde_json::from_slice(&govde).ok()
}

fn gonder(deger: &Value) {
    let govde = deger.to_string();
    let mut cikti = std::io::stdout().lock();
    let _ = write!(cikti, "Content-Length: {}\r\n\r\n{govde}", govde.len());
    let _ = cikti.flush();
}

fn yanitla(kimlik: &Value, sonuc: Value) {
    gonder(&json!({ "jsonrpc": "2.0", "id": kimlik, "result": sonuc }));
}

fn bildir(yontem: &str, parametreler: Value) {
    gonder(&json!({ "jsonrpc": "2.0", "method": yontem, "params": parametreler }));
}

// ---------------------------------------------------------------------------
// URI ve konum dönüşümleri
// ---------------------------------------------------------------------------

fn yuzde_coz(s: &str) -> String {
    let b = s.as_bytes();
    let mut cikti = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                cikti.push(v);
                i += 3;
                continue;
            }
        }
        cikti.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&cikti).into_owned()
}

pub fn uri_yol(uri: &str) -> Option<PathBuf> {
    let yol = yuzde_coz(uri.strip_prefix("file://")?);
    // Windows: file:///c:/Kullanıcılar/... → c:/Kullanıcılar/...
    let b = yol.as_bytes();
    if b.len() > 2 && b[0] == b'/' && b[2] == b':' {
        return Some(PathBuf::from(&yol[1..]));
    }
    Some(PathBuf::from(yol))
}

pub fn yol_uri(yol: &Path) -> String {
    let metin = yol.to_string_lossy().replace('\\', "/");
    let mut kodlu = String::new();
    for b in metin.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~:".contains(&b) {
            kodlu.push(b as char);
        } else {
            kodlu.push_str(&format!("%{b:02X}"));
        }
    }
    if kodlu.starts_with('/') {
        format!("file://{kodlu}")
    } else {
        format!("file:///{kodlu}")
    }
}

/// Karakter sırası (0'dan) → UTF-16 birimi (LSP'nin varsayılanı)
fn utf16_sutun(satir: &str, karakter: usize) -> usize {
    satir.chars().take(karakter).map(char::len_utf16).sum()
}

/// UTF-16 birimi → karakter sırası
fn karakter_sutun(satir: &str, utf16: usize) -> usize {
    let mut toplam = 0;
    for (i, c) in satir.chars().enumerate() {
        if toplam >= utf16 {
            return i;
        }
        toplam += c.len_utf16();
    }
    satir.chars().count()
}

fn aralik(metin: &str, satir: usize, bas: usize, son: usize) -> Value {
    let s = metin.lines().nth(satir).unwrap_or("");
    json!({
        "start": { "line": satir, "character": utf16_sutun(s, bas) },
        "end": { "line": satir, "character": utf16_sutun(s, son) },
    })
}

const HARF_DISI: &[char] = &[
    ' ', '\t', '(', ')', '[', ']', '{', '}', ',', ':', '.', '"', '\'', '’', '=', '+', '-', '*',
    '/', '%', '<', '>', '!', '#',
];

/// Konumdaki kelime ve başlangıç/bitiş karakter sıraları
fn kelime_bul(satir: &str, karakter: usize) -> Option<(String, usize, usize)> {
    let k: Vec<char> = satir.chars().collect();
    let gecerli = |c: char| !HARF_DISI.contains(&c);
    let mut bas = karakter.min(k.len());
    while bas > 0 && gecerli(k[bas - 1]) {
        bas -= 1;
    }
    let mut son = karakter.min(k.len());
    while son < k.len() && gecerli(k[son]) {
        son += 1;
    }
    if bas == son {
        return None;
    }
    Some((k[bas..son].iter().collect(), bas, son))
}

// ---------------------------------------------------------------------------
// Bilgi kaynakları
// ---------------------------------------------------------------------------

const ANAHTAR_KELIMELER: &[(&str, &str)] = &[
    ("eğer", "Koşul: `eğer x 4'ten büyükse:` · `eğer a == b ise:`"),
    ("değilse", "Koşul tutmadığında çalışan blok; `değilse eğer ...:` ile zincirlenebilir."),
    ("ise", "Koşulun sonu: `eğer x > 3 ise:` (isteğe bağlı)"),
    ("her", "Döngü: `her i için 1'den 10'a kadar:` ya da `her öğe için listeden:`"),
    ("için", "`her ... için` döngüsünün parçası"),
    ("kadar", "Aralık sonu: `1'den 10'a kadar` (iki uç dahil)"),
    ("olduğu", "`... olduğu sürece:` döngüsü: koşul doğru olduğu sürece döner"),
    ("sürece", "`x 10'dan küçük olduğu sürece:` döngüsü"),
    ("iken", "`... iken:` döngüsü: koşul doğru olduğu sürece döner"),
    ("işlev", "İşlev tanımı: `işlev topla(a, b):` — tipi yazılmayan parametreler sayıdır"),
    ("fiil", "Fiil tanımı: parametreler hâl ekleriyle belirlenir: `fiil sayı'yı karele:` → `5'i karele.`"),
    ("döndür", "İşlevden ya da fiilden değer döndürür"),
    ("dur", "Döngüden çıkar (break)"),
    ("sürdür", "Döngünün sonraki adımına geçer (continue)"),
    ("kullan", "Başka bir dosyanın işlev, fiil ve sabitlerini alır: `kullan \"araçlar.ohc\"`"),
    ("sabit", "Değişmez değer: `sabit PI = 3.14159` — her yerden görülür"),
    ("ve", "Mantıksal ve (and)"),
    ("veya", "Mantıksal veya (or)"),
    ("değil", "Mantıksal değil (not)"),
    ("doğru", "Mantık değeri: doğru (true)"),
    ("yanlış", "Mantık değeri: yanlış (false)"),
    ("yaz", "Fiil: belirtme hâlindeki değeri yazar: `x'i ekrana yaz.` · dosyaya: `metni \"not.txt\"'ye yaz.`"),
    ("ekle", "Fiil: `5'i sayılara ekle.` — öğe (-i) ve liste (-e)"),
    ("sırala", "Fiil: `sayıları sırala.` — metinler Türk alfabesine göre sıralanır"),
    ("çıkar", "Fiil: `5'i sayılardan çıkar.` — ilk eşleşen öğeyi siler"),
    ("ekrana", "`yaz` fiilinin hedefi (isteğe bağlı)"),
    ("model", "Veri modeli: `model Ürün:` ve altında `ad: metin, zorunlu` gibi alanlar"),
    ("zorunlu", "Model alanı boş bırakılamaz"),
    ("en_az", "Model alanı kuralı: sayılarda en küçük değer, metinlerde en az karakter"),
    ("en_fazla", "Model alanı kuralı: sayılarda en büyük değer, metinlerde en çok karakter"),
    ("kaydet", "Fiil: modeli veri deposuna yazar: `ürün'ü kaydet.`"),
    ("al", "Web yolu (GET): `al \"/ürünler\":` — `istek` değişkeni kullanılabilir"),
    ("gönder", "Web yolu (POST): `gönder \"/ürünler\":` — form: `Ürün.formdan(istek)`"),
    ("koy", "Web yolu (PUT): `koy \"/ürünler/{kimlik: sayı}\":`"),
    ("istek", "Gelen web isteği: istek.yöntem, istek.yol, istek.sorgu, istek.form, istek.gövde, istek.başlıklar"),
];

const TIP_ADLARI: &[(&str, &str)] = &[
    ("sayı", "64 bitlik tamsayı"),
    ("ondalık", "64 bitlik kayan noktalı sayı"),
    ("metin", "UTF-8 metin"),
    ("mantık", "doğru / yanlış"),
    ("liste", "liste<T>: aynı tipte öğeler"),
    (
        "sözlük",
        "sözlük<A, D>: anahtarı sayı ya da metin olan eşleme",
    ),
];

struct Tanim {
    tur: &'static str,
    ad: String,
    satir: usize,
    bas: usize,
    metin: String,
}

/// Belgedeki işlev, fiil, sabit tanımları ve değişkenlerin ilk atamaları
fn tanimlar(metin: &str) -> Vec<Tanim> {
    let mut cikti = Vec::new();
    for (i, satir) in metin.lines().enumerate() {
        let govde = satir.trim_start();
        let girinti = satir.chars().count() - govde.chars().count();
        let ekle = |cikti: &mut Vec<Tanim>, tur, ad: &str| {
            let bas = satir
                .chars()
                .collect::<String>()
                .find(ad)
                .map(|b| satir[..b].chars().count())
                .unwrap_or(girinti);
            cikti.push(Tanim {
                tur,
                ad: ad.to_string(),
                satir: i,
                bas,
                metin: govde.to_string(),
            });
        };
        if let Some(r) = govde.strip_prefix("işlev ") {
            if let Some(ad) = r.split(['(', ' ']).next().filter(|a| !a.is_empty()) {
                ekle(&mut cikti, "işlev", ad);
            }
        } else if govde.starts_with("fiil ") {
            let bas = govde.trim_end().trim_end_matches(':');
            let bas = bas.split("->").next().unwrap_or(bas).trim_end();
            if let Some(ad) = bas
                .rsplit([' ', ')', '\'', '’'])
                .next()
                .filter(|a| !a.is_empty())
            {
                ekle(&mut cikti, "fiil", ad);
            }
        } else if let Some(r) = govde.strip_prefix("sabit ") {
            if let Some(ad) = r.split(['=', ' ']).next().filter(|a| !a.is_empty()) {
                ekle(&mut cikti, "sabit", ad);
            }
        } else if let Some(r) = govde.strip_prefix("model ").filter(|_| girinti == 0) {
            if let Some(ad) = r.trim_end().strip_suffix(':').map(str::trim) {
                ekle(&mut cikti, "model", ad);
            }
        } else if let Some(kalip) = rota_satiri(satir) {
            ekle(&mut cikti, "yol", &kalip);
        } else if let Some((ad, _)) = govde.split_once('=') {
            let ad = ad.trim().trim_end_matches(['+', '-']).trim();
            let ilk = !cikti.iter().any(|t: &Tanim| t.ad == ad);
            if ilk
                && !ad.is_empty()
                && !ad.contains([' ', '[', '(', '!', '<', '>', '.', ':'])
                && !govde[govde.find('=').unwrap() + 1..].starts_with('=')
            {
                ekle(&mut cikti, "değişken", ad);
            }
        } else if let Some(r) = govde.strip_prefix("her ") {
            if let Some(ad) = r.split(' ').next().filter(|a| !a.is_empty()) {
                if !cikti.iter().any(|t| t.ad == ad) {
                    ekle(&mut cikti, "değişken", ad);
                }
            }
        }
    }
    cikti
}

/// `al "/ürünler":` gibi bir web yolu satırıysa kalıbı verir.
fn rota_satiri(satir: &str) -> Option<String> {
    let (kelime, kalan) = satir.split_once(' ')?;
    if !["al", "gönder", "koy", "sil"].contains(&kelime) {
        return None;
    }
    let kalip = kalan.trim().strip_suffix(':')?.trim();
    kalip.strip_prefix('"')?.strip_suffix('"')?;
    Some(format!("{kelime} {kalip}"))
}

/// `kullan` ile alınan dosyaların yolları
fn kullanilan_dosyalar(metin: &str, klasor: &Path) -> Vec<PathBuf> {
    metin
        .lines()
        .filter_map(|s| s.trim().strip_prefix("kullan "))
        .filter_map(|s| {
            s.trim()
                .trim_end_matches('.')
                .strip_prefix('"')?
                .strip_suffix('"')
                .map(str::to_string)
        })
        .map(|y| klasor.join(y))
        .collect()
}

/// Program denetlendiyse değişkenin tipi (içinde bulunduğu işleve göre)
fn degisken_tipi(program: &Program, metin: &str, satir: usize, ad: &str) -> Option<Tip> {
    // Satırın içinde bulunduğu işlev: yukarıdaki en yakın girintisiz işlev/fiil satırı
    let satirlar: Vec<&str> = metin.lines().collect();
    let mut islev_satiri = None;
    for i in (0..=satir.min(satirlar.len().saturating_sub(1))).rev() {
        let s = satirlar[i];
        if s.starts_with(' ')
            || s.starts_with('\t')
            || s.trim().is_empty()
            || s.trim_start().starts_with('#')
        {
            continue;
        }
        if s.starts_with("işlev ") || s.starts_with("fiil ") || rota_satiri(s).is_some() {
            islev_satiri = Some(i + 1);
        }
        break;
    }
    let yereller = match islev_satiri {
        Some(n) => program
            .islevler
            .iter()
            .find(|f| f.konum.dosya == 0 && f.konum.satir == n)
            .map(|f| &f.yereller)?,
        None => &program.ana_yereller,
    };
    yereller
        .iter()
        .find(|(a, _)| a == ad)
        .map(|(_, t)| t.clone())
}

// ---------------------------------------------------------------------------
// Sunucu
// ---------------------------------------------------------------------------

struct Sunucu {
    belgeler: HashMap<String, String>,
    /// Son yayımlanan tanıların URI'leri (temizlemek için)
    yayimlanan: HashMap<String, Vec<String>>,
}

impl Sunucu {
    fn ortulu(&self) -> HashMap<PathBuf, String> {
        self.belgeler
            .iter()
            .filter_map(|(u, m)| Some((std::fs::canonicalize(uri_yol(u)?).ok()?, m.clone())))
            .collect()
    }

    fn denetle(&mut self, uri: &str) {
        let Some(metin) = self.belgeler.get(uri).cloned() else {
            return;
        };
        let mut tanilar: HashMap<String, Vec<Value>> = HashMap::new();
        tanilar.insert(uri.to_string(), Vec::new());
        // Görünümler (.ohchtml) projenin giriş dosyasıyla birlikte derlenir.
        let gorunum = uri.ends_with(".ohchtml");
        let derlenecek = uri_yol(uri).filter(|y| y.exists()).and_then(|y| {
            if gorunum {
                derleme::proje_girisi(&derleme::proje_koku(&y))
                    .ok()
                    .filter(|g| g.exists())
            } else {
                Some(y)
            }
        });

        for u in bicimlendirici::uyarilar(&metin)
            .into_iter()
            .filter(|_| !gorunum)
        {
            let satir = u.konum.satir - 1;
            tanilar.get_mut(uri).unwrap().push(json!({
                "range": aralik(&metin, satir, u.konum.sutun - 1, u.konum.sutun - 1 + u.uzunluk),
                "severity": 2,
                "source": "orhunca",
                "message": u.mesaj,
            }));
        }

        if let Some(yol) = derlenecek {
            if let Err(h) = derleme::yukle_ortulu(&yol, &self.ortulu()) {
                if let Some(t) = &h.teshis {
                    let hedef = std::fs::canonicalize(&t.dosya)
                        .map(|p| {
                            // Açık belgenin istemcinin kullandığı URI'si korunur
                            self.belgeler
                                .keys()
                                .find(|u| {
                                    uri_yol(u)
                                        .and_then(|y| std::fs::canonicalize(y).ok())
                                        .as_ref()
                                        == Some(&p)
                                })
                                .cloned()
                                .unwrap_or_else(|| yol_uri(&p))
                        })
                        .unwrap_or_else(|_| uri.to_string());
                    let kaynak = self
                        .belgeler
                        .get(&hedef)
                        .cloned()
                        .or_else(|| std::fs::read_to_string(&t.dosya).ok())
                        .unwrap_or_default();
                    let satir = t.satir.saturating_sub(1);
                    let uzunluk = kaynak
                        .lines()
                        .nth(satir)
                        .map(|s| s.chars().count())
                        .unwrap_or(0);
                    let bas = t.sutun.saturating_sub(1);
                    let mut mesaj = t.mesaj.clone();
                    if let Some(i) = &t.ipucu {
                        mesaj.push_str(&format!("\nipucu: {i}"));
                    }
                    tanilar.entry(hedef).or_default().push(json!({
                        "range": aralik(&kaynak, satir, bas, uzunluk.max(bas + 1)),
                        "severity": 1,
                        "source": "orhunca",
                        "message": mesaj,
                    }));
                } else {
                    tanilar.get_mut(uri).unwrap().push(json!({
                        "range": aralik(&metin, 0, 0, 0),
                        "severity": 1,
                        "source": "orhunca",
                        "message": h.metin,
                    }));
                }
            }
        }

        // Önceki denetimde hata verilen ama artık temiz olan dosyalar temizlenir.
        let onceki = self.yayimlanan.remove(uri).unwrap_or_default();
        for u in onceki {
            tanilar.entry(u).or_default();
        }
        self.yayimlanan.insert(
            uri.to_string(),
            tanilar.keys().filter(|u| *u != uri).cloned().collect(),
        );
        for (u, d) in tanilar {
            bildir(
                "textDocument/publishDiagnostics",
                json!({ "uri": u, "diagnostics": d }),
            );
        }
    }

    fn uzerine_gelince(&self, p: &Value) -> Value {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(metin) = self.belgeler.get(uri) else {
            return Value::Null;
        };
        let satir_no = p["position"]["line"].as_u64().unwrap_or(0) as usize;
        let satir = metin.lines().nth(satir_no).unwrap_or("");
        let karakter = karakter_sutun(
            satir,
            p["position"]["character"].as_u64().unwrap_or(0) as usize,
        );
        let Some((kelime, bas, son)) = kelime_bul(satir, karakter) else {
            return Value::Null;
        };

        let icerik = if let Some(y) = yerlesik::bul(&kelime) {
            format!("```orhunca\n{}\n```\n{}", y.kullanim, y.aciklama)
        } else if let Some((_, a)) = ANAHTAR_KELIMELER.iter().find(|(k, _)| *k == kelime) {
            format!("**{kelime}** — {a}")
        } else if let Some((_, a)) = TIP_ADLARI.iter().find(|(k, _)| *k == kelime) {
            format!("**{kelime}** (tip) — {a}")
        } else {
            let mut bulunan = None;
            let mut kaynaklar = vec![metin.clone()];
            if let Some(y) = uri_yol(uri) {
                for d in kullanilan_dosyalar(metin, y.parent().unwrap_or(Path::new("."))) {
                    if let Ok(m) = std::fs::read_to_string(d) {
                        kaynaklar.push(m);
                    }
                }
            }
            for k in &kaynaklar {
                if let Some(t) = tanimlar(k)
                    .into_iter()
                    .find(|t| t.ad == kelime && t.tur != "değişken")
                {
                    bulunan = Some(format!("```orhunca\n{}\n```\n{}", t.metin, t.tur));
                    break;
                }
            }
            if bulunan.is_none() {
                if let Some(yol) = uri_yol(uri) {
                    if let Ok(program) = derleme::yukle_ortulu(&yol, &self.ortulu()) {
                        if let Some(tip) = degisken_tipi(&program, metin, satir_no, &kelime) {
                            bulunan = Some(format!("```orhunca\n{kelime}: {tip}\n```\ndeğişken"));
                        }
                    }
                }
            }
            match bulunan {
                Some(b) => b,
                None => return Value::Null,
            }
        };
        json!({
            "contents": { "kind": "markdown", "value": icerik },
            "range": aralik(metin, satir_no, bas, son),
        })
    }

    fn tamamla(&self, p: &Value) -> Value {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let metin = self.belgeler.get(uri).cloned().unwrap_or_default();
        let mut ogeler = Vec::new();
        for (k, a) in ANAHTAR_KELIMELER {
            ogeler.push(json!({ "label": k, "kind": 14, "detail": a }));
        }
        let parcaciklar = [
            ("eğer … ise", "eğer ${1:koşul} ise:\n    $0"),
            (
                "her … için … kadar",
                "her ${1:i} için ${2:1}'den ${3:10}'a kadar:\n    $0",
            ),
            (
                "her … için listeden",
                "her ${1:öğe} için ${2:liste}den:\n    $0",
            ),
            ("… olduğu sürece", "${1:koşul} olduğu sürece:\n    $0"),
            ("işlev", "işlev ${1:ad}(${2:a}):\n    döndür $0"),
            ("fiil", "fiil ${1:x}'i ${2:ad}:\n    döndür $0"),
            (
                "model",
                "model ${1:Ürün}:\n    ${2:ad}: ${3:metin}, zorunlu\n    $0",
            ),
            ("al (web yolu)", "al \"/${1:yol}\":\n    döndür $0"),
            (
                "gönder (web formu)",
                "gönder \"/${1:yol}\":\n    ${2:x} = ${3:Model}.formdan(istek)\n    $0",
            ),
        ];
        for (ad, govde) in parcaciklar {
            ogeler.push(
                json!({ "label": ad, "kind": 15, "insertText": govde, "insertTextFormat": 2 }),
            );
        }
        for y in yerlesik::YERLESIKLER {
            ogeler.push(json!({
                "label": y.ad, "kind": 3, "detail": y.kullanim,
                "documentation": y.aciklama, "insertText": format!("{}($1)", y.ad), "insertTextFormat": 2,
            }));
        }
        for (t, _) in TIP_ADLARI {
            ogeler.push(json!({ "label": t, "kind": 22 }));
        }
        let mut gorulen = std::collections::HashSet::new();
        for t in tanimlar(&metin) {
            if gorulen.insert(t.ad.clone()) {
                let tur = match t.tur {
                    "işlev" => 3,
                    "fiil" => 2,
                    "sabit" => 21,
                    "model" => 7,
                    "yol" => continue,
                    _ => 6,
                };
                ogeler.push(json!({ "label": t.ad, "kind": tur, "detail": t.metin }));
            }
        }
        json!({ "isIncomplete": false, "items": ogeler })
    }

    fn tanima_git(&self, p: &Value) -> Value {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(metin) = self.belgeler.get(uri) else {
            return Value::Null;
        };
        let satir_no = p["position"]["line"].as_u64().unwrap_or(0) as usize;
        let satir = metin.lines().nth(satir_no).unwrap_or("");
        let karakter = karakter_sutun(
            satir,
            p["position"]["character"].as_u64().unwrap_or(0) as usize,
        );
        let Some((kelime, _, _)) = kelime_bul(satir, karakter) else {
            return Value::Null;
        };
        let mut adaylar = vec![(uri.to_string(), metin.clone())];
        if let Some(y) = uri_yol(uri) {
            for d in kullanilan_dosyalar(metin, y.parent().unwrap_or(Path::new("."))) {
                if let Ok(m) = std::fs::read_to_string(&d) {
                    adaylar.push((yol_uri(&std::fs::canonicalize(&d).unwrap_or(d)), m));
                }
            }
        }
        for (u, m) in adaylar {
            if let Some(t) = tanimlar(&m).into_iter().find(|t| t.ad == kelime) {
                let son = t.bas + t.ad.chars().count();
                return json!({ "uri": u, "range": aralik(&m, t.satir, t.bas, son) });
            }
        }
        Value::Null
    }

    fn simgeler(&self, p: &Value) -> Value {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let metin = self.belgeler.get(uri).cloned().unwrap_or_default();
        let liste: Vec<Value> = tanimlar(&metin)
            .into_iter()
            .filter(|t| t.tur != "değişken" || !metin.lines().nth(t.satir).unwrap_or("").starts_with([' ', '\t']))
            .map(|t| {
                let tur = match t.tur {
                    "işlev" => 12,
                    "fiil" => 6,
                    "sabit" => 14,
                    "model" => 23,
                    "yol" => 7,
                    _ => 13,
                };
                let r = aralik(&metin, t.satir, t.bas, t.bas + t.ad.chars().count());
                json!({ "name": t.ad, "detail": t.tur, "kind": tur, "range": r, "selectionRange": r })
            })
            .collect();
        Value::Array(liste)
    }

    fn bicimlendir(&self, p: &Value) -> Value {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(metin) = self.belgeler.get(uri) else {
            return Value::Null;
        };
        let yeni = bicimlendirici::bicimlendir(metin);
        if &yeni == metin {
            return json!([]);
        }
        let satir_sayisi = metin.lines().count() + 1;
        json!([{
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": satir_sayisi, "character": 0 } },
            "newText": yeni,
        }])
    }
}

pub fn calistir() -> Result<(), String> {
    let mut okuyucu = BufReader::new(std::io::stdin().lock());
    let mut s = Sunucu {
        belgeler: HashMap::new(),
        yayimlanan: HashMap::new(),
    };
    let mut kapaniyor = false;
    while let Some(m) = mesaj_oku(&mut okuyucu) {
        let yontem = m["method"].as_str().unwrap_or("");
        let kimlik = m.get("id").cloned();
        let p = &m["params"];
        match yontem {
            "initialize" => yanitla(
                kimlik.as_ref().unwrap_or(&Value::Null),
                json!({
                    "capabilities": {
                        "textDocumentSync": { "openClose": true, "change": 1, "save": { "includeText": false } },
                        "hoverProvider": true,
                        "completionProvider": { "triggerCharacters": [] },
                        "definitionProvider": true,
                        "documentSymbolProvider": true,
                        "documentFormattingProvider": true,
                    },
                    "serverInfo": { "name": "orhunca", "version": env!("CARGO_PKG_VERSION") },
                }),
            ),
            "textDocument/didOpen" => {
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("").to_string();
                s.belgeler.insert(
                    uri.clone(),
                    p["textDocument"]["text"].as_str().unwrap_or("").to_string(),
                );
                s.denetle(&uri);
            }
            "textDocument/didChange" => {
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("").to_string();
                if let Some(son) = p["contentChanges"].as_array().and_then(|d| d.last()) {
                    s.belgeler
                        .insert(uri.clone(), son["text"].as_str().unwrap_or("").to_string());
                }
                s.denetle(&uri);
            }
            "textDocument/didSave" => {
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("").to_string();
                s.denetle(&uri);
            }
            "textDocument/didClose" => {
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("").to_string();
                s.belgeler.remove(&uri);
                bildir(
                    "textDocument/publishDiagnostics",
                    json!({ "uri": uri, "diagnostics": [] }),
                );
            }
            "textDocument/hover" => yanitla(kimlik.as_ref().unwrap(), s.uzerine_gelince(p)),
            "textDocument/completion" => yanitla(kimlik.as_ref().unwrap(), s.tamamla(p)),
            "textDocument/definition" => yanitla(kimlik.as_ref().unwrap(), s.tanima_git(p)),
            "textDocument/documentSymbol" => yanitla(kimlik.as_ref().unwrap(), s.simgeler(p)),
            "textDocument/formatting" => yanitla(kimlik.as_ref().unwrap(), s.bicimlendir(p)),
            "shutdown" => {
                kapaniyor = true;
                yanitla(kimlik.as_ref().unwrap_or(&Value::Null), Value::Null);
            }
            "exit" => break,
            _ => {
                // Bilinmeyen istekler yanıtsız kalmasın
                if let Some(k) = kimlik {
                    gonder(&json!({
                        "jsonrpc": "2.0", "id": k,
                        "error": { "code": -32601, "message": format!("desteklenmeyen yöntem: {yontem}") },
                    }));
                }
            }
        }
    }
    if kapaniyor {
        Ok(())
    } else {
        Err("istemci bağlantıyı 'shutdown' göndermeden kapattı".into())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn uri_donusumu() {
        let y = PathBuf::from("/ev/kullanıcı/örnek proje/ana.ohc");
        let u = yol_uri(&y);
        assert_eq!(
            u,
            "file:///ev/kullan%C4%B1c%C4%B1/%C3%B6rnek%20proje/ana.ohc"
        );
        assert_eq!(uri_yol(&u).unwrap(), y);
        assert_eq!(
            uri_yol("file:///c%3A/Projeler/a.ohc").unwrap(),
            PathBuf::from("c:/Projeler/a.ohc")
        );
    }

    #[test]
    fn utf16_sutunlari() {
        let s = "a𐰆b";
        assert_eq!(utf16_sutun(s, 2), 3);
        assert_eq!(karakter_sutun(s, 3), 2);
    }

    #[test]
    fn tanimlari_bulur() {
        let t = tanimlar("sabit PI = 3.14\nişlev topla(a, b):\n    döndür a + b\nfiil sayı'yı karele:\n    döndür sayı * sayı\nx = 5\n");
        let adlar: Vec<_> = t.iter().map(|t| (t.tur, t.ad.as_str())).collect();
        assert_eq!(
            adlar,
            vec![
                ("sabit", "PI"),
                ("işlev", "topla"),
                ("fiil", "karele"),
                ("değişken", "x")
            ]
        );
    }
}
