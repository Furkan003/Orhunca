//! Anlam ve tip denetimi. Her ifadenin `tip` alanını doldurur, işlevlerin yerel
//! değişkenlerini toplar ve Türkçe hata mesajları üretir.
//!
//! Kapsam kuralı (v0.1): değişkenler işlev düzeyindedir; işlevler yalnızca kendi
//! parametrelerini ve yerel değişkenlerini görür.

use crate::agac::*;
use crate::arayuz::{self, BagTuru, Beklenen};
use crate::ayristirici::YERLESIK_DOSYA;
use crate::ekler::Hal;
use crate::hata::{Hata, Konum, Sonuc};
use crate::on_kutuphane::{self, HATA_SATIRDA, ON_EK, YERLESIK_ON_EK};
use std::collections::{HashMap, HashSet};

/// Olay bloklarında kullanıcının değiştirdiği değer (bağlı öğelerde) bu ada gelir.
pub const OLAY_DEGERI: &str = "‹değer›";

const JSON_TURU: &str = "application/json; charset=utf-8";

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
    sabitler: HashMap<String, Ifade>,
    /// Varsayılan değerleri denetlenmiş model tanımları.
    modeller: HashMap<String, Model>,
    /// Seçenek türleri ve değerleri
    secenekler: HashMap<String, Vec<String>>,
    /// Bir web yolunun gövdesi denetleniyor: `döndür` değerleri yanıta çevrilir.
    rotada: bool,
    /// `durum` değişkenleri: her yerden görülür.
    durumlar: HashMap<String, Tip>,
    /// `arayüz:` ya da bir bileşen denetleniyor: öğeler ve bileşen çağrıları yazılabilir.
    arayuzde: bool,
    bilesenler: HashSet<String>,
    /// Bir olay bloğu denetleniyorsa çevreleyen işlevin yerel değişkenleri (yakalanabilir).
    cevre: Option<HashMap<String, Tip>>,
    /// Olay bloğunun kullandığı çevre değişkenleri (sırayla).
    yakalanan: Vec<(String, Tip)>,
    /// Gövdesi henüz denetlenmemiş işlevler: ilk çağrıldıklarında denetlenir.
    bekleyen: HashMap<String, Islev>,
    /// Denetimi bitmiş işlevler
    biten: HashMap<String, Islev>,
}

pub fn denetle(p: &mut Program) -> Sonuc<()> {
    // Standart kütüphanenin Orhunca ile yazılmış parçası: yalnızca kullanılanlar kalır.
    p.islevler.extend(on_kutuphane::islevler());
    let mut d = Denetci {
        imzalar: HashMap::new(),
        varsayilan: HashMap::new(),
        kapsam: HashMap::new(),
        sira: Vec::new(),
        donus: None,
        donus_belirtildi: false,
        dongu: 0,
        islevde: false,
        sabitler: HashMap::new(),
        modeller: HashMap::new(),
        secenekler: HashMap::new(),
        rotada: false,
        durumlar: HashMap::new(),
        arayuzde: false,
        bilesenler: HashSet::new(),
        cevre: None,
        yakalanan: Vec::new(),
        bekleyen: HashMap::new(),
        biten: HashMap::new(),
    };
    for (ad, deger) in p.sabitler.iter_mut() {
        d.ifade(deger)?;
        if !matches!(
            deger.tur,
            IfadeTuru::Sayi(_) | IfadeTuru::Ondalik(_) | IfadeTuru::Metin(_) | IfadeTuru::Mantik(_)
        ) {
            return Err(Hata::yeni(
                deger.konum,
                "sabitin değeri bir sayı, ondalık, metin ya da mantık değeri olmalı",
            ));
        }
        d.sabitler.insert(ad.clone(), deger.clone());
    }
    d.sabitler.entry("pi".into()).or_insert_with(|| Ifade {
        tur: IfadeTuru::Ondalik(std::f64::consts::PI),
        konum: Konum::default(),
        tip: Tip::Ondalik,
    });
    dongusel_alanlari_isaretle(&mut p.modeller);
    for m in &p.modeller {
        d.modeller.insert(m.ad.clone(), m.clone());
    }
    for s in &p.secenekler {
        if d.modeller.contains_key(&s.ad) || d.sabitler.contains_key(&s.ad) {
            return Err(Hata::yeni(
                s.konum,
                format!(
                    "'{}' hem bir seçenek türünün hem bir model ya da sabitin adı",
                    s.ad
                ),
            ));
        }
        d.secenekler.insert(s.ad.clone(), s.degerler.clone());
    }
    for f in &p.islevler {
        if d.imzalar.contains_key(&f.ad) {
            let mesaj = match &f.rota {
                Some(r) => format!("{} {} yolu iki kez tanımlanmış", r.yontem, r.kalip),
                None => format!("'{}' işlevi birden fazla tanımlanmış", f.ad),
            };
            return Err(Hata::yeni(f.konum, mesaj));
        }
        if d.modeller.contains_key(&f.ad) || d.secenekler.contains_key(&f.ad) {
            return Err(Hata::yeni(
                f.konum,
                format!(
                    "'{}' bir modelin ya da seçenek türünün adı; işleve başka bir ad verin",
                    f.ad
                ),
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
        if f.arayuz && f.ad != ARAYUZ_ISLEVI {
            d.bilesenler.insert(f.ad.clone());
        }
    }

    let islev_sirasi: Vec<String> = p.islevler.iter().map(|f| f.ad.clone()).collect();
    let rota_var = p.islevler.iter().any(|f| f.rota.is_some());
    for f in std::mem::take(&mut p.islevler) {
        d.bekleyen.insert(f.ad.clone(), f);
    }

    // Durum değişkenleri: sırayla (öncekilere başvurabilir).
    for durum in p.durumlar.iter_mut() {
        d.kapsam.clear();
        d.sira.clear();
        d.islevde = false;
        d.durum_denetle(durum)?;
    }

    // Modellerin alanları ve varsayılan değerleri (değişken göremezler).
    for m in p.modeller.iter_mut() {
        d.kapsam.clear();
        d.sira.clear();
        d.islevde = false;
        d.model_denetle(m)?;
        d.modeller.insert(m.ad.clone(), m.clone());
    }

    // Web yolları varsa ve program sunucuyu kendisi başlatmıyorsa sonunda başlatılır.
    if rota_var && !cagri_var(&p.ana, "sun") {
        p.ana.push(Deyim::IfadeDeyimi(Ifade::yeni(
            IfadeTuru::Cagri("sun".into(), Vec::new()),
            Konum::default(),
        )));
    }

    // Önce ana program: işlevler ilk çağrıldıklarında denetlenir; tipi yazılmayan
    // parametreler ilk çağrıdaki değerin tipini alır. Hiç çağrılmayanlar sonra.
    d.kapsam.clear();
    d.sira.clear();
    d.islevde = false;
    d.arayuzde = false;
    d.rotada = false;
    d.donus = None;
    d.blok(&mut p.ana)?;
    p.ana_yereller = d.yereller();
    for ad in islev_sirasi
        .iter()
        .filter(|a| !on_kutuphane::on_kutuphane_mi(a))
    {
        d.islevi_denetle(ad)?;
    }
    // Ön kütüphanenin çağrılmayan işlevleri atılır.
    p.islevler = islev_sirasi
        .iter()
        .filter_map(|ad| d.biten.remove(ad))
        .collect();
    Ok(())
}

/// Model alanı zinciri kendi modeline geri dönüyorsa (`model Düğüm: sonraki: Düğüm`)
/// alanın varsayılanı boştur; değilse alanın modelinin varsayılan nesnesidir.
fn dongusel_alanlari_isaretle(modeller: &mut [Model]) {
    let kenarlar: HashMap<String, Vec<String>> = modeller
        .iter()
        .map(|m| {
            let hedefler = m
                .alanlar
                .iter()
                .filter_map(|a| match &a.tip {
                    Tip::Model(x) => Some(x.clone()),
                    _ => None,
                })
                .collect();
            (m.ad.clone(), hedefler)
        })
        .collect();
    let ulasir = |bas: &str, hedef: &str| {
        let mut gorulen = HashSet::new();
        let mut yigin = vec![bas.to_string()];
        while let Some(x) = yigin.pop() {
            if x == hedef {
                return true;
            }
            if gorulen.insert(x.clone()) {
                yigin.extend(kenarlar.get(&x).into_iter().flatten().cloned());
            }
        }
        false
    };
    for m in modeller.iter_mut() {
        for a in m.alanlar.iter_mut() {
            if let Tip::Model(x) = &a.tip {
                a.dongusel = ulasir(x, &m.ad);
            }
        }
    }
}

/// Bir gövdede (iç içe ifadeler dahil) `ad` adlı bir çağrı var mı?
fn cagri_var(govde: &[Deyim], ad: &str) -> bool {
    fn ifadede(e: &Ifade, ad: &str) -> bool {
        match &e.tur {
            IfadeTuru::Cagri(a, arg) => a == ad || arg.iter().any(|x| ifadede(x, ad)),
            IfadeTuru::Liste(o) | IfadeTuru::Metod(_, _, o) if o.iter().any(|x| ifadede(x, ad)) => {
                true
            }
            IfadeTuru::Metod(a, _, _) | IfadeTuru::Alan(a, _, _) | IfadeTuru::Tekli(_, a) => {
                ifadede(a, ad)
            }
            IfadeTuru::Sozluk(c) => c.iter().any(|(a, b)| ifadede(a, ad) || ifadede(b, ad)),
            IfadeTuru::Ikili(_, a, b) | IfadeTuru::Indeks(a, b) => ifadede(a, ad) || ifadede(b, ad),
            IfadeTuru::FiilCagri(a, arg) => a == ad || arg.iter().any(|(_, x)| ifadede(x, ad)),
            IfadeTuru::Kurucu(_, arg) => arg.iter().any(|(_, x)| ifadede(x, ad)),
            _ => false,
        }
    }
    govde.iter().any(|d| match d {
        Deyim::Atama { deger, .. } | Deyim::Yaz(deger) | Deyim::IfadeDeyimi(deger) => {
            ifadede(deger, ad)
        }
        Deyim::Dondur(Some(e), _) | Deyim::Sirala(e) => ifadede(e, ad),
        Deyim::Eger {
            kosul,
            govde,
            degilse,
        } => ifadede(kosul, ad) || cagri_var(govde, ad) || cagri_var(degilse, ad),
        Deyim::Surece { kosul, govde } => ifadede(kosul, ad) || cagri_var(govde, ad),
        Deyim::HerAralik { govde, .. } | Deyim::HerListe { govde, .. } => cagri_var(govde, ad),
        Deyim::Dene { govde, yakala, .. } => cagri_var(govde, ad) || cagri_var(yakala, ad),
        _ => false,
    })
}

fn alan_listesi(m: &Model) -> String {
    let adlar: Vec<String> = m
        .alanlar
        .iter()
        .map(|a| format!("{}: {}", a.ad, a.tip))
        .collect();
    format!("{} modelinin alanları: {}", m.ad, adlar.join(", "))
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
    fn model_denetle(&mut self, m: &mut Model) -> Sonuc<()> {
        if self.sabitler.contains_key(&m.ad) {
            return Err(Hata::yeni(
                m.konum,
                format!("'{}' hem bir sabitin hem bir modelin adı", m.ad),
            ));
        }
        for a in m.alanlar.iter_mut() {
            if let Some(ic) = a.tip.ic_model() {
                if self
                    .modeller
                    .get(ic)
                    .is_some_and(|x| x.konum.dosya == YERLESIK_DOSYA)
                {
                    return Err(Hata::yeni(
                        a.konum,
                        format!("'{ic}' yerleşik bir model; model alanı olamaz"),
                    ));
                }
                a.ic_model = Some(ic.to_string());
            }
            let sinirli = matches!(a.tip, Tip::Sayi | Tip::Ondalik | Tip::Metin | Tip::Liste(_));
            if (a.en_az.is_some() || a.en_fazla.is_some()) && !sinirli {
                return Err(Hata::yeni(
                    a.konum,
                    format!("en_az / en_fazla {} alanlarında kullanılamaz", a.tip),
                ));
            }
            if let Some(v) = a.varsayilan.as_mut() {
                let t = self.ifade(v)?;
                if !a.tip.kabul_eder(&t) {
                    return Err(Hata::yeni(
                        v.konum,
                        format!("'{}' alanı {} tipinde; varsayılan değer {t}", a.ad, a.tip),
                    ));
                }
                genislet(v, &a.tip);
                v.tip = a.tip.clone();
            }
            if let Tip::Secenek(s) = &a.tip {
                a.secenekler = self.secenekler.get(s).cloned().unwrap_or_default();
                if a.varsayilan.is_none() {
                    // Varsayılan: ilk değer
                    a.varsayilan = Some(Ifade {
                        tur: IfadeTuru::Metin(a.secenekler[0].clone()),
                        konum: a.konum,
                        tip: a.tip.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn model(&self, ad: &str, konum: Konum) -> Sonuc<Model> {
        self.modeller
            .get(ad)
            .cloned()
            .ok_or_else(|| Hata::yeni(konum, format!("tanımsız model '{ad}'")))
    }

    /// Yerleşik modeller (İstek, Yanıt) veri deposuna yazılamaz.
    fn depolanabilir(&self, m: &Model, konum: Konum) -> Sonuc<()> {
        if m.konum.dosya == YERLESIK_DOSYA {
            return Err(Hata::yeni(
                konum,
                format!("'{}' yerleşik bir model; kaydedilip okunamaz", m.ad),
            ));
        }
        Ok(())
    }

    /// Denetlenmiş alan değerlerini modelin tüm alanlarına tamamlar (sırayla).
    fn kurucu_tamamla(m: &Model, mut verilen: Vec<(String, Ifade)>) -> Vec<(String, Ifade)> {
        m.alanlar
            .iter()
            .map(|a| match verilen.iter().position(|(ad, _)| *ad == a.ad) {
                Some(i) => verilen.remove(i),
                None => (a.ad.clone(), a.ilk_deger()),
            })
            .collect()
    }

    /// `döndür` değeri yanıt değilse yanıta çevrilir: metin → HTML, diğerleri → JSON.
    fn yanita_cevir(&mut self, e: &mut Ifade, t: Tip) -> Sonuc<Tip> {
        let yanit = Tip::Model("Yanıt".into());
        if t == yanit {
            return Ok(t);
        }
        if t == Tip::Bos {
            return Err(Hata::yeni(e.konum, "bu işlev bir değer döndürmüyor"));
        }
        let k = e.konum;
        let ic = std::mem::replace(e, Ifade::yeni(IfadeTuru::Mantik(false), k));
        let alanlar = if t == Tip::Metin {
            vec![("gövde".to_string(), ic)]
        } else {
            json_alanlari(ic)
        };
        let m = self.model("Yanıt", k)?;
        *e = Ifade {
            tur: IfadeTuru::Kurucu("Yanıt".into(), Self::kurucu_tamamla(&m, alanlar)),
            konum: k,
            tip: yanit.clone(),
        };
        Ok(yanit)
    }

    /// `yanıt(404, "...")`, `yönlendir("/")`, `json_yanıtı(x)`: Yanıt kurucusuna çevrilir.
    fn yanit_sekeri(&mut self, e: &mut Ifade) -> Sonuc<Tip> {
        let konum = e.konum;
        let IfadeTuru::Cagri(ad, arg) = &mut e.tur else {
            unreachable!()
        };
        let ad = ad.clone();
        let mut tipler = Vec::new();
        for a in arg.iter_mut() {
            tipler.push(self.ifade(a)?);
        }
        let mut arg = std::mem::take(arg);
        let sayi = |n: i64| Ifade {
            tur: IfadeTuru::Sayi(n),
            konum,
            tip: Tip::Sayi,
        };
        use Tip::*;
        let alanlar = match (ad.as_str(), tipler.as_slice()) {
            ("yanıt", [Sayi, Metin]) | ("yanıt", [Sayi, Metin, Metin]) => {
                let mut v = vec![
                    ("durum".to_string(), arg.remove(0)),
                    ("gövde".to_string(), arg.remove(0)),
                ];
                if let Some(t) = arg.pop() {
                    v.push(("tür".to_string(), t));
                }
                v
            }
            ("yönlendir", [Metin]) => vec![
                ("durum".to_string(), sayi(303)),
                ("konum".to_string(), arg.remove(0)),
            ],
            ("json_yanıtı", [x]) | ("json_yanıtı", [x, Sayi]) if *x != Bos => {
                let deger = arg.remove(0);
                let mut v = json_alanlari(deger);
                if let Some(d) = arg.pop() {
                    v.push(("durum".to_string(), d));
                }
                v
            }
            _ => {
                let verilen: Vec<String> = tipler.iter().map(|t| t.to_string()).collect();
                let kullanim = crate::yerlesik::bul(&ad)
                    .map(|y| y.kullanim)
                    .unwrap_or_default();
                return Err(Hata::yeni(
                    konum,
                    format!(
                        "'{ad}' bu bağımsız değişkenlerle kullanılamaz ({})",
                        verilen.join(", ")
                    ),
                )
                .ipucu(format!("kullanım: {kullanim}")));
            }
        };
        let m = self.model("Yanıt", konum)?;
        e.tur = IfadeTuru::Kurucu("Yanıt".into(), Self::kurucu_tamamla(&m, alanlar));
        e.tip = Model("Yanıt".into());
        Ok(e.tip.clone())
    }

    /// `görünüm("ürünler", x)` → `görünüm:ürünler(x)`
    fn gorunum_cagrisi(&mut self, e: &mut Ifade) -> Sonuc<()> {
        let konum = e.konum;
        let IfadeTuru::Cagri(ad, arg) = &mut e.tur else {
            unreachable!()
        };
        let Some(IfadeTuru::Metin(g)) = arg.first().map(|a| &a.tur) else {
            return Err(Hata::yeni(
                konum,
                "görünüm(...) ilk değer olarak görünümün adını metin olarak alır",
            )
            .ipucu("görünüm(\"ürünler\", ürünler)"));
        };
        let hedef = format!("görünüm:{g}");
        if !self.imzalar.contains_key(&hedef) {
            return Err(self.gorunum_yok(g, arg[0].konum));
        }
        arg.remove(0);
        *ad = hedef;
        Ok(())
    }

    fn gorunum_yok(&self, g: &str, konum: Konum) -> Hata {
        let mut var: Vec<&str> = self
            .imzalar
            .keys()
            .filter_map(|a| a.strip_prefix("görünüm:"))
            .collect();
        var.sort();
        let ipucu = if var.is_empty() {
            format!("görünümler/{g}.ohchtml dosyasını oluşturun")
        } else {
            format!(
                "görünümler/{g}.ohchtml dosyasını oluşturun; var olanlar: {}",
                var.join(", ")
            )
        };
        Hata::yeni(konum, format!("'{g}' görünümü bulunamadı")).ipucu(ipucu)
    }

    /// Seçenek türü kullanımları: `Renk.kırmızı` (değer), `Renk.hepsi()` (tüm
    /// değerler) ve `Renk(m)` (metinden çevirme, geçersizse çalışma hatası).
    fn secenek_ifadesi(&mut self, e: &mut Ifade) -> Sonuc<Option<Tip>> {
        let konum = e.konum;
        let metin = |m: &str, tip: Tip| Ifade {
            tur: IfadeTuru::Metin(m.to_string()),
            konum,
            tip,
        };
        let yeni = match &mut e.tur {
            IfadeTuru::Alan(n, ad, _) => {
                let IfadeTuru::ModelAdi(s) = &n.tur else {
                    return Ok(None);
                };
                let Some(degerler) = self.secenekler.get(s) else {
                    return Ok(None);
                };
                if !degerler.contains(ad) {
                    return Err(Hata::yeni(
                        konum,
                        format!("'{s}' türünde '{ad}' diye bir değer yok"),
                    )
                    .ipucu(format!("değerler: {}", degerler.join(", "))));
                }
                let t = Tip::Secenek(s.clone());
                (IfadeTuru::Metin(ad.clone()), t)
            }
            IfadeTuru::Metod(n, ad, arg) => {
                let IfadeTuru::ModelAdi(s) = &n.tur else {
                    return Ok(None);
                };
                let Some(degerler) = self.secenekler.get(s) else {
                    return Ok(None);
                };
                if ad != "hepsi" || !arg.is_empty() {
                    return Err(Hata::yeni(
                        konum,
                        format!("'{s}' bir seçenek türü; yalnızca {s}.hepsi() çağrılabilir"),
                    ));
                }
                let t = Tip::Secenek(s.clone());
                let ogeler = degerler.iter().map(|d| metin(d, t.clone())).collect();
                (IfadeTuru::Liste(ogeler), Tip::Liste(Box::new(t)))
            }
            IfadeTuru::Cagri(ad, arg)
                if self.secenekler.contains_key(ad.as_str())
                    && !self.imzalar.contains_key(ad.as_str()) =>
            {
                let s = ad.clone();
                let t = Tip::Secenek(s.clone());
                if arg.len() != 1 {
                    return Err(Hata::yeni(konum, format!("{s}(m) tek bir metin alır")));
                }
                let mut a = arg.remove(0);
                let at = self.ifade(&mut a)?;
                if at != Tip::Metin && at != t {
                    return Err(Hata::yeni(
                        a.konum,
                        format!("{s}(...) metin bekler, {at} verildi"),
                    ));
                }
                let degerler = self.secenekler[&s].join("|");
                let args = vec![a, metin(&degerler, Tip::Metin), metin(&s, Tip::Metin)];
                (IfadeTuru::Cagri(SECENEK_CEVIR.into(), args), t)
            }
            // Yeniden denetlenen çevirme
            IfadeTuru::Cagri(ad, arg) if ad == SECENEK_CEVIR => match &arg[2].tur {
                IfadeTuru::Metin(s) => return Ok(Some(Tip::Secenek(s.clone()))),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        e.tur = yeni.0;
        Ok(Some(yeni.1))
    }

    /// İşlevin gövdesini denetler (henüz denetlenmemişse). Bir çağrının içinden
    /// çağrılabilir: çağıranın denetim durumu saklanıp geri yüklenir.
    fn islevi_denetle(&mut self, ad: &str) -> Sonuc<()> {
        let Some(mut f) = self.bekleyen.remove(ad) else {
            return Ok(());
        };
        let eski = (
            std::mem::take(&mut self.kapsam),
            std::mem::take(&mut self.sira),
            self.islevde,
            self.arayuzde,
            self.rotada,
            self.donus_belirtildi,
            self.donus.take(),
            self.dongu,
            self.cevre.take(),
            std::mem::take(&mut self.yakalanan),
        );
        self.dongu = 0;
        let sonuc = self.islev_govdesi(&mut f);
        (
            self.kapsam,
            self.sira,
            self.islevde,
            self.arayuzde,
            self.rotada,
            self.donus_belirtildi,
            self.donus,
            self.dongu,
            self.cevre,
            self.yakalanan,
        ) = eski;
        self.biten.insert(ad.to_string(), f);
        sonuc
    }

    fn islev_govdesi(&mut self, f: &mut Islev) -> Sonuc<()> {
        // Tipi yazılmayan parametreler: ilk çağrıdan çıkarılan tip, yoksa sayı.
        let imza = self.imzalar.get_mut(&f.ad).expect("işlevin imzası");
        for (i, (_, tip)) in f.parametreler.iter_mut().enumerate() {
            if *tip == Tip::Bilinmeyen {
                *tip = match &imza.parametreler[i] {
                    Tip::Bilinmeyen => Tip::Sayi,
                    t => t.clone(),
                };
                imza.parametreler[i] = tip.clone();
            }
        }
        self.kapsam.clear();
        self.sira.clear();
        self.islevde = true;
        self.arayuzde = f.arayuz;
        self.rotada = f.rota.is_some();
        self.donus_belirtildi = f.donus.is_some();
        self.donus = f.donus.clone();
        for (ad, tip) in &f.parametreler {
            if self.kapsam.contains_key(ad) {
                return Err(Hata::yeni(
                    f.konum,
                    format!("'{ad}' parametresi iki kez yazılmış"),
                ));
            }
            self.tanimla(ad, tip.clone());
        }
        self.blok(&mut f.govde)?;
        let donus = self.donus.clone().unwrap_or(Tip::Bos);
        f.donus = Some(donus.clone());
        self.imzalar.get_mut(&f.ad).unwrap().donus = Some(donus.clone());
        if let Some(k) = self.varsayilan.get(&f.ad) {
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
        f.yereller = self.yereller();
        Ok(())
    }

    fn metod(&mut self, e: &mut Ifade) -> Sonuc<Tip> {
        let konum = e.konum;
        let IfadeTuru::Metod(alici, ad, arg) = &mut e.tur else {
            unreachable!()
        };
        let ad = ad.clone();
        let mut tipler = Vec::new();
        for a in arg.iter_mut() {
            tipler.push(self.ifade(a)?);
        }
        use Tip::*;
        if let IfadeTuru::ModelAdi(m) = &alici.tur {
            let model = self.model(m, alici.konum)?;
            alici.tip = Model(model.ad.clone());
            self.depolanabilir(&model, konum)?;
            let mt = Model(model.ad.clone());
            return Ok(match (ad.as_str(), tipler.as_slice()) {
                ("hepsi", []) => Liste(Box::new(mt)),
                ("bul", [Sayi]) => mt,
                ("var_mı" | "sil", [Sayi]) => Mantik,
                ("formdan", [Model(i)]) if i == "İstek" => mt,
                _ => {
                    let bilinen =
                        ["hepsi", "bul", "var_mı", "sil", "formdan"].contains(&ad.as_str());
                    let mesaj = if bilinen {
                        format!("'{}.{ad}' bu bağımsız değişkenlerle kullanılamaz", model.ad)
                    } else {
                        format!("'{}' modelinin '{ad}' diye bir yöntemi yok", model.ad)
                    };
                    return Err(Hata::yeni(konum, mesaj).ipucu(format!(
                        "yöntemler: {m}.hepsi(), {m}.bul(kimlik), {m}.var_mı(kimlik), {m}.sil(kimlik), {m}.formdan(istek)",
                        m = model.ad
                    )));
                }
            });
        }
        let t = self.ifade(alici)?;
        let Model(m) = &t else {
            return Err(Hata::yeni(
                konum,
                format!("yöntem çağrısı yalnızca model nesnelerinde kullanılabilir ({t} bulundu)"),
            )
            .ipucu(format!("işlevleri parantezle çağırın: {ad}(x)")));
        };
        let model = self.model(m, konum)?;
        Ok(match (ad.as_str(), tipler.as_slice()) {
            ("kaydet", []) => {
                self.depolanabilir(&model, konum)?;
                Sayi
            }
            ("sil", []) => {
                self.depolanabilir(&model, konum)?;
                Mantik
            }
            ("geçerli_mi", []) => Mantik,
            ("hatalar", []) => Liste(Box::new(Metin)),
            ("json", []) => Metin,
            _ => {
                return Err(Hata::yeni(
                    konum,
                    format!("'{m}' nesnelerinin '{ad}' diye bir yöntemi yok"),
                )
                .ipucu("yöntemler: kaydet(), sil(), geçerli_mi(), hatalar(), json()"))
            }
        })
    }

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
        self.daralt(liste, &Tip::Liste(Box::new(oge.clone())));
    }

    /// `l = []` ya da `s = {}` sonrasında değişkenin tipini öğrenilen tiple daraltır.
    fn daralt(&mut self, ifade: &Ifade, tip: &Tip) {
        if let IfadeTuru::Isim(ad) = &ifade.tur {
            if let Some(eski) = self.kapsam.get(ad).cloned() {
                if let Some(t) = eski.birlestir(tip) {
                    self.kapsam.insert(ad.clone(), t);
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
                tip,
                deger,
                konum,
            } => {
                let mut t = self.ifade(deger)?;
                if self.kapsam.get(hedef.as_str()) == Some(&Tip::Ondalik) {
                    genislet(deger, &Tip::Ondalik);
                    t = deger.tip.clone();
                }
                // `ad: tip = değer`: değişken yazılan tiple tanımlanır.
                if let Some(bildirilen) = tip {
                    if !bildirilen.kabul_eder(&t) {
                        return Err(Hata::yeni(
                            deger.konum,
                            format!("'{hedef}' {bildirilen} olarak tanımlandı; değeri {t}"),
                        ));
                    }
                    genislet(deger, bildirilen);
                    t = bildirilen.clone();
                    deger.tip = t.clone();
                }
                if self.sabitler.contains_key(hedef.as_str()) {
                    return Err(Hata::yeni(
                        *konum,
                        format!("'{hedef}' bir sabit, değeri değiştirilemez"),
                    ));
                }
                if self.imzalar.contains_key(hedef.as_str()) {
                    return Err(Hata::yeni(
                        *konum,
                        format!("'{hedef}' bir işlevin adı, değişken olarak kullanılamaz"),
                    ));
                }
                if self.modeller.contains_key(hedef.as_str()) {
                    return Err(Hata::yeni(
                        *konum,
                        format!("'{hedef}' bir modelin adı, değişken olarak kullanılamaz"),
                    ));
                }
                if !self.kapsam.contains_key(hedef.as_str()) {
                    if self
                        .cevre
                        .as_ref()
                        .is_some_and(|c| c.contains_key(hedef.as_str()))
                    {
                        return Err(Hata::yeni(
                            *konum,
                            format!(
                                "olay bloğunda '{hedef}' değiştirilemez: öğe çizilirken alınmış bir kopyadır"
                            ),
                        )
                        .ipucu("kalıcı değerler için durum değişkeni kullanın: durum ad = ..."));
                    }
                    if let Some(dt) = self.durumlar.get(hedef.as_str()).cloned() {
                        if !dt.kabul_eder(&t) {
                            return Err(Hata::yeni(
                                *konum,
                                format!("'{hedef}' durumu {dt} tipinde; {t} değer atanamaz"),
                            ));
                        }
                        genislet(deger, &dt);
                        return Ok(());
                    }
                }
                self.ata(hedef, t, *konum)?;
            }
            Deyim::AlanAtama {
                nesne,
                alan,
                sira,
                deger,
                konum,
            } => {
                let t = self.ifade(nesne)?;
                let Tip::Model(m) = &t else {
                    return Err(Hata::yeni(
                        *konum,
                        format!(
                            "'.{alan}' yalnızca model nesnelerinde kullanılabilir ({t} bulundu)"
                        ),
                    ));
                };
                let model = self.model(m, *konum)?;
                let Some((i, a)) = model.alan(alan) else {
                    return Err(
                        Hata::yeni(*konum, format!("'{m}' modelinde '{alan}' alanı yok"))
                            .ipucu(alan_listesi(&model)),
                    );
                };
                *sira = i;
                let dt = self.ifade(deger)?;
                if !a.tip.kabul_eder(&dt) {
                    return Err(Hata::yeni(
                        deger.konum,
                        format!("'{alan}' alanı {} tipinde; {dt} atanamaz", a.tip),
                    ));
                }
                genislet(deger, &a.tip);
            }
            Deyim::IndeksAtama {
                liste,
                indeks,
                deger,
            } => match self.ifade(liste)? {
                Tip::Sozluk(a, d) => {
                    let kt = self.ifade(indeks)?;
                    if !a.kabul_eder(&kt) {
                        return Err(Hata::yeni(
                            indeks.konum,
                            format!("anahtar {a} olmalı, {kt} bulundu"),
                        ));
                    }
                    if !matches!(kt, Tip::Sayi | Tip::Metin | Tip::Secenek(_)) {
                        return Err(Hata::yeni(
                            indeks.konum,
                            "sözlük anahtarları sayı, metin ya da seçenek olmalı",
                        ));
                    }
                    let t = self.ifade(deger)?;
                    if !d.kabul_eder(&t) {
                        return Err(Hata::yeni(
                            deger.konum,
                            format!("sözlüğün değerleri {d} tipinde; {t} konamaz"),
                        ));
                    }
                    genislet(deger, &d);
                    let yeni = Tip::Sozluk(Box::new(kt), Box::new(deger.tip.clone()));
                    self.daralt(liste, &yeni);
                    liste.tip = liste.tip.birlestir(&yeni).unwrap_or(yeni);
                }
                Tip::Metin => {
                    return Err(Hata::yeni(
                        liste.konum,
                        "metinler değiştirilemez; yeni bir metin oluşturun",
                    )
                    .ipucu("değiştir(m, eski, yeni) ya da parça(...) kullanın"))
                }
                Tip::Liste(ic) => {
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
                t => {
                    return Err(Hata::yeni(
                        liste.konum,
                        format!("liste ya da sözlük bekleniyordu, {t} bulundu"),
                    ))
                }
            },
            Deyim::Cikar { oge, liste } => {
                let t = self.ifade(oge)?;
                let ic = self.liste_tipi(liste)?;
                if !ic.kabul_eder(&t) {
                    return Err(Hata::yeni(
                        oge.konum,
                        format!("liste<{ic}> listesinde {t} aranamaz"),
                    ));
                }
                genislet(oge, &ic);
            }
            Deyim::DosyayaYaz { deger, yol } => {
                if self.ifade(deger)? == Tip::Bos {
                    return Err(Hata::yeni(deger.konum, "bu işlev bir değer döndürmüyor"));
                }
                let t = self.ifade(yol)?;
                if t != Tip::Metin {
                    return Err(Hata::yeni(
                        yol.konum,
                        format!("dosya yolu metin olmalı, {t} bulundu"),
                    )
                    .ipucu("ekrana yazmak için: x'i ekrana yaz."));
                }
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
                let ic = match self.ifade(liste)? {
                    Tip::Liste(ic) | Tip::Sozluk(ic, _) => *ic,
                    Tip::Metin => Tip::Metin,
                    t => {
                        return Err(Hata::yeni(
                            liste.konum,
                            format!("liste, metin ya da sözlük bekleniyordu, {t} bulundu"),
                        ))
                    }
                };
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
                if self.rotada {
                    if let Some(i) = deger.as_mut() {
                        t = self.yanita_cevir(i, t)?;
                    }
                }
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
            Deyim::Oge(o) => self.oge(o)?,
            Deyim::Dene {
                govde,
                degisken,
                yakala,
                konum,
            } => {
                self.blok(govde)?;
                if let Some(d) = degisken {
                    if self.durumlar.contains_key(d) && !self.kapsam.contains_key(d) {
                        return Err(Hata::yeni(
                            *konum,
                            format!("'{d}' bir durum değişkeni; hata için başka bir ad verin"),
                        ));
                    }
                    self.ata(d, Tip::Metin, *konum)?;
                }
                self.blok(yakala)?;
            }
        }
        Ok(())
    }

    fn durum_denetle(&mut self, d: &mut Durum) -> Sonuc<()> {
        let ad = d.ad.as_str();
        let cakisma = if self.sabitler.contains_key(ad) {
            Some("bir sabitin")
        } else if self.imzalar.contains_key(ad) {
            Some("bir işlevin")
        } else if self.modeller.contains_key(ad) {
            Some("bir modelin")
        } else {
            None
        };
        if let Some(ne) = cakisma {
            return Err(Hata::yeni(
                d.konum,
                format!("'{ad}' {ne} adı; durum başka bir ad almalı"),
            ));
        }
        let t = self.ifade(&mut d.deger)?;
        let tip = match &d.tip {
            Some(beklenen) => {
                if !beklenen.kabul_eder(&t) {
                    return Err(Hata::yeni(
                        d.deger.konum,
                        format!("'{ad}' durumu {beklenen} tipinde; ilk değeri {t}"),
                    ));
                }
                genislet(&mut d.deger, beklenen);
                d.deger.tip = beklenen.clone();
                beklenen.clone()
            }
            None => t,
        };
        if tip == Tip::Bos {
            return Err(Hata::yeni(d.deger.konum, "bu işlev bir değer döndürmüyor"));
        }
        if tip_belirsiz(&tip) {
            return Err(
                Hata::yeni(d.konum, format!("'{ad}' durumunun tipi belirsiz"))
                    .ipucu(format!("tipini yazın: durum {ad}: liste<metin> = []")),
            );
        }
        self.durumlar.insert(d.ad.clone(), tip);
        Ok(())
    }

    /// Arayüz öğesi: değerler, seçenekler, bağlama, olay ve içindeki öğeler.
    fn oge(&mut self, o: &mut Oge) -> Sonuc<()> {
        if !self.arayuzde {
            return Err(Hata::yeni(
                o.konum,
                format!(
                    "'{}' öğesi yalnızca arayüz: bloğunda ya da bir bileşende kullanılabilir",
                    o.ad
                ),
            ));
        }
        let tanim = arayuz::oge(&o.ad).expect("ayrıştırıcı öğeyi tanıdı");
        let n = o.argumanlar.len();
        if n < tanim.zorunlu || n > tanim.degerler.len() {
            let beklenen = if tanim.zorunlu == tanim.degerler.len() {
                format!("{} değer", tanim.zorunlu)
            } else {
                format!("{}–{} değer", tanim.zorunlu, tanim.degerler.len())
            };
            return Err(
                Hata::yeni(o.konum, format!("'{}' {beklenen} alır, {n} verildi", o.ad))
                    .ipucu(format!("örnek: {}", tanim.ornek)),
            );
        }
        let mut bag = None;
        for (i, a) in o.argumanlar.iter_mut().enumerate() {
            let (ne, beklenen) = tanim.degerler[i];
            let t = self.ifade(a)?;
            let uygun = match beklenen {
                Beklenen::Herhangi => t != Tip::Bos,
                Beklenen::Metin => t == Tip::Metin,
                Beklenen::Sayi => t == Tip::Sayi,
                Beklenen::Sayisal => t.sayisal(),
                Beklenen::MetinListesi => {
                    Tip::Liste(Box::new(Tip::Metin)).kabul_eder(&t)
                        || matches!(&t, Tip::Liste(ic) if matches!(**ic, Tip::Secenek(_)))
                }
                Beklenen::Bag(tur) => {
                    self.bag_hedefi(a, &o.ad)?;
                    bag = Some(tur);
                    match tur {
                        BagTuru::Yazi => {
                            matches!(t, Tip::Metin | Tip::Sayi | Tip::Ondalik | Tip::Secenek(_))
                        }
                        BagTuru::Metin => t == Tip::Metin,
                        BagTuru::Mantik => t == Tip::Mantik,
                        BagTuru::Sayisal => t.sayisal(),
                    }
                }
            };
            if !uygun {
                let istenen = match beklenen {
                    Beklenen::Herhangi => "bir değer".to_string(),
                    Beklenen::Metin => "metin".into(),
                    Beklenen::Sayi => "sayı".into(),
                    Beklenen::Sayisal => "sayı ya da ondalık".into(),
                    Beklenen::MetinListesi => "liste<metin>".into(),
                    Beklenen::Bag(BagTuru::Yazi) => "metin, sayı ya da ondalık".into(),
                    Beklenen::Bag(BagTuru::Metin) => "metin".into(),
                    Beklenen::Bag(BagTuru::Mantik) => "mantık".into(),
                    Beklenen::Bag(BagTuru::Sayisal) => "sayı ya da ondalık".into(),
                };
                return Err(Hata::yeni(
                    a.konum,
                    format!(
                        "'{}' öğesinin '{ne}' değeri {istenen} olmalı, {t} verildi",
                        o.ad
                    ),
                )
                .ipucu(format!("örnek: {}", tanim.ornek)));
            }
        }
        for (ad, d) in o.secenekler.iter_mut() {
            let Some((beklenen, _)) = arayuz::secenek(ad) else {
                let adlar: Vec<&str> = arayuz::SECENEKLER.iter().map(|s| s.0).collect();
                return Err(Hata::yeni(d.konum, format!("'{ad}' diye bir seçenek yok"))
                    .ipucu(format!("seçenekler: {}", adlar.join(", "))));
            };
            let t = self.ifade(d)?;
            let uygun = if arayuz::MANTIK_SECENEKLERI.contains(&ad.as_str()) {
                t == Tip::Mantik
            } else {
                match beklenen {
                    Beklenen::Metin => t == Tip::Metin,
                    Beklenen::Sayisal => t.sayisal(),
                    _ => matches!(t, Tip::Metin | Tip::Sayi | Tip::Ondalik | Tip::Mantik),
                }
            };
            if !uygun {
                return Err(Hata::yeni(
                    d.konum,
                    format!("'{ad}' seçeneğine {t} verilemez"),
                ));
            }
        }
        if !tanim.kapsayici && !o.cocuklar.is_empty() {
            let mut h = Hata::yeni(o.konum, format!("'{}' içine öğe alamaz", o.ad));
            if let Some(olay) = tanim.olaylar.first() {
                h = h.ipucu(format!("bir olay için: {}(...) {olay}:", o.ad));
            }
            return Err(h);
        }
        if let Some(olay) = &o.olay {
            if !tanim.olaylar.contains(&olay.ad.as_str()) {
                let mut h = Hata::yeni(
                    olay.konum,
                    format!("'{}' öğesinin '{}' olayı yok", o.ad, olay.ad),
                );
                if !tanim.olaylar.is_empty() {
                    h = h.ipucu(format!("olayları: {}", tanim.olaylar.join(", ")));
                }
                return Err(h);
            }
        } else if o.ad == "zamanlayıcı" {
            return Err(
                Hata::yeni(o.konum, "zamanlayıcı bir 'çalınca:' bloğu ister")
                    .ipucu("zamanlayıcı(1) çalınca:"),
            );
        }
        // Bağlı öğe: kullanıcı değeri değiştirince değişken güncellenir, sonra `değişince:` çalışır.
        if let Some(tur) = bag {
            let hedef = o.argumanlar[0].clone();
            let mut govde = vec![bag_atamasi(hedef, tur)];
            if let Some(olay) = o.olay.take_if(|o| o.ad == "değişince") {
                govde.extend(olay.govde);
            }
            let mut b = Olay {
                ad: "bağ".into(),
                govde,
                yakalananlar: Vec::new(),
                yereller: Vec::new(),
                konum: o.konum,
            };
            self.olay_denetle(&mut b)?;
            o.baglama = Some(b);
        }
        if let Some(olay) = o.olay.as_mut() {
            self.olay_denetle(olay)?;
        }
        self.blok(&mut o.cocuklar)
    }

    /// Bağlanan değer bir durum değişkeni, liste öğesi ya da model alanı olmalı.
    fn bag_hedefi(&self, a: &Ifade, oge: &str) -> Sonuc<()> {
        match &a.tur {
            IfadeTuru::Isim(ad) if self.kapsam.contains_key(ad.as_str()) => Err(Hata::yeni(
                a.konum,
                format!(
                    "'{oge}' bir durum değişkenine bağlanmalı; '{ad}' her çizimde yeniden hesaplanan yerel bir değişken"
                ),
            )
            .ipucu(format!("programın başında tanımlayın: durum {ad} = ..."))),
            IfadeTuru::Isim(_) | IfadeTuru::Alan(..) | IfadeTuru::Indeks(..) => Ok(()),
            _ => Err(Hata::yeni(
                a.konum,
                format!("'{oge}' öğesinin değeri bir durum değişkeni olmalı (kullanıcı değiştirince güncellenir)"),
            )
            .ipucu(format!("durum ad = \"\"  …  {oge}(ad)"))),
        }
    }

    /// Olay bloğu: çevredeki yerel değişkenler yalnızca okunabilir ve yakalanır;
    /// blok kendi yerel değişkenlerini tanımlayabilir, durumları değiştirebilir.
    fn olay_denetle(&mut self, olay: &mut Olay) -> Sonuc<()> {
        let cevre = self.kapsam.clone();
        let eski_kapsam = std::mem::take(&mut self.kapsam);
        let eski_sira = std::mem::take(&mut self.sira);
        let eski_cevre = self.cevre.replace(cevre);
        let eski_yakalanan = std::mem::take(&mut self.yakalanan);
        let eski = (
            self.arayuzde,
            self.islevde,
            self.donus.take(),
            self.donus_belirtildi,
            self.dongu,
        );
        self.arayuzde = false;
        self.islevde = true;
        self.donus = Some(Tip::Bos);
        self.donus_belirtildi = true;
        self.dongu = 0;
        self.tanimla(OLAY_DEGERI, Tip::Metin);
        let sonuc = self.blok(&mut olay.govde);
        olay.yakalananlar = std::mem::replace(&mut self.yakalanan, eski_yakalanan);
        olay.yereller = self.yereller();
        self.kapsam = eski_kapsam;
        self.sira = eski_sira;
        self.cevre = eski_cevre;
        (
            self.arayuzde,
            self.islevde,
            self.donus,
            self.donus_belirtildi,
            self.dongu,
        ) = eski;
        sonuc
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
        if let IfadeTuru::Cagri(ad, _) = &e.tur {
            if !self.imzalar.contains_key(ad.as_str()) {
                match ad.as_str() {
                    "yanıt" | "yönlendir" | "json_yanıtı" => return self.yanit_sekeri(e),
                    "görünüm" => self.gorunum_cagrisi(e)?,
                    _ => {}
                }
            }
        }
        if let Some(t) = self.secenek_ifadesi(e)? {
            e.tip = t.clone();
            return Ok(t);
        }
        if let IfadeTuru::Metod(..) = &e.tur {
            let t = self.metod(e)?;
            e.tip = t.clone();
            return Ok(t);
        }
        let tip = match &mut e.tur {
            IfadeTuru::Sayi(_) => Tip::Sayi,
            IfadeTuru::Ondalik(_) => Tip::Ondalik,
            IfadeTuru::Metin(_) => Tip::Metin,
            IfadeTuru::Mantik(_) => Tip::Mantik,
            IfadeTuru::Isim(ad) => match self.kapsam.get(ad.as_str()) {
                Some(t) => t.clone(),
                None if self
                    .cevre
                    .as_ref()
                    .is_some_and(|c| c.contains_key(ad.as_str())) =>
                {
                    // Olay bloğu çevredeki yerel değişkeni kullanıyor: çizim anındaki değeri yakalanır.
                    let t = self.cevre.as_ref().unwrap()[ad.as_str()].clone();
                    if !self.yakalanan.iter().any(|(a, _)| a == ad) {
                        self.yakalanan.push((ad.clone(), t.clone()));
                    }
                    t
                }
                None if self.durumlar.contains_key(ad.as_str()) => {
                    self.durumlar[ad.as_str()].clone()
                }
                None if self.sabitler.contains_key(ad.as_str()) => {
                    let deger = self.sabitler[ad.as_str()].clone();
                    e.tur = deger.tur;
                    e.tip = deger.tip.clone();
                    return Ok(deger.tip);
                }
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
            IfadeTuru::Indeks(l, i) => match self.ifade(l)? {
                Tip::Liste(ic) => {
                    self.sayi_bekle(i, "indeks")?;
                    if *ic == Tip::Bilinmeyen {
                        Tip::Sayi
                    } else {
                        *ic
                    }
                }
                Tip::Metin => {
                    self.sayi_bekle(i, "indeks")?;
                    Tip::Metin
                }
                Tip::Sozluk(a, d) => {
                    let kt = self.ifade(i)?;
                    if !a.kabul_eder(&kt) {
                        return Err(Hata::yeni(
                            i.konum,
                            format!("anahtar {a} olmalı, {kt} bulundu"),
                        ));
                    }
                    if *d == Tip::Bilinmeyen {
                        Tip::Sayi
                    } else {
                        *d
                    }
                }
                t => {
                    return Err(Hata::yeni(
                        l.konum,
                        format!("liste, metin ya da sözlük bekleniyordu, {t} bulundu"),
                    ))
                }
            },
            IfadeTuru::Sozluk(ciftler) => {
                let (mut a, mut d) = (Tip::Bilinmeyen, Tip::Bilinmeyen);
                for (k, v) in ciftler.iter_mut() {
                    let kt = self.ifade(k)?;
                    if !matches!(kt, Tip::Sayi | Tip::Metin | Tip::Secenek(_)) {
                        return Err(Hata::yeni(
                            k.konum,
                            format!(
                                "sözlük anahtarları sayı, metin ya da seçenek olmalı, {kt} bulundu"
                            ),
                        ));
                    }
                    a = a.birlestir(&kt).ok_or_else(|| {
                        Hata::yeni(k.konum, "sözlüğün tüm anahtarları aynı tipte olmalı")
                    })?;
                    let vt = self.ifade(v)?;
                    if d.sayisal() && vt.sayisal() && d != vt {
                        d = Tip::Ondalik;
                        continue;
                    }
                    d = d.birlestir(&vt).ok_or_else(|| {
                        Hata::yeni(
                            v.konum,
                            format!("sözlüğün tüm değerleri aynı tipte olmalı ({d} ve {vt})"),
                        )
                    })?;
                }
                for (_, v) in ciftler.iter_mut() {
                    genislet(v, &d);
                }
                Tip::Sozluk(Box::new(a), Box::new(d))
            }
            IfadeTuru::Cagri(ad, arg) => self.cagri(ad, arg, konum)?,
            IfadeTuru::FiilCagri(ad, arg) => {
                let ad = ad.clone();
                let sirali = self.fiil_eslestir(&ad, std::mem::take(arg), konum)?;
                e.tur = IfadeTuru::Cagri(ad, sirali);
                return self.ifade(e);
            }
            IfadeTuru::Kurucu(m, arg) => {
                let model = self.model(m, konum)?;
                for (ad, d) in arg.iter_mut() {
                    let Some((_, alan)) = model.alan(ad) else {
                        return Err(Hata::yeni(
                            d.konum,
                            format!("'{}' modelinde '{ad}' alanı yok", model.ad),
                        )
                        .ipucu(alan_listesi(&model)));
                    };
                    let t = self.ifade(d)?;
                    if !alan.tip.kabul_eder(&t) {
                        return Err(Hata::yeni(
                            d.konum,
                            format!("'{ad}' alanı {} tipinde; {t} verilemez", alan.tip),
                        ));
                    }
                    genislet(d, &alan.tip);
                }
                *arg = Self::kurucu_tamamla(&model, std::mem::take(arg));
                Tip::Model(model.ad)
            }
            IfadeTuru::Alan(n, ad, sira) => {
                if let IfadeTuru::ModelAdi(m) = &n.tur {
                    return Err(Hata::yeni(
                        konum,
                        format!("'{m}' bir model adı; alanlar nesnelerden okunur"),
                    )
                    .ipucu(format!(
                        "yöntem çağırın: {m}.hepsi()  ya da  {m}.bul(1).{ad}"
                    )));
                }
                let t = self.ifade(n)?;
                let Tip::Model(m) = &t else {
                    return Err(Hata::yeni(
                        konum,
                        format!("'.{ad}' yalnızca model nesnelerinde kullanılabilir ({t} bulundu)"),
                    ));
                };
                let model = self.model(m, konum)?;
                let Some((i, alan)) = model.alan(ad) else {
                    return Err(
                        Hata::yeni(konum, format!("'{m}' modelinde '{ad}' alanı yok"))
                            .ipucu(alan_listesi(&model)),
                    );
                };
                *sira = i;
                alan.tip.clone()
            }
            IfadeTuru::ModelAdi(m) => {
                return Err(Hata::yeni(
                    konum,
                    format!("'{m}' bir model adı; değer olarak kullanılamaz"),
                ))
            }
            IfadeTuru::Metod(..) => unreachable!(),
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

    /// Çağrıyı denetler. Ön kütüphanedeki karşılığı olan yerleşik çağrının adı
    /// (ve gerekirse bağımsız değişkenleri) o işleve göre değiştirilir.
    fn cagri(&mut self, ad_: &mut String, arg: &mut Vec<Ifade>, konum: Konum) -> Sonuc<Tip> {
        let mut tipler = Vec::new();
        for a in arg.iter_mut() {
            tipler.push(self.ifade(a)?);
        }
        // Ön kütüphanenin içinden yerleşik çağrı: programın tanımları gölgelemez.
        // (Önek kalır: kod üretici de programın tanımına değil yerleşiğe bağlar.)
        if let Some(y) = ad_.strip_prefix(YERLESIK_ON_EK) {
            let mut y = y.to_string();
            let t = self.yerlesik(&y, arg, &tipler, konum)?;
            if on_kutuphane::esle(&y, &tipler).is_some() {
                self.on_kutuphaneye_bagla(&mut y, arg, &tipler, konum)?;
                *ad_ = y;
            }
            return Ok(t);
        }
        if ad_ == HATA_SATIRDA {
            return match tipler.as_slice() {
                [Tip::Metin, Tip::Sayi] => Ok(Tip::Bos),
                _ => Err(Hata::yeni(konum, "hata_ver_satırda(mesaj, satır)")),
            };
        }
        let ad = ad_.clone();
        let ad = ad.as_str();
        if ad == ARAYUZ_ISLEVI {
            return Err(Hata::yeni(
                konum,
                "arayüz kendiliğinden çizilir; çağrılamaz",
            ));
        }
        if self.bilesenler.contains(ad) && !self.arayuzde {
            return Err(Hata::yeni(
                konum,
                format!("'{ad}' bir bileşen; yalnızca arayüz: bloğunda ya da başka bir bileşende kullanılabilir"),
            ));
        }
        // Tipi yazılmayan parametre ilk çağrıdaki değerin tipini alır; sonra gövde denetlenir.
        if let Some(imza) = self.imzalar.get_mut(ad) {
            if self.bekleyen.contains_key(ad) {
                for (p, t) in imza.parametreler.iter_mut().zip(&tipler) {
                    if *p == Tip::Bilinmeyen && !tip_belirsiz(t) && *t != Tip::Bos {
                        *p = t.clone();
                    }
                }
            }
            self.islevi_denetle(ad)?;
        }
        if let Some(imza) = self.imzalar.get(ad).cloned() {
            if let (Some(g), true) = (
                ad.strip_prefix("görünüm:"),
                imza.parametreler.len() != tipler.len(),
            ) {
                let tipler: Vec<String> = imza.parametreler.iter().map(|t| t.to_string()).collect();
                return Err(Hata::yeni(
                    konum,
                    match imza.parametreler.len() {
                        0 => format!("'{g}' görünümü @model tanımlamıyor; değer geçirilemez"),
                        1 => format!(
                            "'{g}' görünümü bir değer bekler (@model {}): görünüm(\"{g}\", değer)",
                            tipler[0]
                        ),
                        n => format!(
                            "'{g}' görünümü {n} değer bekler ({}), {} verildi",
                            tipler.join(", "),
                            arg.len()
                        ),
                    },
                ));
            }
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
                if let (Some(g), false) = (ad.strip_prefix("görünüm:"), p.kabul_eder(t)) {
                    let mesaj = if imza.parametreler.len() == 1 {
                        format!("'{g}' görünümü @model {p} bekler, {t} verildi")
                    } else {
                        format!(
                            "'{g}' görünümünün {}. değeri {p} olmalı, {t} verildi",
                            i + 1
                        )
                    };
                    return Err(Hata::yeni(arg[i].konum, mesaj));
                }
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
                        "tipi yazılmayan parametre ilk çağrıdaki değerin tipini alır; tanımda belirtin: (ad: {t})"
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
        if let Some(g) = ad.strip_prefix("görünüm:") {
            return Err(self.gorunum_yok(g, konum));
        }
        let t = self.yerlesik(ad, arg, &tipler, konum)?;
        self.on_kutuphaneye_bagla(ad_, arg, &tipler, konum)?;
        Ok(t)
    }

    /// Yerleşik çağrının ön kütüphanede (Orhunca ile yazılmış) karşılığı varsa
    /// çağrıyı oraya yönlendirir ve o işlevi denetler.
    fn on_kutuphaneye_bagla(
        &mut self,
        ad: &mut String,
        arg: &mut Vec<Ifade>,
        tipler: &[Tip],
        konum: Konum,
    ) -> Sonuc<()> {
        let Some((islev, satir)) = on_kutuphane::esle(ad, tipler) else {
            return Ok(());
        };
        // kaçır(x): metin olmayan değer önce metne çevrilir.
        if ad == "kaçır" && tipler[0] != Tip::Metin {
            let x = arg.remove(0);
            let k = x.konum;
            arg.push(Ifade {
                tur: IfadeTuru::Cagri("metin".into(), vec![x]),
                konum: k,
                tip: Tip::Metin,
            });
        }
        if satir {
            arg.push(Ifade {
                tur: IfadeTuru::Sayi(konum.satir as i64),
                konum,
                tip: Tip::Sayi,
            });
        }
        *ad = format!("{ON_EK}{islev}");
        self.islevi_denetle(ad)
    }

    fn yerlesik(&mut self, ad: &str, arg: &mut [Ifade], t: &[Tip], konum: Konum) -> Sonuc<Tip> {
        use Tip::*;
        let ic = |t: &Tip| match t {
            Liste(i) | Sozluk(i, _) if **i != Bilinmeyen => (**i).clone(),
            _ => Sayi,
        };
        let sonuc = match (ad, t) {
            ("uzunluk", [Liste(_) | Metin | Sozluk(..)]) => Sayi,
            ("metin", [x]) if *x != Bos => Metin,
            ("sayı", [Metin | Sayi | Ondalik]) => Sayi,
            ("ondalık", [Metin | Sayi | Ondalik]) => Ondalik,
            ("yuvarla", [x]) if x.sayisal() => Sayi,
            ("yuvarla", [x, Sayi]) if x.sayisal() => {
                genislet(&mut arg[0], &Ondalik);
                Ondalik
            }
            ("sayı_mı" | "ondalık_mı", [Metin]) => Mantik,
            ("büyük_harf" | "küçük_harf" | "kırp", [Metin]) => Metin,
            ("parça", [Metin | Liste(_), Sayi, Sayi]) => t[0].clone(),
            ("böl", [Metin, Metin]) => Liste(Box::new(Metin)),
            ("birleştir", [Liste(i), Metin]) if matches!(**i, Metin | Bilinmeyen) => Metin,
            ("içerir", [Metin, Metin]) => Mantik,
            ("bul", [Metin, Metin]) => Sayi,
            ("içerir" | "bul", [Liste(i), x]) if i.kabul_eder(x) => {
                genislet(&mut arg[1], i);
                if ad == "bul" {
                    Sayi
                } else {
                    Mantik
                }
            }
            ("içerir", [Sozluk(a, _), x]) if a.kabul_eder(x) => Mantik,
            ("değiştir", [Metin, Metin, Metin]) => Metin,
            ("başlar" | "biter", [Metin, Metin]) => Mantik,
            ("tekrarla", [Metin, Sayi]) => Metin,
            ("harfler" | "satırlar", [Metin]) => Liste(Box::new(Metin)),
            ("kodlar", [Metin]) => Liste(Box::new(Sayi)),
            ("kodlardan", [Liste(i)]) if matches!(**i, Sayi | Bilinmeyen) => Metin,
            ("kod", [Metin]) => Sayi,
            ("karakter", [Sayi]) => Metin,
            ("sil", [Liste(_), Sayi]) => ic(&t[0]),
            ("sil", [Sozluk(a, _), x]) if a.kabul_eder(x) => Bos,
            ("ters", [Liste(_) | Metin]) => t[0].clone(),
            ("kopya", [Liste(_)]) => t[0].clone(),
            ("karıştır", [Liste(_)]) => Bos,
            ("en_büyük" | "en_küçük", [Liste(i)]) if matches!(**i, Sayi | Ondalik | Metin) => {
                (**i).clone()
            }
            ("en_büyük" | "en_küçük", [a, b]) if a.sayisal() && b.sayisal() => {
                if a != b {
                    genislet(&mut arg[0], &Ondalik);
                    genislet(&mut arg[1], &Ondalik);
                    Ondalik
                } else {
                    a.clone()
                }
            }
            ("toplam", [Liste(i)]) if matches!(**i, Sayi | Ondalik | Bilinmeyen) => ic(&t[0]),
            ("anahtarlar", [Sozluk(a, _)]) => Liste(a.clone()),
            ("değerler", [Sozluk(_, d)]) => Liste(d.clone()),
            ("dosya_oku", [Metin]) => Metin,
            ("dosyaya_yaz" | "dosyaya_ekle", [Metin, Metin]) => Bos,
            ("dosya_var" | "dosya_sil", [Metin]) => Mantik,
            ("karekök" | "sinüs" | "kosinüs" | "tanjant" | "logaritma", [x]) if x.sayisal() => {
                genislet(&mut arg[0], &Ondalik);
                Ondalik
            }
            ("üs", [Sayi, Sayi]) => Sayi,
            ("üs" | "logaritma", [a, b]) if a.sayisal() && b.sayisal() => {
                genislet(&mut arg[0], &Ondalik);
                genislet(&mut arg[1], &Ondalik);
                Ondalik
            }
            ("mutlak", [x]) if x.sayisal() => x.clone(),
            ("rastgele", []) => Ondalik,
            ("rastgele", [Sayi, Sayi]) => Sayi,
            ("zaman", []) => Ondalik,
            ("tarih" | "oku", []) => Metin,
            ("bekle", [x]) if x.sayisal() => {
                genislet(&mut arg[0], &Ondalik);
                Bos
            }
            ("argümanlar", []) => Liste(Box::new(Metin)),
            ("json" | "kaçır", [x]) if *x != Bos => Metin,
            ("para", [x]) if x.sayisal() => {
                genislet(&mut arg[0], &Ondalik);
                Metin
            }
            ("url_kodla", [Metin]) => Metin,
            ("sun", []) | ("sun", [Sayi]) => Bos,
            ("ortam", [Metin]) => Metin,
            ("çık", [Sayi]) => Bos,
            ("hata_ver", [Metin]) => Bos,
            ("boş_mu", [Model(_)]) => Mantik,
            _ if arayuz::oge(ad).is_some() => {
                return Err(Hata::yeni(
                    konum,
                    format!("'{ad}' bir arayüz öğesi; yalnızca arayüz: bloğunda (ya da bir bileşende) kullanılabilir"),
                )
                .ipucu("ekrana yazmak için: x'i yaz."))
            }
            _ => {
                let verilen: Vec<String> = t.iter().map(|t| t.to_string()).collect();
                return Err(match crate::yerlesik::bul(ad) {
                    Some(y) => Hata::yeni(
                        konum,
                        format!(
                            "'{ad}' bu bağımsız değişkenlerle kullanılamaz ({})",
                            verilen.join(", ")
                        ),
                    )
                    .ipucu(format!("kullanım: {}", y.kullanim)),
                    None => Hata::yeni(konum, format!("tanımsız işlev '{ad}'")),
                });
            }
        };
        Ok(sonuc)
    }
}

/// Tipin içinde henüz bilinmeyen bir parça var mı (`liste<?>`)?
fn tip_belirsiz(t: &Tip) -> bool {
    match t {
        Tip::Bilinmeyen => true,
        Tip::Liste(i) => tip_belirsiz(i),
        Tip::Sozluk(a, d) => tip_belirsiz(a) || tip_belirsiz(d),
        _ => false,
    }
}

/// Bağlı öğenin değişkenine kullanıcının girdiği değeri yazan deyim. Sayıya
/// çevrilemeyen girdiler yok sayılır (kullanıcı yazmayı sürdürüyor olabilir).
fn bag_atamasi(hedef: Ifade, tur: BagTuru) -> Deyim {
    let k = hedef.konum;
    let e = |tur| Ifade::yeni(tur, k);
    let deger = || e(IfadeTuru::Isim(OLAY_DEGERI.into()));
    let cagri = |ad: &str, a: Ifade| e(IfadeTuru::Cagri(ad.into(), vec![a]));
    let ata = |yeni: Ifade| match hedef.tur.clone() {
        IfadeTuru::Isim(ad) => Deyim::Atama {
            hedef: ad,
            tip: None,
            deger: yeni,
            konum: k,
        },
        IfadeTuru::Alan(nesne, alan, _) => Deyim::AlanAtama {
            nesne: *nesne,
            alan,
            sira: 0,
            deger: yeni,
            konum: k,
        },
        IfadeTuru::Indeks(liste, indeks) => Deyim::IndeksAtama {
            liste: *liste,
            indeks: *indeks,
            deger: yeni,
        },
        _ => unreachable!("bag_hedefi denetledi"),
    };
    let kosullu = |sinama: &str, cevir: &str| Deyim::Eger {
        kosul: cagri(sinama, deger()),
        govde: vec![ata(cagri(cevir, deger()))],
        degilse: Vec::new(),
    };
    match (tur, &hedef.tip) {
        (BagTuru::Mantik, _) => ata(e(IfadeTuru::Ikili(
            IkiliOp::Esit,
            Box::new(deger()),
            Box::new(e(IfadeTuru::Metin("doğru".into()))),
        ))),
        (_, Tip::Sayi) => kosullu("sayı_mı", "sayı"),
        (_, Tip::Ondalik) => kosullu("ondalık_mı", "ondalık"),
        // Seçenek: seçilen metin seçenek türüne çevrilir (geçersizse çalışma hatası).
        (_, Tip::Secenek(ad)) => ata(cagri(ad, deger())),
        _ => ata(deger()),
    }
}

/// Değeri JSON olarak gönderen yanıtın alanları.
fn json_alanlari(deger: Ifade) -> Vec<(String, Ifade)> {
    let k = deger.konum;
    vec![
        (
            "tür".to_string(),
            Ifade {
                tur: IfadeTuru::Metin(JSON_TURU.into()),
                konum: k,
                tip: Tip::Metin,
            },
        ),
        (
            "gövde".to_string(),
            Ifade {
                tur: IfadeTuru::Cagri("json".into(), vec![deger]),
                konum: k,
                tip: Tip::Metin,
            },
        ),
    ]
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
        (Topla, Metin, Sayi | Ondalik | Metin | Mantik | Secenek(_))
        | (Topla, Sayi | Ondalik | Mantik | Secenek(_), Metin) => Some(Metin),
        (TamBol | Mod, Sayi, Sayi) => Some(Sayi),
        (Kucuk | Buyuk | KucukEsit | BuyukEsit, _, _) if sayisal => Some(Mantik),
        (Kucuk | Buyuk | KucukEsit | BuyukEsit, Metin, Metin) => Some(Mantik),
        (Esit | EsitDegil, _, _) if sayisal => Some(Mantik),
        (Esit | EsitDegil, Metin, Metin) | (Esit | EsitDegil, Mantik, Mantik) => Some(Mantik),
        (Esit | EsitDegil, Model(a), Model(b)) if a == b => Some(Mantik),
        (Esit | EsitDegil, Secenek(a), Secenek(b)) if a == b => Some(Mantik),
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
