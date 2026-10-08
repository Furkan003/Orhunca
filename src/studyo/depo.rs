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
    ayar_klasoru().join("studyo.json")
}

/// Stüdyo'nun ayar klasörü (temalar da buradadır).
pub fn ayar_klasoru() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(ev_klasoru)
            .join("Orhunca")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| ev_klasoru().join(".config"))
            .join("orhunca")
    }
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
    // Geçiş: güvenilenler listesi, yeni proje son açılanlara eklenmeden önce kaydedilir.
    if d["guvenilen"].is_null() {
        d["guvenilen"] = json!(guvenilenler(&d));
    }
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

/// Güvenilen proje klasörleri (tam yollar). Güvenilmeyen bir projede Stüdyo kod çalıştırmaz,
/// derlemez, paket kurmaz ve asistanın kod çalıştıran araçlarını kapatır (kısıtlı mod).
/// Ayar dosyasında liste hiç yoksa (önceki sürümlerden geçiş) son açılan projeler güvenilir
/// sayılır: kullanıcı onları zaten çalıştırıyordu.
fn guvenilenler(d: &Value) -> Vec<String> {
    match d["guvenilen"].as_array() {
        Some(l) => l
            .iter()
            .filter_map(|y| y.as_str().map(str::to_string))
            .collect(),
        None => d["projeler"]
            .as_array()
            .map(|l| {
                l.iter()
                    .filter_map(|p| p["yol"].as_str())
                    .filter_map(|y| std::fs::canonicalize(y).ok())
                    .map(|y| y.to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Klasör (ya da içindeki dosya) güvenilen bir projenin içinde mi?
pub fn guvenilir_mi(yol: &std::path::Path) -> bool {
    let Ok(tam) = std::fs::canonicalize(yol) else {
        return false;
    };
    guvenilenler(&oku())
        .iter()
        .any(|g| tam.starts_with(std::path::Path::new(g)))
}

pub fn guven(yol: &std::path::Path, guvenilir: bool) {
    let Ok(tam) = std::fs::canonicalize(yol) else {
        return;
    };
    let tam = tam.to_string_lossy().into_owned();
    let mut d = oku();
    let mut l = guvenilenler(&d);
    l.retain(|g| *g != tam);
    if guvenilir {
        l.push(tam);
    }
    d["guvenilen"] = json!(l);
    kaydet(&d);
}
