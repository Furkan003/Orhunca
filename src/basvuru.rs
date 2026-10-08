//! Yerleşik işlev başvurusu: `orhunca başvuru [arama]`, docs/basvuru.md ve sitenin
//! başvuru sayfası aynı listeden (yerlesik.rs) üretilir.

use crate::yerlesik::{Yerlesik, BOLUMLER, YERLESIKLER};

/// Derleyicinin sıradan deyimlere çevirdiği sınama işlevleri (bkz. sinama.rs).
const SINAMA: &[Yerlesik] = &[
    Yerlesik {
        ad: "doğrula",
        kullanim: "doğrula(koşul) · doğrula(koşul, açıklama)",
        aciklama: "Koşul yanlışsa satırı ve açıklamayı gösteren bir çalışma hatası verir: doğrula(yaş >= 0, \"yaş eksi olamaz\").",
    },
    Yerlesik {
        ad: "eşit_olmalı",
        kullanim: "eşit_olmalı(gerçek, beklenen)",
        aciklama: "İki değer farklıysa ikisini de gösteren bir çalışma hatası verir. Her tiple çalışır. Sınamaları çalıştırmak için: orhunca sına.",
    },
];

fn tumu() -> impl Iterator<Item = &'static Yerlesik> {
    YERLESIKLER.iter().chain(SINAMA.iter())
}

fn bolum(y: &Yerlesik) -> &'static str {
    if SINAMA.iter().any(|s| s.ad == y.ad) {
        "Sınama"
    } else {
        crate::yerlesik::bolum(y)
    }
}

fn bolumler() -> impl Iterator<Item = &'static str> {
    BOLUMLER.iter().map(|(b, _)| *b).chain(["Sınama"])
}
use serde_json::json;

/// Bölümün kısa açıklaması (yalnızca bazı ortamlarda çalışan işlevler için).
fn bolum_notu(b: &str) -> Option<&'static str> {
    Some(match b {
        "Telefon" => "Android/iOS uygulamasında ve tarayıcıda çalışır; bilgisayar programında etkisizdir.",
        "Arayüz" => "Arayüz programlarında (tarayıcıda ve masaüstü paketinde) çalışır.",
        "Oyun" => "`oyun_alanı(...)` içindeki `her_karede:` bloğunda çizer; bilgisayar programında etkisizdir.",
        "Sınama" => "Sınama dosyalarında (`*_sına.ohc`) ve programın her yerinde kullanılabilir; ayrıntılar dil rehberinin Sınamalar bölümünde.",
        "Web" => "İnternetten veri alma ve web sunucusu yanıtları (`görünüm`, `yanıt`, `yönlendir`...).",
        _ => return None,
    })
}

/// Aramada Türkçe harfler sadeleştirilir: `buyuk` → `büyük` bulunur.
fn sade(m: &str) -> String {
    m.chars()
        .flat_map(|c| match c {
            'ç' | 'Ç' => Some('c'),
            'ğ' | 'Ğ' => Some('g'),
            'ı' | 'I' | 'İ' => Some('i'),
            'ö' | 'Ö' => Some('o'),
            'ş' | 'Ş' => Some('s'),
            'ü' | 'Ü' => Some('u'),
            'â' => Some('a'),
            'î' => Some('i'),
            '\u{307}' => None,
            c => c.to_lowercase().next(),
        })
        .collect()
}

/// Aramaya uyan işlevler: önce adı uyanlar, sonra açıklaması uyanlar.
pub fn ara(sorgu: &str) -> Vec<&'static Yerlesik> {
    let s = sade(sorgu.trim());
    if s.is_empty() {
        return tumu().collect();
    }
    let mut adda: Vec<_> = tumu().filter(|y| sade(y.ad).contains(&s)).collect();
    adda.sort_by_key(|y| (sade(y.ad) != s, !sade(y.ad).starts_with(&s)));
    let aciklamada = tumu().filter(|y| {
        !sade(y.ad).contains(&s) && (sade(y.aciklama).contains(&s) || sade(y.kullanim).contains(&s))
    });
    adda.into_iter().chain(aciklamada).collect()
}

fn md_kac(m: &str) -> String {
    m.replace('|', "\\|")
}

/// docs/basvuru.md
pub fn markdown() -> String {
    let mut m = String::from(
        "# Yerleşik işlevler başvurusu\n\n\
         <!-- Bu dosya üretilir: cargo run -- başvuru --md > docs/basvuru.md -->\n\n\
         Orhunca'nın her programda hazır bulunan işlevleri. Terminalde aramak için\n\
         `orhunca başvuru <kelime>` (ör. `orhunca başvuru tarih`); Stüdyo'da **Öğren** sayfasında\n\
         ve kodda işlevin üzerine gelince de görünür. Dilin kendisi (değişkenler, koşullar,\n\
         döngüler, fiiller, modeller) için: [dil rehberi](dil-rehberi.md).\n\n",
    );
    m.push_str("**Bölümler:** ");
    m.push_str(&bolumler().collect::<Vec<_>>().join(" · "));
    m.push('\n');
    for b in bolumler() {
        m.push_str(&format!("\n## {b}\n\n"));
        if let Some(n) = bolum_notu(b) {
            m.push_str(&format!("{n}\n\n"));
        }
        m.push_str("| İşlev | Kullanım | Açıklama |\n|---|---|---|\n");
        for y in tumu().filter(|y| bolum(y) == b) {
            m.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                y.ad,
                md_kac(y.kullanim),
                md_kac(y.aciklama)
            ));
        }
    }
    m
}

pub fn json() -> serde_json::Value {
    json!({
        "bolumler": bolumler().map(|b| json!({ "ad": b, "not": bolum_notu(b) })).collect::<Vec<_>>(),
        "islevler": tumu().map(|y| json!({
            "ad": y.ad, "kullanim": y.kullanim, "aciklama": y.aciklama, "bolum": bolum(y),
        })).collect::<Vec<_>>(),
    })
}

/// `orhunca başvuru [arama] [--md | --json]`
pub fn komut(args: &[String]) -> Result<(), String> {
    let mut sorgu = Vec::new();
    let mut bicim = "";
    for a in args {
        match a.as_str() {
            "--md" => bicim = "md",
            "--json" => bicim = "json",
            a if a.starts_with("--") => {
                return Err(format!(
                    "bilinmeyen seçenek '{a}'\nKullanım: orhunca başvuru [arama] [--md | --json]"
                ))
            }
            a => sorgu.push(a),
        }
    }
    match bicim {
        "md" => {
            print!("{}", markdown());
            return Ok(());
        }
        "json" => {
            println!("{}", json());
            return Ok(());
        }
        _ => {}
    }
    let sorgu = sorgu.join(" ");
    let bulunan = ara(&sorgu);
    if bulunan.is_empty() {
        return Err(format!(
            "'{sorgu}' ile ilgili yerleşik işlev bulunamadı. Bütün liste: orhunca başvuru"
        ));
    }
    let mut onceki = "";
    for y in bulunan {
        let b = bolum(y);
        if sorgu.is_empty() && b != onceki {
            println!("\n{b}");
            onceki = b;
        }
        println!("  {}\n      {}", y.kullanim, y.aciklama);
    }
    Ok(())
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn arama() {
        let adlar = |s: &str| ara(s).iter().map(|y| y.ad).collect::<Vec<_>>();
        assert_eq!(adlar("uzunluk")[0], "uzunluk");
        // Türkçe harfsiz yazım da bulunur
        assert!(adlar("buyuk_harf").contains(&"büyük_harf"));
        // Açıklamada geçenler de bulunur, adı uyanlardan sonra
        let k = adlar("karekök");
        assert_eq!(k[0], "karekök");
        assert!(ara("").len() == YERLESIKLER.len() + SINAMA.len());
        assert_eq!(adlar("esit_olmali")[0], "eşit_olmalı");
        assert!(ara("bulunmayan_bir_sey_xyz").is_empty());
    }

    /// docs/basvuru.md yerleşik işlev listesiyle aynı kalmalı.
    #[test]
    fn belge_guncel() {
        let yol = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/basvuru.md");
        let belge = std::fs::read_to_string(yol)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            belge == markdown(),
            "docs/basvuru.md eski; yeniden üretin: cargo run -- başvuru --md > docs/basvuru.md"
        );
    }
}
