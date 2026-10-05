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
    // Başka bir sitenin adıyla (DNS yeniden bağlama) gelen istek reddedilir.
    let mut a = TcpStream::connect(("127.0.0.1", s.kapi)).unwrap();
    a.write_all(b"GET / HTTP/1.1\r\nHost: kotu.example:80\r\n\r\n")
        .unwrap();
    let mut y = String::new();
    a.read_to_string(&mut y).unwrap();
    assert!(y.starts_with("HTTP/1.1 403"), "{y}");

    // Açık projeden `..` ile dışarı çıkılamaz (var olmayan ara klasörle de).
    let konum = s.ev.join("Projeler");
    let r = s.api(
        "/api/proje/olustur",
        serde_json::json!({ "sablon": "konsol", "ad": "p", "konum": konum, "git": false, "ornek": false }),
    );
    assert!(r["hata"].is_null(), "{r}");
    for yol in [
        konum
            .join("p")
            .join("yok")
            .join("..")
            .join("..")
            .join("kacis.txt"),
        konum.join("p").join("..").join("kacis.txt"),
    ] {
        let (durum, _) = s.istek(
            "POST",
            "/api/dosya",
            Some(&serde_json::json!({ "yol": yol, "icerik": "x" }).to_string()),
            true,
        );
        assert_eq!(durum, 403, "{}", yol.display());
    }
    assert!(!konum.join("kacis.txt").exists());
    let (durum, _) = s.istek(
        "POST",
        "/api/dosya",
        Some(
            &serde_json::json!({ "yol": konum.join("p").join("yeni.ohc"), "icerik": "x" })
                .to_string(),
        ),
        true,
    );
    assert_eq!(durum, 200);

    // Bitmeyen başlık satırı bellek tüketmez: 64 KB'tan sonra bağlantı kapanır.
    let mut a = TcpStream::connect(("127.0.0.1", s.kapi)).unwrap();
    let _ = a.write_all(&vec![b'a'; 200 * 1024]);
    let mut y = Vec::new();
    let _ = a.read_to_end(&mut y);
    assert!(y.is_empty());
    assert_eq!(s.istek("GET", "/api/durum", None, true).0, 200);
}

#[test]
fn sablonlar_derlenir_ve_calisir() {
    let s = baslat("sablon");
    let konum = s.ev.join("Projeler");
    for (sablon, giris) in [
        ("konsol", "ana.ohc"),
        ("sayi_tahmin", "oyun.ohc"),
        ("kutuphane", "testler/kutuphane_testi.ohc"),
        ("bos_web", "sunucu.ohc"),
        ("web_sitesi", "sunucu.ohc"),
        ("web_uyg", "sunucu.ohc"),
        ("acilis", "sunucu.ohc"),
        ("web_api", "sunucu.ohc"),
        ("tam_yigin", "sunucu.ohc"),
        ("arayuz", "uygulama.ohc"),
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
            assert_eq!(
                d["uyarilar"],
                serde_json::json!([]),
                "{sablon} (örnek: {ornek}) uyarı veriyor"
            );
            // Şablonlar biçimlendirici kurallarına uygun olmalı (uyarı yok).
            let b = Command::new(env!("CARGO_BIN_EXE_orhunca"))
                .args(["biçimlendir", "--denetle"])
                .arg(konum.join(&ad))
                .output()
                .unwrap();
            assert!(
                b.status.success(),
                "{sablon}: {}",
                String::from_utf8_lossy(&b.stdout)
            );
        }
    }
    // Arayüz uygulaması WebAssembly'ye derlenir; sayfası Stüdyo'dan (anahtarsız) sunulur.
    let proje = konum.join("arayuz_true");
    let r = s.api(
        "/api/calistir",
        serde_json::json!({ "dosya": proje.join("uygulama.ohc"), "klasor": proje }),
    );
    let adres = r["arayuz"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_string();
    let (durum, sayfa) = s.istek("GET", &adres, None, false);
    assert_eq!(durum, 200);
    assert!(sayfa.contains("<div id=\"uygulama\">"));
    assert!(sayfa.contains("<title>uygulama</title>"));
    assert_eq!(s.istek("GET", "/onizleme/999999", None, false).0, 404);

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

#[test]
fn web_projesi_onizlemeyle_calisir() {
    let s = baslat("web");
    let konum = s.ev.join("Projeler");
    let r = s.api(
        "/api/proje/olustur",
        serde_json::json!({ "sablon": "bos_web", "ad": "sayfa", "konum": konum, "git": false }),
    );
    assert_eq!(r["web"], true, "{r}");
    let proje = konum.join("sayfa");
    let r = s.api(
        "/api/calistir",
        serde_json::json!({ "dosya": proje.join("sunucu.ohc"), "klasor": proje }),
    );
    let kimlik = r["kimlik"].as_u64().unwrap_or_else(|| panic!("{r}"));
    let kapi = r["kapi"].as_u64().unwrap();
    // Sunucu "dinleniyor" satırını yazana kadar çıktı izlenir.
    let mut cikti = String::new();
    let mut konum_ = 0;
    let bas = Instant::now();
    while !cikti.contains("Sunucu dinleniyor") {
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
        assert!(d["bitti"] != true, "sunucu kapandı: {cikti}");
        assert!(bas.elapsed() < Duration::from_secs(20), "{cikti}");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        cikti.contains(&format!("http://localhost:{kapi}")),
        "{cikti}"
    );
    // Sayfa, Stüdyo'nun yenileme betiğiyle gelir.
    let mut a = TcpStream::connect(("127.0.0.1", kapi as u16)).unwrap();
    a.write_all(b"GET /selam/2 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut yanit = String::new();
    a.read_to_string(&mut yanit).unwrap();
    assert!(yanit.starts_with("HTTP/1.1 200"), "{yanit}");
    assert!(yanit.contains("Tekrar hoş geldin! (3. ziyaret)"), "{yanit}");
    assert!(yanit.contains("orhunca:yenile"), "{yanit}");
    s.api("/api/durdur", serde_json::json!({ "kimlik": kimlik }));
}

/// Hata ayıklayıcı: kesme noktası, değişkenler, çağrı yığını, adım adım
/// yürütme ve yakalanmamış hatada durma.
#[test]
fn hata_ayiklayici() {
    let s = baslat("ayikla");
    let konum = s.ev.join("Projeler");
    let r = s.api(
        "/api/proje/olustur",
        serde_json::json!({ "sablon": "konsol", "ad": "ayikla", "konum": konum, "git": false, "ornek": false }),
    );
    assert!(r["hata"].is_null(), "{r}");
    let proje = konum.join("ayikla");
    let dosya = proje.join("ana.ohc");
    let kaynak = "işlev kare(n: sayı) -> sayı:\n    sonuç = n * n\n    döndür sonuç\n\ntoplam = 0\nadlar = [\"a\", \"b\"]\nher i için 1'den 3'e kadar:\n    toplam += kare(i)\ntoplam'ı yaz.\nadlar[5]'i yaz.\n";
    let r = s.api(
        "/api/dosya",
        serde_json::json!({ "yol": dosya, "icerik": kaynak }),
    );
    assert!(r["hata"].is_null(), "{r}");

    let r = s.api(
        "/api/calistir",
        serde_json::json!({ "dosya": dosya, "klasor": proje, "ayikla": true,
            "kesmeler": [{ "dosya": dosya, "satir": 2 }] }),
    );
    let kimlik = r["kimlik"].as_u64().unwrap_or_else(|| panic!("{r}"));
    let mut cikti = String::new();
    let mut konum_ = 0;
    let mut surum = 0;
    // Bir sonraki durağı (ya da bitişi) bekler.
    let mut bekle = |s: &Sunucu| -> serde_json::Value {
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
            let a = &d["ayiklama"];
            if a["durdu"] == true && a["surum"].as_u64().unwrap() != surum {
                surum = a["surum"].as_u64().unwrap();
                return d;
            }
            if d["bitti"] == true {
                return d;
            }
            assert!(bas.elapsed() < Duration::from_secs(20), "durmadı: {d}");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let komut = |k: &str| {
        let r = s.api(
            "/api/ayikla",
            serde_json::json!({ "kimlik": kimlik, "komut": k }),
        );
        assert_eq!(r["tamam"], true, "{k}: {r}");
    };

    // Kesme noktası: kare'nin içinde, ilk çağrıda
    let d = bekle(&s);
    let a = &d["ayiklama"];
    assert_eq!(a["neden"], "kesme", "{d}");
    assert_eq!(a["yigin"][0]["islev"], "kare");
    assert_eq!(a["yigin"][0]["satir"], 2);
    assert_eq!(a["yigin"][1]["islev"], "ana");
    assert_eq!(a["yigin"][1]["satir"], 8);
    let deg = |a: &serde_json::Value, ad: &str| {
        a["degiskenler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["ad"] == ad)
            .map(|d| d["deger"].as_str().unwrap().to_string())
    };
    assert_eq!(deg(a, "n").as_deref(), Some("1"), "{a}");

    // Çağıranın çerçevesi
    let r = s.api(
        "/api/ayikla",
        serde_json::json!({ "kimlik": kimlik, "komut": "cerceve", "cerceve": 1 }),
    );
    assert_eq!(r["tamam"], true);
    let bas = Instant::now();
    loop {
        let (_, g) = s.istek(
            "GET",
            &format!("/api/cikti?kimlik={kimlik}&konum=0"),
            None,
            true,
        );
        let d: serde_json::Value = serde_json::from_str(&g).unwrap();
        let a = &d["ayiklama"];
        if a["cerceve"] == 1 && !a["degiskenler"].as_array().unwrap().is_empty() {
            assert_eq!(deg(a, "adlar").as_deref(), Some("[\"a\", \"b\"]"), "{a}");
            assert_eq!(deg(a, "i").as_deref(), Some("1"));
            break;
        }
        assert!(bas.elapsed() < Duration::from_secs(10), "{d}");
        std::thread::sleep(Duration::from_millis(20));
    }

    // Kesme noktası kaldırılır; dışına adım → ana programda sonraki deyim
    let r = s.api(
        "/api/ayikla",
        serde_json::json!({ "kimlik": kimlik, "komut": "kesmeler", "kesmeler": [] }),
    );
    assert_eq!(r["tamam"], true);
    komut("cik");
    let d = bekle(&s);
    let a = &d["ayiklama"];
    assert_eq!(a["yigin"][0]["islev"], "ana", "{d}");
    assert_eq!(deg(a, "toplam").as_deref(), Some("1"), "{a}");

    // Devam: yakalanmamış hatada durur, mesajı gösterir
    komut("devam");
    let d = bekle(&s);
    let a = &d["ayiklama"];
    assert_eq!(a["neden"], "hata", "{d}");
    assert_eq!(a["yigin"][0]["satir"], 10);
    assert!(a["ileti"].as_str().unwrap().contains("satır 10"), "{a}");

    komut("devam");
    let d = bekle(&s);
    assert_eq!(d["bitti"], true, "{d}");
    assert_eq!(d["kod"], 1);
    assert!(cikti.contains("14"), "{cikti}");
}

#[test]
fn temalar() {
    let s = baslat("temalar");
    let tema = serde_json::json!({
        "orhunca_tema": 1, "ad": "Gök Mavi", "taban": "koyu",
        "renkler": { "vurgu": "#3399ff" }, "yazi": {}
    });
    let r = s.api("/api/tema/kaydet", serde_json::json!({ "tema": tema }));
    let kimlik = r["kimlik"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_string();
    assert!(kimlik.starts_with("gok-mavi-"), "{kimlik}");
    let (_, g) = s.istek("GET", "/api/temalar", None, true);
    assert!(g.contains("Gök Mavi"), "{g}");
    let (_, g) = s.istek("GET", &format!("/api/tema?kimlik={kimlik}"), None, true);
    assert!(g.contains("#3399ff"), "{g}");
    // Tema olmayan dosya ve galeride klasör dışına çıkan ad reddedilir
    let r = s.api(
        "/api/tema/kaydet",
        serde_json::json!({ "tema": { "ad": "x" } }),
    );
    assert!(r["hata"].is_string(), "{r}");
    for dosya in ["../../etc/passwd", "../x.ohctema", "a/b.ohctema"] {
        let r = s.api("/api/tema/galeriden", serde_json::json!({ "dosya": dosya }));
        assert!(r["hata"].is_string(), "{dosya}: {r}");
    }
    let (_, g) = s.istek("GET", "/api/tema?kimlik=..%2F..%2Fstudyo", None, true);
    assert!(g.contains("geçersiz"), "{g}");
    let r = s.api("/api/tema/sil", serde_json::json!({ "kimlik": kimlik }));
    assert_eq!(r["tamam"], true, "{r}");
    let (_, g) = s.istek("GET", "/api/temalar", None, true);
    assert!(!g.contains("Gök Mavi"), "{g}");
}
