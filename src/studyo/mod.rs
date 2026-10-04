//! Orhunca Stüdyo: tarayıcıda (ileride Tauri penceresinde) açılan geliştirme ortamı.
//!
//! `orhunca stüdyo` yalnızca bu bilgisayardan (127.0.0.1) erişilebilen bir sunucu
//! başlatır. Arayüz dosyaları ikili dosyanın içine gömülüdür. Her oturum rastgele
//! bir anahtar üretir; API çağrıları bu anahtarı `X-Orhunca-Anahtar` başlığında
//! taşımak zorundadır, böylece başka web sitelerinin yerel dosyalara ya da
//! derleyiciye erişmesi engellenir.

mod api;
mod calisma;
mod depo;
mod http;
pub mod sablonlar;

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;

mod gomulu {
    include!(concat!(env!("OUT_DIR"), "/studio_dosyalari.rs"));
}

const VARSAYILAN_KAPI: u16 = 7313;

fn rastgele_anahtar() -> String {
    (0..4)
        .map(|i| {
            let mut h = RandomState::new().build_hasher();
            h.write_u64(i ^ std::process::id() as u64);
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// Bilgi satırı yazar; çıktı kapatılmışsa (ör. boru okunmuyorsa) sessizce geçer.
fn bilgi(metin: &str) {
    use std::io::Write;
    let mut cikti = std::io::stdout();
    let _ = writeln!(cikti, "{metin}").and_then(|_| cikti.flush());
}

pub fn tarayicida_ac(adres: &str) {
    let sonuc = if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", "", adres]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(adres).spawn()
    } else {
        Command::new("xdg-open").arg(adres).spawn()
    };
    if sonuc.is_err() {
        bilgi("Tarayıcı açılamadı; adresi kendiniz açın.");
    }
}

pub fn calistir(args: &[String]) -> Result<(), String> {
    let mut kapi = VARSAYILAN_KAPI;
    let mut ac = true;
    let mut acilacak = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--kapı" | "--kapi" | "--port" => {
                i += 1;
                kapi = args
                    .get(i)
                    .and_then(|k| k.parse().ok())
                    .ok_or("--kapı sonrasında bir sayı bekleniyordu")?;
            }
            "--tarayıcı-açma" | "--tarayici-acma" => ac = false,
            a if a.starts_with('-') => return Err(format!("bilinmeyen seçenek '{a}'")),
            a => acilacak = Some(PathBuf::from(a)),
        }
        i += 1;
    }

    let (dinleyici, adres, anahtar) = dinleyici_ac(kapi)?;
    let adres = acilacak_adres(adres, acilacak.as_deref());
    bilgi(&format!("Orhunca Stüdyo çalışıyor: {adres}"));
    bilgi("Kapatmak için Ctrl+C.");
    if ac {
        tarayicida_ac(&adres);
    }
    dinle(dinleyici, anahtar);
    Ok(())
}

/// Stüdyo sunucusunu arka planda başlatır ve arayüzün (anahtarlı) adresini
/// döndürür. Masaüstü uygulaması bu adresi kendi penceresinde açar.
pub fn arka_planda_baslat(kapi: u16) -> Result<String, String> {
    arka_planda_ac(kapi, None)
}

/// `acilacak`: açılışta açılacak proje klasörü ya da dosya (ör. çift tıklanan .ohc).
pub fn arka_planda_ac(kapi: u16, acilacak: Option<&Path>) -> Result<String, String> {
    let (dinleyici, adres, anahtar) = dinleyici_ac(kapi)?;
    std::thread::spawn(move || dinle(dinleyici, anahtar));
    Ok(acilacak_adres(adres, acilacak))
}

/// Arayüz `ac` parametresindeki yolu açar (tam yola çevrilir).
fn acilacak_adres(adres: String, acilacak: Option<&Path>) -> String {
    let Some(yol) = acilacak else { return adres };
    let tam = std::fs::canonicalize(yol).unwrap_or_else(|_| yol.to_path_buf());
    let tam = tam.display().to_string();
    let tam = tam.strip_prefix(r"\\?\").unwrap_or(&tam).to_string();
    let mut kodlu = String::new();
    for b in tam.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~/".contains(&b) {
            kodlu.push(b as char);
        } else {
            kodlu.push_str(&format!("%{b:02X}"));
        }
    }
    format!("{adres}&ac={kodlu}")
}

/// Çalıştırılan programları (ör. web sunucularını) durdurur; uygulama kapanırken çağrılır.
pub fn kapat() {
    calisma::hepsini_durdur();
}

fn dinleyici_ac(kapi: u16) -> Result<(TcpListener, String, String), String> {
    let dinleyici = TcpListener::bind(("127.0.0.1", kapi))
        .or_else(|_| TcpListener::bind(("127.0.0.1", 0)))
        .map_err(|e| format!("sunucu başlatılamadı: {e}"))?;
    let kapi = dinleyici.local_addr().map_err(|e| e.to_string())?.port();
    let anahtar = rastgele_anahtar();
    let adres = format!("http://127.0.0.1:{kapi}/?anahtar={anahtar}");
    Ok((dinleyici, adres, anahtar))
}

fn dinle(dinleyici: TcpListener, anahtar: String) {
    let Ok(kapi) = dinleyici.local_addr().map(|a| a.port()) else {
        return;
    };
    for baglanti in dinleyici.incoming() {
        let Ok(akis) = baglanti else { continue };
        let anahtar = anahtar.clone();
        std::thread::spawn(move || isle(akis, &anahtar, kapi));
    }
}

fn isle(mut akis: TcpStream, anahtar: &str, kapi: u16) {
    let Some(istek) = http::oku(&mut akis) else {
        return;
    };

    // DNS yeniden bağlama saldırılarına karşı yalnızca yerel adlar kabul edilir.
    let host = istek.baslik("host");
    if host != format!("127.0.0.1:{kapi}") && host != format!("localhost:{kapi}") {
        http::yaz(&mut akis, &http::Yanit::hata(403, "geçersiz adres"));
        return;
    }

    let yanit = if istek.yol.starts_with("/api/") {
        if istek.baslik("x-orhunca-anahtar") != anahtar {
            http::Yanit::hata(403, "oturum anahtarı geçersiz; Stüdyo'yu yeniden açın")
        } else if istek.yol == "/api/kapat" {
            http::yaz(
                &mut akis,
                &http::Yanit::json(&serde_json::json!({ "tamam": true })),
            );
            calisma::hepsini_durdur();
            bilgi("Stüdyo kapatıldı.");
            std::process::exit(0);
        } else {
            api::yonlendir(&istek)
        }
    } else if let Some(kimlik) = istek.yol.strip_prefix("/onizleme/") {
        // Arayüz programlarının derlenmiş sayfası (Stüdyo'da yalıtılmış çerçevede açılır).
        api::onizleme(kimlik.trim_end_matches('/'))
    } else {
        statik(&istek.yol)
    };
    http::yaz(&mut akis, &yanit);
}

fn statik(yol: &str) -> http::Yanit {
    let ad = match yol.trim_start_matches('/') {
        "" => "index.html",
        a => a,
    };
    match gomulu::DOSYALAR.iter().find(|(a, _)| *a == ad) {
        Some((a, icerik)) => http::Yanit {
            durum: 200,
            tur: http::icerik_turu(a),
            govde: icerik.to_vec(),
        },
        None => http::Yanit::hata(404, "bulunamadı"),
    }
}
