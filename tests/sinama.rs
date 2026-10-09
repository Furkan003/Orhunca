//! `orhunca sına`: sınama dosyalarını bulma, çalıştırma ve sonuçları.

use std::process::Command;

#[test]
fn orhunca_sina() {
    let k = std::env::temp_dir().join(format!("orhunca-sina-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(k.join("sınamalar")).unwrap();
    std::fs::create_dir_all(k.join("paketler/p")).unwrap();
    std::fs::write(
        k.join("hesap.ohc"),
        "işlev topla(a: sayı, b: sayı) -> sayı:\n    döndür a + b\n",
    )
    .unwrap();
    std::fs::write(
        k.join("hesap_sına.ohc"),
        "kullan \"hesap.ohc\"\n\nişlev sına_toplama():\n    eşit_olmalı(topla(2, 3), 5)\n    eşit_olmalı([\"a\"], [\"a\"])\n\n\
         işlev sına_yanlış():\n    \"ara çıktı\"'yı yaz.\n    eşit_olmalı(topla(2, 2), 5)\n\n\
         işlev sına_doğrula():\n    doğrula(1 > 2, \"bir ikiden büyük değil\")\n\n\
         işlev sına_çöken():\n    l = [1]\n    l[3]'ü yaz.\n\n\
         işlev yardımcı():\n    1'i yaz.\n",
    )
    .unwrap();
    std::fs::write(
        k.join("sınamalar/metin.ohc"),
        "işlev sına_harf():\n    eşit_olmalı(büyük_harf(\"i\"), \"İ\")\n",
    )
    .unwrap();
    // Paketlerdeki sınamalar çalıştırılmaz
    std::fs::write(
        k.join("paketler/p/p_sına.ohc"),
        "işlev sına_x():\n    doğrula(yanlış)\n",
    )
    .unwrap();

    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["sına", "--json"])
        .current_dir(&k)
        .output()
        .unwrap();
    assert!(!c.status.success());
    let j: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&c.stdout).trim()).unwrap();
    assert_eq!(
        (j["gecti"].as_u64(), j["kaldi"].as_u64()),
        (Some(2), Some(3)),
        "{j}"
    );
    let dosyalar = j["dosyalar"].as_array().unwrap();
    assert_eq!(dosyalar.len(), 2, "{j}");
    assert_eq!(dosyalar[0]["dosya"], "hesap_sına.ohc");
    let sinama = |ad: &str| {
        dosyalar[0]["sinamalar"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["ad"] == ad)
            .cloned()
            .unwrap_or_else(|| panic!("{ad} yok: {j}"))
    };
    assert_eq!(sinama("sına_toplama")["gecti"], true);
    let y = sinama("sına_yanlış");
    assert_eq!(y["mesaj"], "hesap_sına.ohc:9: beklenen 5, bulunan 4", "{y}");
    assert_eq!(y["cikti"], "ara çıktı\n", "{y}");
    assert_eq!(y["satir"], 7);
    let d = sinama("sına_doğrula");
    assert!(
        d["mesaj"]
            .as_str()
            .unwrap()
            .contains("hesap_sına.ohc:12: doğrulanamadı")
            && d["mesaj"]
                .as_str()
                .unwrap()
                .ends_with("bir ikiden büyük değil"),
        "{d}"
    );
    assert!(sinama("sına_çöken")["mesaj"]
        .as_str()
        .unwrap()
        .contains("liste"));
    assert!(dosyalar[0]["sinamalar"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["ad"] != "yardımcı"));

    // --ad süzgeci; hepsi geçince çıkış kodu 0
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["sına", "--ad", "toplama"])
        .current_dir(&k)
        .output()
        .unwrap();
    let cikti = String::from_utf8_lossy(&c.stdout);
    assert!(c.status.success(), "{cikti}");
    assert!(
        cikti.contains("✓ sına_toplama") && cikti.contains("1 sınama: 1 geçti, 0 kaldı"),
        "{cikti}"
    );

    // Derleme hatası olan sınama dosyası
    std::fs::write(k.join("bozuk_sına.ohc"), "işlev sına_a():\n    doğrula()\n").unwrap();
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["sına", "bozuk_sına.ohc", "--json"])
        .current_dir(&k)
        .output()
        .unwrap();
    let j: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&c.stdout).trim()).unwrap();
    assert!(
        j["dosyalar"][0]["hata"]
            .as_str()
            .unwrap()
            .contains("doğrula bir koşul alır"),
        "{j}"
    );
    std::fs::remove_dir_all(&k).unwrap();
}

#[test]
fn sina_islevi_olmayan_dosya_tek_sinamadir() {
    // 1.0 test raporu, bulgu 7: üst düzeyde eşit_olmalı yazılan dosya hiç sınanmıyordu.
    let k = std::env::temp_dir().join(format!("orhunca-sina-ust-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    std::fs::write(k.join("hesap_sına.ohc"), "eşit_olmalı(2 + 2, 4)\n").unwrap();
    std::fs::write(
        k.join("sınama_yanlış.ohc"),
        "\"ara\"'yı yaz.\neşit_olmalı(2 + 2, 5)\n",
    )
    .unwrap();
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["sına", "--json"])
        .current_dir(&k)
        .output()
        .unwrap();
    let j: serde_json::Value = serde_json::from_slice(&c.stdout).unwrap();
    assert_eq!(j["gecti"], 1, "{j}");
    assert_eq!(j["kaldi"], 1, "{j}");
    let yanlis = &j["dosyalar"][1]["sinamalar"][0];
    assert_eq!(yanlis["ad"], "sınama_yanlış", "{j}");
    assert_eq!(yanlis["satir"], 2, "{j}");
    assert!(
        yanlis["mesaj"]
            .as_str()
            .unwrap()
            .contains("beklenen 5, bulunan 4"),
        "{j}"
    );
    assert_eq!(yanlis["cikti"], "ara\n", "{j}");
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn resmi_paketlerin_sinamalari_gecer() {
    // kütüphaneler/ altındaki her paketin *_sına.ohc dosyaları
    let kok = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("kütüphaneler");
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args(["sına", "--json"])
        .current_dir(&kok)
        .output()
        .unwrap();
    let j: serde_json::Value = serde_json::from_slice(&c.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&c.stderr)));
    assert_eq!(j["kaldi"], 0, "{j:#}");
    assert!(j["gecti"].as_u64().unwrap() >= 19, "{j}");
}
