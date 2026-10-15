//! Yalnızca Stüdyo'nun ihtiyaç duyduğu kadar küçük bir HTTP/1.1 sunucu katmanı.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

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
    /// Stüdyo'nun kendi sayfası: içerik güvenlik politikası eklenir.
    pub csp: bool,
}

impl Yanit {
    pub fn json(deger: &serde_json::Value) -> Self {
        Yanit {
            durum: 200,
            tur: "application/json; charset=utf-8",
            govde: deger.to_string().into_bytes(),
            csp: false,
        }
    }

    pub fn hata(durum: u16, mesaj: &str) -> Self {
        Yanit {
            durum,
            tur: "application/json; charset=utf-8",
            govde: serde_json::json!({ "hata": mesaj })
                .to_string()
                .into_bytes(),
            csp: false,
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

const EN_BUYUK_BASLIK: u64 = 64 * 1024;
/// İstek satırı ve başlıklar bu sürede gelmeli; gövde dahil tüm istek için üst sınır
/// `GOVDE_SURESI`. Zaman aşımı her okumaya değil isteğin tamamına uygulanır: baytları
/// tek tek yollayan bağlantı da işçiyi bu süreden uzun tutamaz.
const BASLIK_SURESI: Duration = Duration::from_secs(10);
const GOVDE_SURESI: Duration = Duration::from_secs(60);
/// Yanıtı okumayan istemci de işçiyi sonsuza dek tutmasın.
const YAZMA_SURESI: Duration = Duration::from_secs(30);

/// Her okumadan önce kalan süreyi zaman aşımı yapar; süre bitince okuma hata verir.
struct SureliAkis {
    akis: TcpStream,
    son: Instant,
}

impl Read for SureliAkis {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        let kalan = self.son.saturating_duration_since(Instant::now());
        if kalan.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        self.akis.set_read_timeout(Some(kalan))?;
        self.akis.read(b)
    }
}

pub fn oku(akis: &mut TcpStream) -> Option<Istek> {
    sureli_oku(akis, BASLIK_SURESI, GOVDE_SURESI)
}

fn sureli_oku(
    akis: &mut TcpStream,
    baslik_suresi: Duration,
    govde_suresi: Duration,
) -> Option<Istek> {
    let bas = Instant::now();
    let mut okuyucu = BufReader::new(SureliAkis {
        akis: akis.try_clone().ok()?,
        son: bas + baslik_suresi,
    });
    // İstek satırı ve başlıklar en çok 64 KB
    let mut sinirli = (&mut okuyucu).take(EN_BUYUK_BASLIK);
    let mut ilk = String::new();
    sinirli.read_line(&mut ilk).ok()?;
    let mut parcalar = ilk.split_whitespace();
    let yontem = parcalar.next()?.to_string();
    let hedef = parcalar.next()?.to_string();

    let mut basliklar = HashMap::new();
    loop {
        let mut satir = String::new();
        if sinirli.read_line(&mut satir).ok()? == 0 {
            return None;
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
    okuyucu.get_mut().son = bas + govde_suresi;
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
        503 => "Service Unavailable",
        _ => "Error",
    };
    // Betik yalnızca Stüdyo'nun kendi dosyalarından yüklenir (sayfaya sızan bir betik
    // çalışmaz); eklenti, <base> ve başka sitenin çerçevesi kapalı. Masaüstü kabuğunun
    // (Tauri) iletişimi bozulmasın diye bağlantı ve görsel kaynakları kısıtlanmaz.
    let csp = if y.csp {
        "Content-Security-Policy: script-src 'self'; object-src 'none'; base-uri 'none'; \
         frame-ancestors 'self'; form-action 'self'\r\n"
    } else {
        ""
    };
    let baslik = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\nX-Frame-Options: SAMEORIGIN\r\n{csp}\
         Connection: close\r\n\r\n",
        y.durum,
        durum_metni,
        y.tur,
        y.govde.len()
    );
    let _ = akis.set_write_timeout(Some(YAZMA_SURESI));
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
    use std::io::Write;
    use std::time::{Duration, Instant};

    #[test]
    fn bayt_bayt_gelen_istek_sure_sinirini_asamaz() {
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let adres = l.local_addr().unwrap();
        let yollayici = std::thread::spawn(move || {
            let mut c = TcpStream::connect(adres).unwrap();
            // Her okuma zaman aşımından kısa aralıklarla tek bayt: eski kodda hiç bitmezdi.
            for b in b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nX: "
                .iter()
                .cycle()
                .take(40)
            {
                if c.write_all(&[*b]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        let (mut akis, _) = l.accept().unwrap();
        let bas = Instant::now();
        let istek = sureli_oku(&mut akis, Duration::from_secs(1), Duration::from_secs(2));
        assert!(istek.is_none());
        assert!(
            bas.elapsed() < Duration::from_secs(3),
            "{:?}",
            bas.elapsed()
        );
        drop(akis);
        yollayici.join().unwrap();
    }

    #[test]
    fn tam_istek_govdesiyle_okunur() {
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let adres = l.local_addr().unwrap();
        std::thread::spawn(move || {
            let mut c = TcpStream::connect(adres).unwrap();
            c.write_all(b"POST /api/a?x=1 HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}")
                .unwrap();
            std::thread::sleep(Duration::from_millis(200));
        });
        let (mut akis, _) = l.accept().unwrap();
        let i = oku(&mut akis).unwrap();
        assert_eq!(
            (i.yontem.as_str(), i.yol.as_str(), i.sorgu("x")),
            ("POST", "/api/a", "1")
        );
        assert_eq!(i.govde, b"{}");
    }

    use super::*;

    #[test]
    fn yuzde_cozme() {
        assert_eq!(yuzde_coz("%C3%A7al%C4%B1%C5%9F"), "çalış");
        assert_eq!(yuzde_coz("a+b%2Fc"), "a b/c");
        assert_eq!(yuzde_coz("%zz%"), "%zz%");
    }
}
