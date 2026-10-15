//! SQLite şema geçişleri ve özgün projeden ayrı demo verisi.
use super::{arsiv, dosya, gelistirme, veritabani};
use serde_json::{json, Value};
use std::path::Path;

fn metin(v: &Value, ad: &str) -> String {
    v[ad].as_str().unwrap_or("").to_string()
}
fn tirnak(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
fn plan(kok: &Path) -> Result<Value, String> {
    if !matches!(veritabani::veri_turu(kok).as_str(), "json" | "sqlite") {
        return Err(
            "Geçiş paneli SQLite içindir; uzak veritabanlarında motorun geçiş aracını kullanın."
                .into(),
        );
    }
    let defter = veritabani::sorgula(
        kok,
        "SELECT name FROM sqlite_master WHERE type='table' AND name='orhunca_gecisler'",
    )?;
    let uygulanmis = if defter.as_array().is_some_and(|l| !l.is_empty()) {
        veritabani::sorgula(
            kok,
            "SELECT ad, ozet, zaman FROM orhunca_gecisler ORDER BY ad",
        )?
    } else {
        json!([])
    };
    let klasor = kok.join("gecisler");
    arsiv::baglantisiz(kok, &klasor).map_err(|e| e.to_string())?;
    let mut gecisler = Vec::new();
    if klasor.exists() {
        for g in std::fs::read_dir(&klasor).map_err(|e| e.to_string())? {
            let g = g.map_err(|e| e.to_string())?;
            let ad = g.file_name().to_string_lossy().into_owned();
            if !ad.ends_with(".sql") || ad.ends_with(".geri.sql") {
                continue;
            }
            arsiv::baglantisiz(kok, &g.path()).map_err(|e| e.to_string())?;
            let sql = std::fs::read_to_string(g.path()).map_err(|e| e.to_string())?;
            let geri = g.path().with_file_name(ad.replace(".sql", ".geri.sql"));
            arsiv::baglantisiz(kok, &geri).map_err(|e| e.to_string())?;
            let geri = std::fs::read_to_string(geri).ok();
            let ozet = dosya::surum(sql.as_bytes());
            let eski = uygulanmis
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["ad"] == ad);
            if eski.is_some_and(|v| v["ozet"] != ozet) {
                return Err(format!("Uygulanmış geçiş değişmiş: {ad}"));
            }
            gecisler.push(
                json!({"ad":ad,"sql":sql,"geri":geri,"ozet":ozet,"uygulandi":eski.is_some()}),
            );
        }
    }
    gecisler.sort_by_key(|v| metin(v, "ad"));
    for g in uygulanmis.as_array().unwrap() {
        if !gecisler.iter().any(|v| v["ad"] == g["ad"]) {
            return Err(format!("Uygulanmış geçiş dosyası eksik: {}", g["ad"]));
        }
    }
    let sema = veritabani::sorgula(
        kok,
        "SELECT name, sql FROM sqlite_master WHERE type='table' ORDER BY name",
    )?;
    let (modeller, model_hatasi) = match model_onizle(kok) {
        Ok(v) => (v, None),
        Err(e) => (json!([]), Some(e)),
    };
    let surum = dosya::surum(json!([gecisler, sema, modeller]).to_string().as_bytes());
    Ok(
        json!({"gecisler":gecisler,"sema":sema,"modeller":modeller,"model_hatasi":model_hatasi,"surum":surum}),
    )
}
/// Model alanlarını mevcut tabloyla karşılaştırır; kaynak kodu çalıştırmaz.
fn model_onizle(kok: &Path) -> Result<Value, String> {
    let giris = crate::derleme::proje_girisi(kok)?;
    arsiv::baglantisiz(kok, &giris).map_err(|e| e.to_string())?;
    let p = crate::derleme::yukle(&giris).map_err(|h| h.metin)?;
    let mut sonuc = Vec::new();
    for m in &p.modeller {
        if !p
            .dosyalar
            .get(m.konum.dosya)
            .is_some_and(|y| Path::new(y).starts_with(kok))
        {
            continue;
        }
        let kolonlar = veritabani::sorgula(kok, &format!("PRAGMA table_info({})", tirnak(&m.ad)))?;
        let kolonlar = kolonlar.as_array().ok_or("Tablo alanları okunamadı.")?;
        let eklenen = m
            .alanlar
            .iter()
            .filter(|a| !kolonlar.iter().any(|k| k["name"] == a.ad))
            .map(|a| a.ad.clone())
            .collect::<Vec<_>>();
        let kaldirilan = kolonlar
            .iter()
            .filter(|k| !m.alanlar.iter().any(|a| k["name"] == a.ad))
            .filter_map(|k| k["name"].as_str())
            .collect::<Vec<_>>();
        let alanlar=m.alanlar.iter().map(|a|json!({"ad":a.ad,"tip":a.tip.to_string(),"veritabani_tipi":kolonlar.iter().find(|k|k["name"]==a.ad).map(|k|k["type"].clone())})).collect::<Vec<_>>();
        sonuc.push(json!({"ad":m.ad,"alanlar":alanlar,"eklenen":eklenen,"kaldirilan":kaldirilan,"tablo_var":!kolonlar.is_empty()}));
    }
    Ok(json!(sonuc))
}

fn sql_dogrula(sql: &str) -> Result<(), String> {
    // Geçiş dosyası işlem sınırını ve geçiş defterini yönetemez.
    let kelimeler = sql
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>();
    if kelimeler.iter().any(|k| {
        [
            "COMMIT",
            "ROLLBACK",
            "BEGIN",
            "END",
            "VACUUM",
            "ATTACH",
            "DETACH",
            "PRAGMA",
            "ORHUNCA_GECISLER",
        ]
        .contains(&k.as_str())
    }) {
        return Err("Geçiş dosyası işlem yönetimi, PRAGMA veya geçiş defteri içeremez.".into());
    }
    Ok(())
}
fn demo_hazirla(kok: &Path, hedef: &Path) -> Result<(), String> {
    let mut dosyalar = arsiv::kopyala(kok, hedef, true, true).map_err(|e| e.to_string())?;
    let ornek = kok.join("demo").join("veri");
    if ornek.exists() {
        arsiv::baglantisiz(kok, &ornek).map_err(|e| e.to_string())?;
        for mut d in
            arsiv::kopyala(&ornek, &hedef.join("veri"), false, false).map_err(|e| e.to_string())?
        {
            d["yol"] = json!(format!("veri/{}", d["yol"].as_str().unwrap_or("")));
            dosyalar.push(d);
        }
    }
    // Dosya adı süzgeci (.env, *.pem) yetmez: anahtar başka bir dosyaya da yazılmış olabilir.
    gelistirme::demo_dogrula(kok, hedef, &dosyalar)?;
    dosya::atomik_yaz(
        &hedef.join(".env"),
        b"ORHUNCA_VERITABANI=sqlite\nORHUNCA_DEMO=1\n",
    )
    .map_err(|e| e.to_string())?;
    dosya::atomik_yaz(&hedef.join("DEMO.txt"),"Bu kopya özgün veriden ayrıdır. Örnek veriler demo/veri klasöründen alınır. Sıfırlamak için asıl projeden yeni demo oluşturun. WebAssembly çıktısı internet olmadan açılabilir; uzak API çağrıları için API senaryolarının benzetim kipini kullanın.\n".as_bytes()).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn islem(kok: &Path, g: &Value) -> Result<Value, String> {
    if g["islem"] == "demo" {
        let id = dosya::kimlik().map_err(|e| e.to_string())?;
        let hedef = kok.join(format!("kurtarilan-demo-{id}"));
        // Kopya önce gizli bir hazırlık klasöründe denetlenir; gizli değer bulunursa
        // ortada yarım ya da sızdıran bir demo kalmaz.
        let hazirlik = kok.join(format!(".orhunca-demo-{id}"));
        let sonuc = demo_hazirla(kok, &hazirlik)
            .and_then(|()| std::fs::rename(&hazirlik, &hedef).map_err(|e| e.to_string()));
        if let Err(e) = sonuc {
            let _ = std::fs::remove_dir_all(&hazirlik);
            return Err(e);
        }
        return Ok(json!({"yol":hedef,"tamam":true}));
    }
    let _kilit =
        dosya::kilitle(&gelistirme::alan(kok)?.join("gecis-kilidi")).map_err(|e| e.to_string())?;
    let p = plan(kok)?;
    if g["islem"] == "gecis-onizle" {
        return Ok(p);
    }
    if p["surum"] != g["surum"] {
        return Err("Geçiş planı değişti; önizlemeyi yenileyin.".into());
    }
    let geri = g["islem"] == "gecis-geri";
    let mut secilen = p["gecisler"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["uygulandi"] == geri)
        .collect::<Vec<_>>();
    if geri {
        secilen = secilen.last().copied().into_iter().collect();
    }
    if secilen.is_empty() {
        return Ok(json!({"tamam":true,"adet":0}));
    }
    let mut sql = String::from("BEGIN IMMEDIATE;\nCREATE TABLE IF NOT EXISTS orhunca_gecisler (ad TEXT PRIMARY KEY, ozet TEXT NOT NULL, zaman TEXT NOT NULL);\n");
    for v in &secilen {
        let s = if geri {
            v["geri"].as_str().ok_or("Geri dönüş dosyası eksik.")?
        } else {
            v["sql"].as_str().unwrap()
        };
        sql_dogrula(s)?;
        sql.push_str(s);
        sql.push_str(";\n");
        if geri {
            sql.push_str(&format!(
                "DELETE FROM orhunca_gecisler WHERE ad={};\n",
                tirnak(&metin(v, "ad"))
            ));
        } else {
            sql.push_str(&format!(
                "INSERT INTO orhunca_gecisler VALUES ({},{},CURRENT_TIMESTAMP);\n",
                tirnak(&metin(v, "ad")),
                tirnak(&metin(v, "ozet"))
            ));
        }
    }
    sql.push_str("COMMIT;");
    let yedek = gelistirme::alan(kok)?.join(format!(
        "veritabani-{}.sqlite",
        dosya::kimlik().map_err(|e| e.to_string())?
    ));
    veritabani::sorgula(
        kok,
        &format!("VACUUM INTO {}", tirnak(&yedek.to_string_lossy())),
    )?;
    veritabani::sorgula(kok, &sql)?;
    Ok(json!({"tamam":true,"adet":secilen.len(),"yedek":yedek,"plan":plan(kok)?}))
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    fn kalanlar(kok: &Path) -> Vec<String> {
        std::fs::read_dir(kok)
            .unwrap()
            .flatten()
            .map(|g| g.file_name().to_string_lossy().into_owned())
            .filter(|a| a.contains("demo"))
            .collect()
    }

    #[test]
    fn demo_baska_dosyaya_yazilmis_anahtari_tasimaz() {
        let kok = crate::derleme::gecici_klasor("demo-gizli")
            .unwrap()
            .canonicalize()
            .unwrap();
        std::fs::write(kok.join(".env"), "API_KEY=sahte-anahtar-123456\n").unwrap();
        std::fs::write(kok.join("ana.ohc"), "yaz(1)\n").unwrap();
        std::fs::write(kok.join("config.json"), r#"{"k":"sahte-anahtar-123456"}"#).unwrap();
        let h = islem(&kok, &json!({"islem":"demo"})).unwrap_err();
        assert!(h.contains("config.json"), "{h}");
        assert!(kalanlar(&kok).is_empty(), "{:?}", kalanlar(&kok));
        // `.env.*` dosyalarındaki değerler de aranır.
        std::fs::write(kok.join("config.json"), "{}").unwrap();
        std::fs::write(kok.join(".env.uretim"), "TOKEN=uretim-gizli-987654\n").unwrap();
        std::fs::write(kok.join("not.txt"), "uretim-gizli-987654").unwrap();
        assert!(islem(&kok, &json!({"islem":"demo"})).is_err());
        assert!(kalanlar(&kok).is_empty());
        // Temiz projede demo oluşur ve gizli dosyalar kopyalanmaz.
        std::fs::remove_file(kok.join("not.txt")).unwrap();
        let v = islem(&kok, &json!({"islem":"demo"})).unwrap();
        let hedef = Path::new(v["yol"].as_str().unwrap());
        assert!(hedef.join("ana.ohc").exists() && hedef.join("DEMO.txt").exists());
        assert!(!hedef.join(".env.uretim").exists());
        std::fs::remove_dir_all(kok).unwrap();
    }
}
