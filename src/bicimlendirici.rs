//! Biçimlendirici: girintiyi düzenler, gereksiz boşlukları siler ve kesme işaretli
//! eklerde ünlü uyumunu / ünsüz benzeşmesini düzeltir (`5'a` → `5'e`, `3'den` → `3'ten`).
//!
//! Derleyici ekleri hoşgörüyle kabul eder; doğru yazımı biçimlendirici ve dil
//! sunucusunun uyarıları sağlar. Sayılar okunuşlarına göre ek alır
//! (6 "altı" → `6'ya`, 10 "on" → `10'u`, 3.5 "üç virgül beş" → `3.5'e`).

use crate::ekler::{hal_bul, Hal};
use crate::hata::Konum;
use crate::sozcuk::{sozcukle, Tok};

#[derive(Debug, Clone, PartialEq)]
pub struct Uyari {
    /// Ekin ilk harfinin konumu (kesme işaretinden sonraki)
    pub konum: Konum,
    /// Ekin karakter uzunluğu
    pub uzunluk: usize,
    pub mesaj: String,
    /// Ekin doğru yazımı
    pub duzeltme: String,
}

const UNLULER: &str = "aeıioöuü";
const KALIN: &str = "aıou";
const SERT: &str = "çfhkpsşt";

fn tr_kucuk(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        'â' | 'Â' => 'a',
        'î' | 'Î' => 'i',
        'û' | 'Û' => 'u',
        c => c.to_lowercase().next().unwrap_or(c),
    }
}

/// Bir okunuşun ek almayı belirleyen özellikleri.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Ses {
    son_unlu: char,
    unluyle_biter: bool,
    sert_biter: bool,
}

/// Sondaki ünlüleri kalın okunsa da ince ek alan alıntı kelimeler (saat → saati).
const INCE_ISTISNALAR: &[&str] = &[
    "saat", "harf", "hal", "rol", "kontrol", "alkol", "gol", "sembol", "petrol", "dikkat", "hayal",
    "kabul", "ihtimal", "protokol", "terminal", "sinyal", "festival", "misal", "meal", "istiklal",
    "kemal", "metal", "lokal", "global", "total", "normal", "orijinal", "sosyal", "beyaz",
];

fn sesi_bul(kelime: &str) -> Option<Ses> {
    let k: String = kelime.chars().map(tr_kucuk).collect();
    let son = k.chars().last()?;
    let mut son_unlu = k.chars().rev().find(|c| UNLULER.contains(*c))?;
    if INCE_ISTISNALAR.contains(&k.as_str()) {
        son_unlu = match son_unlu {
            'a' => 'e',
            'ı' => 'i',
            'o' => 'ö',
            'u' => 'ü',
            c => c,
        };
    }
    Some(Ses {
        son_unlu,
        unluyle_biter: UNLULER.contains(son),
        sert_biter: SERT.contains(son),
    })
}

/// Sayının Türkçe okunuşunun son kelimesi: 6 → "altı", 40 → "kırk", 1000 → "bin".
fn sayinin_son_kelimesi(n: u128) -> &'static str {
    const BIRLER: [&str; 10] = [
        "", "bir", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz",
    ];
    const ONLAR: [&str; 10] = [
        "", "on", "yirmi", "otuz", "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan",
    ];
    if n == 0 {
        return "sıfır";
    }
    if !n.is_multiple_of(10) {
        return BIRLER[(n % 10) as usize];
    }
    if !n.is_multiple_of(100) {
        return ONLAR[((n / 10) % 10) as usize];
    }
    if !n.is_multiple_of(1000) {
        return "yüz";
    }
    let buyukler = [
        "bin",
        "milyon",
        "milyar",
        "trilyon",
        "katrilyon",
        "kentilyon",
    ];
    let mut m = n / 1000;
    for b in buyukler {
        if !m.is_multiple_of(1000) {
            return b;
        }
        m /= 1000;
    }
    "kentilyon"
}

/// Sayı yazımının (`42`, `1_000`, `3.14`) okunuşunun son kelimesi.
fn sayi_metni_okunusu(metin: &str) -> Option<&'static str> {
    let temiz: String = metin.chars().filter(|c| *c != '_').collect();
    let son_kisim = match temiz.split_once('.') {
        Some((_, kesir)) => kesir.to_string(),
        None => temiz,
    };
    let rakamlar = son_kisim.trim_start_matches('0');
    if rakamlar.is_empty() {
        return Some("sıfır");
    }
    if rakamlar.len() > 36 {
        return None;
    }
    rakamlar.parse::<u128>().ok().map(sayinin_son_kelimesi)
}

/// Kısaltmalar harf harf okunur: KDV → "ve", API → "i", PDF → "fe".
fn harf_adi(c: char) -> &'static str {
    match c {
        'A' => "a",
        'B' => "be",
        'C' => "ce",
        'Ç' => "çe",
        'D' => "de",
        'E' => "e",
        'F' => "fe",
        'G' => "ge",
        'Ğ' => "ge",
        'H' => "he",
        'I' => "ı",
        'İ' => "i",
        'J' => "je",
        'K' => "ke",
        'L' => "le",
        'M' => "me",
        'N' => "ne",
        'O' => "o",
        'Ö' => "ö",
        'P' => "pe",
        'Q' => "kü",
        'R' => "re",
        'S' => "se",
        'Ş' => "şe",
        'T' => "te",
        'U' => "u",
        'Ü' => "ü",
        'V' => "ve",
        'W' => "ve",
        'X' => "iks",
        'Y' => "ye",
        'Z' => "ze",
        _ => "",
    }
}

/// Bir ismin olası okunuşları. Kısa büyük harfli isimler hem kelime hem kısaltma
/// olarak okunabilir (SON, API); ikisi de kabul edilir.
fn isim_okunuslari(isim: &str) -> Vec<Ses> {
    // `liste_2` → "iki", `EN_KÜÇÜK` → "KÜÇÜK"
    let son_parca = isim.rsplit('_').find(|p| !p.is_empty()).unwrap_or(isim);
    let rakamlar: String = son_parca
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if !rakamlar.is_empty() {
        return sayi_metni_okunusu(&rakamlar)
            .and_then(sesi_bul)
            .into_iter()
            .collect();
    }
    let mut okunuslar = Vec::new();
    let buyuk_harfli = son_parca.chars().all(|c| !c.is_lowercase());
    let unlu_var = son_parca.chars().any(|c| UNLULER.contains(tr_kucuk(c)));
    if !(buyuk_harfli && !unlu_var) {
        okunuslar.extend(sesi_bul(son_parca));
    }
    if buyuk_harfli && (!unlu_var || son_parca.chars().count() <= 3) {
        if let Some(son) = son_parca.chars().last() {
            okunuslar.extend(sesi_bul(harf_adi(son)));
            // Latin I, İngilizce kısaltmalarda "i" okunur (API'ye).
            if son == 'I' {
                okunuslar.extend(sesi_bul("i"));
            }
        }
    }
    okunuslar
}

fn dortlu(v: char) -> char {
    match v {
        'a' | 'ı' => 'ı',
        'e' | 'i' => 'i',
        'o' | 'u' => 'u',
        _ => 'ü',
    }
}

fn ikili(v: char) -> char {
    if KALIN.contains(v) {
        'a'
    } else {
        'e'
    }
}

/// Bu okunuş ve hâl için kabul edilen ek biçimleri; ilki önerilen yazımdır.
fn dogru_ekler(ses: Ses, hal: Hal) -> Vec<String> {
    let i = dortlu(ses.son_unlu);
    let a = ikili(ses.son_unlu);
    let d = if ses.sert_biter { 't' } else { 'd' };
    let u = ses.unluyle_biter;
    match hal {
        Hal::Belirtme if u => vec![format!("y{i}"), format!("n{i}")],
        Hal::Belirtme => vec![format!("{i}")],
        Hal::Yonelme if u => vec![format!("y{a}"), format!("n{a}")],
        Hal::Yonelme => vec![format!("{a}")],
        Hal::Ayrilma if u => vec![format!("d{a}n"), format!("nd{a}n")],
        Hal::Ayrilma => vec![format!("{d}{a}n")],
        Hal::Bulunma if u => vec![format!("d{a}"), format!("nd{a}")],
        Hal::Bulunma => vec![format!("{d}{a}")],
        Hal::Vasita if u => vec![format!("yl{a}")],
        Hal::Vasita => vec![format!("l{a}")],
        Hal::Ilgi if u => vec![format!("n{i}n")],
        Hal::Ilgi => vec![format!("{i}n")],
    }
}

/// Ünlüler yok sayılarak karşılaştırma: yabancı kelimelerin okunuşu yazılışından
/// bilinemez ("Buzz"'ı), yalnızca tampon harf ve ünsüz benzeşmesi denetlenir.
fn unlusuz(ek: &str) -> String {
    ek.chars()
        .map(|c| if UNLULER.contains(c) { '*' } else { c })
        .collect()
}

/// Yazılan ek kabul edilmiyorsa önerilen düzeltme. Yazan kişinin tampon harfi
/// tercihi (`'nu` / `'yu`) korunur.
fn duzeltme_oner(okunuslar: &[Ses], hal: Hal, yazilan: &str, gevsek: bool) -> Option<String> {
    let yazilan_k: String = yazilan.chars().map(tr_kucuk).collect();
    let mut oneri = None;
    for ses in okunuslar {
        let ekler = dogru_ekler(*ses, hal);
        if ekler.contains(&yazilan_k)
            || (gevsek && ekler.iter().any(|e| unlusuz(e) == unlusuz(&yazilan_k)))
        {
            return None;
        }
        if oneri.is_none() {
            let n_tercihi = yazilan_k.starts_with('n');
            oneri = Some(
                ekler
                    .iter()
                    .find(|e| e.starts_with('n') == n_tercihi)
                    .unwrap_or(&ekler[0])
                    .clone(),
            );
        }
    }
    oneri
}

fn karakter_parcasi(satir: &str, bas: usize, son: usize) -> String {
    satir
        .chars()
        .skip(bas)
        .take(son.saturating_sub(bas))
        .collect()
}

/// Bir ifadenin (kaynak metni) alması gereken ek: `"merhaba"` + belirtme → `yı`.
/// Hata mesajlarındaki önerilerde kullanılır; bilinmiyorsa `i`.
pub fn ek_oner(ifade: &str, hal: Hal) -> String {
    let ifade = ifade.trim();
    let ses = if let Some(m) = ifade.strip_prefix('"').and_then(|m| m.strip_suffix('"')) {
        let son: String = m
            .chars()
            .rev()
            .skip_while(|c| !c.is_alphanumeric())
            .take_while(|c| c.is_alphanumeric())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if !son.is_empty() && son.chars().all(|c| c.is_ascii_digit()) {
            sayi_metni_okunusu(&son).and_then(sesi_bul)
        } else {
            isim_okunuslari(&son).into_iter().next()
        }
    } else if ifade.ends_with(')') || ifade.ends_with(']') {
        None
    } else if ifade.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        sayi_metni_okunusu(ifade).and_then(sesi_bul)
    } else {
        let son: String = ifade
            .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("")
            .to_string();
        isim_okunuslari(&son).into_iter().next()
    };
    match ses {
        Some(s) => dogru_ekler(s, hal).into_iter().next().unwrap_or_default(),
        None => dogru_ekler(
            Ses {
                son_unlu: 'i',
                unluyle_biter: false,
                sert_biter: false,
            },
            hal,
        )
        .into_iter()
        .next()
        .unwrap_or_default(),
    }
}

/// Kesme işaretli eklerdeki yazım uyarıları.
pub fn uyarilar(kaynak: &str) -> Vec<Uyari> {
    let Ok(sozcukler) = sozcukle(kaynak) else {
        return Vec::new();
    };
    let satirlar: Vec<&str> = kaynak.lines().collect();
    let mut cikti = Vec::new();
    // Eskiyen yazımlar (geriye uyumluluk: önce uyarı, `orhunca düzelt` ile çevrilir).
    for (konum, uzunluk, g) in crate::goc::bul(kaynak, crate::goc::GOCLER) {
        cikti.push(Uyari {
            konum,
            uzunluk,
            mesaj: format!(
                "'{}' {} sürümünde eskidi; yerine '{}' yazın (orhunca düzelt kendiliğinden çevirir)",
                g.eski, g.surum, g.yeni
            ),
            duzeltme: g.yeni.to_string(),
        });
    }
    // "5" + 3 → "53": sayı gibi görünen metin + ile birleştirilir, toplanmaz.
    for w in sozcukler.windows(3) {
        let (sol, op, sag) = (&w[0], &w[1], &w[2]);
        if op.tok != Tok::Op("+") {
            continue;
        }
        let sayi_gibi = |t: &Tok| matches!(t, Tok::Metin(m) if !m.trim().is_empty() && m.trim().parse::<f64>().is_ok());
        let sayi_ya_da_isim =
            |t: &Tok| matches!(t, Tok::Sayi(_) | Tok::Ondalik(_) | Tok::Kelime(_));
        let metin = if sayi_gibi(&sol.tok) && sayi_ya_da_isim(&sag.tok) {
            sol
        } else if sayi_gibi(&sag.tok) && sayi_ya_da_isim(&sol.tok) {
            sag
        } else {
            continue;
        };
        let Tok::Metin(m) = &metin.tok else { continue };
        let satir = satirlar.get(metin.konum.satir - 1).copied().unwrap_or("");
        let bas = metin.konum.sutun - 1;
        let karakterler: Vec<char> = satir.chars().collect();
        let mut son = bas + 1;
        while son < karakterler.len() && karakterler[son] != '"' {
            son += if karakterler[son] == '\\' { 2 } else { 1 };
        }
        if son >= karakterler.len() {
            continue;
        }
        let yazilan: String = karakterler[bas..=son].iter().collect();
        cikti.push(Uyari {
            konum: metin.konum,
            uzunluk: son + 1 - bas,
            mesaj: format!(
                "{yazilan} bir metin: + ile toplanmaz, yan yana eklenir (\"5\" + 3 → \"53\"); sayı olarak toplamak için: sayı({yazilan}) ya da {}",
                m.trim()
            ),
            duzeltme: yazilan,
        });
    }
    for w in sozcukler.windows(2) {
        let (onceki, simdiki) = (&w[0], &w[1]);
        let Tok::Ek(ek) = &simdiki.tok else { continue };
        let Some(hal) = hal_bul(ek) else { continue };
        let satir = satirlar.get(simdiki.konum.satir - 1).copied().unwrap_or("");
        let gevsek = matches!(onceki.tok, Tok::Metin(_));
        let (okunuslar, kok) = match &onceki.tok {
            Tok::Sayi(_) | Tok::Ondalik(_) => {
                let metin =
                    karakter_parcasi(satir, onceki.konum.sutun - 1, simdiki.konum.sutun - 1);
                let ses = sayi_metni_okunusu(&metin).and_then(sesi_bul);
                (ses.into_iter().collect::<Vec<_>>(), metin)
            }
            Tok::Metin(m) => {
                // Metnin son kelimesine göre: "Merhaba"'yı
                let son_kelime: String = m
                    .chars()
                    .rev()
                    .skip_while(|c| !c.is_alphanumeric())
                    .take_while(|c| c.is_alphanumeric())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let ses =
                    if son_kelime.chars().all(|c| c.is_ascii_digit()) && !son_kelime.is_empty() {
                        sayi_metni_okunusu(&son_kelime)
                            .and_then(sesi_bul)
                            .into_iter()
                            .collect()
                    } else {
                        isim_okunuslari(&son_kelime)
                    };
                (ses, format!("\"{m}\""))
            }
            Tok::Kelime(k) => (isim_okunuslari(k), k.clone()),
            _ => continue,
        };
        if okunuslar.is_empty() {
            continue;
        }
        if let Some(dogru) = duzeltme_oner(&okunuslar, hal, ek, gevsek) {
            let tur = if ek
                .chars()
                .map(tr_kucuk)
                .filter(|c| "dt".contains(*c))
                .ne(dogru.chars().filter(|c| "dt".contains(*c)))
            {
                "ünsüz benzeşmesi"
            } else {
                "ünlü uyumu"
            };
            cikti.push(Uyari {
                konum: Konum {
                    satir: simdiki.konum.satir,
                    sutun: simdiki.konum.sutun + 1,
                    dosya: 0,
                },
                uzunluk: ek.chars().count(),
                mesaj: format!("{tur}: {kok}'{ek} yerine {kok}'{dogru} yazılmalı"),
                duzeltme: dogru,
            });
        }
    }
    cikti
}

/// Satırın girinti dışındaki kısmı açık bir parantezin içinde mi? (devam satırı)
/// Metin ve yorumlar atlanarak parantez derinliği hesaplanır.
fn parantez_derinlikleri(kaynak: &str) -> Vec<i32> {
    let mut derinlik = 0i32;
    let mut sonuc = Vec::new();
    for satir in kaynak.lines() {
        sonuc.push(derinlik);
        let mut metinde = false;
        let mut kacis = false;
        for c in satir.chars() {
            if metinde {
                if kacis {
                    kacis = false;
                } else if c == '\\' {
                    kacis = true;
                } else if c == '"' {
                    metinde = false;
                }
                continue;
            }
            match c {
                '"' => metinde = true,
                '#' => break,
                '(' | '[' | '{' => derinlik += 1,
                ')' | ']' | '}' => derinlik = (derinlik - 1).max(0),
                _ => {}
            }
        }
    }
    sonuc
}

/// Kaynağı biçimlendirir. Sözdizimi hatalı olsa da güvenli düzenlemeler yapılır.
pub fn bicimlendir(kaynak: &str) -> String {
    // 1. Ek düzeltmeleri (sütunlar değişmeden önce)
    let mut satirlar: Vec<String> = kaynak.lines().map(str::to_string).collect();
    let mut duzeltmeler = uyarilar(kaynak);
    // Aynı satırda sağdan sola uygulanır ki sütunlar kaymasın.
    duzeltmeler.sort_by_key(|u| std::cmp::Reverse((u.konum.satir, u.konum.sutun)));
    for u in duzeltmeler {
        if let Some(s) = satirlar.get_mut(u.konum.satir - 1) {
            let mut k: Vec<char> = s.chars().collect();
            let bas = u.konum.sutun - 1;
            if bas + u.uzunluk <= k.len() {
                k.splice(bas..bas + u.uzunluk, u.duzeltme.chars());
                *s = k.into_iter().collect();
            }
        }
    }
    let ara = satirlar.join("\n");

    // 2. Girinti: her blok düzeyi 4 boşluk; parantez içindeki devam satırlarına dokunulmaz.
    let derinlikler = parantez_derinlikleri(&ara);
    let mut cikti: Vec<String> = Vec::new();
    let mut yigin: Vec<usize> = vec![0];
    for (i, satir) in ara.lines().enumerate() {
        let satir = satir.trim_end();
        let govde = satir.trim_start_matches([' ', '\t']);
        if govde.is_empty() {
            cikti.push(String::new());
            continue;
        }
        let girinti: usize = satir[..satir.len() - govde.len()]
            .chars()
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum();
        if derinlikler[i] > 0 {
            // Devam satırı: yalnızca sekmeler boşluğa çevrilir.
            cikti.push(format!("{}{}", " ".repeat(girinti), govde));
            continue;
        }
        if govde.starts_with('#') {
            // Yorumun düzeyi kendi girintisinden: bilinen bir düzeyse o, en derin düzeyden
            // de derinse (ör. yeni açılan bloğun ilk satırı) bir içerisi, arada kalıyorsa
            // bir üstteki düzey. Yığın değişmez; yorum blok yapısını etkilemez.
            let duzey = if let Some(i) = yigin.iter().position(|g| *g == girinti) {
                i
            } else if girinti > *yigin.last().unwrap() {
                yigin.len()
            } else {
                yigin
                    .iter()
                    .filter(|g| **g < girinti)
                    .count()
                    .saturating_sub(1)
            };
            cikti.push(format!("{}{}", "    ".repeat(duzey), govde));
            continue;
        }
        if girinti > *yigin.last().unwrap() {
            yigin.push(girinti);
        } else {
            while yigin.len() > 1 && girinti < *yigin.last().unwrap() {
                yigin.pop();
            }
        }
        let duzey = yigin.len() - 1;
        cikti.push(format!("{}{}", "    ".repeat(duzey), govde));
    }

    // 3. Boş satırlar: baştakiler silinir, art arda en fazla iki, sonda tek satır sonu.
    let mut sonuc = String::new();
    let mut bos = 0;
    let mut basladi = false;
    for s in cikti {
        if s.is_empty() {
            if basladi {
                bos += 1;
            }
            continue;
        }
        if basladi {
            for _ in 0..bos.min(2) {
                sonuc.push('\n');
            }
        }
        bos = 0;
        basladi = true;
        sonuc.push_str(&s);
        sonuc.push('\n');
    }
    sonuc
}

#[cfg(test)]
mod testler {
    use super::*;

    fn uyari_var(k: &str) -> bool {
        !uyarilar(k).is_empty()
    }

    #[test]
    fn blogun_ilk_satirindaki_yorum_girintisini_korur() {
        // Hata #70: yeni açılan bloğun ilk satırı yorumsa başlığın düzeyine kayıyordu.
        let k = "eğer a ise:\n    eğer b ise:\n        x = 1\n    değilse:\n        # ilk\n        x = 2\n\
                 arayüz:\n    düğme(\"A\") tıklanınca:\n        # olay\n        x = 3\n\
                 işlev f():\n    # gövde\n    döndür 1\n";
        assert_eq!(bicimlendir(k), k);
        // Bloğun sonundaki yorum yerinde kalır; sonraki koşula ait yorum onunla hizalanır.
        let k = "eğer a ise:\n    x = 1\n    # son not\n# değilse için\ndeğilse:\n    x = 2\n";
        assert_eq!(bicimlendir(k), k);
        // Sekme ve 2 boşluk 4 boşluğa çevrilir.
        assert_eq!(
            bicimlendir("eğer a ise:\n  # y\n  x = 1\n"),
            "eğer a ise:\n    # y\n    x = 1\n"
        );
    }

    #[test]
    fn sayilarin_okunusu() {
        assert_eq!(sayinin_son_kelimesi(6), "altı");
        assert_eq!(sayinin_son_kelimesi(40), "kırk");
        assert_eq!(sayinin_son_kelimesi(100), "yüz");
        assert_eq!(sayinin_son_kelimesi(2000), "bin");
        assert_eq!(sayinin_son_kelimesi(3_000_000), "milyon");
        assert_eq!(sayi_metni_okunusu("3.14"), Some("dört"));
        assert_eq!(sayi_metni_okunusu("2.0"), Some("sıfır"));
    }

    #[test]
    fn dogru_yazimlar_uyari_vermez() {
        for k in [
            "5'i yaz.",
            "6'yı yaz.",
            "10'u yaz.",
            "3'ten büyük",
            "x 4'ten büyükse",
            "x 4'e eşitse",
            "40'ı yaz.",
            "100'e ekle",
            "\"Merhaba\"'yı yaz.",
            "\"FizzBuzz\"'ı yaz.",
            "sayılar'ı sırala.",
            "kitap'ı yaz.",
            "API'yi yaz.",
            "KDV'yi yaz.",
            "EN_KÜÇÜK'ten büyük",
            "saat'i yaz.",
            "uzunluğu'nu yaz.",
            "x2'yi yaz.",
            "3.5'i yaz.",
            "ad'ı yaz.",
            "(a)'yı yaz.",
        ] {
            assert!(!uyari_var(k), "yanlış uyarı: {k} → {:?}", uyarilar(k));
        }
    }

    #[test]
    fn yanlis_yazimlar_duzeltilir() {
        assert_eq!(bicimlendir("5'a yaz.\n"), "5'e yaz.\n");
        assert_eq!(bicimlendir("x = 3'den\n"), "x = 3'ten\n");
        assert_eq!(bicimlendir("6'i yaz.\n"), "6'yı yaz.\n");
        // Metinlerde ünlü uyumu denetlenmez, tampon harf denetlenir.
        assert_eq!(bicimlendir("\"Merhaba\"'ı yaz.\n"), "\"Merhaba\"'yı yaz.\n");
        let u = uyarilar("x 4'dan büyükse");
        assert_eq!(u.len(), 1);
        assert!(u[0].mesaj.contains("4'ten"), "{}", u[0].mesaj);
    }

    #[test]
    fn girinti_ve_bosluklar() {
        let girdi = "\n\neğer x:\n  y = 1\n  eğer z:\n    t = 2   \n\n\n\n\nson = 3";
        let beklenen = "eğer x:\n    y = 1\n    eğer z:\n        t = 2\n\n\nson = 3\n";
        assert_eq!(bicimlendir(girdi), beklenen);
        // iki kez biçimlendirmek bir şey değiştirmez
        assert_eq!(bicimlendir(beklenen), beklenen);
    }

    #[test]
    fn devam_satirlari_korunur() {
        let girdi = "l = [\n        1,\n        2\n]\n";
        assert_eq!(bicimlendir(girdi), girdi);
    }
}
