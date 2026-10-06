//! `orhunca etkileşim`: satır satır deneme ortamı (REPL).
//!
//! Orhunca derlenen bir dildir; her girdi, önceki tanımlarla birlikte küçük bir
//! program olarak derlenip çalıştırılır. Tanımlar (işlev, fiil, model, seçenek,
//! sabit) ve atamalar oturumda kalır; yalnızca yeni çıktı gösterilir. Tek başına
//! yazılan bir ifadenin değeri ekrana yazılır (`3 + 4` → `7`).

use crate::{bicimlendirici, derleme, ekler::Hal};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const YARDIM: &str = "\
Orhunca etkileşim — yazdığınız her satır hemen çalışır.
  3 + 4                     ifadenin değeri yazılır
  ad = \"Ayşe\"                atamalar ve tanımlar oturumda kalır
  işlev kare(n):            ':' ile biten satırdan sonra blok yazın; boş satırla bitirin
:yardım  bu yardım     :liste  oturumdaki kod     :sil  oturumu temizle     :çık  çıkış";

/// Oturumda kalan bir girdi mi (tanım ya da atama)?
fn kalici_mi(girdi: &str) -> bool {
    let ilk = girdi.trim_start();
    let ilk_kelime = ilk
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    if matches!(
        ilk_kelime,
        "işlev" | "fiil" | "model" | "seçenek" | "sabit" | "kullan" | "durum" | "bileşen"
    ) {
        return true;
    }
    // `x = ...`, `x += ...`, `x: tip = ...` (karşılaştırma `==` değil)
    let satir = ilk.lines().next().unwrap_or("");
    let ad_sonu = satir
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(satir.len());
    let kalan = satir[ad_sonu..].trim_start();
    ad_sonu > 0
        && (kalan.starts_with("+=")
            || kalan.starts_with("-=")
            || (kalan.starts_with('=') && !kalan.starts_with("=="))
            || (kalan.starts_with(':') && satir.contains('=')))
}

/// Tek başına bir ifade mi (yazdırılmak üzere sarılabilir)?
fn ifade_mi(girdi: &str) -> bool {
    let g = girdi.trim();
    !g.is_empty()
        && !g.contains('\n')
        && !g.ends_with('.')
        && !g.ends_with(':')
        && !kalici_mi(g)
        && !g.starts_with('#')
}

struct Oturum {
    klasor: PathBuf,
    kalici: Vec<String>,
    /// Kalıcı kodun son çalıştırmadaki çıktısı (yeni çıktıyı ayırmak için)
    onceki_cikti: String,
}

enum Sonuc {
    Tamam { cikti: String, hata: String },
    Derleme(String),
}

impl Oturum {
    fn calistir(&self, kaynak: &str) -> Sonuc {
        let dosya = self.klasor.join("etkileşim.ohc");
        let program = self.klasor.join(if cfg!(windows) { "p.exe" } else { "p" });
        if let Err(e) = std::fs::write(&dosya, kaynak) {
            return Sonuc::Derleme(e.to_string());
        }
        if let Err(h) = derleme::derle(&dosya, &program, None) {
            return Sonuc::Derleme(h.metin);
        }
        match crate::komut(&program).current_dir(&self.klasor).output() {
            Ok(c) => Sonuc::Tamam {
                cikti: String::from_utf8_lossy(&c.stdout).into_owned(),
                hata: String::from_utf8_lossy(&c.stderr).into_owned(),
            },
            Err(e) => Sonuc::Derleme(format!("program çalıştırılamadı: {e}")),
        }
    }

    fn kod(&self, ek: &str) -> String {
        let mut k = self.kalici.join("\n");
        if !k.is_empty() {
            k.push('\n');
        }
        k.push_str(ek);
        k.push('\n');
        k
    }

    /// Bir girdiyi çalıştırır; gösterilecek metni döndürür.
    fn isle(&mut self, girdi: &str) -> String {
        // Tek başına ifade: değeri yazdırılır; olmazsa olduğu gibi denenir.
        if ifade_mi(girdi) {
            let g = girdi.trim();
            let sarili = if g.contains(' ') {
                format!("({g})")
            } else {
                g.to_string()
            };
            let ek = bicimlendirici::ek_oner(&sarili, Hal::Belirtme);
            if let Sonuc::Tamam { cikti, hata } =
                self.calistir(&self.kod(&format!("{sarili}'{ek} yaz.")))
            {
                return self.yeni_cikti(&cikti, &hata);
            }
        }
        match self.calistir(&self.kod(girdi)) {
            Sonuc::Derleme(h) => self.hata_satirlari(&h),
            Sonuc::Tamam { cikti, hata } => {
                let goster = self.yeni_cikti(&cikti, &hata);
                if kalici_mi(girdi) && hata.is_empty() {
                    self.kalici.push(girdi.to_string());
                    self.onceki_cikti = cikti;
                }
                goster
            }
        }
    }

    /// Derleme hatasındaki satır numaraları girdiye göre yazılır (oturumdaki
    /// önceki satırlar sayılmaz).
    fn hata_satirlari(&self, h: &str) -> String {
        let fark: usize = self.kalici.iter().map(|k| k.lines().count()).sum();
        let yol = self
            .klasor
            .join("etkileşim.ohc")
            .to_string_lossy()
            .into_owned();
        h.lines()
            .map(|l| {
                if let Some(k) = l.strip_prefix(&format!("  --> {yol}:")) {
                    let (n, kalan) = k.split_once(':').unwrap_or((k, ""));
                    let n = n
                        .parse::<usize>()
                        .map(|n| n.saturating_sub(fark))
                        .unwrap_or(0);
                    return format!("  --> girdi:{n}:{kalan}");
                }
                let rakam = l.chars().take_while(char::is_ascii_digit).count();
                if rakam > 0 && l[rakam..].starts_with(" |") {
                    let n: usize = l[..rakam].parse().unwrap_or(0);
                    return format!("{} |{}", n.saturating_sub(fark), &l[rakam + 2..]);
                }
                l.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    fn yeni_cikti(&self, cikti: &str, hata: &str) -> String {
        let yeni = cikti
            .strip_prefix(self.onceki_cikti.as_str())
            .unwrap_or(cikti);
        format!("{yeni}{hata}")
    }
}

pub fn calistir() -> Result<(), String> {
    let klasor = derleme::gecici_klasor("etkilesim")?;
    let mut o = Oturum {
        klasor: klasor.clone(),
        kalici: Vec::new(),
        onceki_cikti: String::new(),
    };
    println!(
        "Orhunca {} etkileşim — yardım için :yardım, çıkmak için :çık",
        crate::SURUM
    );
    let stdin = std::io::stdin();
    let mut satirlar = stdin.lock().lines();
    let mut tampon = String::new();
    loop {
        print!("{}", if tampon.is_empty() { "› " } else { "… " });
        let _ = std::io::stdout().flush();
        let Some(Ok(satir)) = satirlar.next() else {
            println!();
            break;
        };
        if tampon.is_empty() {
            match satir.trim() {
                "" => continue,
                ":çık" | ":cik" | ":q" | "çık" => break,
                ":yardım" | ":yardim" | ":?" => {
                    println!("{YARDIM}");
                    continue;
                }
                ":liste" => {
                    for k in &o.kalici {
                        println!("{k}");
                    }
                    continue;
                }
                ":sil" => {
                    o.kalici.clear();
                    o.onceki_cikti.clear();
                    println!("oturum temizlendi");
                    continue;
                }
                _ => {}
            }
        }
        // Blok: ':' ile biten satırdan sonra boş satıra kadar okunur.
        if !tampon.is_empty() || satir.trim_end().ends_with(':') {
            if !satir.trim().is_empty() {
                tampon.push_str(&satir);
                tampon.push('\n');
                continue;
            }
        } else {
            tampon = satir;
        }
        let girdi = std::mem::take(&mut tampon);
        let cikti = o.isle(girdi.trim_end());
        print!("{cikti}");
        if !cikti.is_empty() && !cikti.ends_with('\n') {
            println!();
        }
    }
    let _ = std::fs::remove_dir_all(Path::new(&klasor));
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn girdi_turleri() {
        assert!(kalici_mi("x = 5"));
        assert!(kalici_mi("toplam += 1"));
        assert!(kalici_mi("işlev kare(n):\n    döndür n * n"));
        assert!(!kalici_mi("x == 5"));
        assert!(!kalici_mi("x'i yaz."));
        assert!(ifade_mi("3 + 4"));
        assert!(!ifade_mi("x'i yaz."));
        assert!(!ifade_mi("y = 2"));
    }
}
