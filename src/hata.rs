//! Derleyici hataları ve kaynak kodu gösteren Türkçe hata çıktısı.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Konum {
    pub satir: usize,
    pub sutun: usize,
    /// Derlemedeki dosyanın sırası (0: ana dosya).
    pub dosya: usize,
}

#[derive(Debug, Clone)]
pub struct Hata {
    pub mesaj: String,
    pub konum: Konum,
    pub ipucu: Option<String>,
}

impl Hata {
    pub fn yeni(konum: Konum, mesaj: impl Into<String>) -> Self {
        Hata {
            mesaj: mesaj.into(),
            konum,
            ipucu: None,
        }
    }

    pub fn ipucu(mut self, ipucu: impl Into<String>) -> Self {
        self.ipucu = Some(ipucu.into());
        self
    }

    /// Hatayı dosya adı, satır ve işaretçiyle birlikte biçimlendirir.
    /// `dosyalar`: derlemedeki (ad, kaynak) çiftleri; `konum.dosya` buraya bir sıradır.
    pub fn goster(&self, dosyalar: &[(String, String)]) -> String {
        let (dosya, kaynak) = dosyalar
            .get(self.konum.dosya)
            .map(|(a, k)| (a.as_str(), k.as_str()))
            .unwrap_or(("?", ""));
        let mut s = format!(
            "hata: {}\n  --> {}:{}:{}\n",
            self.mesaj, dosya, self.konum.satir, self.konum.sutun
        );
        if let Some(satir) = kaynak.lines().nth(self.konum.satir.saturating_sub(1)) {
            let no = self.konum.satir.to_string();
            let bosluk = " ".repeat(no.len());
            let isaret = " ".repeat(self.konum.sutun.saturating_sub(1));
            s.push_str(&format!(
                "{bosluk} |\n{no} | {satir}\n{bosluk} | {isaret}^\n"
            ));
        }
        if let Some(i) = &self.ipucu {
            s.push_str(&format!("ipucu: {i}\n"));
        }
        s
    }
}

impl fmt::Display for Hata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.konum.satir, self.konum.sutun, self.mesaj
        )
    }
}

pub type Sonuc<T> = Result<T, Hata>;
