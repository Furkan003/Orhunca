//! Stüdyo veritabanı görüntüleyicisi: projenin JSON kayıtlarını ve SQL veritabanını
//! (SQLite, PostgreSQL, MySQL, SQL Server) tablo olarak gösterir, SQL sorgusu çalıştırır.
//!
//! SQL veritabanlarına Stüdyo doğrudan bağlanmaz. Proje klasöründe küçük bir Orhunca
//! programı derlenip çalıştırılır; böylece bağlantı programla aynı yolu (`.env`,
//! `ORHUNCA_VERITABANI`, istemci kütüphaneleri) kullanır.

use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

use crate::derleme;

/// Projenin `.env` dosyasındaki (yoksa ortamdaki) `ORHUNCA_VERITABANI` değeri.
fn adres(kok: &Path) -> String {
    if let Ok(icerik) = std::fs::read_to_string(kok.join(".env")) {
        for satir in icerik.lines() {
            let s = satir.trim().trim_start_matches("export ").trim();
            if let Some(d) = s.strip_prefix("ORHUNCA_VERITABANI") {
                if let Some(d) = d.trim_start().strip_prefix('=') {
                    let d = d.split(" #").next().unwrap_or("").trim();
                    return d.trim_matches(|c| c == '"' || c == '\'').to_string();
                }
            }
        }
    }
    std::env::var("ORHUNCA_VERITABANI").unwrap_or_default()
}

/// "json", "sqlite", "postgresql", "mysql" ya da "sqlserver".
fn tur(adres: &str) -> &'static str {
    let a = adres.to_lowercase();
    if a.starts_with("postgres") {
        "postgresql"
    } else if a.starts_with("mysql") || a.starts_with("mariadb") {
        "mysql"
    } else if a.starts_with("sqlserver") || a.starts_with("mssql") {
        "sqlserver"
    } else if a.starts_with("sqlite") {
        "sqlite"
    } else {
        "json"
    }
}

fn tablo_sorgusu(tur: &str) -> &'static str {
    match tur {
        "postgresql" => {
            "SELECT table_name AS ad FROM information_schema.tables \
             WHERE table_schema = 'public' ORDER BY 1"
        }
        "mysql" => {
            "SELECT table_name AS ad FROM information_schema.tables \
             WHERE table_schema = DATABASE() ORDER BY 1"
        }
        "sqlserver" => "SELECT name AS ad FROM sys.tables ORDER BY 1",
        _ => "SELECT name AS ad FROM sqlite_master WHERE type = 'table' ORDER BY 1",
    }
}

fn veri_klasoru(kok: &Path) -> std::path::PathBuf {
    kok.join("veri")
}

/// Veritabanı türü, JSON kayıt dosyaları ve SQL tabloları.
pub fn ozet(kok: &Path) -> Result<Value, String> {
    let adres = adres(kok);
    let tur = tur(&adres);
    let mut json_tablolar = Vec::new();
    if let Ok(okunan) = std::fs::read_dir(veri_klasoru(kok)) {
        for g in okunan.flatten() {
            let ad = g.file_name().to_string_lossy().to_string();
            if let Some(model) = ad.strip_suffix(".json") {
                json_tablolar.push(model.to_string());
            }
        }
    }
    json_tablolar.sort();
    let sqlite_var = veri_klasoru(kok).join("orhunca.sqlite").exists();
    let (sql_tablolar, sql_hatasi) = if tur != "json" || sqlite_var {
        match sorgula(kok, tablo_sorgusu(tur)) {
            Ok(Value::Array(satirlar)) => (
                satirlar
                    .iter()
                    .filter_map(|s| s["ad"].as_str().map(str::to_string))
                    .collect(),
                Value::Null,
            ),
            Ok(_) => (Vec::new(), Value::Null),
            Err(h) => (Vec::new(), json!(h)),
        }
    } else {
        (Vec::new(), Value::Null)
    };
    Ok(json!({
        "tur": tur,
        "json": json_tablolar,
        "sql": sql_tablolar,
        "sql_hatasi": sql_hatasi,
        "sql_kullanilabilir": tur != "json" || sqlite_var,
    }))
}

/// `veri/<model>.json` içindeki kayıtlar (satır listesi).
pub fn json_kayitlari(kok: &Path, model: &str) -> Result<Value, String> {
    if model.is_empty() || model.contains(['/', '\\', '.']) {
        return Err("geçersiz model adı".into());
    }
    let yol = veri_klasoru(kok).join(format!("{model}.json"));
    let icerik = std::fs::read_to_string(&yol).map_err(|e| format!("{}: {e}", yol.display()))?;
    let deger: Value = serde_json::from_str(&icerik).map_err(|e| format!("JSON okunamadı: {e}"))?;
    Ok(match deger {
        Value::Array(_) => deger,
        Value::Object(ref o) => o.values().find(|d| d.is_array()).cloned().unwrap_or(deger),
        d => json!([d]),
    })
}

/// SQL sorgusunu projenin veritabanında çalıştırır; satırları JSON olarak döndürür.
/// SELECT dışındaki deyimler için `[{"etkilenen": n}]` döner.
pub fn sorgula(kok: &Path, sql: &str) -> Result<Value, String> {
    let sql = sql.trim().trim_end_matches(';');
    if sql.is_empty() {
        return Err("sorgu boş".into());
    }
    let gecici = derleme::gecici_klasor("vt")?;
    let kaynak = gecici.join("vt.ohc");
    let program = gecici.join(if cfg!(windows) { "vt.exe" } else { "vt" });
    let okuma = {
        let k = sql.trim_start().to_lowercase();
        [
            "select", "with", "show", "pragma", "explain", "describe", "values",
        ]
        .iter()
        .any(|a| k.starts_with(a))
    };
    let govde = if okuma {
        "json(sql_sorgu(a[uzunluk(a) - 1]))'yi yaz."
    } else {
        "sql_çalıştır(a[uzunluk(a) - 1])'yi yaz."
    };
    let sonuc = (|| {
        std::fs::write(&kaynak, format!("a = argümanlar()\n{govde}\n"))
            .map_err(|e| e.to_string())?;
        derleme::derle(&kaynak, &program, None).map_err(|h| h.metin)?;
        let cikti = Command::new(&program)
            .arg(sql)
            .current_dir(kok)
            .output()
            .map_err(|e| e.to_string())?;
        let ust = String::from_utf8_lossy(&cikti.stdout);
        if !cikti.status.success() {
            let alt = String::from_utf8_lossy(&cikti.stderr);
            return Err(format!("{}{}", alt.trim(), ust.trim()));
        }
        let deger: Value = serde_json::from_str(ust.trim()).map_err(|_| ust.trim().to_string())?;
        Ok(if okuma {
            deger
        } else {
            json!([{ "etkilenen": deger }])
        })
    })();
    let _ = std::fs::remove_dir_all(&gecici);
    sonuc
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn tur_adresten_anlasilir() {
        assert_eq!(tur(""), "json");
        assert_eq!(tur("sqlite"), "sqlite");
        assert_eq!(tur("postgresql://a@b/c"), "postgresql");
        assert_eq!(tur("mysql://a@b/c"), "mysql");
        assert_eq!(tur("sqlserver://b/c"), "sqlserver");
    }
}
