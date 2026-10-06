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
            "orhunca_bicimlendir"
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
