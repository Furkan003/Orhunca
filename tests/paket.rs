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

/// Paket mağazası güvenliği: izin onayı, yeni izin isteyen güncelleme, işleme ve içerik
/// özeti doğrulaması.
#[test]
fn paket_izinleri_ve_magaza_dogrulamasi() {
    let kok = std::env::temp_dir().join(format!("orhunca-izin-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    std::fs::create_dir_all(&kok).unwrap();
    let dosyaci = depo(
        &kok,
        "dosyaci",
        &[
            (
                "dosyaci.ohcproj",
                "ad = \"dosyaci\"\nsürüm = \"1.0.0\"\ngiriş = \"dosyaci.ohc\"\naçıklama = \"Dosya yardımcıları\"\n",
            ),
            (
                "dosyaci.ohc",
                "işlev oku_hepsi(yol: metin) -> metin:\n    döndür dosya_oku(yol)\n",
            ),
        ],
    );
    git(&dosyaci, &["tag", "v1.0.0"]);
    let kaynak = format!("{}#v1.0.0", dosyaci.display());

    // İnceleme: izinler koddan çıkarılır.
    let (ok, m) = orhunca(&kok, &["paket", "bilgi", &kaynak, "--json"]);
    assert!(ok, "{m}");
    let bilgi: serde_json::Value = serde_json::from_str(m.trim()).unwrap();
    assert_eq!(bilgi["ad"], "dosyaci");
    assert_eq!(bilgi["sürüm"], "1.0.0");
    assert_eq!(bilgi["izinler"], serde_json::json!(["dosya"]));
    let ozet = bilgi["özet"].as_str().unwrap().to_string();
    let isleme = bilgi["işleme"].as_str().unwrap().to_string();

    let (ok, m) = orhunca(&kok, &["yeni", "uyg"]);
    assert!(ok, "{m}");
    let uyg = kok.join("uyg");
    // Onaysız (etkileşimsiz) eklenmez ve proje değişmez.
    let (ok, m) = orhunca(&uyg, &["paket", "ekle", &kaynak]);
    assert!(
        !ok && m.contains("dosya — bilgisayardaki dosyaları") && m.contains("--izin-ver"),
        "{m}"
    );
    assert!(!std::fs::read_to_string(uyg.join("uyg.ohcproj"))
        .unwrap()
        .contains("dosyaci"));
    assert!(!uyg.join("paketler/dosyaci").exists());
    let (ok, m) = orhunca(&uyg, &["paket", "ekle", &kaynak, "--izin-ver"]);
    assert!(ok, "{m}");
    let kilit = std::fs::read_to_string(uyg.join("orhunca.kilit")).unwrap();
    assert!(kilit.contains("izinler = \"dosya\""), "{kilit}");
    assert!(kilit.contains(&format!("özet = \"{ozet}\"")), "{kilit}");

    // Yeni izin (ağ) isteyen sürüm onaysız kurulmaz; eski sürüm yerinde kalır.
    std::fs::write(
        dosyaci.join("dosyaci.ohc"),
        "işlev oku_hepsi(yol: metin) -> metin:\n    döndür dosya_oku(yol)\n\nişlev indir(a: metin) -> metin:\n    döndür http_al(a)\n",
    )
    .unwrap();
    git(&dosyaci, &["commit", "-qam", "iki"]);
    git(&dosyaci, &["tag", "v1.1.0"]);
    let ayar = std::fs::read_to_string(uyg.join("uyg.ohcproj")).unwrap();
    std::fs::write(uyg.join("uyg.ohcproj"), ayar.replace("#v1.0.0", "#v1.1.0")).unwrap();
    let (ok, m) = orhunca(&uyg, &["paket", "yükle"]);
    assert!(!ok && m.contains("ağ — internete"), "{m}");
    assert!(
        !std::fs::read_to_string(uyg.join("paketler/dosyaci/dosyaci.ohc"))
            .unwrap()
            .contains("http_al")
    );
    let (ok, m) = orhunca(&uyg, &["paket", "yükle", "--izin-ver"]);
    assert!(ok, "{m}");
    assert!(std::fs::read_to_string(uyg.join("orhunca.kilit"))
        .unwrap()
        .contains("izinler = \"dosya, ağ\""));
    let (ok, m) = orhunca(&uyg, &["paket", "listele"]);
    assert!(ok && m.contains("[izinler: dosya, ağ]"), "{m}");

    // Mağazadan ekleme: kayıttaki işleme ve özet tutmazsa kurulmaz.
    let dizin = kok.join("dizin.json");
    let calistir = |ozet: &str, isleme: &str| {
        std::fs::write(
            &dizin,
            serde_json::json!({ "sürüm": 2, "paketler": [{
                "ad": "dosyaci", "açıklama": "Dosya yardımcıları", "kaynak": kaynak,
                "sürüm": "1.0.0", "sahip": "deneme", "işleme": isleme, "özet": ozet,
                "izinler": ["dosya"],
            }]})
            .to_string(),
        )
        .unwrap();
        let p = kok.join(format!("p{}", &ozet[ozet.len() - 4..]));
        let _ = std::fs::remove_dir_all(&p);
        Command::new(env!("CARGO_BIN_EXE_orhunca"))
            .args(["yeni", p.file_name().unwrap().to_str().unwrap()])
            .current_dir(&kok)
            .output()
            .unwrap();
        let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
            .args(["paket", "ekle", "dosyaci", "--izin-ver"])
            .current_dir(&p)
            .env("ORHUNCA_PAKET_DIZINI", &dizin)
            .output()
            .unwrap();
        let mut m = String::from_utf8_lossy(&c.stdout).into_owned();
        m.push_str(&String::from_utf8_lossy(&c.stderr));
        (c.status.success(), m, p)
    };
    let (ok, m, _) = calistir(&ozet, &isleme);
    assert!(ok, "{m}");
    let (ok, m, p) = calistir("sha256:0000", &isleme);
    assert!(!ok && m.contains("özetle uyuşmuyor"), "{m}");
    assert!(!p.join("paketler/dosyaci").exists());
    let (ok, m, _) = calistir(&ozet, "0000000000000000000000000000000000000000");
    assert!(!ok && m.contains("işlemeyi göstermiyor"), "{m}");
    let _ = std::fs::remove_dir_all(&kok);
}
