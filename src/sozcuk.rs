//! Sözcük çözümleyici (lexer): kaynak metni sözcüklere ayırır.
//!
//! Girinti Python'daki gibi bloğu belirler; `Girinti`/`Cikinti` sözcükleri üretilir.
//! Kesme işaretinden sonra gelen ek (`5'i`, `x'den`) ayrı bir `Ek` sözcüğü olur.
//! Kesme işareti olmadan yazılan ekler (`sayılara`) burada çözülmez; tanımlı isimleri
//! bilen ayrıştırıcı onları çözer.

use crate::hata::{Hata, Konum, Sonuc};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Sayi(i64),
    Ondalik(f64),
    Metin(String),
    Kelime(String),
    /// Kesme işaretinden sonraki ek, ör. `'den` → `den`.
    Ek(String),
    Op(&'static str),
    /// Boşluksuz yazılmış üye noktası: `ürün.ad`, `Ürün.hepsi()`.
    Uye,
    YeniSatir,
    Girinti,
    Cikinti,
    Son,
}

#[derive(Debug, Clone)]
pub struct Sozcuk {
    pub tok: Tok,
    pub konum: Konum,
}

const OPLAR: [&str; 24] = [
    "->", "//", "==", "!=", "<=", ">=", "+=", "-=", "+", "-", "*", "/", "%", "=", "<", ">", "(",
    ")", "[", "]", "{", "}", ",", ":",
];

fn kesme_mi(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

fn kelime_basi(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn kelime_devami(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn sozcukle(kaynak: &str) -> Sonuc<Vec<Sozcuk>> {
    let mut cikti = Vec::new();
    let mut girintiler = vec![0usize];
    let mut parantez = 0i32;

    for (satir_i, satir) in kaynak.lines().enumerate() {
        let satir_no = satir_i + 1;
        let karakterler: Vec<char> = satir.chars().collect();

        // Girinti hesabı (sekme = 4 boşluk).
        let mut girinti = 0;
        let mut i = 0;
        while i < karakterler.len() && (karakterler[i] == ' ' || karakterler[i] == '\t') {
            girinti += if karakterler[i] == '\t' { 4 } else { 1 };
            i += 1;
        }
        // Boş ve yalnızca yorum içeren satırlar yok sayılır.
        if i >= karakterler.len() || karakterler[i] == '#' {
            continue;
        }

        if parantez == 0 {
            let konum = Konum {
                satir: satir_no,
                sutun: i + 1,
                dosya: 0,
            };
            let ust = *girintiler.last().unwrap();
            if girinti > ust {
                girintiler.push(girinti);
                cikti.push(Sozcuk {
                    tok: Tok::Girinti,
                    konum,
                });
            } else {
                while girinti < *girintiler.last().unwrap() {
                    girintiler.pop();
                    cikti.push(Sozcuk {
                        tok: Tok::Cikinti,
                        konum,
                    });
                }
                if girinti != *girintiler.last().unwrap() {
                    return Err(Hata::yeni(
                        konum,
                        "girinti önceki satırların hiçbiriyle uyuşmuyor",
                    )
                    .ipucu("bir bloktaki tüm satırlar aynı miktarda içeriden başlamalı"));
                }
            }
        }

        while i < karakterler.len() {
            let c = karakterler[i];
            let konum = Konum {
                satir: satir_no,
                sutun: i + 1,
                dosya: 0,
            };

            if c == ' ' || c == '\t' {
                i += 1;
                continue;
            }
            if c == '#' {
                break;
            }

            if c.is_ascii_digit() {
                let bas = i;
                while i < karakterler.len()
                    && (karakterler[i].is_ascii_digit() || karakterler[i] == '_')
                {
                    i += 1;
                }
                // Ondalık kısım: noktadan sonra rakam gelmeliyse (`3.14`). `5.` cümle sonudur.
                let mut ondalik = false;
                if i + 1 < karakterler.len()
                    && karakterler[i] == '.'
                    && karakterler[i + 1].is_ascii_digit()
                {
                    ondalik = true;
                    i += 1;
                    while i < karakterler.len()
                        && (karakterler[i].is_ascii_digit() || karakterler[i] == '_')
                    {
                        i += 1;
                    }
                }
                let metin: String = karakterler[bas..i].iter().filter(|c| **c != '_').collect();
                let tok =
                    if ondalik {
                        Tok::Ondalik(metin.parse().map_err(|_| {
                            Hata::yeni(konum, format!("'{metin}' geçerli bir ondalık sayı değil"))
                        })?)
                    } else {
                        Tok::Sayi(metin.parse().map_err(|_| {
                            Hata::yeni(konum, format!("'{metin}' sayısı çok büyük"))
                        })?)
                    };
                if i < karakterler.len() && kelime_basi(karakterler[i]) {
                    let ek: String = karakterler[i..]
                        .iter()
                        .take_while(|c| kelime_devami(**c))
                        .collect();
                    return Err(Hata::yeni(
                        konum,
                        format!("sayıdan sonra gelen '{ek}' ekinin önünde kesme işareti olmalı"),
                    )
                    .ipucu(format!("{metin}'{ek} şeklinde yazın")));
                }
                cikti.push(Sozcuk { tok, konum });
                i = ek_oku(&karakterler, i, satir_no, &mut cikti)?;
                continue;
            }

            if c == '"' {
                i += 1;
                let mut metin = String::new();
                loop {
                    if i >= karakterler.len() {
                        return Err(
                            Hata::yeni(konum, "metin kapatılmamış").ipucu("metnin sonuna \" koyun")
                        );
                    }
                    let d = karakterler[i];
                    i += 1;
                    match d {
                        '"' => break,
                        '\\' => {
                            let e = karakterler.get(i).copied().unwrap_or(' ');
                            i += 1;
                            match e {
                                'n' => metin.push('\n'),
                                't' => metin.push('\t'),
                                '"' => metin.push('"'),
                                '\\' => metin.push('\\'),
                                // Bilinmeyen kaçış olduğu gibi kalır: desenlerde "\d+" yazılabilsin.
                                _ => {
                                    metin.push('\\');
                                    i -= 1;
                                }
                            }
                        }
                        _ => metin.push(d),
                    }
                }
                if i < karakterler.len() && kelime_basi(karakterler[i]) {
                    let ek: String = karakterler[i..]
                        .iter()
                        .take_while(|c| kelime_devami(**c))
                        .collect();
                    let goster = if metin.chars().count() > 30 {
                        "...".to_string()
                    } else {
                        metin.clone()
                    };
                    return Err(Hata::yeni(
                        konum,
                        format!("metinden sonra gelen '{ek}' ekinin önünde kesme işareti olmalı"),
                    )
                    .ipucu(format!("\"{goster}\"'{ek} şeklinde yazın")));
                }
                cikti.push(Sozcuk {
                    tok: Tok::Metin(metin),
                    konum,
                });
                i = ek_oku(&karakterler, i, satir_no, &mut cikti)?;
                continue;
            }

            if kelime_basi(c) {
                let bas = i;
                while i < karakterler.len() && kelime_devami(karakterler[i]) {
                    i += 1;
                }
                let kelime: String = karakterler[bas..i].iter().collect();
                cikti.push(Sozcuk {
                    tok: Tok::Kelime(kelime),
                    konum,
                });
                i = ek_oku(&karakterler, i, satir_no, &mut cikti)?;
                continue;
            }

            if c == '.' {
                // `ürün.ad`: nokta iki yanında boşluk olmadan bir isim (ya da `)`, `]`)
                // ile bir kelime arasındaysa üye erişimidir; değilse cümle sonudur.
                let onceki_uygun = matches!(
                    cikti.last().map(|s: &Sozcuk| &s.tok),
                    Some(Tok::Kelime(_)) | Some(Tok::Op(")")) | Some(Tok::Op("]"))
                ) && i > 0
                    && !matches!(karakterler[i - 1], ' ' | '\t');
                let sonraki_kelime = karakterler.get(i + 1).is_some_and(|c| kelime_basi(*c));
                cikti.push(Sozcuk {
                    tok: if onceki_uygun && sonraki_kelime {
                        Tok::Uye
                    } else {
                        Tok::Op(".")
                    },
                    konum,
                });
                i += 1;
                continue;
            }

            let kalan: String = karakterler[i..karakterler.len().min(i + 2)]
                .iter()
                .collect();
            if let Some(op) = OPLAR.iter().find(|op| kalan.starts_with(**op)) {
                match *op {
                    "(" | "[" | "{" => parantez += 1,
                    ")" | "]" | "}" => parantez -= 1,
                    _ => {}
                }
                cikti.push(Sozcuk {
                    tok: Tok::Op(op),
                    konum,
                });
                i += op.chars().count();
                if *op == ")" || *op == "]" || *op == "}" {
                    i = ek_oku(&karakterler, i, satir_no, &mut cikti)?;
                }
                continue;
            }

            if kesme_mi(c) {
                return Err(Hata::yeni(
                    konum,
                    "kesme işaretinden önce bir isim, sayı ya da metin olmalı",
                )
                .ipucu("ek ile isim arasında boşluk bırakmayın: x'i"));
            }
            return Err(Hata::yeni(konum, format!("beklenmeyen karakter '{c}'")));
        }

        if parantez == 0 {
            let konum = Konum {
                satir: satir_no,
                sutun: karakterler.len() + 1,
                dosya: 0,
            };
            cikti.push(Sozcuk {
                tok: Tok::YeniSatir,
                konum,
            });
        }
    }

    let son_satir = kaynak.lines().count() + 1;
    let konum = Konum {
        satir: son_satir,
        sutun: 1,
        dosya: 0,
    };
    if parantez > 0 {
        return Err(Hata::yeni(konum, "kapatılmamış parantez"));
    }
    while girintiler.len() > 1 {
        girintiler.pop();
        cikti.push(Sozcuk {
            tok: Tok::Cikinti,
            konum,
        });
    }
    cikti.push(Sozcuk {
        tok: Tok::Son,
        konum,
    });
    Ok(cikti)
}

/// Kesme işaretiyle başlayan bir ek varsa okur ve `Ek` sözcüğü ekler.
fn ek_oku(k: &[char], mut i: usize, satir: usize, cikti: &mut Vec<Sozcuk>) -> Sonuc<usize> {
    if i < k.len() && kesme_mi(k[i]) {
        let konum = Konum {
            satir,
            sutun: i + 1,
            dosya: 0,
        };
        i += 1;
        let bas = i;
        while i < k.len() && k[i].is_alphabetic() {
            i += 1;
        }
        if bas == i {
            return Err(Hata::yeni(
                konum,
                "kesme işaretinden sonra bir ek bekleniyordu",
            ));
        }
        cikti.push(Sozcuk {
            tok: Tok::Ek(k[bas..i].iter().collect()),
            konum,
        });
    }
    Ok(i)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn turler(k: &str) -> Vec<Tok> {
        sozcukle(k).unwrap().into_iter().map(|s| s.tok).collect()
    }

    #[test]
    fn ekli_sayi() {
        assert_eq!(
            turler("5'i sayılara ekle."),
            vec![
                Tok::Sayi(5),
                Tok::Ek("i".into()),
                Tok::Kelime("sayılara".into()),
                Tok::Kelime("ekle".into()),
                Tok::Op("."),
                Tok::YeniSatir,
                Tok::Son
            ]
        );
    }

    #[test]
    fn girinti() {
        let t = turler("eğer x:\n    y = 1\nz = 2\n");
        assert!(t.contains(&Tok::Girinti));
        assert!(t.contains(&Tok::Cikinti));
    }

    #[test]
    fn ondalik_ve_cumle_sonu() {
        assert_eq!(
            turler("2.75'i yaz. 5."),
            vec![
                Tok::Ondalik(2.75),
                Tok::Ek("i".into()),
                Tok::Kelime("yaz".into()),
                Tok::Op("."),
                Tok::Sayi(5),
                Tok::Op("."),
                Tok::YeniSatir,
                Tok::Son
            ]
        );
    }

    #[test]
    fn uye_noktasi() {
        assert_eq!(
            turler("ü.ad'ı yaz. x.\ny = Ürün.hepsi()"),
            vec![
                Tok::Kelime("ü".into()),
                Tok::Uye,
                Tok::Kelime("ad".into()),
                Tok::Ek("ı".into()),
                Tok::Kelime("yaz".into()),
                Tok::Op("."),
                Tok::Kelime("x".into()),
                Tok::Op("."),
                Tok::YeniSatir,
                Tok::Kelime("y".into()),
                Tok::Op("="),
                Tok::Kelime("Ürün".into()),
                Tok::Uye,
                Tok::Kelime("hepsi".into()),
                Tok::Op("("),
                Tok::Op(")"),
                Tok::YeniSatir,
                Tok::Son
            ]
        );
    }

    #[test]
    fn eksiz_sayi_hatasi() {
        assert!(sozcukle("5i yaz").is_err());
    }
}
