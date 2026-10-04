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
    /// Duruma özel öneri (ör. "bunu mu demek istediniz", başka dillerdeki karşılık);
    /// varsa genel ipucunun yerine gösterilir.
    pub oneri: Option<String>,
}

impl Hata {
    pub fn yeni(konum: Konum, mesaj: impl Into<String>) -> Self {
        Hata {
            mesaj: mesaj.into(),
            konum,
            ipucu: None,
            oneri: None,
        }
    }

    pub fn ipucu(mut self, ipucu: impl Into<String>) -> Self {
        self.ipucu = Some(ipucu.into());
        self
    }

    pub fn oneri(mut self, oneri: impl Into<String>) -> Self {
        self.oneri = Some(oneri.into());
        self
    }

    /// Gösterilecek ipucu: özel öneri, yoksa genel ipucu.
    pub fn gosterilecek_ipucu(&self) -> Option<&str> {
        self.oneri.as_deref().or(self.ipucu.as_deref())
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
            // Çok uzun satırlar sütunun çevresinden kısaltılarak gösterilir.
            const GENISLIK: usize = 120;
            let sutun = self.konum.sutun.saturating_sub(1);
            let (satir, sutun) = if satir.chars().count() > GENISLIK {
                let bas = sutun.saturating_sub(GENISLIK / 2);
                let mut parca: String = satir.chars().skip(bas).take(GENISLIK).collect();
                if satir.chars().count() > bas + GENISLIK {
                    parca.push('…');
                }
                if bas > 0 {
                    (format!("…{parca}"), sutun - bas + 1)
                } else {
                    (parca, sutun)
                }
            } else {
                (satir.to_string(), sutun)
            };
            let isaret = " ".repeat(sutun);
            s.push_str(&format!(
                "{bosluk} |\n{no} | {satir}\n{bosluk} | {isaret}^\n"
            ));
        }
        if let Some(i) = self.gosterilecek_ipucu() {
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
