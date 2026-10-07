//! `orhunca mcp`: Model Context Protocol sunucusu (stdin/stdout, satır başına bir
//! JSON-RPC iletisi). Claude Code, Claude Desktop, Cursor gibi yapay zekâ
//! araçları bu sunucuya bağlanıp Orhunca kodu yazabilir, denetleyebilir ve
//! çalıştırabilir. Bkz. docs/yapay-zeka.md.

use crate::ajan;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::Duration;

const PROTOKOL: &str = "2025-06-18";
const REHBER_ADRESI: &str = "orhunca://rehber";

const TALIMAT: &str = "Orhunca, Türkçe dil bilgisine dayanan bir programlama dilidir (.ohc). \
Kod yazmadan önce orhunca_rehber aracıyla kısa rehberi okuyun; emin olmadığınız konuda \
aynı araçla ilgili bölümü (bolum) okuyun. Yazdığınız her kodu \
orhunca_denetle ile denetleyin, orhunca_calistir ile çalıştırıp çıktısına bakın. \
Arayüz programlarını orhunca_arayuz ile çalıştırıp düğmelere tıklayarak deneyin.";

fn araclar() -> Value {
    let kod_ya_da_dosya = |ek: Value| {
        let mut ozellikler = json!({
            "kod": { "type": "string", "description": "Orhunca kaynak kodu (.ohc dosyasının içeriği)" },
            "dosya": { "type": "string", "description": "Kod yerine diskteki bir .ohc dosyasının tam yolu (çok dosyalı projeler için)" }
        });
        if let (Some(o), Some(e)) = (ozellikler.as_object_mut(), ek.as_object()) {
            o.extend(e.clone());
        }
        json!({ "type": "object", "properties": ozellikler })
    };
    json!([
        {
            "name": "orhunca_rehber",
            "title": "Orhunca dil rehberi",
            "description": "Orhunca rehberini döndürür. Bölüm verilmezse dilin özünü ve bölüm adlarını içeren kısa rehberi verir (Orhunca kodu yazmadan önce bir kez okuyun). 'bolum' ile tek bir bölüm (ör. 'Standart kütüphane', 'Modeller', 'Oyunlar'), 'hepsi' ile tam rehber alınır.",
            "inputSchema": { "type": "object", "properties": {
                "bolum": { "type": "string", "description": "İsteğe bağlı bölüm adı ya da 'hepsi'" }
            } },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "orhunca_denetle",
            "title": "Orhunca kodunu denetle",
            "description": "Orhunca kodunu çalıştırmadan derler; Türkçe hata mesajlarını (satır, sütun ve çoğu zaman bir ipucuyla) ya da 'Hata yok.' döndürür. Her kod yazdığınızda kullanın.",
            "inputSchema": kod_ya_da_dosya(json!({})),
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "orhunca_calistir",
            "title": "Orhunca kodunu çalıştır",
            "description": "Orhunca programını derleyip çalıştırır ve çıktısını döndürür. Program süre sınırında (varsayılan 10 saniye) bitmezse durdurulur. Arayüz programları yalnızca derlenir.",
            "inputSchema": kod_ya_da_dosya(json!({
                "girdi": { "type": "string", "description": "Programın oku() ile okuyacağı standart girdi (satırlar \\n ile ayrılır)" },
                "sure": { "type": "number", "description": "Saniye cinsinden süre sınırı (1-60, varsayılan 10)" }
            }))
        },
        {
            "name": "orhunca_bicimlendir",
            "title": "Orhunca kodunu biçimlendir",
            "description": "Orhunca kodunu standart biçime (girinti, boşluklar) getirir ve biçimlendirilmiş kodu döndürür.",
            "inputSchema": {
                "type": "object",
                "properties": { "kod": { "type": "string", "description": "Orhunca kaynak kodu" } },
                "required": ["kod"]
            },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "orhunca_arayuz",
            "title": "Arayüz programını dene",
            "description": "Arayüz (arayüz: bloğu olan) programını tarayıcısız çalıştırır ve ekranı metin olarak döndürür (öğe türü, metni, değeri, olayları). 'eylemler' sırayla uygulanır ve her eylemden sonra ekran yeniden yazılır: {\"tıkla\": \"Düğme metni\"}, {\"yaz\": {\"tür\": \"giriş\", \"sıra\": 0, \"değer\": \"Ali\"}}, {\"gönder\": {}} (girişte Enter), {\"kare\": 30} (oyun alanında 30 kare). Node.js gerekir.",
            "inputSchema": kod_ya_da_dosya(json!({
                "eylemler": { "type": "array", "items": { "type": "object" }, "description": "Uygulanacak eylemler" },
                "sure": { "type": "number", "description": "Saniye cinsinden süre sınırı (1-60, varsayılan 10)" }
            }))
        },
        {
            "name": "orhunca_yeni_proje",
            "title": "Yeni Orhunca projesi",
            "description": "Stüdyo şablonlarından yeni bir proje klasörü oluşturur. Şablonlar: konsol, sayi_tahmin, kutuphane, bos_web, web_sitesi, web_uyg, acilis, web_api, tam_yigin, arayuz, oyun.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "konum": { "type": "string", "description": "Projenin oluşturulacağı üst klasörün tam yolu" },
                    "ad": { "type": "string", "description": "Proje (klasör) adı" },
                    "sablon": { "type": "string", "description": "Şablon kimliği (varsayılan: konsol)" }
                },
                "required": ["konum", "ad"]
            }
        },
        {
            "name": "orhunca_dosyalar",
            "title": "Proje dosyalarını listele",
            "description": "Bir proje klasöründeki dosyaları listeler (gizli dosyalar, cikti/ ve paketler/ hariç).",
            "inputSchema": {
                "type": "object",
                "properties": { "klasor": { "type": "string", "description": "Klasörün tam yolu" } },
                "required": ["klasor"]
            },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "orhunca_dosya_oku",
            "title": "Proje dosyası oku",
            "description": "Bir proje dosyasını (.ohc, .ohchtml, .ohcproj, .css, .js, .html, .md, .json, .txt, .csv) okur.",
            "inputSchema": {
                "type": "object",
                "properties": { "dosya": { "type": "string", "description": "Dosyanın tam yolu" } },
                "required": ["dosya"]
            },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "orhunca_dosya_yaz",
            "title": "Proje dosyası yaz",
            "description": "Bir proje dosyasını yazar (yoksa oluşturur, varsa üzerine yazar). Yalnızca proje dosya türleri yazılabilir. Yazdıktan sonra orhunca_denetle ile denetleyin.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dosya": { "type": "string", "description": "Dosyanın tam yolu" },
                    "icerik": { "type": "string", "description": "Dosyanın yeni içeriği" }
                },
                "required": ["dosya", "icerik"]
            },
            "annotations": { "destructiveHint": true }
        }
    ])
}

fn metin<'a>(g: &'a Value, ad: &str) -> Result<&'a str, String> {
    g[ad].as_str().ok_or_else(|| format!("'{ad}' verilmeli"))
}

fn arac_cagir(ad: &str, g: &Value) -> Result<String, String> {
    let kod = g["kod"].as_str();
    let dosya = g["dosya"].as_str().map(Path::new);
    match ad {
        "orhunca_rehber" => Ok(match g["bolum"].as_str().filter(|b| !b.trim().is_empty()) {
            Some(b) => ajan::rehber_bolumu(b),
            None => ajan::kisa_rehber(),
        }),
        "orhunca_denetle" => ajan::denetle(kod, dosya),
        "orhunca_calistir" => {
            let sure = g["sure"].as_f64().unwrap_or(10.0).clamp(1.0, 60.0);
            ajan::calistir(
                kod,
                dosya,
                g["girdi"].as_str().unwrap_or(""),
                Duration::from_secs_f64(sure),
            )
            .map(|c| c.metin())
        }
        "orhunca_bicimlendir" => Ok(ajan::bicimlendir(kod.ok_or("'kod' verilmeli")?)),
        "orhunca_arayuz" => {
            let sure = g["sure"].as_f64().unwrap_or(10.0).clamp(1.0, 60.0);
            ajan::arayuz_calistir(kod, dosya, &g["eylemler"], Duration::from_secs_f64(sure))
        }
        "orhunca_yeni_proje" => ajan::yeni_proje(
            Path::new(metin(g, "konum")?),
            metin(g, "ad")?,
            g["sablon"].as_str().unwrap_or("konsol"),
        ),
        "orhunca_dosyalar" => ajan::proje_dosyalari(Path::new(metin(g, "klasor")?)),
        "orhunca_dosya_oku" => ajan::dosya_oku(Path::new(metin(g, "dosya")?)),
        "orhunca_dosya_yaz" => ajan::dosya_yaz(Path::new(metin(g, "dosya")?), metin(g, "icerik")?),
        _ => Err(format!("bilinmeyen araç '{ad}'")),
    }
}

/// Bir iletiyi işler; yanıt gerekiyorsa döndürür (bildirimlere yanıt verilmez).
pub fn isle(ileti: &Value) -> Option<Value> {
    let kimlik = ileti.get("id")?.clone();
    let p = &ileti["params"];
    let sonuc = match ileti["method"].as_str().unwrap_or("") {
        "initialize" => Ok(json!({
            "protocolVersion": p["protocolVersion"].as_str().unwrap_or(PROTOKOL),
            "capabilities": { "tools": {}, "resources": {} },
            "serverInfo": { "name": "orhunca", "title": "Orhunca", "version": crate::SURUM },
            "instructions": TALIMAT
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": araclar() })),
        "tools/call" => {
            let ad = p["name"].as_str().unwrap_or("");
            let g = p.get("arguments").cloned().unwrap_or(json!({}));
            Ok(match arac_cagir(ad, &g) {
                Ok(m) => json!({ "content": [{ "type": "text", "text": m }], "isError": false }),
                Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
            })
        }
        "resources/list" => Ok(json!({ "resources": [{
            "uri": REHBER_ADRESI, "name": "rehber", "title": "Orhunca dil rehberi",
            "mimeType": "text/markdown"
        }] })),
        "resources/read" if p["uri"] == REHBER_ADRESI => Ok(json!({ "contents": [{
            "uri": REHBER_ADRESI, "mimeType": "text/markdown", "text": ajan::rehber()
        }] })),
        "resources/read" => Err((-32002, "kaynak bulunamadı".to_string())),
        "prompts/list" => Ok(json!({ "prompts": [] })),
        m => Err((-32601, format!("bilinmeyen yöntem '{m}'"))),
    };
    Some(match sonuc {
        Ok(s) => json!({ "jsonrpc": "2.0", "id": kimlik, "result": s }),
        Err((kod, mesaj)) => {
            json!({ "jsonrpc": "2.0", "id": kimlik, "error": { "code": kod, "message": mesaj } })
        }
    })
}

pub fn calistir() -> Result<(), String> {
    let girdi = std::io::stdin();
    let mut cikti = std::io::stdout();
    for satir in girdi.lock().lines() {
        let satir = satir.map_err(|e| e.to_string())?;
        if satir.trim().is_empty() {
            continue;
        }
        let yanit = match serde_json::from_str::<Value>(&satir) {
            Ok(Value::Array(l)) => {
                let y: Vec<Value> = l.iter().filter_map(isle).collect();
                (!y.is_empty()).then_some(Value::Array(y))
            }
            Ok(v) => isle(&v),
            Err(_) => Some(
                json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "geçersiz JSON" } }),
            ),
        };
        if let Some(y) = yanit {
            writeln!(cikti, "{y}").map_err(|e| e.to_string())?;
            cikti.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
