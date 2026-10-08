//! Dil sunucusunun (LSP) uçtan uca testi: gerçek JSON-RPC mesajlarıyla.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

struct Istemci {
    girdi: ChildStdin,
    cikti: BufReader<ChildStdout>,
    sira: u64,
}

impl Istemci {
    fn gonder(&mut self, m: Value) {
        let g = m.to_string();
        write!(self.girdi, "Content-Length: {}\r\n\r\n{g}", g.len()).unwrap();
        self.girdi.flush().unwrap();
    }

    fn oku(&mut self) -> Value {
        let mut uzunluk = 0;
        loop {
            let mut s = String::new();
            self.cikti.read_line(&mut s).unwrap();
            let s = s.trim_end();
            if s.is_empty() {
                break;
            }
            if let Some(d) = s.strip_prefix("Content-Length:") {
                uzunluk = d.trim().parse().unwrap();
            }
        }
        let mut b = vec![0; uzunluk];
        self.cikti.read_exact(&mut b).unwrap();
        serde_json::from_slice(&b).unwrap()
    }

    fn istek(&mut self, yontem: &str, p: Value) -> Value {
        self.sira += 1;
        let id = self.sira;
        self.gonder(json!({ "jsonrpc": "2.0", "id": id, "method": yontem, "params": p }));
        loop {
            let m = self.oku();
            if m["id"] == id {
                return m["result"].clone();
            }
        }
    }

    fn bildirim_bekle(&mut self, yontem: &str, uri: &str) -> Value {
        loop {
            let m = self.oku();
            if m["method"] == yontem && m["params"]["uri"] == uri {
                return m["params"].clone();
            }
        }
    }
}

#[test]
fn dil_sunucusu() {
    let klasor = std::env::temp_dir().join(format!("orhunca-lsp-{}", std::process::id()));
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join("ana.ohc");
    let metin = "sayılar = [3, 1]\n5'a sayılara ekle.\nsayıları sırala.\nuzunluk(sayılar)'ı yaz.\neğer sayılar 2'e büyükse:\n    1'i yaz.\n";
    std::fs::write(&dosya, metin).unwrap();
    let uri = format!("file://{}", dosya.display());

    let mut cocuk = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg("dil-sunucusu")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut i = Istemci {
        girdi: cocuk.stdin.take().unwrap(),
        cikti: BufReader::new(cocuk.stdout.take().unwrap()),
        sira: 0,
    };
    let r = i.istek("initialize", json!({ "capabilities": {} }));
    assert_eq!(r["capabilities"]["hoverProvider"], true);
    i.gonder(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
    i.gonder(
        json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": { "uri": uri, "languageId": "orhunca", "version": 1, "text": metin } } }),
    );

    // Tanılar: bir derleme hatası ve bir ünlü uyumu uyarısı
    let d = i.bildirim_bekle("textDocument/publishDiagnostics", &uri);
    let tanilar = d["diagnostics"].as_array().unwrap();
    assert!(
        tanilar
            .iter()
            .any(|t| t["severity"] == 2 && t["message"].as_str().unwrap().contains("5'e")),
        "{d}"
    );
    assert!(
        tanilar
            .iter()
            .any(|t| t["severity"] == 1 && t["range"]["start"]["line"] == 1),
        "{d}"
    );

    // Hızlı düzelt: yanlış yazılmış değişken adı için quick fix
    let yazim = "toplam = 5\ntoplm'u yaz.\n";
    i.gonder(
        json!({ "jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": { "uri": uri, "version": 2 }, "contentChanges": [{ "text": yazim }] } }),
    );
    let d = i.bildirim_bekle("textDocument/publishDiagnostics", &uri);
    let tani = d["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["data"]["duzeltme"].is_object())
        .cloned()
        .unwrap_or_else(|| panic!("{d}"));
    let e = i.istek(
        "textDocument/codeAction",
        json!({ "textDocument": { "uri": uri }, "range": tani["range"],
            "context": { "diagnostics": [tani] } }),
    );
    assert_eq!(e[0]["kind"], "quickfix", "{e}");
    let degisiklik = &e[0]["edit"]["changes"][&uri][0];
    assert_eq!(degisiklik["newText"], "toplam", "{e}");
    assert_eq!(
        degisiklik["range"]["start"],
        json!({ "line": 1, "character": 0 }),
        "{e}"
    );
    assert_eq!(
        degisiklik["range"]["end"],
        json!({ "line": 1, "character": 5 }),
        "{e}"
    );

    // Düzeltilmiş metin: hata kalmaz
    let duzgun =
        "sayılar = [3, 1]\n5'i sayılara ekle.\nsayıları sırala.\nuzunluk(sayılar)'ı yaz.\n";
    i.gonder(
        json!({ "jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": { "uri": uri, "version": 3 }, "contentChanges": [{ "text": duzgun }] } }),
    );
    let d = i.bildirim_bekle("textDocument/publishDiagnostics", &uri);
    assert_eq!(d["diagnostics"], json!([]), "{d}");

    // Üzerine gelince: yerleşik işlev ve değişken tipi
    let h = i.istek(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 3, "character": 3 } }),
    );
    assert!(
        h["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("uzunluk("),
        "{h}"
    );
    let h = i.istek(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 0, "character": 2 } }),
    );
    assert!(
        h["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("liste<sayı>"),
        "{h}"
    );

    // Tamamlama ve tanıma git
    let t = i.istek(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 0, "character": 0 } }),
    );
    let etiketler: Vec<&str> = t["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["label"].as_str())
        .collect();
    assert!(
        etiketler.contains(&"eğer")
            && etiketler.contains(&"büyük_harf")
            && etiketler.contains(&"sayılar")
    );
    let g = i.istek(
        "textDocument/definition",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 3, "character": 10 } }),
    );
    assert_eq!(g["range"]["start"]["line"], 0, "{g}");

    // Başvurular ve yeniden adlandırma (bitişik ek: sayılara → notlara)
    let r = i.istek(
        "textDocument/references",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 0, "character": 2 },
            "context": { "includeDeclaration": true } }),
    );
    let satirlar: Vec<u64> = r
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["range"]["start"]["line"].as_u64().unwrap())
        .collect();
    assert_eq!(satirlar, vec![0, 1, 2, 3], "{r}");
    let h = i.istek(
        "textDocument/prepareRename",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 1, "character": 6 } }),
    );
    assert_eq!(h["placeholder"], "sayılar", "{h}");
    let a = i.istek(
        "textDocument/rename",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 0, "character": 2 }, "newName": "notlar" }),
    );
    let d = a["changes"][&uri].as_array().unwrap();
    assert!(d.iter().any(|x| x["newText"] == "notlara"), "{a}");
    assert_eq!(d.len(), 4, "{a}");
    let a = i.istek(
        "textDocument/rename",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 0, "character": 2 }, "newName": "eğer" }),
    );
    assert!(
        a.is_null(),
        "ayrılmış kelimeye adlandırma reddedilmeli: {a}"
    );
    let w = i.istek("workspace/symbol", json!({ "query": "" }));
    assert!(w.is_array(), "{w}");

    // Biçimlendirme
    i.gonder(json!({ "jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": { "uri": uri, "version": 4 }, "contentChanges": [{ "text": "eğer doğru ise:\n  6'i yaz.\n" }] } }));
    let b = i.istek("textDocument/formatting", json!({ "textDocument": { "uri": uri }, "options": { "tabSize": 4, "insertSpaces": true } }));
    assert_eq!(b[0]["newText"], "eğer doğru ise:\n    6'yı yaz.\n");

    assert_eq!(i.istek("shutdown", Value::Null), Value::Null);
    i.gonder(json!({ "jsonrpc": "2.0", "method": "exit" }));
    assert!(cocuk.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&klasor);
}
