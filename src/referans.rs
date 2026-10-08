//! Başvurular ve yeniden adlandırma (dil sunucusu ve Stüdyo).
//!
//! Sözcük çözümleyicinin çıktısı üzerinde çalışır: metinlerdeki ve yorumlardaki kelimeler
//! sayılmaz. Orhunca'da hâl ekleri isme bitişik yazılabildiği için (`sayılara` = `sayılar` +
//! yönelme) her kelime tanımlı isimlerle çözümlenir. Yeniden adlandırmada ek, yeni adın
//! ünlü uyumuna göre yeniden seçilir (`toplam'ı` → `sonuç'u`).
//!
//! Kapsam: işlev, fiil, model, sabit, seçenek, bileşen ve `durum` bütün projede geçerlidir
//! (aynı adı kendi içinde tanımlayan işlevler hariç); değişken ve parametreler tanımlandıkları
//! işlevin (fiilin, web yolunun) içinde, üst düzey değişkenler yalnızca o dosyanın blok
//! dışındaki kodunda.

use crate::bicimlendirici::ek_oner;
use crate::dil_sunucusu::{girinti, kapsam_basligi, tanimlar, ANAHTAR_KELIMELER};
use crate::ekler::{Cozum, Hal, Sozluk};
use crate::sozcuk::{sozcukle, Tok};
use std::path::PathBuf;

/// Kaynakta bir başvuru. Satır ve sütunlar 0'dan başlar; sütunlar karakter sırasıdır.
#[derive(Debug, Clone, PartialEq)]
pub struct Yer {
    pub dosya: usize,
    pub satir: usize,
    pub bas: usize,
    pub son: usize,
    /// Bitişik ek (`sayılara`): adın yerine `yeni + ek` yazılır.
    pub bitisik: Option<Hal>,
    /// Kesme işaretinden sonraki ek (`toplam'ı`): (ekin başlangıç sütunu, uzunluk, hâl).
    pub kesmeli: Option<(usize, usize, Hal)>,
    /// Tanımın kendisi mi
    pub tanim: bool,
}

/// Değişiklik: (dosya sırası, satır, baş, son, yeni metin)
pub type Degisiklik = (usize, usize, usize, usize, String);

const KURESEL: &[&str] = &[
    "işlev", "fiil", "sabit", "model", "seçenek", "bileşen", "durum",
];

/// Üst düzey kapsam blokları: (başlık satırı, bitiş satırı hariç).
fn bloklar(metin: &str) -> Vec<(usize, usize)> {
    let satirlar: Vec<&str> = metin.lines().collect();
    let dolu = |s: &str| {
        let t = s.trim();
        !t.is_empty() && !t.starts_with('#')
    };
    let mut l = Vec::new();
    let mut i = 0;
    while i < satirlar.len() {
        let s = satirlar[i];
        if dolu(s) && girinti(s) == 0 && kapsam_basligi(s.trim_start()) {
            let son = (i + 1..satirlar.len())
                .find(|&j| dolu(satirlar[j]) && girinti(satirlar[j]) == 0)
                .unwrap_or(satirlar.len());
            l.push((i, son));
            i = son;
        } else {
            i += 1;
        }
    }
    l
}

/// `model` blokları: (başlık, son)
fn model_bloklari(metin: &str) -> Vec<(usize, usize)> {
    let satirlar: Vec<&str> = metin.lines().collect();
    let dolu = |s: &str| {
        let t = s.trim();
        !t.is_empty() && !t.starts_with('#')
    };
    (0..satirlar.len())
        .filter(|&i| girinti(satirlar[i]) == 0 && satirlar[i].starts_with("model "))
        .map(|i| {
            let son = (i + 1..satirlar.len())
                .find(|&j| dolu(satirlar[j]) && girinti(satirlar[j]) == 0)
                .unwrap_or(satirlar.len());
            (i, son)
        })
        .collect()
}

/// Başlıktaki parametre adları: `işlev topla(a: sayı, b)` → a, b; `fiil x'i karele` → x.
fn parametreler(baslik: &str) -> Vec<String> {
    let b = baslik.trim();
    let govde = if let Some(r) = b
        .strip_prefix("işlev ")
        .or_else(|| b.strip_prefix("bileşen "))
    {
        match (r.find('('), r.rfind(')')) {
            (Some(a), Some(k)) if a < k => r[a + 1..k].to_string(),
            _ => return vec![],
        }
    } else if let Some(r) = b.strip_prefix("fiil ") {
        // Son kelime fiilin adıdır.
        let r = r.trim_end_matches(':');
        let r = r.split("->").next().unwrap_or(r);
        let mut k: Vec<&str> = r.split_whitespace().collect();
        k.pop();
        k.join(" ")
    } else if let Some((_, kalip)) = b.split_once('"') {
        // Web yolu: "/ürünler/{kimlik: sayı}"
        kalip
            .split('{')
            .skip(1)
            .filter_map(|p| p.split(['}', ':']).next())
            .collect::<Vec<_>>()
            .join(",")
    } else {
        return vec![];
    };
    govde
        .split([',', ' '])
        .map(|p| {
            p.split([':', '\'', '’'])
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches('(')
                .to_string()
        })
        .filter(|p| !p.is_empty() && p.chars().next().is_some_and(char::is_alphabetic))
        .collect()
}

/// Dosyadaki tanımlı isimler (tanımlar, parametreler, döngü değişkenleri).
fn isimler(metin: &str) -> Vec<String> {
    let mut l: Vec<String> = tanimlar(metin)
        .into_iter()
        .filter(|t| !matches!(t.tur, "yol" | "arayüz"))
        .map(|t| t.ad)
        .collect();
    for s in metin.lines() {
        let g = s.trim_start();
        if kapsam_basligi(g) {
            l.extend(parametreler(g));
        }
        if let Some((ad, _)) = g.split_once('=') {
            let ad = ad.split(':').next().unwrap_or(ad).trim();
            if !ad.is_empty() && !ad.contains([' ', '[', '(', '.', '!', '<', '>']) {
                l.push(ad.trim_end_matches(['+', '-']).to_string());
            }
        }
    }
    l.sort();
    l.dedup();
    l
}

fn sozluk(adlar: &[String]) -> Sozluk {
    let mut s = Sozluk::default();
    for a in adlar {
        s.ekle(a);
    }
    s
}

/// Kelimenin işaret ettiği isim (ek varsa çözülür).
fn cozumle(s: &Sozluk, adlar: &[String], kelime: &str) -> Option<(String, Option<Hal>)> {
    if adlar.iter().any(|a| a == kelime) {
        return Some((kelime.to_string(), None));
    }
    match s.cozumle(kelime) {
        Cozum::EkliIsim(ad, h) => Some((ad, Some(h))),
        Cozum::Isim(ad) => Some((ad, None)),
        _ => None,
    }
}

/// Satırın bloğu: (başlık, son) ya da üst düzey ise None.
fn blogu(bloklar: &[(usize, usize)], satir: usize) -> Option<(usize, usize)> {
    bloklar
        .iter()
        .copied()
        .find(|(b, s)| (*b..*s).contains(&satir))
}

/// Blok `ad`ı kendisi tanımlıyor mu (parametre, atama ya da döngü değişkeni)?
fn blokta_tanimli(satirlar: &[&str], blok: (usize, usize), ad: &str) -> bool {
    if parametreler(satirlar[blok.0]).iter().any(|p| p == ad) {
        return true;
    }
    satirlar[blok.0 + 1..blok.1].iter().any(|s| {
        let g = s.trim_start();
        if let Some(r) = g.strip_prefix("her ") {
            return r.split(' ').next() == Some(ad);
        }
        g.strip_prefix(ad).is_some_and(|r| {
            let r = r.trim_start();
            (r.starts_with('=') && !r.starts_with("=="))
                || (r.starts_with(':') && r.contains('=') && !r.contains("=="))
                || r.starts_with("+=")
                || r.starts_with("-=")
        })
    })
}

/// İmlecin üzerindeki ismin bütün başvuruları. `kaynaklar[0]` imlecin dosyasıdır; ötekiler
/// projenin diğer dosyalarıdır. Yerleşik işlev, anahtar kelime ya da tanımsız isimde None.
pub fn bul(
    kaynaklar: &[(PathBuf, String)],
    satir: usize,
    sutun: usize,
) -> Option<(String, Vec<Yer>)> {
    let metin = &kaynaklar.first()?.1;
    let sozcukler = sozcukle(metin).ok()?;
    // İmlecin üzerindeki kelime (sütunlar 1'den başlar)
    let w = sozcukler.iter().find(|w| {
        matches!(&w.tok, Tok::Kelime(k) if w.konum.satir == satir + 1
            && w.konum.sutun <= sutun + 1
            && sutun < w.konum.sutun + k.chars().count())
    })?;
    let Tok::Kelime(kelime) = &w.tok else {
        return None;
    };
    // Bütün dosyalarda tanımlı küresel isimler (işlev, fiil, model...): başka dosyadan
    // çağrılanlar da çözülebilsin.
    let kureseller: Vec<String> = kaynaklar
        .iter()
        .flat_map(|(_, m)| tanimlar(m))
        .filter(|t| KURESEL.contains(&t.tur))
        .map(|t| t.ad)
        .collect();
    let birlesik = |mut l: Vec<String>| {
        l.extend(kureseller.iter().cloned());
        l.sort();
        l.dedup();
        l
    };
    let adlar = birlesik(isimler(metin));
    let (ad, _) = cozumle(&sozluk(&adlar), &adlar, kelime)?;
    if ANAHTAR_KELIMELER.iter().any(|(k, _)| *k == ad) {
        return None;
    }
    let satirlar: Vec<&str> = metin.lines().collect();
    let b = bloklar(metin);
    // Kapsam
    let kuresel_mi = |m: &str| {
        tanimlar(m)
            .iter()
            .any(|t| t.ad == ad && KURESEL.contains(&t.tur))
    };
    let yerel_blok = blogu(&b, satir).filter(|bl| blokta_tanimli(&satirlar, *bl, &ad));
    let kuresel = yerel_blok.is_none() && kaynaklar.iter().any(|(_, m)| kuresel_mi(m));
    if yerel_blok.is_none() && !kuresel && !adlar.contains(&ad) {
        return None;
    }
    let mut yerler = Vec::new();
    for (sira, (_, m)) in kaynaklar.iter().enumerate() {
        if !kuresel && sira > 0 {
            break;
        }
        let Ok(s) = sozcukle(m) else { continue };
        let adlar_m = birlesik(isimler(m));
        let sozluk_m = sozluk(&adlar_m);
        let satirlar_m: Vec<&str> = m.lines().collect();
        let b_m = bloklar(m);
        let modeller = model_bloklari(m);
        let tanim_yeri = tanimlar(m)
            .into_iter()
            .find(|t| t.ad == ad)
            .map(|t| (t.satir, t.bas));
        let mut parantez = 0i32;
        for (i, w) in s.iter().enumerate() {
            if let Tok::Op(o) = &w.tok {
                match *o {
                    "(" => parantez += 1,
                    ")" => parantez -= 1,
                    _ => {}
                }
                continue;
            }
            if matches!(w.tok, Tok::YeniSatir) {
                parantez = 0;
            }
            let Tok::Kelime(k) = &w.tok else { continue };
            let sat = w.konum.satir - 1;
            // Kapsam denetimi
            let bl = blogu(&b_m, sat);
            let gecerli = match (yerel_blok, bl) {
                (Some(y), Some(x)) => sira == 0 && x == y,
                (Some(_), None) => false,
                (None, Some(x)) => kuresel && !blokta_tanimli(&satirlar_m, x, &ad),
                (None, None) => true,
            };
            if !gecerli {
                continue;
            }
            // Alan ve yöntem adları (`ürün.ad`) ile adlandırılmış argümanlar (`Ürün(ad: ...)`)
            if i > 0 && matches!(s[i - 1].tok, Tok::Uye) {
                continue;
            }
            let sonraki_iki_nokta = matches!(s.get(i + 1).map(|x| &x.tok), Some(Tok::Op(":")));
            // Model alanlarının tanımı (`    ad: metin`)
            if sonraki_iki_nokta && modeller.iter().any(|(b, e)| sat > *b && sat < *e) {
                continue;
            }
            let baslik = satirlar_m
                .get(sat)
                .is_some_and(|l| kapsam_basligi(l.trim_start()));
            if !baslik && parantez > 0 && sonraki_iki_nokta {
                continue;
            }
            let Some((bulunan, hal)) = cozumle(&sozluk_m, &adlar_m, k) else {
                continue;
            };
            if bulunan != ad {
                continue;
            }
            let bas = w.konum.sutun - 1;
            let kesmeli = match (hal, s.get(i + 1)) {
                (None, Some(x)) => match &x.tok {
                    Tok::Ek(e) => {
                        crate::ekler::hal_bul(e).map(|h| (x.konum.sutun, e.chars().count(), h))
                    }
                    _ => None,
                },
                _ => None,
            };
            yerler.push(Yer {
                dosya: sira,
                satir: sat,
                bas,
                son: bas + k.chars().count(),
                bitisik: hal,
                kesmeli,
                tanim: tanim_yeri == Some((sat, bas)),
            });
        }
    }
    Some((ad, yerler))
}

/// Dosya ve projesindeki öteki .ohc dosyaları diskten (ilk öğe dosyanın kendisi).
pub fn diskten(dosya: &std::path::Path) -> Vec<(PathBuf, String)> {
    fn gez(k: &std::path::Path, l: &mut Vec<PathBuf>, derinlik: usize) {
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
    let tam = std::fs::canonicalize(dosya).unwrap_or(dosya.to_path_buf());
    let mut l = vec![(
        tam.clone(),
        std::fs::read_to_string(dosya).unwrap_or_default(),
    )];
    let kok = crate::derleme::proje_koku(dosya);
    let mut dosyalar = Vec::new();
    gez(&kok, &mut dosyalar, 0);
    dosyalar.sort();
    for d in dosyalar.into_iter().take(500) {
        let t = std::fs::canonicalize(&d).unwrap_or(d);
        if t != tam {
            if let Ok(m) = std::fs::read_to_string(&t) {
                l.push((t, m));
            }
        }
    }
    l
}

/// Yeni ad geçerli bir isim mi?
pub fn gecerli_ad(yeni: &str) -> Result<(), String> {
    let s = sozcukle(yeni).map_err(|_| format!("'{yeni}' bir isim olamaz"))?;
    let kelimeler: Vec<_> = s
        .iter()
        .filter(|w| !matches!(w.tok, Tok::YeniSatir | Tok::Son))
        .collect();
    match kelimeler.as_slice() {
        [w] if matches!(&w.tok, Tok::Kelime(k) if k == yeni) => {}
        _ => {
            return Err(format!(
                "'{yeni}' bir isim olamaz (harf, rakam ve _; boşluksuz)"
            ))
        }
    }
    if ANAHTAR_KELIMELER.iter().any(|(k, _)| *k == yeni) || crate::yerlesik::bul(yeni).is_some() {
        return Err(format!("'{yeni}' Orhunca'da ayrılmış bir kelime"));
    }
    Ok(())
}

/// Yeniden adlandırma değişiklikleri.
pub fn yeniden_adlandir(yerler: &[Yer], yeni: &str) -> Vec<Degisiklik> {
    let mut l = Vec::new();
    for y in yerler {
        match y.bitisik {
            Some(h) => l.push((
                y.dosya,
                y.satir,
                y.bas,
                y.son,
                format!("{yeni}{}", ek_oner(yeni, h)),
            )),
            None => {
                l.push((y.dosya, y.satir, y.bas, y.son, yeni.to_string()));
                if let Some((ek_bas, uzunluk, h)) = y.kesmeli {
                    let ek = ek_oner(yeni, h);
                    l.push((y.dosya, y.satir, ek_bas, ek_bas + uzunluk, ek));
                }
            }
        }
    }
    l
}

/// Değişiklikleri metne uygular (aynı dosyanın değişiklikleri).
pub fn uygula(metin: &str, degisiklikler: &[&Degisiklik]) -> String {
    let mut satirlar: Vec<Vec<char>> = metin.split('\n').map(|s| s.chars().collect()).collect();
    let mut sirali: Vec<&&Degisiklik> = degisiklikler.iter().collect();
    sirali.sort_by_key(|d| std::cmp::Reverse((d.1, d.2)));
    for d in sirali {
        if let Some(s) = satirlar.get_mut(d.1) {
            let son = d.3.min(s.len());
            let bas = d.2.min(son);
            s.splice(bas..son, d.4.chars());
        }
    }
    satirlar
        .into_iter()
        .map(|s| s.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    fn adlandir(kaynaklar: &[(&str, &str)], satir: usize, sutun: usize, yeni: &str) -> Vec<String> {
        let k: Vec<(PathBuf, String)> = kaynaklar
            .iter()
            .map(|(a, m)| (PathBuf::from(a), m.to_string()))
            .collect();
        let (_, yerler) = bul(&k, satir, sutun).expect("isim bulunamadı");
        let d = yeniden_adlandir(&yerler, yeni);
        k.iter()
            .enumerate()
            .map(|(i, (_, m))| uygula(m, &d.iter().filter(|x| x.0 == i).collect::<Vec<_>>()))
            .collect()
    }

    #[test]
    fn degisken_ve_ekler() {
        let m = "sayılar = [3, 1]\n5'i sayılara ekle.\nsayıların uzunluğunu yaz.\n\"sayılar\"'ı yaz.\n# sayılar\n";
        let s = adlandir(&[("a.ohc", m)], 0, 2, "notlar");
        assert_eq!(
            s[0],
            "notlar = [3, 1]\n5'i notlara ekle.\nnotların uzunluğunu yaz.\n\"sayılar\"'ı yaz.\n# sayılar\n"
        );
        let m = "toplam = 0\ntoplam'ı yaz.\n";
        assert_eq!(
            adlandir(&[("a.ohc", m)], 1, 1, "sonuç")[0],
            "sonuç = 0\nsonuç'u yaz.\n"
        );
    }

    #[test]
    fn kapsam() {
        let m = "x = 1\nx'i yaz.\n\nişlev f(x: sayı) -> sayı:\n    y = x + 1\n    döndür y\n\nf(x)'i yaz.\n";
        // İşlevin parametresi yalnızca işlevin içinde
        let s = adlandir(&[("a.ohc", m)], 4, 8, "a");
        assert_eq!(
            s[0],
            "x = 1\nx'i yaz.\n\nişlev f(a: sayı) -> sayı:\n    y = a + 1\n    döndür y\n\nf(x)'i yaz.\n"
        );
        // Üst düzey değişken işlevin içine girmez
        let s = adlandir(&[("a.ohc", m)], 0, 0, "z");
        assert_eq!(
            s[0],
            "z = 1\nz'i yaz.\n\nişlev f(x: sayı) -> sayı:\n    y = x + 1\n    döndür y\n\nf(z)'i yaz.\n"
        );
    }

    #[test]
    fn dosyalar_arasi() {
        let a = "kullan \"b.ohc\"\ntopla(1, 2)'yi yaz.\n";
        let b = "işlev topla(a: sayı, b: sayı) -> sayı:\n    döndür a + b\n";
        let s = adlandir(&[("a.ohc", a), ("b.ohc", b)], 1, 2, "ekle_hepsi");
        assert_eq!(s[0], "kullan \"b.ohc\"\nekle_hepsi(1, 2)'yi yaz.\n");
        assert_eq!(
            s[1],
            "işlev ekle_hepsi(a: sayı, b: sayı) -> sayı:\n    döndür a + b\n"
        );
    }

    #[test]
    fn alanlar_ve_gecersiz_adlar() {
        let m = "model Ürün:\n    ad: metin\n\nad = \"x\"\nü = Ürün(ad: ad)\nü.ad'ı yaz.\n";
        let s = adlandir(&[("a.ohc", m)], 3, 0, "isim");
        assert_eq!(
            s[0],
            "model Ürün:\n    ad: metin\n\nisim = \"x\"\nü = Ürün(ad: isim)\nü.ad'ı yaz.\n"
        );
        assert!(gecerli_ad("yeni_ad").is_ok());
        assert!(gecerli_ad("eğer").is_err());
        assert!(gecerli_ad("uzunluk").is_err());
        assert!(gecerli_ad("iki kelime").is_err());
        let k = vec![(PathBuf::from("a.ohc"), "uzunluk([1])'i yaz.\n".to_string())];
        assert!(bul(&k, 0, 2).is_none());
    }
}
