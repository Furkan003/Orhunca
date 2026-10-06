//! Yapay zekâ ajanları için ortak araçlar: dil rehberi, kod denetleme, biçimlendirme
//! ve süre sınırlı çalıştırma. `orhunca mcp` (MCP sunucusu) ve Stüdyo asistanı
//! bunları kullanır.

use crate::{bicimlendirici, derleme};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Ajanlara verilen dil rehberi: kısa bir giriş (docs/ajan-girisi.md), ardından
/// docs/dil-rehberi.md ve docs/arayuz.md. Site aynısını llms-full.txt olarak sunar.
pub fn rehber() -> String {
    format!(
        "{}\n{}\n\n{}",
        include_str!("../docs/ajan-girisi.md"),
        include_str!("../docs/dil-rehberi.md"),
        include_str!("../docs/arayuz.md")
    )
}

/// Her istekte gönderilen kısa rehber: dilin özü (docs/ajan-ozeti.md) ve ayrıntılı
/// bölümlerin adları. Tam rehber yaklaşık 7 bin belirteç tutar; özet bunun dörtte biri
/// kadardır. Ajan emin olmadığı konuda `rehber_bolumu` ile ilgili bölümü okur.
pub fn kisa_rehber() -> String {
    let adlar: Vec<String> = bolumler().into_iter().map(|(b, _)| b).collect();
    format!(
        "{}\nRehberin bölümleri: {}.\n",
        include_str!("../docs/ajan-ozeti.md").trim_end(),
        adlar.join(", ")
    )
}

/// Tam rehberin bölümleri (başlık, metin): dil rehberinin ve arayüz rehberinin `###` başlıkları.
pub fn bolumler() -> Vec<(String, String)> {
    let mut l: Vec<(String, String)> = Vec::new();
    let mut ekle = |belge: &str, ilk: Option<&str>| {
        let mut baslik = ilk.map(String::from);
        let mut metin = String::new();
        for satir in belge.lines() {
            if let Some(b) = satir.strip_prefix("### ") {
                if let Some(onceki) = baslik.take() {
                    l.push((onceki, metin.trim().to_string()));
                }
                baslik = Some(b.trim().to_string());
                metin.clear();
            } else if baslik.is_some() && !satir.starts_with("# ") && !satir.starts_with("[←") {
                metin.push_str(satir);
                metin.push('\n');
            }
        }
        if let Some(b) = baslik {
            l.push((b, metin.trim().to_string()));
        }
    };
    ekle(include_str!("../docs/dil-rehberi.md"), None);
    ekle(include_str!("../docs/arayuz.md"), Some("Arayüz dili"));
    // Tam rehberde olmayan ama ajanların sık ihtiyaç duyduğu belgeler
    ekle(include_str!("../docs/oyun.md"), Some("Oyun yapmak"));
    ekle(
        include_str!("../docs/mobil.md"),
        Some("Telefon uygulamaları"),
    );
    l
}

fn kucuk(m: &str) -> String {
    m.chars()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            c => c.to_lowercase().next().unwrap_or(c),
        })
        .collect()
}

/// Adı (ya da adının bir parçası) verilen bölümü döndürür; "hepsi" tam rehberi verir.
/// Bulunamazsa bölüm adlarını listeler.
pub fn rehber_bolumu(ad: &str) -> String {
    let aranan = kucuk(ad.trim());
    if aranan.is_empty() || aranan == "hepsi" || aranan == "tamamı" {
        return rehber();
    }
    let l = bolumler();
    let bulunan: Vec<String> = l
        .iter()
        .filter(|(b, _)| kucuk(b).contains(&aranan) || aranan.contains(&kucuk(b)))
        .map(|(b, m)| format!("### {b}\n\n{m}"))
        .collect();
    if bulunan.is_empty() {
        let adlar: Vec<&str> = l.iter().map(|(b, _)| b.as_str()).collect();
        return format!(
            "'{ad}' adlı bölüm yok. Bölümler: {}. Tam rehber için: hepsi",
            adlar.join(", ")
        );
    }
    bulunan.join("\n\n")
}

/// Ajana giden uzun metinleri kırpar: başı ve sonu kalır (hata çoğu zaman sondadır).
pub fn kirp(m: &str, en_cok: usize) -> String {
    let n = m.chars().count();
    if n <= en_cok {
        return m.to_string();
    }
    let bas: String = m.chars().take(en_cok * 2 / 3).collect();
    let son: String = m.chars().skip(n - en_cok / 3).collect();
    format!(
        "{bas}\n… ({} karakter atlandı) …\n{son}",
        n - bas.chars().count() - son.chars().count()
    )
}

/// Kaynağı denetler: derleme hatası varsa onu, yoksa yazım uyarılarını döndürür.
/// `dosya` verilirse (çok dosyalı projeler için) o dosya denetlenir.
pub fn denetle(kod: Option<&str>, dosya: Option<&Path>) -> Result<String, String> {
    let (yol, gecici) = hazirla(kod, dosya)?;
    let kaynak = std::fs::read_to_string(&yol).unwrap_or_default();
    let sonuc = match derleme::yukle(&yol) {
        Err(h) if gecici.is_some() => format!("HATA\n{}", kisa_yol(&h.metin, &yol)),
        Err(h) => format!("HATA\n{}", h.metin),
        Ok(_) => {
            let uyarilar: Vec<String> = bicimlendirici::uyarilar(&kaynak)
                .into_iter()
                .map(|u| format!("{}:{}: uyarı: {}", u.konum.satir, u.konum.sutun, u.mesaj))
                .collect();
            if uyarilar.is_empty() {
                "Hata yok.".into()
            } else {
                format!("Hata yok. Uyarılar:\n{}", uyarilar.join("\n"))
            }
        }
    };
    temizle(gecici);
    Ok(sonuc)
}

pub fn bicimlendir(kod: &str) -> String {
    bicimlendirici::bicimlendir(kod)
}

/// Çalıştırma sonucu.
pub struct Calisma {
    pub cikti: String,
    pub cikis_kodu: Option<i32>,
    pub zaman_asimi: bool,
    /// Derleme hatası (program hiç çalışmadı)
    pub derleme_hatasi: Option<String>,
}

impl Calisma {
    /// Ajanın okuyacağı özet metin.
    pub fn metin(&self) -> String {
        if let Some(h) = &self.derleme_hatasi {
            return format!("DERLEME HATASI\n{h}");
        }
        let mut m = String::new();
        if self.zaman_asimi {
            m.push_str("Program süre sınırında bitmedi ve durduruldu (sonsuz döngü, girdi bekleyen bir döngü ya da web sunucusu olabilir).\n");
        } else {
            m.push_str(&format!(
                "Çıkış kodu: {}\n",
                self.cikis_kodu.map_or("—".into(), |k| k.to_string())
            ));
        }
        m.push_str("Çıktı:\n");
        // Uzun çıktı ajanın belirteçlerini boşa harcamasın diye kırpılır.
        m.push_str(&if self.cikti.is_empty() {
            "(boş)".into()
        } else {
            kirp(&self.cikti, EN_COK_AJAN_CIKTISI)
        });
        m
    }
}

/// Ajana gösterilen çıktının en fazla bu kadar karakteri gönderilir.
const EN_COK_AJAN_CIKTISI: usize = 6000;

/// Çıktının en fazla bu kadar baytı tutulur.
const EN_COK_CIKTI: usize = 32 * 1024;

/// Programı derler ve `sure` kadar çalıştırır; `girdi` programın standart girdisine
/// verilir (`oku()`). Arayüz programları çalıştırılmaz, yalnızca derlenir.
pub fn calistir(
    kod: Option<&str>,
    dosya: Option<&Path>,
    girdi: &str,
    sure: Duration,
) -> Result<Calisma, String> {
    let (yol, gecici) = hazirla(kod, dosya)?;
    let mut sonuc = calistir_yol(&yol, girdi, sure);
    if gecici.is_some() {
        if let Ok(c) = &mut sonuc {
            c.cikti = kisa_yol(&c.cikti, &yol);
            c.derleme_hatasi = c.derleme_hatasi.as_deref().map(|h| kisa_yol(h, &yol));
        }
    }
    temizle(gecici);
    sonuc
}

fn calistir_yol(yol: &Path, girdi: &str, sure: Duration) -> Result<Calisma, String> {
    let hatali = |h: String| Calisma {
        cikti: String::new(),
        cikis_kodu: None,
        zaman_asimi: false,
        derleme_hatasi: Some(h),
    };
    let program = match derleme::yukle(yol) {
        Ok(p) => p,
        Err(h) => return Ok(hatali(h.metin)),
    };
    let klasor = derleme::gecici_klasor("ajan-calistir")?;
    if program.arayuz_programi() {
        let r = derleme::derle_web(yol, &klasor.join("sayfa.html"));
        temizle(Some(klasor));
        return Ok(match r {
            Err(h) => hatali(h.metin),
            Ok(()) => Calisma {
                cikti: "Bu bir arayüz programı: hatasız derlendi. Arayüz programları pencerede açılır; kullanıcı Stüdyo'da F5 ile ya da 'orhunca çalıştır' ile açabilir.".into(),
                cikis_kodu: Some(0),
                zaman_asimi: false,
                derleme_hatasi: None,
            },
        });
    }
    let cikti = klasor.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    if let Err(h) = derleme::derle(yol, &cikti, None) {
        temizle(Some(klasor));
        return Ok(hatali(h.metin));
    }
    let calisma_klasoru = yol.parent().filter(|p| !p.as_os_str().is_empty());
    let mut komut = Command::new(&cikti);
    komut
        .current_dir(calisma_klasoru.unwrap_or(&klasor))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut c = komut
        .spawn()
        .map_err(|e| format!("program çalıştırılamadı: {e}"))?;
    if let Some(mut g) = c.stdin.take() {
        let girdi = girdi.to_string();
        std::thread::spawn(move || {
            let _ = g.write_all(girdi.as_bytes());
        });
    }
    let okuyucu = |a: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut v = Vec::new();
            if let Some(mut a) = a {
                let mut parca = [0u8; 8192];
                while let Ok(n) = a.read(&mut parca) {
                    if n == 0 {
                        break;
                    }
                    if v.len() < EN_COK_CIKTI {
                        v.extend_from_slice(&parca[..n]);
                    }
                }
            }
            v
        })
    };
    let o1 = okuyucu(c.stdout.take().map(|a| Box::new(a) as Box<dyn Read + Send>));
    let o2 = okuyucu(c.stderr.take().map(|a| Box::new(a) as Box<dyn Read + Send>));
    let bas = Instant::now();
    let (durum, zaman_asimi) = loop {
        if let Some(d) = c.try_wait().map_err(|e| e.to_string())? {
            break (Some(d), false);
        }
        if bas.elapsed() > sure {
            let _ = c.kill();
            let _ = c.wait();
            break (None, true);
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut v = o1.join().unwrap_or_default();
    v.extend(o2.join().unwrap_or_default());
    let mut metin = String::from_utf8_lossy(&v).replace('\r', "");
    if metin.len() > EN_COK_CIKTI {
        let mut s = EN_COK_CIKTI;
        while !metin.is_char_boundary(s) {
            s -= 1;
        }
        metin.truncate(s);
        metin.push_str("\n… (çıktı kısaltıldı)");
    }
    temizle(Some(klasor));
    Ok(Calisma {
        cikti: metin,
        cikis_kodu: durum.and_then(|d| d.code()),
        zaman_asimi,
        derleme_hatasi: None,
    })
}

/// Kod verildiyse geçici bir dosyaya yazar; dosya verildiyse onu kullanır.
fn hazirla(kod: Option<&str>, dosya: Option<&Path>) -> Result<(PathBuf, Option<PathBuf>), String> {
    match (kod, dosya) {
        (Some(k), _) => {
            let klasor = derleme::gecici_klasor("ajan")?;
            let yol = klasor.join("program.ohc");
            std::fs::write(&yol, k).map_err(|e| e.to_string())?;
            Ok((yol, Some(klasor)))
        }
        (None, Some(d)) => {
            if !d.is_file() {
                return Err(format!("'{}' bulunamadı", d.display()));
            }
            Ok((d.to_path_buf(), None))
        }
        (None, None) => Err("'kod' ya da 'dosya' verilmeli".into()),
    }
}

/// Geçici dosyanın tam yolu yerine yalnızca adı gösterilir.
fn kisa_yol(metin: &str, yol: &Path) -> String {
    metin.replace(&yol.display().to_string(), "program.ohc")
}

fn temizle(klasor: Option<PathBuf>) {
    if let Some(k) = klasor {
        let _ = std::fs::remove_dir_all(k);
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn calistirir_ve_denetler() {
        let c = calistir(
            Some("ad = oku()\n(\"Merhaba \" + ad)'yı yaz.\n"),
            None,
            "Ayşe\n",
            Duration::from_secs(20),
        )
        .unwrap();
        assert!(c.cikti.contains("Merhaba Ayşe"), "{}", c.metin());
        assert_eq!(c.cikis_kodu, Some(0));

        let d = denetle(Some("x = 5\nx'i yazz.\n"), None).unwrap();
        assert!(d.starts_with("HATA"), "{d}");
        assert_eq!(denetle(Some("5'i yaz.\n"), None).unwrap(), "Hata yok.");

        let s = calistir(
            Some("x = 0\nx 1'den küçükken:\n    x = 0\n"),
            None,
            "",
            Duration::from_millis(1500),
        )
        .unwrap();
        assert!(s.zaman_asimi);
        assert!(rehber().contains("Hâl ekleri"));
    }
}

#[cfg(test)]
mod sinamalar_rehber {
    use super::*;

    #[test]
    fn kisa_rehber_kisadir_ve_bolumleri_listeler() {
        let k = kisa_rehber();
        assert!(
            k.len() * 3 < rehber().len(),
            "{} / {}",
            k.len(),
            rehber().len()
        );
        for (b, m) in bolumler() {
            assert!(k.contains(&b), "{b}");
            assert!(!m.is_empty(), "{b} boş");
        }
    }

    #[test]
    fn bolum_aranir() {
        assert!(rehber_bolumu("modeller").contains("zorunlu"));
        assert!(rehber_bolumu("STANDART kütüphane").contains("büyük_harf"));
        assert!(rehber_bolumu("oyun").contains("oyun_alanı"));
        assert!(rehber_bolumu("yok-böyle").contains("Bölümler:"));
        assert_eq!(rehber_bolumu("hepsi"), rehber());
    }

    #[test]
    fn ozetteki_kod_derlenir() {
        let ozet = include_str!("../docs/ajan-ozeti.md");
        let kod: String = ozet
            .split("```orhunca\n")
            .skip(1)
            .map(|p| p.split("```").next().unwrap_or(""))
            .collect();
        assert!(!kod.is_empty());
        let s = denetle(Some(&kod), None).unwrap();
        assert!(s.starts_with("Hata yok"), "{s}");
    }

    #[test]
    fn uzun_cikti_kirpilir() {
        let m: String = (0..5000).map(|i| format!("{i}\n")).collect();
        let k = kirp(&m, 600);
        assert!(
            k.len() < 800 && k.starts_with("0\n") && k.ends_with("4999\n"),
            "{k}"
        );
        assert_eq!(kirp("kısa", 600), "kısa");
    }
}
