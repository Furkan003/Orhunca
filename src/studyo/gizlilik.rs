//! Akış parçaları arasında bölünen gizli değerler de çıktıya verilmez.
pub struct Gizleyici {
    degerler: Vec<String>,
    tampon: String,
}
impl Gizleyici {
    pub fn yeni(mut degerler: Vec<String>) -> Self {
        degerler.retain(|s| !s.is_empty());
        degerler.sort_by_key(|s| std::cmp::Reverse(s.len()));
        degerler.dedup();
        Self {
            degerler,
            tampon: String::new(),
        }
    }
    pub fn ekle(&mut self, s: &str) -> String {
        self.tampon.push_str(s);
        for d in &self.degerler {
            self.tampon = self.tampon.replace(d, "[gizlendi]");
        }
        let en = self.degerler.iter().map(String::len).max().unwrap_or(0);
        let bas = self.tampon.len().saturating_sub(en.saturating_sub(1));
        let sinir = self
            .tampon
            .char_indices()
            .map(|(i, _)| i)
            .filter(|i| *i >= bas)
            .find(|i| {
                let son = &self.tampon[*i..];
                self.degerler.iter().any(|d| d.starts_with(son))
            })
            .unwrap_or(self.tampon.len());
        let sonuc = self.tampon[..sinir].to_string();
        self.tampon.drain(..sinir);
        sonuc
    }
    pub fn bitir(&mut self) -> String {
        std::mem::take(&mut self.tampon)
    }
}
#[cfg(test)]
mod sinamalar {
    use super::*;
    #[test]
    fn bolunen_anahtar_ve_utf8_gizlenir() {
        let mut g = Gizleyici::yeni(vec!["çok_gizli_anahtar".into(), "kisa".into()]);
        let mut s = String::new();
        for p in ["baş: ço", "k_gizli_", "anahtar son ki", "sa! ç"] {
            s.push_str(&g.ekle(p));
        }
        s.push_str(&g.bitir());
        assert_eq!(s, "baş: [gizlendi] son [gizlendi]! ç");
    }
}
