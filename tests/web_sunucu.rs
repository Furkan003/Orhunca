//! Web sunucusunun bağlantı katmanı: aynı anda birçok bağlantı (olay döngüsü),
//! açık kalan bağlantılar, oturum ve çerezler, dosya yükleme, parçalı gövde, HTTPS.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const SUNUCU: &str = r#"al "/sayaç":
    n = 0
    eğer içerir(istek.oturum, "n") ise:
        n = sayı(istek.oturum["n"])
    n += 1
    istek.oturum["n"] = metin(n)
    y = yanıt(200, "ziyaret " + n)
    y.çerezler["tema"] = "koyu mod"
    döndür y

al "/çıkış":
    istek.oturum = {}
    döndür "çıkıldı"

al "/çerez":
    eğer içerir(istek.çerezler, "tema") ise:
        döndür "tema=" + istek.çerezler["tema"]
    döndür "çerez yok"

gönder "/yükle":
    d = istek.dosyalar["belge"]
    taşındı = dosya_taşı(d.yol, "statik/son.bin")
    döndür d.ad + "|" + d.tür + "|" + d.boyut + "|" + istek.form["açıklama"] + "|" + d.yol + "|" + taşındı

gönder "/yankı":
    döndür istek.gövde
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

fn baslat(ad: &str, ortam: &[(&str, &str)], hazirla: impl FnOnce(&Path)) -> Sunucu {
    let klasor = std::env::temp_dir().join(format!("orhunca-sunucu-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&klasor);
    std::fs::create_dir_all(&klasor).unwrap();
    std::fs::write(klasor.join("sunucu.ohc"), SUNUCU).unwrap();
    hazirla(&klasor);
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
        .envs(ortam.iter().copied())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut satir = String::new();
    let mut okuyucu = BufReader::new(cocuk.stdout.take().unwrap());
    okuyucu.read_line(&mut satir).unwrap();
    let kapi = satir
        .trim()
        .rsplit(':')
        .next()
        .unwrap_or_else(|| panic!("beklenmeyen çıktı: {satir}"))
        .parse()
        .unwrap_or_else(|_| panic!("beklenmeyen çıktı: {satir}"));
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
    govde: Vec<u8>,
}

impl Yanit {
    fn metin(&self) -> String {
        String::from_utf8_lossy(&self.govde).into_owned()
    }

    fn cerezler(&self) -> Vec<String> {
        self.basliklar
            .lines()
            .filter_map(|s| s.strip_prefix("Set-Cookie: "))
            .map(str::to_string)
            .collect()
    }
}

/// Bir yanıtı Content-Length'e göre okur (bağlantı açık kalabilir).
fn yanit_oku(a: &mut BufReader<TcpStream>) -> Yanit {
    let mut basliklar = String::new();
    loop {
        let mut s = String::new();
        assert!(a.read_line(&mut s).unwrap() > 0, "bağlantı erken kapandı");
        if s == "\r\n" {
            break;
        }
        basliklar.push_str(&s);
    }
    let uzunluk: usize = basliklar
        .lines()
        .find_map(|s| s.strip_prefix("Content-Length: "))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let mut govde = vec![0; uzunluk];
    a.read_exact(&mut govde).unwrap();
    Yanit {
        durum: basliklar
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap(),
        basliklar,
        govde,
    }
}

fn baglan(kapi: u16) -> (TcpStream, BufReader<TcpStream>) {
    let a = TcpStream::connect(("127.0.0.1", kapi)).unwrap();
    a.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let b = BufReader::new(a.try_clone().unwrap());
    (a, b)
}

fn istek(kapi: u16, metin: &str) -> Yanit {
    let (mut a, mut b) = baglan(kapi);
    a.write_all(metin.as_bytes()).unwrap();
    yanit_oku(&mut b)
}

fn al(kapi: u16, yol: &str, cerez: Option<&str>) -> Yanit {
    let c = cerez
        .map(|c| format!("Cookie: {c}\r\n"))
        .unwrap_or_default();
    istek(
        kapi,
        &format!("GET {yol} HTTP/1.1\r\nHost: localhost\r\n{c}Connection: close\r\n\r\n"),
    )
}

#[test]
fn baglantilar_acik_kalir_ve_birbirini_bekletmez() {
    let s = baslat("baglanti", &[], |_| {});
    // Aynı bağlantıda ardışık iki istek ve tek seferde gönderilen iki istek
    let (mut a, mut b) = baglan(s.kapi);
    for _ in 0..2 {
        a.write_all(b"GET /\xc3\xa7erez HTTP/1.1\r\nHost: x\r\n\r\n")
            .unwrap();
        let y = yanit_oku(&mut b);
        assert_eq!((y.durum, y.metin().as_str()), (200, "çerez yok"));
        assert!(y.basliklar.contains("Connection: keep-alive"));
    }
    a.write_all(
        b"GET /yok HTTP/1.1\r\nHost: x\r\n\r\nGET /\xc3\xa7erez HTTP/1.1\r\nHost: x\r\n\r\n",
    )
    .unwrap();
    assert_eq!(yanit_oku(&mut b).durum, 404);
    assert_eq!(yanit_oku(&mut b).metin(), "çerez yok");

    // Yarım isteğiyle bekleyen yavaş bir istemci ötekileri bekletmez.
    let (mut yavas, mut yavas_okuyucu) = baglan(s.kapi);
    yavas
        .write_all(b"GET /\xc3\xa7erez HTTP/1.1\r\nHo")
        .unwrap();
    let bas = Instant::now();
    for _ in 0..5 {
        assert_eq!(al(s.kapi, "/%C3%A7erez", None).metin(), "çerez yok");
    }
    assert!(
        bas.elapsed() < Duration::from_secs(3),
        "{:?}",
        bas.elapsed()
    );
    yavas.write_all(b"st: x\r\n\r\n").unwrap();
    assert_eq!(yanit_oku(&mut yavas_okuyucu).metin(), "çerez yok");

    // Parçalı (chunked) gövde
    let y = istek(
        s.kapi,
        "POST /yank%C4%B1 HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n\
         5\r\nMerha\r\n4\r\nba d\r\n5\r\nünya\r\n0\r\n\r\n",
    );
    assert_eq!(y.metin(), "Merhaba dünya");
}

#[test]
fn oturum_ve_cerezler() {
    let s = baslat("oturum", &[], |_| {});
    let y = al(s.kapi, "/saya%C3%A7", None);
    assert_eq!(y.metin(), "ziyaret 1");
    let cerezler = y.cerezler();
    let oturum = cerezler
        .iter()
        .find(|c| c.starts_with("orhunca_oturum="))
        .unwrap_or_else(|| panic!("{cerezler:?}"));
    assert!(oturum.contains("HttpOnly") && oturum.contains("SameSite=Lax"));
    let kimlik = oturum
        .split(';')
        .next()
        .unwrap()
        .trim_start_matches("orhunca_oturum=")
        .to_string();
    assert_eq!(kimlik.len(), 32);
    assert!(cerezler
        .iter()
        .any(|c| c.starts_with("tema=koyu%20mod; Path=/")));

    let cerez = format!("orhunca_oturum={kimlik}");
    assert_eq!(al(s.kapi, "/saya%C3%A7", Some(&cerez)).metin(), "ziyaret 2");
    assert_eq!(al(s.kapi, "/saya%C3%A7", Some(&cerez)).metin(), "ziyaret 3");
    // Başka bir tarayıcı: ayrı oturum
    assert_eq!(al(s.kapi, "/saya%C3%A7", None).metin(), "ziyaret 1");
    // Bilinmeyen oturum kimliği kabul edilmez.
    assert_eq!(
        al(
            s.kapi,
            "/saya%C3%A7",
            Some("orhunca_oturum=0123456789abcdef0123456789abcdef")
        )
        .metin(),
        "ziyaret 1"
    );
    // Gelen çerezler
    assert_eq!(
        al(s.kapi, "/%C3%A7erez", Some("a=1; tema=koyu%20mod")).metin(),
        "tema=koyu mod"
    );
    // Oturum boşaltılınca silinir ve çerezi temizlenir.
    let y = al(s.kapi, "/%C3%A7%C4%B1k%C4%B1%C5%9F", Some(&cerez));
    assert!(
        y.cerezler()
            .iter()
            .any(|c| c.starts_with("orhunca_oturum=;") && c.contains("Max-Age=0")),
        "{:?}",
        y.cerezler()
    );
    assert_eq!(al(s.kapi, "/saya%C3%A7", Some(&cerez)).metin(), "ziyaret 1");
}

#[test]
fn dosya_yukleme() {
    let s = baslat("yukleme", &[("ORHUNCA_VERI", "veri")], |k| {
        std::fs::create_dir_all(k.join("statik")).unwrap()
    });
    let mut icerik: Vec<u8> = (0..=255u8).collect();
    icerik.extend_from_slice(b"\r\n--sinir-gibi ama degil\r\n\0son");
    let mut govde = Vec::new();
    govde.extend_from_slice(
        b"--XyZ\r\nContent-Disposition: form-data; name=\"a\xc3\xa7\xc4\xb1klama\"\r\n\r\nYaz\xc4\xb1 notu\r\n\
          --XyZ\r\nContent-Disposition: form-data; name=\"belge\"; filename=\"../../r\xc3\xa2por.bin\"\r\n\
          Content-Type: application/octet-stream\r\n\r\n",
    );
    govde.extend_from_slice(&icerik);
    govde.extend_from_slice(b"\r\n--XyZ--\r\n");
    let mut metin = format!(
        "POST /y%C3%BCkle HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\
         Content-Type: multipart/form-data; boundary=XyZ\r\nContent-Length: {}\r\n\r\n",
        govde.len()
    )
    .into_bytes();
    metin.extend_from_slice(&govde);
    let (mut a, mut b) = baglan(s.kapi);
    a.write_all(&metin).unwrap();
    let y = yanit_oku(&mut b);
    assert_eq!(y.durum, 200, "{}", y.metin());
    let parcalar: Vec<String> = y.metin().split('|').map(str::to_string).collect();
    assert_eq!(parcalar[0], "../../râpor.bin");
    assert_eq!(parcalar[1], "application/octet-stream");
    assert_eq!(parcalar[2], icerik.len().to_string());
    assert_eq!(parcalar[3], "Yazı notu");
    // Dosya, adı güvenli hâle getirilerek yükleme klasörüne kaydedilir.
    let yol = &parcalar[4];
    assert!(
        yol.starts_with("veri/yüklemeler/") && yol.ends_with("-râpor.bin"),
        "{yol}"
    );
    // dosya_taşı ile statik/ klasörüne taşındı; sunucu aynı baytları verir.
    assert_eq!(parcalar[5], "doğru");
    assert!(!s.klasor.join(yol).exists());
    let y = al(s.kapi, "/son.bin", None);
    assert_eq!(y.govde, icerik);
}

#[test]
fn https() {
    let var = |p: &str| {
        Command::new(p)
            .arg("--version")
            .output()
            .is_ok_and(|c| c.status.success())
    };
    if !var("openssl") || !var("curl") {
        eprintln!("openssl ya da curl yok: test atlandı");
        return;
    }
    let s = baslat(
        "https",
        &[
            ("ORHUNCA_SERTIFIKA", "sertifika.pem"),
            ("ORHUNCA_ANAHTAR", "anahtar.pem"),
        ],
        |k| {
            let c = Command::new("openssl")
                .args([
                    "req",
                    "-x509",
                    "-newkey",
                    "rsa:2048",
                    "-nodes",
                    "-days",
                    "1",
                    "-subj",
                    "/CN=localhost",
                    "-keyout",
                    "anahtar.pem",
                    "-out",
                    "sertifika.pem",
                ])
                .current_dir(k)
                .output()
                .unwrap();
            assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
        },
    );
    let adres = format!("https://localhost:{}/saya%C3%A7", s.kapi);
    let c = Command::new("curl")
        .args(["-sk", "-i", &adres, &adres])
        .output()
        .unwrap();
    let cikti = String::from_utf8_lossy(&c.stdout);
    assert!(c.status.success(), "{cikti}");
    // İki istek aynı TLS bağlantısında; oturum çerezi Secure
    assert_eq!(cikti.matches("ziyaret 1").count(), 2, "{cikti}");
    assert!(cikti.contains("HttpOnly; Secure"), "{cikti}");
}

/// Saldırı girdileri: statik klasörün dışına çıkma, gizli dosyalar, Windows aygıt
/// adları, aşırı büyük parçalı gövde uzunluğu, bozuk istek satırı.
#[test]
fn kotu_niyetli_istekler() {
    let s = baslat("saldiri", &[], |k| {
        std::fs::create_dir_all(k.join("statik/alt")).unwrap();
        std::fs::write(k.join("statik/alt/a.txt"), "açık").unwrap();
        std::fs::write(k.join("statik/.env"), "GİZLİ").unwrap();
        std::fs::write(k.join("gizli.txt"), "GİZLİ").unwrap();
    });
    assert_eq!(al(s.kapi, "/alt/a.txt", None).metin(), "açık");
    for yol in [
        "/../gizli.txt",
        "/alt/../../gizli.txt",
        "/%2e%2e/gizli.txt",
        "/alt/%2E%2E/%2e%2e/gizli.txt",
        "/..%2fgizli.txt",
        "/alt%2f..%2f..%2fgizli.txt",
        "/.env",
        "/%2eenv",
        "/alt/..%5c..%5cgizli.txt",
        "/con",
        "/NUL.txt",
        "/alt/com1",
        "/C:/Windows/win.ini",
    ] {
        let y = al(s.kapi, yol, None);
        assert!(
            y.durum == 404 && !y.metin().contains("GİZLİ"),
            "{yol}: {}",
            y.durum
        );
    }

    // Parçalı gövdede devasa uzunluk: taşma yok, 413
    let y = istek(
        s.kapi,
        "POST /yankı HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n7fffffffffffffff\r\nabc\r\n",
    );
    assert_eq!(y.durum, 413);
    let y = istek(
        s.kapi,
        "POST /yankı HTTP/1.1\r\nHost: x\r\nContent-Length: -5\r\n\r\n",
    );
    assert_eq!(y.durum, 413);
    // Bozuk istek satırı
    let y = istek(s.kapi, "BOZUK\r\n\r\n");
    assert_eq!(y.durum, 400);
    // Sunucu hâlâ çalışıyor
    assert_eq!(al(s.kapi, "/alt/a.txt", None).metin(), "açık");
}
