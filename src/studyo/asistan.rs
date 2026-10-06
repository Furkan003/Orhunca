//! Stüdyo'nun yapay zekâ asistanı. Kullanıcı bir sağlayıcı seçer: Anthropic (Claude),
//! OpenAI, Google Gemini, OpenRouter, Groq, Mistral, DeepSeek, xAI ya da bilgisayarında
//! çalışan yerel bir model (Ollama, LM Studio, llama.cpp, vLLM, Jan…). Asistan kodu kendi
//! denetler ve çalıştırır (araçlar), dosya değişikliğini öneri olarak verir; değişikliği
//! kullanıcı "Uygula" ile yazar. Araç kullanamayan modellerle düz sohbet edilir.
//!
//! Model kodda sabit değildir: kullanıcı, sağlayıcının model listesinden seçer ya da adını
//! kendisi yazar. Yönetici `ORHUNCA_YAPAY_ZEKA=kapali` ile asistanı kapatabilir.

use super::depo;
use crate::ajan;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

const SISTEM: &str = "\
Sen Orhunca Stüdyo'nun içindeki kodlama asistanısın. Orhunca, Türkçe dil bilgisine dayanan \
bir programlama dilidir; kullanıcıların bir kısmı programlamayı yeni öğreniyor olabilir. \
Kullanıcıyla Türkçe konuş; açıklamaların kısa, sade ve öğretici olsun.

Kod yazdığında önce kodu_denetle ile denetle, mümkünse kodu_calistir ile çalıştırıp çıktısına \
bak; hata varsa düzelt. Kullanıcının açık dosyasını değiştirmen gerektiğinde dosyanın yeni \
içeriğinin tamamını dosyayi_degistir ile öner; değişikliği kullanıcı onaylayınca dosyaya \
yazılır. Kullanıcı yalnızca bir soru sorduysa dosyayı değiştirme; kısa örneklerle yanıtla. \
Kullanıcı bir alıştırmayı kendisi çözmeye çalışıyorsa çözümü hemen verme; ipucuyla yol göster.

Aşağıda Orhunca'nın kısa rehberi var. Bir konuda (standart kütüphane, modeller, web, arayüz \
öğeleri, oyunlar…) emin değilsen rehber_oku ile ilgili bölümü oku; tahmin yürütme.";

/// Araç kullanamayan modeller için sistem iletisinin başına eklenir.
const ARACSIZ: &str = "\
Bu oturumda araçların yok: kodu kendin denetleyemez ve çalıştıramazsın. Bu yüzden rehberdeki \
kurallara özellikle dikkat et. Kod verirken kodu ```orhunca ile başlayan bir kod bloğuna yaz; \
kullanıcı bloğun altındaki Uygula düğmesiyle kodu dosyasına yazabilir. Açık dosyayı \
değiştiriyorsan dosyanın yeni içeriğinin tamamını tek bir blokta ver.

";

/// Bir istekte en fazla bu kadar araç turu yapılır.
const EN_COK_TUR: usize = 12;

/// Ollama'nın bağlam penceresi. Varsayılan (2-4 bin belirteç) dil rehberine yetmez;
/// rehber sessizce kesilmesin diye istekte açıkça büyütülür.
const OLLAMA_BAGLAM: u32 = 16384;

#[derive(Clone, Copy, PartialEq)]
enum Tur {
    /// Anthropic Messages API
    Anthropic,
    /// OpenAI uyumlu `/chat/completions` (sağlayıcıların ve yerel sunucuların çoğu)
    OpenAi,
    /// Ollama'nın kendi API'si (`/api/chat`; bağlam penceresi ayarlanabilir)
    Ollama,
}

#[derive(Clone, Copy, PartialEq)]
enum Anahtar {
    Gerekli,
    Istege,
}

struct Saglayici {
    kimlik: &'static str,
    ad: &'static str,
    tur: Tur,
    adres: &'static str,
    anahtar: Anahtar,
    /// Anahtarın alınacağı sayfa ya da kurulum adresi
    sayfa: &'static str,
    /// Bilgisayarda çalışan sunucu (internet ve ücret gerekmez)
    yerel: bool,
    /// Kurulum ve bağlantı hatalarında gösterilen ipucu
    ipucu: &'static str,
}

const SAGLAYICILAR: &[Saglayici] = &[
    Saglayici {
        kimlik: "anthropic",
        ad: "Anthropic (Claude)",
        tur: Tur::Anthropic,
        adres: "https://api.anthropic.com",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://console.anthropic.com",
        yerel: false,
        ipucu: "Anahtarı console.anthropic.com adresinden alabilirsiniz.",
    },
    Saglayici {
        kimlik: "openai",
        ad: "OpenAI (GPT)",
        tur: Tur::OpenAi,
        adres: "https://api.openai.com/v1",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://platform.openai.com/api-keys",
        yerel: false,
        ipucu: "Anahtarı platform.openai.com/api-keys adresinden alabilirsiniz.",
    },
    Saglayici {
        kimlik: "gemini",
        ad: "Google Gemini",
        tur: Tur::OpenAi,
        adres: "https://generativelanguage.googleapis.com/v1beta/openai",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://aistudio.google.com/apikey",
        yerel: false,
        ipucu: "Anahtarı aistudio.google.com/apikey adresinden ücretsiz alabilirsiniz.",
    },
    Saglayici {
        kimlik: "openrouter",
        ad: "OpenRouter",
        tur: Tur::OpenAi,
        adres: "https://openrouter.ai/api/v1",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://openrouter.ai/keys",
        yerel: false,
        ipucu: "Tek anahtarla yüzlerce modele erişim. Anahtarı openrouter.ai/keys adresinden alabilirsiniz.",
    },
    Saglayici {
        kimlik: "groq",
        ad: "Groq",
        tur: Tur::OpenAi,
        adres: "https://api.groq.com/openai/v1",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://console.groq.com/keys",
        yerel: false,
        ipucu: "Anahtarı console.groq.com/keys adresinden alabilirsiniz.",
    },
    Saglayici {
        kimlik: "mistral",
        ad: "Mistral",
        tur: Tur::OpenAi,
        adres: "https://api.mistral.ai/v1",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://console.mistral.ai/api-keys",
        yerel: false,
        ipucu: "Anahtarı console.mistral.ai/api-keys adresinden alabilirsiniz.",
    },
    Saglayici {
        kimlik: "deepseek",
        ad: "DeepSeek",
        tur: Tur::OpenAi,
        adres: "https://api.deepseek.com/v1",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://platform.deepseek.com/api_keys",
        yerel: false,
        ipucu: "Anahtarı platform.deepseek.com/api_keys adresinden alabilirsiniz.",
    },
    Saglayici {
        kimlik: "xai",
        ad: "xAI (Grok)",
        tur: Tur::OpenAi,
        adres: "https://api.x.ai/v1",
        anahtar: Anahtar::Gerekli,
        sayfa: "https://console.x.ai",
        yerel: false,
        ipucu: "Anahtarı console.x.ai adresinden alabilirsiniz.",
    },
    Saglayici {
        kimlik: "ollama",
        ad: "Ollama (yerel)",
        tur: Tur::Ollama,
        adres: "http://localhost:11434",
        anahtar: Anahtar::Istege,
        sayfa: "https://ollama.com/download",
        yerel: true,
        ipucu: "Ollama'yı ollama.com adresinden kurun, ardından bir model indirin: \
                ollama pull qwen2.5-coder:7b (araç kullanabilen modeller en iyi sonucu verir).",
    },
    Saglayici {
        kimlik: "lmstudio",
        ad: "LM Studio (yerel)",
        tur: Tur::OpenAi,
        adres: "http://localhost:1234/v1",
        anahtar: Anahtar::Istege,
        sayfa: "https://lmstudio.ai",
        yerel: true,
        ipucu: "LM Studio'da bir model indirip yükleyin ve Developer sekmesinden sunucuyu \
                başlatın. Modeli yüklerken bağlam uzunluğunu en az 16384 yapın.",
    },
    Saglayici {
        kimlik: "ozel",
        ad: "Başka (OpenAI uyumlu)",
        tur: Tur::OpenAi,
        adres: "http://localhost:8080/v1",
        anahtar: Anahtar::Istege,
        sayfa: "",
        yerel: true,
        ipucu: "llama.cpp (llama-server), vLLM, Jan, LocalAI, text-generation-webui ya da \
                OpenAI uyumlu herhangi bir sunucunun adresini yazın (genellikle /v1 ile biter).",
    },
];

fn saglayici(kimlik: &str) -> Option<&'static Saglayici> {
    SAGLAYICILAR.iter().find(|s| s.kimlik == kimlik)
}

/// Stüdyo'nun dış bağlantı olarak açabileceği sağlayıcı sayfaları ve belgeler.
pub fn sayfa_mi(adres: &str) -> bool {
    adres == BELGE
        || SAGLAYICILAR
            .iter()
            .any(|s| !s.sayfa.is_empty() && s.sayfa == adres)
}

const BELGE: &str = "https://github.com/furkan003/Orhunca/blob/HEAD/docs/yapay-zeka.md";

/// Ajanların MCP sunucusunu başlatmak için çalıştıracağı program (`<komut> mcp`):
/// `orhunca` komutu ya da masaüstü uygulamasının kendisi.
pub fn mcp_komutu() -> String {
    // AppImage her açılışta geçici bir klasöre açılır; kalıcı yol APPIMAGE'dadır.
    std::env::var("APPIMAGE")
        .ok()
        .filter(|y| !y.is_empty())
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .map(|y| y.display().to_string())
        })
        .unwrap_or_else(|| "orhunca".into())
}

/// Projeye eklenen AGENTS.md: Codex, Cursor, Copilot, Gemini CLI, Claude Code gibi
/// ajanlar projedeki bu dosyayı okuyup Orhunca kodunu nasıl yazacaklarını öğrenir.
pub fn ajan_talimati() -> String {
    let giris = include_str!("../../docs/ajan-girisi.md");
    let giris = giris
        .split_once('\n')
        .map(|(_, k)| k)
        .unwrap_or(giris)
        .trim();
    format!(
        "# Yapay zekâ ajanları için\n\n\
         Bu proje Orhunca ile yazılmıştır. {giris}\n\n\
         ## Araçlar\n\n\
         - `orhunca denetle <dosya.ohc>`: kodu çalıştırmadan derler, Türkçe hata ve ipuçlarını gösterir.\n\
         - `orhunca çalıştır <dosya.ohc>`: programı çalıştırır (proje klasöründe yalnızca `orhunca çalıştır`).\n\
         - `orhunca biçimlendir <dosya.ohc>`: kodu standart biçime getirir.\n\
         - `orhunca mcp`: MCP sunucusu (araçlar: orhunca_rehber, orhunca_denetle, orhunca_calistir, \
         orhunca_bicimlendir). Ajanınız MCP destekliyorsa bu sunucuya bağlanın.\n\n\
         Dil rehberinin tamamı: https://furkan003.github.io/Orhunca/llms-full.txt\n"
    )
}

pub fn kapali_mi() -> bool {
    std::env::var("ORHUNCA_YAPAY_ZEKA").is_ok_and(|d| {
        matches!(
            d.to_lowercase().as_str(),
            "kapali" | "kapalı" | "0" | "hayir" | "hayır" | "off"
        )
    })
}

fn kapali_hatasi() -> String {
    "Yapay zekâ asistanı yönetici tarafından kapatılmış.".into()
}

fn ayar_dosyasi() -> PathBuf {
    depo::ayar_klasoru().join("asistan.json")
}

/// Ayarlar: `{ saglayici, saglayicilar: { kimlik: { anahtar, model, adres } } }`.
/// Önceki sürümlerin `{ anahtar, model }` biçimi Anthropic ayarı olarak okunur.
fn ayarlar() -> Value {
    let mut a: Value = std::fs::read(ayar_dosyasi())
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    if !a["saglayicilar"].is_object() {
        let eski = json!({ "anahtar": a["anahtar"], "model": a["model"] });
        a = json!({ "saglayici": "anthropic", "saglayicilar": {} });
        if eski["anahtar"].is_string() || eski["model"].is_string() {
            a["saglayicilar"]["anthropic"] = eski;
        }
    }
    a
}

fn secili(a: &Value) -> &'static Saglayici {
    a["saglayici"]
        .as_str()
        .and_then(saglayici)
        .unwrap_or(&SAGLAYICILAR[0])
}

fn alan<'a>(a: &'a Value, s: &Saglayici, ad: &str) -> &'a str {
    a["saglayicilar"][s.kimlik][ad].as_str().unwrap_or("")
}

/// Sağlayıcının kullanılacak adresi: sınamalarda ortam değişkeni, sonra kullanıcının
/// yazdığı adres, sonra varsayılan.
fn adres(a: &Value, s: &Saglayici) -> String {
    let adres = std::env::var("ORHUNCA_ASISTAN_ADRESI")
        .ok()
        .or_else(|| Some(alan(a, s, "adres").to_string()).filter(|a| !a.is_empty()))
        .unwrap_or_else(|| s.adres.to_string());
    let adres = adres.trim_end_matches('/');
    // Ollama'nın kendi API'si kökte; OpenAI uyumlu adres (/v1) yapıştırılmışsa düzeltilir.
    if s.tur == Tur::Ollama {
        return adres.trim_end_matches("/v1").to_string();
    }
    adres.to_string()
}

fn anahtar_sonu(k: &str) -> String {
    if k.len() <= 8 {
        return String::new();
    }
    let son: Vec<char> = k.chars().rev().take(4).collect();
    son.into_iter().rev().collect()
}

/// Kullanıma hazır mı: anahtar gerekiyorsa girilmiş olmalı.
fn hazir_mi(a: &Value, s: &Saglayici) -> bool {
    s.anahtar != Anahtar::Gerekli || !alan(a, s, "anahtar").is_empty()
}

/// Arayüze gösterilen durum; anahtarların kendisi hiçbir zaman gönderilmez.
pub fn durum() -> Value {
    if kapali_mi() {
        return json!({ "kapali": true });
    }
    let a = ayarlar();
    let s = secili(&a);
    let liste: Vec<Value> = SAGLAYICILAR
        .iter()
        .map(|p| {
            let k = alan(&a, p, "anahtar");
            json!({
                "kimlik": p.kimlik,
                "ad": p.ad,
                "yerel": p.yerel,
                "anahtar_gerekli": p.anahtar == Anahtar::Gerekli,
                "sayfa": p.sayfa,
                "ipucu": p.ipucu,
                "varsayilan_adres": p.adres,
                "adres": alan(&a, p, "adres"),
                "anahtar_var": !k.is_empty(),
                "anahtar_sonu": anahtar_sonu(k),
                "model": alan(&a, p, "model"),
                "hazir": hazir_mi(&a, p),
            })
        })
        .collect();
    let k = alan(&a, s, "anahtar");
    let model = alan(&a, s, "model");
    json!({
        "kapali": false,
        "saglayici": s.kimlik,
        "saglayici_adi": s.ad,
        "yerel": s.yerel,
        "hazir": hazir_mi(&a, s),
        "anahtar_var": !k.is_empty(),
        "anahtar_sonu": anahtar_sonu(k),
        "model": if model.is_empty() { Value::Null } else { json!(model) },
        "adres": adres(&a, s),
        "saglayicilar": liste,
    })
}

fn adres_gecerli_mi(m: &str) -> bool {
    (m.starts_with("http://") || m.starts_with("https://"))
        && m.len() <= 300
        && !m
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\"'<>\\`".contains(c))
}

/// Ayarları kaydeder. `saglayici` verilirse o sağlayıcı seçilir; `anahtar`, `model` ve
/// `adres` seçili sağlayıcıya yazılır. Boş anahtar ya da adres kayıtlı olanı siler.
pub fn ayar_kaydet(g: &Value) -> Result<Value, String> {
    if kapali_mi() {
        return Err(kapali_hatasi());
    }
    let mut a = ayarlar();
    if let Some(k) = g["saglayici"].as_str() {
        saglayici(k).ok_or("Bilinmeyen sağlayıcı.")?;
        a["saglayici"] = json!(k);
    }
    let s = secili(&a);
    let mut ayar = a["saglayicilar"][s.kimlik].clone();
    if !ayar.is_object() {
        ayar = json!({});
    }
    if let Some(k) = g["anahtar"].as_str() {
        let k = k.trim();
        if k.chars().any(|c| c.is_control() || c.is_whitespace()) || k.len() > 400 {
            return Err("API anahtarı geçersiz görünüyor.".into());
        }
        ayar["anahtar"] = json!(k);
    }
    if let Some(m) = g["model"].as_str() {
        let m = m.trim();
        if m.len() > 200
            || !m
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.:@/+".contains(c))
        {
            return Err("Model adı geçersiz.".into());
        }
        ayar["model"] = json!(m);
    }
    if let Some(m) = g["adres"].as_str() {
        let m = m.trim().trim_end_matches('/');
        if !m.is_empty() && !adres_gecerli_mi(m) {
            return Err(
                "Adres http:// ya da https:// ile başlamalı (ör. http://localhost:11434).".into(),
            );
        }
        ayar["adres"] = json!(m);
    }
    a["saglayicilar"][s.kimlik] = ayar;
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

/// Bir sağlayıcıya bağlanmak için gereken her şey.
struct Baglanti {
    s: &'static Saglayici,
    adres: String,
    anahtar: String,
}

impl Baglanti {
    fn kur(a: &Value) -> Result<Baglanti, String> {
        let s = secili(a);
        let anahtar = alan(a, s, "anahtar").to_string();
        if s.anahtar == Anahtar::Gerekli && anahtar.is_empty() {
            return Err(format!("Önce {} için bir API anahtarı girin.", s.ad));
        }
        Ok(Baglanti {
            s,
            adres: adres(a, s),
            anahtar,
        })
    }

    /// Anahtarı taşıyan başlık; komut satırında görünmesin diye curl'e standart
    /// girdiden verilir.
    fn gizli_baslik(&self) -> Option<String> {
        if self.anahtar.is_empty() {
            return None;
        }
        Some(match self.s.tur {
            Tur::Anthropic => format!("x-api-key: {}", self.anahtar),
            _ => format!("Authorization: Bearer {}", self.anahtar),
        })
    }

    fn basliklar(&self) -> Vec<&'static str> {
        match self.s.kimlik {
            "anthropic" => vec!["anthropic-version: 2023-06-01"],
            "openrouter" => vec![
                "HTTP-Referer: https://furkan003.github.io/Orhunca/",
                "X-Title: Orhunca Studyo",
            ],
            _ => vec![],
        }
    }

    /// curl ile istek. 400 yanıtları "İSTEK:" önekiyle döner (çağıran, isteği
    /// sadeleştirip yeniden deneyebilir).
    fn istek(&self, yol: &str, govde: Option<&Value>) -> Result<Value, String> {
        let gecici = crate::derleme::gecici_klasor("asistan")?;
        let mut komut = crate::komut("curl");
        komut
            .args(["-sS", "--connect-timeout", "15", "--max-time", "900"])
            .args(["-w", "\n%{http_code}"])
            .args(["-H", "content-type: application/json"]);
        for b in self.basliklar() {
            komut.args(["-H", b]);
        }
        let gizli = self.gizli_baslik();
        if gizli.is_some() {
            komut.args(["-H", "@-"]);
        }
        if let Some(g) = govde {
            let dosya = gecici.join("istek.json");
            std::fs::write(&dosya, g.to_string()).map_err(|e| e.to_string())?;
            komut
                .arg("--data-binary")
                .arg(format!("@{}", dosya.display()));
        }
        komut
            .arg(format!("{}{yol}", self.adres))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let sonuc = (|| {
            let mut c = komut
                .spawn()
                .map_err(|e| format!("curl çalıştırılamadı ({e}); curl kurulu olmalı"))?;
            if let Some(mut g) = c.stdin.take() {
                if let Some(b) = &gizli {
                    let _ = writeln!(g, "{b}");
                }
            }
            let c = c.wait_with_output().map_err(|e| e.to_string())?;
            if !c.status.success() {
                let hata = String::from_utf8_lossy(&c.stderr).trim().to_string();
                return Err(match c.status.code() {
                    Some(7) if self.s.yerel => format!(
                        "{} adresine bağlanılamadı ({}). Sunucu çalışıyor mu? {}",
                        self.s.ad, self.adres, self.s.ipucu
                    ),
                    Some(28) => "Yanıt zaman aşımına uğradı.".into(),
                    _ if self.s.yerel => format!("{} adresine bağlanılamadı: {hata}", self.adres),
                    _ => format!("İnternete bağlanılamadı: {hata}"),
                });
            }
            let metin = String::from_utf8_lossy(&c.stdout).into_owned();
            let (govde, kod) = metin.rsplit_once('\n').unwrap_or(("", &metin));
            let kod: u16 = kod.trim().parse().unwrap_or(0);
            let v: Value = serde_json::from_str(govde).unwrap_or(Value::Null);
            if kod == 200 {
                if v.is_null() {
                    return Err(format!(
                        "Sunucunun yanıtı anlaşılamadı. Adres doğru mu? ({})",
                        self.adres
                    ));
                }
                return Ok(v);
            }
            let mesaj = v["error"]["message"]
                .as_str()
                .or(v["error"].as_str())
                .or(v["message"].as_str())
                .or(v["detail"].as_str())
                .map(String::from)
                .unwrap_or_else(|| govde.trim().chars().take(300).collect());
            Err(match kod {
                401 => "API anahtarı geçersiz. Ayarlardan anahtarı denetleyin.".into(),
                402 => format!("Hesabınızda yeterli kredi yok: {mesaj}"),
                403 => format!("Bu anahtarın izni yok: {mesaj}"),
                404 => format!("Bulunamadı (404): {mesaj}"),
                429 => format!("İstek sınırına ulaşıldı; biraz bekleyip yeniden deneyin. {mesaj}"),
                529 | 503 => "Sunucu şu an yoğun; biraz sonra yeniden deneyin.".into(),
                400 | 422 => format!("İSTEK:{mesaj}"),
                _ => format!("API hatası ({kod}): {mesaj}"),
            })
        })();
        let _ = std::fs::remove_dir_all(&gecici);
        sonuc
    }
}

fn temiz(e: String) -> String {
    e.strip_prefix("İSTEK:")
        .map(|m| format!("İstek reddedildi: {m}"))
        .unwrap_or(e)
}

/// Sohbet için uygun olmayan modeller (gömme, ses, görüntü…) listeden çıkarılır.
fn sohbet_modeli_mi(kimlik: &str) -> bool {
    let k = kimlik.to_lowercase();
    ![
        "embed",
        "tts",
        "whisper",
        "dall-e",
        "moderation",
        "davinci",
        "babbage",
        "realtime",
        "transcribe",
        "audio",
        "imagen",
        "image",
        "veo",
        "aqa",
        "guard",
        "rerank",
        // OpenAI'ın yalnızca Responses API'siyle ya da eski tamamlama uç noktasıyla çalışanlar
        "codex",
        "o1-pro",
        "o3-pro",
        "gpt-5-pro",
        "deep-research",
        "search",
        "computer-use",
        "turbo-instruct",
    ]
    .iter()
    .any(|x| k.contains(x))
}

/// Seçili sağlayıcının modelleri (biliniyorsa en yeniden eskiye).
pub fn modeller() -> Result<Value, String> {
    if kapali_mi() {
        return Err(kapali_hatasi());
    }
    let b = Baglanti::kur(&ayarlar())?;
    let liste: Vec<Value> = match b.s.tur {
        Tur::Anthropic => {
            let v = b.istek("/v1/models?limit=100", None).map_err(temiz)?;
            v["data"]
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
                .collect()
        }
        Tur::Ollama => {
            let v = b.istek("/api/tags", None).map_err(temiz)?;
            v["models"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|m| {
                    let ad = m["name"].as_str().or(m["model"].as_str())?;
                    sohbet_modeli_mi(ad).then(|| json!({ "kimlik": ad, "ad": ad }))
                })
                .collect()
        }
        Tur::OpenAi => {
            let v = b.istek("/models", None).map_err(temiz)?;
            let mut l: Vec<Value> = v["data"]
                .as_array()
                .or(v["models"].as_array())
                .cloned()
                .unwrap_or_default();
            if l.iter().all(|m| m["created"].is_number()) {
                l.sort_by_key(|m| std::cmp::Reverse(m["created"].as_i64().unwrap_or(0)));
            }
            l.into_iter()
                .filter_map(|m| {
                    let kimlik = m["id"].as_str().or(m["name"].as_str())?;
                    // Gemini kimlikleri "models/" önekiyle gelir
                    let kimlik = kimlik.strip_prefix("models/").unwrap_or(kimlik);
                    if !sohbet_modeli_mi(kimlik) {
                        return None;
                    }
                    let ad = m["name"]
                        .as_str()
                        .or(m["display_name"].as_str())
                        .filter(|a| !a.starts_with("models/"))
                        .unwrap_or(kimlik);
                    Some(json!({ "kimlik": kimlik, "ad": ad }))
                })
                .collect()
        }
    };
    Ok(json!({ "modeller": liste }))
}

/// Araçların adı, açıklaması ve girdi şeması (sağlayıcıdan bağımsız).
fn arac_tanimlari() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "kodu_denetle",
            "Orhunca kodunu çalıştırmadan derler. Türkçe hata mesajlarını (satır, sütun, ipucu) ya da 'Hata yok.' döndürür.",
            json!({
                "type": "object",
                "properties": { "kod": { "type": "string", "description": "Orhunca kaynak kodu" } },
                "required": ["kod"]
            }),
        ),
        (
            "kodu_calistir",
            "Orhunca programını derleyip en fazla 10 saniye çalıştırır ve çıktısını döndürür. Arayüz programları yalnızca derlenir.",
            json!({
                "type": "object",
                "properties": {
                    "kod": { "type": "string", "description": "Orhunca kaynak kodu" },
                    "girdi": { "type": "string", "description": "Programın oku() ile okuyacağı girdi; satırlar \\n ile ayrılır" }
                },
                "required": ["kod"]
            }),
        ),
        (
            "rehber_oku",
            "Orhunca rehberinin bir bölümünü döndürür (bölüm adları kısa rehberin sonunda). Tam rehber için 'hepsi'.",
            json!({
                "type": "object",
                "properties": { "bolum": { "type": "string", "description": "Bölüm adı, ör. 'Standart kütüphane', 'Modeller', 'Oyunlar'" } },
                "required": ["bolum"]
            }),
        ),
        (
            "dosyayi_degistir",
            "Kullanıcının açık dosyası için yeni içerik önerir. Dosyanın yeni içeriğinin TAMAMINI verin. Kullanıcı öneriyi görür ve onaylarsa dosyaya yazılır.",
            json!({
                "type": "object",
                "properties": {
                    "icerik": { "type": "string", "description": "Dosyanın yeni içeriğinin tamamı" },
                    "aciklama": { "type": "string", "description": "Değişikliğin bir cümlelik açıklaması" }
                },
                "required": ["icerik"]
            }),
        ),
    ]
}

fn araclar(tur: Tur) -> Value {
    arac_tanimlari()
        .into_iter()
        .map(|(ad, aciklama, sema)| match tur {
            Tur::Anthropic => json!({ "name": ad, "description": aciklama, "input_schema": sema }),
            _ => json!({ "type": "function", "function": {
                "name": ad, "description": aciklama, "parameters": sema
            } }),
        })
        .collect()
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
        "rehber_oku" => (
            ajan::rehber_bolumu(g["bolum"].as_str().unwrap_or("")),
            false,
        ),
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

/// Düşünen modellerin metne kattığı `<think>…</think>` bölümlerini çıkarır.
fn dusunceyi_cikar(m: &str) -> String {
    let mut m = m.to_string();
    while let Some(b) = m.find("<think>") {
        match m[b..].find("</think>") {
            Some(s) => m.replace_range(b..b + s + "</think>".len(), ""),
            None => m.truncate(b),
        }
    }
    m.trim().to_string()
}

/// Bir sorunun yanıtı.
#[derive(Default)]
struct Sonuc {
    metinler: Vec<String>,
    adimlar: Vec<Value>,
    oneri: Option<Value>,
    /// Model araç kullanamadığı için düz sohbet edildi
    aracsiz: bool,
}

impl Sonuc {
    fn metin(&mut self, m: &str) {
        let m = dusunceyi_cikar(m);
        if !m.is_empty() {
            self.metinler.push(m);
        }
    }

    fn arac(&mut self, ad: &str, girdi: &Value) -> String {
        let (sonuc, hatali) = arac_yurut(ad, girdi, &mut self.oneri);
        self.adimlar
            .push(json!({ "ad": ad, "sonuc": sonuc.chars().take(2000).collect::<String>() }));
        if hatali {
            format!("HATA: {sonuc}")
        } else {
            sonuc
        }
    }
}

/// Konuşmayı sürdürür: `mesajlar` [{rol: kullanici|asistan, metin}], `dosya` ve
/// `icerik` kullanıcının açık dosyası. Asistanın yanıtını, kullandığı araçları ve
/// varsa dosya önerisini döndürür.
pub fn sor(g: &Value) -> Result<Value, String> {
    if kapali_mi() {
        return Err(kapali_hatasi());
    }
    let a = ayarlar();
    let b = Baglanti::kur(&a)?;
    let model = alan(&a, b.s, "model").to_string();
    if model.is_empty() {
        return Err("Önce bir model seçin.".into());
    }
    let gelen = g["mesajlar"].as_array().cloned().unwrap_or_default();
    if gelen.is_empty() || gelen.len() > 200 {
        return Err("Geçersiz konuşma.".into());
    }
    // (rol, metin) çiftleri; ardışık aynı roller birleştirilir (API'ler dönüşümlü roller ister).
    let mut konusma: Vec<(&str, String)> = Vec::new();
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
        match konusma.last_mut() {
            Some(son) if son.0 == rol => son.1 = format!("{}\n\n{metin}", son.1),
            _ => konusma.push((rol, metin)),
        }
    }
    if konusma.first().map(|m| m.0 != "user").unwrap_or(true)
        || konusma.last().map(|m| m.0 != "user").unwrap_or(true)
    {
        return Err("Konuşma kullanıcı mesajıyla başlayıp bitmeli.".into());
    }
    let mut sonuc = Sonuc::default();
    let durma = match b.s.tur {
        Tur::Anthropic => anthropic_sor(&b, &model, &konusma, &mut sonuc),
        _ => openai_sor(&b, &model, &konusma, &mut sonuc),
    }
    .map_err(|e| {
        let e = temiz(e);
        if b.s.tur == Tur::Ollama && e.contains("not found") {
            format!("{e}\nModel bu bilgisayarda yok; indirmek için: ollama pull {model}")
        } else if b.s.yerel && (e.contains("context") || e.contains("token")) {
            format!(
                "{e}\nModelin bağlam uzunluğu dil rehberine yetmiyor olabilir; \
                 sunucu ayarlarından en az 16384 yapın."
            )
        } else {
            e
        }
    })?;
    match durma.as_str() {
        "max_tokens" | "length" => sonuc
            .metinler
            .push("(Yanıt uzunluk sınırında kesildi.)".into()),
        "refusal" | "content_filter" => sonuc.metinler.push(
            "Asistan bu isteği yanıtlamadı. Sorunuzu farklı bir biçimde sormayı deneyin.".into(),
        ),
        _ => {}
    }
    if sonuc.metinler.is_empty() && sonuc.oneri.is_none() {
        sonuc.metinler.push(if b.s.yerel {
            "(Model boş yanıt verdi. Küçük yerel modeller bu işte zorlanabilir; \
             daha büyük bir model deneyin, ör. qwen2.5-coder:7b.)"
                .into()
        } else {
            "(Model boş yanıt verdi.)".into()
        });
    }
    Ok(json!({
        "yanit": sonuc.metinler.join("\n\n"),
        "adimlar": sonuc.adimlar,
        "oneri": sonuc.oneri,
        "aracsiz": sonuc.aracsiz,
    }))
}

/// Anthropic Messages API ile araç döngüsü; son durma nedenini döndürür.
fn anthropic_sor(
    b: &Baglanti,
    model: &str,
    konusma: &[(&str, String)],
    sonuc: &mut Sonuc,
) -> Result<String, String> {
    let mut mesajlar: Vec<Value> = konusma
        .iter()
        .map(|(rol, metin)| json!({ "role": rol, "content": metin }))
        .collect();
    let sistem = json!([{
        "type": "text",
        "text": format!("{SISTEM}\n\n{}", ajan::kisa_rehber()),
        "cache_control": { "type": "ephemeral" }
    }]);
    let mut uyumlu = false;
    let mut durma = String::new();
    for _ in 0..EN_COK_TUR {
        let mut govde = json!({
            "model": model,
            "max_tokens": 16000,
            "system": sistem,
            "tools": araclar(Tur::Anthropic),
            "messages": mesajlar,
        });
        if !uyumlu {
            govde["thinking"] = json!({ "type": "adaptive" });
            govde["output_config"] = json!({ "effort": "medium" });
        }
        let yanit = match b.istek("/v1/messages", Some(&govde)) {
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
                b.istek("/v1/messages", Some(&govde))?
            }
            Err(e) => return Err(e),
        };
        durma = yanit["stop_reason"].as_str().unwrap_or("").to_string();
        if durma == "refusal" {
            break;
        }
        let icerik = yanit["content"].as_array().cloned().unwrap_or_default();
        let mut sonuclar: Vec<Value> = Vec::new();
        for blok in &icerik {
            match blok["type"].as_str() {
                Some("text") => sonuc.metin(blok["text"].as_str().unwrap_or("")),
                Some("tool_use") => {
                    let ad = blok["name"].as_str().unwrap_or("");
                    let cikti = sonuc.arac(ad, &blok["input"]);
                    sonuclar.push(json!({
                        "type": "tool_result",
                        "tool_use_id": blok["id"],
                        "is_error": cikti.starts_with("HATA: "),
                        "content": cikti,
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
    Ok(durma)
}

/// OpenAI uyumlu `/chat/completions` ya da Ollama `/api/chat` ile araç döngüsü.
/// Sunucu araçları reddederse (model araç kullanamıyorsa) düz sohbete geçilir.
fn openai_sor(
    b: &Baglanti,
    model: &str,
    konusma: &[(&str, String)],
    sonuc: &mut Sonuc,
) -> Result<String, String> {
    let ollama = b.s.tur == Tur::Ollama;
    let yol = if ollama {
        "/api/chat"
    } else {
        "/chat/completions"
    };
    let rehber = ajan::kisa_rehber();
    let sistem = |aracsiz: bool| {
        json!({
            "role": "system",
            "content": format!("{}{SISTEM}\n\n{rehber}", if aracsiz { ARACSIZ } else { "" }),
        })
    };
    let mut mesajlar: Vec<Value> = std::iter::once(sistem(false))
        .chain(
            konusma
                .iter()
                .map(|(rol, metin)| json!({ "role": rol, "content": metin })),
        )
        .collect();
    let mut durma = String::new();
    for tur in 0..EN_COK_TUR {
        let mut govde = json!({ "model": model, "messages": mesajlar });
        if ollama {
            govde["stream"] = json!(false);
            govde["options"] = json!({ "num_ctx": OLLAMA_BAGLAM });
        }
        if !sonuc.aracsiz {
            govde["tools"] = araclar(b.s.tur);
        }
        let yanit = match b.istek(yol, Some(&govde)) {
            Ok(y) => y,
            // Araç desteklemeyen modeller ve sunucular 400 verir: ilk turda araçsız
            // yeniden denenir.
            Err(e) if e.starts_with("İSTEK:") && tur == 0 && !sonuc.aracsiz => {
                sonuc.aracsiz = true;
                mesajlar[0] = sistem(true);
                let mut govde = govde;
                govde["messages"] = json!(mesajlar);
                if let Some(o) = govde.as_object_mut() {
                    o.remove("tools");
                }
                b.istek(yol, Some(&govde))?
            }
            Err(e) => return Err(e),
        };
        let (ileti, neden) = if ollama {
            (&yanit["message"], yanit["done_reason"].as_str())
        } else {
            (
                &yanit["choices"][0]["message"],
                yanit["choices"][0]["finish_reason"].as_str(),
            )
        };
        if ileti.is_null() {
            return Err(format!(
                "Sunucunun yanıtı anlaşılamadı: {}",
                yanit.to_string().chars().take(300).collect::<String>()
            ));
        }
        durma = neden.unwrap_or("").to_string();
        sonuc.metin(ileti["content"].as_str().unwrap_or(""));
        let cagrilar = ileti["tool_calls"].as_array().cloned().unwrap_or_default();
        if cagrilar.is_empty() {
            break;
        }
        // Asistan iletisi araç çağrılarıyla birlikte geçmişe eklenir (düşünce metni olmadan).
        let mut asistan =
            json!({ "role": "assistant", "content": ileti["content"].as_str().unwrap_or("") });
        let mut cagrilar_kayit = Vec::new();
        let mut sonuclar = Vec::new();
        for (i, c) in cagrilar.iter().enumerate() {
            let f = &c["function"];
            let ad = f["name"].as_str().unwrap_or("");
            // OpenAI argümanları JSON metni olarak, Ollama nesne olarak verir.
            let girdi = match &f["arguments"] {
                Value::String(m) => serde_json::from_str(m).unwrap_or(Value::Null),
                v => v.clone(),
            };
            let kimlik = c["id"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| format!("cagri_{tur}_{i}"));
            let cikti = if girdi.is_object() {
                sonuc.arac(ad, &girdi)
            } else {
                "HATA: araç argümanları geçerli bir JSON nesnesi değil".into()
            };
            if ollama {
                cagrilar_kayit.push(json!({ "function": { "name": ad, "arguments": girdi } }));
                sonuclar.push(json!({ "role": "tool", "tool_name": ad, "content": cikti }));
            } else {
                cagrilar_kayit.push(json!({
                    "id": kimlik, "type": "function",
                    "function": { "name": ad, "arguments": f["arguments"].as_str().map(String::from).unwrap_or_else(|| girdi.to_string()) }
                }));
                sonuclar.push(json!({ "role": "tool", "tool_call_id": kimlik, "content": cikti }));
            }
        }
        asistan["tool_calls"] = json!(cagrilar_kayit);
        mesajlar.push(asistan);
        mesajlar.extend(sonuclar);
    }
    Ok(durma)
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn dusunce_cikarilir() {
        assert_eq!(dusunceyi_cikar("<think>a\nb</think>\nMerhaba"), "Merhaba");
        assert_eq!(dusunceyi_cikar("x <think>yarım"), "x");
        assert_eq!(dusunceyi_cikar("düz"), "düz");
    }

    #[test]
    fn adres_denetimi() {
        assert!(adres_gecerli_mi("http://localhost:11434"));
        assert!(adres_gecerli_mi("https://ornek.com/v1"));
        assert!(!adres_gecerli_mi("file:///etc/passwd"));
        assert!(!adres_gecerli_mi("http://a b"));
    }

    #[test]
    fn sohbet_modelleri() {
        assert!(sohbet_modeli_mi("gpt-4.1"));
        assert!(sohbet_modeli_mi("qwen2.5-coder:7b"));
        assert!(!sohbet_modeli_mi("text-embedding-3-small"));
        assert!(!sohbet_modeli_mi("nomic-embed-text:latest"));
        assert!(!sohbet_modeli_mi("gpt-5-codex"));
        assert!(!sohbet_modeli_mi("o3-pro"));
        assert!(sohbet_modeli_mi("gemini-2.5-pro"));
        assert!(sohbet_modeli_mi("qwen2.5-coder-7b-instruct"));
        assert!(!sohbet_modeli_mi("gpt-3.5-turbo-instruct"));
    }

    #[test]
    fn saglayici_kimlikleri_benzersiz() {
        for (i, s) in SAGLAYICILAR.iter().enumerate() {
            assert!(SAGLAYICILAR[i + 1..].iter().all(|t| t.kimlik != s.kimlik));
            assert!(adres_gecerli_mi(s.adres), "{}", s.kimlik);
        }
    }
}
