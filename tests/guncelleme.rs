//! `orhunca güncelle`: sahte bir sürüm (yerel HTTP sunucusu) ile indirme, SHA-256
//! doğrulaması ve komutun kendi dosyasını değiştirmesi.
#![cfg(all(unix, not(target_os = "macos")))]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;

/// Klasördeki dosyaları sunan küçük bir HTTP sunucusu; adresini döndürür.
fn sunucu(klasor: &Path) -> String {
    let d = TcpListener::bind("127.0.0.1:0").unwrap();
    let adres = format!("http://{}", d.local_addr().unwrap());
    let klasor = klasor.to_path_buf();
    std::thread::spawn(move || {
        for a in d.incoming() {
            let Ok(mut a) = a else { continue };
            let mut ilk = String::new();
            let mut r = BufReader::new(a.try_clone().unwrap());
            if r.read_line(&mut ilk).is_err() {
                continue;
            }
            loop {
                let mut s = String::new();
                if r.read_line(&mut s).unwrap_or(0) <= 2 {
                    break;
                }
            }
            let yol = ilk
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .trim_start_matches('/');
            match std::fs::read(klasor.join(yol)) {
                Ok(v) => {
                    let _ = write!(
                        a,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        v.len()
                    );
                    let _ = a.write_all(&v);
                }
                Err(_) => {
                    let _ = write!(
                        a,
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                }
            }
        }
    });
    adres
}

fn sha(p: &Path) -> String {
    orhunca::guncelleme::sha256(&std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn guncelle_indirir_dogrular_ve_degistirir() {
    let kok = std::env::temp_dir().join(format!("orhunca-guncelleme-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    let surum = kok.join("surum");
    let paket = kok.join("paket");
    std::fs::create_dir_all(&surum).unwrap();
    std::fs::create_dir_all(&paket).unwrap();
    // "Yeni sürüm": kendini tanıtan küçük bir betik
    std::fs::write(paket.join("orhunca"), "#!/bin/sh\necho yeni-surum\n").unwrap();
    let ad = if cfg!(target_arch = "aarch64") {
        "orhunca-linux-aarch64.tar.gz"
    } else {
        "orhunca-linux-x86_64.tar.gz"
    };
    assert!(Command::new("tar")
        .args(["-czf"])
        .arg(surum.join(ad))
        .arg("-C")
        .arg(&paket)
        .arg("orhunca")
        .status()
        .unwrap()
        .success());
    std::fs::write(
        surum.join("SHA256SUMS.txt"),
        format!("{}  {ad}\n", sha(&surum.join(ad))),
    )
    .unwrap();
    let adres = sunucu(&surum);
    let json = kok.join("son.json");
    let yaz_json = |ozet: &str| {
        std::fs::write(
            &json,
            serde_json::json!({
                "tag_name": "v99.0.0",
                "body": "notlar",
                "html_url": "x",
                "assets": [
                    { "name": ad, "browser_download_url": format!("{adres}/{ad}"), "size": 1 },
                    { "name": "SHA256SUMS.txt", "browser_download_url": format!("{adres}/{ozet}"), "size": 1 },
                ]
            })
            .to_string(),
        )
        .unwrap();
    };
    // Kurulu komut: derlenen orhunca'nın bir kopyası
    let kurulu = kok.join("bin").join("orhunca");
    std::fs::create_dir_all(kurulu.parent().unwrap()).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_orhunca"), &kurulu).unwrap();
    let calistir = |args: &[&str]| {
        let c = Command::new(&kurulu)
            .args(args)
            .env("ORHUNCA_GUNCELLEME_ADRESI", &json)
            .env_remove("APPIMAGE")
            .output()
            .unwrap();
        let mut m = String::from_utf8_lossy(&c.stdout).into_owned();
        m.push_str(&String::from_utf8_lossy(&c.stderr));
        (c.status.success(), m)
    };

    // Özet tutmayan dosya reddedilir, komut değişmez.
    std::fs::write(
        surum.join("BOZUK.txt"),
        format!("{}  {ad}\n", "0".repeat(64)),
    )
    .unwrap();
    yaz_json("BOZUK.txt");
    let (ok, m) = calistir(&["güncelle"]);
    assert!(!ok && m.contains("doğrulanamadı"), "{m}");
    assert!(calistir(&["sürüm"]).1.contains(orhunca::SURUM));

    // Denetim: yalnızca bildirir
    yaz_json("SHA256SUMS.txt");
    let (ok, m) = calistir(&["güncelle", "--denetle"]);
    assert!(ok && m.contains("99.0.0"), "{m}");

    let (ok, m) = calistir(&["güncelle"]);
    assert!(ok && m.contains("Doğrulandı"), "{m}");
    let c = Command::new(&kurulu).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&c.stdout), "yeni-surum\n");

    // Yönetici kapattıysa Stüdyo'nun denetimi çalışmaz
    assert!(orhunca::guncelleme::daha_yeni("99.0.0", orhunca::SURUM));
    let _ = std::fs::remove_dir_all(&kok);
}
