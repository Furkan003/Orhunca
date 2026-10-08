//! Koşullu kesme noktaları ve günlük noktaları (hata ayıklayıcı).
//!
//! Çalışma zamanı yalnızca (dosya, satır) kesme noktalarını bilir. Koşullar ve günlük
//! mesajları, hata ayıklama için derlenen programa ayrıştırmadan sonra deyim olarak
//! yerleştirilir; böylece tip denetiminden geçer ve o satırın kapsamındaki değişkenleri
//! görür:
//!
//! - koşullu kesme: `eğer koşul ise: ayıklama_durağı = doğru` — bu gizli deyimin satırı
//!   `satır + KAYDIRMA`dır ve çalışma zamanına kesme noktası olarak o satır gönderilir;
//!   durulunca Stüdyo satırı geri çevirir.
//! - günlük noktası: `"◆ x = " + metin(x)` yazdırılır, program durmaz (koşul varsa yalnızca
//!   koşul doğruyken).

use crate::agac::{Deyim, Ifade, IfadeTuru, IkiliOp, Program};
use crate::hata::{Hata, Konum};
use crate::sozcuk::sozcukle;

/// Gizli durak deyiminin satırı: asıl satır + KAYDIRMA.
pub const KAYDIRMA: usize = 10_000_000;
/// Gizli değişkenin adı (değişken listesinde gösterilmez).
pub const GIZLI_DEGISKEN: &str = "ayıklama_durağı";

#[derive(Debug, Clone, Default)]
pub struct KosulluKesme {
    pub dosya: String,
    pub satir: usize,
    pub kosul: Option<String>,
    pub gunluk: Option<String>,
}

impl KosulluKesme {
    /// Çalışma zamanına gönderilecek kesme noktası: günlük noktası durmaz, koşullu kesme
    /// gizli deyimin satırında durur.
    pub fn calisma_zamani_satiri(&self) -> Option<usize> {
        match (&self.kosul, &self.gunluk) {
            (_, Some(_)) => None,
            (Some(_), None) => Some(self.satir + KAYDIRMA),
            (None, None) => Some(self.satir),
        }
    }
}

/// Bir ifadeyi ayrıştırır; bütün sözcükleri kesme noktasının yerine taşır (hatalar o
/// satırı göstersin).
fn ifade_ayristir(kaynak: &str, k: Konum, ne: &str) -> Result<Ifade, Hata> {
    let hata = |m: String| Hata::yeni(k, format!("{ne} ({}. satır): {m}", k.satir));
    // Ayrıştırıcı isimleri tanır; ifadedeki değişkenler o satırın kapsamındadır, burada
    // bilinmez. Tanımsız denen her isim önceden tanımlanıp yeniden denenir (asıl kapsam
    // denetimi yerleştirildikten sonra tip denetçisinde yapılır).
    let mut onceden = String::new();
    for _ in 0..32 {
        let mut s = sozcukle(&format!("{onceden}ayıklama_ifadesi = ({kaynak})\n"))
            .map_err(|h| hata(h.mesaj))?;
        for w in s.iter_mut() {
            w.konum = k;
        }
        match crate::ayristirici::ayristir_cok(vec![s], Vec::new()) {
            Ok(p) => {
                return p
                    .ana
                    .into_iter()
                    .find_map(|d| match d {
                        Deyim::Atama { hedef, deger, .. } if hedef == "ayıklama_ifadesi" => {
                            Some(deger)
                        }
                        _ => None,
                    })
                    .ok_or_else(|| hata("ifade anlaşılamadı".into()))
            }
            Err(h) => match h
                .mesaj
                .strip_prefix("tanımsız isim '")
                .and_then(|r| r.split('\'').next())
            {
                Some(ad) if !onceden.contains(&format!("\n{ad} = 0\n")) => {
                    onceden = format!("\n{ad} = 0\n{onceden}");
                }
                _ => return Err(hata(h.mesaj)),
            },
        }
    }
    Err(hata("ifade anlaşılamadı".into()))
}

/// `"x = {x}, toplam {a + b}"` → `"x = " + metin(x) + ", toplam " + metin(a + b)`
fn gunluk_ifadesi(sablon: &str, k: Konum) -> Result<Ifade, Hata> {
    let metin = |m: &str| Ifade::yeni(IfadeTuru::Metin(m.to_string()), k);
    let topla = |a: Ifade, b: Ifade| {
        Ifade::yeni(
            IfadeTuru::Ikili(IkiliOp::Topla, Box::new(a), Box::new(b)),
            k,
        )
    };
    let mut sonuc = metin("◆ ");
    let mut kalan = sablon;
    while let Some(i) = kalan.find('{') {
        let Some(j) = kalan[i..].find('}') else { break };
        if i > 0 {
            sonuc = topla(sonuc, metin(&kalan[..i]));
        }
        let ic = ifade_ayristir(&kalan[i + 1..i + j], k, "günlük noktası")?;
        sonuc = topla(
            sonuc,
            Ifade::yeni(IfadeTuru::Cagri("metin".into(), vec![ic]), k),
        );
        kalan = &kalan[i + j + 1..];
    }
    if !kalan.is_empty() {
        sonuc = topla(sonuc, metin(kalan));
    }
    Ok(sonuc)
}

/// Bloklarda (iç içe) `konum`daki ilk deyimin önüne `yeni` deyimleri ekler.
fn yerlestir_blok(
    govde: &mut Vec<Deyim>,
    dosya: usize,
    satir: usize,
    yeni: &mut Option<Vec<Deyim>>,
) {
    if yeni.is_none() {
        return;
    }
    let mut i = 0;
    while i < govde.len() {
        if govde[i]
            .konum()
            .is_some_and(|k| k.dosya == dosya && k.satir == satir)
        {
            let eklenen = yeni.take().unwrap();
            govde.splice(i..i, eklenen);
            return;
        }
        match &mut govde[i] {
            Deyim::Eger {
                govde: a,
                degilse: b,
                ..
            }
            | Deyim::Dene {
                govde: a,
                yakala: b,
                ..
            } => {
                yerlestir_blok(a, dosya, satir, yeni);
                yerlestir_blok(b, dosya, satir, yeni);
            }
            Deyim::Surece { govde: a, .. }
            | Deyim::HerAralik { govde: a, .. }
            | Deyim::HerListe { govde: a, .. } => yerlestir_blok(a, dosya, satir, yeni),
            _ => {}
        }
        if yeni.is_none() {
            return;
        }
        i += 1;
    }
}

/// Koşullu kesme ve günlük noktalarını programa yerleştirir (tip denetiminden önce).
pub fn yerlestir(
    p: &mut Program,
    dosyalar: &[(String, String)],
    kesmeler: &[KosulluKesme],
) -> Result<(), Hata> {
    let tam = |y: &str| std::fs::canonicalize(y).unwrap_or_else(|_| y.into());
    for kb in kesmeler {
        if kb.kosul.is_none() && kb.gunluk.is_none() {
            continue;
        }
        let Some(dosya) = dosyalar.iter().position(|(y, _)| tam(y) == tam(&kb.dosya)) else {
            continue;
        };
        let k = Konum {
            dosya,
            satir: kb.satir,
            sutun: 1,
        };
        let mut eylem = match &kb.gunluk {
            Some(g) => Deyim::Yaz(gunluk_ifadesi(g, k)?),
            None => Deyim::Atama {
                hedef: GIZLI_DEGISKEN.into(),
                tip: None,
                deger: Ifade::yeni(IfadeTuru::Mantik(true), k),
                konum: Konum {
                    satir: kb.satir + KAYDIRMA,
                    ..k
                },
            },
        };
        if let Some(kosul) = kb.kosul.as_deref().filter(|c| !c.trim().is_empty()) {
            eylem = Deyim::Eger {
                kosul: ifade_ayristir(kosul, k, "kesme noktasının koşulu")?,
                govde: vec![eylem],
                degilse: vec![],
            };
        }
        let mut yeni = Some(vec![eylem]);
        for f in p.islevler.iter_mut() {
            yerlestir_blok(&mut f.govde, dosya, kb.satir, &mut yeni);
        }
        yerlestir_blok(&mut p.ana, dosya, kb.satir, &mut yeni);
        if yeni.is_some() {
            return Err(Hata::yeni(
                k,
                format!(
                    "{}. satırdaki {} bir deyime denk gelmiyor (boş satır, yorum ya da başlık olabilir)",
                    kb.satir,
                    if kb.gunluk.is_some() { "günlük noktası" } else { "koşullu kesme noktası" }
                ),
            ));
        }
    }
    Ok(())
}
