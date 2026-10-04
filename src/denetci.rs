//! Anlam ve tip denetimi. Her ifadenin `tip` alanını doldurur, işlevlerin yerel
//! değişkenlerini toplar ve Türkçe hata mesajları üretir.
//!
//! Kapsam kuralı (v0.1): değişkenler işlev düzeyindedir; işlevler yalnızca kendi
//! parametrelerini ve yerel değişkenlerini görür.

use crate::agac::*;
use crate::ekler::Hal;
use crate::hata::{Hata, Konum, Sonuc};
use std::collections::HashMap;

#[derive(Clone)]
struct Imza {
    parametreler: Vec<Tip>,
    /// Fiillerde parametrelerin hâl ekleri; işlevlerde boş.
    haller: Vec<Hal>,
    /// `None`: henüz çıkarılmadı.
    donus: Option<Tip>,
}

pub struct Denetci {
    imzalar: HashMap<String, Imza>,
    /// Dönüş tipi bilinmeden çağrıldığı için `sayı` varsayılan işlevler.
    varsayilan: HashMap<String, Konum>,
    kapsam: HashMap<String, Tip>,
    sira: Vec<String>,
    donus: Option<Tip>,
    donus_belirtildi: bool,
    dongu: usize,
    islevde: bool,
}

pub fn denetle(p: &mut Program) -> Sonuc<()> {
    let mut d = Denetci {
        imzalar: HashMap::new(),
        varsayilan: HashMap::new(),
        kapsam: HashMap::new(),
        sira: Vec::new(),
        donus: None,
        donus_belirtildi: false,
        dongu: 0,
        islevde: false,
    };
    for f in &p.islevler {
        if d.imzalar.contains_key(&f.ad) {
            return Err(Hata::yeni(
                f.konum,
                format!("'{}' işlevi birden fazla tanımlanmış", f.ad),
            ));
        }
        d.imzalar.insert(
            f.ad.clone(),
            Imza {
                parametreler: f.parametreler.iter().map(|p| p.1.clone()).collect(),
                haller: f.haller.clone(),
                donus: f.donus.clone(),
            },
        );
    }

    for f in p.islevler.iter_mut() {
        d.kapsam.clear();
        d.sira.clear();
        d.islevde = true;
        d.donus_belirtildi = f.donus.is_some();
        d.donus = f.donus.clone();
        for (ad, tip) in &f.parametreler {
            if d.kapsam.contains_key(ad) {
                return Err(Hata::yeni(
                    f.konum,
                    format!("'{ad}' parametresi iki kez yazılmış"),
                ));
            }
            d.tanimla(ad, tip.clone());
        }
        d.blok(&mut f.govde)?;
        let donus = d.donus.clone().unwrap_or(Tip::Bos);
        f.donus = Some(donus.clone());
        d.imzalar.get_mut(&f.ad).unwrap().donus = Some(donus.clone());
        if let Some(k) = d.varsayilan.get(&f.ad) {
            if donus != Tip::Sayi {
                return Err(Hata::yeni(
                    *k,
                    format!(
                        "'{}' işlevinin dönüş tipi tanımından önce çıkarılamadı",
                        f.ad
                    ),
                )
                .ipucu(format!(
                    "tanımda dönüş tipini belirtin: işlev {}(...) -> {donus}:",
                    f.ad
                )));
            }
        }
        f.yereller = d.yereller();
    }

    d.kapsam.clear();
    d.sira.clear();
    d.islevde = false;
    d.donus = None;
    d.blok(&mut p.ana)?;
    p.ana_yereller = d.yereller();
    Ok(())
}

/// Sayı bekleyen bir yere ondalık gerekiyorsa ifadeyi `ondalık(...)` ile sarar.
fn genislet(e: &mut Ifade, hedef: &Tip) {
    if *hedef == Tip::Ondalik && e.tip == Tip::Sayi {
        let konum = e.konum;
        let ic = std::mem::replace(e, Ifade::yeni(IfadeTuru::Mantik(false), konum));
        *e = Ifade {
            tur: IfadeTuru::Cagri("ondalık".into(), vec![ic]),
            konum,
            tip: Tip::Ondalik,
        };
    }
}

impl Denetci {
    fn tanimla(&mut self, ad: &str, tip: Tip) {
        if self.kapsam.insert(ad.to_string(), tip).is_none() {
            self.sira.push(ad.to_string());
        }
    }

    fn yereller(&self) -> Vec<(String, Tip)> {
        self.sira
            .iter()
            .map(|a| (a.clone(), self.kapsam[a].clone()))
            .collect()
    }

    /// Değişkene tip atar ya da mevcut tiple birleştirir.
    fn ata(&mut self, ad: &str, tip: Tip, konum: Konum) -> Sonuc<()> {
        if tip == Tip::Bos {
            return Err(Hata::yeni(
                konum,
                "değer döndürmeyen bir işlevin sonucu atanamaz",
            ));
        }
        match self.kapsam.get(ad).cloned() {
            None => {
                self.tanimla(ad, tip);
                Ok(())
            }
            Some(eski) => match eski.birlestir(&tip) {
                Some(t) => {
                    self.kapsam.insert(ad.to_string(), t);
                    Ok(())
                }
                None => {
                    let mut h = Hata::yeni(
                        konum,
                        format!("'{ad}' değişkeni {eski} tipinde; {tip} değer atanamaz"),
                    );
                    if eski == Tip::Sayi && tip == Tip::Ondalik {
                        h = h.ipucu(format!("değişkeni ondalık olarak başlatın: {ad} = 0.0"));
                    }
                    Err(h)
                }
            },
        }
    }

    /// Bir liste ifadesinin öğe tipini bilinen bir tiple daraltır (`l = []` sonrası).
    fn listeyi_daralt(&mut self, liste: &Ifade, oge: &Tip) {
        if let IfadeTuru::Isim(ad) = &liste.tur {
            if let Some(Tip::Liste(ic)) = self.kapsam.get(ad).cloned() {
                if let Some(t) = ic.birlestir(oge) {
                    self.kapsam.insert(ad.clone(), Tip::Liste(Box::new(t)));
                }
            }
        }
    }

    fn blok(&mut self, govde: &mut [Deyim]) -> Sonuc<()> {
        for d in govde {
            self.deyim(d)?;
        }
        Ok(())
    }

    fn kosul(&mut self, k: &mut Ifade) -> Sonuc<()> {
        let t = self.ifade(k)?;
        if t != Tip::Mantik {
            return Err(Hata::yeni(
                k.konum,
                format!("koşul mantık (doğru/yanlış) olmalı, {t} bulundu"),
            ));
        }
        Ok(())
    }

    fn liste_tipi(&mut self, l: &mut Ifade) -> Sonuc<Tip> {
        match self.ifade(l)? {
            Tip::Liste(ic) => Ok(*ic),
            t => Err(Hata::yeni(
                l.konum,
                format!("liste bekleniyordu, {t} bulundu"),
            )),
        }
    }

    fn deyim(&mut self, d: &mut Deyim) -> Sonuc<()> {
        match d {
            Deyim::Atama {
                hedef,
                deger,
                konum,
            } => {
                let mut t = self.ifade(deger)?;
                if self.kapsam.get(hedef.as_str()) == Some(&Tip::Ondalik) {
                    genislet(deger, &Tip::Ondalik);
                    t = deger.tip.clone();
                }
                if self.imzalar.contains_key(hedef.as_str()) {
                    return Err(Hata::yeni(
                        *konum,
                        format!("'{hedef}' bir işlevin adı, değişken olarak kullanılamaz"),
                    ));
                }
                self.ata(hedef, t, *konum)?;
            }
            Deyim::IndeksAtama {
                liste,
                indeks,
                deger,
            } => {
                let ic = self.liste_tipi(liste)?;
                self.sayi_bekle(indeks, "indeks")?;
                let t = self.ifade(deger)?;
                if !ic.kabul_eder(&t) {
                    return Err(Hata::yeni(
                        deger.konum,
                        format!("liste<{ic}> içine {t} konamaz"),
                    ));
                }
                genislet(deger, &ic);
                self.listeyi_daralt(liste, &deger.tip.clone());
            }
            Deyim::Yaz(i) => {
                if self.ifade(i)? == Tip::Bos {
                    return Err(Hata::yeni(
                        i.konum,
                        "bu işlev bir değer döndürmüyor, yazdırılamaz",
                    ));
                }
            }
            Deyim::Ekle { oge, liste } => {
                let t = self.ifade(oge)?;
                let ic = self.liste_tipi(liste)?;
                if !ic.kabul_eder(&t) {
                    return Err(Hata::yeni(
                        oge.konum,
                        format!("liste<{ic}> listesine {t} eklenemez"),
                    ));
                }
                genislet(oge, &ic);
                let t = oge.tip.clone();
                self.listeyi_daralt(liste, &t);
                liste.tip = Tip::Liste(Box::new(ic.birlestir(&t).unwrap()));
            }
            Deyim::Sirala(l) => {
                let ic = self.liste_tipi(l)?;
                if !matches!(ic, Tip::Sayi | Tip::Ondalik | Tip::Metin | Tip::Bilinmeyen) {
                    return Err(Hata::yeni(
                        l.konum,
                        format!(
                            "liste<{ic}> sıralanamaz; yalnızca sayı, ondalık ve metin listeleri sıralanır"
                        ),
                    ));
                }
            }
            Deyim::Eger {
                kosul,
                govde,
                degilse,
            } => {
                self.kosul(kosul)?;
                self.blok(govde)?;
                self.blok(degilse)?;
            }
            Deyim::Surece { kosul, govde } => {
                self.kosul(kosul)?;
                self.dongu += 1;
                self.blok(govde)?;
                self.dongu -= 1;
            }
            Deyim::HerAralik {
                degisken,
                bas,
                son,
                govde,
                konum,
            } => {
                self.sayi_bekle(bas, "aralığın başı")?;
                self.sayi_bekle(son, "aralığın sonu")?;
                self.ata(degisken, Tip::Sayi, *konum)?;
                self.dongu += 1;
                self.blok(govde)?;
                self.dongu -= 1;
            }
            Deyim::HerListe {
                degisken,
                liste,
                govde,
                konum,
            } => {
                let ic = self.liste_tipi(liste)?;
                let ic = if ic == Tip::Bilinmeyen { Tip::Sayi } else { ic };
                self.ata(degisken, ic, *konum)?;
                self.dongu += 1;
                self.blok(govde)?;
                self.dongu -= 1;
            }
            Deyim::Dondur(deger, konum) => {
                if !self.islevde {
                    return Err(Hata::yeni(
                        *konum,
                        "'döndür' yalnızca bir işlevin içinde kullanılabilir",
                    ));
                }
                let mut t = match deger {
                    Some(i) => self.ifade(i)?,
                    None => Tip::Bos,
                };
                if let (Some(i), Some(beklenen)) = (deger.as_mut(), &self.donus) {
                    genislet(i, beklenen);
                    t = i.tip.clone();
                }
                match &self.donus {
                    None => self.donus = Some(t),
                    Some(beklenen) => {
                        if beklenen.birlestir(&t).is_none() {
                            if *beklenen == Tip::Sayi && t == Tip::Ondalik {
                                return Err(Hata::yeni(
                                    *konum,
                                    "önceki 'döndür' sayı, burada ondalık döndürülüyor",
                                )
                                .ipucu("tanımda dönüş tipini belirtin: -> ondalık"));
                            }
                            let neden = if self.donus_belirtildi {
                                "işlevin dönüş tipi"
                            } else {
                                "önceki 'döndür' deyimi"
                            };
                            return Err(Hata::yeni(
                                *konum,
                                format!("{neden} {beklenen}, burada {t} döndürülüyor"),
                            ));
                        }
                    }
                }
            }
            Deyim::Dur(k) | Deyim::Surdur(k) => {
                if self.dongu == 0 {
                    return Err(Hata::yeni(
                        *k,
                        "'dur' ve 'sürdür' yalnızca bir döngünün içinde kullanılabilir",
                    ));
                }
            }
            Deyim::IfadeDeyimi(i) => {
                self.ifade(i)?;
            }
        }
        Ok(())
    }

    fn sayi_bekle(&mut self, i: &mut Ifade, ne: &str) -> Sonuc<()> {
        let t = self.ifade(i)?;
        if t != Tip::Sayi {
            return Err(Hata::yeni(
                i.konum,
                format!("{ne} sayı olmalı, {t} bulundu"),
            ));
        }
        Ok(())
    }

    fn ifade(&mut self, e: &mut Ifade) -> Sonuc<Tip> {
        let konum = e.konum;
        let tip = match &mut e.tur {
            IfadeTuru::Sayi(_) => Tip::Sayi,
            IfadeTuru::Ondalik(_) => Tip::Ondalik,
            IfadeTuru::Metin(_) => Tip::Metin,
            IfadeTuru::Mantik(_) => Tip::Mantik,
            IfadeTuru::Isim(ad) => match self.kapsam.get(ad.as_str()) {
                Some(t) => t.clone(),
                None => {
                    let mut h = Hata::yeni(konum, format!("'{ad}' tanımlanmadan kullanıldı"));
                    if self.islevde {
                        h = h.ipucu("işlevler dışarıdaki değişkenleri göremez; değeri parametre olarak geçirin");
                    }
                    return Err(h);
                }
            },
            IfadeTuru::Liste(ogeler) => {
                let mut ic = Tip::Bilinmeyen;
                for o in ogeler.iter_mut() {
                    let t = self.ifade(o)?;
                    // [1, 2.5] → liste<ondalık>
                    if ic.sayisal() && t.sayisal() && ic != t {
                        ic = Tip::Ondalik;
                        continue;
                    }
                    ic = ic.birlestir(&t).ok_or_else(|| {
                        Hata::yeni(
                            o.konum,
                            format!("listenin tüm öğeleri aynı tipte olmalı ({ic} ve {t})"),
                        )
                    })?;
                }
                for o in ogeler.iter_mut() {
                    genislet(o, &ic);
                }
                Tip::Liste(Box::new(ic))
            }
            IfadeTuru::Tekli(op, ic) => {
                let t = self.ifade(ic)?;
                match (op, &t) {
                    (TekliOp::Eksi, Tip::Sayi) => Tip::Sayi,
                    (TekliOp::Eksi, Tip::Ondalik) => Tip::Ondalik,
                    (TekliOp::Degil, Tip::Mantik) => Tip::Mantik,
                    (TekliOp::Eksi, _) => {
                        return Err(Hata::yeni(
                            konum,
                            format!("eksi işareti sayılara uygulanır, {t} bulundu"),
                        ))
                    }
                    (TekliOp::Degil, _) => {
                        return Err(Hata::yeni(
                            konum,
                            format!("'değil' mantık değerlerine uygulanır, {t} bulundu"),
                        ))
                    }
                }
            }
            IfadeTuru::Ikili(op, sol, sag) => {
                let a = self.ifade(sol)?;
                let b = self.ifade(sag)?;
                let sonuc = ikili_tip(*op, &a, &b).ok_or_else(|| {
                    let mut h = Hata::yeni(
                        konum,
                        format!("'{}' işlemi {a} ve {b} arasında yapılamaz", op_adi(*op)),
                    );
                    if *op == IkiliOp::TamBol || *op == IkiliOp::Mod {
                        h = h.ipucu(
                            "'//' ve '%' yalnızca sayılarla kullanılır; sayı(x) ile çevirin",
                        );
                    }
                    h
                })?;
                // Karışık sayı/ondalık işlemlerinde iki taraf da ondalığa çevrilir.
                if a.sayisal() && b.sayisal() && (a != b || *op == IkiliOp::Bol) {
                    genislet(sol, &Tip::Ondalik);
                    genislet(sag, &Tip::Ondalik);
                }
                sonuc
            }
            IfadeTuru::Indeks(l, i) => {
                let ic = self.liste_tipi(l)?;
                self.sayi_bekle(i, "indeks")?;
                if ic == Tip::Bilinmeyen {
                    Tip::Sayi
                } else {
                    ic
                }
            }
            IfadeTuru::Cagri(ad, arg) => {
                let ad = ad.clone();
                self.cagri(&ad, arg, konum)?
            }
            IfadeTuru::FiilCagri(ad, arg) => {
                let ad = ad.clone();
                let sirali = self.fiil_eslestir(&ad, std::mem::take(arg), konum)?;
                e.tur = IfadeTuru::Cagri(ad, sirali);
                return self.ifade(e);
            }
        };
        e.tip = tip.clone();
        Ok(tip)
    }

    /// Fiil çağrısındaki öğeleri hâl eklerine göre parametre sırasına dizer.
    fn fiil_eslestir(
        &self,
        ad: &str,
        mut arg: Vec<(Hal, Ifade)>,
        konum: Konum,
    ) -> Sonuc<Vec<Ifade>> {
        let Some(imza) = self.imzalar.get(ad) else {
            return Err(Hata::yeni(konum, format!("tanımsız fiil '{ad}'")));
        };
        if imza.haller.is_empty() && !imza.parametreler.is_empty() {
            return Err(Hata::yeni(
                konum,
                format!("'{ad}' bir işlev; parantezle çağırın: {ad}(...)"),
            ));
        }
        if let Some((hal, fazla)) = arg.iter().find(|(h, _)| !imza.haller.contains(h)) {
            return Err(Hata::yeni(
                fazla.konum,
                format!("'{ad}' fiili {} hâlinde bir öğe almaz", hal.adi()),
            )
            .ipucu(fiil_ornegi(ad, &imza.haller)));
        }
        let mut sirali = Vec::new();
        for hal in &imza.haller {
            match arg.iter().position(|(h, _)| h == hal) {
                Some(i) => sirali.push(arg.remove(i).1),
                None => {
                    return Err(Hata::yeni(
                        konum,
                        format!("'{ad}' fiili {} hâlinde bir öğe bekliyor", hal.adi()),
                    )
                    .ipucu(fiil_ornegi(ad, &imza.haller)))
                }
            }
        }
        if let Some((hal, fazla)) = arg.first() {
            let neden = if imza.haller.contains(hal) {
                format!("{} hâlinde birden fazla öğe var", hal.adi())
            } else {
                format!("'{ad}' fiili {} hâlinde bir öğe almaz", hal.adi())
            };
            return Err(Hata::yeni(fazla.konum, neden).ipucu(fiil_ornegi(ad, &imza.haller)));
        }
        Ok(sirali)
    }

    fn cagri(&mut self, ad: &str, arg: &mut [Ifade], konum: Konum) -> Sonuc<Tip> {
        let mut tipler = Vec::new();
        for a in arg.iter_mut() {
            tipler.push(self.ifade(a)?);
        }
        if let Some(imza) = self.imzalar.get(ad).cloned() {
            if imza.parametreler.len() != tipler.len() {
                return Err(Hata::yeni(
                    konum,
                    format!(
                        "'{ad}' {} bağımsız değişken bekler, {} verildi",
                        imza.parametreler.len(),
                        tipler.len()
                    ),
                ));
            }
            for (i, (p, t)) in imza.parametreler.iter().zip(&tipler).enumerate() {
                genislet(&mut arg[i], p);
                if !p.kabul_eder(t) {
                    let tur = if imza.haller.is_empty() {
                        "işlevinin"
                    } else {
                        "fiilinin"
                    };
                    return Err(Hata::yeni(
                        arg[i].konum,
                        format!(
                            "'{ad}' {tur} {}. parametresi {p} olmalı, {t} verildi",
                            i + 1
                        ),
                    )
                    .ipucu(format!(
                        "tipi belirtilmeyen parametreler sayıdır; tanımda belirtin: (ad: {t})"
                    )));
                }
            }
            return Ok(match imza.donus {
                Some(t) => t,
                None => {
                    self.varsayilan.entry(ad.to_string()).or_insert(konum);
                    Tip::Sayi
                }
            });
        }
        let tek = |beklenen: &str| -> Sonuc<()> {
            if tipler.len() != 1 {
                return Err(Hata::yeni(
                    konum,
                    format!("'{ad}' bir bağımsız değişken bekler ({beklenen})"),
                ));
            }
            Ok(())
        };
        Ok(match ad {
            "uzunluk" => {
                tek("liste ya da metin")?;
                match tipler[0] {
                    Tip::Liste(_) | Tip::Metin => Tip::Sayi,
                    ref t => {
                        return Err(Hata::yeni(
                            konum,
                            format!("uzunluk liste ya da metin için hesaplanır, {t} bulundu"),
                        ))
                    }
                }
            }
            "metin" => {
                tek("sayı, ondalık, mantık ya da metin")?;
                match tipler[0] {
                    Tip::Sayi | Tip::Ondalik | Tip::Mantik | Tip::Metin => Tip::Metin,
                    ref t => return Err(Hata::yeni(konum, format!("{t} metne dönüştürülemez"))),
                }
            }
            "sayı" => {
                tek("metin ya da ondalık")?;
                match tipler[0] {
                    Tip::Metin | Tip::Sayi | Tip::Ondalik => Tip::Sayi,
                    ref t => return Err(Hata::yeni(konum, format!("{t} sayıya dönüştürülemez"))),
                }
            }
            "ondalık" => {
                tek("metin ya da sayı")?;
                match tipler[0] {
                    Tip::Metin | Tip::Sayi | Tip::Ondalik => Tip::Ondalik,
                    ref t => return Err(Hata::yeni(konum, format!("{t} ondalığa dönüştürülemez"))),
                }
            }
            "yuvarla" => match tipler.as_slice() {
                [t] if t.sayisal() => Tip::Sayi,
                [t, Tip::Sayi] if t.sayisal() => {
                    genislet(&mut arg[0], &Tip::Ondalik);
                    Tip::Ondalik
                }
                _ => {
                    return Err(Hata::yeni(
                        konum,
                        "'yuvarla' bir sayı ya da (ondalık, basamak sayısı) bekler",
                    )
                    .ipucu("yuvarla(3.7) → 4,  yuvarla(3.14159, 2) → 3.14"))
                }
            },
            "oku" => {
                if !tipler.is_empty() {
                    return Err(Hata::yeni(konum, "'oku' bağımsız değişken almaz"));
                }
                Tip::Metin
            }
            _ => return Err(Hata::yeni(konum, format!("tanımsız işlev '{ad}'"))),
        })
    }
}

fn fiil_ornegi(ad: &str, haller: &[Hal]) -> String {
    let ornek: Vec<&str> = haller
        .iter()
        .map(|h| match h {
            Hal::Belirtme => "x'i",
            Hal::Yonelme => "y'ye",
            Hal::Ayrilma => "z'den",
            Hal::Bulunma => "w'de",
            Hal::Vasita => "v'yle",
            Hal::Ilgi => "u'nun",
        })
        .collect();
    format!("kullanım: {} {ad}.", ornek.join(" "))
}

pub fn ikili_tip(op: IkiliOp, a: &Tip, b: &Tip) -> Option<Tip> {
    use IkiliOp::*;
    use Tip::*;
    let sayisal = a.sayisal() && b.sayisal();
    let karisik = if *a == Ondalik || *b == Ondalik {
        Ondalik
    } else {
        Sayi
    };
    match (op, a, b) {
        (Topla | Cikar | Carp, _, _) if sayisal => Some(karisik),
        (Bol, _, _) if sayisal => Some(Ondalik),
        (Topla, Metin, Sayi | Ondalik | Metin | Mantik)
        | (Topla, Sayi | Ondalik | Mantik, Metin) => Some(Metin),
        (TamBol | Mod, Sayi, Sayi) => Some(Sayi),
        (Kucuk | Buyuk | KucukEsit | BuyukEsit, _, _) if sayisal => Some(Mantik),
        (Esit | EsitDegil, _, _) if sayisal => Some(Mantik),
        (Esit | EsitDegil, Metin, Metin) | (Esit | EsitDegil, Mantik, Mantik) => Some(Mantik),
        (Ve | Veya, Mantik, Mantik) => Some(Mantik),
        _ => None,
    }
}

fn op_adi(op: IkiliOp) -> &'static str {
    use IkiliOp::*;
    match op {
        Topla => "+",
        Cikar => "-",
        Carp => "*",
        Bol => "/",
        TamBol => "//",
        Mod => "%",
        Esit => "eşitlik",
        EsitDegil => "eşitsizlik",
        Kucuk => "küçüktür",
        Buyuk => "büyüktür",
        KucukEsit => "küçük eşittir",
        BuyukEsit => "büyük eşittir",
        Ve => "ve",
        Veya => "veya",
    }
}
