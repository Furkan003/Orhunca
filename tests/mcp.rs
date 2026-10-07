//! `orhunca mcp`: MCP sunucusu (satır başına bir JSON-RPC iletisi).

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

fn konus(iletiler: &[Value]) -> Vec<Value> {
    let mut c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut g = c.stdin.take().unwrap();
        for i in iletiler {
            writeln!(g, "{i}").unwrap();
        }
    }
    let c = c.wait_with_output().unwrap();
    String::from_utf8(c.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn cagir(kimlik: u64, ad: &str, girdi: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": kimlik, "method": "tools/call", "params": { "name": ad, "arguments": girdi } })
}

#[test]
fn mcp_sunucusu() {
    let y = konus(&[
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "sinama", "version": "1" } } }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        cagir(
            3,
            "orhunca_calistir",
            json!({ "kod": "ad = oku()\n(\"Merhaba \" + ad)'yı yaz.\n", "girdi": "Ayşe\n" }),
        ),
        cagir(
            4,
            "orhunca_denetle",
            json!({ "kod": "her i için 1den 3e kadar:\n    i'yi yaz.\n" }),
        ),
        cagir(
            5,
            "orhunca_bicimlendir",
            json!({ "kod": "eğer 1 < 2 ise:\n  5'a yaz.\n" }),
        ),
        cagir(6, "orhunca_rehber", json!({})),
        json!({ "jsonrpc": "2.0", "id": 7, "method": "resources/read", "params": { "uri": "orhunca://rehber" } }),
        json!({ "jsonrpc": "2.0", "id": 8, "method": "yok" }),
        cagir(9, "orhunca_rehber", json!({ "bolum": "oyunlar" })),
    ]);
    // Bildirime yanıt verilmez
    assert_eq!(y.len(), 9, "{y:?}");
    assert_eq!(y[0]["result"]["serverInfo"]["name"], "orhunca");
    let araclar: Vec<&str> = y[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        araclar,
        [
            "orhunca_rehber",
            "orhunca_denetle",
            "orhunca_calistir",
            "orhunca_bicimlendir",
            "orhunca_arayuz",
            "orhunca_yeni_proje",
            "orhunca_dosyalar",
            "orhunca_dosya_oku",
            "orhunca_dosya_yaz"
        ]
    );
    let metin = |i: usize| {
        y[i]["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(metin(2).contains("Merhaba Ayşe"), "{}", metin(2));
    assert!(metin(3).contains("1'den"), "{}", metin(3));
    assert!(metin(3).contains("program.ohc:1:12"), "{}", metin(3));
    assert_eq!(metin(4), "eğer 1 < 2 ise:\n    5'e yaz.\n");
    assert!(metin(5).contains("Hâl ekleri"));
    assert!(y[6]["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("arayüz"));
    assert_eq!(y[7]["error"]["code"], -32601);
    assert!(metin(8).contains("her_karede"), "{}", metin(8));
}

fn metni(y: &Value) -> String {
    y["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn mcp_proje_araclari() {
    let kok = std::env::temp_dir().join(format!("orhunca-mcp-proje-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    std::fs::create_dir_all(&kok).unwrap();
    let proje = kok.join("deneme");
    let y = konus(&[
        cagir(
            1,
            "orhunca_yeni_proje",
            json!({ "konum": kok, "ad": "deneme" }),
        ),
        cagir(2, "orhunca_dosyalar", json!({ "klasor": proje })),
        cagir(
            3,
            "orhunca_dosya_yaz",
            json!({ "dosya": proje.join("ana.ohc"), "icerik": "\"selam\"'ı yaz.\n" }),
        ),
        cagir(
            4,
            "orhunca_dosya_oku",
            json!({ "dosya": proje.join("ana.ohc") }),
        ),
        cagir(
            5,
            "orhunca_calistir",
            json!({ "dosya": proje.join("ana.ohc") }),
        ),
        cagir(
            6,
            "orhunca_dosya_yaz",
            json!({ "dosya": kok.join("kotu.exe"), "icerik": "x" }),
        ),
        cagir(
            7,
            "orhunca_dosya_oku",
            json!({ "dosya": proje.join("../deneme/ana.ohc") }),
        ),
        cagir(
            8,
            "orhunca_yeni_proje",
            json!({ "konum": kok, "ad": "b", "sablon": "yok" }),
        ),
        cagir(
            9,
            "orhunca_yeni_proje",
            json!({ "konum": kok, "ad": "deneme" }),
        ),
    ]);
    assert!(
        metni(&y[0]).contains("projesi oluşturuldu"),
        "{}",
        metni(&y[0])
    );
    assert!(metni(&y[1]).contains("deneme.ohcproj"), "{}", metni(&y[1]));
    assert!(metni(&y[1]).contains("ana.ohc"), "{}", metni(&y[1]));
    assert_eq!(y[2]["result"]["isError"], false);
    assert_eq!(metni(&y[3]), "\"selam\"'ı yaz.\n");
    assert!(metni(&y[4]).contains("selam"), "{}", metni(&y[4]));
    for i in [5, 6, 7, 8] {
        assert_eq!(y[i]["result"]["isError"], true, "{}", metni(&y[i]));
    }
    assert!(!kok.join("kotu.exe").exists());
    assert!(metni(&y[7]).contains("arayuz"), "{}", metni(&y[7]));
    std::fs::remove_dir_all(&kok).unwrap();
}

#[test]
fn mcp_arayuz() {
    let y = konus(&[cagir(
        1,
        "orhunca_arayuz",
        json!({ "kod": "durum sayaç = 0\ndurum ad = \"\"\narayüz:\n    yazı(\"Sayaç: \" + sayaç)\n    düğme(\"Artır\") tıklanınca:\n        sayaç += 1\n    giriş(ad)\n    yazı(\"Merhaba \" + ad)\n",
                "eylemler": [{ "tıkla": "Artır" }, { "tıkla": "Artır" }, { "yaz": { "değer": "Ali" } }, { "tıkla": "Yok" }] }),
    )]);
    let m = metni(&y[0]);
    if m.contains("Node.js gerekli") {
        eprintln!("Node.js bulunamadı: arayüz testi atlandı");
        return;
    }
    assert_eq!(y[0]["result"]["isError"], false, "{m}");
    assert!(m.starts_with("Ekran:\nyazı \"Sayaç: 0\""), "{m}");
    assert!(
        m.contains("> tıkla \"Artır\"\nEkran:\nyazı \"Sayaç: 1\""),
        "{m}"
    );
    assert!(m.contains("yazı \"Sayaç: 2\""), "{m}");
    assert!(m.contains("yazı \"Merhaba Ali\""), "{m}");
    assert!(m.contains("tıklanabilir bir öğe yok"), "{m}");

    // Arayüz olmayan program ve derleme hatası
    let y = konus(&[
        cagir(1, "orhunca_arayuz", json!({ "kod": "5'i yaz.\n" })),
        cagir(
            2,
            "orhunca_arayuz",
            json!({ "kod": "arayüz:\n    yazı(yok)\n" }),
        ),
    ]);
    assert_eq!(y[0]["result"]["isError"], true);
    assert!(metni(&y[0]).contains("arayüz programı değil"));
    assert!(
        metni(&y[1]).starts_with("DERLEME HATASI"),
        "{}",
        metni(&y[1])
    );
}
