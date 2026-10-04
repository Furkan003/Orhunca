//! Dersler (dersler/*.md): bütün örnekler ve alıştırma çözümleri derlenir; çıktısı
//! verilenler verilen girdiyle çalıştırılıp beklenen çıktıyla karşılaştırılır.

use std::path::Path;
use std::process::{Command, Stdio};

struct Blok {
    tur: String,
    kod: String,
}

fn bloklar(metin: &str) -> Vec<Blok> {
    let mut v = Vec::new();
    let mut satirlar = metin.lines();
    while let Some(s) = satirlar.next() {
        if let Some(tur) = s.strip_prefix("```") {
            let mut kod = String::new();
            for k in satirlar.by_ref() {
                if k.starts_with("```") {
                    break;
                }
                kod.push_str(k);
                kod.push('\n');
            }
            v.push(Blok {
                tur: tur.trim().to_string(),
                kod,
            });
        }
    }
    v
}

fn calistir(kod: &str, girdi: &str, beklenen: Option<&str>, ad: &str) {
    let klasor = std::env::temp_dir().join(format!("orhunca-ders-{}", std::process::id()));
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join("ders.ohc");
    std::fs::write(&dosya, kod).unwrap();
    let komut = if beklenen.is_some() {
        "çalıştır"
    } else {
        "denetle"
    };
    let mut c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg(komut)
        .arg(&dosya)
        .current_dir(&klasor)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    c.stdin.take().unwrap().write_all(girdi.as_bytes()).unwrap();
    let c = c.wait_with_output().unwrap();
    let cikti = String::from_utf8_lossy(&c.stdout);
    assert!(
        c.status.success(),
        "{ad}: {}\n{kod}",
        String::from_utf8_lossy(&c.stderr)
    );
    if let Some(b) = beklenen {
        assert_eq!(cikti, b, "{ad}\n{kod}");
    }
}

#[test]
fn ders_ornekleri_ve_cozumleri_calisir() {
    let klasor = Path::new(env!("CARGO_MANIFEST_DIR")).join("dersler");
    let mut sayi = 0;
    let mut dosyalar: Vec<_> = std::fs::read_dir(&klasor)
        .unwrap()
        .map(|g| g.unwrap().path())
        .filter(|p| p.extension().is_some_and(|u| u == "md"))
        .collect();
    dosyalar.sort();
    for yol in dosyalar {
        let ad = yol.file_name().unwrap().to_string_lossy().into_owned();
        let b = bloklar(&std::fs::read_to_string(&yol).unwrap());
        for (i, blok) in b.iter().enumerate() {
            match blok.tur.as_str() {
                // Örnek: hemen ardından gelen girdi/çıktı bloklarıyla
                "orhunca" => {
                    let mut girdi = "";
                    let mut cikti = None;
                    for s in b[i + 1..]
                        .iter()
                        .take_while(|s| s.tur == "girdi" || s.tur == "cikti")
                    {
                        if s.tur == "girdi" {
                            girdi = &s.kod;
                        } else {
                            cikti = Some(s.kod.as_str());
                        }
                    }
                    calistir(&blok.kod, girdi, cikti, &format!("{ad}: örnek {i}"));
                    sayi += 1;
                }
                // Çözüm: alıştırmanın (başlangıçtan sonraki) girdi/çıktısıyla
                "orhunca cozum" => {
                    let bas = b[..i]
                        .iter()
                        .rposition(|s| s.tur == "orhunca baslangic")
                        .expect("çözümden önce başlangıç kodu olmalı");
                    let girdi = b[bas..i]
                        .iter()
                        .find(|s| s.tur == "girdi")
                        .map(|s| s.kod.as_str())
                        .unwrap_or("");
                    let cikti = b[bas..i]
                        .iter()
                        .find(|s| s.tur == "cikti")
                        .map(|s| s.kod.as_str());
                    calistir(&blok.kod, girdi, cikti, &format!("{ad}: çözüm"));
                    sayi += 1;
                }
                _ => {}
            }
        }
    }
    assert!(sayi > 40, "{sayi}");
}
