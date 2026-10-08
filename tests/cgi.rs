//! Paylaşımlı hosting (CGI): çalışma zamanının CGI kipi, `orhunca yayınla --cgi` ve
//! `.ohc` dosyalarını PHP gibi çalıştıran `orhunca.cgi` işleyicisi. Web sunucusu yerine
//! CGI ortam değişkenleriyle doğrudan çalıştırılır.
#![cfg(target_os = "linux")]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn klasor(ad: &str) -> PathBuf {
    let k = std::env::temp_dir().join(format!("orhunca-cgi-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    k
}

fn orhunca(k: &Path, args: &[&str]) -> std::process::Output {
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(args)
        .current_dir(k)
        .env("ORHUNCA_CGI_DERLEYICI", env!("CARGO_BIN_EXE_orhunca"))
        .output()
        .unwrap();
    assert!(
        c.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&c.stderr)
    );
    c
}

/// Programı bir CGI isteğiyle çalıştırır; (başlıklar, gövde).
fn istek(
    program: &Path,
    yontem: &str,
    uri: &str,
    ek: &[(&str, &str)],
    govde: &str,
) -> (String, String) {
    let mut c = Command::new(program)
        .current_dir(program.parent().unwrap())
        .env_clear()
        .env("GATEWAY_INTERFACE", "CGI/1.1")
        .env("REQUEST_METHOD", yontem)
        .env("REQUEST_URI", uri)
        .env(
            "SCRIPT_NAME",
            format!("/{}", program.file_name().unwrap().to_string_lossy()),
        )
        .env("CONTENT_LENGTH", govde.len().to_string())
        .envs(ek.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(govde.as_bytes()).unwrap();
    let c = c.wait_with_output().unwrap();
    let m = String::from_utf8_lossy(&c.stdout).into_owned();
    let (b, g) = m.split_once("\r\n\r\n").unwrap_or((&m, ""));
    (b.to_string(), g.to_string())
}

#[test]
fn derlenmis_web_programi() {
    let k = klasor("web");
    std::fs::write(
        k.join("sunucu.ohc"),
        "\"bu satır yanıtı bozmamalı\"'ı yaz.
al \"/\":
    ziyaret = 1
    eğer içerir(istek.oturum, \"sayı\") ise:
        ziyaret = sayı(istek.oturum[\"sayı\"]) + 1
    istek.oturum[\"sayı\"] = metin(ziyaret)
    döndür \"<p>Ziyaret: \" + metin(ziyaret) + \"</p>\"
gönder \"/kaydet\":
    döndür {\"ad\": istek.form[\"ad\"], \"tarayıcı\": istek.başlıklar[\"user-agent\"]}
al \"/hata\":
    l = [1]
    döndür metin(l[5])
",
    )
    .unwrap();
    std::fs::create_dir_all(k.join("statik")).unwrap();
    std::fs::write(k.join("statik/stil.css"), "body{}").unwrap();
    orhunca(&k, &["derle", "sunucu.ohc", "-o", "uygulama.cgi"]);
    let p = k.join("uygulama.cgi");

    let (b, g) = istek(&p, "GET", "/", &[], "");
    assert!(b.starts_with("Status: 200 OK\r\n"), "{b}");
    assert!(!b.contains("Connection"), "{b}");
    assert_eq!(g, "<p>Ziyaret: 1</p>");
    let cerez = b
        .lines()
        .find_map(|l| l.strip_prefix("Set-Cookie: "))
        .and_then(|c| c.split(';').next())
        .unwrap()
        .to_string();
    let kimlik = cerez.split('=').nth(1).unwrap();
    assert!(k.join("veri/.oturumlar").join(kimlik).is_file());
    // Oturum bir sonraki süreçte (istekte) sürer.
    let (_, g) = istek(&p, "GET", "/", &[("HTTP_COOKIE", &cerez)], "");
    assert_eq!(g, "<p>Ziyaret: 2</p>");

    let (b, g) = istek(
        &p,
        "POST",
        "/kaydet",
        &[
            ("CONTENT_TYPE", "application/x-www-form-urlencoded"),
            ("HTTP_USER_AGENT", "Deneme/1"),
        ],
        "ad=%C3%87ay",
    );
    assert!(b.contains("application/json"), "{b}");
    assert_eq!(g, "{\"ad\":\"Çay\",\"tarayıcı\":\"Deneme/1\"}");

    let (b, g) = istek(&p, "GET", "/stil.css", &[], "");
    assert!(b.contains("text/css"), "{b}");
    assert_eq!(g, "body{}");
    let (b, _) = istek(&p, "GET", "/yok", &[], "");
    assert!(b.starts_with("Status: 404"), "{b}");
    let (b, g) = istek(&p, "GET", "/hata", &[], "");
    assert!(b.starts_with("Status: 500"), "{b}");
    assert!(g.contains("liste sınırı"), "{g}");
    // Alt klasöre kurulum: /site/ öneki yollardan çıkarılır.
    let (_, g) = istek(
        &p,
        "GET",
        "/site/",
        &[("SCRIPT_NAME", "/site/uygulama.cgi")],
        "",
    );
    assert_eq!(g, "<p>Ziyaret: 1</p>");
    std::fs::remove_dir_all(&k).unwrap();
}

#[test]
fn yazdiran_program_sayfa_olur() {
    let k = klasor("sayfa");
    std::fs::write(
        k.join("sayfa.ohc"),
        "\"<h1>Merhaba</h1>\"'yı yaz.\n(\"<p>\" + metin(2 + 3) + \"</p>\")'yi yaz.\n",
    )
    .unwrap();
    std::fs::write(k.join("hatali.ohc"), "l = [1]\nl[4]'ü yaz.\n").unwrap();
    orhunca(&k, &["derle", "sayfa.ohc", "-o", "sayfa.cgi"]);
    orhunca(&k, &["derle", "hatali.ohc", "-o", "hatali.cgi"]);
    let (b, g) = istek(&k.join("sayfa.cgi"), "GET", "/sayfa.cgi", &[], "");
    assert!(b.starts_with("Status: 200 OK"), "{b}");
    assert!(b.contains("text/html"), "{b}");
    assert_eq!(g, "<h1>Merhaba</h1>\n<p>5</p>\n");
    let (b, g) = istek(&k.join("hatali.cgi"), "GET", "/hatali.cgi", &[], "");
    assert!(b.starts_with("Status: 500"), "{b}");
    assert!(g.contains("liste sınırı aşıldı"), "{g}");
    // CGI dışında program her zamanki gibi çalışır.
    let c = Command::new(k.join("sayfa.cgi")).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "<h1>Merhaba</h1>\n<p>5</p>\n"
    );
    std::fs::remove_dir_all(&k).unwrap();
}

#[test]
fn yayinla_cgi_klasoru() {
    let k = klasor("yayinla");
    orhunca(&k, &["yeni", "site", "--şablon", "web_sitesi"]);
    let p = k.join("site");
    std::fs::write(p.join(".env.sunucu"), "A=1\n").unwrap();
    orhunca(&p, &["yayınla", "--cgi"]);
    let c = p.join("cikti/cgi");
    let ht = std::fs::read_to_string(c.join(".htaccess")).unwrap();
    assert!(ht.contains("RewriteRule ^\\.well-known/ - [L]"), "{ht}");
    assert!(ht.contains("RewriteRule ^ uygulama.cgi [L]"), "{ht}");
    assert_eq!(
        &std::fs::read(c.join("uygulama.cgi")).unwrap()[..4],
        b"\x7fELF"
    );
    assert_eq!(
        std::fs::read_to_string(c.join("veri/.htaccess")).unwrap(),
        "Require all denied\n"
    );
    assert_eq!(std::fs::read_to_string(c.join(".env")).unwrap(), "A=1\n");
    assert!(c.join("statik/stil.css").is_file());
    let (b, _) = istek(&c.join("uygulama.cgi"), "GET", "/", &[], "");
    assert!(b.starts_with("Status: 200"), "{b}");

    let h = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["yayınla", "--cgi", "kok@sunucu"])
        .current_dir(&p)
        .output()
        .unwrap();
    assert!(!h.status.success());
    std::fs::remove_dir_all(&k).unwrap();
}

/// `--kaynakla`: .ohc dosyaları sunucuda orhunca.cgi ile derlenip çalışır (PHP gibi).
#[test]
fn php_gibi_ohc_dosyalari() {
    let k = klasor("php");
    std::fs::create_dir_all(k.join("blog")).unwrap();
    std::fs::write(k.join("index.ohc"), "\"<h1>Ana sayfa</h1>\"'i yaz.\n").unwrap();
    std::fs::write(
        k.join("merhaba.ohc"),
        "(\"<p>Merhaba \" + ortam(\"REQUEST_METHOD\") + \"</p>\")'yi yaz.\n",
    )
    .unwrap();
    std::fs::write(k.join("blog/index.ohc"), "\"<h2>Blog</h2>\"'yi yaz.\n").unwrap();
    std::fs::write(k.join("bozuk.ohc"), "yaz(\"x\")\n").unwrap();
    orhunca(&k, &["yayınla", "--cgi", "--kaynakla", "index.ohc"]);
    let c = k.join("cikti/cgi");
    let ht = std::fs::read_to_string(c.join(".htaccess")).unwrap();
    assert!(ht.contains("RewriteRule \\.ohc$ orhunca.cgi [L]"), "{ht}");
    let isleyici = c.join("orhunca.cgi");
    let (b, g) = istek(&isleyici, "GET", "/", &[], "");
    assert!(b.starts_with("Status: 200"), "{b}");
    assert_eq!(g, "<h1>Ana sayfa</h1>\n");
    assert!(c.join(".orhunca-onbellek").read_dir().unwrap().count() >= 1);
    let (_, g) = istek(&isleyici, "GET", "/merhaba.ohc?x=1", &[], "");
    assert_eq!(g, "<p>Merhaba GET</p>\n");
    let (_, g) = istek(&isleyici, "GET", "/blog/", &[], "");
    assert_eq!(g, "<h2>Blog</h2>\n");
    let (b, _) = istek(&isleyici, "GET", "/yok.ohc", &[], "");
    assert!(b.starts_with("Status: 404"), "{b}");
    let (b, _) = istek(&isleyici, "GET", "/../../etc/passwd.ohc", &[], "");
    assert!(b.starts_with("Status: 404"), "{b}");
    let (b, g) = istek(&isleyici, "GET", "/bozuk.ohc", &[], "");
    assert!(b.starts_with("Status: 500"), "{b}");
    assert!(g.contains("derlenemedi") && g.contains("yaz"), "{g}");
    // Dosya değişince yeniden derlenir.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(c.join("index.ohc"), "\"<h1>Yeni</h1>\"'yi yaz.\n").unwrap();
    let (_, g) = istek(&isleyici, "GET", "/", &[], "");
    assert_eq!(g, "<h1>Yeni</h1>\n");
    std::fs::remove_dir_all(&k).unwrap();
}

/// Projede (.ohcproj) bütün adresler giriş dosyasına gider; modül dosyaları çalışmaz.
#[test]
fn php_gibi_proje() {
    let k = klasor("proje");
    orhunca(&k, &["yeni", "dukkan", "--şablon", "tam_yigin"]);
    let p = k.join("dukkan");
    orhunca(&p, &["yayınla", "--cgi", "--kaynakla"]);
    let c = p.join("cikti/cgi");
    assert!(c.join("index.ohc").is_file());
    assert!(!c.join("sunucu.ohc").exists());
    let proje = std::fs::read_to_string(c.join("dukkan.ohcproj")).unwrap();
    assert!(proje.contains("giriş = \"index.ohc\""), "{proje}");
    let isleyici = c.join("orhunca.cgi");
    let (b, _) = istek(&isleyici, "GET", "/%C3%BCr%C3%BCnler", &[], "");
    assert!(b.starts_with("Status: 200"), "{b}");
    let (b, _) = istek(&isleyici, "GET", "/modeller/%C3%BCr%C3%BCn.ohc", &[], "");
    assert!(b.starts_with("Status: 404"), "{b}");
    std::fs::remove_dir_all(&k).unwrap();
}
