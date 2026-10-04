//! `.ohchtml` görünümleri: HTML içine gömülü Orhunca.
//!
//! Bu modül görünüm metnini parçalara ayırır; gömülü kod parçalarını sözcüklere
//! çevirir (konumlar görünüm dosyasındaki yerlerini gösterir). Parçaları Orhunca
//! işlevlerine ayrıştırıcı çevirir (`ayristirici::sablon_islevi`).
//!
//! ```text
//! @model liste<Ürün>          görünüme geçirilen değerin tipi (`model` adıyla kullanılır)
//! @düzen "düzen"              sayfayı görünümler/düzen.ohchtml içine yerleştirir
//! @başlık "Ürünler"           düzene geçirilen sayfa başlığı
//! @ürün.ad   @(a + b)         HTML'ye kaçırılarak yazılan değer
//! @ham(metin)                 kaçırılmadan yazılan HTML
//! @eğer koşul { ... } @değilse { ... }
//! @her ürün için model'den { ... }
//! @içerik                     (düzende) sayfanın içeriği
//! @@  @* yorum *@
//! ```

use crate::hata::{Hata, Konum, Sonuc};
use crate::sozcuk::{sozcukle, Sozcuk, Tok};

/// Derlemeye katılan bir görünüm dosyası.
pub struct SablonKaynagi {
    /// `görünümler/` klasörüne göre uzantısız yol: `ürünler/liste`
    pub ad: String,
    /// Derlemedeki dosya sırası (hata gösterimi için).
    pub dosya: usize,
    pub kaynak: String,
}

#[derive(Debug)]
pub enum Parca {
    Metin(String),
    /// `@ifade`, `@(ifade)`; `ham`: `@ham(...)`
    Cikti {
        sozcukler: Vec<Sozcuk>,
        ham: bool,
    },
    Eger {
        kosul: Vec<Sozcuk>,
        govde: Vec<Parca>,
        degilse: Vec<Parca>,
    },
    Her {
        baslik: Vec<Sozcuk>,
        govde: Vec<Parca>,
        konum: Konum,
    },
    Icerik,
}

#[derive(Debug)]
pub struct Sablon {
    pub ad: String,
    pub konum: Konum,
    pub model: Option<Vec<Sozcuk>>,
    pub duzen: Option<(String, Konum)>,
    pub baslik: Option<Vec<Sozcuk>>,
    pub parcalar: Vec<Parca>,
    /// `@içerik` kullanan görünüm bir düzendir.
    pub icerik_var: bool,
}

impl Sablon {
    /// Gömülü kod parçalarının tüm sözcükleri (isim sözlüğü kurulurken taranır).
    pub fn tum_sozcukler(&self) -> Vec<Sozcuk> {
        fn topla(p: &[Parca], v: &mut Vec<Sozcuk>) {
            for p in p {
                match p {
                    Parca::Cikti { sozcukler, .. } => v.extend(sozcukler.iter().cloned()),
                    Parca::Eger {
                        kosul,
                        govde,
                        degilse,
                    } => {
                        v.extend(kosul.iter().cloned());
                        topla(govde, v);
                        topla(degilse, v);
                    }
                    Parca::Her { baslik, govde, .. } => {
                        // `her` kelimesi döngü değişkeninin tanınması için başa eklenir.
                        if let Some(ilk) = baslik.first() {
                            v.push(Sozcuk {
                                tok: Tok::Kelime("her".into()),
                                konum: ilk.konum,
                            });
                        }
                        v.extend(baslik.iter().cloned());
                        topla(govde, v);
                    }
                    Parca::Metin(_) | Parca::Icerik => {}
                }
            }
        }
        let mut v = Vec::new();
        for s in [&self.model, &self.baslik].into_iter().flatten() {
            v.extend(s.iter().cloned());
        }
        topla(&self.parcalar, &mut v);
        v
    }
}

/// CSS'in `@media`, `@import` gibi kuralları kod sayılmaz.
const CSS_KURALLARI: &[&str] = &[
    "media",
    "import",
    "keyframes",
    "font",
    "supports",
    "charset",
    "page",
    "layer",
    "container",
    "property",
    "namespace",
    "counter",
];

fn kelime_basi(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn kelime_devami(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn bosluk(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r'
}

struct Cozucu {
    k: Vec<char>,
    i: usize,
    dosya: usize,
    satir_baslari: Vec<usize>,
}

/// Bir görünüm dosyasını parçalara ayırır.
pub fn coz(kaynak: &SablonKaynagi) -> Sonuc<Sablon> {
    let k: Vec<char> = kaynak.kaynak.chars().collect();
    let mut satir_baslari = vec![0];
    for (i, c) in k.iter().enumerate() {
        if *c == '\n' {
            satir_baslari.push(i + 1);
        }
    }
    let mut c = Cozucu {
        k,
        i: 0,
        dosya: kaynak.dosya,
        satir_baslari,
    };
    let mut s = Sablon {
        ad: kaynak.ad.clone(),
        konum: Konum {
            satir: 1,
            sutun: 1,
            dosya: kaynak.dosya,
        },
        model: None,
        duzen: None,
        baslik: None,
        parcalar: Vec::new(),
        icerik_var: false,
    };
    s.parcalar = c.parcalar(None, &mut s)?;
    // Dosyanın sonundaki satır sonu çıktıya girmez (parça görünümler boş satır bırakmasın).
    if let Some(Parca::Metin(m)) = s.parcalar.last_mut() {
        if m.ends_with('\n') {
            m.pop();
            if m.ends_with('\r') {
                m.pop();
            }
        }
        if m.is_empty() {
            s.parcalar.pop();
        }
    }
    if s.icerik_var && s.model.is_some() {
        return Err(Hata::yeni(
            s.konum,
            "düzen görünümleri (@içerik kullananlar) @model alamaz",
        ));
    }
    if let (true, Some((_, k))) = (s.icerik_var, &s.duzen) {
        return Err(Hata::yeni(
            *k,
            "bir düzen başka bir düzen içine yerleştirilemez (henüz)",
        ));
    }
    Ok(s)
}

impl Cozucu {
    fn konum(&self, i: usize) -> Konum {
        let satir = match self.satir_baslari.binary_search(&i) {
            Ok(s) => s,
            Err(s) => s - 1,
        };
        Konum {
            satir: satir + 1,
            sutun: i - self.satir_baslari[satir] + 1,
            dosya: self.dosya,
        }
    }

    fn bak(&self, i: usize) -> Option<char> {
        self.k.get(i).copied()
    }

    /// `i` konumundan önce satırda yalnızca boşluk mu var?
    fn satir_basinda(&self, i: usize) -> bool {
        let mut j = i;
        while j > 0 {
            match self.k[j - 1] {
                '\n' => return true,
                c if bosluk(c) => j -= 1,
                _ => return false,
            }
        }
        true
    }

    /// `i` konumundan satır sonuna kadar yalnızca boşluk varsa satır sonundan
    /// sonraki konumu verir.
    fn satir_sonu(&self, i: usize) -> Option<usize> {
        let mut j = i;
        loop {
            match self.bak(j) {
                None => return Some(j),
                Some('\n') => return Some(j + 1),
                Some(c) if bosluk(c) => j += 1,
                _ => return None,
            }
        }
    }

    /// Kontrol satırının (yalnızca `@eğer ... {` ya da `}` içeren satır) başındaki
    /// girintiyi çıktıdan siler.
    fn girinti_sil(metin: &mut String) {
        let kalan = metin.trim_end_matches([' ', '\t']).len();
        metin.truncate(kalan);
    }

    /// `ac` karakterinden başlayarak eşini bulur, eşin sonrasındaki konumu verir.
    /// Metinlerin içindeki parantezler sayılmaz.
    fn dengeli(&self, bas: usize, ac: char, kapa: char) -> Sonuc<usize> {
        let mut derinlik = 0;
        let mut j = bas;
        while let Some(c) = self.bak(j) {
            if c == '"' {
                j += 1;
                while let Some(d) = self.bak(j) {
                    j += 1;
                    if d == '\\' {
                        j += 1;
                    } else if d == '"' {
                        break;
                    }
                }
                continue;
            }
            if c == ac {
                derinlik += 1;
            } else if c == kapa {
                derinlik -= 1;
                if derinlik == 0 {
                    return Ok(j + 1);
                }
            }
            j += 1;
        }
        Err(Hata::yeni(self.konum(bas), format!("'{ac}' kapatılmamış"))
            .ipucu(format!("ifadeyi '{kapa}' ile kapatın")))
    }

    /// `{`'den önceki koşul ya da döngü başlığını okur; `{`'nin konumunu verir.
    fn suslu_oncesi(&self, bas: usize, ne: &str) -> Sonuc<usize> {
        let mut j = bas;
        while let Some(c) = self.bak(j) {
            match c {
                '{' => return Ok(j),
                '"' | '(' | '[' => {
                    j = if c == '"' {
                        let mut m = j + 1;
                        while let Some(d) = self.bak(m) {
                            m += 1;
                            if d == '\\' {
                                m += 1;
                            } else if d == '"' {
                                break;
                            }
                        }
                        m
                    } else {
                        self.dengeli(j, c, if c == '(' { ')' } else { ']' })?
                    };
                }
                '\n' => break,
                _ => j += 1,
            }
        }
        Err(
            Hata::yeni(self.konum(bas), format!("{ne} sonunda '{{' bekleniyordu"))
                .ipucu("@eğer x 0'dan büyükse { ... }  ya da  @her ürün için ürünler'den { ... }"),
        )
    }

    /// Gömülü kod parçasını sözcüklere çevirir; konumları görünüm dosyasına göre ayarlar.
    fn sozcukle(&self, bas: usize, son: usize) -> Sonuc<Vec<Sozcuk>> {
        let mut bas = bas;
        while bas < son && (bosluk(self.k[bas]) || self.k[bas] == '\n') {
            bas += 1;
        }
        let metin: String = self.k[bas..son]
            .iter()
            .map(|c| if *c == '\n' || *c == '\r' { ' ' } else { *c })
            .collect();
        let k0 = self.konum(bas);
        let tasi = |k: Konum| Konum {
            satir: k0.satir + k.satir - 1,
            sutun: if k.satir == 1 {
                k0.sutun + k.sutun - 1
            } else {
                k.sutun
            },
            dosya: self.dosya,
        };
        let mut s = sozcukle(&metin).map_err(|mut h| {
            h.konum = tasi(h.konum);
            h
        })?;
        for t in s.iter_mut() {
            t.konum = tasi(t.konum);
        }
        Ok(s)
    }

    /// `@` işaretinden sonraki kelimeyi okur (ilerlemeden).
    fn kelime(&self, bas: usize) -> String {
        self.k[bas..]
            .iter()
            .take_while(|c| kelime_devami(**c))
            .collect()
    }

    /// `@eğer` ya da `@her` sonrasındaki `{ ... }` bloğunu okur. `ac`: `{` konumu.
    /// Kapanış `}` satırın başındaysa bunu da bildirir.
    fn blok(&mut self, ac: usize, s: &mut Sablon) -> Sonuc<(Vec<Parca>, bool)> {
        self.i = ac + 1;
        if let Some(j) = self.satir_sonu(self.i) {
            self.i = j;
        }
        let mut govde = self.parcalar(Some(ac), s)?;
        // Kapanış `}` tek başına bir satırdaysa o satır çıktıya girmez.
        let satir_basi = self.satir_basinda(self.i);
        if satir_basi {
            if let Some(Parca::Metin(m)) = govde.last_mut() {
                Self::girinti_sil(m);
            }
        }
        self.i += 1; // }
        if let Some(Parca::Metin(m)) = govde.last() {
            if m.is_empty() {
                govde.pop();
            }
        }
        Ok((govde, satir_basi))
    }

    /// Satırın başındaki kapanış `}` sonrasındaki boşluğu (satır sonu dahil) atlar.
    fn kapanis_satiri(&mut self, satir_basi: bool) {
        if !satir_basi {
            return;
        }
        if let Some(j) = self.satir_sonu(self.i) {
            self.i = j;
        }
    }

    /// `@eğer` (i: `eğer` kelimesinin başı) ve olası `@değilse` dalları.
    fn eger(&mut self, kelime_basi: usize, s: &mut Sablon) -> Sonuc<Parca> {
        let kosul_basi = kelime_basi + "eğer".chars().count();
        let ac = self.suslu_oncesi(kosul_basi, "@eğer koşulunun")?;
        let mut kosul = self.sozcukle(kosul_basi, ac)?;
        if kosul.len() <= 2 {
            return Err(Hata::yeni(
                self.konum(kelime_basi),
                "@eğer'den sonra bir koşul bekleniyordu",
            ));
        }
        // Ayrıştırıcının koşul kuralı ':' ile biter.
        let yer = kosul.len() - 2;
        let k = kosul[yer].konum;
        kosul.insert(
            yer,
            Sozcuk {
                tok: Tok::Op(":"),
                konum: k,
            },
        );
        let (govde, satir_basi) = self.blok(ac, s)?;
        // `} @değilse {` ya da `}` + yeni satır + `@değilse {`
        let mut j = self.i;
        while self.bak(j).is_some_and(|c| bosluk(c) || c == '\n') {
            j += 1;
        }
        let mut degilse = Vec::new();
        if self.bak(j) == Some('@') && self.kelime(j + 1) == "değilse" {
            let mut m = j + 1 + "değilse".chars().count();
            while self.bak(m).is_some_and(bosluk) {
                m += 1;
            }
            if self.kelime(m) == "eğer" {
                self.i = m;
                degilse.push(self.eger(m, s)?);
            } else if self.bak(m) == Some('{') {
                let (g, satir_basi) = self.blok(m, s)?;
                degilse = g;
                self.kapanis_satiri(satir_basi);
            } else {
                return Err(Hata::yeni(
                    self.konum(m),
                    "@değilse'den sonra '{' bekleniyordu",
                ));
            }
        } else {
            self.kapanis_satiri(satir_basi);
        }
        Ok(Parca::Eger {
            kosul,
            govde,
            degilse,
        })
    }

    /// Yönerge satırının kalanı: `@model liste<Ürün>` → `liste<Ürün>`
    fn yonerge(&mut self, bas: usize) -> (usize, usize) {
        let mut son = bas;
        while self.bak(son).is_some_and(|c| c != '\n') {
            son += 1;
        }
        self.i = (son + 1).min(self.k.len());
        (bas, son)
    }

    fn parcalar(&mut self, blok: Option<usize>, s: &mut Sablon) -> Sonuc<Vec<Parca>> {
        let mut parcalar = Vec::new();
        let mut metin = String::new();
        let mut derinlik = 0;
        macro_rules! bosalt {
            () => {
                if !metin.is_empty() {
                    parcalar.push(Parca::Metin(std::mem::take(&mut metin)));
                }
            };
        }
        loop {
            let Some(c) = self.bak(self.i) else {
                if let Some(ac) = blok {
                    return Err(
                        Hata::yeni(self.konum(ac), "'{' ile açılan blok kapatılmamış")
                            .ipucu("bloğu '}' ile kapatın"),
                    );
                }
                break;
            };
            if blok.is_some() {
                if c == '}' && derinlik == 0 {
                    break;
                }
                if c == '{' {
                    derinlik += 1;
                } else if c == '}' {
                    derinlik -= 1;
                }
            }
            if c != '@' {
                metin.push(c);
                self.i += 1;
                continue;
            }
            let at = self.i;
            let sonraki = self.bak(at + 1);
            match sonraki {
                Some('@') => {
                    metin.push('@');
                    self.i += 2;
                    continue;
                }
                Some('*') => {
                    let mut j = at + 2;
                    while j + 1 < self.k.len() && !(self.k[j] == '*' && self.k[j + 1] == '@') {
                        j += 1;
                    }
                    if j + 1 >= self.k.len() {
                        return Err(Hata::yeni(self.konum(at), "yorum kapatılmamış")
                            .ipucu("yorumu *@ ile kapatın"));
                    }
                    let bas = self.satir_basinda(at);
                    self.i = j + 2;
                    if bas {
                        if let Some(son) = self.satir_sonu(self.i) {
                            Self::girinti_sil(&mut metin);
                            self.i = son;
                        }
                    }
                    continue;
                }
                Some('(') => {
                    let son = self.dengeli(at + 1, '(', ')')?;
                    bosalt!();
                    parcalar.push(Parca::Cikti {
                        sozcukler: self.sozcukle(at + 1, son)?,
                        ham: false,
                    });
                    self.i = son;
                    continue;
                }
                _ => {}
            }
            if !sonraki.is_some_and(kelime_basi) {
                metin.push('@');
                self.i += 1;
                continue;
            }
            let kelime = self.kelime(at + 1);
            // e-posta adresleri: ad@site.com (ama `TL@eğer x {` koddur)
            let onceki_harf = at > 0 && self.k[at - 1].is_alphanumeric();
            if onceki_harf && kelime != "eğer" && kelime != "her" {
                metin.push('@');
                self.i += 1;
                continue;
            }
            let kelime_sonu = at + 1 + kelime.chars().count();
            let satir_basi = self.satir_basinda(at);
            let ardindan = self.bak(kelime_sonu);
            // Yönergeden sonra boşluk ve bir değer gelir: `@model liste<Ürün>`
            let mut deger_basi = kelime_sonu;
            while self.bak(deger_basi).is_some_and(bosluk) {
                deger_basi += 1;
            }
            let deger = self.bak(deger_basi);
            let yonerge = satir_basi && deger_basi > kelime_sonu;
            match kelime.as_str() {
                "eğer" if ardindan.is_some_and(bosluk) => {
                    if satir_basi {
                        Self::girinti_sil(&mut metin);
                    }
                    bosalt!();
                    let p = self.eger(at + 1, s)?;
                    parcalar.push(p);
                }
                "her" if ardindan.is_some_and(bosluk) => {
                    if satir_basi {
                        Self::girinti_sil(&mut metin);
                    }
                    bosalt!();
                    let ac = self.suslu_oncesi(kelime_sonu, "@her başlığının")?;
                    let baslik = self.sozcukle(kelime_sonu, ac)?;
                    let konum = self.konum(at);
                    let (govde, satir_basi) = self.blok(ac, s)?;
                    self.kapanis_satiri(satir_basi);
                    parcalar.push(Parca::Her {
                        baslik,
                        govde,
                        konum,
                    });
                }
                "değilse" => {
                    return Err(Hata::yeni(
                        self.konum(at),
                        "@değilse bir @eğer bloğunun hemen ardından gelmeli",
                    ))
                }
                "model" if yonerge && deger.is_some_and(kelime_basi) => {
                    if blok.is_some() || s.model.is_some() {
                        return Err(Hata::yeni(
                            self.konum(at),
                            "@model görünümün başında bir kez yazılır",
                        ));
                    }
                    Self::girinti_sil(&mut metin);
                    let (b, e) = self.yonerge(deger_basi);
                    s.model = Some(self.sozcukle(b, e)?);
                }
                "düzen" if yonerge && deger == Some('"') => {
                    if blok.is_some() || s.duzen.is_some() {
                        return Err(Hata::yeni(
                            self.konum(at),
                            "@düzen görünümün başında bir kez yazılır",
                        ));
                    }
                    Self::girinti_sil(&mut metin);
                    let (b, e) = self.yonerge(deger_basi);
                    let sozcukler = self.sozcukle(b, e)?;
                    match sozcukler.first().map(|s| &s.tok) {
                        Some(Tok::Metin(ad)) if sozcukler.len() == 3 => {
                            s.duzen = Some((ad.clone(), self.konum(b)))
                        }
                        _ => {
                            return Err(Hata::yeni(
                                self.konum(b),
                                "@düzen'den sonra düzen görünümünün adı gelmeli",
                            )
                            .ipucu("@düzen \"düzen\""))
                        }
                    }
                }
                "başlık" if yonerge && matches!(deger, Some('"') | Some('(')) => {
                    if blok.is_some() || s.baslik.is_some() {
                        return Err(Hata::yeni(
                            self.konum(at),
                            "@başlık görünümün başında bir kez yazılır",
                        ));
                    }
                    Self::girinti_sil(&mut metin);
                    let (b, e) = self.yonerge(deger_basi);
                    s.baslik = Some(self.sozcukle(b, e)?);
                }
                "içerik" if !ardindan.is_some_and(|c| c == '.' || c == '(') => {
                    bosalt!();
                    s.icerik_var = true;
                    parcalar.push(Parca::Icerik);
                    self.i = kelime_sonu;
                }
                "ham" if ardindan == Some('(') => {
                    let son = self.dengeli(kelime_sonu, '(', ')')?;
                    bosalt!();
                    parcalar.push(Parca::Cikti {
                        sozcukler: self.sozcukle(kelime_sonu, son)?,
                        ham: true,
                    });
                    self.i = son;
                }
                k if CSS_KURALLARI.iter().any(|c| k.starts_with(c)) => {
                    metin.push('@');
                    self.i += 1;
                }
                _ => {
                    // @ad, @ürün.ad, @işlev(x), @liste[0].ad
                    let mut j = at + 1;
                    loop {
                        while self.bak(j).is_some_and(kelime_devami) {
                            j += 1;
                        }
                        loop {
                            match self.bak(j) {
                                Some('(') => j = self.dengeli(j, '(', ')')?,
                                Some('[') => j = self.dengeli(j, '[', ']')?,
                                _ => break,
                            }
                        }
                        if self.bak(j) == Some('.') && self.bak(j + 1).is_some_and(kelime_basi) {
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    bosalt!();
                    parcalar.push(Parca::Cikti {
                        sozcukler: self.sozcukle(at + 1, j)?,
                        ham: false,
                    });
                    self.i = j;
                }
            }
        }
        bosalt!();
        Ok(parcalar)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn coz_metin(k: &str) -> Sablon {
        coz(&SablonKaynagi {
            ad: "deneme".into(),
            dosya: 0,
            kaynak: k.into(),
        })
        .unwrap()
    }

    fn metinler(p: &[Parca]) -> Vec<String> {
        p.iter()
            .map(|p| match p {
                Parca::Metin(m) => m.clone(),
                Parca::Cikti { ham, .. } => format!("<{}>", if *ham { "ham" } else { "ifade" }),
                Parca::Eger { .. } => "<eğer>".into(),
                Parca::Her { .. } => "<her>".into(),
                Parca::Icerik => "<içerik>".into(),
            })
            .collect()
    }

    #[test]
    fn ifadeler_ve_metin() {
        let s = coz_metin("<p>@ürün.ad TL, @(a + b). ali@site.com @@ @ham(x)</p>\n");
        assert_eq!(
            metinler(&s.parcalar),
            vec![
                "<p>",
                "<ifade>",
                " TL, ",
                "<ifade>",
                ". ali@site.com @ ",
                "<ham>",
                "</p>"
            ]
        );
        let Parca::Cikti { sozcukler, .. } = &s.parcalar[1] else {
            panic!()
        };
        assert_eq!(sozcukler[1].tok, Tok::Uye);
        assert_eq!(sozcukler[0].konum.sutun, 5);
    }

    #[test]
    fn bloklar_ve_satir_kirpma() {
        let s = coz_metin(
            "@model liste<Ürün>\n<ul>\n    @her ü için model'den {\n        <li>@ü.ad</li>\n    }\n</ul>\n@eğer x 0'a eşitse {\n<b>yok</b>\n} @değilse {\n<i>var</i>\n}\n",
        );
        assert!(s.model.is_some());
        assert_eq!(
            metinler(&s.parcalar),
            vec!["<ul>\n", "<her>", "</ul>\n", "<eğer>"]
        );
        let Parca::Her { govde, .. } = &s.parcalar[1] else {
            panic!()
        };
        assert_eq!(metinler(govde), vec!["        <li>", "<ifade>", "</li>\n"]);
        let Parca::Eger { kosul, degilse, .. } = &s.parcalar[3] else {
            panic!()
        };
        assert!(kosul.iter().any(|s| s.tok == Tok::Op(":")));
        assert_eq!(metinler(degilse), vec!["<i>var</i>\n"]);
    }

    #[test]
    fn css_ve_suslu_parantezler() {
        let s = coz_metin(
            "<style>@media (x) { a { b: c } }</style>\n@eğer d { <style>p { e: f }</style> }\n",
        );
        assert_eq!(
            metinler(&s.parcalar),
            vec!["<style>@media (x) { a { b: c } }</style>\n", "<eğer>"]
        );
    }

    #[test]
    fn duzen_ve_hatalar() {
        let s = coz_metin("@düzen \"düzen\"\n@başlık \"Ürünler\"\n<h1>@başlık</h1>\n");
        assert_eq!(s.duzen.as_ref().unwrap().0, "düzen");
        assert!(s.baslik.is_some());
        assert_eq!(metinler(&s.parcalar), vec!["<h1>", "<ifade>", "</h1>"]);
        let h = coz(&SablonKaynagi {
            ad: "x".into(),
            dosya: 0,
            kaynak: "@eğer x {\n<p>\n".into(),
        })
        .unwrap_err();
        assert!(h.mesaj.contains("kapatılmamış"));
        assert_eq!(h.konum.satir, 1);
    }
}
