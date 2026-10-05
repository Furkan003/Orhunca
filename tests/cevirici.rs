//! `orhunca çevir`: örneklerin Python ve JavaScript karşılıkları çalıştırılıp çıktıları
//! Orhunca'nınkiyle karşılaştırılır (python3 / node yoksa o dil atlanır).

use std::process::Command;

fn calistir(komut: &str, dosya: &std::path::Path) -> Option<String> {
    let c = Command::new(komut).arg(dosya).output().ok()?;
    assert!(
        c.status.success(),
        "{komut} {}: {}",
        dosya.display(),
        String::from_utf8_lossy(&c.stderr)
    );
    Some(String::from_utf8_lossy(&c.stdout).replace('\r', ""))
}

fn dene(dil: &str, komut: &str, uzanti: &str, ornekler: &[&str]) {
    if Command::new(komut).arg("--version").output().is_err() {
        eprintln!("{komut} yok; {dil} çevirisi atlandı");
        return;
    }
    let k = std::env::temp_dir().join(format!("orhunca-cevir-{dil}-{}", std::process::id()));
    std::fs::create_dir_all(&k).unwrap();
    for ad in ornekler {
        let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
            .args(["çevir", &format!("örnekler/{ad}.ohc"), "--dil", dil])
            .output()
            .unwrap();
        assert!(
            c.status.success(),
            "{ad}: {}",
            String::from_utf8_lossy(&c.stderr)
        );
        let dosya = k.join(format!("{ad}.{uzanti}"));
        std::fs::write(&dosya, &c.stdout).unwrap();
        let beklenen = std::fs::read_to_string(format!("örnekler/{ad}.beklenen"))
            .unwrap()
            .replace('\r', "");
        let cikti = calistir(komut, &dosya).unwrap();
        assert_eq!(
            cikti,
            beklenen,
            "{ad} ({dil}):\n{}",
            String::from_utf8_lossy(&c.stdout)
        );
    }
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn python_karsiligi() {
    dene(
        "python",
        "python3",
        "py",
        &[
            "merhaba",
            "asal",
            "faktoriyel",
            "fizzbuzz",
            "kelime_sayacı",
            "not_ortalaması",
            "sayilar",
            "birim_çevirici",
        ],
    );
}

#[test]
fn javascript_karsiligi() {
    dene(
        "javascript",
        "node",
        "js",
        &[
            "merhaba",
            "faktoriyel",
            "fizzbuzz",
            "kelime_sayacı",
            "not_ortalaması",
        ],
    );
}

#[test]
fn ozel_bolumler_aciklanir() {
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["çevir", "örnekler/arayüz/sayaç.ohc"])
        .output()
        .unwrap();
    let m = String::from_utf8_lossy(&c.stdout);
    assert!(m.contains("Orhunca'ya özeldir"), "{m}");
}
