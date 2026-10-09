//! Stüdyo arayüzünün çağırdığı JSON uç noktaları.

use super::http::{Istek, Yanit};
use super::{asistan, calisma, depo, gecmis, git, sablonlar, temalar};
use crate::{agac, derleme};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Dosya işlemlerine izin verilen klasörler: bu oturumda açılan ya da oluşturulan projeler.
fn izinli_kokler() -> &'static Mutex<Vec<PathBuf>> {
    static K: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    K.get_or_init(Default::default)
}

fn koke_izin_ver(yol: &Path) {
    if let Ok(tam) = std::fs::canonicalize(yol) {
        let mut k = izinli_kokler().lock().unwrap();
        if !k.contains(&tam) {
            k.push(tam);
        }
    }
}

/// Yol açık projelerden birinin içinde mi? Henüz var olmayan dosyalar için üst klasöre bakılır.
fn izinli_mi(yol: &Path) -> bool {
    // `..` reddedilir: var olmayan bir klasörden sonra gelen `..` aşağıdaki denetimi
    // aşabilir (Windows yolu sözcüksel olarak çözer: izinli/yok/../../başka).
    if yol
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }
    let mut aday = yol.to_path_buf();
    let tam = loop {
        if let Ok(t) = std::fs::canonicalize(&aday) {
            break t;
        }
        match aday.parent() {
            Some(u) if u != aday => aday = u.to_path_buf(),
            _ => return false,
        }
    };
    izinli_kokler()
        .lock()
        .unwrap()
        .iter()
        .any(|k| tam.starts_with(k))
}

fn govde(istek: &Istek) -> Value {
    serde_json::from_slice(&istek.govde).unwrap_or(Value::Null)
}

fn metin<'a>(v: &'a Value, ad: &str) -> &'a str {
    v[ad].as_str().unwrap_or("")
}

fn hata(mesaj: impl Into<String>) -> Yanit {
    Yanit::json(&json!({ "hata": mesaj.into() }))
}

pub fn yonlendir(istek: &Istek) -> Yanit {
    let g = govde(istek);
    match (istek.yontem.as_str(), istek.yol.as_str()) {
        ("GET", "/api/durum") => durum(),
        ("GET", "/api/guncelleme") => guncelleme_denetle(),
        ("GET", "/api/asistan") => Yanit::json(&asistan::durum()),
        ("POST", "/api/asistan/ayar") => match asistan::ayar_kaydet(&g) {
            Ok(d) => Yanit::json(&d),
            Err(e) => hata(e),
        },
        ("GET", "/api/asistan/modeller") => match asistan::modeller() {
            Ok(d) => Yanit::json(&d),
            Err(e) => hata(e),
        },
        ("POST", "/api/asistan/durdur") => {
            asistan::durdur();
            Yanit::json(&json!({ "tamam": true }))
        }
        ("POST", "/api/asistan/sor") => match asistan::sor(
            &g,
            g["proje"]
                .as_str()
                .is_some_and(|p| !depo::guvenilir_mi(Path::new(p))),
        ) {
            Ok(d) => Yanit::json(&d),
            Err(e) => hata(e),
        },
        ("GET", "/api/ajan") => Yanit::json(&json!({
            "komut": asistan::mcp_komutu(),
            "talimat": asistan::ajan_talimati(),
        })),
        ("GET", "/api/temalar") => Yanit::json(&temalar::liste()),
        ("GET", "/api/tema") => match temalar::oku(istek.sorgu("kimlik")) {
            Ok(t) => Yanit::json(&json!({ "tema": t })),
            Err(e) => hata(e),
        },
        ("POST", "/api/tema/kaydet") => match temalar::kaydet(g["kimlik"].as_str(), &g["tema"]) {
            Ok(k) => Yanit::json(&json!({ "kimlik": k })),
            Err(e) => hata(e),
        },
        ("POST", "/api/tema/sil") => match temalar::sil(metin(&g, "kimlik")) {
            Ok(()) => Yanit::json(&json!({ "tamam": true })),
            Err(e) => hata(e),
        },
        ("POST", "/api/tema/disa_aktar") => match temalar::disa_aktar(&g["tema"]) {
            Ok(yol) => Yanit::json(&json!({ "yol": yol })),
            Err(e) => hata(e),
        },
        ("GET", "/api/tema/galeri") => match temalar::galeri() {
            Ok(d) => Yanit::json(&d),
            Err(e) => hata(e),
        },
        ("POST", "/api/tema/galeriden") => match temalar::galeriden(metin(&g, "dosya")) {
            Ok(t) => Yanit::json(&json!({ "tema": t })),
            Err(e) => hata(e),
        },
        ("POST", "/api/guncelleme/kur") => guncelleme_kur(g["masaustu"].as_bool() == Some(true)),
        ("GET", "/api/projeler") => projeler(),
        ("GET", "/api/sablonlar") => sablon_listesi(),
        ("GET", "/api/yerlesikler") => yerlesikler(),
        ("GET", "/api/ekler") => ek_onerileri(istek.sorgu("ifade")),
        ("POST", "/api/ders/hazirla") => ders_hazirla(&g),
        ("POST", "/api/ders/denetle") => ders_denetle(&g),
        ("GET", "/api/klasor") => klasor(istek.sorgu("yol")),
        ("GET", "/api/dosya") => dosya_oku(istek.sorgu("yol")),
        ("GET", "/api/agac") => agac(istek.sorgu("kok")),
        ("GET", "/api/ara") => ara(
            istek.sorgu("kok"),
            istek.sorgu("metin"),
            istek.sorgu("tam") == "1",
        ),
        ("GET", "/api/cikti") => cikti(istek.sorgu("kimlik"), istek.sorgu("konum")),
        ("POST", "/api/proje/olustur") => proje_olustur(&g),
        ("POST", "/api/proje/ac") => proje_ac(metin(&g, "yol")),
        ("POST", "/api/proje/klonla") => proje_klonla(metin(&g, "url"), metin(&g, "konum")),
        ("GET", "/api/git/durum") => git_islemi(istek.sorgu("kok"), |k| {
            git::durum(k).map(|mut d| {
                if d["depo"] == true {
                    d["gecmis"] = git::gecmis(k);
                }
                d
            })
        }),
        ("GET", "/api/git/fark") => git_islemi(istek.sorgu("kok"), |k| {
            git::fark(k, istek.sorgu("yol"), istek.sorgu("hazir") == "1")
                .map(|f| json!({ "fark": f }))
        }),
        ("POST", "/api/git/hazirla") => git_islemi(metin(&g, "kok"), |k| {
            let yollar: Vec<String> = g["yollar"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|y| y.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            git::hazirla(k, &yollar, g["geri"] == true).map(|_| json!({ "tamam": true }))
        }),
        ("POST", "/api/git/at") => git_islemi(metin(&g, "kok"), |k| {
            git::degisikligi_at(k, metin(&g, "yol"), g["takipsiz"] == true)
                .map(|_| json!({ "tamam": true }))
        }),
        ("POST", "/api/git/isle") => git_islemi(metin(&g, "kok"), |k| {
            git::isle(k, metin(&g, "mesaj")).map(|m| json!({ "mesaj": m }))
        }),
        ("POST", "/api/git/gonder") => git_islemi(metin(&g, "kok"), |k| {
            git::gonder(k).map(|m| json!({ "mesaj": m }))
        }),
        ("POST", "/api/git/cek") => git_islemi(metin(&g, "kok"), |k| {
            git::cek(k).map(|m| json!({ "mesaj": m }))
        }),
        ("POST", "/api/git/baslat") => git_islemi(metin(&g, "kok"), |k| {
            git::baslat(k).map(|_| json!({ "tamam": true }))
        }),
        ("POST", "/api/basvurular") => basvurular(&g),
        ("POST", "/api/adlandir") => adlandir(&g),
        ("GET", "/api/sinamalar") => {
            let kok = PathBuf::from(istek.sorgu("kok"));
            if !izinli_mi(&kok) {
                return Yanit::hata(403, "bu klasöre erişim yok");
            }
            Yanit::json(&crate::sinama::listele(&kok))
        }
        ("POST", "/api/sina") => sina(&g),
        ("POST", "/api/proje/guven") => {
            let yol = PathBuf::from(metin(&g, "yol"));
            if !izinli_mi(&yol) {
                return Yanit::hata(403, "bu klasöre erişim yok");
            }
            depo::guven(&yol, g["guven"].as_bool() != Some(false));
            Yanit::json(&json!({ "guvenilir": depo::guvenilir_mi(&yol) }))
        }
        ("POST", "/api/proje/unut") => {
            depo::proje_unut(metin(&g, "yol"));
            Yanit::json(&json!({ "tamam": true }))
        }
        ("GET", "/api/gecmis") => {
            let p = Path::new(istek.sorgu("yol"));
            if !izinli_mi(p) {
                return Yanit::hata(403, "bu dosyaya erişim yok");
            }
            let l: Vec<Value> = gecmis::liste(p)
                .into_iter()
                .map(|(z, b)| json!({ "zaman": z.to_string(), "boyut": b }))
                .collect();
            Yanit::json(&json!({ "kayitlar": l }))
        }
        ("GET", "/api/gecmis/oku") => {
            let p = Path::new(istek.sorgu("yol"));
            if !izinli_mi(p) {
                return Yanit::hata(403, "bu dosyaya erişim yok");
            }
            match istek
                .sorgu("zaman")
                .parse::<u128>()
                .ok()
                .and_then(|z| gecmis::oku(p, z))
            {
                Some(m) => Yanit::json(&json!({ "icerik": m })),
                None => hata("Bu kayıt bulunamadı."),
            }
        }
        ("POST", "/api/dosya") => dosya_yaz(metin(&g, "yol"), metin(&g, "icerik")),
        ("POST", "/api/degistir") => degistir(
            metin(&g, "kok"),
            metin(&g, "aranan"),
            metin(&g, "yeni"),
            g["tamKelime"].as_bool() == Some(true),
        ),
        ("POST", "/api/dosya/yeni") => {
            dosya_yeni(metin(&g, "yol"), g["klasor"].as_bool() == Some(true))
        }
        ("POST", "/api/denetle") => denetle(metin(&g, "dosya"), &g["acik"]),
        ("POST", "/api/cevir") => cevir(metin(&g, "dosya"), metin(&g, "icerik"), metin(&g, "dil")),
        ("POST", "/api/calistir") => calistir(&g),
        ("POST", "/api/tarayicida_ac") => {
            // Yalnızca bu bilgisayardaki sunucuların adresleri (canlı önizleme) ve yapay zekâ
            // sağlayıcılarının anahtar/kurulum sayfaları açılır.
            let adres = metin(&g, "adres");
            if !adres.starts_with("http://localhost:")
                && !adres.starts_with("http://127.0.0.1:")
                && !asistan::sayfa_mi(adres)
            {
                return Yanit::hata(403, "yalnızca yerel adresler açılabilir");
            }
            super::tarayicida_ac(adres);
            Yanit::json(&json!({ "tamam": true }))
        }
        ("POST", "/api/girdi") => girdi(&g),
        ("POST", "/api/ayikla") => ayikla(&g),
        ("POST", "/api/durdur") => {
            calisma::durdur(g["kimlik"].as_u64().unwrap_or(0));
            Yanit::json(&json!({ "tamam": true }))
        }
        ("POST", "/api/derle") => derle(metin(&g, "dosya"), metin(&g, "hedef")),
        ("GET", "/api/paket/liste") => paket_listesi(istek.sorgu("kok")),
        ("GET", "/api/paket/dizin") => match crate::paket::dizin() {
            Ok(l) => Yanit::json(&json!({ "paketler": l.into_iter().map(|p| json!({
                "ad": p.ad, "aciklama": p.aciklama, "kaynak": p.kaynak, "surum": p.surum,
                "sahip": p.sahip, "izinler": p.izinler,
            })).collect::<Vec<_>>() })),
            Err(e) => hata(e),
        },
        ("POST", "/api/paket/ekle") => paket_islemi(metin(&g, "kok"), true, |k| {
            crate::paket::ekle(
                k,
                metin(&g, "kaynak").trim(),
                None,
                g["izinVer"].as_bool() == Some(true),
            )
        }),
        ("POST", "/api/paket/yukle") => paket_islemi(metin(&g, "kok"), true, |k| {
            crate::paket::yukle(
                k,
                g["guncelle"].as_bool() == Some(true),
                g["izinVer"].as_bool() == Some(true),
            )
        }),
        ("POST", "/api/paket/kaldir") => paket_islemi(metin(&g, "kok"), false, |k| {
            crate::paket::kaldir(k, metin(&g, "ad"))
        }),
        ("POST", "/api/bicimlendir") => Yanit::json(&json!({
            "icerik": crate::bicimlendirici::bicimlendir(metin(&g, "icerik"))
        })),
        _ => Yanit::hata(404, "bilinmeyen uç nokta"),
    }
}

fn kullanici_adi() -> String {
    std::env::var(if cfg!(windows) { "USERNAME" } else { "USER" })
        .ok()
        .filter(|a| !a.is_empty())
        .or_else(|| {
            depo::ev_klasoru()
                .file_name()
                .map(|a| a.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "geliştirici".into())
}

fn isletim() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn durum() -> Yanit {
    let konum = depo::ev_klasoru().join("Orhunca").join("Projeler");
    Yanit::json(&json!({
        "kullanici": kullanici_adi(),
        "surum": env!("CARGO_PKG_VERSION"),
        "isletim": isletim(),
        "ayrac": std::path::MAIN_SEPARATOR.to_string(),
        "ev": depo::ev_klasoru().to_string_lossy(),
        "varsayilan_konum": konum.to_string_lossy(),
        "mingw": windows_baglayici_var(),
    }))
}

/// Yeni sürüm var mı? (Yönetici `ORHUNCA_GUNCELLEME=kapali` ile kapatabilir.)
fn guncelleme_denetle() -> Yanit {
    use crate::guncelleme as g;
    if g::kapali_mi() {
        return Yanit::json(&json!({ "kapali": true }));
    }
    match g::son_surum() {
        Ok(s) => Yanit::json(&json!({
            "yeni": g::daha_yeni(&s.surum, crate::SURUM),
            "surum": s.surum,
            "simdiki": crate::SURUM,
            "notlar": s.notlar,
            "sayfa": s.sayfa,
        })),
        Err(e) => hata(e),
    }
}

/// Uygun dosyayı indirir, doğrular ve kurar. Kurulum başladıysa Stüdyo kapanır.
fn guncelleme_kur(masaustu: bool) -> Yanit {
    use crate::guncelleme as g;
    if g::kapali_mi() {
        return hata("güncellemeler yönetici tarafından kapatılmış");
    }
    let sonuc = g::son_surum().and_then(|s| {
        let k = g::kurulum_turu(masaustu);
        let dosya = g::indir(&s, &g::varlik_adi(&k))?;
        g::kur(&k, &dosya)
    });
    match sonuc {
        Ok((mesaj, kapan)) => {
            if kapan {
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(1500));
                    calisma::hepsini_durdur();
                    std::process::exit(0);
                });
            }
            Yanit::json(&json!({ "mesaj": mesaj, "kapaniyor": kapan }))
        }
        Err(e) => hata(e),
    }
}

fn windows_baglayici_var() -> bool {
    if cfg!(windows) {
        return true;
    }
    crate::komut("x86_64-w64-mingw32-gcc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn projeler() -> Yanit {
    let d = depo::oku();
    let projeler: Vec<Value> = d["projeler"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|mut p| {
            let var = Path::new(p["yol"].as_str().unwrap_or("")).is_dir();
            p["var"] = json!(var);
            if let Some(s) = sablonlar::bul(p["sablon"].as_str().unwrap_or("")) {
                p["sablon_adi"] = json!(s.ad);
                p["simge"] = json!(s.simge);
            }
            p
        })
        .collect();
    Yanit::json(&json!({ "projeler": projeler, "son_sablonlar": d["son_sablonlar"] }))
}

fn sablon_listesi() -> Yanit {
    let liste: Vec<Value> = sablonlar::SABLONLAR
        .iter()
        .map(|s| {
            json!({
                "kimlik": s.kimlik, "ad": s.ad, "aciklama": s.aciklama, "simge": s.simge,
                "kategoriler": s.kategoriler, "etiketler": s.etiketler,
                "yakinda": s.yakinda, "dosyalar": s.dosyalar, "giris": s.giris,
                "web": sablonlar::web_mi(s),
            })
        })
        .collect();
    Yanit::json(&json!({ "sablonlar": liste }))
}

fn yerlesikler() -> Yanit {
    // Başvuru listesi: yerleşik işlevler ve sınama işlevleri, bölümleriyle.
    Yanit::json(&json!({ "yerlesikler": crate::basvuru::json()["islevler"] }))
}

/// Dersler projesi (`~/Orhunca/Dersler`): alıştırmanın dosyası yoksa başlangıç koduyla
/// oluşturulur; proje bilgisi ve dosyanın göreli yolu döner.
fn ders_hazirla(g: &Value) -> Yanit {
    let kok = depo::ev_klasoru().join("Orhunca").join("Dersler");
    if let Err(e) = std::fs::create_dir_all(&kok) {
        return hata(format!("Dersler klasörü oluşturulamadı: {e}"));
    }
    let proje = kok.join("dersler.ohcproj");
    if !proje.exists() {
        let _ = std::fs::write(
            &proje,
            "# Orhunca dersleri: alıştırma dosyaları\nad = \"dersler\"\nsürüm = \"1.0.0\"\n",
        );
    }
    let ad: String = metin(g, "dosya")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if ad.is_empty() {
        return hata("dosya adı gerekli");
    }
    let dosya = kok.join(format!("{ad}.ohc"));
    if !dosya.exists() {
        if let Err(e) = std::fs::write(&dosya, metin(g, "baslangic")) {
            return hata(format!("alıştırma dosyası yazılamadı: {e}"));
        }
    }
    koke_izin_ver(&kok);
    depo::guven(&kok, true);
    Yanit::json(&json!({ "proje": proje_bilgisi(&kok), "dosya": format!("{ad}.ohc") }))
}

/// Alıştırmayı denetler: program verilen girdiyle çalıştırılır ve çıktısı beklenenle
/// karşılaştırılır (beklenen yoksa yalnızca derlenebilmesi yeterlidir).
fn ders_denetle(g: &Value) -> Yanit {
    let dosya = PathBuf::from(metin(g, "dosya"));
    if !izinli_mi(&dosya) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    let beklenen = g["beklenen"].as_str();
    let Some(beklenen) = beklenen else {
        return match derleme::yukle(&dosya) {
            Ok(_) => Yanit::json(&json!({ "basarili": true })),
            Err(h) => Yanit::json(&json!({ "basarili": false, "derleme_hatasi": h.metin })),
        };
    };
    let gecici = match derleme::gecici_klasor("ders") {
        Ok(k) => k,
        Err(e) => return hata(e),
    };
    let program = gecici.join(if cfg!(windows) { "p.exe" } else { "p" });
    if let Err(h) = derleme::derle(&dosya, &program, None) {
        let _ = std::fs::remove_dir_all(&gecici);
        return Yanit::json(&json!({ "basarili": false, "derleme_hatasi": h.metin }));
    }
    let sonuc = sinirli_calistir(
        &program,
        &gecici,
        metin(g, "girdi"),
        Duration::from_secs(10),
    );
    let _ = std::fs::remove_dir_all(&gecici);
    match sonuc {
        Ok((cikti, hata_ciktisi)) => {
            let duz = |m: &str| m.replace("\r\n", "\n").trim_end().to_string();
            Yanit::json(&json!({
                "basarili": duz(&cikti) == duz(beklenen) && hata_ciktisi.is_empty(),
                "cikti": cikti,
                "hata": hata_ciktisi,
            }))
        }
        Err(e) => Yanit::json(&json!({ "basarili": false, "hata": e })),
    }
}

/// Programı girdiyle çalıştırır; süre aşılırsa durdurur.
fn sinirli_calistir(
    program: &Path,
    klasor: &Path,
    girdi: &str,
    sure: Duration,
) -> Result<(String, String), String> {
    use std::io::{Read, Write};
    let mut c = crate::komut(program)
        .current_dir(klasor)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("program başlatılamadı: {e}"))?;
    if let Some(mut s) = c.stdin.take() {
        let _ = s.write_all(girdi.as_bytes());
    }
    let mut cikis = c.stdout.take().unwrap();
    let mut hatalar = c.stderr.take().unwrap();
    let o1 = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = cikis.read_to_string(&mut s);
        s
    });
    let o2 = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = hatalar.read_to_string(&mut s);
        s
    });
    let bas = Instant::now();
    loop {
        match c.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if bas.elapsed() < sure => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = c.kill();
                let _ = c.wait();
                return Err("program 10 saniyede bitmedi (sonsuz döngü olabilir)".into());
            }
        }
    }
    Ok((o1.join().unwrap_or_default(), o2.join().unwrap_or_default()))
}

/// Düzenleyicide `'` yazılınca: ifadenin her hâldeki doğru eki (ünlü uyumuyla).
fn ek_onerileri(ifade: &str) -> Yanit {
    use crate::ekler::Hal;
    let liste: Vec<Value> = [
        (Hal::Belirtme, "belirtme · nesne", "'yi yaz, 'yi sırala"),
        (Hal::Yonelme, "yönelme · hedef", "listeye ekle, 5'e kadar"),
        (Hal::Ayrilma, "ayrılma · kaynak", "1'den, listeden"),
        (Hal::Bulunma, "bulunma · yer", "listede"),
        (Hal::Vasita, "vasıta · araç", "x'le"),
        (Hal::Ilgi, "ilgi · sahiplik", "listenin"),
    ]
    .iter()
    .map(|(hal, ad, ornek)| {
        json!({ "ek": crate::bicimlendirici::ek_oner(ifade, *hal), "hal": ad, "ornek": ornek })
    })
    .collect();
    Yanit::json(&json!({ "ekler": liste }))
}

/// Klasör seçici: verilen klasördeki alt klasörleri listeler.
fn klasor(yol: &str) -> Yanit {
    let yol = if yol.is_empty() {
        depo::ev_klasoru()
    } else {
        PathBuf::from(yol)
    };
    let okunan = match std::fs::read_dir(&yol) {
        Ok(o) => o,
        Err(e) => return hata(format!("'{}' açılamadı: {e}", yol.display())),
    };
    let mut klasorler: Vec<Value> = okunan
        .filter_map(|g| g.ok())
        .filter(|g| g.path().is_dir())
        .filter(|g| !g.file_name().to_string_lossy().starts_with('.'))
        .map(|g| {
            let p = g.path();
            json!({
                "ad": g.file_name().to_string_lossy(),
                "yol": p.to_string_lossy(),
                "proje": derleme::proje_dosyasi(&p).is_some(),
            })
        })
        .collect();
    klasorler.sort_by_key(|k| k["ad"].as_str().unwrap_or("").to_lowercase());
    Yanit::json(&json!({
        "yol": yol.to_string_lossy(),
        "ust": yol.parent().map(|u| u.to_string_lossy().into_owned()),
        "klasorler": klasorler,
        "proje": derleme::proje_dosyasi(&yol).is_some(),
    }))
}

fn gecerli_ad(ad: &str) -> Result<(), String> {
    if ad.trim().is_empty() {
        return Err("Proje adı gerekli.".into());
    }
    if !ad
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Yalnızca harf, rakam, alt çizgi (_) ve tire (-) kullanılabilir.".into());
    }
    Ok(())
}

fn proje_olustur(g: &Value) -> Yanit {
    let Some(sablon) = sablonlar::bul(metin(g, "sablon")) else {
        return hata("bilinmeyen şablon");
    };
    if let Some(asama) = sablon.yakinda {
        return hata(format!("{} şablonu {asama}'da gelecek.", sablon.ad));
    }
    let ad = metin(g, "ad").trim();
    if let Err(e) = gecerli_ad(ad) {
        return hata(e);
    }
    let kok = PathBuf::from(metin(g, "konum")).join(ad);
    if kok.exists() {
        return hata("Bu konumda aynı adlı bir klasör zaten var.");
    }
    let ornek = g["ornek"].as_bool() != Some(false);
    for dosya in sablon.dosyalar {
        let goreli = dosya.replace("{ad}", ad);
        let yol = kok.join(&goreli);
        if let Some(u) = yol.parent() {
            if let Err(e) = std::fs::create_dir_all(u) {
                return hata(format!("klasör oluşturulamadı: {e}"));
            }
        }
        if let Err(e) = std::fs::write(&yol, sablonlar::icerik(sablon, dosya, ad, ornek)) {
            return hata(format!("'{goreli}' yazılamadı: {e}"));
        }
    }
    let mut uyari = None;
    if g["git"].as_bool() == Some(true) {
        let mut yoksay = String::from(
            "# Derleme çıktıları\n/cikti/\n*.exe\n\n# Gizli ayarlar (API anahtarları, şifreler)\n.env\n.env.sunucu\n",
        );
        if sablonlar::web_mi(sablon) {
            yoksay.push_str("\n# Model kayıtları (yerel deneme verisi)\n/veri/\n");
        }
        let _ = std::fs::write(kok.join(".gitignore"), yoksay);
        // Türkçe dal adı; eski Git sürümleri --initial-branch bilmez.
        let git = crate::komut("git")
            .args(["init", "-q", "--initial-branch=ana"])
            .current_dir(&kok)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .or_else(|| {
                crate::komut("git")
                    .args(["init", "-q"])
                    .current_dir(&kok)
                    .output()
                    .ok()
            });
        if !git.map(|o| o.status.success()).unwrap_or(false) {
            uyari = Some("Git bulunamadı; depo başlatılamadı.");
        }
    }
    depo::sablon_kullanildi(sablon.kimlik);
    koke_izin_ver(&kok);
    // Stüdyo'da oluşturulan proje kullanıcının kendi projesidir.
    depo::guven(&kok, true);
    let mut y = proje_bilgisi(&kok);
    y["uyari"] = json!(uyari);
    Yanit::json(&y)
}

/// Açılan proje hakkında arayüzün ihtiyaç duyduğu bilgiler; son projelere eklenir.
fn proje_bilgisi(kok: &Path) -> Value {
    let proje_dosyasi = derleme::proje_dosyasi(kok);
    let ad = proje_dosyasi
        .as_ref()
        .and_then(|p| derleme::proje_ayari(p, &["ad"]))
        .or_else(|| kok.file_name().map(|a| a.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let sablon = proje_dosyasi
        .as_ref()
        .and_then(|p| derleme::proje_ayari(p, &["şablon", "sablon"]))
        .unwrap_or_else(|| "konsol".into());
    let giris = proje_dosyasi
        .as_ref()
        .and_then(|p| derleme::proje_ayari(p, &["giriş", "giris"]));
    let dal = std::fs::read_to_string(kok.join(".git").join("HEAD"))
        .ok()
        .and_then(|h| {
            h.trim()
                .strip_prefix("ref: refs/heads/")
                .map(str::to_string)
        });
    // Web projesi: web şablonundan ya da görünüm/statik klasörü olan proje.
    let web = sablonlar::bul(&sablon).is_some_and(sablonlar::web_mi)
        || kok.join("görünümler").is_dir()
        || kok.join("statik").is_dir();
    let yol = kok.to_string_lossy().into_owned();
    depo::proje_acildi(&ad, &yol, &sablon);
    json!({
        "ad": ad, "yol": yol, "sablon": sablon, "giris": giris, "dal": dal, "web": web,
        "guvenilir": depo::guvenilir_mi(kok),
        "proje_dosyasi": proje_dosyasi.map(|p| p.to_string_lossy().into_owned()),
    })
}

fn proje_ac(yol: &str) -> Yanit {
    let mut kok = PathBuf::from(yol);
    if kok.is_file() {
        // Dosyanın projesi (.ohcproj bulunan üst klasör), yoksa dosyanın klasörü
        kok = derleme::proje_koku(&kok);
    }
    if !kok.is_dir() {
        return hata(format!("'{yol}' bulunamadı."));
    }
    koke_izin_ver(&kok);
    Yanit::json(&proje_bilgisi(&kok))
}

fn proje_klonla(url: &str, konum: &str) -> Yanit {
    let url = url.trim();
    if url.is_empty() || url.starts_with('-') {
        return hata("Geçerli bir depo adresi girin.");
    }
    let ad = url
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("proje")
        .trim_end_matches(".git")
        .to_string();
    let hedef = PathBuf::from(konum).join(&ad);
    if hedef.exists() {
        return hata(format!("'{}' zaten var.", hedef.display()));
    }
    let _ = std::fs::create_dir_all(konum);
    let sonuc = crate::komut("git")
        .args(["clone", "--depth", "1", "--", url])
        .arg(&hedef)
        .output();
    match sonuc {
        Ok(o) if o.status.success() => {
            koke_izin_ver(&hedef);
            Yanit::json(&proje_bilgisi(&hedef))
        }
        Ok(o) => hata(format!(
            "Depo klonlanamadı:\n{}",
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(_) => hata("Git bulunamadı; depoyu klonlamak için Git kurun."),
    }
}

/// Proje ağacı: gizli klasörler ve derleme çıktıları gösterilmez.
fn agac(kok: &str) -> Yanit {
    let kok = PathBuf::from(kok);
    if !izinli_mi(&kok) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    fn gez(kok: &Path, klasor: &Path, cikti: &mut Vec<Value>, derinlik: usize) {
        if derinlik > 8 || cikti.len() > 2000 {
            return;
        }
        let Ok(okunan) = std::fs::read_dir(klasor) else {
            return;
        };
        let mut girdiler: Vec<_> = okunan.filter_map(|g| g.ok()).collect();
        // Önce klasörler, sonra dosyalar; her grup kendi içinde ada göre
        girdiler.sort_by_key(|g| {
            (
                !g.path().is_dir(),
                g.file_name().to_string_lossy().to_lowercase(),
            )
        });
        for g in girdiler {
            let ad = g.file_name().to_string_lossy().into_owned();
            // Gizli dosyalar gösterilmez; yalnızca .env (ve .env.örnek) düzenlenebilsin diye görünür.
            if (ad.starts_with('.') && !ad.starts_with(".env")) || ad == "cikti" || ad == "target" {
                continue;
            }
            let p = g.path();
            let goreli = p
                .strip_prefix(kok)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let klasor_mu = p.is_dir();
            cikti.push(json!({ "yol": goreli, "klasor": klasor_mu }));
            if klasor_mu {
                gez(kok, &p, cikti, derinlik + 1);
            }
        }
    }
    let mut girdiler = Vec::new();
    gez(&kok, &kok, &mut girdiler, 0);
    Yanit::json(&json!({ "girdiler": girdiler }))
}

fn dosya_oku(yol: &str) -> Yanit {
    let p = Path::new(yol);
    if !izinli_mi(p) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    match std::fs::read(p) {
        Ok(b) if b.len() > 2 * 1024 * 1024 => hata("Dosya düzenleyicide açılamayacak kadar büyük."),
        Ok(b) => match String::from_utf8(b) {
            Ok(m) => Yanit::json(&json!({ "icerik": m })),
            Err(_) => Yanit::json(&json!({ "ikili": true })),
        },
        Err(e) => hata(format!("Dosya okunamadı: {e}")),
    }
}

fn dosya_yaz(yol: &str, icerik: &str) -> Yanit {
    let p = Path::new(yol);
    if !izinli_mi(p) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    gecmis::ilk_hali_sakla(p);
    match std::fs::write(p, icerik) {
        Ok(()) => {
            gecmis::kaydet(p, icerik);
            Yanit::json(&json!({ "tamam": true }))
        }
        Err(e) => hata(format!("Kaydedilemedi: {e}")),
    }
}

fn dosya_yeni(yol: &str, klasor: bool) -> Yanit {
    let p = Path::new(yol);
    if !izinli_mi(p) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    if p.exists() {
        return hata("Bu adla bir dosya zaten var.");
    }
    let sonuc = if klasor {
        std::fs::create_dir_all(p)
    } else {
        p.parent()
            .map(std::fs::create_dir_all)
            .unwrap_or(Ok(()))
            .and_then(|_| std::fs::write(p, ""))
    };
    match sonuc {
        Ok(()) => Yanit::json(&json!({ "tamam": true })),
        Err(e) => hata(format!("Oluşturulamadı: {e}")),
    }
}

/// Türkçeye uygun küçük harf: I → ı, İ → i. Karakter sayısını korur (eşleşme yerleri
/// özgün metne aynen taşınabilsin diye).
fn kucuk_harf(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

fn kelime_harfi(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `aranan`ın satırdaki yerleri (karakter sırası olarak), büyük/küçük harf ayrımı yapmadan.
fn eslesmeler(satir: &[char], aranan: &[char], tam_kelime: bool) -> Vec<usize> {
    let mut yerler = Vec::new();
    if aranan.is_empty() || satir.len() < aranan.len() {
        return yerler;
    }
    let mut i = 0;
    while i + aranan.len() <= satir.len() {
        let uyar = satir[i..i + aranan.len()]
            .iter()
            .zip(aranan)
            .all(|(a, b)| kucuk_harf(*a) == *b);
        let sinirda = !tam_kelime
            || ((i == 0 || !kelime_harfi(satir[i - 1]))
                && satir
                    .get(i + aranan.len())
                    .is_none_or(|c| !kelime_harfi(*c)));
        if uyar && sinirda {
            yerler.push(i);
            i += aranan.len();
        } else {
            i += 1;
        }
    }
    yerler
}

/// Metindeki bütün eşleşmeleri `yeni` ile değiştirir; değişen metin ve değişiklik sayısı.
fn metinde_degistir(icerik: &str, aranan: &str, yeni: &str, tam_kelime: bool) -> (String, usize) {
    let aranan: Vec<char> = aranan.chars().map(kucuk_harf).collect();
    let mut sonuc = String::with_capacity(icerik.len());
    let mut sayi = 0;
    for parca in icerik.split_inclusive('\n') {
        let k: Vec<char> = parca.chars().collect();
        let yerler = eslesmeler(&k, &aranan, tam_kelime);
        let mut onceki = 0;
        for y in &yerler {
            sonuc.extend(&k[onceki..*y]);
            sonuc.push_str(yeni);
            onceki = y + aranan.len();
        }
        sonuc.extend(&k[onceki..]);
        sayi += yerler.len();
    }
    (sonuc, sayi)
}

/// Projede aranacak dosyalar: gizli dosyalar, derleme çıktıları ve indirilen paketler hariç.
fn proje_dosyalari(kok: &Path) -> Vec<PathBuf> {
    fn gez(klasor: &Path, kok: &Path, cikti: &mut Vec<PathBuf>) {
        let Ok(okunan) = std::fs::read_dir(klasor) else {
            return;
        };
        for g in okunan.filter_map(|g| g.ok()) {
            let ad = g.file_name().to_string_lossy().into_owned();
            if ad.starts_with('.') || ad == "cikti" || ad == "target" {
                continue;
            }
            if klasor == kok && ad == "paketler" {
                continue;
            }
            let Ok(tur) = g.file_type() else { continue };
            if tur.is_symlink() {
                continue;
            }
            if tur.is_dir() {
                gez(&g.path(), kok, cikti);
            } else {
                cikti.push(g.path());
            }
        }
    }
    let mut dosyalar = Vec::new();
    gez(kok, kok, &mut dosyalar);
    dosyalar.sort();
    dosyalar
}

fn goreli(kok: &Path, d: &Path) -> String {
    d.strip_prefix(kok)
        .unwrap_or(d)
        .to_string_lossy()
        .replace('\\', "/")
}

fn ara(kok: &str, aranan: &str, tam_kelime: bool) -> Yanit {
    let kok = PathBuf::from(kok);
    if !izinli_mi(&kok) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    let aranan: Vec<char> = aranan.chars().map(kucuk_harf).collect();
    let mut sonuclar = Vec::new();
    'dis: for d in proje_dosyalari(&kok) {
        let Ok(icerik) = std::fs::read_to_string(&d) else {
            continue;
        };
        for (i, satir) in icerik.lines().enumerate() {
            let k: Vec<char> = satir.chars().collect();
            if !eslesmeler(&k, &aranan, tam_kelime).is_empty() {
                sonuclar.push(json!({
                    "dosya": goreli(&kok, &d),
                    "satir": i + 1,
                    "metin": satir.trim(),
                }));
                if sonuclar.len() >= 200 {
                    break 'dis;
                }
            }
        }
    }
    Yanit::json(&json!({ "sonuclar": sonuclar }))
}

/// Projenin bütün dosyalarında bul ve değiştir. Her değişen dosyanın önceki hâli yerel
/// geçmişe kaydedilir (geri alınabilsin diye).
fn degistir(kok: &str, aranan: &str, yeni: &str, tam_kelime: bool) -> Yanit {
    let kok = PathBuf::from(kok);
    if !izinli_mi(&kok) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    if aranan.is_empty() {
        return hata("Aranacak metni yazın.");
    }
    let mut degisen = Vec::new();
    let mut toplam = 0;
    for d in proje_dosyalari(&kok) {
        let Ok(icerik) = std::fs::read_to_string(&d) else {
            continue;
        };
        let (yeni_icerik, sayi) = metinde_degistir(&icerik, aranan, yeni, tam_kelime);
        if sayi == 0 {
            continue;
        }
        gecmis::ilk_hali_sakla(&d);
        if let Err(e) = std::fs::write(&d, &yeni_icerik) {
            return hata(format!("{} kaydedilemedi: {e}", goreli(&kok, &d)));
        }
        gecmis::kaydet(&d, &yeni_icerik);
        toplam += sayi;
        degisen.push(goreli(&kok, &d));
    }
    Yanit::json(&json!({ "degisen": degisen, "sayi": toplam }))
}

fn paket_listesi(kok: &str) -> Yanit {
    let kok = Path::new(kok);
    if !izinli_mi(kok) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    match crate::paket::listele(kok) {
        Ok(l) => Yanit::json(&json!({ "paketler": l.into_iter().map(|p| json!({
            "ad": p.ad, "kaynak": p.kaynak, "isleme": p.isleme, "kurulu": p.kurulu,
            "izinler": p.izinler,
        })).collect::<Vec<_>>() })),
        Err(e) => hata(e),
    }
}

/// Kısıtlı mod: güvenilmeyen projede kod çalıştıran ya da indiren işlemler yapılmaz. Arayüz
/// `guvensiz` yanıtını görünce kullanıcıya projeye güvenip güvenmediğini sorar.
fn guven_gerekli(yol: &Path, islem: &str) -> Option<Yanit> {
    if depo::guvenilir_mi(yol) {
        return None;
    }
    Some(Yanit::json(&json!({
        "guvensiz": true,
        "hata": format!(
            "Bu proje güvenilir olarak işaretlenmedi (kısıtlı mod); {islem} kapalı. \
             Projeyi tanıyorsanız “Projeye güven” ile açabilirsiniz."
        ),
    })))
}

/// Git paneli: yalnızca açık ve güvenilen projelerde (Git deponun kendi ayarlarındaki
/// komutları çalıştırabilir).
fn git_islemi(kok: &str, f: impl FnOnce(&Path) -> Result<Value, String>) -> Yanit {
    let kok = PathBuf::from(kok);
    if !izinli_mi(&kok) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    if let Some(y) = guven_gerekli(&kok, "Git paneli") {
        return y;
    }
    match f(&kok) {
        Ok(v) => Yanit::json(&v),
        Err(e) => hata(e),
    }
}

/// İmlecin üzerindeki ismin projedeki başvuruları (Ara panelinin sonuç biçiminde).
fn basvurular(g: &Value) -> Yanit {
    let dosya = PathBuf::from(metin(g, "dosya"));
    if !izinli_mi(&dosya) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    let kaynaklar = crate::referans::diskten(&dosya);
    let satir = g["satir"].as_u64().unwrap_or(0) as usize;
    let sutun = g["sutun"].as_u64().unwrap_or(0) as usize;
    let Some((ad, yerler)) = crate::referans::bul(&kaynaklar, satir, sutun) else {
        return hata("Burada başvurusu aranabilecek bir isim yok.");
    };
    let kok = derleme::proje_koku(&dosya);
    let kok = std::fs::canonicalize(&kok).unwrap_or(kok);
    let sonuclar: Vec<Value> = yerler
        .iter()
        .map(|y| {
            let (yol, m) = &kaynaklar[y.dosya];
            json!({
                "dosya": yol.strip_prefix(&kok).unwrap_or(yol).to_string_lossy().replace('\\', "/"),
                "satir": y.satir + 1,
                "metin": m.lines().nth(y.satir).unwrap_or("").trim(),
                "tanim": y.tanim,
            })
        })
        .collect();
    Yanit::json(&json!({ "ad": ad, "sonuclar": sonuclar }))
}

/// İsmi projenin bütün dosyalarında yeniden adlandırır; dosyaların önceki hâlleri yerel geçmişe.
fn adlandir(g: &Value) -> Yanit {
    let dosya = PathBuf::from(metin(g, "dosya"));
    if !izinli_mi(&dosya) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    let yeni = metin(g, "yeni").trim();
    if let Err(e) = crate::referans::gecerli_ad(yeni) {
        return hata(e);
    }
    let kaynaklar = crate::referans::diskten(&dosya);
    let satir = g["satir"].as_u64().unwrap_or(0) as usize;
    let sutun = g["sutun"].as_u64().unwrap_or(0) as usize;
    let Some((ad, yerler)) = crate::referans::bul(&kaynaklar, satir, sutun) else {
        return hata("Burada yeniden adlandırılabilecek bir isim yok.");
    };
    let degisiklikler = crate::referans::yeniden_adlandir(&yerler, yeni);
    let kok = derleme::proje_koku(&dosya);
    let kok = std::fs::canonicalize(&kok).unwrap_or(kok);
    let mut degisen = Vec::new();
    for (i, (yol, m)) in kaynaklar.iter().enumerate() {
        let bu: Vec<_> = degisiklikler.iter().filter(|d| d.0 == i).collect();
        if bu.is_empty() || !izinli_mi(yol) {
            continue;
        }
        let yeni_metin = crate::referans::uygula(m, &bu);
        gecmis::ilk_hali_sakla(yol);
        if let Err(e) = std::fs::write(yol, &yeni_metin) {
            return hata(format!("{} kaydedilemedi: {e}", yol.display()));
        }
        gecmis::kaydet(yol, &yeni_metin);
        degisen.push(
            yol.strip_prefix(&kok)
                .unwrap_or(yol)
                .to_string_lossy()
                .replace('\\', "/"),
        );
    }
    Yanit::json(&json!({ "ad": ad, "yeni": yeni, "sayi": yerler.len(), "degisen": degisen }))
}

/// Sınamaları çalıştırır: `dosya` (göreli) ve `ad` verilmezse projedeki hepsi.
fn sina(g: &Value) -> Yanit {
    let kok = PathBuf::from(metin(g, "kok"));
    if !izinli_mi(&kok) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    if let Some(y) = guven_gerekli(&kok, "sınamaları çalıştırma") {
        return y;
    }
    let dosya = g["dosya"]
        .as_str()
        .filter(|d| !d.is_empty())
        .map(|d| kok.join(d));
    if let Some(d) = &dosya {
        if !izinli_mi(d) || !d.is_file() {
            return Yanit::hata(403, "bu dosyaya erişim yok");
        }
    }
    // Tek sınama: adı tam eşleşir (sına_a çalışırken sına_ab çalışmasın).
    let ad = g["ad"]
        .as_str()
        .filter(|a| !a.is_empty())
        .map(|a| format!("={a}"));
    Yanit::json(&crate::sinama::calistir_json(
        &kok,
        dosya.as_deref(),
        ad.as_deref(),
    ))
}

fn paket_islemi(
    kok: &str,
    kur: bool,
    f: impl FnOnce(&Path) -> Result<Vec<String>, String>,
) -> Yanit {
    let kok = Path::new(kok);
    if !izinli_mi(kok) {
        return Yanit::hata(403, "bu klasöre erişim yok");
    }
    if kur {
        if let Some(y) = guven_gerekli(kok, "paket kurma") {
            return y;
        }
    }
    match f(kok) {
        Ok(g) => Yanit::json(&json!({ "gunluk": g })),
        // Arayüz izinleri gösterip onay ister, onaylanırsa izinVer ile yineler.
        Err(e) if e.starts_with(crate::paket::IZIN_GEREKLI) => Yanit::json(&json!({
            "izin": true,
            "ayrinti": e[crate::paket::IZIN_GEREKLI.len()..].trim(),
        })),
        Err(e) => hata(e),
    }
}

fn teshis_json(h: &derleme::DerlemeHatasi) -> Value {
    match &h.teshis {
        Some(t) => json!([{
            "dosya": t.dosya, "satir": t.satir, "sutun": t.sutun,
            "mesaj": t.mesaj, "ipucu": t.ipucu,
            "duzeltme": t.duzeltme.as_ref().map(|d| json!({
                "satir": d.satir, "sutun": d.sutun, "uzunluk": d.uzunluk,
                "yeni": d.yeni, "baslik": d.baslik,
            })),
        }]),
        None => json!([{ "dosya": "", "satir": 0, "sutun": 0, "mesaj": h.metin, "ipucu": null }]),
    }
}

/// `acik`: düzenleyicide açık, kaydedilmemiş dosyalar {tam yol: içerik}
/// Açık dosyanın (kaydedilmemiş hâliyle) Python ya da JavaScript karşılığı.
fn cevir(dosya: &str, icerik: &str, dil: &str) -> Yanit {
    let p = Path::new(dosya);
    if !izinli_mi(p) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    let Some(dil) = crate::cevirici::Dil::coz(dil) else {
        return hata("bilinmeyen dil");
    };
    let mut ortulu = std::collections::HashMap::new();
    if let Ok(tam) = std::fs::canonicalize(p) {
        ortulu.insert(tam, icerik.to_string());
    }
    match derleme::yukle_ortulu(p, &ortulu) {
        Ok(program) => Yanit::json(&json!({
            "satirlar": crate::cevirici::cevir(&program, dil)
                .into_iter()
                .map(|s| json!({ "k": s.kaynak, "m": s.metin }))
                .collect::<Vec<_>>()
        })),
        Err(h) => hata(format!("Önce programdaki hataları düzeltin:\n{}", h.metin)),
    }
}

fn denetle(dosya: &str, acik: &Value) -> Yanit {
    let p = Path::new(dosya);
    if !izinli_mi(p) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    let mut ortulu = std::collections::HashMap::new();
    if let Some(acik) = acik.as_object() {
        for (yol, icerik) in acik {
            let yol = Path::new(yol);
            if let (true, Ok(tam), Some(icerik)) =
                (izinli_mi(yol), std::fs::canonicalize(yol), icerik.as_str())
            {
                ortulu.insert(tam, icerik.to_string());
            }
        }
    }
    // Yazım uyarıları (ünlü uyumu): açık dosyalar ve denetlenen dosya
    let mut uyarilar = Vec::new();
    let mut kaynaklar: Vec<(String, String)> = acik
        .as_object()
        .map(|a| {
            a.iter()
                .filter_map(|(y, i)| Some((y.clone(), i.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    if !kaynaklar.iter().any(|(y, _)| Path::new(y) == p) {
        if let Ok(i) = std::fs::read_to_string(p) {
            kaynaklar.push((dosya.to_string(), i));
        }
    }
    for (yol, icerik) in &kaynaklar {
        for u in crate::bicimlendirici::uyarilar(icerik) {
            uyarilar.push(json!({
                "dosya": yol, "satir": u.konum.satir, "sutun": u.konum.sutun,
                "uzunluk": u.uzunluk, "mesaj": u.mesaj, "duzeltme": u.duzeltme,
            }));
        }
    }
    let hatalar = match derleme::yukle_ortulu(p, &ortulu) {
        Ok(_) => json!([]),
        Err(h) => teshis_json(&h),
    };
    Yanit::json(&json!({ "hatalar": hatalar, "uyarilar": uyarilar }))
}

/// Arayüz programlarının derlenmiş sayfaları (son birkaçı tutulur).
fn onizlemeler() -> &'static Mutex<std::collections::VecDeque<(u64, String)>> {
    static O: OnceLock<Mutex<std::collections::VecDeque<(u64, String)>>> = OnceLock::new();
    O.get_or_init(Default::default)
}

pub fn onizleme(kimlik: &str) -> Yanit {
    let kimlik: u64 = kimlik.parse().unwrap_or(0);
    let liste = onizlemeler().lock().unwrap();
    match liste.iter().find(|(k, _)| *k == kimlik) {
        Some((_, sayfa)) => Yanit {
            durum: 200,
            tur: "text/html; charset=utf-8",
            govde: sayfa.clone().into_bytes(),
            // Kullanıcı programının sayfası: kendi betiklerini (gömülü) çalıştırır.
            csp: false,
        },
        None => Yanit::hata(404, "önizleme bulunamadı; programı yeniden çalıştırın"),
    }
}

/// Arayüz programı: WebAssembly'ye derlenir, sayfası önizleme için saklanır.
fn arayuz_calistir(program: &agac::Program, dosya: &Path, baslangic: Instant) -> Yanit {
    let wasm = match crate::wasm_uretici::uret(program) {
        Ok(w) => w,
        Err(e) => {
            let m = format!("WebAssembly kod üretimi başarısız: {e}");
            return Yanit::json(&json!({ "derleme_hatasi": m, "hatalar": [] }));
        }
    };
    let ad = dosya
        .file_stem()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_else(|| "uygulama".into());
    static SAYAC: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let kimlik = SAYAC.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut liste = onizlemeler().lock().unwrap();
    liste.push_back((kimlik, derleme::web_sayfasi(&ad, &wasm, true)));
    while liste.len() > 8 {
        liste.pop_front();
    }
    Yanit::json(&json!({
        "arayuz": format!("/onizleme/{kimlik}"),
        "derleme_ms": baslangic.elapsed().as_millis(),
    }))
}

fn calistir(g: &Value) -> Yanit {
    let dosya = PathBuf::from(metin(g, "dosya"));
    if !izinli_mi(&dosya) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    if let Some(y) = guven_gerekli(&dosya, "çalıştırma ve hata ayıklama") {
        return y;
    }
    let baslangic = Instant::now();
    match derleme::yukle(&dosya) {
        Ok(p) if p.arayuz_programi() => return arayuz_calistir(&p, &dosya, baslangic),
        Ok(_) => {}
        Err(h) => {
            return Yanit::json(&json!({ "derleme_hatasi": h.metin, "hatalar": teshis_json(&h) }))
        }
    }
    let klasor = PathBuf::from(metin(g, "klasor"));
    let argumanlar: Vec<String> = g["argumanlar"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let gecici = match derleme::gecici_klasor("studyo") {
        Ok(k) => k,
        Err(e) => return hata(e),
    };
    let program = gecici.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    // Hata ayıklama: her deyimde kanca; program Stüdyo'ya bağlanır.
    let ayiklama = g["ayikla"].as_bool().unwrap_or(false);
    let sonuc = if ayiklama {
        derleme::derle_ayiklamali(&dosya, &program, &kosullu_kesmeler(g))
    } else {
        derleme::derle(&dosya, &program, None).map(|_| Vec::new())
    };
    let dosyalar = match sonuc {
        Ok(d) => d,
        Err(h) => {
            let _ = std::fs::remove_dir_all(&gecici);
            return Yanit::json(&json!({ "derleme_hatasi": h.metin, "hatalar": teshis_json(&h) }));
        }
    };
    let ayiklama = if ayiklama {
        match std::net::TcpListener::bind(("127.0.0.1", 0)) {
            Ok(dinleyici) => Some(calisma::AyiklamaBaslangici {
                dinleyici,
                dosyalar,
                kesmeler: kesme_noktalari(g),
                ilkte_dur: g["ilkte_dur"].as_bool().unwrap_or(false),
            }),
            Err(e) => return hata(format!("hata ayıklayıcı başlatılamadı: {e}")),
        }
    } else {
        None
    };
    let derleme_ms = baslangic.elapsed().as_millis();
    // Web sunucuları için kapı: boşsa 3000, değilse sistemin verdiği boş bir kapı.
    let kapi = bos_kapi();
    let ortam = [
        ("ORHUNCA_KAPI", kapi.to_string()),
        ("ORHUNCA_ONIZLEME", "1".to_string()),
        // Stüdyo beklenmedik biçimde kapanırsa sunucu da kendini kapatır.
        ("ORHUNCA_EBEVEYN", std::process::id().to_string()),
    ];
    match calisma::baslat(
        &program,
        &klasor,
        &argumanlar,
        &ortam,
        gecici.clone(),
        ayiklama,
    ) {
        Ok(kimlik) => {
            Yanit::json(&json!({ "kimlik": kimlik, "derleme_ms": derleme_ms, "kapi": kapi }))
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&gecici);
            hata(e)
        }
    }
}

fn bos_kapi() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 3000))
        .or_else(|_| std::net::TcpListener::bind(("127.0.0.1", 0)))
        .and_then(|d| d.local_addr())
        .map(|a| a.port())
        .unwrap_or(3000)
}

fn cikti(kimlik: &str, konum: &str) -> Yanit {
    let kimlik = kimlik.parse().unwrap_or(0);
    let konum = konum.parse().unwrap_or(0);
    match calisma::durum(kimlik, konum) {
        Some(d) => {
            let parcalar: Vec<Value> = d
                .parcalar
                .into_iter()
                .map(|(tur, t)| json!({ "tur": tur, "t": t }))
                .collect();
            let ayiklama = d.ayiklama.map(|a| {
                json!({
                    "bagli": a.bagli, "durdu": a.durdu, "neden": a.neden, "ileti": a.ileti,
                    "cerceve": a.cerceve, "surum": a.surum,
                    "yigin": a.yigin.iter().map(|(i, d, s)| json!({ "islev": i, "dosya": d, "satir": s })).collect::<Vec<_>>(),
                    "degiskenler": a.degiskenler.iter().map(|(ad, t, d)| json!({ "ad": ad, "tip": t, "deger": d })).collect::<Vec<_>>(),
                })
            });
            Yanit::json(&json!({
                "parcalar": parcalar, "konum": d.konum, "bitti": d.bitti,
                "kod": d.kod, "sure_ms": d.sure_ms as u64, "ayiklama": ayiklama,
            }))
        }
        None => hata("çalıştırma bulunamadı"),
    }
}

/// `kesmeler: [{dosya, satir}]`
fn kosullu_kesmeler(g: &Value) -> Vec<crate::ayiklama::KosulluKesme> {
    let metin_al = |v: &Value| {
        v.as_str()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(str::to_string)
    };
    g["kesmeler"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|k| {
                    Some(crate::ayiklama::KosulluKesme {
                        dosya: k["dosya"].as_str()?.to_string(),
                        satir: k["satir"].as_u64()? as usize,
                        kosul: metin_al(&k["kosul"]),
                        gunluk: metin_al(&k["gunluk"]),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Çalışma zamanına gönderilen kesme noktaları: günlük noktaları durmaz, koşullu kesmeler
/// gizli deyimin satırında durur (bkz. ayiklama.rs).
fn kesme_noktalari(g: &Value) -> Vec<(String, usize)> {
    kosullu_kesmeler(g)
        .into_iter()
        .filter_map(|k| Some((k.dosya.clone(), k.calisma_zamani_satiri()?)))
        .collect()
}

fn ayikla(g: &Value) -> Yanit {
    match calisma::ayiklama_komutu(
        g["kimlik"].as_u64().unwrap_or(0),
        metin(g, "komut"),
        &kesme_noktalari(g),
        g["cerceve"].as_u64().unwrap_or(0) as usize,
    ) {
        Ok(()) => Yanit::json(&json!({ "tamam": true })),
        Err(e) => hata(e),
    }
}

fn girdi(g: &Value) -> Yanit {
    match calisma::girdi_gonder(g["kimlik"].as_u64().unwrap_or(0), metin(g, "metin")) {
        Ok(()) => Yanit::json(&json!({ "tamam": true })),
        Err(e) => hata(e),
    }
}

/// Dağıtım için derler: çıktı projenin `cikti/` klasörüne yazılır.
fn derle(dosya: &str, hedef: &str) -> Yanit {
    let p = PathBuf::from(dosya);
    if !izinli_mi(&p) {
        return Yanit::hata(403, "bu dosyaya erişim yok");
    }
    if let Some(y) = guven_gerekli(&p, "derleme ve paketleme") {
        return y;
    }
    // "masaustu-linux" / "masaustu-windows": pencere kabuğuna paketlenir.
    let masaustu = hedef.strip_prefix("masaustu-");
    let hedef = masaustu.or((!hedef.is_empty()).then_some(hedef));
    let kok = izinli_kokler()
        .lock()
        .unwrap()
        .iter()
        .filter(|k| {
            std::fs::canonicalize(&p)
                .map(|t| t.starts_with(k))
                .unwrap_or(false)
        })
        .max_by_key(|k| k.as_os_str().len())
        .cloned()
        .unwrap_or_else(|| p.parent().map(Path::to_path_buf).unwrap_or_default());
    let klasor = kok.join("cikti");
    let _ = std::fs::create_dir_all(&klasor);
    let ad = derleme::proje_dosyasi(&kok)
        .and_then(|pd| derleme::proje_ayari(&pd, &["ad"]))
        .unwrap_or_else(|| "program".into());
    let mut cikti = klasor.join(&ad);
    if derleme::web_hedefi_mi(hedef) {
        cikti.set_extension("html");
    } else if derleme::android_mi(hedef) {
        cikti.set_extension("apk");
    } else if derleme::ios_mu(hedef) {
        cikti = klasor.join(format!("{ad}-ios"));
    } else if derleme::windows_mu(hedef) {
        cikti.set_extension("exe");
    }
    let baslangic = Instant::now();
    let sonuc = if masaustu.is_some() {
        derleme::paketle(&p, &cikti, hedef)
    } else {
        derleme::derle(&p, &cikti, hedef)
    };
    match sonuc {
        Ok(()) => Yanit::json(&json!({
            "cikti": cikti.to_string_lossy(),
            "sure_ms": baslangic.elapsed().as_millis() as u64,
            "boyut": std::fs::metadata(&cikti).map(|m| m.len()).unwrap_or(0),
        })),
        Err(h) => Yanit::json(&json!({ "derleme_hatasi": h.metin, "hatalar": teshis_json(&h) })),
    }
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn bul_ve_degistir() {
        let (m, n) = metinde_degistir(
            "sayı = 1\nSAYI'yı yaz.\nsayılar = []\n",
            "sayı",
            "adet",
            false,
        );
        assert_eq!(
            (m.as_str(), n),
            ("adet = 1\nadet'yı yaz.\nadetlar = []\n", 3)
        );
        let (m, n) = metinde_degistir("sayı = 1\nsayılar = [sayı]\n", "sayı", "adet", true);
        assert_eq!((m.as_str(), n), ("adet = 1\nsayılar = [adet]\n", 2));
        // Türkçe büyük harfler: IŞIK ve ışık aynı kelime; İl ve il aynı kelime
        let (m, n) = metinde_degistir("IŞIK ışık İl il", "ışık", "x", false);
        assert_eq!((m.as_str(), n), ("x x İl il", 2));
        let (m, n) = metinde_degistir("İl il Il", "il", "y", true);
        assert_eq!((m.as_str(), n), ("y y Il", 2));
        let (m, n) = metinde_degistir("aaa", "a", "aa", false);
        assert_eq!((m.as_str(), n), ("aaaaaa", 3));
        assert_eq!(metinde_degistir("abc", "x", "y", false).1, 0);
    }
}
