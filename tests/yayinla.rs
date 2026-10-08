//! `orhunca yayınla`: yayın klasörü, Linux için derlenen program, kurulum betiği ve SSH ile
//! gönderme (ssh yerine sahte bir betikle).

use std::path::{Path, PathBuf};
use std::process::Command;

fn orhunca(klasor: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(args)
        .current_dir(klasor)
        .output()
        .unwrap()
}

fn proje(ad: &str, sablon: &str) -> PathBuf {
    let kok = std::env::temp_dir().join(format!("orhunca-yayinla-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    std::fs::create_dir_all(&kok).unwrap();
    let c = orhunca(&kok, &["yeni", ad, "--şablon", sablon]);
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    kok.join(ad)
}

#[test]
fn yayin_klasoru() {
    let p = proje("Ürün Takibi", "tam_yigin");
    std::fs::write(p.join(".env"), "GELISTIRME=1\n").unwrap();
    std::fs::write(p.join(".env.sunucu"), "SUNUCU=1\n").unwrap();
    let c = orhunca(&p, &["yayınla", "--alan", "Ornek.com", "--kapı", "4100"]);
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let y = p.join("cikti").join("yayın");
    let ikili = std::fs::read(y.join("urun-takibi")).unwrap();
    assert_eq!(&ikili[..4], b"\x7fELF", "Linux programı olmalı");
    assert!(y.join("statik").join("stil.css").is_file());
    // Bu bilgisayardaki .env değil, .env.sunucu gönderilir.
    assert_eq!(
        std::fs::read_to_string(y.join(".env")).unwrap(),
        "SUNUCU=1\n"
    );
    let hizmet = std::fs::read_to_string(y.join("urun-takibi.service")).unwrap();
    assert!(
        hizmet.contains("ExecStart=/srv/urun-takibi/urun-takibi"),
        "{hizmet}"
    );
    assert!(hizmet.contains("Environment=ORHUNCA_KAPI=4100"), "{hizmet}");
    assert!(
        hizmet.contains("Environment=ORHUNCA_ADRES=127.0.0.1"),
        "{hizmet}"
    );
    let kur = std::fs::read_to_string(y.join("kur.sh")).unwrap();
    assert!(kur.contains("ALAN=ornek.com"), "{kur}");
    assert!(kur.contains("reverse_proxy"), "{kur}");
    if cfg!(unix) {
        let d = Command::new("sh")
            .arg("-n")
            .arg(y.join("kur.sh"))
            .status()
            .unwrap();
        assert!(d.success(), "kur.sh sözdizimi");
    }

    // Alan adı yoksa program dışarıya açık kapıda dinler.
    let c = orhunca(&p, &["yayınla", "--klasör", "/opt/urunler"]);
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let hizmet = std::fs::read_to_string(y.join("urun-takibi.service")).unwrap();
    assert!(hizmet.contains("ORHUNCA_ADRES=0.0.0.0"), "{hizmet}");
    assert!(hizmet.contains("WorkingDirectory=/opt/urunler"), "{hizmet}");
    assert!(!std::fs::read_to_string(y.join("kur.sh"))
        .unwrap()
        .contains("caddy"));

    for (args, beklenen) in [
        (&["yayınla", "--alan", "ornek com"][..], "geçersiz alan adı"),
        (
            &["yayınla", "--klasör", "srv/x"][..],
            "geçersiz sunucu klasörü",
        ),
        (
            &["yayınla", "--klasör", "/srv/a b"][..],
            "geçersiz sunucu klasörü",
        ),
        (&["yayınla", "--kapı", "yetmiş"][..], "geçersiz kapı"),
        (&["yayınla", "--hedef", "web"][..], "bilinmeyen seçenek"),
    ] {
        let c = orhunca(&p, args);
        assert!(!c.status.success());
        let m = String::from_utf8_lossy(&c.stderr);
        assert!(m.contains(beklenen), "{args:?}: {m}");
    }
    std::fs::remove_dir_all(p.parent().unwrap()).unwrap();
}

#[test]
fn arayuz_programi_yayinlanmaz() {
    let p = proje("sayac", "arayuz");
    let c = orhunca(&p, &["yayınla"]);
    assert!(!c.status.success());
    assert!(String::from_utf8_lossy(&c.stderr).contains("arayüz programı"));
    std::fs::remove_dir_all(p.parent().unwrap()).unwrap();
}

/// Yayın klasöründeki program sunucudaki gibi çalışır: statik dosyalar ve sayfalar gelir.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn yayinlanan_program_calisir() {
    use std::io::{BufRead, Read, Write};
    let p = proje("site", "web_sitesi");
    assert!(orhunca(&p, &["yayınla"]).status.success());
    let y = p.join("cikti").join("yayın");
    let mut c = Command::new(y.join("site"))
        .current_dir(&y)
        .env("ORHUNCA_KAPI", "0")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut satir = String::new();
    std::io::BufReader::new(c.stdout.take().unwrap())
        .read_line(&mut satir)
        .unwrap();
    let kapi = satir.trim().rsplit(':').next().unwrap().to_string();
    let al = |yol: &str| {
        let mut s = std::net::TcpStream::connect(format!("127.0.0.1:{kapi}")).unwrap();
        write!(
            s,
            "GET {yol} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut m = String::new();
        s.read_to_string(&mut m).unwrap();
        m
    };
    let ana = al("/");
    let stil = al("/stil.css");
    let _ = c.kill();
    let _ = c.wait();
    assert!(ana.starts_with("HTTP/1.1 200"), "{ana}");
    assert!(stil.starts_with("HTTP/1.1 200"), "{stil}");
    std::fs::remove_dir_all(p.parent().unwrap()).unwrap();
}

/// `ssh` yerine komutu yerelde çalıştıran sahte bir betikle: arşiv sunucuya açılır,
/// ardından kurulum betiği çağrılır.
#[cfg(unix)]
#[test]
fn ssh_ile_gonderme() {
    use std::os::unix::fs::PermissionsExt;
    let p = proje("gonder", "web_sitesi");
    let ev = p.parent().unwrap().join("ev");
    let bin = p.parent().unwrap().join("bin");
    std::fs::create_dir_all(&ev).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    let kayit = p.parent().unwrap().join("ssh.txt");
    // Kurulum betiği (sudo sh kur.sh) çalıştırılmaz, yalnızca kaydedilir.
    std::fs::write(
        bin.join("ssh"),
        format!(
            "#!/bin/sh\necho \"$@\" >> '{kayit}'\nwhile [ $# -gt 1 ]; do shift; done\n\
             case \"$1\" in *kur.sh*) exit 0;; esac\nHOME='{ev}' exec sh -c \"$1\"\n",
            kayit = kayit.display(),
            ev = ev.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(bin.join("ssh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let yol = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["yayınla", "kok@203.0.113.5", "--ssh-kapı", "2222"])
        .current_dir(&p)
        .env("PATH", yol)
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let uzak = ev.join(".orhunca-yayin").join("gonder");
    for d in ["gonder", "gonder.service", "kur.sh", "statik/stil.css"] {
        let yerel = std::fs::read(p.join("cikti").join("yayın").join(d)).unwrap();
        assert_eq!(std::fs::read(uzak.join(d)).unwrap(), yerel, "{d}");
    }
    let k = std::fs::read_to_string(&kayit).unwrap();
    let satirlar: Vec<&str> = k.lines().collect();
    assert_eq!(satirlar.len(), 2, "{k}");
    assert!(
        satirlar[0].starts_with("-p 2222 kok@203.0.113.5 rm -rf"),
        "{k}"
    );
    assert!(satirlar[1].starts_with("-p 2222 -t kok@203.0.113.5"), "{k}");
    assert!(satirlar[1].contains("sudo sh"), "{k}");
    std::fs::remove_dir_all(p.parent().unwrap()).unwrap();
}
