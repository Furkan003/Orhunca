//! Adsız işlevler: `süz(sayılar, işlev(x) -> x > eşik)`.
//!
//! Denetçiden önce her `süz`, `dönüştür`, `sırala`, `biri_mi`, `hepsi_mi` çağrısı iki
//! üretilmiş işleve çevrilir:
//! - `‹adsız›N(x, eşik)`: adsız işlevin gövdesi; dışarıdan kullandığı yerel değişkenler
//!   (burada `eşik`) parametre olarak gelir.
//! - `‹süz›N(liste, eşik)`: listeyi gezip `‹adsız›N`'yi çağıran işlev (aşağıdaki kalıplar).
//!
//! Çağrı `‹süz›N(sayılar, eşik)` olur. Parametrelerin tipleri ilk çağrıdan çıkarıldığından
//! kod üreticiler (yerel, WebAssembly, PHP) adsız işlevleri ayrıca bilmez. Python ve
//! JavaScript çevirisi `‹adsız›N`'nin gövdesinden dilin kendi biçimini yazar.

use crate::agac::*;
use crate::hata::{Hata, Konum};
use std::collections::HashSet;

/// Adsız işlev alan işlevler ve kalıpları. `ADSIZ_ANAHTAR` adsız işlevin, `YAKALANAN`
/// dışarıdan gelen değişkenlerin yerine geçer.
const KALIPLAR: &[(&str, &str)] = &[
    (
        "süz",
        "işlev ADSIZ_SARGI(adsız_l YAKALANAN):
    adsız_s = []
    her adsız_x için adsız_l'den:
        eğer ADSIZ_ANAHTAR(adsız_x YAKALANAN) ise:
            adsız_x'i adsız_s'ye ekle.
    döndür adsız_s
",
    ),
    (
        "dönüştür",
        "işlev ADSIZ_SARGI(adsız_l YAKALANAN):
    adsız_s = []
    her adsız_x için adsız_l'den:
        ADSIZ_ANAHTAR(adsız_x YAKALANAN)'yi adsız_s'ye ekle.
    döndür adsız_s
",
    ),
    (
        "biri_mi",
        "işlev ADSIZ_SARGI(adsız_l YAKALANAN):
    her adsız_x için adsız_l'den:
        eğer ADSIZ_ANAHTAR(adsız_x YAKALANAN) ise:
            döndür doğru
    döndür yanlış
",
    ),
    (
        "hepsi_mi",
        "işlev ADSIZ_SARGI(adsız_l YAKALANAN):
    her adsız_x için adsız_l'den:
        eğer değil ADSIZ_ANAHTAR(adsız_x YAKALANAN) ise:
            döndür yanlış
    döndür doğru
",
    ),
    // Listeyi yerinde, anahtara göre sıralar (kararlı, alttan üste birleştirmeli sıralama).
    (
        "sırala",
        "işlev ADSIZ_SARGI(adsız_l YAKALANAN):
    adsız_a = []
    her adsız_x için adsız_l'den:
        ADSIZ_ANAHTAR(adsız_x YAKALANAN)'yi adsız_a'ya ekle.
    adsız_n = uzunluk(adsız_l)
    adsız_s = []
    adsız_i = 0
    adsız_i adsız_n'den küçük olduğu sürece:
        adsız_i'yi adsız_s'ye ekle.
        adsız_i += 1
    adsız_g = 1
    adsız_g adsız_n'den küçük olduğu sürece:
        adsız_t = []
        adsız_b = 0
        adsız_b adsız_n'den küçük olduğu sürece:
            adsız_o = adsız_b + adsız_g
            eğer adsız_o > adsız_n ise:
                adsız_o = adsız_n
            adsız_e = adsız_b + 2 * adsız_g
            eğer adsız_e > adsız_n ise:
                adsız_e = adsız_n
            adsız_i = adsız_b
            adsız_j = adsız_o
            (adsız_i < adsız_o veya adsız_j < adsız_e) olduğu sürece:
                adsız_sol = yanlış
                eğer adsız_j >= adsız_e ise:
                    adsız_sol = doğru
                değilse eğer adsız_i < adsız_o ise:
                    eğer adsız_a[adsız_s[adsız_i]] <= adsız_a[adsız_s[adsız_j]] ise:
                        adsız_sol = doğru
                eğer adsız_sol ise:
                    adsız_v = adsız_s[adsız_i]
                    adsız_i += 1
                değilse:
                    adsız_v = adsız_s[adsız_j]
                    adsız_j += 1
                adsız_v'yi adsız_t'ye ekle.
            adsız_b = adsız_e
        adsız_s = adsız_t
        adsız_g = adsız_g * 2
    adsız_k = kopya(adsız_l)
    adsız_i = 0
    adsız_i adsız_n'den küçük olduğu sürece:
        adsız_l[adsız_i] = adsız_k[adsız_s[adsız_i]]
        adsız_i += 1
",
    ),
];

/// Üretilen işlevlerin ad önekleri: `‹süz›3`, `‹adsız›3`.
pub const ANAHTAR_ONEKI: &str = "‹adsız›";

/// Üretilmiş bir sarmalayıcı adıysa (`‹süz›3`) işlevin türü (`süz`) ve sırası.
pub fn sargi(ad: &str) -> Option<(&'static str, &str)> {
    let ic = ad.strip_prefix('‹')?;
    let (tur, sira) = ic.split_once('›')?;
    let tur = KALIPLAR.iter().find(|(t, _)| *t == tur)?.0;
    Some((tur, sira))
}

pub fn indir(p: &mut Program) -> Result<(), Hata> {
    let mut ind = Indirgeyici {
        sayac: 0,
        yeni: Vec::new(),
        kullanici: p.islevler.iter().map(|f| f.ad.clone()).collect(),
        haric: p
            .sabitler
            .iter()
            .map(|(a, _)| a.clone())
            .chain(p.durumlar.iter().map(|d| d.ad.clone()))
            .collect(),
    };
    for f in p.islevler.iter_mut() {
        let parametreler: Vec<String> = f.parametreler.iter().map(|(a, _)| a.clone()).collect();
        ind.govde(&parametreler, &mut f.govde)?;
    }
    ind.govde(&[], &mut p.ana)?;
    // Üretilen anahtar işlevlerinin gövdesinde iç içe adsız işlevler olabilir.
    while let Some(mut f) = ind.yeni.pop() {
        let parametreler: Vec<String> = f.parametreler.iter().map(|(a, _)| a.clone()).collect();
        ind.govde(&parametreler, &mut f.govde)?;
        p.islevler.push(f);
    }
    Ok(())
}

struct Indirgeyici {
    sayac: usize,
    yeni: Vec<Islev>,
    /// Programın kendi işlevleri: `süz` adlı bir işlev tanımlanmışsa çağrısına dokunulmaz.
    kullanici: HashSet<String>,
    /// Her yerden görülen adlar (sabitler, durumlar): parametre olarak geçirilmez.
    haric: HashSet<String>,
}

impl Indirgeyici {
    fn govde(&mut self, parametreler: &[String], govde: &mut [Deyim]) -> Result<(), Hata> {
        let mut yereller: HashSet<String> = parametreler.iter().cloned().collect();
        yereller.extend(gecen_adlar(govde));
        ifadeleri_gez(govde, &mut |e| {
            if let IfadeTuru::Isim(a) = &e.tur {
                yereller.insert(a.clone());
            }
        });
        olay_adlari(govde, &mut yereller);
        let mut hata = None;
        ifadeleri_gez(govde, &mut |e| {
            if hata.is_none() {
                if let Err(h) = self.cagri(e, &yereller) {
                    hata = Some(h);
                }
            }
        });
        hata.map_or(Ok(()), Err)
    }

    fn cagri(&mut self, e: &mut Ifade, yereller: &HashSet<String>) -> Result<(), Hata> {
        let IfadeTuru::Cagri(ad, arg) = &mut e.tur else {
            return Ok(());
        };
        let Some((tur, kalip)) = KALIPLAR.iter().find(|(t, _)| t == ad) else {
            return Ok(());
        };
        if self.kullanici.contains(ad.as_str()) {
            return Ok(());
        }
        let ornek = if *tur == "sırala" {
            "sırala(kişiler, işlev(k) -> k.yaş)".to_string()
        } else {
            format!("{tur}(sayılar, işlev(x) -> x > 10)")
        };
        let (parametreler, govde) = match arg.as_slice() {
            [_, Ifade {
                tur: IfadeTuru::Adsiz(p, g),
                ..
            }] => (p.clone(), (**g).clone()),
            _ => {
                let mut h = Hata::yeni(
                    e.konum,
                    format!("{tur}(liste, işlev(x) -> ...) bir liste ve bir adsız işlev alır"),
                )
                .ipucu(ornek);
                if *tur == "sırala" {
                    h = h.ipucu("liste kendi sırasına göre sıralanacaksa: sayıları sırala.");
                }
                return Err(h);
            }
        };
        if parametreler.len() != 1 {
            return Err(Hata::yeni(
                arg[1].konum,
                format!(
                    "{tur} için adsız işlev tek parametre almalı, {} verildi",
                    parametreler.len()
                ),
            )
            .ipucu(ornek));
        }
        let konum = arg[1].konum;
        // Dışarıdan kullanılan yerel değişkenler, ilk geçiş sırasıyla
        let mut yakalanan = Vec::new();
        adlar(&govde, &parametreler, &mut yakalanan);
        yakalanan.retain(|a| yereller.contains(a) && !self.haric.contains(a));

        self.sayac += 1;
        let anahtar = format!("{ANAHTAR_ONEKI}{}", self.sayac);
        let sargi_adi = format!("‹{tur}›{}", self.sayac);
        let mut aparam: Vec<(String, Tip)> = vec![(parametreler[0].clone(), Tip::Bilinmeyen)];
        aparam.extend(yakalanan.iter().map(|a| (a.clone(), Tip::Bilinmeyen)));
        self.yeni.push(Islev {
            ad: anahtar.clone(),
            parametreler: aparam,
            haller: Vec::new(),
            donus: None,
            govde: vec![Deyim::Dondur(Some(govde), konum)],
            konum,
            yereller: Vec::new(),
            rota: None,
            arayuz: false,
            dis: None,
        });
        let ek: String = yakalanan.iter().map(|a| format!(", {a}")).collect();
        let kaynak = kalip.replace(" YAKALANAN", &ek);
        let mut s = kaliptan(&kaynak, &anahtar, konum)?;
        s.ad = sargi_adi.clone();
        self.yeni.push(s);

        let liste = arg.remove(0);
        let mut yeni_arg = vec![liste];
        yeni_arg.extend(
            yakalanan
                .into_iter()
                .map(|a| Ifade::yeni(IfadeTuru::Isim(a), konum)),
        );
        e.tur = IfadeTuru::Cagri(sargi_adi, yeni_arg);
        Ok(())
    }
}

/// Kalıbı ayrıştırır; `ADSIZ_ANAHTAR` çağrılarını üretilen anahtar işlevine bağlar ve
/// bütün konumları adsız işlevin konumu yapar (hatalar oraya gösterilir).
fn kaliptan(kaynak: &str, anahtar: &str, konum: Konum) -> Result<Islev, Hata> {
    let ic_hata = |h: Hata| Hata::yeni(konum, format!("iç hata (adsız işlev kalıbı): {}", h.mesaj));
    let sozcukler = crate::sozcuk::sozcukle(kaynak).map_err(ic_hata)?;
    let mut p = crate::ayristirici::ayristir_cok(vec![sozcukler], Vec::new()).map_err(ic_hata)?;
    let mut f = p.islevler.remove(0);
    f.konum = konum;
    konumlari_ata(&mut f.govde, konum);
    ifadeleri_gez(&mut f.govde, &mut |e| {
        e.konum = konum;
        if let IfadeTuru::Cagri(ad, _) = &mut e.tur {
            if ad == "ADSIZ_ANAHTAR" {
                *ad = anahtar.to_string();
            }
        }
    });
    Ok(f)
}

fn konumlari_ata(govde: &mut [Deyim], k: Konum) {
    for d in govde {
        match d {
            Deyim::Atama { konum, .. }
            | Deyim::AlanAtama { konum, .. }
            | Deyim::Dondur(_, konum)
            | Deyim::Dur(konum)
            | Deyim::Surdur(konum) => *konum = k,
            Deyim::HerAralik { konum, govde, .. } | Deyim::HerListe { konum, govde, .. } => {
                *konum = k;
                konumlari_ata(govde, k);
            }
            Deyim::Eger { govde, degilse, .. } => {
                konumlari_ata(govde, k);
                konumlari_ata(degilse, k);
            }
            Deyim::Surece { govde, .. } => konumlari_ata(govde, k),
            Deyim::Dene {
                konum,
                govde,
                yakala,
                ..
            } => {
                *konum = k;
                konumlari_ata(govde, k);
                konumlari_ata(yakala, k);
            }
            _ => {}
        }
    }
}

/// Olay bloklarının (`tıklanınca:`) içinde atanan değişkenler.
fn olay_adlari(govde: &[Deyim], adlar: &mut HashSet<String>) {
    for d in govde {
        match d {
            Deyim::Oge(o) => {
                if let Some(olay) = &o.olay {
                    adlar.extend(gecen_adlar(&olay.govde));
                    olay_adlari(&olay.govde, adlar);
                }
                olay_adlari(&o.cocuklar, adlar);
            }
            Deyim::Eger { govde, degilse, .. } => {
                olay_adlari(govde, adlar);
                olay_adlari(degilse, adlar);
            }
            Deyim::Surece { govde, .. }
            | Deyim::HerAralik { govde, .. }
            | Deyim::HerListe { govde, .. } => olay_adlari(govde, adlar),
            Deyim::Dene { govde, yakala, .. } => {
                olay_adlari(govde, adlar);
                olay_adlari(yakala, adlar);
            }
            _ => {}
        }
    }
}

/// İfadede geçen değişken adları (iç adsız işlevlerin kendi parametreleri hariç).
fn adlar(e: &Ifade, bagli: &[String], cikti: &mut Vec<String>) {
    let mut ekle = |a: &String| {
        if !bagli.contains(a) && !cikti.contains(a) {
            cikti.push(a.clone());
        }
    };
    match &e.tur {
        IfadeTuru::Isim(a) => ekle(a),
        IfadeTuru::Liste(l) | IfadeTuru::Cagri(_, l) => {
            l.iter().for_each(|x| adlar(x, bagli, cikti))
        }
        IfadeTuru::Sozluk(c) => c.iter().for_each(|(a, d)| {
            adlar(a, bagli, cikti);
            adlar(d, bagli, cikti)
        }),
        IfadeTuru::Ikili(_, a, b) | IfadeTuru::Indeks(a, b) => {
            adlar(a, bagli, cikti);
            adlar(b, bagli, cikti)
        }
        IfadeTuru::Tekli(_, a) | IfadeTuru::Alan(a, ..) => adlar(a, bagli, cikti),
        IfadeTuru::FiilCagri(_, l) => l.iter().for_each(|(_, x)| adlar(x, bagli, cikti)),
        IfadeTuru::Kurucu(_, l) => l.iter().for_each(|(_, x)| adlar(x, bagli, cikti)),
        IfadeTuru::Metod(a, _, l) => {
            adlar(a, bagli, cikti);
            l.iter().for_each(|x| adlar(x, bagli, cikti))
        }
        IfadeTuru::Adsiz(p, g) => {
            let mut ic: Vec<String> = bagli.to_vec();
            ic.extend(p.iter().cloned());
            adlar(g, &ic, cikti)
        }
        IfadeTuru::Sayi(_)
        | IfadeTuru::Ondalik(_)
        | IfadeTuru::Metin(_)
        | IfadeTuru::Mantik(_)
        | IfadeTuru::ModelAdi(_) => {}
    }
}
