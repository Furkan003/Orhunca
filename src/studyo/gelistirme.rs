//! Proje kurtarma, hedef profilleri, sağlık ve teslim kayıtları.
use super::{arsiv, depo, dosya};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

type Sonuc = Result<Value, String>;
fn oku(p: &Path) -> Sonuc {
    serde_json::from_slice(&std::fs::read(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn yaz(p: &Path, v: &Value) -> Result<(), String> {
    dosya::atomik_yaz(p, &serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
pub fn alan(kok: &Path) -> Result<PathBuf, String> {
    let p = depo::ayar_klasoru()
        .join("projeler")
        .join(dosya::surum(kok.to_string_lossy().as_bytes()));
    dosya::ozel_klasor(&p).map_err(|e| e.to_string())?;
    Ok(p)
}
fn kimlik(v: &Value, ad: &str) -> Result<String, String> {
    let s = v[ad].as_str().unwrap_or("");
    if s.is_empty() || s.len() > 80 || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err("Geçersiz kayıt adı.".into());
    }
    Ok(s.into())
}
pub fn kayit(kok: &Path, ad: &str, veri: &Value) -> Result<(), String> {
    yaz(
        &alan(kok)?.join(format!("{ad}.json")),
        &json!({"zaman": depo::simdi(), "sonuc": veri}),
    )
}

pub fn yedekle(kok: &Path, otomatik: bool) -> Sonuc {
    let alan = alan(kok)?.join("yedekler");
    dosya::ozel_klasor(&alan).map_err(|e| e.to_string())?;
    let _kilit = dosya::kilitle(&alan.join("kilit")).map_err(|e| e.to_string())?;
    if otomatik {
        let l = kayitlar(&alan)?;
        if l.as_array()
            .unwrap()
            .iter()
            .any(|v| depo::simdi().saturating_sub(v["zaman"].as_u64().unwrap_or(0)) < 600)
        {
            return Ok(json!({"atlandi": true, "neden": "Son yedek 10 dakikadan yeni."}));
        }
    }
    let toplam = kayitlar(&alan)?
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["boyut"].as_u64())
        .sum::<u64>();
    if toplam >= 2 * 1024 * 1024 * 1024 {
        return Err(
            "Yedek alanı 2 GiB sınırına ulaştı; kurtarma ekranından eski yedekleri temizleyin."
                .into(),
        );
    }
    let id = dosya::kimlik().map_err(|e| e.to_string())?;
    let gecici = arsiv::Gecici(alan.join(format!(".hazir-{id}")));
    dosya::ozel_klasor(&gecici.0).map_err(|e| e.to_string())?;
    let dosyalar =
        arsiv::kopyala(kok, &gecici.0.join("icerik"), true, false).map_err(|e| e.to_string())?;
    if toplam
        + dosyalar
            .iter()
            .filter_map(|v| v["boyut"].as_u64())
            .sum::<u64>()
        > 2 * 1024 * 1024 * 1024
    {
        return Err("Yeni yedek 2 GiB sınırını aşıyor; önce eski yedekleri temizleyin.".into());
    }
    let bilgi = json!({"kimlik": id, "zaman": depo::simdi(), "dosyalar": dosyalar, "boyut": dosyalar.iter().filter_map(|v| v["boyut"].as_u64()).sum::<u64>()});
    yaz(&gecici.0.join("bilgi.json"), &bilgi)?;
    std::fs::rename(&gecici.0, alan.join(&id)).map_err(|e| e.to_string())?;
    Ok(bilgi)
}
fn kayitlar(klasor: &Path) -> Sonuc {
    if !klasor.exists() {
        return Ok(json!([]));
    }
    let mut liste = Vec::new();
    for g in std::fs::read_dir(klasor).map_err(|e| e.to_string())? {
        let g = g.map_err(|e| e.to_string())?;
        if g.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        arsiv::baglantisiz(klasor, &g.path().join("bilgi.json")).map_err(|e| e.to_string())?;
        if let Ok(v) = oku(&g.path().join("bilgi.json")) {
            liste.push(v);
        }
    }
    liste.sort_by_key(|v| std::cmp::Reverse(v["zaman"].as_u64().unwrap_or(0)));
    Ok(json!(liste))
}
fn yedegi_dogrula(p: &Path) -> Sonuc {
    let bilgi = oku(&p.join("bilgi.json"))?;
    for d in bilgi["dosyalar"].as_array().ok_or("Yedek bilgisi bozuk.")? {
        let yol = p.join("icerik").join(
            arsiv::goreli(d["yol"].as_str().ok_or("Yedek yolu yok.")?)
                .map_err(|e| e.to_string())?,
        );
        arsiv::baglantisiz(p, &yol).map_err(|e| e.to_string())?;
        if dosya::surum(&std::fs::read(yol).map_err(|e| e.to_string())?)
            != d["ozet"].as_str().unwrap_or("")
        {
            return Err("Yedek özeti uyuşmuyor; kurtarma durdu.".into());
        }
    }
    Ok(bilgi)
}

fn taslak(kok: &Path, g: &Value) -> Sonuc {
    let id = kimlik(g, "oturum")?;
    let p = alan(kok)?.join("taslaklar");
    dosya::ozel_klasor(&p).map_err(|e| e.to_string())?;
    let hedef = p.join(format!("{id}.json"));
    let _kilit = dosya::kilitle(&hedef).map_err(|e| e.to_string())?;
    let onceki = oku(&hedef).unwrap_or(Value::Null);
    let sira = g["sira"].as_u64().ok_or("Taslak sırası eksik.")?;
    if onceki["sira"].as_u64().is_some_and(|s| s >= sira) {
        return Ok(json!({"atlandi":true}));
    }
    let sekmeler = g["sekmeler"].as_array().ok_or("Sekmeler eksik.")?;
    if sekmeler.len() > 100 {
        return Err("En fazla 100 sekme kurtarılabilir.".into());
    }
    let mut boy = 0;
    for s in sekmeler {
        let p = arsiv::goreli(s["yol"].as_str().ok_or("Sekme yolu eksik.")?)
            .map_err(|e| e.to_string())?;
        arsiv::baglantisiz(kok, &kok.join(p)).map_err(|e| e.to_string())?;
        boy += s["icerik"].as_str().ok_or("Sekme metni eksik.")?.len();
    }
    if boy > 8 * 1024 * 1024 {
        return Err("Kurtarma taslağı 8 MiB sınırını aşıyor.".into());
    }
    yaz(
        &hedef,
        &json!({"oturum": id, "sira": sira, "zaman": depo::simdi(), "sekmeler": sekmeler}),
    )?;
    Ok(json!({"tamam":true}))
}
fn taslaklar(kok: &Path) -> Sonuc {
    let p = alan(kok)?.join("taslaklar");
    let mut l = Vec::new();
    if p.exists() {
        for g in std::fs::read_dir(&p).map_err(|e| e.to_string())? {
            let g = g.map_err(|e| e.to_string())?;
            arsiv::baglantisiz(&p, &g.path()).map_err(|e| e.to_string())?;
            if let Ok(v) = oku(&g.path()) {
                if v["sekmeler"].as_array().is_some_and(|a| !a.is_empty()) {
                    l.push(v);
                }
            }
        }
    }
    Ok(json!(l))
}

/// Alt süreç çıktısı ve süresi sınırlıdır. Kabuk metni hiçbir zaman yorumlanmaz.
pub fn komut(mut k: Command, saniye: u64) -> Result<(bool, String), String> {
    let mut c = k
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    fn okuyucu(akis: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            let mut sonuc = Vec::new();
            let mut akis = akis;
            let mut b = [0; 8192];
            while let Ok(n) = akis.read(&mut b) {
                if n == 0 {
                    break;
                }
                let kalan = (1024 * 1024usize).saturating_sub(sonuc.len());
                sonuc.extend_from_slice(&b[..n.min(kalan)]);
            }
            String::from_utf8_lossy(&sonuc).into_owned()
        })
    }
    let a = okuyucu(c.stdout.take().unwrap());
    let b = okuyucu(c.stderr.take().unwrap());
    let bas = Instant::now();
    loop {
        match c.try_wait().map_err(|e| e.to_string())? {
            Some(d) => {
                return Ok((
                    d.success(),
                    format!(
                        "{}{}",
                        a.join().unwrap_or_default(),
                        b.join().unwrap_or_default()
                    ),
                ))
            }
            None if bas.elapsed() < Duration::from_secs(saniye) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = c.kill();
                let _ = c.wait();
                return Err(format!("İşlem {saniye} saniyede tamamlanmadı."));
            }
        }
    }
}
fn arac(ad: &str, arg: &str, kok: &Path) -> Value {
    let mut k = crate::komut(ad);
    k.arg(arg).current_dir(kok);
    match komut(k, 5) {
        Ok((true, s)) => {
            json!({"ad": ad, "durum":"hazır", "surum": s.lines().next().unwrap_or("")})
        }
        Ok((false, _)) => json!({"ad":ad,"durum":"çalışmadı"}),
        Err(_) => json!({"ad":ad,"durum":"bulunamadı"}),
    }
}
fn saglik(kok: &Path, hedef: &str) -> Sonuc {
    let mut araclar = vec![
        json!({"ad":"Orhunca","durum":"hazır","surum":env!("CARGO_PKG_VERSION")}),
        arac("git", "--version", kok),
    ];
    for (ad, arg) in match hedef {
        "web" => vec![("node", "--version")],
        "android" => vec![("adb", "version")],
        "ios" | "macos" => vec![("xcodebuild", "-version")],
        _ => Vec::new(),
    } {
        araclar.push(arac(ad, arg, kok));
    }
    let paketler = crate::paket::listele(kok).map(|p| {
        p.into_iter()
            .map(|p| json!({"ad":p.ad,"kurulu":p.kurulu}))
            .collect::<Vec<_>>()
    });
    let a = alan(kok)?;
    Ok(
        json!({"hedef": hedef,"araclar":araclar,"paketler":paketler.as_ref().ok(),"paket_hatasi":paketler.err(),"son_derleme":oku(&a.join("derleme.json")).ok(),"son_basarili_derleme":oku(&a.join("basarili-derleme.json")).ok(),"son_sinama":oku(&a.join("sinama.json")).ok(),"cihaz_denemesi":"Bu rapor fiziksel cihaz denemesi yapmaz."}),
    )
}

pub fn ayarlar(kok: &Path) -> Sonuc {
    let p = kok.join("studyo.ayar.json");
    arsiv::baglantisiz(kok, &p).map_err(|e| e.to_string())?;
    if !p.exists() {
        return Ok(
            json!({"profiller":[{"ad":"gelistirme","hedef":"web","kip":"geliştirme","ortam":{}},{"ad":"sinama","hedef":"web","kip":"sınama","ortam":{}},{"ad":"teslim","hedef":"web","kip":"teslim","ortam":{}}],"senaryolar":[]}),
        );
    }
    let v = oku(&p)?;
    ayarlari_dogrula(&v)?;
    Ok(v)
}
fn profil(kok: &Path, ad: &str) -> Sonuc {
    ayarlar(kok)?["profiller"]
        .as_array()
        .and_then(|p| p.iter().find(|p| p["ad"] == ad))
        .cloned()
        .ok_or("Profil bulunamadı.".into())
}
fn ortam(kok: &Path) -> BTreeMap<String, String> {
    let mut sonuc: BTreeMap<String, String> = std::env::vars().collect();
    env_ekle(kok, &kok.join(".env"), &mut sonuc);
    sonuc
}
fn env_ekle(kok: &Path, p: &Path, sonuc: &mut BTreeMap<String, String>) {
    if arsiv::baglantisiz(kok, p).is_ok() {
        if let Ok(s) = std::fs::read_to_string(p) {
            for l in s.lines().map(str::trim).filter(|l| !l.starts_with('#')) {
                if let Some((k, v)) = l.trim_start_matches("export ").split_once('=') {
                    let v = v.trim();
                    let d = if v.len() >= 2
                        && (v.starts_with('\'') || v.starts_with('"'))
                        && v.ends_with(v.chars().next().unwrap())
                    {
                        &v[1..v.len() - 1]
                    } else {
                        v.split(" #").next().unwrap_or("").trim_end()
                    };
                    sonuc.entry(k.trim().into()).or_insert_with(|| d.into());
                }
            }
        }
    }
}
/// Demo kopyasında projenin gizli değerlerinden (`.env`, `.env.*`, ortam) biri ya da
/// özel anahtar bulunursa hata verir.
pub fn demo_dogrula(kok: &Path, hedef: &Path, dosyalar: &[Value]) -> Result<(), String> {
    let mut o = ortam(kok);
    if let Ok(girdiler) = std::fs::read_dir(kok) {
        for g in girdiler.flatten() {
            let ad = g.file_name().to_string_lossy().to_lowercase();
            if ad.starts_with(".env.") {
                let mut ek = BTreeMap::new();
                env_ekle(kok, &g.path(), &mut ek);
                // Aynı ad farklı dosyalarda farklı değer taşıyabilir: hepsi denetlenir.
                for (k, v) in ek {
                    o.insert(format!("{k}#{ad}"), v);
                }
            }
        }
    }
    paylasimi_dogrula(hedef, dosyalar, &o).map_err(|e| e.replace("Teslim", "Demo"))
}
fn yerlestir(s: &str, ortam: &BTreeMap<String, String>) -> Result<String, String> {
    let mut sonuc = String::new();
    let mut kalan = s;
    while let Some((bas, son)) = kalan.split_once("${") {
        let (ad, son) = son.split_once('}').ok_or("Ortam değişkeni kapanmamış.")?;
        sonuc.push_str(bas);
        sonuc.push_str(
            ortam
                .get(ad)
                .ok_or_else(|| format!("Ortam değişkeni eksik: {ad}"))?,
        );
        kalan = son;
    }
    sonuc.push_str(kalan);
    Ok(sonuc)
}
fn gizle(mut s: String, ortam: &BTreeMap<String, String>) -> String {
    for (k, v) in ortam {
        let k = k.to_uppercase();
        if v.len() >= 4
            && [
                "KEY",
                "TOKEN",
                "SECRET",
                "PASSWORD",
                "ANAHTAR",
                "SIFRE",
                "ŞİFRE",
                "VERITABANI",
            ]
            .iter()
            .any(|a| k.contains(a))
        {
            s = s.replace(v, "[gizlendi]");
        }
    }
    s
}

pub fn islem(kok: &Path, g: &Value) -> Sonuc {
    match g["islem"].as_str().unwrap_or("") {
        "taslak" => taslak(kok, g),
        "taslak-sil" => {
            let id = kimlik(g, "oturum")?;
            let p = alan(kok)?.join("taslaklar").join(format!("{id}.json"));
            arsiv::baglantisiz(&alan(kok)?, &p).map_err(|e| e.to_string())?;
            let _kilit = dosya::kilitle(&p).map_err(|e| e.to_string())?;
            std::fs::remove_file(p).map_err(|e| e.to_string())?;
            Ok(json!({"tamam":true}))
        }
        "kurtarma" => Ok(
            json!({"taslaklar":taslaklar(kok)?,"yedekler":kayitlar(&alan(kok)?.join("yedekler"))?}),
        ),
        "yedek" => yedekle(kok, g["otomatik"] == true),
        "yedek-dene" | "yedek-sil" => {
            let id = kimlik(g, "kimlik")?;
            let p = alan(kok)?.join("yedekler").join(&id);
            arsiv::baglantisiz(&alan(kok)?, &p).map_err(|e| e.to_string())?;
            let bilgi = yedegi_dogrula(&p)?;
            if g["islem"] == "yedek-sil" {
                std::fs::remove_dir_all(p).map_err(|e| e.to_string())?;
                return Ok(json!({"tamam":true}));
            }
            let hedef = kok.join(format!(
                "kurtarilan-{}",
                dosya::kimlik().map_err(|e| e.to_string())?
            ));
            arsiv::kopyala(&p.join("icerik"), &hedef, false, false).map_err(|e| e.to_string())?;
            Ok(json!({"yol":hedef,"bilgi":bilgi}))
        }
        "saglik" => saglik(kok, g["hedef"].as_str().unwrap_or("web")),
        "ayarlar" => {
            let p = kok.join("studyo.ayar.json");
            let surum = std::fs::read(&p)
                .ok()
                .map(|b| dosya::surum(&b))
                .unwrap_or_else(|| "yok".into());
            Ok(json!({"ayarlar":ayarlar(kok)?,"surum":surum}))
        }
        "profil-derle" => profil_derle(kok, g),
        "api-sina" => api_sina(kok, g),
        "cihazlar" => cihazlari_listele(kok, g["hedef"].as_str().unwrap_or("android")),
        "cihaz" => cihaz(kok, g),
        "gecis-onizle" | "gecis-uygula" | "gecis-geri" | "demo" => super::gecis::islem(kok, g),
        _ => Err("Bilinmeyen geliştirme işlemi.".into()),
    }
}

fn profil_derle(kok: &Path, g: &Value) -> Sonuc {
    let p = profil(kok, g["profil"].as_str().unwrap_or(""))?;
    let hedef = p["hedef"].as_str().ok_or("Profil hedefi eksik.")?;
    if !["web", "linux", "windows", "macos", "android", "ios"].contains(&hedef) {
        return Err("Desteklenmeyen hedef.".into());
    }
    let a = alan(kok)?.join("teslimler");
    dosya::ozel_klasor(&a).map_err(|e| e.to_string())?;
    let id = dosya::kimlik().map_err(|e| e.to_string())?;
    let gecici = arsiv::Gecici(a.join(format!(".hazir-{id}")));
    dosya::ozel_klasor(&gecici.0).map_err(|e| e.to_string())?;
    let kaynak = gecici.0.join("kaynak");
    let mut dosyalar = arsiv::kopyala(kok, &kaynak, true, true).map_err(|e| e.to_string())?;
    // Paketler kilitle sabitlenmiştir; gizli .git verisi teslim edilmez.
    if kok.join("paketler").is_dir() {
        let paketler = arsiv::kopyala(&kok.join("paketler"), &kaynak.join("paketler"), true, true)
            .map_err(|e| e.to_string())?;
        for mut d in paketler {
            d["yol"] = json!(format!("paketler/{}", d["yol"].as_str().unwrap_or("")));
            dosyalar.push(d);
        }
    }
    let ortam = ortam(kok);
    paylasimi_dogrula(&kaynak, &dosyalar, &ortam)?;
    let sinama = if matches!(p["kip"].as_str(), Some("sınama" | "teslim")) {
        let mut r = gizle_json(
            crate::sinama::calistir_json(&kaynak, None, None, false),
            &ortam,
        );
        r["durum"] = json!(
            if r["gecti"].as_u64() == Some(0) && r["kaldi"].as_u64() == Some(0) {
                "sınama yok"
            } else {
                "çalıştırıldı"
            }
        );
        kayit(kok, "sinama", &r)?;
        if r["kaldi"].as_u64().unwrap_or(1) > 0 {
            return Ok(
                json!({"tamam":false,"sinama":r,"hata":"Sınamalar başarısız; teslim oluşturulmadı."}),
            );
        }
        r
    } else {
        json!({"durum":"çalıştırılmadı"})
    };
    let mut k = crate::komut(std::env::current_exe().map_err(|e| e.to_string())?);
    let paket = p["masaustu"] == true || ["android", "ios", "macos"].contains(&hedef);
    let cikti = gecici.0.join(match hedef {
        "web" => "uygulama.html",
        "windows" => "uygulama.exe",
        "android" => "uygulama.apk",
        "ios" => "ios",
        _ => "uygulama",
    });
    k.current_dir(&kaynak)
        .args([
            if paket { "paketle" } else { "derle" },
            "--hedef",
            hedef,
            "-o",
        ])
        .arg(&cikti);
    if let Some(o) = p["ortam"].as_object() {
        for (ad, v) in o {
            k.env(
                ad,
                yerlestir(v.as_str().ok_or("Ortam değeri metin olmalı.")?, &ortam)?,
            );
        }
    }
    let acik = acik_ortam_coz(kok, &p)?;
    if hedef != "web" && !acik.as_object().unwrap().is_empty() {
        let mut env = String::new();
        for (ad, v) in acik.as_object().unwrap() {
            env.push_str(&format!("{ad}=\"{}\"\n", v.as_str().unwrap()));
        }
        dosya::atomik_yaz(&gecici.0.join(".env"), env.as_bytes()).map_err(|e| e.to_string())?;
    }
    k.env("ORHUNCA_ACIK_ORTAM_JSON", acik.to_string());
    let bas = Instant::now();
    let (ok, cikti_metni) = komut(k, 180)?;
    let mut git = crate::komut("git");
    git.current_dir(kok).args(["rev-parse", "HEAD"]);
    let isleme = komut(git, 5)
        .ok()
        .filter(|r| r.0)
        .map(|r| r.1.trim().to_string());
    let mut durum = crate::komut("git");
    durum.current_dir(kok).args(["status", "--porcelain"]);
    let degismis = komut(durum, 5)
        .ok()
        .filter(|r| r.0)
        .map(|r| !r.1.trim().is_empty());
    let mut rapor = gizle_json(
        json!({"tamam":ok,"hedef":hedef,"profil":p["ad"],"acik_ortam":acik,"isleme":isleme,"degismis":degismis,"surum":env!("CARGO_PKG_VERSION"),"dosyalar":dosyalar,"cikti":gizle(cikti_metni,&ortam),"sure_ms":bas.elapsed().as_millis(),"sinama":sinama,"araclar":saglik(kok,hedef)?["araclar"],"zaman":depo::simdi()}),
        &ortam,
    );
    rapor["acik_ortam"] = acik;
    kayit(kok, "derleme", &rapor)?;
    if !ok {
        return Ok(rapor);
    }
    kayit(kok, "basarili-derleme", &rapor)?;
    yaz(&gecici.0.join("teslim.json"), &rapor)?;
    dosya::atomik_yaz(&gecici.0.join("KURULUM.txt"),format!("Orhunca {}\nHedef: {hedef}\nKaynak özeti ve sınamalar: teslim.json\nKaynağı yeniden derlemek için kaynak/ klasöründe:\norhunca {} --hedef {hedef}\nWeb: uygulama.html dosyasını tarayıcıda açın. Android: APK'yı cihaza kurun. iOS: Xcode projesini açın. Masaüstü: uygulama dosyasını çalıştırın.\nAçık istemci ayarları teslim.json içindeki acik_ortam alanındadır. Web/mobil yeniden derlemesinde ORHUNCA_ACIK_ORTAM_JSON değerini bu JSON nesnesi olarak tanımlayın. Gizli ortam değerleri teslim edilmez; gerekli değerleri kendi ortamınızda tanımlayın.\n",env!("CARGO_PKG_VERSION"),if paket {"paketle"} else {"derle"}).as_bytes()).map_err(|e|e.to_string())?;
    let yol = a.join(id);
    std::fs::rename(&gecici.0, &yol).map_err(|e| e.to_string())?;
    Ok(json!({"tamam":true,"yol":yol,"rapor":rapor}))
}

fn api_sina(kok: &Path, g: &Value) -> Sonuc {
    let ayar = ayarlar(kok)?;
    let ortam = ortam(kok);
    let mut sonuclar = Vec::new();
    for s in ayar["senaryolar"]
        .as_array()
        .ok_or("API senaryosu yok.")?
        .iter()
        .take(100)
    {
        if g["ad"].as_str().is_some_and(|a| s["ad"] != a) {
            continue;
        }
        let benzetim = matches!(s["kip"].as_str(), Some("cevrimdisi" | "hatali-yanit"));
        let url = if benzetim {
            benzetim_sunucusu(s["kip"] == "cevrimdisi")?
        } else {
            yerlestir(s["url"].as_str().ok_or("İstek adresi eksik.")?, &ortam)?
        };
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("Yalnızca HTTP/HTTPS adresleri desteklenir.".into());
        }
        let gecici = arsiv::Gecici(crate::derleme::gecici_klasor("api-sinama")?);
        let baslik = gecici.0.join("basliklar");
        let mut basliklar = String::new();
        if let Some(b) = s["basliklar"].as_object() {
            for (ad, v) in b {
                let v = yerlestir(v.as_str().ok_or("Başlık değeri metin olmalı.")?, &ortam)?;
                if ad.contains(['\r', '\n', ':']) || v.contains(['\r', '\n']) {
                    return Err("Geçersiz HTTP başlığı.".into());
                }
                basliklar.push_str(&format!("{ad}: {v}\n"));
            }
        }
        dosya::atomik_yaz(&baslik, basliklar.as_bytes()).map_err(|e| e.to_string())?;
        let govde = gecici.0.join("govde");
        dosya::atomik_yaz(
            &govde,
            yerlestir(s["govde"].as_str().unwrap_or(""), &ortam)?.as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        let yanit = gecici.0.join("yanit");
        let mut k = crate::komut("curl");
        k.args([
            "--silent",
            "--show-error",
            "--max-time",
            "15",
            "--max-filesize",
            "1048576",
            "--proto",
            "=http,https",
            "--request",
            s["yontem"].as_str().unwrap_or("GET"),
            "--header",
        ])
        .arg(format!("@{}", baslik.display()))
        .arg("--output")
        .arg(&yanit)
        .args(["--write-out", "%{http_code}"]);
        if s["govde"].as_str().is_some_and(|s| !s.is_empty()) {
            k.arg("--data-binary").arg(format!("@{}", govde.display()));
        }
        if url.contains(['\r', '\n', '\0']) {
            return Err("İstek adresi tek satır olmalı.".into());
        }
        let ayar = gecici.0.join("curl.ayar");
        let adres = url.replace('\\', "\\\\").replace('"', "\\\"");
        dosya::atomik_yaz(&ayar, format!("url = \"{adres}\"\n").as_bytes())
            .map_err(|e| e.to_string())?;
        k.arg("--config").arg(ayar);
        let (ok, c) = komut(k, 20)?;
        let durum = c.trim().parse::<u64>().unwrap_or(0);
        let metin = std::fs::read_to_string(yanit).unwrap_or_default();
        let beklenen = s["beklenen"].as_u64().unwrap_or(200);
        let icerir = s["icerir"].as_str().unwrap_or("");
        sonuclar.push(json!({"ad":s["ad"],"durum":durum,"benzetim":benzetim,"gecti":(ok || (benzetim && beklenen==0)) && durum==beklenen && metin.contains(icerir),"yanit":gizle(metin,&ortam),"hata":if ok {String::new()} else {gizle(c,&ortam)}}));
    }
    let r = json!({"sonuclar":sonuclar,"gecti":sonuclar.iter().filter(|s|s["gecti"]==true).count(),"kaldi":sonuclar.iter().filter(|s|s["gecti"]!=true).count()});
    kayit(kok, "api-sinama", &r)?;
    Ok(r)
}
fn cihazlari_listele(kok: &Path, hedef: &str) -> Sonuc {
    if hedef == "masaustu" {
        return Ok(
            json!({"tamam":true,"hedef":hedef,"cikti":"Bu bilgisayar: F5 çalıştırma, F6 hata ayıklama ve canlı program günlükleri kullanılabilir."}),
        );
    }
    let mut k = match hedef {
        "android" => {
            let mut k = crate::komut("adb");
            k.args(["devices", "-l"]);
            k
        }
        "ios" => {
            let mut k = crate::komut("xcrun");
            k.args(["simctl", "list", "devices", "available"]);
            k
        }
        _ => return Err("Desteklenmeyen cihaz hedefi.".into()),
    };
    k.current_dir(kok);
    let (ok, s) = komut(k, 10).map_err(|e| {
        format!(
            "{} araçları kullanılamıyor: {e}",
            if hedef == "ios" {
                "Xcode"
            } else {
                "Android SDK / adb"
            }
        )
    })?;
    Ok(json!({"tamam":ok,"hedef":hedef,"cikti":s}))
}
fn cihaz(kok: &Path, g: &Value) -> Sonuc {
    let seri = g["seri"].as_str().unwrap_or("");
    if seri.is_empty()
        || seri.len() > 100
        || !seri
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
    {
        return Err("Geçersiz cihaz kimliği.".into());
    }
    let eylem = g["eylem"].as_str().unwrap_or("");
    let ios = g["hedef"] == "ios";
    let mut k = crate::komut(if ios { "xcrun" } else { "adb" });
    if ios {
        k.arg("simctl");
    } else {
        k.args(["-s", seri]);
    }
    match eylem {
        "gunluk" => {
            if ios {
                k.args([
                    "spawn", seri, "log", "show", "--last", "1m", "--style", "compact",
                ]);
            } else {
                k.args(["logcat", "-d", "-t", "200"]);
            }
        }
        "kur" => {
            let p = kok.join(
                arsiv::goreli(g["apk"].as_str().ok_or("Uygulama yolu eksik.")?)
                    .map_err(|e| e.to_string())?,
            );
            arsiv::baglantisiz(kok, &p).map_err(|e| e.to_string())?;
            if p.extension().and_then(|s| s.to_str()) != Some(if ios { "app" } else { "apk" }) {
                return Err(if ios {
                    "Xcode ile derlenmiş .app klasörü seçin."
                } else {
                    "APK dosyası seçin."
                }
                .into());
            }
            if ios {
                k.args(["install", seri]).arg(p);
            } else {
                k.args(["install", "-r"]).arg(p);
            }
        }
        "baslat" | "durdur" => {
            let uygulama = g["uygulama"].as_str().ok_or("Uygulama kimliği eksik.")?;
            if uygulama.is_empty()
                || !uygulama
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._".contains(&b))
            {
                return Err("Geçersiz uygulama kimliği.".into());
            }
            if ios {
                k.args([
                    if eylem == "durdur" {
                        "terminate"
                    } else {
                        "launch"
                    },
                    seri,
                    uygulama,
                ]);
            } else if eylem == "durdur" {
                k.args(["shell", "am", "force-stop", uygulama]);
            } else {
                k.args([
                    "shell",
                    "monkey",
                    "-p",
                    uygulama,
                    "-c",
                    "android.intent.category.LAUNCHER",
                    "1",
                ]);
            }
        }
        "ac" if ios => {
            k.args(["boot", seri]);
        }
        _ => return Err("Geçersiz cihaz işlemi.".into()),
    }
    let (ok, s) = komut(k, 60)?;
    Ok(json!({"tamam":ok,"cikti":gizle(s,&ortam(kok))}))
}

/// Paylaşılabilen ayarlarda kimlik bilgileri yalnızca ortam başvurusu olarak tutulur.
pub fn ayarlari_dogrula(v: &Value) -> Result<(), String> {
    let profiller = v["profiller"]
        .as_array()
        .ok_or("Profiller listesi gerekli.")?;
    let senaryolar = v["senaryolar"]
        .as_array()
        .ok_or("Senaryolar listesi gerekli.")?;
    if profiller.len() > 50 || senaryolar.len() > 100 {
        return Err("En fazla 50 profil ve 100 senaryo tanımlayın.".into());
    }
    let mut adlar = std::collections::BTreeSet::new();
    for p in profiller {
        let ad = kimlik(p, "ad")?;
        if !adlar.insert(ad) {
            return Err("Profil adları benzersiz olmalı.".into());
        }
        if !["web", "android", "ios", "linux", "windows", "macos"]
            .contains(&p["hedef"].as_str().unwrap_or(""))
        {
            return Err("Profil hedefi geçersiz.".into());
        }
        if !["geliştirme", "sınama", "teslim"].contains(&p["kip"].as_str().unwrap_or("")) {
            return Err("Profil kipi geliştirme, sınama veya teslim olmalı.".into());
        }
        for alan in ["ortam", "acik_ortam"] {
            if !p[alan].is_null() && !p[alan].is_object() {
                return Err(format!("{alan} nesne olmalı."));
            }
        }
        if let Some(o) = p["acik_ortam"].as_object() {
            for (k, v) in o {
                if k.is_empty() || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                    return Err("Açık ortam değişkeni adı geçersiz.".into());
                }
                let s = v.as_str().ok_or("Açık ortam değeri metin olmalı.")?;
                if [
                    "TOKEN", "KEY", "SECRET", "PASSWORD", "ANAHTAR", "SIFRE", "ŞİFRE",
                ]
                .iter()
                .any(|n| k.to_uppercase().contains(n))
                {
                    return Err("Gizli anahtarlar açık istemci ayarlarına eklenemez.".into());
                }
                for parca in s.split("${").skip(1) {
                    let ad = parca.split('}').next().unwrap_or("").to_uppercase();
                    if [
                        "TOKEN", "KEY", "SECRET", "PASSWORD", "ANAHTAR", "SIFRE", "ŞİFRE",
                    ]
                    .iter()
                    .any(|n| ad.contains(n))
                    {
                        return Err("Açık ayarlar gizli değişkenlere başvuramaz.".into());
                    }
                }
            }
        }
        if let Some(o) = p["ortam"].as_object() {
            for (k, v) in o {
                if k.is_empty() || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                    return Err("Ortam değişkeni adı geçersiz.".into());
                }
                let s = v.as_str().ok_or("Ortam değeri metin olmalı.")?;
                let buyuk = k.to_uppercase();
                if [
                    "KEY",
                    "TOKEN",
                    "SECRET",
                    "PASSWORD",
                    "ANAHTAR",
                    "SIFRE",
                    "VERITABANI",
                ]
                .iter()
                .any(|a| buyuk.contains(a))
                    && !(s.starts_with("${") && s.ends_with('}'))
                {
                    return Err("Gizli değer yerine ${DEGISKEN} başvurusu kullanın.".into());
                }
            }
        }
    }
    adlar.clear();
    for s in senaryolar {
        let ad = s["ad"]
            .as_str()
            .filter(|a| !a.is_empty())
            .ok_or("Senaryo adı gerekli.")?;
        if !adlar.insert(ad.to_string()) {
            return Err("Senaryo adları benzersiz olmalı.".into());
        }
        if let Some(b) = s["basliklar"].as_object() {
            for (k, v) in b {
                let v = v.as_str().ok_or("Başlık değeri metin olmalı.")?;
                if ["authorization", "cookie", "x-api-key"].contains(&k.to_lowercase().as_str())
                    && !v.contains("${")
                {
                    return Err("Kimlik başlıkları ${DEGISKEN} kullanmalı.".into());
                }
            }
        }
        let kod = s["beklenen"]
            .as_u64()
            .ok_or("Beklenen durum kodu gerekli.")?;
        if kod != 0 && !(100..=599).contains(&kod) {
            return Err("Geçersiz beklenen durum kodu.".into());
        }
    }
    Ok(())
}

fn benzetim_sunucusu(cevrimdisi: bool) -> Result<String, String> {
    use std::io::Write;
    let l = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    let adres = format!("http://{}/", l.local_addr().map_err(|e| e.to_string())?);
    l.set_nonblocking(true).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let bas = Instant::now();
        while bas.elapsed() < Duration::from_secs(20) {
            match l.accept() {
                Ok((mut c, _)) => {
                    let _ = c.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut b = [0; 8192];
                    let _ = c.read(&mut b);
                    if !cevrimdisi {
                        let govde = "{\"hata\":\"Benzetilmiş servis hatası\"}";
                        let _=write!(c,"HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",govde.len());
                    }
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => break,
            }
        }
    });
    Ok(adres)
}

pub fn profil_ortami(kok: &Path, ad: &str) -> Result<Vec<(String, String)>, String> {
    let p = profil(kok, ad)?;
    let o = ortam(kok);
    let mut sonuc: Vec<(String, String)> = acik_ortam_coz(kok, &p)?
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().into()))
        .collect();
    let gizli = p["ortam"]
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| {
                    Ok((
                        k.clone(),
                        yerlestir(v.as_str().ok_or("Ortam değeri metin olmalı.")?, &o)?,
                    ))
                })
                .collect::<Result<Vec<(String, String)>, String>>()
        })
        .unwrap_or(Ok(Vec::new()))?;
    sonuc.extend(gizli);
    Ok(sonuc)
}

/// Bilinen gizli ortam değerleri veya özel anahtar içeren kaynak paylaşılmaz.
fn paylasimi_dogrula(
    kok: &Path,
    dosyalar: &[Value],
    ortam: &BTreeMap<String, String>,
) -> Result<(), String> {
    let gizli: Vec<&str> = ortam
        .iter()
        .filter(|(k, v)| {
            let k = k.to_uppercase();
            v.len() >= 8
                && [
                    "KEY", "TOKEN", "SECRET", "PASSWORD", "ANAHTAR", "SIFRE", "ŞİFRE",
                ]
                .iter()
                .any(|a| k.contains(a))
        })
        .map(|(_, v)| v.as_str())
        .collect();
    for d in dosyalar {
        let p = kok.join(
            arsiv::goreli(d["yol"].as_str().ok_or("Teslim yolu eksik.")?)
                .map_err(|e| e.to_string())?,
        );
        let b = std::fs::read(&p).map_err(|e| e.to_string())?;
        if gizli
            .iter()
            .any(|s| b.windows(s.len()).any(|w| w == s.as_bytes()))
            || [
                b"-----BEGIN PRIVATE KEY-----".as_slice(),
                b"-----BEGIN RSA PRIVATE KEY-----".as_slice(),
                b"-----BEGIN OPENSSH PRIVATE KEY-----".as_slice(),
            ]
            .iter()
            .any(|s| b.windows(s.len()).any(|w| w == *s))
        {
            return Err(format!(
                "Teslim dosyasında gizli değer bulundu: {}. Değeri ortam değişkenine taşıyın.",
                d["yol"]
            ));
        }
    }
    Ok(())
}

pub fn gizli_degerler(kok: &Path, ek: &[(&str, String)]) -> Vec<String> {
    let mut o = ortam(kok);
    o.extend(ek.iter().map(|(k, v)| (k.to_string(), v.clone())));
    o.into_iter()
        .filter(|(k, v)| {
            let k = k.to_uppercase();
            // Kısa değerler ("1", "true") gizlenmez: program çıktısındaki sıradan sayı ve
            // sözcükler bozulur, üstelik böyle bir değer gerçek bir anahtar olamaz.
            v.chars().count() >= 8
                && v.len() <= 65536
                && [
                    "KEY",
                    "TOKEN",
                    "SECRET",
                    "PASSWORD",
                    "ANAHTAR",
                    "SIFRE",
                    "ŞİFRE",
                    "VERITABANI",
                ]
                .iter()
                .any(|a| k.contains(a))
        })
        .map(|(_, v)| v)
        .collect()
}
fn gizle_json(mut v: Value, o: &BTreeMap<String, String>) -> Value {
    match &mut v {
        Value::String(s) => *s = gizle(std::mem::take(s), o),
        Value::Array(l) => {
            for v in l {
                *v = gizle_json(std::mem::take(v), o);
            }
        }
        Value::Object(m) => {
            for v in m.values_mut() {
                *v = gizle_json(std::mem::take(v), o);
            }
        }
        _ => (),
    }
    v
}

pub fn gizle_sonuc(kok: &Path, v: Value) -> Value {
    gizle_json(v, &ortam(kok))
}
pub fn derleme_kaydi(kok: &Path, v: &Value) -> Result<(), String> {
    kayit(kok, "derleme", v)?;
    kayit(kok, "basarili-derleme", v)
}

fn acik_ortam_coz(kok: &Path, p: &Value) -> Sonuc {
    let o = ortam(kok);
    let gizli = gizli_degerler(kok, &[]);
    let mut sonuc = serde_json::Map::new();
    if let Some(m) = p["acik_ortam"].as_object() {
        for (ad, v) in m {
            let buyuk = ad.to_uppercase();
            if [
                "TOKEN", "KEY", "SECRET", "PASSWORD", "ANAHTAR", "SIFRE", "ŞİFRE",
            ]
            .iter()
            .any(|s| buyuk.contains(s))
            {
                return Err("Kimlik bilgileri açık istemci ayarlarına eklenemez.".into());
            }
            let deger = yerlestir(v.as_str().ok_or("Açık ortam değeri metin olmalı.")?, &o)?;
            if deger.contains(['\n', '\r', '\0']) {
                return Err("Açık ortam değeri tek satır olmalı.".into());
            }
            if gizli
                .iter()
                .filter(|s| s.len() >= 8)
                .any(|s| deger.contains(s.as_str()))
            {
                return Err("Açık ortam değeri gizli anahtar içeriyor.".into());
            }
            sonuc.insert(ad.clone(), json!(deger));
        }
    }
    Ok(Value::Object(sonuc))
}
pub fn profil_acik_ortami(kok: &Path, ad: &str) -> Sonuc {
    acik_ortam_coz(kok, &profil(kok, ad)?)
}
