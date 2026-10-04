//! Stüdyo ayarları: son açılan projeler ve son kullanılan şablonlar.
//! Linux: ~/.config/orhunca/studyo.json · Windows: %APPDATA%\Orhunca\studyo.json

use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn ev_klasoru() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn ayar_dosyasi() -> PathBuf {
    let kok = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(ev_klasoru)
            .join("Orhunca")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| ev_klasoru().join(".config"))
            .join("orhunca")
    };
    kok.join("studyo.json")
}

pub fn simdi() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn oku() -> Value {
    std::fs::read_to_string(ayar_dosyasi())
        .ok()
        .and_then(|m| serde_json::from_str(&m).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({ "projeler": [], "son_sablonlar": [] }))
}

fn kaydet(deger: &Value) {
    let yol = ayar_dosyasi();
    if let Some(k) = yol.parent() {
        let _ = std::fs::create_dir_all(k);
    }
    let _ = std::fs::write(yol, serde_json::to_string_pretty(deger).unwrap_or_default());
}

/// Projeyi son açılanların başına taşır.
pub fn proje_acildi(ad: &str, yol: &str, sablon: &str) {
    let mut d = oku();
    let mut liste: Vec<Value> = d["projeler"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p["yol"].as_str() != Some(yol))
        .collect();
    liste.insert(
        0,
        json!({ "ad": ad, "yol": yol, "sablon": sablon, "tarih": simdi() }),
    );
    liste.truncate(30);
    d["projeler"] = Value::Array(liste);
    kaydet(&d);
}

pub fn proje_unut(yol: &str) {
    let mut d = oku();
    if let Some(l) = d["projeler"].as_array_mut() {
        l.retain(|p| p["yol"].as_str() != Some(yol));
    }
    kaydet(&d);
}

pub fn sablon_kullanildi(kimlik: &str) {
    let mut d = oku();
    let mut liste: Vec<Value> = d["son_sablonlar"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.as_str() != Some(kimlik))
        .collect();
    liste.insert(0, json!(kimlik));
    liste.truncate(3);
    d["son_sablonlar"] = Value::Array(liste);
    kaydet(&d);
}
