//! Stüdyo temaları: kullanıcının kaydettiği temalar ayar klasöründeki `temalar/`
//! altında `.ohctema` (JSON) dosyalarıdır. Topluluk temaları depodaki `temalar/`
//! klasöründen (temalar/dizin.json) indirilir.

use super::depo;
use serde_json::{json, Value};
use std::path::PathBuf;

const EN_BUYUK: usize = 30 * 1024 * 1024;
pub const GALERI: &str = "https://raw.githubusercontent.com/Furkan003/Orhunca/HEAD/temalar/";

fn klasor() -> PathBuf {
    depo::ayar_klasoru().join("temalar")
}

fn gecerli_kimlik(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 80
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Tema dosyası mı? (Ayrıntılı doğrulama arayüzde yapılır; burada yapı ve boyut.)
fn denetle(t: &Value) -> Result<(), String> {
    if t["orhunca_tema"] != json!(1) {
        return Err("bu bir Orhunca tema dosyası değil".into());
    }
    if t.to_string().len() > EN_BUYUK {
        return Err("tema çok büyük (en çok 30 MB; arka plan resmini küçültün)".into());
    }
    Ok(())
}

fn kimlik_uret(ad: &str) -> String {
    let mut k: String = ad
        .chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'İ' | 'I' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    k.truncate(48);
    if k.is_empty() {
        k = "tema".into();
    }
    format!("{k}-{}", depo::simdi() % 1_000_000)
}

/// Kayıtlı temalar (arka plan resmi olmadan; resim `tema` ile alınır).
pub fn liste() -> Value {
    let mut l = Vec::new();
    if let Ok(d) = std::fs::read_dir(klasor()) {
        for g in d.flatten() {
            let yol = g.path();
            if yol.extension().is_none_or(|u| u != "ohctema") {
                continue;
            }
            let Ok(m) = std::fs::read_to_string(&yol) else {
                continue;
            };
            let Ok(mut t) = serde_json::from_str::<Value>(&m) else {
                continue;
            };
            let resimli = t["arka_plan"]["kaynak"].is_string();
            if let Some(a) = t.get_mut("arka_plan").and_then(|a| a.as_object_mut()) {
                a.remove("kaynak");
            }
            l.push(json!({
                "kimlik": yol.file_stem().map(|s| s.to_string_lossy().into_owned()),
                "tema": t,
                "resimli": resimli,
            }));
        }
    }
    json!({ "temalar": l })
}

pub fn oku(kimlik: &str) -> Result<Value, String> {
    if !gecerli_kimlik(kimlik) {
        return Err("geçersiz tema".into());
    }
    let m = std::fs::read_to_string(klasor().join(format!("{kimlik}.ohctema")))
        .map_err(|_| "tema bulunamadı".to_string())?;
    serde_json::from_str(&m).map_err(|e| e.to_string())
}

/// Kaydeder; kimlik verilmezse yeni tema olarak. Kimliği döndürür.
pub fn kaydet(kimlik: Option<&str>, t: &Value) -> Result<String, String> {
    denetle(t)?;
    let kimlik = match kimlik.filter(|k| gecerli_kimlik(k)) {
        Some(k) => k.to_string(),
        None => kimlik_uret(t["ad"].as_str().unwrap_or("tema")),
    };
    std::fs::create_dir_all(klasor()).map_err(|e| e.to_string())?;
    std::fs::write(klasor().join(format!("{kimlik}.ohctema")), t.to_string())
        .map_err(|e| e.to_string())?;
    Ok(kimlik)
}

pub fn sil(kimlik: &str) -> Result<(), String> {
    if !gecerli_kimlik(kimlik) {
        return Err("geçersiz tema".into());
    }
    std::fs::remove_file(klasor().join(format!("{kimlik}.ohctema"))).map_err(|e| e.to_string())
}

/// Masaüstü uygulamasında indirme olmadığı için dışa aktarılan tema belgelere yazılır.
pub fn disa_aktar(t: &Value) -> Result<String, String> {
    denetle(t)?;
    let ad: String = t["ad"]
        .as_str()
        .unwrap_or("tema")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .take(60)
        .collect();
    let ad = if ad.trim().is_empty() {
        "tema".to_string()
    } else {
        ad.trim().to_string()
    };
    let hedef_klasor = depo::ev_klasoru().join("Orhunca").join("Temalar");
    std::fs::create_dir_all(&hedef_klasor).map_err(|e| e.to_string())?;
    let mut yol = hedef_klasor.join(format!("{ad}.ohctema"));
    let mut i = 2;
    while yol.exists() {
        yol = hedef_klasor.join(format!("{ad} ({i}).ohctema"));
        i += 1;
    }
    let guzel = serde_json::to_string_pretty(t).map_err(|e| e.to_string())?;
    std::fs::write(&yol, guzel).map_err(|e| e.to_string())?;
    Ok(yol.to_string_lossy().into_owned())
}

fn indir(adres: &str) -> Result<Vec<u8>, String> {
    let c = crate::komut("curl")
        .args([
            "-sSfL",
            "--max-time",
            "60",
            "--proto",
            "=https,http",
            "--",
            adres,
        ])
        .output()
        .map_err(|_| "curl bulunamadı".to_string())?;
    if !c.status.success() {
        return Err(format!(
            "indirilemedi: {}",
            String::from_utf8_lossy(&c.stderr).trim()
        ));
    }
    Ok(c.stdout)
}

fn galeri_adresi() -> String {
    std::env::var("ORHUNCA_TEMA_GALERISI").unwrap_or_else(|_| GALERI.into())
}

/// Topluluk galerisi: temalar/dizin.json
pub fn galeri() -> Result<Value, String> {
    let kok = galeri_adresi();
    let veri = if kok.starts_with("http") {
        indir(&format!("{kok}dizin.json"))?
    } else {
        std::fs::read(PathBuf::from(&kok).join("dizin.json")).map_err(|e| e.to_string())?
    };
    serde_json::from_slice(&veri).map_err(|e| format!("galeri bozuk: {e}"))
}

/// Galeriden bir temayı indirir (yalnızca `ad.ohctema` biçimindeki dosya adları).
pub fn galeriden(dosya: &str) -> Result<Value, String> {
    let gecerli = dosya.ends_with(".ohctema")
        && !dosya.contains("..")
        && dosya
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !gecerli {
        return Err("geçersiz tema dosyası".into());
    }
    let kok = galeri_adresi();
    let veri = if kok.starts_with("http") {
        indir(&format!("{kok}{dosya}"))?
    } else {
        std::fs::read(PathBuf::from(&kok).join(dosya)).map_err(|e| e.to_string())?
    };
    let t: Value = serde_json::from_slice(&veri).map_err(|e| format!("tema bozuk: {e}"))?;
    denetle(&t)?;
    Ok(t)
}
