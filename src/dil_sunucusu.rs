//! Dil sunucusu (Language Server Protocol): `orhunca dil-sunucusu`
//!
//! VS Code ve LSP destekleyen diğer düzenleyiciler stdin/stdout üzerinden bağlanır.
//! Desteklenenler: tanılar (hatalar ve yazım uyarıları), üzerine gelince açıklama,
//! tamamlama, biçimlendirme, tanıma gitme ve belge simgeleri.

use crate::agac::{Program, Tip};
use crate::{arayuz, bicimlendirici, derleme, yerlesik};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// JSON-RPC iletişimi
// ---------------------------------------------------------------------------

pub(crate) fn mesaj_oku(okuyucu: &mut impl BufRead) -> Option<Value> {
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

pub(crate) fn gonder(deger: &Value) {
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
pub(crate) fn utf16_sutun(satir: &str, karakter: usize) -> usize {
    satir.chars().take(karakter).map(char::len_utf16).sum()
}

/// UTF-16 birimi → karakter sırası
pub(crate) fn karakter_sutun(satir: &str, utf16: usize) -> usize {
    let mut toplam = 0;
    for (i, c) in satir.chars().enumerate() {
        if toplam >= utf16 {
            return i;
        }
        toplam += c.len_utf16();
    }
    satir.chars().count()
}

/// Tanıların taşıdığı düzeltmelerden "quick fix" eylemleri.
fn hizli_duzeltmeler(p: &Value) -> Value {
    let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
    let eylemler: Vec<Value> = p["context"]["diagnostics"]
        .as_array()
        .map(|l| l.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|t| {
            let d = t["data"].get("duzeltme")?;
            Some(json!({
                "title": d["baslik"],
                "kind": "quickfix",
                "diagnostics": [t],
                "isPreferred": true,
                "edit": { "changes": { uri: [{ "range": d["range"], "newText": d["yeni"] }] } },
            }))
        })
        .collect();
    Value::Array(eylemler)
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

pub(crate) const ANAHTAR_KELIMELER: &[(&str, &str)] = &[
    ("eğer", "Koşul: `eğer x 4'ten büyükse:` · `eğer a == b ise:`"),
    ("değilse", "Koşul tutmadığında çalışan blok; `değilse eğer ...:` ile zincirlenebilir."),
    ("ise", "Koşulun sonu: `eğer x > 3 ise:` (isteğe bağlı)"),
    ("her", "Döngü: `her i için 1'den 10'a kadar:` ya da `her öğe için listeden:`"),
    ("için", "`her ... için` döngüsünün parçası"),
    ("kadar", "Aralık sonu: `1'den 10'a kadar` (iki uç dahil)"),
    ("olduğu", "`... olduğu sürece:` döngüsü: koşul doğru olduğu sürece döner"),
    ("sürece", "`x 10'dan küçük olduğu sürece:` döngüsü"),
    ("iken", "`... iken:` döngüsü: koşul doğru olduğu sürece döner"),
    ("işlev", "İşlev tanımı: `işlev topla(a, b):` — tipi yazılmayan parametre ilk çağrıdaki değerin tipini alır"),
    ("fiil", "Fiil tanımı: parametreler hâl ekleriyle belirlenir: `fiil sayı'yı karele:` → `5'i karele.`"),
    ("döndür", "İşlevden ya da fiilden değer döndürür"),
    ("dur", "Döngüden çıkar (break)"),
    ("sürdür", "Döngünün sonraki adımına geçer (continue)"),
    ("dene", "Hata yakalama: `dene:` bloğunda bir çalışma hatası olursa `yakala` bloğu çalışır"),
    ("yakala", "`dene:` bloğundaki hatayı yakalar: `yakala hata:` — `hata` hatanın mesajıdır (metin)"),
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
    ("seçenek", "Seçenek türü (numaralandırma): `seçenek Renk: kırmızı, yeşil, mavi` — değer: `Renk.kırmızı`, hepsi: `Renk.hepsi()`, metinden: `Renk(m)`"),
    ("zorunlu", "Model alanı boş bırakılamaz"),
    ("en_az", "Model alanı kuralı: sayılarda en küçük değer, metinlerde en az karakter"),
    ("en_fazla", "Model alanı kuralı: sayılarda en büyük değer, metinlerde en çok karakter"),
    ("kaydet", "Fiil: modeli veri deposuna yazar: `ürün'ü kaydet.`"),
    ("al", "Web yolu (GET): `al \"/ürünler\":` — `istek` değişkeni kullanılabilir"),
    ("gönder", "Web yolu (POST): `gönder \"/ürünler\":` — form: `Ürün.formdan(istek)`"),
    ("koy", "Web yolu (PUT): `koy \"/ürünler/{kimlik: sayı}\":`"),
    ("istek", "Gelen web isteği: istek.yöntem, istek.yol, istek.sorgu, istek.form, istek.gövde, istek.başlıklar, istek.çerezler, istek.oturum, istek.dosyalar"),
    ("durum", "Arayüz programının değişkeni: `durum sayaç = 0` — her yerden görülür; bir olaydan sonra arayüz yeniden çizilir"),
    ("arayüz", "Ekranda görünenler: `arayüz:` ve altında `başlık(...)`, `düğme(...)`, `satır:` gibi öğeler (tarayıcıda çalışır)"),
    ("bileşen", "Arayüzün yeniden kullanılan parçası: `bileşen Kart(başlık: metin):` — arayüzde `Kart(\"...\")` diye kullanılır"),
    ("tıklanınca", "Olay: `düğme(\"Ekle\") tıklanınca:` — blok tıklanınca çalışır"),
    ("değişince", "Olay: `giriş(ad) değişince:` — kullanıcı değeri değiştirince (bağlı değişken güncellendikten sonra)"),
    ("gönderilince", "Olay: `giriş(ad) gönderilince:` — giriş kutusunda Enter'a basılınca"),
    ("çalınca", "Olay: `zamanlayıcı(1) çalınca:` — her süre dolduğunda"),
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

pub(crate) fn girinti(satir: &str) -> usize {
    satir
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .count()
}

/// Kendi kapsamı olan üst düzey blok başlığı (işlev, fiil, web yolu, bileşen).
pub(crate) fn kapsam_basligi(govde: &str) -> bool {
    [
        "işlev ",
        "fiil ",
        "al ",
        "gönder ",
        "koy ",
        "sil ",
        "bileşen ",
    ]
    .iter()
    .any(|b| govde.starts_with(b))
}

/// Satırda `ad`ın tam kelime olarak geçtiği ilk yer (karakter sırası), `baslangic`tan itibaren.
fn kelime_konumu(satir: &str, ad: &str, baslangic: usize) -> Option<usize> {
    let k: Vec<char> = satir.chars().collect();
    let a: Vec<char> = ad.chars().collect();
    let harf = |c: char| !HARF_DISI.contains(&c);
    (baslangic..k.len().saturating_sub(a.len() - 1)).find(|&i| {
        k[i..i + a.len()] == a[..]
            && (i == 0 || !harf(k[i - 1]))
            && k.get(i + a.len()).is_none_or(|c| !harf(*c))
    })
}

/// Satır `ad`a ilk değer veren bir atama ya da döngü değişkeni tanımı mı? Konumu döner.
fn atama_konumu(satir: &str, ad: &str) -> Option<usize> {
    let g = girinti(satir);
    let govde: String = satir.chars().skip(g).collect();
    if let Some(r) = govde.strip_prefix("her ") {
        return (r.split(' ').next() == Some(ad)).then_some(g + 4);
    }
    let r = govde.strip_prefix(ad)?;
    let r = r.trim_start();
    let atama = (r.starts_with('=') && !r.starts_with("=="))
        || (r.starts_with(':') && r.contains('=') && !r.contains("=="));
    atama.then_some(g)
}

/// Kapsamı bilen tanım arama. Kullanım bir işlevin (fiil, yol, bileşen) içindeyse önce o
/// bloğun başlığındaki parametrelere, sonra bloktaki ilk atamaya; değilse yalnızca blokların
/// dışındaki (üst düzey) ilk atamaya bakılır. Bulunamazsa None (genel aramaya düşülür).
fn kapsamli_tanim(metin: &str, satir_no: usize, ad: &str) -> Option<(usize, usize)> {
    let satirlar: Vec<&str> = metin.lines().collect();
    let dolu = |s: &str| {
        let t = s.trim();
        !t.is_empty() && !t.starts_with('#')
    };
    // Kullanımı içeren üst düzey blok: geriye doğru ilk girintisiz satır.
    let blok_basi = (0..=satir_no.min(satirlar.len().saturating_sub(1)))
        .rev()
        .find(|&i| dolu(satirlar[i]) && girinti(satirlar[i]) == 0)?;
    let baslik = satirlar[blok_basi].trim_start();
    let blok_sonu = |b: usize| {
        (b + 1..satirlar.len())
            .find(|&i| dolu(satirlar[i]) && girinti(satirlar[i]) == 0)
            .unwrap_or(satirlar.len())
    };
    if kapsam_basligi(baslik) {
        // Başlıktaki parametre: işlevde ilk parantezden, fiilde baştan itibaren aranır;
        // işlevin/fiilin kendi adı parametre sayılmaz.
        let bas = if baslik.starts_with("işlev ") || baslik.starts_with("bileşen ") {
            satirlar[blok_basi].chars().position(|c| c == '(')
        } else {
            Some(0)
        };
        if let Some(b) = bas {
            let isim = tanimlar(satirlar[blok_basi])
                .first()
                .map(|t| (t.ad.clone(), t.bas));
            let mut i = b;
            while let Some(k) = kelime_konumu(satirlar[blok_basi], ad, i) {
                if isim.as_ref().is_some_and(|(n, nb)| n == ad && *nb == k) {
                    i = k + 1;
                    continue;
                }
                return Some((blok_basi, k));
            }
        }
        // Bloğun içindeki ilk atama.
        let son = blok_sonu(blok_basi);
        if let Some((i, k)) = satirlar[blok_basi + 1..son]
            .iter()
            .enumerate()
            .find_map(|(j, s)| atama_konumu(s, ad).map(|k| (blok_basi + 1 + j, k)))
        {
            return Some((i, k));
        }
        // Blokta tanımlı değil: üst düzey değişkene ancak blok dışında ise gidilir.
    }
    // Üst düzey (blokların dışındaki) ilk atama.
    let mut i = 0;
    while i < satirlar.len() {
        let s = satirlar[i];
        if dolu(s) && girinti(s) == 0 && kapsam_basligi(s.trim_start()) {
            i = blok_sonu(i);
            continue;
        }
        if let Some(k) = atama_konumu(s, ad) {
            return Some((i, k));
        }
        i += 1;
    }
    None
}

pub(crate) struct Tanim {
    pub tur: &'static str,
    pub ad: String,
    pub satir: usize,
    pub bas: usize,
    pub metin: String,
}

/// Belgedeki işlev, fiil, sabit tanımları ve değişkenlerin ilk atamaları
pub(crate) fn tanimlar(metin: &str) -> Vec<Tanim> {
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
        } else if let Some(r) = govde.strip_prefix("seçenek ").filter(|_| girinti == 0) {
            if let Some((ad, _)) = r.split_once(':') {
                ekle(&mut cikti, "seçenek", ad.trim());
            }
        } else if let Some(kalip) = rota_satiri(satir) {
            ekle(&mut cikti, "yol", &kalip);
        } else if let Some(r) = govde.strip_prefix("bileşen ") {
            if let Some(ad) = r.split(['(', ' ']).next().filter(|a| !a.is_empty()) {
                ekle(&mut cikti, "bileşen", ad);
            }
        } else if let Some(r) = govde.strip_prefix("durum ").filter(|_| girinti == 0) {
            if let Some(ad) = r.split(['=', ':', ' ']).next().filter(|a| !a.is_empty()) {
                ekle(&mut cikti, "durum", ad);
            }
        } else if girinti == 0 && govde.trim_end() == "arayüz:" {
            ekle(&mut cikti, "arayüz", "arayüz");
        } else if let Some((ad, _)) = govde.split_once('=') {
            // `liste: liste<metin> = []` → liste
            let ad = ad.split(':').next().unwrap_or(ad);
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
    /// Belge ve projesindeki öteki .ohc dosyaları (açık olanların düzenleyicideki hâliyle);
    /// ilk öğe belgenin kendisidir. URI'ler aynı sırayla döner.
    fn proje_kaynaklari(&self, uri: &str) -> (Vec<(PathBuf, String)>, Vec<String>) {
        let mut kaynaklar = Vec::new();
        let mut urler = Vec::new();
        let Some(metin) = self.belgeler.get(uri) else {
            return (kaynaklar, urler);
        };
        let yol = uri_yol(uri).unwrap_or_default();
        kaynaklar.push((yol.clone(), metin.clone()));
        urler.push(uri.to_string());
        let ortulu = self.ortulu();
        let tam = std::fs::canonicalize(&yol).unwrap_or(yol.clone());
        let kok = derleme::proje_koku(&yol);
        fn gez(k: &Path, l: &mut Vec<PathBuf>, derinlik: usize) {
            let Ok(g) = std::fs::read_dir(k) else { return };
            for x in g.flatten() {
                let ad = x.file_name().to_string_lossy().into_owned();
                if ad.starts_with('.') || matches!(ad.as_str(), "cikti" | "target" | "paketler") {
                    continue;
                }
                let p = x.path();
                if p.is_dir() && derinlik < 8 {
                    gez(&p, l, derinlik + 1);
                } else if p.extension().is_some_and(|e| e == "ohc") {
                    l.push(p);
                }
            }
        }
        let mut dosyalar = Vec::new();
        if kok.join(".ohcproj").exists() || derleme::proje_dosyasi(&kok).is_some() {
            gez(&kok, &mut dosyalar, 0);
        } else if let Some(k) = yol.parent() {
            // Proje dosyası yoksa yalnızca aynı klasör
            gez(k, &mut dosyalar, 8);
        }
        dosyalar.sort();
        for d in dosyalar.into_iter().take(500) {
            let t = std::fs::canonicalize(&d).unwrap_or(d.clone());
            if t == tam {
                continue;
            }
            let icerik = match ortulu.get(&t) {
                Some(m) => m.clone(),
                None => match std::fs::read_to_string(&d) {
                    Ok(m) => m,
                    Err(_) => continue,
                },
            };
            urler.push(yol_uri(&t));
            kaynaklar.push((t, icerik));
        }
        (kaynaklar, urler)
    }

    fn konum(p: &Value, metin: &str) -> (usize, usize) {
        let satir_no = p["position"]["line"].as_u64().unwrap_or(0) as usize;
        let satir = metin.lines().nth(satir_no).unwrap_or("");
        let sutun = karakter_sutun(
            satir,
            p["position"]["character"].as_u64().unwrap_or(0) as usize,
        );
        (satir_no, sutun)
    }

    fn basvurular(&self, p: &Value) -> Value {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let (kaynaklar, urler) = self.proje_kaynaklari(uri);
        let Some((_, metin)) = kaynaklar.first() else {
            return Value::Null;
        };
        let (satir, sutun) = Self::konum(p, metin);
        let Some((_, yerler)) = crate::referans::bul(&kaynaklar, satir, sutun) else {
            return Value::Null;
        };
        let tanim_dahil = p["context"]["includeDeclaration"].as_bool() != Some(false);
        Value::Array(
            yerler
                .iter()
                .filter(|y| tanim_dahil || !y.tanim)
                .map(|y| {
                    json!({ "uri": urler[y.dosya], "range": aralik(&kaynaklar[y.dosya].1, y.satir, y.bas, y.son) })
                })
                .collect(),
        )
    }

    fn adlandirma_hazirla(&self, p: &Value) -> Value {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let (kaynaklar, _) = self.proje_kaynaklari(uri);
        let Some((_, metin)) = kaynaklar.first() else {
            return Value::Null;
        };
        let (satir, sutun) = Self::konum(p, metin);
        let Some((ad, yerler)) = crate::referans::bul(&kaynaklar, satir, sutun) else {
            return Value::Null;
        };
        match yerler
            .iter()
            .find(|y| y.dosya == 0 && y.satir == satir && y.bas <= sutun && sutun <= y.son)
        {
            Some(y) => json!({ "range": aralik(metin, y.satir, y.bas, y.son), "placeholder": ad }),
            None => Value::Null,
        }
    }

    fn yeniden_adlandir(&self, p: &Value) -> Result<Value, String> {
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        let yeni = p["newName"].as_str().unwrap_or("").trim();
        crate::referans::gecerli_ad(yeni)?;
        let (kaynaklar, urler) = self.proje_kaynaklari(uri);
        let Some((_, metin)) = kaynaklar.first() else {
            return Ok(Value::Null);
        };
        let (satir, sutun) = Self::konum(p, metin);
        let (_, yerler) = crate::referans::bul(&kaynaklar, satir, sutun)
            .ok_or("Burada yeniden adlandırılabilecek bir isim yok.")?;
        let mut degisiklikler: serde_json::Map<String, Value> = serde_json::Map::new();
        for (dosya, satir, bas, son, yeni_metin) in crate::referans::yeniden_adlandir(&yerler, yeni)
        {
            let e = degisiklikler
                .entry(urler[dosya].clone())
                .or_insert_with(|| json!([]));
            e.as_array_mut().unwrap().push(json!({
                "range": aralik(&kaynaklar[dosya].1, satir, bas, son),
                "newText": yeni_metin,
            }));
        }
        Ok(json!({ "changes": degisiklikler }))
    }

    fn calisma_alani_simgeleri(&self, p: &Value) -> Value {
        let sorgu = p["query"].as_str().unwrap_or("").to_lowercase();
        let Some(uri) = self.belgeler.keys().next() else {
            return json!([]);
        };
        let (kaynaklar, urler) = self.proje_kaynaklari(uri);
        let mut l = Vec::new();
        for (i, (_, m)) in kaynaklar.iter().enumerate() {
            for t in tanimlar(m) {
                if t.tur == "değişken" || !t.ad.to_lowercase().contains(&sorgu) {
                    continue;
                }
                let tur = match t.tur {
                    "işlev" => 12,
                    "fiil" => 6,
                    "sabit" => 14,
                    "model" => 23,
                    "seçenek" => 10,
                    "yol" => 7,
                    "bileşen" => 5,
                    _ => 13,
                };
                l.push(json!({
                    "name": t.ad, "kind": tur, "containerName": t.tur,
                    "location": { "uri": urler[i], "range": aralik(m, t.satir, t.bas, t.bas + t.ad.chars().count()) },
                }));
            }
        }
        Value::Array(l)
    }

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
            let yer = aralik(
                &metin,
                satir,
                u.konum.sutun - 1,
                u.konum.sutun - 1 + u.uzunluk,
            );
            tanilar.get_mut(uri).unwrap().push(json!({
                "range": yer,
                "severity": 2,
                "source": "orhunca",
                "message": u.mesaj,
                // Hızlı düzelt (textDocument/codeAction) için
                "data": { "duzeltme": {
                    "range": yer, "yeni": u.duzeltme, "baslik": format!("'{}' yaz", u.duzeltme),
                } },
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
                    let mut tani = json!({
                        "range": aralik(&kaynak, satir, bas, uzunluk.max(bas + 1)),
                        "severity": 1,
                        "source": "orhunca",
                        "message": mesaj,
                    });
                    if let Some(d) = &t.duzeltme {
                        let b = d.sutun - 1;
                        tani["data"] = json!({ "duzeltme": {
                            "range": aralik(&kaynak, d.satir - 1, b, b + d.uzunluk),
                            "yeni": d.yeni,
                            "baslik": d.baslik,
                        } });
                    }
                    tanilar.entry(hedef).or_default().push(tani);
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
        } else if let Some(o) = arayuz::oge(&kelime) {
            let olaylar = if o.olaylar.is_empty() {
                String::new()
            } else {
                format!("\n\nolaylar: {}", o.olaylar.join(", "))
            };
            format!(
                "```orhunca\n{}\n```\n{} (arayüz öğesi){olaylar}",
                o.ornek, o.aciklama
            )
        } else if let Some((_, a)) = arayuz::secenek(&kelime) {
            format!("**{kelime}** (arayüz seçeneği) — {a}")
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
            ("durum", "durum ${1:sayaç} = ${2:0}"),
            (
                "seçenek",
                "seçenek ${1:Renk}: ${2:kırmızı}, ${3:yeşil}, ${4:mavi}",
            ),
            ("dene … yakala", "dene:\n    $1\nyakala ${2:hata}:\n    $0"),
            ("arayüz", "arayüz:\n    başlık(\"${1:Başlık}\")\n    $0"),
            (
                "bileşen",
                "bileşen ${1:Kart}(${2:başlık}: ${3:metin}):\n    kart:\n        $0",
            ),
            (
                "düğme … tıklanınca",
                "düğme(\"${1:Tamam}\") tıklanınca:\n    $0",
            ),
            ("satır (yan yana)", "satır:\n    $0"),
            ("sütun (alt alta)", "sütun:\n    $0"),
        ];
        for (ad, govde) in parcaciklar {
            ogeler.push(
                json!({ "label": ad, "kind": 15, "insertText": govde, "insertTextFormat": 2 }),
            );
        }
        for o in arayuz::OGELER {
            let ekle = if o.kapsayici && o.degerler.is_empty() {
                format!("{}:\n    $0", o.ad)
            } else {
                format!("{}($1)", o.ad)
            };
            ogeler.push(json!({
                "label": o.ad, "kind": 10, "detail": o.ornek,
                "documentation": format!("{} (arayüz öğesi)", o.aciklama),
                "insertText": ekle, "insertTextFormat": 2,
            }));
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
                    "seçenek" => 13,
                    "bileşen" => 7,
                    "yol" | "arayüz" => continue,
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
        // Değişken ve parametreler kapsamına göre çözülür (işlev içi → parametre, yerel atama).
        if let Some((s, b)) = kapsamli_tanim(metin, satir_no, &kelime) {
            let son = b + kelime.chars().count();
            return json!({ "uri": uri, "range": aralik(metin, s, b, son) });
        }
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
                    "seçenek" => 10,
                    "yol" => 7,
                    "bileşen" => 5,
                    "arayüz" => 2,
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
                        "referencesProvider": true,
                        "renameProvider": { "prepareProvider": true },
                        "workspaceSymbolProvider": true,
                        "documentSymbolProvider": true,
                        "documentFormattingProvider": true,
                        "codeActionProvider": { "codeActionKinds": ["quickfix"] },
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
            "textDocument/codeAction" => yanitla(kimlik.as_ref().unwrap(), hizli_duzeltmeler(p)),
            "textDocument/references" => yanitla(kimlik.as_ref().unwrap(), s.basvurular(p)),
            "textDocument/prepareRename" => {
                yanitla(kimlik.as_ref().unwrap(), s.adlandirma_hazirla(p))
            }
            "textDocument/rename" => match s.yeniden_adlandir(p) {
                Ok(d) => yanitla(kimlik.as_ref().unwrap(), d),
                Err(e) => gonder(&json!({
                    "jsonrpc": "2.0", "id": kimlik,
                    "error": { "code": -32803, "message": e },
                })),
            },
            "workspace/symbol" => yanitla(kimlik.as_ref().unwrap(), s.calisma_alani_simgeleri(p)),
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
    fn arayuz_tanimlarini_bulur() {
        let t = tanimlar("durum sayaç = 0\ndurum ad: metin = \"\"\nbileşen Kart(b: metin):\n    yazı(b)\narayüz:\n    Kart(\"x\")\n");
        let adlar: Vec<_> = t.iter().map(|t| (t.tur, t.ad.as_str())).collect();
        assert_eq!(
            adlar,
            vec![
                ("durum", "sayaç"),
                ("durum", "ad"),
                ("bileşen", "Kart"),
                ("arayüz", "arayüz")
            ]
        );
    }

    #[test]
    fn tanima_git_kapsami_bilir() {
        // Hata raporu B22: parametre dıştaki aynı adlı değişkene, ikinci işlevin yereli
        // birincinin yereline gidiyordu.
        let m = "x = 1\nişlev f(x: sayı) -> sayı:\n    döndür x + 1\n\
                 işlev g():\n    y = 2\n    y'yi yaz.\nişlev h():\n    y = 3\n    y'yi yaz.\nx'i yaz.\n";
        assert_eq!(kapsamli_tanim(m, 2, "x"), Some((1, 8)));
        assert_eq!(kapsamli_tanim(m, 5, "y"), Some((4, 4)));
        assert_eq!(kapsamli_tanim(m, 8, "y"), Some((7, 4)));
        assert_eq!(kapsamli_tanim(m, 9, "x"), Some((0, 0)));
        // İşlevin kendi adı parametre değildir; fiil parametresi bulunur.
        let m = "fiil sayı'yı karele:\n    döndür sayı * sayı\n";
        assert_eq!(kapsamli_tanim(m, 1, "sayı"), Some((0, 5)));
        let m = "işlev f(a):\n    döndür a\nf(2)'yi yaz.\n";
        assert_eq!(kapsamli_tanim(m, 2, "f"), None);
        // Döngü değişkeni
        let m = "her i için 1'den 3'e kadar:\n    i'yi yaz.\n";
        assert_eq!(kapsamli_tanim(m, 1, "i"), Some((0, 4)));
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
