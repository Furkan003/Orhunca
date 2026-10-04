//! `orhunca paketle`: arayüz programları pencere kabuğuna eklenir. Biçim her
//! zaman sınanır; Linux kabuğu (WebKitGTK ve xvfb varsa) gerçek pencerede de.

use std::path::{Path, PathBuf};
use std::process::Command;

fn orhunca() -> Command {
    Command::new(env!("CARGO_BIN_EXE_orhunca"))
}

fn gecici(ad: &str) -> PathBuf {
    let k = std::env::temp_dir().join(format!("orhunca-paketle-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    k
}

fn paketle(dosya: &Path, cikti: &Path, hedef: &str) {
    let s = orhunca()
        .args(["paketle"])
        .arg(dosya)
        .arg("-o")
        .arg(cikti)
        .args(["--hedef", hedef])
        .output()
        .unwrap();
    assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
}

/// Sondaki yükü çözer: (başlık, sayfa).
fn yuk(veri: &[u8]) -> (String, String) {
    let n = veri.len();
    assert_eq!(&veri[n - 16..n - 8], b"ORHUNCA!");
    let uzunluk = u64::from_le_bytes(veri[n - 8..].try_into().unwrap()) as usize;
    let y = &veri[n - 16 - uzunluk..n - 16];
    assert_eq!(&y[..8], b"OHCKABUK");
    let b = u32::from_le_bytes(y[8..12].try_into().unwrap()) as usize;
    (
        String::from_utf8(y[12..12 + b].to_vec()).unwrap(),
        String::from_utf8(y[12 + b..].to_vec()).unwrap(),
    )
}

fn ornek(ad: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("örnekler/arayüz")
        .join(ad)
}

#[test]
fn linux_ve_windows_icin_paketlenir() {
    let k = gecici("bicim");
    for (hedef, cikti, bas) in [
        ("linux", k.join("sayaç"), &b"\x7fELF"[..]),
        ("windows", k.join("sayaç.exe"), &b"MZ"[..]),
    ] {
        paketle(&ornek("sayaç.ohc"), &cikti, hedef);
        let veri = std::fs::read(&cikti).unwrap();
        assert!(veri.starts_with(bas), "{hedef}");
        let (baslik, sayfa) = yuk(&veri);
        assert_eq!(baslik, "sayaç");
        assert!(sayfa.starts_with("<!DOCTYPE html>") || sayfa.starts_with("<!doctype html>"));
        assert!(
            sayfa.contains("orhuncaKabukDepo"),
            "yükleyici kabuk deposunu tanımalı"
        );
    }
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn hatali_program_paketlenmez() {
    let k = gecici("hata");
    let dosya = k.join("bozuk.ohc");
    std::fs::write(&dosya, "yaz(tanımsız)\n").unwrap();
    let s = orhunca()
        .arg("paketle")
        .arg(&dosya)
        .arg("-o")
        .arg(k.join("bozuk"))
        .output()
        .unwrap();
    assert!(!s.status.success());
    assert!(String::from_utf8_lossy(&s.stderr).contains("tanımsız"));
    assert!(!k.join("bozuk").exists());
    let _ = std::fs::remove_dir_all(&k);
}

/// Paketlenen uygulama gerçek pencerede açılır; dosyaları kalıcıdır (ikinci
/// açılışta önceki yazdığını okur).
#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn linux_penceresinde_calisir() {
    let xvfb = Command::new("xvfb-run").arg("--help").output();
    if xvfb.is_err() {
        eprintln!("xvfb-run yok; pencere sınaması atlandı");
        return;
    }
    let k = gecici("pencere");
    let dosya = k.join("açılış.ohc");
    std::fs::write(
        &dosya,
        "işlev say() -> metin:\n    n = 0\n    eğer dosya_var(\"sayı.txt\") ise:\n        n = sayı(dosya_oku(\"sayı.txt\"))\n    n += 1\n    dosyaya_yaz(\"sayı.txt\", metin(n))\n    döndür \"Açılış: \" + n\n\ndurum ileti = say()\n\narayüz:\n    başlık(\"Açılış sayacı\")\n    yazı(ileti)\n",
    )
    .unwrap();
    let uygulama = k.join("açılış");
    paketle(&dosya, &uygulama, "linux");
    for beklenen in ["Açılış: 1", "Açılış: 2"] {
        let s = Command::new("xvfb-run")
            .arg("-a")
            .arg(&uygulama)
            .env("ORHUNCA_KABUK_SINAMA", "1")
            .env("XDG_DATA_HOME", k.join("veri"))
            .output()
            .unwrap();
        let hata = String::from_utf8_lossy(&s.stderr);
        if hata.contains("error while loading shared libraries") {
            eprintln!("WebKitGTK kurulu değil; pencere sınaması atlandı");
            break;
        }
        let cikti = String::from_utf8_lossy(&s.stdout);
        assert!(s.status.success(), "{hata}");
        assert!(cikti.contains("Açılış sayacı"), "{cikti}");
        assert!(cikti.contains(beklenen), "{cikti}");
    }
    let _ = std::fs::remove_dir_all(&k);
}
