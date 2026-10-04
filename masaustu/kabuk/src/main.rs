//! Orhunca pencere kabuğu: `orhunca paketle` ile üretilen masaüstü uygulamalarının
//! hazır gövdesi.
//!
//! Derleyici bu programın sonuna arayüz programının tek dosyalık sayfasını ekler:
//!
//! ```text
//! [kabuk] "OHCKABUK" [u32 başlık uzunluğu] [başlık] [sayfa] "ORHUNCA!" [u64 yük uzunluğu]
//! ```
//!
//! Kabuk kendi dosyasının sonundaki bu yükü okur ve sayfayı kendi penceresinde
//! (Windows'ta WebView2, Linux'ta WebKitGTK) açar. Programın dosyaları
//! (`dosya_yaz`, `dosya_oku`...) kullanıcının veri klasöründe `depo.json`
//! dosyasında kalıcı olarak saklanır.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::http::Response;
use wry::{WebContext, WebViewBuilder};

const YUK_IMI: &[u8; 8] = b"OHCKABUK";
const SON_IMI: &[u8; 8] = b"ORHUNCA!";
/// Pencere simgesi: Orhunca logosu, 64×64 RGBA (docs/marka/logo.svg).
const SIMGE: &[u8] = include_bytes!("../simge/simge64.rgba");

struct Uygulama {
    baslik: String,
    sayfa: Vec<u8>,
}

fn yuku_oku() -> Result<Uygulama, String> {
    let yol = std::env::current_exe().map_err(|e| e.to_string())?;
    let veri = std::fs::read(&yol).map_err(|e| format!("'{}' okunamadı: {e}", yol.display()))?;
    yuku_coz(&veri)
}

fn yuku_coz(veri: &[u8]) -> Result<Uygulama, String> {
    let bos = || "bu kabuğa arayüz programı eklenmemiş (orhunca paketle ile üretin)".to_string();
    if veri.len() < 16 || &veri[veri.len() - 16..veri.len() - 8] != SON_IMI {
        return Err(bos());
    }
    let uzunluk = u64::from_le_bytes(veri[veri.len() - 8..].try_into().unwrap()) as usize;
    let bas = (veri.len() - 16).checked_sub(uzunluk).ok_or_else(bos)?;
    let yuk = &veri[bas..veri.len() - 16];
    if yuk.len() < 12 || &yuk[..8] != YUK_IMI {
        return Err(bos());
    }
    let n = u32::from_le_bytes(yuk[8..12].try_into().unwrap()) as usize;
    let baslik = yuk.get(12..12 + n).ok_or_else(bos)?;
    Ok(Uygulama {
        baslik: String::from_utf8_lossy(baslik).into_owned(),
        sayfa: yuk[12 + n..].to_vec(),
    })
}

/// Uygulamanın verilerini sakladığı klasör (başlığa göre ayrı).
fn veri_klasoru(baslik: &str) -> PathBuf {
    let ad: String = baslik
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let ad = format!("orhunca-{}", if ad.is_empty() { "uygulama".into() } else { ad });
    let kok = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/share")))
    };
    kok.unwrap_or_else(std::env::temp_dir).join(ad)
}

type Depo = BTreeMap<String, String>;

fn depoyu_oku(yol: &Path) -> Depo {
    std::fs::read_to_string(yol)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Sayfadaki depo işlemi: `["yaz", anahtar, değer]` ya da `["sil", anahtar]`.
fn depo_islemi(depo: &mut Depo, ileti: &str) -> bool {
    let Ok(serde_json::Value::Array(p)) = serde_json::from_str(ileti) else {
        return false;
    };
    match (p.first().and_then(|v| v.as_str()), p.get(1).and_then(|v| v.as_str())) {
        (Some("yaz"), Some(a)) => {
            let d = p.get(2).and_then(|v| v.as_str()).unwrap_or("");
            depo.insert(a.to_string(), d.to_string());
            true
        }
        (Some("sil"), Some(a)) => depo.remove(a).is_some(),
        _ => false,
    }
}

/// Sayfa yüklenmeden çalışan betik: `window.orhuncaKabukDepo` (orhunca.js
/// dosyaları localStorage yerine burada tutar). Okumalar bellekten, yazmalar
/// hem bellekte hem kabuğa iletilerek diskte.
fn baslangic_betigi(depo: &Depo, sinama: bool) -> String {
    let veri = serde_json::to_string(depo).unwrap_or_else(|_| "{}".into());
    // Sınama kipi: sayfa yüklendikten kısa süre sonra görünen metni kabuğa
    // iletir; kabuk bunu yazıp kapanır (gerçek pencerenin çalıştığını gösterir).
    let sinama = if sinama {
        "window.addEventListener('load', () => setTimeout(() => \
           ilet(['sınama', document.body ? document.body.innerText : '']), 1500));"
    } else {
        ""
    };
    format!(
        r#"(function () {{
  const d = new Map(Object.entries({veri}));
  const ilet = (m) => window.ipc.postMessage(JSON.stringify(m));
  window.orhuncaKabukDepo = {{
    getItem(a) {{ return d.has(a) ? d.get(a) : null; }},
    setItem(a, v) {{ v = String(v); d.set(a, v); ilet(['yaz', a, v]); }},
    removeItem(a) {{ if (d.delete(a)) ilet(['sil', a]); }},
  }};
  {sinama}
}})();"#
    )
}

fn main() {
    let uygulama = match yuku_oku() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Orhunca: {e}");
            std::process::exit(1);
        }
    };
    // Test ve sorun giderme için: pencere açmadan gömülü sayfayı yazar.
    if std::env::var_os("ORHUNCA_KABUK_DOKUM").is_some() {
        println!("{}", uygulama.baslik);
        print!("{}", String::from_utf8_lossy(&uygulama.sayfa));
        return;
    }
    if let Err(e) = calistir(uygulama) {
        eprintln!("Orhunca: pencere açılamadı: {e}");
        std::process::exit(1);
    }
}

fn calistir(uygulama: Uygulama) -> Result<(), String> {
    let klasor = veri_klasoru(&uygulama.baslik);
    let _ = std::fs::create_dir_all(&klasor);
    let depo_yolu = klasor.join("depo.json");
    let mut depo = depoyu_oku(&depo_yolu);
    let sinama = std::env::var_os("ORHUNCA_KABUK_SINAMA").is_some();
    let betik = baslangic_betigi(&depo, sinama);

    let olaylar = EventLoopBuilder::<String>::with_user_event().build();
    let vekil = olaylar.create_proxy();
    let pencere = WindowBuilder::new()
        .with_title(&uygulama.baslik)
        .with_inner_size(tao::dpi::LogicalSize::new(1100.0, 760.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(360.0, 300.0))
        .with_window_icon(tao::window::Icon::from_rgba(SIMGE.to_vec(), 64, 64).ok())
        .build(&olaylar)
        .map_err(|e| e.to_string())?;

    let mut baglam = WebContext::new(Some(klasor.join("webview")));
    let sayfa = uygulama.sayfa;
    // Windows'ta (WebView2) özel şemalar http://<şema>.localhost biçiminde açılır.
    let adres = if cfg!(windows) {
        "http://orhunca.localhost/"
    } else {
        "orhunca://uygulama/"
    };
    let kurucu = WebViewBuilder::new_with_web_context(&mut baglam)
        .with_custom_protocol("orhunca".into(), move |_, _istek| {
            Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(Cow::Owned(sayfa.clone()))
                .unwrap()
        })
        .with_initialization_script(&betik)
        .with_ipc_handler(move |ileti| {
            let _ = vekil.send_event(ileti.into_body());
        })
        .with_url(adres);

    #[cfg(not(target_os = "linux"))]
    let _gorunum = kurucu.build(&pencere).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    let _gorunum = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        kurucu.build_gtk(pencere.default_vbox().ok_or("GTK kutusu yok")?)
            .map_err(|e| e.to_string())?
    };

    olaylar.run(move |olay, _, akis| {
        *akis = ControlFlow::Wait;
        match olay {
            Event::UserEvent(ileti) => {
                if sinama {
                    if let Ok(serde_json::Value::Array(p)) = serde_json::from_str(&ileti) {
                        if p.first().and_then(|v| v.as_str()) == Some("sınama") {
                            println!("{}", p.get(1).and_then(|v| v.as_str()).unwrap_or(""));
                            *akis = ControlFlow::Exit;
                            return;
                        }
                    }
                }
                if depo_islemi(&mut depo, &ileti) {
                    if let Ok(s) = serde_json::to_string(&depo) {
                        let gecici = depo_yolu.with_extension("json.yeni");
                        if std::fs::write(&gecici, s).is_ok() {
                            let _ = std::fs::rename(&gecici, &depo_yolu);
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *akis = ControlFlow::Exit,
            _ => {}
        }
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn yuk_cozulur() {
        let mut v = b"kabuk".to_vec();
        let mut yuk = YUK_IMI.to_vec();
        yuk.extend(3u32.to_le_bytes());
        yuk.extend(b"abc<html>");
        v.extend(&yuk);
        v.extend(SON_IMI);
        v.extend((yuk.len() as u64).to_le_bytes());
        let u = yuku_coz(&v).unwrap();
        assert_eq!(u.baslik, "abc");
        assert_eq!(u.sayfa, b"<html>");
        assert!(yuku_coz(b"kabuk").is_err());
    }

    #[test]
    fn depo_islemleri() {
        let mut d = Depo::new();
        assert!(depo_islemi(&mut d, r#"["yaz","a","1"]"#));
        assert_eq!(d["a"], "1");
        assert!(depo_islemi(&mut d, r#"["sil","a"]"#));
        assert!(!depo_islemi(&mut d, "bozuk"));
    }
}
