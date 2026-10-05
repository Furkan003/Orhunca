//! Stüdyo'nun yapay zekâ asistanı: kullanıcının kendi Anthropic API anahtarıyla
//! Claude'a bağlanır. Asistan kodu kendi denetler ve çalıştırır (araçlar), dosya
//! değişikliğini öneri olarak verir; değişikliği kullanıcı "Uygula" ile yazar.
//!
//! Model kodda sabit değildir: kullanıcı, API'nin model listesinden seçer.
//! Okullar `ORHUNCA_YAPAY_ZEKA=kapali` ile asistanı kapatabilir.

use super::depo;
use crate::ajan;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const SISTEM: &str = "\
Sen Orhunca Stüdyo'nun içindeki kodlama asistanısın. Orhunca, Türkçe dil bilgisine dayanan \
bir programlama dilidir; kullanıcıların çoğu programlamayı yeni öğrenen öğrencilerdir. \
Kullanıcıyla Türkçe konuş; açıklamaların kısa, sade ve öğretici olsun.

Kod yazdığında önce kodu_denetle ile denetle, mümkünse kodu_calistir ile çalıştırıp çıktısına \
bak; hata varsa düzelt. Kullanıcının açık dosyasını değiştirmen gerektiğinde dosyanın yeni \
içeriğinin tamamını dosyayi_degistir ile öner; değişikliği kullanıcı onaylayınca dosyaya \
yazılır. Kullanıcı yalnızca bir soru sorduysa dosyayı değiştirme; kısa örneklerle yanıtla. \
Öğrenci bir alıştırmayı kendisi çözmeye çalışıyorsa çözümü hemen verme; ipucuyla yol göster.

Aşağıda Orhunca'nın dil rehberi var.";

/// Bir istekte en fazla bu kadar araç turu yapılır.
const EN_COK_TUR: usize = 12;

pub fn kapali_mi() -> bool {
    std::env::var("ORHUNCA_YAPAY_ZEKA").is_ok_and(|d| {
        matches!(
            d.to_lowercase().as_str(),
            "kapali" | "kapalı" | "0" | "hayir" | "hayır" | "off"
        )
    })
}

fn ayar_dosyasi() -> PathBuf {
    depo::ayar_klasoru().join("asistan.json")
}

fn ayarlar() -> Value {
    std::fs::read(ayar_dosyasi())
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or_else(|| json!({}))
}

fn anahtar() -> Option<String> {
    ayarlar()["anahtar"]
        .as_str()
        .filter(|a| !a.is_empty())
        .map(String::from)
}

fn adres() -> String {
    std::env::var("ORHUNCA_ASISTAN_ADRESI")
        .unwrap_or_else(|_| "https://api.anthropic.com".into())
        .trim_end_matches('/')
        .to_string()
}

/// Arayüze gösterilen durum; anahtarın kendisi hiçbir zaman gönderilmez.
pub fn durum() -> Value {
    if kapali_mi() {
        return json!({ "kapali": true });
    }
    let a = ayarlar();
    let anahtar = a["anahtar"].as_str().unwrap_or("");
    let son: String = anahtar
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    json!({
        "kapali": false,
        "anahtar_var": !anahtar.is_empty(),
        "anahtar_sonu": if anahtar.len() > 8 { son } else { String::new() },
        "model": a["model"],
    })
}

/// Anahtarı ve/veya modeli kaydeder. Boş anahtar, kayıtlı anahtarı siler.
pub fn ayar_kaydet(g: &Value) -> Result<Value, String> {
    if kapali_mi() {
        return Err("Yapay zekâ asistanı yönetici tarafından kapatılmış.".into());
    }
    let mut a = ayarlar();
    if let Some(k) = g["anahtar"].as_str() {
        let k = k.trim();
        if k.chars().any(|c| c.is_control() || c.is_whitespace()) || k.len() > 400 {
            return Err("API anahtarı geçersiz görünüyor.".into());
        }
        a["anahtar"] = json!(k);
    }
    if let Some(m) = g["model"].as_str() {
        if m.len() > 200
            || !m
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.:@".contains(c))
        {
            return Err("Model adı geçersiz.".into());
        }
        a["model"] = json!(m);
    }
    let klasor = depo::ayar_klasoru();
    std::fs::create_dir_all(&klasor).map_err(|e| e.to_string())?;
    let yol = ayar_dosyasi();
    std::fs::write(&yol, serde_json::to_vec_pretty(&a).unwrap()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o600));
    }
    Ok(durum())
}

/// Anthropic API'sine curl ile istek. Anahtar komut satırında görünmesin diye
/// başlık standart girdiden verilir.
fn istek(yol: &str, anahtar: &str, govde: Option<&Value>) -> Result<Value, String> {
    let gecici = crate::derleme::gecici_klasor("asistan")?;
    let mut komut = Command::new("curl");
    komut
        .args(["-sS", "--max-time", "600", "-w", "\n%{http_code}"])
        .args(["-H", "content-type: application/json"])
        .args(["-H", "anthropic-version: 2023-06-01"])
        .args(["-H", "@-"]);
    if let Some(g) = govde {
        let dosya = gecici.join("istek.json");
        std::fs::write(&dosya, g.to_string()).map_err(|e| e.to_string())?;
        komut
            .arg("--data-binary")
            .arg(format!("@{}", dosya.display()));
    }
    komut
        .arg(format!("{}{yol}", adres()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let sonuc = (|| {
        let mut c = komut
            .spawn()
            .map_err(|e| format!("curl çalıştırılamadı ({e}); curl kurulu olmalı"))?;
        if let Some(mut g) = c.stdin.take() {
            let _ = writeln!(g, "x-api-key: {anahtar}");
        }
        let c = c.wait_with_output().map_err(|e| e.to_string())?;
        if !c.status.success() {
            return Err(format!(
                "İnternete bağlanılamadı: {}",
                String::from_utf8_lossy(&c.stderr).trim()
            ));
        }
        let metin = String::from_utf8_lossy(&c.stdout).into_owned();
        let (govde, kod) = metin.rsplit_once('\n').unwrap_or(("", &metin));
        let kod: u16 = kod.trim().parse().unwrap_or(0);
        let v: Value = serde_json::from_str(govde).unwrap_or(Value::Null);
        if kod == 200 {
            return Ok(v);
        }
        let mesaj = v["error"]["message"].as_str().unwrap_or("").to_string();
        Err(match kod {
            401 => "API anahtarı geçersiz. Ayarlardan anahtarı denetleyin.".into(),
            403 => format!("Bu anahtarın izni yok: {mesaj}"),
            429 => "İstek sınırına ulaşıldı; biraz bekleyip yeniden deneyin.".into(),
            529 | 503 => "Sunucu şu an yoğun; biraz sonra yeniden deneyin.".into(),
            400 => format!("İSTEK:{mesaj}"),
            _ => format!("API hatası ({kod}): {mesaj}"),
        })
    })();
    let _ = std::fs::remove_dir_all(&gecici);
    sonuc
}

/// Hesabın kullanabildiği modeller (en yeniden eskiye).
pub fn modeller() -> Result<Value, String> {
    if kapali_mi() {
        return Err("Yapay zekâ asistanı yönetici tarafından kapatılmış.".into());
    }
    let anahtar = anahtar().ok_or("Önce bir API anahtarı girin.")?;
    let v = istek("/v1/models?limit=100", &anahtar, None).map_err(temiz)?;
    let liste: Vec<Value> = v["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            Some(json!({
                "kimlik": m["id"].as_str()?,
                "ad": m["display_name"].as_str().or(m["id"].as_str())?,
            }))
        })
        .collect();
    Ok(json!({ "modeller": liste }))
}

fn temiz(e: String) -> String {
    e.strip_prefix("İSTEK:")
        .map(|m| format!("İstek reddedildi: {m}"))
        .unwrap_or(e)
}

fn araclar() -> Value {
    json!([
        {
            "name": "kodu_denetle",
            "description": "Orhunca kodunu çalıştırmadan derler. Türkçe hata mesajlarını (satır, sütun, ipucu) ya da 'Hata yok.' döndürür.",
            "input_schema": {
                "type": "object",
                "properties": { "kod": { "type": "string", "description": "Orhunca kaynak kodu" } },
                "required": ["kod"]
            }
        },
        {
            "name": "kodu_calistir",
            "description": "Orhunca programını derleyip en fazla 10 saniye çalıştırır ve çıktısını döndürür. Arayüz programları yalnızca derlenir.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "kod": { "type": "string", "description": "Orhunca kaynak kodu" },
                    "girdi": { "type": "string", "description": "Programın oku() ile okuyacağı girdi; satırlar \\n ile ayrılır" }
                },
                "required": ["kod"]
            }
        },
        {
            "name": "dosyayi_degistir",
            "description": "Kullanıcının açık dosyası için yeni içerik önerir. Dosyanın yeni içeriğinin TAMAMINI verin. Kullanıcı öneriyi görür ve onaylarsa dosyaya yazılır.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "icerik": { "type": "string", "description": "Dosyanın yeni içeriğinin tamamı" },
                    "aciklama": { "type": "string", "description": "Değişikliğin bir cümlelik açıklaması" }
                },
                "required": ["icerik"]
            }
        }
    ])
}

/// Bir araç çağrısını yürütür; (sonuç metni, hata mı).
fn arac_yurut(ad: &str, g: &Value, oneri: &mut Option<Value>) -> (String, bool) {
    let kod = g["kod"].as_str();
    match ad {
        "kodu_denetle" if kod.is_some() => match ajan::denetle(kod, None) {
            Ok(m) => (m, false),
            Err(e) => (e, true),
        },
        "kodu_calistir" if kod.is_some() => match ajan::calistir(
            kod,
            None,
            g["girdi"].as_str().unwrap_or(""),
            Duration::from_secs(10),
        ) {
            Ok(c) => (c.metin(), false),
            Err(e) => (e, true),
        },
        "dosyayi_degistir" => match g["icerik"].as_str() {
            Some(i) => {
                *oneri =
                    Some(json!({ "icerik": i, "aciklama": g["aciklama"].as_str().unwrap_or("") }));
                (
                    "Öneri kullanıcıya gösterildi; kullanıcı onaylarsa dosyaya yazılacak.".into(),
                    false,
                )
            }
            None => ("'icerik' verilmeli".into(), true),
        },
        _ => (format!("bilinmeyen araç ya da eksik girdi: {ad}"), true),
    }
}

/// Konuşmayı sürdürür: `mesajlar` [{rol: kullanici|asistan, metin}], `dosya` ve
/// `icerik` kullanıcının açık dosyası. Asistanın yanıtını, kullandığı araçları ve
/// varsa dosya önerisini döndürür.
pub fn sor(g: &Value) -> Result<Value, String> {
    if kapali_mi() {
        return Err("Yapay zekâ asistanı yönetici tarafından kapatılmış.".into());
    }
    let anahtar = anahtar().ok_or("Önce Ayarlar'dan bir API anahtarı girin.")?;
    let model = ayarlar()["model"]
        .as_str()
        .filter(|m| !m.is_empty())
        .map(String::from)
        .ok_or("Önce bir model seçin.")?;
    let gelen = g["mesajlar"].as_array().cloned().unwrap_or_default();
    if gelen.is_empty() || gelen.len() > 200 {
        return Err("Geçersiz konuşma.".into());
    }
    let mut mesajlar: Vec<Value> = Vec::new();
    for (i, m) in gelen.iter().enumerate() {
        let rol = if m["rol"] == "asistan" {
            "assistant"
        } else {
            "user"
        };
        let mut metin = m["metin"].as_str().unwrap_or("").to_string();
        if metin.trim().is_empty() {
            continue;
        }
        // Açık dosya yalnızca son mesaja eklenir (önceki tur önbellekte kalır).
        if i == gelen.len() - 1 && rol == "user" {
            if let Some(icerik) = g["icerik"].as_str() {
                let ad = g["dosya"].as_str().unwrap_or("dosya.ohc");
                metin = format!("<acik_dosya ad=\"{ad}\">\n{icerik}\n</acik_dosya>\n\n{metin}");
            }
        }
        // Ardışık aynı roller birleştirilir (API dönüşümlü roller ister).
        match mesajlar.last_mut() {
            Some(son) if son["role"] == rol => {
                let onceki = son["content"].as_str().unwrap_or("").to_string();
                son["content"] = json!(format!("{onceki}\n\n{metin}"));
            }
            _ => mesajlar.push(json!({ "role": rol, "content": metin })),
        }
    }
    if mesajlar
        .first()
        .map(|m| m["role"] != "user")
        .unwrap_or(true)
        || mesajlar.last().map(|m| m["role"] != "user").unwrap_or(true)
    {
        return Err("Konuşma kullanıcı mesajıyla başlayıp bitmeli.".into());
    }

    let sistem = json!([{
        "type": "text",
        "text": format!("{SISTEM}\n\n{}", ajan::rehber()),
        "cache_control": { "type": "ephemeral" }
    }]);
    let mut uyumlu = false;
    let mut metinler: Vec<String> = Vec::new();
    let mut adimlar: Vec<Value> = Vec::new();
    let mut oneri: Option<Value> = None;
    let mut durma = String::new();
    for _ in 0..EN_COK_TUR {
        let mut govde = json!({
            "model": model,
            "max_tokens": 16000,
            "system": sistem,
            "tools": araclar(),
            "messages": mesajlar,
        });
        if !uyumlu {
            govde["thinking"] = json!({ "type": "adaptive" });
            govde["output_config"] = json!({ "effort": "medium" });
        }
        let yanit = match istek("/v1/messages", &anahtar, Some(&govde)) {
            Ok(y) => y,
            // Eski modeller uyarlanır düşünmeyi ya da çaba ayarını tanımayabilir:
            // bu alanlar olmadan yeniden denenir.
            Err(e) if e.starts_with("İSTEK:") && !uyumlu => {
                uyumlu = true;
                let mut govde = govde;
                if let Some(o) = govde.as_object_mut() {
                    o.remove("thinking");
                    o.remove("output_config");
                }
                istek("/v1/messages", &anahtar, Some(&govde)).map_err(temiz)?
            }
            Err(e) => return Err(temiz(e)),
        };
        durma = yanit["stop_reason"].as_str().unwrap_or("").to_string();
        if durma == "refusal" {
            metinler.push(
                "Asistan bu isteği yanıtlamadı. Sorunuzu farklı bir biçimde sormayı deneyin."
                    .into(),
            );
            break;
        }
        let icerik = yanit["content"].as_array().cloned().unwrap_or_default();
        let mut sonuclar: Vec<Value> = Vec::new();
        for b in &icerik {
            match b["type"].as_str() {
                Some("text") => {
                    if let Some(t) = b["text"].as_str().filter(|t| !t.trim().is_empty()) {
                        metinler.push(t.to_string());
                    }
                }
                Some("tool_use") => {
                    let ad = b["name"].as_str().unwrap_or("");
                    let (sonuc, hatali) = arac_yurut(ad, &b["input"], &mut oneri);
                    adimlar.push(
                        json!({ "ad": ad, "sonuc": sonuc.chars().take(2000).collect::<String>() }),
                    );
                    sonuclar.push(json!({
                        "type": "tool_result",
                        "tool_use_id": b["id"],
                        "content": sonuc,
                        "is_error": hatali,
                    }));
                }
                _ => {}
            }
        }
        if durma != "tool_use" || sonuclar.is_empty() {
            break;
        }
        mesajlar.push(json!({ "role": "assistant", "content": icerik }));
        mesajlar.push(json!({ "role": "user", "content": sonuclar }));
    }
    if durma == "max_tokens" {
        metinler.push("(Yanıt uzunluk sınırında kesildi.)".into());
    }
    Ok(json!({
        "yanit": metinler.join("\n\n"),
        "adimlar": adimlar,
        "oneri": oneri,
    }))
}
