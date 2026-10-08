//! Hızlı düzelt: derleme hatasının ipucundan otomatik düzeltme çıkarır.
//!
//! - `bunu mu demek istediniz: toplam` → hatanın yerindeki kelime `toplam` olur.
//! - `şöyle yazın: "merhaba"'yı yaz.` → hata satırın başındaysa satır bu yazımla değişir.
//!
//! Stüdyo hatanın yanında "Düzelt" düğmesi gösterir; dil sunucusu (LSP) aynı düzeltmeyi
//! "quick fix" olarak sunar.

/// Kaynakta değiştirilecek yer. Satır ve sütun 1'den başlar; sütun ve uzunluk karakter sayısıdır.
#[derive(Debug, Clone, PartialEq)]
pub struct Duzeltme {
    pub satir: usize,
    pub sutun: usize,
    pub uzunluk: usize,
    pub yeni: String,
    /// Düğmede gösterilecek açıklama
    pub baslik: String,
}

fn kelime_karakteri(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn duzeltme(kaynak: &str, satir: usize, sutun: usize, ipucu: Option<&str>) -> Option<Duzeltme> {
    let ipucu = ipucu?;
    let metin = kaynak.lines().nth(satir.checked_sub(1)?)?;
    let karakterler: Vec<char> = metin.chars().collect();
    let bas = sutun.checked_sub(1)?;

    if let Some(oneri) = ipucu.strip_prefix("bunu mu demek istediniz: ") {
        let oneri = oneri.trim_end_matches('?').trim_end_matches("(...)").trim();
        if oneri.is_empty() || !oneri.chars().all(kelime_karakteri) {
            return None;
        }
        if !karakterler.get(bas).copied().is_some_and(kelime_karakteri) {
            return None;
        }
        let mut son = bas;
        while son < karakterler.len() && kelime_karakteri(karakterler[son]) {
            son += 1;
        }
        let kelime: String = karakterler[bas..son].iter().collect();
        if kelime == oneri {
            return None;
        }
        return Some(Duzeltme {
            satir,
            sutun,
            uzunluk: son - bas,
            yeni: oneri.to_string(),
            baslik: format!("'{kelime}' yerine '{oneri}' yaz"),
        });
    }

    if let Some(yazim) = ipucu.strip_prefix("şöyle yazın: ") {
        let yazim = yazim.trim();
        // Yalnızca bütün satır yanlışsa (hata satırın ilk kelimesinde) ve öneri tek satırsa.
        let girinti = karakterler.iter().take_while(|c| c.is_whitespace()).count();
        if bas != girinti || yazim.is_empty() || yazim.contains('\n') {
            return None;
        }
        let uzunluk = metin.trim_end().chars().count() - bas;
        return Some(Duzeltme {
            satir,
            sutun,
            uzunluk,
            yeni: yazim.to_string(),
            baslik: format!("Düzelt: {yazim}"),
        });
    }
    None
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn oneriden_duzeltme() {
        let d = duzeltme(
            "toplam = 5\ntoplm'ı yaz.\n",
            2,
            1,
            Some("bunu mu demek istediniz: toplam"),
        )
        .unwrap();
        assert_eq!(
            (d.satir, d.sutun, d.uzunluk, d.yeni.as_str()),
            (2, 1, 5, "toplam")
        );
        let d = duzeltme(
            "x = uzunlk([1])\n",
            1,
            5,
            Some("bunu mu demek istediniz: uzunluk(...)"),
        )
        .unwrap();
        assert_eq!((d.sutun, d.uzunluk, d.yeni.as_str()), (5, 6, "uzunluk"));
        let d = duzeltme("x = 5\n    yaz(x)  \n", 2, 5, Some("şöyle yazın: x'i yaz.")).unwrap();
        assert_eq!((d.sutun, d.uzunluk, d.yeni.as_str()), (5, 6, "x'i yaz."));
        // Satırın ortasındaki hata ya da başka ipuçları düzeltilmez.
        assert!(duzeltme("a = yaz(x)\n", 1, 5, Some("şöyle yazın: x'i yaz.")).is_none());
        assert!(duzeltme("x\n", 1, 1, Some("tipler: sayı")).is_none());
        assert!(duzeltme("x\n", 1, 1, None).is_none());
    }
}
