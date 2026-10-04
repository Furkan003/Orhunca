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

#[test]
fn paket_dizininden_alt_klasor() {
    let kok = std::env::temp_dir().join(format!("orhunca-dizin-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    std::fs::create_dir_all(&kok).unwrap();
    let depo_yolu = kok.join("resmi");
    std::fs::create_dir_all(depo_yolu.join("kütüphaneler/istatistik")).unwrap();
    std::fs::write(depo_yolu.join("BENİOKU.md"), "depo\n").unwrap();
    let depo = depo(
        &kok,
        "resmi",
        &[
            (
                "kütüphaneler/istatistik/istatistik.ohcproj",
                "ad = \"istatistik\"\ngiriş = \"istatistik.ohc\"\n",
            ),
            (
                "kütüphaneler/istatistik/istatistik.ohc",
                "işlev ortalama(l) -> ondalık:\n    döndür toplam(l) / uzunluk(l)\n",
            ),
        ],
    );
    let dizin = kok.join("dizin.json");
    std::fs::write(
        &dizin,
        format!(
            r#"{{"paketler": [{{"ad": "istatistik", "açıklama": "Ortalama ve sapma", "kaynak": "{}#:kütüphaneler/istatistik"}}]}}"#,
            depo.display()
        ),
    )
    .unwrap();
    let calistir = |klasor: &Path, args: &[&str]| {
        let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
            .args(args)
            .current_dir(klasor)
            .env("ORHUNCA_PAKET_DIZINI", &dizin)
            .output()
            .unwrap();
        let mut m = String::from_utf8_lossy(&c.stdout).into_owned();
        m.push_str(&String::from_utf8_lossy(&c.stderr));
        (c.status.success(), m)
    };

    let (ok, m) = calistir(&kok, &["paket", "ara", "ortalam"]);
    assert!(
        ok && m.contains("istatistik") && m.contains("Ortalama ve sapma"),
        "{m}"
    );

    let (ok, m) = orhunca(&kok, &["yeni", "proje"]);
    assert!(ok, "{m}");
    let proje = kok.join("proje");
    std::fs::write(
        proje.join("ana.ohc"),
        "kullan \"istatistik\"\nortalama([1, 2, 6])'yı yaz.\n",
    )
    .unwrap();

    let (ok, m) = calistir(&proje, &["paket", "ekle", "istatstik"]);
    assert!(
        !ok && m.contains("bunu mu demek istediniz: istatistik"),
        "{m}"
    );

    let (ok, m) = calistir(&proje, &["paket", "ekle", "istatistik"]);
    assert!(ok, "{m}");
    let paket = proje.join("paketler/istatistik");
    assert!(paket.join("istatistik.ohc").exists());
    assert!(!paket.join("BENİOKU.md").exists() && !paket.join(".git").exists());
    let (ok, m) = calistir(&proje, &["çalıştır"]);
    assert!(ok && m == "3.0\n", "{m}");

    // Kilitteki işlemeyle yeniden kurulur; zaten kuruluysa dokunulmaz.
    std::fs::remove_dir_all(proje.join("paketler")).unwrap();
    let (ok, m) = calistir(&proje, &["paket", "yükle"]);
    assert!(ok && m.contains("indiriliyor"), "{m}");
    let (ok, m) = calistir(&proje, &["paket", "yükle"]);
    assert!(ok && !m.contains("indiriliyor"), "{m}");

    // Deponun dışına çıkan alt klasör ve seçenek gibi görünen etiket reddedilir.
    for kotu in ["#:../x", "#--upload-pack=x:a"] {
        let (ok, m) = calistir(
            &proje,
            &[
                "paket",
                "ekle",
                &format!("{}{kotu}", depo.display()),
                "--ad",
                "kotu",
            ],
        );
        assert!(!ok && m.contains("geçersiz"), "{kotu}: {m}");
    }
    let (ok, m) = calistir(
        &proje,
        &["paket", "ekle", "/yok/böyle/bir/depo", "--ad", "yok"],
    );
    assert!(!ok, "{m}");
    let ayar = std::fs::read_to_string(proje.join("proje.ohcproj")).unwrap();
    assert!(!ayar.contains("kotu") && !ayar.contains("yok"), "{ayar}");
    let (ok, m) = calistir(&proje, &["çalıştır"]);
    assert!(ok && m == "3.0\n", "{m}");
    let _ = std::fs::remove_dir_all(&kok);
}

/// kütüphaneler/dizin.json'daki her resmi paket depoda var, derleniyor ve adı doğru.
#[test]
fn resmi_paketler_derlenir() {
    let kok = Path::new(env!("CARGO_MANIFEST_DIR")).join("kütüphaneler");
    let dizin: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(kok.join("dizin.json")).unwrap()).unwrap();
    let gecici = std::env::temp_dir().join(format!("orhunca-resmi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&gecici);
    for p in dizin["paketler"].as_array().unwrap() {
        let ad = p["ad"].as_str().unwrap();
        let kaynak = p["kaynak"].as_str().unwrap();
        assert!(
            kaynak.ends_with(&format!(":kütüphaneler/{ad}")),
            "{ad}: {kaynak}"
        );
        assert!(!p["açıklama"].as_str().unwrap().is_empty());
        let hedef = gecici.join("paketler").join(ad);
        std::fs::create_dir_all(&hedef).unwrap();
        for g in std::fs::read_dir(kok.join(ad)).unwrap() {
            let g = g.unwrap();
            std::fs::copy(g.path(), hedef.join(g.file_name())).unwrap();
        }
        let dosya = gecici.join(format!("{ad}.ohc"));
        std::fs::write(&dosya, format!("kullan \"{ad}\"\n\"tamam\"'ı yaz.\n")).unwrap();
        let (ok, m) = orhunca(&gecici, &["çalıştır", dosya.to_str().unwrap()]);
        assert!(ok && m == "tamam\n", "{ad}: {m}");
    }
    let _ = std::fs::remove_dir_all(&gecici);
}
