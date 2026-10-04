//! Web çatısının uçtan uca testi: yollar, görünümler, modeller, formlar, JSON,
//! statik dosyalar ve hata sayfaları. Sunucu, çöp toplayıcı çok sık çalışacak
//! biçimde başlatılır; istekler arasında hiçbir canlı nesne kaybolmamalıdır.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const SUNUCU: &str = r#"model Ürün:
    ad: metin, zorunlu, en_fazla 40
    fiyat: ondalık, en_az 0

al "/":
    döndür görünüm("liste", Ürün.hepsi())

al "/api/ürünler":
    döndür Ürün.hepsi()

al "/ürünler/{kimlik: sayı}":
    ü = Ürün.bul(kimlik)
    eğer ü.kimlik 0'a eşitse:
        döndür yanıt(404, "<h1>Ürün bulunamadı</h1>")
    döndür "<h1>" + kaçır(ü.ad) + "</h1>"

gönder "/ürünler":
    ü = Ürün.formdan(istek)
    eğer değil ü.geçerli_mi() ise:
        döndür yanıt(422, birleştir(ü.hatalar(), "; "))
    ü'yü kaydet.
    döndür yönlendir("/ürünler/" + ü.kimlik)

al "/selam":
    ad = "misafir"
    eğer içerir(istek.sorgu, "ad") ise:
        ad = istek.sorgu["ad"]
    y = yanıt(200, "Merhaba " + kaçır(ad))
    y.başlıklar["X-Orhunca"] = "evet"
    döndür y

al "/hata":
    l = []
    döndür "x" + l[3]

al "/güvenli":
    dene:
        l = [1]
        döndür "değer " + l[5]
    yakala h:
        döndür yanıt(400, "yakalandı: " + h)
"#;

const DUZEN: &str = r#"<!DOCTYPE html>
<html lang="tr"><head><meta charset="utf-8"><title>@başlık</title></head>
<body>
@içerik
</body></html>
"#;

const LISTE: &str = r#"@model liste<Ürün>
@düzen "düzen"
@başlık "Ürünler"
<h1>@uzunluk(model) ürün</h1>
@eğer uzunluk(model) 0'a eşitse {
<p>Henüz ürün yok.</p>
} @değilse {
<ul>
    @her ü için model'den {
    <li><a href="/ürünler/@ü.kimlik">@ü.ad</a> @para(ü.fiyat) TL</li>
    }
</ul>
}
"#;

struct Sunucu {
    cocuk: Child,
    kapi: u16,
    klasor: PathBuf,
}

impl Drop for Sunucu {
    fn drop(&mut self) {
        let _ = self.cocuk.kill();
        let _ = self.cocuk.wait();
        let _ = std::fs::remove_dir_all(&self.klasor);
    }
}

fn yaz(yol: &Path, icerik: &str) {
    std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
    std::fs::write(yol, icerik).unwrap();
}

fn baslat() -> Sunucu {
    let klasor = std::env::temp_dir().join(format!("orhunca-web-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&klasor);
    yaz(&klasor.join("sunucu.ohc"), SUNUCU);
    yaz(
        &klasor.join("dükkan.ohcproj"),
        "ad = \"dükkan\"\ngiriş = \"sunucu.ohc\"\n",
    );
    yaz(&klasor.join("görünümler/düzen.ohchtml"), DUZEN);
    yaz(&klasor.join("görünümler/liste.ohchtml"), LISTE);
    yaz(&klasor.join("statik/stil.css"), "h1 { color: teal }\n");
    let derle = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["derle", "sunucu.ohc", "-o", "sunucu"])
        .current_dir(&klasor)
        .output()
        .unwrap();
    assert!(
        derle.status.success(),
        "{}",
        String::from_utf8_lossy(&derle.stderr)
    );
    let mut cocuk = Command::new(klasor.join("sunucu"))
        .current_dir(&klasor)
        .env("ORHUNCA_KAPI", "0")
        .env("ORHUNCA_GC_ESIK", "8192")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut satir = String::new();
    let mut okuyucu = BufReader::new(cocuk.stdout.take().unwrap());
    okuyucu.read_line(&mut satir).unwrap();
    let kapi = satir
        .trim()
        .split_once("Sunucu dinleniyor: http://localhost:")
        .unwrap_or_else(|| panic!("beklenmeyen çıktı: {satir}"))
        .1
        .parse()
        .unwrap();
    // İstek günlüğü okunmazsa boru dolup sunucuyu durdurabilir.
    std::thread::spawn(move || {
        let mut s = String::new();
        while okuyucu.read_line(&mut s).is_ok_and(|n| n > 0) {
            s.clear();
        }
    });
    Sunucu {
        cocuk,
        kapi,
        klasor,
    }
}

struct Yanit {
    durum: u16,
    basliklar: String,
    govde: String,
}

impl Sunucu {
    fn istek(&self, yontem: &str, yol: &str, govde: Option<(&str, &str)>) -> Yanit {
        let mut a = TcpStream::connect(("127.0.0.1", self.kapi)).unwrap();
        let mut istek = format!("{yontem} {yol} HTTP/1.1\r\nHost: localhost\r\n");
        if let Some((tur, g)) = govde {
            istek.push_str(&format!(
                "Content-Type: {tur}\r\nContent-Length: {}\r\n\r\n{g}",
                g.len()
            ));
        } else {
            istek.push_str("\r\n");
        }
        a.write_all(istek.as_bytes()).unwrap();
        let mut y = Vec::new();
        a.read_to_end(&mut y).unwrap();
        let y = String::from_utf8(y).unwrap();
        let (basliklar, govde) = y.split_once("\r\n\r\n").unwrap();
        Yanit {
            durum: basliklar
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse()
                .unwrap(),
            basliklar: basliklar.to_string(),
            govde: govde.to_string(),
        }
    }

    fn form(&self, yol: &str, govde: &str) -> Yanit {
        self.istek(
            "POST",
            yol,
            Some(("application/x-www-form-urlencoded", govde)),
        )
    }
}

#[test]
fn web_cati_uctan_uca() {
    let s = baslat();

    let y = s.istek("GET", "/", None);
    assert_eq!(y.durum, 200);
    assert!(y.basliklar.contains("text/html; charset=utf-8"));
    assert!(y.govde.contains("<title>Ürünler</title>"), "{}", y.govde);
    assert!(y.govde.contains("<h1>0 ürün</h1>"), "{}", y.govde);
    assert!(y.govde.contains("<p>Henüz ürün yok.</p>"), "{}", y.govde);

    // Form: geçerli kayıt 303 ile yönlendirir, geçersiz kayıt Türkçe hatalar verir.
    let y = s.form("/%C3%BCr%C3%BCnler", "ad=%3CKalem%3E&fiyat=12%2C5");
    assert_eq!(y.durum, 303);
    assert!(
        y.basliklar.contains("Location: /%C3%BCr%C3%BCnler/1"),
        "{}",
        y.basliklar
    );
    let y = s.form("/%C3%BCr%C3%BCnler", "ad=&fiyat=abc");
    assert_eq!(y.durum, 422);
    assert_eq!(y.govde, "Ad boş bırakılamaz; Fiyat bir sayı olmalı");
    let y = s.istek(
        "POST",
        "/%C3%BCr%C3%BCnler",
        Some(("application/json", r#"{"ad": "Silgi", "fiyat": 3}"#)),
    );
    assert_eq!(y.durum, 303);

    // Görünüm, kaçırma ve para biçimi
    let y = s.istek("GET", "/", None);
    assert!(y.govde.contains("<h1>2 ürün</h1>"), "{}", y.govde);
    assert!(
        y.govde
            .contains("<li><a href=\"/ürünler/1\">&lt;Kalem&gt;</a> 12,50 TL</li>"),
        "{}",
        y.govde
    );

    // Yol parametresi ve özel durum kodu
    assert_eq!(
        s.istek("GET", "/%C3%BCr%C3%BCnler/2", None).govde,
        "<h1>Silgi</h1>"
    );
    assert_eq!(s.istek("GET", "/%C3%BCr%C3%BCnler/99", None).durum, 404);
    assert_eq!(s.istek("GET", "/%C3%BCr%C3%BCnler/abc", None).durum, 404);

    // JSON
    let y = s.istek("GET", "/api/%C3%BCr%C3%BCnler", None);
    assert!(y.basliklar.contains("application/json"));
    assert_eq!(
        y.govde,
        r#"[{"kimlik":1,"ad":"<Kalem>","fiyat":12.5},{"kimlik":2,"ad":"Silgi","fiyat":3.0}]"#
    );

    // Sorgu, kaçırma ve özel başlık
    let y = s.istek("GET", "/selam?ad=%C3%A7ay+%3Cb%3E", None);
    assert_eq!(y.govde, "Merhaba çay &lt;b&gt;");
    assert!(y.basliklar.contains("X-Orhunca: evet"), "{}", y.basliklar);

    // Statik dosya, bilinmeyen yol, yanlış yöntem, HEAD
    let y = s.istek("GET", "/stil.css", None);
    assert_eq!(y.govde, "h1 { color: teal }\n");
    assert!(y.basliklar.contains("text/css"));
    assert_eq!(s.istek("GET", "/yok", None).durum, 404);
    assert_eq!(s.istek("DELETE", "/", None).durum, 405);
    let y = s.istek("HEAD", "/", None);
    assert_eq!((y.durum, y.govde.as_str()), (200, ""));

    // Çalışma hatası 500 sayfası verir ama sunucu çalışmayı sürdürür.
    let y = s.istek("GET", "/hata", None);
    assert_eq!(y.durum, 500);
    assert!(
        y.govde.contains("Çalışma hatası (satır 34)") && y.govde.contains("liste sınırı aşıldı"),
        "{}",
        y.govde
    );

    // Yolun içinde yakalanan hata 500 vermez; ardından yakalanmayan hata yine 500 verir.
    let y = s.istek("GET", "/g%C3%BCvenli", None);
    assert_eq!(y.durum, 400, "{}", y.govde);
    assert!(
        y.govde.starts_with("yakalandı: liste sınırı aşıldı"),
        "{}",
        y.govde
    );
    assert_eq!(s.istek("GET", "/hata", None).durum, 500);

    // Çok sayıda istek: çöp toplayıcı defalarca çalışır, veriler bozulmamalı.
    for i in 0..60 {
        let y = s.form(
            "/%C3%BCr%C3%BCnler",
            &format!("ad=%C3%BCr%C3%BCn{i}&fiyat={i}"),
        );
        assert_eq!(y.durum, 303);
    }
    let y = s.istek("GET", "/", None);
    assert!(y.govde.contains("<h1>62 ürün</h1>"), "{}", y.govde);
    assert!(y.govde.contains(">ürün59</a> 59,00 TL"), "{}", y.govde);
    let kayitlar = std::fs::read_to_string(s.klasor.join("veri/Ürün.json")).unwrap();
    assert!(kayitlar.starts_with("[\n  {\"kimlik\": 1, \"ad\": \"<Kalem>\""));
    assert_eq!(kayitlar.lines().count(), 64);
}
