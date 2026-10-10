//! Profil çıkarıcı: `orhunca profil dosya.ohc [-- argümanlar]`.
//!
//! Program hata ayıklama kancalarıyla derlenir ve `ORHUNCA_PROFIL` ile çalıştırılır;
//! çalışma zamanı işlev çağrılarını, sürelerini ve satırların kaç kez çalıştığını bir
//! dosyaya yazar. Bu modül o dosyayı okuyup en çok zaman alan işlevleri ve en sık çalışan
//! satırları gösterir. Kancalar programı yavaşlatır; süreler birbirine göre okunmalıdır.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::derleme;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct IslevOlcumu {
    pub ad: String,
    pub cagri: u64,
    pub toplam_ns: u64,
    pub kendi_ns: u64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SatirOlcumu {
    pub dosya: String,
    pub satir: usize,
    pub kez: u64,
}

#[derive(Debug, Default)]
pub struct Rapor {
    pub islevler: Vec<IslevOlcumu>,
    pub satirlar: Vec<SatirOlcumu>,
}

/// Çalışma zamanının yazdığı dosyayı okur. `dosyalar`: derleyicinin dosya sırası.
pub fn raporu_oku(metin: &str, dosyalar: &[String]) -> Rapor {
    let mut r = Rapor::default();
    for satir in metin.lines() {
        if let Some(k) = satir.strip_prefix("islev ") {
            let p: Vec<&str> = k.split('\t').collect();
            if p.len() == 4 {
                r.islevler.push(IslevOlcumu {
                    ad: p[0].to_string(),
                    cagri: p[1].parse().unwrap_or(0),
                    toplam_ns: p[2].parse().unwrap_or(0),
                    kendi_ns: p[3].parse().unwrap_or(0),
                });
            }
        } else if let Some(k) = satir.strip_prefix("satir ") {
            let p: Vec<&str> = k.split('\t').collect();
            if p.len() == 3 {
                let d: usize = p[0].parse().unwrap_or(0);
                r.satirlar.push(SatirOlcumu {
                    dosya: dosyalar.get(d).cloned().unwrap_or_else(|| d.to_string()),
                    satir: p[1].parse().unwrap_or(0),
                    kez: p[2].parse().unwrap_or(0),
                });
            }
        }
    }
    r.islevler
        .sort_by(|a, b| b.kendi_ns.cmp(&a.kendi_ns).then(a.ad.cmp(&b.ad)));
    r.satirlar.sort_by(|a, b| {
        b.kez
            .cmp(&a.kez)
            .then(a.dosya.cmp(&b.dosya))
            .then(a.satir.cmp(&b.satir))
    });
    r
}

fn ms(ns: u64) -> String {
    format!("{:.2}", ns as f64 / 1e6)
}

fn kaynak_satiri(dosya: &str, satir: usize) -> String {
    std::fs::read_to_string(dosya)
        .ok()
        .and_then(|k| {
            k.lines()
                .nth(satir.saturating_sub(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_default()
}

pub fn raporu_yaz(r: &Rapor, kok: &Path) -> String {
    let mut c = String::new();
    c.push_str("\nİşlevler (kendi süresine göre; toplam = çağırdıkları dahil)\n");
    c.push_str(&format!(
        "{:>10} {:>12} {:>12}  işlev\n",
        "çağrı", "toplam ms", "kendi ms"
    ));
    for f in r.islevler.iter().take(20) {
        c.push_str(&format!(
            "{:>10} {:>12} {:>12}  {}\n",
            f.cagri,
            ms(f.toplam_ns),
            ms(f.kendi_ns),
            f.ad
        ));
    }
    c.push_str("\nEn sık çalışan satırlar\n");
    for s in r.satirlar.iter().take(15) {
        let ad = Path::new(&s.dosya)
            .strip_prefix(kok)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| s.dosya.clone());
        c.push_str(&format!(
            "{:>10}  {}:{}  {}\n",
            s.kez,
            ad,
            s.satir,
            kaynak_satiri(&s.dosya, s.satir)
        ));
    }
    c
}

/// `orhunca profil dosya.ohc [-- argümanlar] [--json]`
pub fn komut(args: &[String]) -> Result<(), String> {
    let (once, sonra) = match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], &args[i + 1..]),
        None => (args, &[][..]),
    };
    let json = once.iter().any(|a| a == "--json");
    let dosya = match once.iter().find(|a| !a.starts_with("--")) {
        Some(d) => PathBuf::from(d),
        None => derleme::proje_girisi(Path::new("."))?,
    };
    let gecici = derleme::gecici_klasor("profil")?;
    let program = gecici.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let rapor_yolu = gecici.join("profil.txt");
    let sonuc = (|| {
        let dosyalar = derleme::derle_ayiklamali(&dosya, &program, &[]).map_err(|h| h.metin)?;
        let durum = Command::new(&program)
            .args(sonra)
            .env("ORHUNCA_PROFIL", &rapor_yolu)
            .env_remove("ORHUNCA_AYIKLA")
            .status()
            .map_err(|e| format!("program çalıştırılamadı: {e}"))?;
        let metin = std::fs::read_to_string(&rapor_yolu).map_err(|_| {
            "profil raporu yazılmadı (program erken sonlanmış olabilir)".to_string()
        })?;
        let r = raporu_oku(&metin, &dosyalar);
        if json {
            let islevler: Vec<_> = r
                .islevler
                .iter()
                .map(|f| serde_json::json!({ "ad": f.ad, "cagri": f.cagri, "toplam_ns": f.toplam_ns, "kendi_ns": f.kendi_ns }))
                .collect();
            let satirlar: Vec<_> = r
                .satirlar
                .iter()
                .map(|s| serde_json::json!({ "dosya": s.dosya, "satir": s.satir, "kez": s.kez }))
                .collect();
            println!(
                "{}",
                serde_json::json!({ "islevler": islevler, "satirlar": satirlar })
            );
        } else {
            let kok = derleme::proje_koku(&dosya);
            eprint!("{}", raporu_yaz(&r, &kok));
            if !durum.success() {
                eprintln!(
                    "\nNot: program {} koduyla bitti.",
                    durum.code().unwrap_or(1)
                );
            }
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&gecici);
    sonuc
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn rapor_okunur_ve_siralanir() {
        let m = "islev ana\t1\t5000000\t1000000\nislev fib\t177\t4000000\t4000000\nsatir 0\t3\t177\nsatir 0\t7\t1\n";
        let r = raporu_oku(m, &["a.ohc".into()]);
        assert_eq!(r.islevler[0].ad, "fib");
        assert_eq!(r.islevler[0].cagri, 177);
        assert_eq!(
            r.satirlar[0],
            SatirOlcumu {
                dosya: "a.ohc".into(),
                satir: 3,
                kez: 177
            }
        );
    }
}
