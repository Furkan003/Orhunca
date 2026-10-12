//! İsteğe bağlı EMA LiteRT paketi: kurulum, kalıcı yerel işçi ve sınırlı ses istekleri.
use super::{depo, http::Yanit};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;

const SURUM: &str = "4720c1c02f8cbf3835189264bbf468378b16e8c5";
const EN_FAZLA_SES: usize = 16 * 1024 * 1024;
type SesSonucu = Result<Vec<u8>, String>;

#[derive(Default)]
struct Kurulum {
    suruyor: bool,
    yuzde: u64,
    ileti: String,
}
fn kurulum() -> &'static Mutex<Kurulum> {
    static K: OnceLock<Mutex<Kurulum>> = OnceLock::new();
    K.get_or_init(Default::default)
}
fn klasor() -> PathBuf {
    depo::ayar_klasoru().join("ses-ema-litert-v1")
}
fn python() -> PathBuf {
    klasor().join("ortam").join(if cfg!(windows) {
        "Scripts/python.exe"
    } else {
        "bin/python"
    })
}
fn hazir() -> bool {
    python().is_file()
        && std::fs::read_to_string(klasor().join("hazir.json"))
            .ok()
            .and_then(|m| serde_json::from_str::<Value>(&m).ok())
            .is_some_and(|v| v["surum"] == SURUM)
}
pub fn durum() -> Value {
    let k = kurulum().lock().unwrap();
    json!({ "hazir": hazir() && !k.suruyor, "kuruluyor": k.suruyor,
        "yuzde": k.yuzde, "ileti": k.ileti, "model_bayt": 27267188,
        "motor": "EMA Lightning LiteRT", "python_gerekli": true })
}

/// Gömülü, incelenmiş betikler kullanılır; ağdan yürütülebilir betik indirilmez.
fn betikleri_yaz() -> Result<(), String> {
    let kok = klasor();
    super::dosya::ozel_klasor(&kok).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&kok, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    for (ad, metin) in [
        ("ses_isci.py", include_str!("ses_isci.py")),
        (
            "ema_host.py",
            include_str!("../../ucuncu_taraf/ema_litert/ema_host.py"),
        ),
        (
            "config.json",
            include_str!("../../ucuncu_taraf/ema_litert/config.json"),
        ),
        (
            "LICENSE",
            include_str!("../../ucuncu_taraf/ema_litert/LICENSE"),
        ),
        (
            "NOTICE",
            include_str!("../../ucuncu_taraf/ema_litert/NOTICE"),
        ),
    ] {
        super::dosya::atomik_yaz(&kok.join(ad), metin.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn sistem_python() -> Result<PathBuf, String> {
    // Sadece sunucunun ortam ayarı; tarayıcı yürütülebilir dosya seçemez.
    let adaylar = if let Some(p) = std::env::var_os("ORHUNCA_SES_PYTHON") {
        vec![PathBuf::from(p)]
    } else {
        vec![PathBuf::from("python3"), PathBuf::from("python")]
    };
    for aday in adaylar {
        if crate::komut(&aday)
            .args([
                "-I",
                "-c",
                "import sys; sys.exit(not ((3,11) <= sys.version_info[:2] <= (3,13)))",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
        {
            return Ok(aday);
        }
    }
    Err("EMA için Python 3.11–3.13 gerekli. Python'u kurup Stüdyo'yu yeniden açın.".into())
}
fn kurulum_calistir() -> Result<(), String> {
    let kok = klasor();
    super::dosya::ozel_klasor(&kok).map_err(|e| e.to_string())?;
    let _kilit = super::dosya::kilitle(&kok).map_err(|e| e.to_string())?;
    let sistem = sistem_python()?;
    betikleri_yaz()?;
    let mut cocuk = crate::komut(sistem)
        .arg("-I")
        .arg(klasor().join("ses_isci.py"))
        .arg("kur")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Ses kurucusu açılamadı: {e}"))?;
    // pip ve indirme alt işlemlerinin kendi süre sınırları vardır.
    let cikti = cocuk
        .stdout
        .take()
        .ok_or("Ses kurucusunun çıktısı açılamadı")?;
    for satir in BufReader::new(cikti).lines().map_while(Result::ok) {
        if let Ok(v) = serde_json::from_str::<Value>(&satir) {
            let mut k = kurulum().lock().unwrap();
            k.ileti = v["ileti"].as_str().unwrap_or("Ses kuruluyor").to_owned();
            k.yuzde = v["yuzde"].as_u64().unwrap_or(0).min(100);
        }
    }
    let basarili = cocuk.wait().is_ok_and(|s| s.success());
    if basarili && hazir() {
        Ok(())
    } else {
        let k = kurulum().lock().unwrap();
        Err(if k.ileti.is_empty() {
            "Ses paketi kurulamadı".into()
        } else {
            k.ileti.clone()
        })
    }
}
pub fn kur() -> Value {
    let Ok(mut isci) = isci().try_lock() else {
        return json!({"hata": "Ses okunuyor; bitince kurulumu yeniden deneyin."});
    };
    let mut k = kurulum().lock().unwrap();
    if k.suruyor {
        return json!({"tamam": true});
    }
    // Yeniden kurulum bozuk dosyayı onarabilir; eski işçi yeni dosyalarla karışmaz.
    isci.take();
    k.suruyor = true;
    k.yuzde = 0;
    k.ileti = "Kurulum başlıyor".into();
    std::thread::spawn(|| {
        let sonuc = kurulum_calistir();
        let mut k = kurulum().lock().unwrap();
        k.suruyor = false;
        if let Err(e) = sonuc {
            k.ileti = e;
            k.yuzde = 0;
        }
    });
    json!({"tamam": true})
}

struct Isci {
    cocuk: Child,
    girdi: ChildStdin,
    yanit: mpsc::Receiver<SesSonucu>,
}
impl Drop for Isci {
    fn drop(&mut self) {
        let _ = self.cocuk.kill();
        let _ = self.cocuk.wait();
    }
}
fn isci() -> &'static Mutex<Option<Isci>> {
    static I: OnceLock<Mutex<Option<Isci>>> = OnceLock::new();
    I.get_or_init(Default::default)
}
fn yanit_oku(oku: &mut impl BufRead) -> SesSonucu {
    let mut satir = String::new();
    // Hatalı işçi bellek taşırmasın; normal başlık yaklaşık 50 bayttır.
    let n = oku
        .take(4096)
        .read_line(&mut satir)
        .map_err(|e| e.to_string())?;
    if n == 0 || !satir.ends_with('\n') {
        return Err("Ses işçisi kapandı veya geçersiz yanıt verdi".into());
    }
    let baslik: Value = serde_json::from_str(&satir).map_err(|_| "Geçersiz ses başlığı")?;
    let boyut = baslik["bayt"].as_u64().ok_or("Ses boyutu yok")?;
    if boyut > EN_FAZLA_SES as u64 {
        return Err("Ses boyutu sınırı aşıldı".into());
    }
    if let Some(hata) = baslik["hata"].as_str() {
        return Err(hata.to_owned());
    }
    let mut ses = vec![0; boyut as usize];
    oku.read_exact(&mut ses)
        .map_err(|_| "Ses işçisi eksik ses döndürdü")?;
    if ses.len() < 44 || &ses[..4] != b"RIFF" || &ses[8..12] != b"WAVE" {
        return Err("Geçersiz WAV sesi".into());
    }
    Ok(ses)
}
fn isci_baslat() -> Result<Isci, String> {
    betikleri_yaz()?;
    let mut cocuk = crate::komut(python())
        .arg("-I")
        .arg(klasor().join("ses_isci.py"))
        .arg("calis")
        .env("PYTHONNOUSERSITE", "1")
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("EMA ses işçisi açılamadı: {e}"))?;
    let girdi = cocuk.stdin.take().ok_or("Ses işçisine yazılamıyor")?;
    let cikti = cocuk.stdout.take().ok_or("Ses işçisinden okunamıyor")?;
    let (gonder, yanit) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut oku = BufReader::new(cikti);
        loop {
            let sonuc = yanit_oku(&mut oku);
            let hata = sonuc.is_err();
            if gonder.send(sonuc).is_err() || hata {
                break;
            }
        }
    });
    Ok(Isci {
        cocuk,
        girdi,
        yanit,
    })
}
pub fn oku(g: &Value) -> Yanit {
    let metin = g["metin"].as_str().unwrap_or("");
    let hiz = g["hiz"].as_f64().unwrap_or(1.0);
    if metin.trim().is_empty() || metin.chars().count() > 1200 || !(0.75..=1.5).contains(&hiz) {
        return Yanit::hata(400, "Ses metni 1–1200 karakter, hız 0,75–1,5 olmalı");
    }
    let Ok(mut i) = isci().try_lock() else {
        return Yanit::hata(429, "Ses motoru meşgul");
    };
    if !hazir() || kurulum().lock().unwrap().suruyor {
        return Yanit::hata(409, "EMA ses paketi hazır değil");
    }
    if i.is_none() {
        match isci_baslat() {
            Ok(yeni) => *i = Some(yeni),
            Err(e) => return Yanit::hata(503, &e),
        }
    }
    let sonuc = (|| {
        let isci = i.as_mut().unwrap();
        let veri = json!({"metin": metin, "hiz": hiz}).to_string() + "\n";
        isci.girdi
            .write_all(veri.as_bytes())
            .and_then(|_| isci.girdi.flush())
            .map_err(|_| "EMA işçisi kapandı".to_string())?;
        isci.yanit
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "EMA ses üretimi zaman aşımına uğradı".to_string())?
    })();
    match sonuc {
        Ok(ses) => Yanit {
            durum: 200,
            tur: "audio/wav",
            govde: ses,
            csp: false,
        },
        Err(e) => {
            i.take();
            Yanit::hata(503, &e)
        }
    }
}
pub fn durdur() {
    if let Ok(mut i) = isci().try_lock() {
        i.take();
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    #[test]
    fn ses_yaniti_sinirlari() {
        let mut fazla = std::io::Cursor::new(b"{\"bayt\":999999999}\n".to_vec());
        assert!(yanit_oku(&mut fazla).unwrap_err().contains("sınırı"));
        let mut eksik = std::io::Cursor::new(b"{\"bayt\":44}\nRIFF".to_vec());
        assert!(yanit_oku(&mut eksik).unwrap_err().contains("eksik"));
        let mut satir = std::io::Cursor::new(vec![b'a'; 5000]);
        assert!(yanit_oku(&mut satir).is_err());
        let mut hata = std::io::Cursor::new(b"{\"bayt\":0,\"hata\":\"bozuk model\"}\n".to_vec());
        assert_eq!(yanit_oku(&mut hata).unwrap_err(), "bozuk model");
    }
    #[test]
    fn ses_istegi_sinirlari() {
        assert_eq!(oku(&json!({"metin": ""})).durum, 400);
        assert_eq!(oku(&json!({"metin": "a".repeat(1201)})).durum, 400);
        assert_eq!(oku(&json!({"metin": "deneme", "hiz": 5})).durum, 400);
    }
}
