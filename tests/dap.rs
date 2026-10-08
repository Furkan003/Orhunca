//! `orhunca ayıkla-dap`: hata ayıklama bağdaştırıcısı (Debug Adapter Protocol).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

fn gonder(girdi: &mut impl Write, m: Value) {
    let g = m.to_string();
    write!(girdi, "Content-Length: {}\r\n\r\n{g}", g.len()).unwrap();
    girdi.flush().unwrap();
}

fn okuyucu(cikti: impl Read + Send + 'static) -> Receiver<Value> {
    let (g, a) = channel();
    std::thread::spawn(move || {
        let mut r = BufReader::new(cikti);
        loop {
            let mut uzunluk = 0;
            loop {
                let mut s = String::new();
                if r.read_line(&mut s).unwrap_or(0) == 0 {
                    return;
                }
                let s = s.trim_end();
                if s.is_empty() {
                    break;
                }
                if let Some(d) = s.strip_prefix("Content-Length:") {
                    uzunluk = d.trim().parse().unwrap();
                }
            }
            let mut v = vec![0; uzunluk];
            r.read_exact(&mut v).unwrap();
            if g.send(serde_json::from_slice(&v).unwrap()).is_err() {
                return;
            }
        }
    });
    a
}

#[test]
fn dap_oturumu() {
    let k = std::env::temp_dir().join(format!("orhunca-dap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    let dosya = k.join("ana.ohc");
    std::fs::write(
        &dosya,
        "işlev kare(n: sayı) -> sayı:\n    döndür n * n\n\ntoplam = 0\nher i için 1'den 4'e kadar:\n    toplam += kare(i)\ntoplam'ı yaz.\n",
    )
    .unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg("ayıkla-dap")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut girdi = c.stdin.take().unwrap();
    let gelen = okuyucu(c.stdout.take().unwrap());
    let mut cikti = String::new();
    let bekle = |ne: &dyn Fn(&Value) -> bool, cikti: &mut String| -> Value {
        loop {
            let m = gelen
                .recv_timeout(Duration::from_secs(30))
                .expect("bağdaştırıcı yanıt vermedi");
            if m["event"] == "output" {
                cikti.push_str(m["body"]["output"].as_str().unwrap_or(""));
            }
            if ne(&m) {
                return m;
            }
        }
    };
    let mut seq = 0;
    let mut istek = |girdi: &mut std::process::ChildStdin, komut: &str, a: Value| {
        seq += 1;
        gonder(
            girdi,
            json!({ "seq": seq, "type": "request", "command": komut, "arguments": a }),
        );
        seq
    };
    let yanit = |s: i64| move |m: &Value| m["type"] == "response" && m["request_seq"] == s;

    let s = istek(&mut girdi, "initialize", json!({ "adapterID": "orhunca" }));
    let r = bekle(&yanit(s), &mut cikti);
    assert_eq!(r["body"]["supportsConditionalBreakpoints"], true, "{r}");
    bekle(&|m| m["event"] == "initialized", &mut cikti);
    let s = istek(&mut girdi, "launch", json!({ "program": dosya }));
    bekle(&yanit(s), &mut cikti);
    let s = istek(
        &mut girdi,
        "setBreakpoints",
        json!({ "source": { "path": dosya }, "breakpoints": [
            { "line": 2, "condition": "n == 3" },
            { "line": 6, "logMessage": "i={i}" },
        ] }),
    );
    let r = bekle(&yanit(s), &mut cikti);
    assert_eq!(r["body"]["breakpoints"][0]["verified"], true);
    let s = istek(&mut girdi, "configurationDone", json!({}));
    bekle(&yanit(s), &mut cikti);

    let d = bekle(
        &|m| m["event"] == "stopped" || m["event"] == "terminated",
        &mut cikti,
    );
    assert_eq!(d["body"]["reason"], "breakpoint", "{d} {cikti}");
    let s = istek(&mut girdi, "stackTrace", json!({ "threadId": 1 }));
    let r = bekle(&yanit(s), &mut cikti);
    assert_eq!(r["body"]["stackFrames"][0]["name"], "kare", "{r}");
    assert_eq!(r["body"]["stackFrames"][0]["line"], 2, "{r}");
    assert_eq!(r["body"]["stackFrames"][1]["line"], 6, "{r}");
    let s = istek(&mut girdi, "scopes", json!({ "frameId": 0 }));
    let r = bekle(&yanit(s), &mut cikti);
    let ref_ = r["body"]["scopes"][0]["variablesReference"].clone();
    let s = istek(
        &mut girdi,
        "variables",
        json!({ "variablesReference": ref_ }),
    );
    let r = bekle(&yanit(s), &mut cikti);
    assert!(
        r["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["name"] == "n" && v["value"] == "3"),
        "{r}"
    );
    // Çağıranın çerçevesi: çalışma zamanından istenir
    let s = istek(&mut girdi, "variables", json!({ "variablesReference": 2 }));
    let r = bekle(&yanit(s), &mut cikti);
    assert!(
        r["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["name"] == "toplam" && v["value"] == "5"),
        "{r}"
    );
    let s = istek(&mut girdi, "continue", json!({ "threadId": 1 }));
    bekle(&yanit(s), &mut cikti);
    let e = bekle(&|m| m["event"] == "exited", &mut cikti);
    assert_eq!(e["body"]["exitCode"], 0);
    bekle(&|m| m["event"] == "terminated", &mut cikti);
    assert!(
        cikti.contains("◆ i=1") && cikti.contains("◆ i=4"),
        "{cikti}"
    );
    assert!(cikti.contains("30"), "{cikti}");
    let s = istek(&mut girdi, "disconnect", json!({}));
    bekle(&yanit(s), &mut cikti);
    assert!(c.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&k);
}
