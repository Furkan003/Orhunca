//! Yalnızca Stüdyo'nun ihtiyaç duyduğu kadar küçük bir HTTP/1.1 sunucu katmanı.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

const EN_BUYUK_GOVDE: usize = 32 * 1024 * 1024;

pub struct Istek {
    pub yontem: String,
    pub yol: String,
    pub sorgu: HashMap<String, String>,
    /// Küçük harfli başlık adları
    pub basliklar: HashMap<String, String>,
    pub govde: Vec<u8>,
}

impl Istek {
    pub fn sorgu(&self, ad: &str) -> &str {
        self.sorgu.get(ad).map(String::as_str).unwrap_or("")
    }

    pub fn baslik(&self, ad: &str) -> &str {
        self.basliklar.get(ad).map(String::as_str).unwrap_or("")
    }
}

pub struct Yanit {
    pub durum: u16,
    pub tur: &'static str,
    pub govde: Vec<u8>,
}

impl Yanit {
    pub fn json(deger: &serde_json::Value) -> Self {
        Yanit {
            durum: 200,
            tur: "application/json; charset=utf-8",
            govde: deger.to_string().into_bytes(),
        }
    }

    pub fn hata(durum: u16, mesaj: &str) -> Self {
        Yanit {
            durum,
            tur: "application/json; charset=utf-8",
            govde: serde_json::json!({ "hata": mesaj })
                .to_string()
                .into_bytes(),
        }
    }
}

/// `%C3%A7` gibi yüzde kodlamalarını çözer (`+` boşluk olur).
pub fn yuzde_coz(s: &str) -> String {
    let b = s.as_bytes();
    let mut cikti = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
                {
                    Ok(v) => {
                        cikti.push(v);
                        i += 3;
                        continue;
                    }
                    Err(_) => cikti.push(b'%'),
                }
            }
            b'+' => cikti.push(b' '),
            c => cikti.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&cikti).into_owned()
}

pub fn oku(akis: &mut TcpStream) -> Option<Istek> {
    let mut okuyucu = BufReader::new(akis.try_clone().ok()?);
    let mut ilk = String::new();
    okuyucu.read_line(&mut ilk).ok()?;
    let mut parcalar = ilk.split_whitespace();
    let yontem = parcalar.next()?.to_string();
    let hedef = parcalar.next()?.to_string();

    let mut basliklar = HashMap::new();
    loop {
        let mut satir = String::new();
        if okuyucu.read_line(&mut satir).ok()? == 0 {
            break;
        }
        let satir = satir.trim_end();
        if satir.is_empty() {
            break;
        }
        if let Some((a, d)) = satir.split_once(':') {
            basliklar.insert(a.trim().to_lowercase(), d.trim().to_string());
        }
    }

    let uzunluk: usize = basliklar
        .get("content-length")
        .and_then(|u| u.parse().ok())
        .unwrap_or(0);
    if uzunluk > EN_BUYUK_GOVDE {
        return None;
    }
    let mut govde = vec![0; uzunluk];
    okuyucu.read_exact(&mut govde).ok()?;

    let (yol, sorgu_metni) = hedef.split_once('?').unwrap_or((&hedef, ""));
    let sorgu = sorgu_metni
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (a, d) = p.split_once('=').unwrap_or((p, ""));
            (yuzde_coz(a), yuzde_coz(d))
        })
        .collect();
    Some(Istek {
        yontem,
        yol: yuzde_coz(yol),
        sorgu,
        basliklar,
        govde,
    })
}

pub fn yaz(akis: &mut TcpStream, y: &Yanit) {
    let durum_metni = match y.durum {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let baslik = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        y.durum,
        durum_metni,
        y.tur,
        y.govde.len()
    );
    let _ = akis.write_all(baslik.as_bytes());
    let _ = akis.write_all(&y.govde);
    let _ = akis.flush();
}

pub fn icerik_turu(yol: &str) -> &'static str {
    match yol.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "woff2" => "font/woff2",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn yuzde_cozme() {
        assert_eq!(yuzde_coz("%C3%A7al%C4%B1%C5%9F"), "çalış");
        assert_eq!(yuzde_coz("a+b%2Fc"), "a b/c");
        assert_eq!(yuzde_coz("%zz%"), "%zz%");
    }
}
