//! Paylaşımlı hosting (cPanel, Plesk, DirectAdmin; Apache ya da LiteSpeed): Orhunca
//! programları CGI olarak çalışır. Bkz. docs/yayinlama.md.
//!
//! İki yol vardır:
//! - `orhunca yayınla --cgi`: program bu bilgisayarda Linux için `uygulama.cgi` olarak
//!   derlenir; `cikti/cgi/` klasörü olduğu gibi `public_html`e yüklenir. Bütün istekler
//!   `.htaccess` ile programa gider.
//! - `orhunca yayınla --cgi --kaynakla`: `.ohc` dosyaları yüklenir ve PHP gibi çalışır.
//!   Linux'taki `orhunca` programı `orhunca.cgi` adıyla yüklenir; bir `.ohc` dosyası ilk
//!   istendiğinde derlenip `.orhunca-onbellek/` klasöründe saklanır (dosya değişince
//!   yeniden derlenir). Bu işleyici [`isleyici`]dir.
//!
//! CGI isteğinin kendisi (ortam değişkenleri, oturumlar) çalışma zamanında karşılanır
//! (runtime/orhunca_rt.c, cgi_isle).

use crate::derleme;
use crate::yayinla::klasor_kopyala;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// cPanel'in ücretsiz SSL sertifikası (AutoSSL, Let's Encrypt) `/.well-known/` altındaki
/// dosyalarla doğrulanır; bu adresler programa yönlendirilmez.
const HTACCESS_DERLENMIS: &str = "# orhunca yayınla --cgi tarafından üretildi.
# Bütün istekler Orhunca programına (uygulama.cgi) gider; statik/ dosyalarını da o sunar.
Options +ExecCGI -Indexes
AddHandler cgi-script .cgi
DirectoryIndex uygulama.cgi
RewriteEngine On
RewriteRule ^\\.well-known/ - [L]
RewriteRule ^uygulama\\.cgi$ - [L]
RewriteRule ^ uygulama.cgi [L]
";

const HTACCESS_KAYNAKLA: &str = "# orhunca yayınla --cgi --kaynakla tarafından üretildi.
# .ohc dosyaları PHP gibi çalışır: orhunca.cgi onları ilk istekte derler ve çalıştırır.
Options +ExecCGI -Indexes
AddHandler cgi-script .cgi
DirectoryIndex index.ohc
RewriteEngine On
RewriteRule ^\\.well-known/ - [L]
# Gizli dosyalar (.env, derleme önbelleği), veriler ve proje dosyaları dışarıya kapalı
RewriteRule (^|/)\\. - [F]
RewriteRule ^(veri|paketler)(/|$) - [F]
RewriteRule \\.(ohcproj|ohchtml|md)$ - [F]
RewriteRule ^orhunca\\.cgi$ - [L]
RewriteRule \\.ohc$ orhunca.cgi [L]
# Olmayan adresler index.ohc'deki yollara (al \"/ürünler\": ...) gider
RewriteCond %{REQUEST_FILENAME} !-f
RewriteCond %{REQUEST_FILENAME} !-d
RewriteRule ^ orhunca.cgi [L]
";

/// Veri klasörü ikinci bir önlemle de kapatılır (.htaccess yönlendirmesi çalışmasa bile).
const HTACCESS_KAPALI: &str = "Require all denied\n";

/// `cikti/cgi/` klasörünü hazırlar.
pub fn hazirla(giris: &Path, kaynakla: bool) -> Result<PathBuf, String> {
    let kok = derleme::proje_koku(giris);
    let program = derleme::yukle(giris).map_err(|h| h.metin)?;
    if program.arayuz_programi() {
        return Err(
            "bu bir arayüz programı; sunucu gerekmez\nipucu: orhunca derle --hedef web \
             ile tek bir .html dosyası üretip barındırmaya yükleyin"
                .into(),
        );
    }
    let cikti = kok.join("cikti").join("cgi");
    if cikti.exists() {
        std::fs::remove_dir_all(&cikti)
            .map_err(|e| format!("'{}' silinemedi: {e}", cikti.display()))?;
    }
    std::fs::create_dir_all(cikti.join("veri")).map_err(|e| e.to_string())?;
    let yaz =
        |ad: &str, icerik: &[u8]| std::fs::write(cikti.join(ad), icerik).map_err(|e| e.to_string());
    yaz("veri/.htaccess", HTACCESS_KAPALI.as_bytes())?;
    if kaynakla {
        kaynaklari_kopyala(&kok, &kok, &cikti)?;
        // Giriş dosyası index.ohc değilse adres çubuğunda görünmesin diye index.ohc olur.
        let goreli = giris.strip_prefix(&kok).unwrap_or(giris);
        if goreli != Path::new("index.ohc")
            && !cikti.join("index.ohc").exists()
            && goreli.parent().is_some_and(|u| u.as_os_str().is_empty())
        {
            std::fs::rename(cikti.join(goreli), cikti.join("index.ohc"))
                .map_err(|e| e.to_string())?;
            if let Some(proje) = derleme::proje_dosyasi(&cikti) {
                let p = std::fs::read_to_string(&proje).unwrap_or_default();
                let ad = goreli.to_string_lossy();
                let _ = std::fs::write(&proje, p.replace(&format!("\"{ad}\""), "\"index.ohc\""));
            }
        }
        yaz("orhunca.cgi", &linux_orhunca()?)?;
        yaz(".htaccess", HTACCESS_KAYNAKLA.as_bytes())?;
    } else {
        derleme::derle(giris, &cikti.join("uygulama.cgi"), Some("linux")).map_err(|h| h.metin)?;
        let statik = kok.join("statik");
        if statik.is_dir() {
            klasor_kopyala(&statik, &cikti.join("statik"))?;
        }
        yaz(".htaccess", HTACCESS_DERLENMIS.as_bytes())?;
    }
    let env = kok.join(".env.sunucu");
    if env.is_file() {
        std::fs::copy(&env, cikti.join(".env")).map_err(|e| e.to_string())?;
    }
    izinleri_ayarla(&cikti)?;
    Ok(cikti)
}

/// Projenin kaynakları (derleme çıktıları, yerel veriler ve gizli dosyalar hariç).
fn kaynaklari_kopyala(kok: &Path, k: &Path, hedef: &Path) -> Result<(), String> {
    for g in std::fs::read_dir(k).map_err(|e| e.to_string())?.flatten() {
        let ad = g.file_name().to_string_lossy().into_owned();
        let p = g.path();
        let goreli = p.strip_prefix(kok).unwrap_or(&p);
        if ad.starts_with('.') || (k == kok && matches!(ad.as_str(), "cikti" | "veri" | "target")) {
            continue;
        }
        let h = hedef.join(goreli);
        if p.is_dir() {
            std::fs::create_dir_all(&h).map_err(|e| e.to_string())?;
            kaynaklari_kopyala(kok, &p, hedef)?;
        } else {
            std::fs::copy(&p, &h).map_err(|e| format!("'{}' kopyalanamadı: {e}", p.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn izinleri_ayarla(k: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    for ad in ["uygulama.cgi", "orhunca.cgi"] {
        let p = k.join(ad);
        if p.exists() {
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn izinleri_ayarla(_: &Path) -> Result<(), String> {
    Ok(())
}

/// Sunucuda çalışacak Linux (x86_64) `orhunca` programı: bu bilgisayar Linux ise kendisi,
/// değilse GitHub'daki son sürümden indirilir (SHA-256 ile doğrulanır).
fn linux_orhunca() -> Result<Vec<u8>, String> {
    if let Ok(y) = std::env::var("ORHUNCA_CGI_DERLEYICI") {
        return std::fs::read(&y).map_err(|e| format!("'{y}' okunamadı: {e}"));
    }
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        if let Ok(exe) = std::env::current_exe() {
            if exe.file_name().is_some_and(|a| a == "orhunca") {
                return std::fs::read(&exe).map_err(|e| e.to_string());
            }
        }
    }
    println!("→ sunucu için Linux'taki orhunca indiriliyor");
    let s = crate::guncelleme::son_surum()?;
    let arsiv = crate::guncelleme::indir(&s, "orhunca-linux-x86_64.tar.gz")?;
    let klasor = arsiv.with_extension("acilan");
    let _ = std::fs::remove_dir_all(&klasor);
    std::fs::create_dir_all(&klasor).map_err(|e| e.to_string())?;
    let d = crate::komut("tar")
        .arg("-xf")
        .arg(&arsiv)
        .arg("-C")
        .arg(&klasor)
        .status()
        .map_err(|_| "arşiv açılamadı: tar bulunamadı".to_string())?;
    if !d.success() {
        return Err("arşiv açılamadı".into());
    }
    std::fs::read(klasor.join("orhunca")).map_err(|e| format!("arşivde orhunca yok: {e}"))
}

// ---------------------------------------------------------------------------
// Sunucuda: orhunca.cgi
// ---------------------------------------------------------------------------

/// `orhunca` argümansız ve bir CGI isteği içinde çalıştırıldı mı?
pub fn isleyici_mi(args: &[String]) -> bool {
    args.is_empty() && std::env::var("GATEWAY_INTERFACE").is_ok_and(|g| g.starts_with("CGI/"))
}

fn yuzde_coz(m: &str) -> String {
    let b = m.as_bytes();
    let mut c = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |x: u8| (x as char).to_digit(16);
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(u), Some(a)) = (hex(b[i + 1]), hex(b[i + 2])) {
                c.push((u * 16 + a) as u8);
                i += 3;
                continue;
            }
        }
        c.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&c).into_owned()
}

fn html_kacir(m: &str) -> String {
    m.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hata_yaniti(durum: &str, baslik: &str, ayrinti: &str) -> ExitCode {
    print!(
        "Status: {durum}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
         <!DOCTYPE html><html lang=\"tr\"><head><meta charset=\"utf-8\"><title>{b}</title>\
         <style>body{{font-family:system-ui,sans-serif;margin:0;padding:48px 24px;background:#f5f3ee;\
         color:#1b1d21}}main{{max-width:720px;margin:auto}}pre{{white-space:pre-wrap;background:#1b1d21;\
         color:#f5f3ee;padding:16px;border-radius:8px;font-size:13px}}</style></head><body><main>\
         <h1>{b}</h1>{a}<small>Orhunca</small></main></body></html>",
        b = html_kacir(baslik),
        a = if ayrinti.is_empty() {
            String::new()
        } else {
            format!("<pre>{}</pre>", html_kacir(ayrinti))
        }
    );
    ExitCode::SUCCESS
}

/// İstenen `.ohc` dosyası. Projede (`.ohcproj`) her zaman giriş dosyası; değilse adresteki
/// dosya, klasörse içindeki `index.ohc`, ikisi de değilse (yollar için) kökteki `index.ohc`.
fn istenen_dosya(kok: &Path) -> Option<PathBuf> {
    let uri = std::env::var("REQUEST_URI").unwrap_or_else(|_| "/".into());
    let yol = yuzde_coz(uri.split('?').next().unwrap_or("/"));
    // orhunca.cgi'nin bulunduğu klasörün adresi (alt klasöre kurulduysa ör. /site)
    let betik = std::env::var("SCRIPT_NAME").unwrap_or_default();
    let taban = betik.rsplit_once('/').map_or("", |(t, _)| t);
    let goreli = yol
        .strip_prefix(taban)
        .unwrap_or(&yol)
        .trim_start_matches('/');
    // Proje (.ohcproj) bir uygulamadır: bütün adresler giriş dosyasındaki yollara gider,
    // modül dosyaları (modeller/, yollar/) tek başına çalıştırılamaz.
    if let Some(proje) = derleme::proje_dosyasi(kok) {
        if goreli.ends_with(".ohc") && goreli != "index.ohc" {
            return None;
        }
        let giris =
            derleme::proje_ayari(&proje, &["giriş", "giris"]).unwrap_or_else(|| "index.ohc".into());
        let giris = kok.join(giris).canonicalize().ok()?;
        return giris.starts_with(kok).then_some(giris);
    }
    let guvenli = !goreli
        .split('/')
        .any(|p| p == ".." || (p.starts_with('.') && !p.is_empty()));
    let aday = if !guvenli {
        None
    } else if goreli.ends_with(".ohc") {
        Some(kok.join(goreli))
    } else if goreli.is_empty() || kok.join(goreli).is_dir() {
        Some(kok.join(goreli).join("index.ohc"))
    } else {
        None
    };
    // Olmayan bir .ohc dosyası 404'tür; başka adresler index.ohc'deki yollara gider.
    let aday = match aday.filter(|a| a.is_file()) {
        Some(a) => a,
        None if goreli.ends_with(".ohc") => return None,
        None => Some(kok.join("index.ohc")).filter(|a| a.is_file())?,
    };
    let aday = aday.canonicalize().ok()?;
    aday.starts_with(kok).then_some(aday)
}

/// Derleme önbelleğinin anahtarı: Orhunca sürümü ve projedeki kaynak dosyaların adı,
/// boyutu ve değişme zamanı. Bir dosya değişince program yeniden derlenir.
fn onbellek_anahtari(dosya: &Path) -> String {
    fn gez(k: &Path, s: &mut String) {
        let Ok(g) = std::fs::read_dir(k) else { return };
        let mut g: Vec<_> = g.flatten().collect();
        g.sort_by_key(|x| x.file_name());
        for x in g {
            let ad = x.file_name().to_string_lossy().into_owned();
            if ad.starts_with('.') || ad == "veri" || ad == "statik" {
                continue;
            }
            let p = x.path();
            if p.is_dir() {
                gez(&p, s);
            } else if ad.ends_with(".ohc") || ad.ends_with(".ohchtml") || ad.ends_with(".ohcproj") {
                let (boy, zaman) = x.metadata().map_or((0, 0), |m| {
                    let z = m
                        .modified()
                        .ok()
                        .and_then(|z| z.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_nanos());
                    (m.len(), z)
                });
                s.push_str(&format!("{}|{boy}|{zaman}\n", p.display()));
            }
        }
    }
    let mut s = format!("{}\n{}\n", env!("CARGO_PKG_VERSION"), dosya.display());
    gez(&derleme::proje_koku(dosya), &mut s);
    let ozet = crate::guncelleme::sha256(s.as_bytes());
    ozet[..12].iter().map(|b| format!("{b:02x}")).collect()
}

/// Sunucuda `orhunca.cgi` olarak çalışır: istenen `.ohc` dosyasını (gerekirse) derler ve
/// derlenen programı aynı CGI isteğiyle çalıştırır.
pub fn isleyici() -> ExitCode {
    let kok = match std::env::current_dir().and_then(|k| k.canonicalize()) {
        Ok(k) => k,
        Err(e) => return hata_yaniti("500 Internal Server Error", "Sunucu hatası", &e.to_string()),
    };
    derleme::proje_sinirini_koy(kok.clone());
    let Some(dosya) = istenen_dosya(&kok) else {
        return hata_yaniti("404 Not Found", "Sayfa bulunamadı", "");
    };
    let onbellek = kok.join(".orhunca-onbellek");
    let _ = std::fs::create_dir_all(&onbellek);
    let program = onbellek.join(onbellek_anahtari(&dosya));
    if !program.is_file() {
        let gecici = onbellek.join(format!(".derleniyor-{}", std::process::id()));
        if let Err(h) = derleme::derle(&dosya, &gecici, None) {
            let _ = std::fs::remove_file(&gecici);
            let goreli = dosya.strip_prefix(&kok).unwrap_or(&dosya);
            return hata_yaniti(
                "500 Internal Server Error",
                &format!("{} derlenemedi", goreli.display()),
                &h.metin.replace(&kok.display().to_string(), ""),
            );
        }
        if std::fs::rename(&gecici, &program).is_err() {
            let _ = std::fs::remove_file(&gecici);
        }
        // Eski derlemeler bir günden sonra silinir.
        if let Ok(g) = std::fs::read_dir(&onbellek) {
            for x in g.flatten() {
                let eski = x
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|z| z.elapsed().ok())
                    .is_some_and(|s| s.as_secs() > 86_400);
                if eski && x.path() != program {
                    let _ = std::fs::remove_file(x.path());
                }
            }
        }
    }
    calistir(&program, &derleme::proje_koku(&dosya))
}

#[cfg(unix)]
fn calistir(program: &Path, klasor: &Path) -> ExitCode {
    use std::os::unix::process::CommandExt;
    let h = std::process::Command::new(program)
        .current_dir(klasor)
        .exec();
    hata_yaniti(
        "500 Internal Server Error",
        "Program çalıştırılamadı",
        &h.to_string(),
    )
}

#[cfg(not(unix))]
fn calistir(_: &Path, _: &Path) -> ExitCode {
    hata_yaniti(
        "500 Internal Server Error",
        "Desteklenmiyor",
        "orhunca.cgi yalnızca Linux sunucularda çalışır",
    )
}
