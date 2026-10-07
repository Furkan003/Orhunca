//! Dil sürümü ve geriye uyumluluk.
//!
//! Kural (docs/uyumluluk.md): aynı dil sürümünde yazılmış hiçbir program Orhunca
//! güncellenince bozulmaz. Bir yazım değişecekse eski yazım önce en az bir sürüm boyunca
//! kabul edilip uyarı verir (`GOCLER`); `orhunca düzelt` eski yazımı kendiliğinden yenisine
//! çevirir. Eski yazım ancak dil sürümü (`DIL_SURUMU`) artarken kaldırılabilir.

use crate::hata::Konum;
use crate::sozcuk::{sozcukle, Tok};

/// Derleyicinin desteklediği dil sürümü. Projeler `.ohcproj` dosyasında `dil = "1"` ile
/// hangi sürüm için yazıldıklarını belirtir.
pub const DIL_SURUMU: u32 = 1;

/// Eskiyen bir yazım: `eski` kelimesi yerine `yeni` yazılmalı.
pub struct Goc {
    pub eski: &'static str,
    pub yeni: &'static str,
    /// Yazımın eskidiği Orhunca sürümü (ör. "0.8")
    pub surum: &'static str,
}

/// Eskiyen yazımlar. Henüz yok; bir yazım değiştiğinde buraya eklenir.
pub const GOCLER: &[Goc] = &[];

/// Kaynakta eskimiş yazımın geçtiği yerler: (konum, karakter uzunluğu, göç).
pub fn bul<'a>(kaynak: &str, gocler: &'a [Goc]) -> Vec<(Konum, usize, &'a Goc)> {
    let Ok(sozcukler) = sozcukle(kaynak) else {
        return Vec::new();
    };
    sozcukler
        .iter()
        .filter_map(|s| match &s.tok {
            Tok::Kelime(k) => gocler
                .iter()
                .find(|g| g.eski == k)
                .map(|g| (s.konum, g.eski.chars().count(), g)),
            _ => None,
        })
        .collect()
}

/// Eskimiş yazımları yenisine çevirir; (yeni kaynak, değişiklik sayısı).
pub fn uygula(kaynak: &str, gocler: &[Goc]) -> (String, usize) {
    let bulunan = bul(kaynak, gocler);
    if bulunan.is_empty() {
        return (kaynak.to_string(), 0);
    }
    // Satır başlarının bayt konumları; konumlar 1'den başlar, sütunlar karakter sayısıdır.
    let mut satir_baslari = vec![0];
    satir_baslari.extend(kaynak.match_indices('\n').map(|(i, _)| i + 1));
    let mut degisiklikler: Vec<(usize, usize, &str)> = bulunan
        .iter()
        .filter_map(|(k, uzunluk, g)| {
            let bas = *satir_baslari.get(k.satir.checked_sub(1)?)?;
            let satir = &kaynak[bas..];
            let (b, _) = satir.char_indices().nth(k.sutun.checked_sub(1)?)?;
            let s = satir
                .char_indices()
                .nth(k.sutun - 1 + uzunluk)
                .map_or(satir.len(), |(i, _)| i);
            (satir[b..s] == *g.eski).then_some((bas + b, bas + s, g.yeni))
        })
        .collect();
    degisiklikler.sort_by_key(|d| std::cmp::Reverse(d.0));
    let mut yeni = kaynak.to_string();
    for (b, s, m) in &degisiklikler {
        yeni.replace_range(*b..*s, m);
    }
    (yeni, degisiklikler.len())
}

/// Proje dosyasındaki dil sürümü bu derleyiciden yeniyse açıklayıcı hata.
pub fn surum_denetle(dil: Option<&str>) -> Result<(), String> {
    let Some(d) = dil else { return Ok(()) };
    match d.trim().parse::<u32>() {
        Ok(n) if n <= DIL_SURUMU => Ok(()),
        Ok(n) => Err(format!(
            "bu proje Orhunca dilinin {n}. sürümü için yazılmış; kurulu Orhunca ({}) en çok {DIL_SURUMU}. sürümü \
             tanıyor\nipucu: Orhunca'yı güncelleyin: orhunca güncelle",
            crate::SURUM
        )),
        Err(_) => Err(format!(
            "proje dosyasındaki dil sürümü geçersiz: '{d}' (ör. dil = \"{DIL_SURUMU}\")"
        )),
    }
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    const DENEME: &[Goc] = &[Goc {
        eski: "yazdır",
        yeni: "yaz",
        surum: "0.8",
    }];

    #[test]
    fn eskimis_yazim_bulunur_ve_cevrilir() {
        let k = "\"çık\"'ı yazdır.\nx = \"yazdır\"\n# yazdır\n\"a\"'yı yazdır.\n";
        let b = bul(k, DENEME);
        assert_eq!(b.len(), 2, "metin ve yorumdaki geçişler sayılmaz");
        assert_eq!((b[0].0.satir, b[0].0.sutun), (1, 9));
        let (yeni, n) = uygula(k, DENEME);
        assert_eq!(n, 2);
        assert_eq!(
            yeni,
            "\"çık\"'ı yaz.\nx = \"yazdır\"\n# yazdır\n\"a\"'yı yaz.\n"
        );
        assert_eq!(uygula("x = 1\n", DENEME), ("x = 1\n".to_string(), 0));
    }

    #[test]
    fn dil_surumu() {
        assert!(surum_denetle(None).is_ok());
        assert!(surum_denetle(Some("1")).is_ok());
        let h = surum_denetle(Some("99")).unwrap_err();
        assert!(h.contains("orhunca güncelle"), "{h}");
        assert!(surum_denetle(Some("bir")).is_err());
    }
}
