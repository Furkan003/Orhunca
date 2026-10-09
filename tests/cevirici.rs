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
    let yollar: Vec<String> = ornekler.iter().map(|a| format!("örnekler/{a}")).collect();
    let yollar: Vec<&str> = yollar.iter().map(String::as_str).collect();
    dene_yollar(dil, komut, uzanti, &yollar);
}

fn dene_yollar(dil: &str, komut: &str, uzanti: &str, ornekler: &[&str]) {
    if Command::new(komut).arg("--version").output().is_err() {
        eprintln!("{komut} yok; {dil} çevirisi atlandı");
        return;
    }
    // Sınamalar aynı anda çalışır: her çağrının kendi klasörü olur.
    static SAYAC: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let sira = SAYAC.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let k = std::env::temp_dir().join(format!("orhunca-cevir-{dil}-{}-{sira}", std::process::id()));
    std::fs::create_dir_all(&k).unwrap();
    for ad in ornekler {
        let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
            .args(["çevir", &format!("{ad}.ohc"), "--dil", dil])
            .output()
            .unwrap();
        assert!(
            c.status.success(),
            "{ad}: {}",
            String::from_utf8_lossy(&c.stderr)
        );
        let dosya = k.join(format!(
            "{}.{uzanti}",
            std::path::Path::new(ad)
                .file_name()
                .unwrap()
                .to_string_lossy()
        ));
        std::fs::write(&dosya, &c.stdout).unwrap();
        let beklenen = std::fs::read_to_string(format!("{ad}.beklenen"))
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
            "c_kütüphanesi",
            "sözlük_sayı_json",
            "bit_işlemleri",
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
            "sözlük_sayı_json",
            "bit_işlemleri",
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

#[test]
fn anlam_farklari_yok() {
    // Türkçe büyük/küçük harf, sıfırdan uzağa yuvarlama, negatif kalan, Unicode harf
    // sayımı, virgüllü ondalık, boş ayraçla bölme, doğru/yanlış ve ondalık yazımı …
    dene_yollar("python", "python3", "py", &["tests/ceviri_anlam"]);
    dene_yollar("javascript", "node", "js", &["tests/ceviri_anlam"]);
}

#[test]
fn buyuk_tamsayi_javascriptte_bildirilir() {
    let k = std::env::temp_dir().join(format!("orhunca-buyuk-{}.ohc", std::process::id()));
    std::fs::write(&k, "(9007199254740993 + 2)'yi yaz.\n").unwrap();
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["çevir"])
        .arg(&k)
        .args(["--dil", "javascript"])
        .output()
        .unwrap();
    let m = String::from_utf8_lossy(&c.stdout);
    assert!(m.contains("2^53") && m.contains("BigInt"), "{m}");
    let _ = std::fs::remove_file(&k);
}
