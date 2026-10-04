//! Paket yöneticisinin uçtan uca testi: yerel Git depolarıyla.

use std::path::Path;
use std::process::Command;

fn orhunca(klasor: &Path, args: &[&str]) -> (bool, String) {
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(args)
        .current_dir(klasor)
        .output()
        .unwrap();
    let mut metin = String::from_utf8_lossy(&c.stdout).into_owned();
    metin.push_str(&String::from_utf8_lossy(&c.stderr));
    (c.status.success(), metin)
}

fn git(klasor: &Path, args: &[&str]) {
    let c = Command::new("git")
        .args(args)
        .current_dir(klasor)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(
        c.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&c.stderr)
    );
}

fn depo(kok: &Path, ad: &str, dosyalar: &[(&str, &str)]) -> std::path::PathBuf {
    let d = kok.join(ad);
    std::fs::create_dir_all(&d).unwrap();
    git(&d, &["init", "-q"]);
    for (a, i) in dosyalar {
        std::fs::write(d.join(a), i).unwrap();
    }
    git(&d, &["add", "-A"]);
    git(&d, &["commit", "-qm", "ilk"]);
    d
}

#[test]
fn paket_ekle_kullan_kaldir() {
    let kok = std::env::temp_dir().join(format!("orhunca-paket-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    std::fs::create_dir_all(&kok).unwrap();

    let matematik = depo(
        &kok,
        "matematik",
        &[
            (
                "matematik.ohcproj",
                "ad = \"matematik\"\ngiriş = \"matematik.ohc\"\n",
            ),
            (
                "matematik.ohc",
                "fiil sayı'yı karele:\n    döndür sayı * sayı\n",
            ),
        ],
    );
    git(&matematik, &["tag", "v1.0"]);
    let geometri = depo(
        &kok,
        "geometri",
        &[
            (
                "geometri.ohcproj",
                &format!(
                    "ad = \"geometri\"\ngiriş = \"geometri.ohc\"\n\n[bağımlılıklar]\nmatematik = \"{}#v1.0\"\n",
                    matematik.display()
                ),
            ),
            (
                "geometri.ohc",
                "kullan \"matematik\"\n\nfiil (kenar: sayı)'dan alan_hesapla:\n    döndür kenar'ı karele\n",
            ),
        ],
    );

    let (ok, m) = orhunca(&kok, &["yeni", "uygulama"]);
    assert!(ok, "{m}");
    let proje = kok.join("uygulama");
    std::fs::write(
        proje.join("ana.ohc"),
        "kullan \"geometri\"\n(5'ten alan_hesapla)'yı yaz.\n",
    )
    .unwrap();

    // Kurulmadan önce anlaşılır hata
    let (ok, m) = orhunca(&proje, &["çalıştır"]);
    assert!(!ok);
    assert!(m.contains("orhunca paket ekle"), "{m}");

    let (ok, m) = orhunca(&proje, &["paket", "ekle", geometri.to_str().unwrap()]);
    assert!(ok, "{m}");
    assert!(m.contains("2 paket hazır"), "{m}");
    let kilit = std::fs::read_to_string(proje.join("orhunca.kilit")).unwrap();
    assert!(
        kilit.contains("[matematik]") && kilit.contains("#v1.0"),
        "{kilit}"
    );

    let (ok, m) = orhunca(&proje, &["çalıştır"]);
    assert!(ok, "{m}");
    assert_eq!(m, "25\n");

    // Paket klasörü silinse bile kilitteki sürümle yeniden kurulur.
    std::fs::remove_dir_all(proje.join("paketler")).unwrap();
    let (ok, m) = orhunca(&proje, &["paket", "yükle"]);
    assert!(ok, "{m}");
    let (ok, m) = orhunca(&proje, &["çalıştır"]);
    assert!(ok && m == "25\n", "{m}");

    let (ok, m) = orhunca(&proje, &["paket", "kaldır", "geometri"]);
    assert!(ok, "{m}");
    assert!(!proje.join("paketler").join("matematik").exists());

    let _ = std::fs::remove_dir_all(&kok);
}
