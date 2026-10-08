//! Söz dizimi ağacının metin dökümü: Orhunca ile yazılmış ayrıştırıcının
//! (öz/ayrıştırıcı.ohc) çıktısıyla karşılaştırmak için. Her deyim bir satırdır;
//! bloklar iki boşluk içeriden yazılır, ifadeler parantezli tek satırdır.

use crate::agac::*;
use crate::ayristirici::YERLESIK_DOSYA;
use crate::hata::{Hata, Konum};

fn k(konum: Konum) -> String {
    format!("@{}:{}", konum.satir, konum.sutun)
}

/// Ondalığın en kısa gösterimi (`1.5`, `2.0`).
fn ondalik(f: f64) -> String {
    format!("{f:?}")
}

fn metin(m: &str) -> String {
    serde_json::to_string(m).unwrap()
}

pub fn ifade(e: &Ifade) -> String {
    let yer = k(e.konum);
    let liste = |l: &[Ifade]| {
        l.iter()
            .map(|x| format!(" {}", ifade(x)))
            .collect::<String>()
    };
    match &e.tur {
        IfadeTuru::Sayi(n) => format!("(sayı{yer} {n})"),
        IfadeTuru::Ondalik(f) => format!("(ondalık{yer} {})", ondalik(*f)),
        IfadeTuru::Metin(m) => format!("(metin{yer} {})", metin(m)),
        IfadeTuru::Mantik(b) => format!("(mantık{yer} {})", if *b { "doğru" } else { "yanlış" }),
        IfadeTuru::Isim(a) => format!("(isim{yer} {a})"),
        IfadeTuru::Liste(l) => format!("(liste{yer}{})", liste(l)),
        IfadeTuru::Sozluk(c) => format!(
            "(sözlük{yer}{})",
            c.iter()
                .map(|(a, d)| format!(" (çift {} {})", ifade(a), ifade(d)))
                .collect::<String>()
        ),
        IfadeTuru::Ikili(op, a, b) => format!("(ikili{yer} {op:?} {} {})", ifade(a), ifade(b)),
        IfadeTuru::Tekli(op, a) => format!("(tekli{yer} {op:?} {})", ifade(a)),
        IfadeTuru::Cagri(ad, l) => format!("(çağrı{yer} {ad}{})", liste(l)),
        IfadeTuru::FiilCagri(ad, l) => format!(
            "(fiil{yer} {ad}{})",
            l.iter()
                .map(|(h, x)| format!(" ({h:?} {})", ifade(x)))
                .collect::<String>()
        ),
        IfadeTuru::Indeks(a, b) => format!("(indeks{yer} {} {})", ifade(a), ifade(b)),
        IfadeTuru::Kurucu(m, l) => format!(
            "(kurucu{yer} {m}{})",
            l.iter()
                .map(|(a, x)| format!(" ({a} {})", ifade(x)))
                .collect::<String>()
        ),
        IfadeTuru::Alan(n, a, _) => format!("(alan{yer} {} {a})", ifade(n)),
        IfadeTuru::Metod(n, a, l) => format!("(yöntem{yer} {} {a}{})", ifade(n), liste(l)),
        IfadeTuru::ModelAdi(m) => format!("(modeladı{yer} {m})"),
    }
}

fn tip(t: &Option<Tip>) -> String {
    t.as_ref()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "-".into())
}

fn blok(govde: &[Deyim], girinti: usize, s: &mut String) {
    for d in govde {
        deyim(d, girinti, s);
    }
}

fn satir(s: &mut String, girinti: usize, metin: &str) {
    s.push_str(&" ".repeat(girinti));
    s.push_str(metin);
    s.push('\n');
}

fn olay(o: &Olay, girinti: usize, s: &mut String) {
    satir(s, girinti, &format!("olay{} {}", k(o.konum), o.ad));
    blok(&o.govde, girinti + 2, s);
}

fn deyim(d: &Deyim, g: usize, s: &mut String) {
    match d {
        Deyim::Atama {
            hedef,
            tip: t,
            deger,
            konum,
        } => satir(
            s,
            g,
            &format!("atama{} {hedef} {} {}", k(*konum), tip(t), ifade(deger)),
        ),
        Deyim::IndeksAtama {
            liste,
            indeks,
            deger,
        } => satir(
            s,
            g,
            &format!(
                "indeks_atama {} {} {}",
                ifade(liste),
                ifade(indeks),
                ifade(deger)
            ),
        ),
        Deyim::AlanAtama {
            nesne,
            alan,
            deger,
            konum,
            ..
        } => satir(
            s,
            g,
            &format!(
                "alan_atama{} {} {alan} {}",
                k(*konum),
                ifade(nesne),
                ifade(deger)
            ),
        ),
        Deyim::Yaz(e) => satir(s, g, &format!("yaz {}", ifade(e))),
        Deyim::Ekle { oge, liste } => satir(s, g, &format!("ekle {} {}", ifade(oge), ifade(liste))),
        Deyim::Cikar { oge, liste } => {
            satir(s, g, &format!("çıkar {} {}", ifade(oge), ifade(liste)))
        }
        Deyim::DosyayaYaz { deger, yol } => satir(
            s,
            g,
            &format!("dosyaya_yaz {} {}", ifade(deger), ifade(yol)),
        ),
        Deyim::Sirala(e) => satir(s, g, &format!("sırala {}", ifade(e))),
        Deyim::Eger {
            kosul,
            govde,
            degilse,
        } => {
            satir(s, g, &format!("eğer {}", ifade(kosul)));
            blok(govde, g + 2, s);
            if !degilse.is_empty() {
                satir(s, g, "değilse");
                blok(degilse, g + 2, s);
            }
        }
        Deyim::Surece { kosul, govde } => {
            satir(s, g, &format!("sürece {}", ifade(kosul)));
            blok(govde, g + 2, s);
        }
        Deyim::HerAralik {
            degisken,
            bas,
            son,
            govde,
            konum,
        } => {
            satir(
                s,
                g,
                &format!(
                    "her_aralık{} {degisken} {} {}",
                    k(*konum),
                    ifade(bas),
                    ifade(son)
                ),
            );
            blok(govde, g + 2, s);
        }
        Deyim::HerListe {
            degisken,
            liste,
            govde,
            konum,
        } => {
            satir(
                s,
                g,
                &format!("her_liste{} {degisken} {}", k(*konum), ifade(liste)),
            );
            blok(govde, g + 2, s);
        }
        Deyim::Dondur(e, konum) => satir(
            s,
            g,
            &match e {
                Some(e) => format!("döndür{} {}", k(*konum), ifade(e)),
                None => format!("döndür{}", k(*konum)),
            },
        ),
        Deyim::Dur(konum) => satir(s, g, &format!("dur{}", k(*konum))),
        Deyim::Surdur(konum) => satir(s, g, &format!("sürdür{}", k(*konum))),
        Deyim::IfadeDeyimi(e) => satir(s, g, &format!("ifade {}", ifade(e))),
        Deyim::Oge(o) => {
            let mut m = format!("öğe{} {}", k(o.konum), o.ad);
            for a in &o.argumanlar {
                m.push(' ');
                m.push_str(&ifade(a));
            }
            for (a, x) in &o.secenekler {
                m.push_str(&format!(" ({a} {})", ifade(x)));
            }
            satir(s, g, &m);
            if let Some(x) = &o.olay {
                olay(x, g + 2, s);
            }
            blok(&o.cocuklar, g + 2, s);
        }
        Deyim::Dene {
            govde,
            degisken,
            yakala,
            konum,
        } => {
            satir(
                s,
                g,
                &format!("dene{} {}", k(*konum), degisken.as_deref().unwrap_or("-")),
            );
            blok(govde, g + 2, s);
            satir(s, g, "yakala");
            blok(yakala, g + 2, s);
        }
    }
}

fn sayi_ya_da_tire(x: Option<f64>) -> String {
    x.map(ondalik).unwrap_or_else(|| "-".into())
}

/// Programın dökümü: modeller (yerleşikler hariç), seçenekler, sabitler, durumlar,
/// işlevler ve ana program, kaynaktaki sırayla.
pub fn program(p: &Program) -> String {
    let mut s = String::new();
    for m in p
        .modeller
        .iter()
        .filter(|m| m.konum.dosya != YERLESIK_DOSYA)
    {
        satir(&mut s, 0, &format!("model{} {}", k(m.konum), m.ad));
        for a in &m.alanlar {
            let varsayilan = a
                .varsayilan
                .as_ref()
                .map(ifade)
                .unwrap_or_else(|| "-".into());
            satir(
                &mut s,
                2,
                &format!(
                    "alan{} {} {} zorunlu={} e_posta={} en_az={} en_fazla={} etiket={} {varsayilan}",
                    k(a.konum),
                    a.ad,
                    a.tip,
                    a.zorunlu,
                    a.e_posta,
                    sayi_ya_da_tire(a.en_az),
                    sayi_ya_da_tire(a.en_fazla),
                    a.etiket.as_deref().map(metin).unwrap_or_else(|| "-".into()),
                ),
            );
        }
    }
    for t in &p.secenekler {
        satir(
            &mut s,
            0,
            &format!("seçenek{} {} {}", k(t.konum), t.ad, t.degerler.join(" ")),
        );
    }
    for (ad, e) in &p.sabitler {
        satir(&mut s, 0, &format!("sabit {ad} {}", ifade(e)));
    }
    for d in &p.durumlar {
        satir(
            &mut s,
            0,
            &format!(
                "durum{} {} {} {}",
                k(d.konum),
                d.ad,
                tip(&d.tip),
                ifade(&d.deger)
            ),
        );
    }
    for f in &p.islevler {
        let mut m = format!("işlev{} {}", k(f.konum), f.ad);
        for ((p, t), i) in f.parametreler.iter().zip(0..) {
            match f.haller.get(i) {
                Some(h) => m.push_str(&format!(" ({p} {t} {h:?})")),
                None => m.push_str(&format!(" ({p} {t})")),
            }
        }
        m.push_str(&format!(" -> {}", tip(&f.donus)));
        if f.arayuz {
            m.push_str(" arayüz");
        }
        if let Some(r) = &f.rota {
            m.push_str(&format!(" rota {} {}", r.yontem, metin(&r.kalip)));
        }
        if let Some(d) = &f.dis {
            let tipler: Vec<&str> = d.tipler.iter().map(|t| t.adi()).collect();
            m.push_str(&format!(
                " dış {} ({}) -> {}",
                metin(&d.kutuphane),
                tipler.join(" "),
                d.donus.adi()
            ));
        }
        satir(&mut s, 0, &m);
        blok(&f.govde, 2, &mut s);
    }
    satir(&mut s, 0, "ana");
    blok(&p.ana, 2, &mut s);
    s
}

/// Ayrıştırma hatasının dökümü.
pub fn hata(h: &Hata) -> String {
    format!(
        "HATA{} {} | {}\n",
        k(h.konum),
        h.mesaj,
        h.ipucu.as_deref().unwrap_or("-")
    )
}

/// Tek bir kaynak dosyayı (kütüphaneleri olmadan) ayrıştırır ve dökümünü verir.
/// Sözcük hatalarında ipucu yazılmaz (Orhunca çözümleyicisi ipucu üretmez).
pub fn kaynak(kaynak: &str) -> String {
    let sozcukler = match crate::sozcuk::sozcukle(kaynak) {
        Ok(s) => s,
        Err(h) => {
            return hata(&Hata {
                ipucu: None,
                oneri: None,
                ..h
            })
        }
    };
    match crate::ayristirici::ayristir_cok(vec![sozcukler], Vec::new()) {
        Ok(p) => program(&p),
        Err(h) => hata(&h),
    }
}
