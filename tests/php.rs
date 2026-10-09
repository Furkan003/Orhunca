//! `orhunca yayınla --php`: programlar PHP'ye çevrilir ve PHP ile çalıştırılır.
//! Konsol örneklerinin çıktısı `.beklenen` dosyalarıyla, web programlarının yanıtları
//! `php -S` sunucusu üzerinden denetlenir. Sistemde PHP 8.1+ yoksa testler atlanır.
#![cfg(unix)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Xdebug (CI makinelerinde kurulu olabilir) çağrı derinliğini sınırlar ve yavaşlatır.
fn php() -> Command {
    let mut c = Command::new("php");
    c.args(["-d", "xdebug.mode=off"]);
    c
}

fn php_var() -> bool {
    let tamam = php()
        .args(["-r", "exit(PHP_VERSION_ID >= 80100 && extension_loaded('pdo_sqlite') && extension_loaded('mbstring') ? 0 : 1);"])
        .output()
        .is_ok_and(|c| c.status.success());
    if !tamam {
        eprintln!("PHP 8.1+ (pdo_sqlite, mbstring) bulunamadı; test atlandı");
    }
    tamam
}

fn klasor(ad: &str) -> PathBuf {
    let k = std::env::temp_dir().join(format!("orhunca-php-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    k
}

fn orhunca(k: &Path, args: &[&str]) {
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(args)
        .current_dir(k)
        .output()
        .unwrap();
    assert!(
        c.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&c.stderr)
    );
}

#[test]
fn ornekler_php_ile_ayni_ciktiyi_verir() {
    if !php_var() {
        return;
    }
    let kok = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sayi = 0;
    for ad in ["örnekler", "tests"] {
        for giris in std::fs::read_dir(kok.join(ad)).unwrap() {
            let yol = giris.unwrap().path();
            let beklenen = yol.with_extension("beklenen");
            // C kütüphanesi çağrıları PHP'de yoktur.
            if yol.extension().is_none_or(|u| u != "ohc")
                || !beklenen.is_file()
                || yol.file_stem().unwrap() == "c_kütüphanesi"
            {
                continue;
            }
            let k = klasor("ornek");
            let dosya = k.join(yol.file_name().unwrap());
            std::fs::copy(&yol, &dosya).unwrap();
            orhunca(&k, &["yayınla", "--php", dosya.to_str().unwrap()]);
            let c = php()
                .arg("index.php")
                .current_dir(k.join("cikti/php"))
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(
                c.status.success(),
                "{}: {}",
                yol.display(),
                String::from_utf8_lossy(&c.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&c.stdout),
                std::fs::read_to_string(&beklenen).unwrap(),
                "{}",
                yol.display()
            );
            sayi += 1;
        }
    }
    assert!(sayi > 10);
}

#[test]
fn calisma_hatasi_orhunca_satirini_gosterir() {
    if !php_var() {
        return;
    }
    let k = klasor("hata");
    std::fs::write(k.join("a.ohc"), "x = 1\ny = 0\n(x / y)'yi yaz.\n").unwrap();
    orhunca(&k, &["yayınla", "--php", "a.ohc"]);
    let c = php()
        .arg("index.php")
        .current_dir(k.join("cikti/php"))
        .output()
        .unwrap();
    assert!(!c.status.success());
    let hata = String::from_utf8_lossy(&c.stderr);
    assert!(
        hata.contains("Çalışma hatası (satır 3): sıfıra bölme"),
        "{hata}"
    );
    std::fs::remove_dir_all(&k).unwrap();
}

/// `php -S` ile açılan sunucu; düşürülünce kapanır. Sunucu testleri sırayla çalışır:
/// aynı anda boş kapı arayan iki test aynı kapıyı alıp birbirinin sunucusuna bağlanmasın.
struct Sunucu(
    Child,
    u16,
    #[allow(dead_code)] std::sync::MutexGuard<'static, ()>,
);

static SUNUCU_SIRASI: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl Drop for Sunucu {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn sunucu_ac(klasor: &Path) -> Sunucu {
    let sira = SUNUCU_SIRASI.lock().unwrap_or_else(|e| e.into_inner());
    let kapi = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut s = Sunucu(
        php()
            .args(["-S", &format!("127.0.0.1:{kapi}"), "index.php"])
            .current_dir(klasor)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
        kapi,
        sira,
    );
    for _ in 0..200 {
        if TcpStream::connect(("127.0.0.1", kapi)).is_ok() {
            return s;
        }
        if let Ok(Some(d)) = s.0.try_wait() {
            panic!("php -S kapandı: {d}");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("php -S açılmadı");
}

struct Yanit {
    durum: u16,
    basliklar: String,
    govde: String,
}

fn istek(s: &Sunucu, yontem: &str, yol: &str, cerez: &str, govde: &str) -> Yanit {
    let mut t = TcpStream::connect(("127.0.0.1", s.1)).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    // php -S parçalı gelen istek satırını bozuk sayabilir; istek tek seferde yazılır.
    let cerez = if cerez.is_empty() {
        String::new()
    } else {
        format!("Cookie: {cerez}\r\n")
    };
    let ham = format!(
        "{yontem} {yol} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{cerez}\
         Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{govde}",
        govde.len()
    );
    t.write_all(ham.as_bytes()).unwrap();
    let mut m = Vec::new();
    t.read_to_end(&mut m).unwrap();
    let m = String::from_utf8_lossy(&m).into_owned();
    let (b, g) = m
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("{yontem} {yol}: eksik yanıt {m:?}"));
    Yanit {
        durum: b[9..12].parse().unwrap(),
        basliklar: b.to_string(),
        govde: g.to_string(),
    }
}

fn baslik<'a>(y: &'a Yanit, ad: &str) -> Vec<&'a str> {
    y.basliklar
        .lines()
        .filter_map(|s| s.split_once(": "))
        .filter(|(a, _)| a.eq_ignore_ascii_case(ad))
        .map(|(_, d)| d)
        .collect()
}

#[test]
fn tam_yigin_sablonu_php_ile_calisir() {
    if !php_var() {
        return;
    }
    let k = klasor("dukkan");
    orhunca(&k, &["yeni", "dukkan", "--şablon", "tam_yigin"]);
    let p = k.join("dukkan");
    orhunca(&p, &["yayınla", "--php"]);
    let c = p.join("cikti/php");
    for dosya in [
        "index.php",
        ".htaccess",
        "orhunca/calisma.php",
        "orhunca/ayarlar.php",
        "stil.css",
    ] {
        assert!(c.join(dosya).is_file(), "{dosya}");
    }
    let s = sunucu_ac(&c);
    let urunler = "/%C3%BCr%C3%BCnler";
    let y = istek(&s, "GET", urunler, "", "");
    assert_eq!(y.durum, 200, "{}", y.govde);
    // Geçersiz form aynı sayfada hatalarla döner.
    let y = istek(
        &s,
        "POST",
        &format!("{urunler}/yeni"),
        "",
        "ad=&fiyat=abc&stok=2",
    );
    assert_eq!(y.durum, 200);
    assert!(y.govde.contains("Ad boş bırakılamaz"), "{}", y.govde);
    let y = istek(
        &s,
        "POST",
        &format!("{urunler}/yeni"),
        "",
        "ad=Kalem&fiyat=12%2C5&stok=3&a%C3%A7%C4%B1klama=%3Cb%3Emavi%3C%2Fb%3E",
    );
    assert_eq!(y.durum, 303, "{}", y.govde);
    assert_eq!(baslik(&y, "Location"), [urunler]);
    let y = istek(&s, "GET", &format!("{urunler}/1"), "", "");
    assert!(y.govde.contains("value=\"Kalem\""), "{}", y.govde);
    assert!(y.govde.contains("&lt;b&gt;mavi"), "{}", y.govde);
    let y = istek(&s, "GET", urunler, "", "");
    assert!(y.govde.contains("12,50"), "{}", y.govde);
    let y = istek(&s, "GET", "/api/%C3%BCr%C3%BCnler", "", "");
    assert!(baslik(&y, "Content-Type")[0].starts_with("application/json"));
    assert!(y.govde.contains("\"ad\":\"Kalem\""), "{}", y.govde);
    assert!(c.join("veri/orhunca.sqlite").is_file());
    assert_eq!(istek(&s, "GET", "/stil.css", "", "").durum, 200);
    let y = istek(&s, "GET", "/yok", "", "");
    assert_eq!(y.durum, 404);
    assert!(
        y.govde.contains("<title>Sayfa bulunamadı</title>"),
        "{}",
        y.govde
    );
    let y = istek(&s, "DELETE", urunler, "", "");
    assert_eq!(y.durum, 405);
    // Veritabanı ve ayar dosyaları dışarıya kapalıdır.
    let ht = std::fs::read_to_string(c.join("veri/.htaccess")).unwrap();
    assert!(ht.contains("Require all denied"));
    drop(s);
    // Yeniden yayınlamak veriyi ve ayarları korur.
    std::fs::write(
        c.join("orhunca/ayarlar.php"),
        "<?php return ['deneme' => 1];\n",
    )
    .unwrap();
    orhunca(&p, &["yayınla", "--php"]);
    assert!(c.join("veri/orhunca.sqlite").is_file());
    assert!(std::fs::read_to_string(c.join("orhunca/ayarlar.php"))
        .unwrap()
        .contains("deneme"));
    std::fs::remove_dir_all(&k).unwrap();
}

#[test]
fn oturum_cerez_ve_hata_sayfasi() {
    if !php_var() {
        return;
    }
    let k = klasor("oturum");
    std::fs::write(
        k.join("a.ohc"),
        r#"al "/":
    ad = "misafir"
    eğer içerir(istek.oturum, "kullanıcı") ise:
        ad = istek.oturum["kullanıcı"]
    döndür "<p>Merhaba " + kaçır(ad) + "</p>"

gönder "/giriş":
    istek.oturum["kullanıcı"] = istek.form["ad"]
    döndür yönlendir("/")

al "/çerez":
    y = yanıt(200, metin(istek.çerezler), "text/plain")
    y.çerezler["tema"] = "koyu"
    döndür y

al "/hata":
    l = [1, 2, 3]
    döndür metin(l[5])

al "/boş":
    "x"'i yaz.
"#,
    )
    .unwrap();
    orhunca(&k, &["yayınla", "--php", "a.ohc"]);
    let s = sunucu_ac(&k.join("cikti/php"));
    let y = istek(&s, "POST", "/giri%C5%9F", "", "ad=Zeynep+%3C3");
    assert_eq!(y.durum, 303);
    let oturum = baslik(&y, "Set-Cookie")
        .into_iter()
        .find(|c| c.starts_with("orhunca_oturum="))
        .expect("oturum çerezi")
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert!(baslik(&y, "Set-Cookie")
        .iter()
        .any(|c| c.contains("HttpOnly")));
    let y = istek(&s, "GET", "/", &oturum, "");
    assert_eq!(y.govde, "<p>Merhaba Zeynep &lt;3</p>");
    assert_eq!(
        istek(&s, "GET", "/", "", "").govde,
        "<p>Merhaba misafir</p>"
    );
    let y = istek(&s, "GET", "/%C3%A7erez", "tema=a%C3%A7%C4%B1k", "");
    assert_eq!(y.govde, "{\"tema\": \"açık\"}");
    assert_eq!(
        baslik(&y, "Set-Cookie"),
        ["tema=koyu; Path=/; SameSite=Lax"]
    );
    let y = istek(&s, "GET", "/hata", "", "");
    assert_eq!(y.durum, 500);
    assert!(
        y.govde
            .contains("Çalışma hatası (satır 18): liste sınırı aşıldı"),
        "{}",
        y.govde
    );
    assert_eq!(istek(&s, "GET", "/bo%C5%9F", "", "").durum, 204);
    drop(s);
    std::fs::remove_dir_all(&k).unwrap();
}

#[test]
fn yerel_kayitlar_yalnizca_veriyle_kopyalanir() {
    // 1.0 test raporu, bulgu 8: bilgisayardaki deneme verisi farkında olmadan yayına gidiyordu.
    let k = klasor("veriyle");
    orhunca(&k, &["yeni", "dukkan", "--şablon", "tam_yigin"]);
    let p = k.join("dukkan");
    std::fs::create_dir_all(p.join("veri")).unwrap();
    std::fs::write(p.join("veri/Ürün.json"), "[]\n").unwrap();
    for kip in ["--php", "--cgi"] {
        let c = p.join("cikti").join(&kip[2..]);
        orhunca(&p, &["yayınla", kip]);
        assert!(!c.join("veri/Ürün.json").exists(), "{kip}");
        let _ = std::fs::remove_dir_all(&c);
        orhunca(&p, &["yayınla", kip, "--veriyle"]);
        assert!(c.join("veri/Ürün.json").is_file(), "{kip}");
    }
    std::fs::remove_dir_all(&k).unwrap();
}
