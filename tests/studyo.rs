//! Orhunca Stüdyo sunucusunun uçtan uca testleri: güvenlik, şablonlar, çalıştırma.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Sunucu {
    cocuk: Child,
    kapi: u16,
    anahtar: String,
    ev: std::path::PathBuf,
}

impl Drop for Sunucu {
    fn drop(&mut self) {
        let _ = self.cocuk.kill();
        let _ = std::fs::remove_dir_all(&self.ev);
    }
}

fn baslat(ad: &str) -> Sunucu {
    let ev = std::env::temp_dir().join(format!("orhunca-studyo-test-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ev);
    std::fs::create_dir_all(&ev).unwrap();
    let mut cocuk = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["stüdyo", "--tarayıcı-açma", "--kapı", "0"])
        .env("HOME", &ev)
        .env("USERPROFILE", &ev)
        .env("APPDATA", &ev)
        .env("XDG_CONFIG_HOME", ev.join(".config"))
        .env("USER", "deneme")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut satir = String::new();
    BufReader::new(cocuk.stdout.take().unwrap())
        .read_line(&mut satir)
        .unwrap();
    let adres = satir
        .split_whitespace()
        .find(|p| p.starts_with("http://"))
        .unwrap();
    let kapi = adres
        .split(':')
        .nth(2)
        .unwrap()
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let anahtar = adres.split("anahtar=").nth(1).unwrap().to_string();
    Sunucu {
        cocuk,
        kapi,
        anahtar,
        ev,
    }
}

impl Sunucu {
    fn istek(&self, yontem: &str, yol: &str, govde: Option<&str>, anahtar: bool) -> (u16, String) {
        let mut a = TcpStream::connect(("127.0.0.1", self.kapi)).unwrap();
        let govde = govde.unwrap_or("");
        let mut istek = format!(
            "{yontem} {yol} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Length: {}\r\n",
            self.kapi,
            govde.len()
        );
        if anahtar {
            istek.push_str(&format!("X-Orhunca-Anahtar: {}\r\n", self.anahtar));
        }
        istek.push_str("\r\n");
        istek.push_str(govde);
        a.write_all(istek.as_bytes()).unwrap();
        let mut yanit = String::new();
        a.read_to_string(&mut yanit).unwrap();
        let durum = yanit.split_whitespace().nth(1).unwrap().parse().unwrap();
        let govde = yanit
            .split_once("\r\n\r\n")
            .map(|x| x.1)
            .unwrap_or("")
            .to_string();
        (durum, govde)
    }

    fn api(&self, yol: &str, govde: serde_json::Value) -> serde_json::Value {
        let (_, g) = self.istek("POST", yol, Some(&govde.to_string()), true);
        serde_json::from_str(&g).unwrap()
    }
}

#[test]
fn guvenlik() {
    let s = baslat("guvenlik");
    let (durum, govde) = s.istek("GET", "/", None, false);
    assert_eq!(durum, 200);
    assert!(govde.contains("uygulama.js"));
    assert_eq!(s.istek("GET", "/api/durum", None, false).0, 403);
    assert_eq!(s.istek("GET", "/api/durum", None, true).0, 200);
    // Açılmamış bir yerdeki dosya okunamaz.
    let (durum, _) = s.istek("GET", "/api/dosya?yol=%2Fetc%2Fpasswd", None, true);
    assert_eq!(durum, 403);
}

#[test]
fn sablonlar_derlenir_ve_calisir() {
    let s = baslat("sablon");
    let konum = s.ev.join("Projeler");
    for (sablon, giris) in [
        ("konsol", "ana.ohc"),
        ("sayi_tahmin", "oyun.ohc"),
        ("kutuphane", "testler/kutuphane_testi.ohc"),
    ] {
        for ornek in [true, false] {
            let ad = format!("{sablon}_{ornek}");
            let r = s.api(
                "/api/proje/olustur",
                serde_json::json!({ "sablon": sablon, "ad": ad, "konum": konum, "git": false, "ornek": ornek }),
            );
            assert!(r["hata"].is_null(), "{sablon}: {r}");
            let dosya = konum.join(&ad).join(giris);
            let d = s.api("/api/denetle", serde_json::json!({ "dosya": dosya }));
            assert_eq!(
                d["hatalar"],
                serde_json::json!([]),
                "{sablon} (örnek: {ornek}) derlenmiyor"
            );
        }
    }
    // Henüz desteklenmeyen şablon oluşturulamaz.
    let r = s.api(
        "/api/proje/olustur",
        serde_json::json!({ "sablon": "web_sitesi", "ad": "web", "konum": konum }),
    );
    assert!(r["hata"].as_str().unwrap().contains("Aşama 6"));

    // Konsol uygulamasını çalıştır, girdi gönder, çıktıyı oku.
    let proje = konum.join("konsol_true");
    let r = s.api(
        "/api/calistir",
        serde_json::json!({ "dosya": proje.join("ana.ohc"), "klasor": proje }),
    );
    let kimlik = r["kimlik"].as_u64().unwrap_or_else(|| panic!("{r}"));
    s.api(
        "/api/girdi",
        serde_json::json!({ "kimlik": kimlik, "metin": "Test\n" }),
    );
    let mut cikti = String::new();
    let mut konum_ = 0;
    let bas = Instant::now();
    loop {
        let (_, g) = s.istek(
            "GET",
            &format!("/api/cikti?kimlik={kimlik}&konum={konum_}"),
            None,
            true,
        );
        let d: serde_json::Value = serde_json::from_str(&g).unwrap();
        for p in d["parcalar"].as_array().unwrap() {
            cikti.push_str(p["t"].as_str().unwrap());
        }
        konum_ = d["konum"].as_u64().unwrap();
        if d["bitti"] == true {
            assert_eq!(d["kod"], 0);
            break;
        }
        assert!(
            bas.elapsed() < Duration::from_secs(20),
            "program bitmedi: {cikti}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(cikti.contains("Merhaba Test! 3"), "{cikti}");
}
