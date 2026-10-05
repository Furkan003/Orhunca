//! Güncelleme: GitHub'daki son sürümü denetler, bu bilgisayardaki kurulum türüne uygun
//! dosyayı indirir, SHA256SUMS.txt ile doğrular ve kurar (ya da kurulumu başlatır).
//!
//! - Windows (Stüdyo): yeni kurulum dosyası çalıştırılır; Stüdyo kapanır.
//! - Linux .deb / .rpm: paket, sistemin yazılım kurucusuyla açılır (yönetici parolası).
//! - Linux AppImage: dosya yerinde değiştirilir.
//! - macOS: .dmg açılır.
//! - Yalnızca komut (kur.sh / kur.ps1 ya da arşivden): `orhunca` dosyası yerinde değişir.
//!
//! Okullar denetimi kapatabilir: `ORHUNCA_GUNCELLEME=kapali` (ortam değişkeni) ya da
//! Stüdyo Ayarlar → Güncellemeler.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const DEPO: &str = "Furkan003/Orhunca";

/// Yöneticinin güncelleme denetimini kapatıp kapatmadığı.
pub fn kapali_mi() -> bool {
    std::env::var("ORHUNCA_GUNCELLEME").is_ok_and(|d| {
        matches!(
            d.to_lowercase().as_str(),
            "kapali" | "kapalı" | "0" | "hayir" | "hayır" | "off" | "false"
        )
    })
}

pub struct Varlik {
    pub ad: String,
    pub adres: String,
    pub boyut: u64,
}

pub struct Surum {
    /// "0.7.0" (baştaki v olmadan)
    pub surum: String,
    pub notlar: String,
    pub sayfa: String,
    pub varliklar: Vec<Varlik>,
}

fn curl(args: &[&str]) -> Result<Vec<u8>, String> {
    let c = Command::new("curl")
        .args(["-sSfL", "--proto", "=https,http", "-A", "Orhunca"])
        .args(args)
        .output()
        .map_err(|_| "curl bulunamadı; güncelleme için curl gerekli".to_string())?;
    if !c.status.success() {
        return Err(format!(
            "indirilemedi: {}",
            String::from_utf8_lossy(&c.stderr).trim()
        ));
    }
    Ok(c.stdout)
}

/// Son yayımlanan sürüm. `ORHUNCA_GUNCELLEME_ADRESI` başka bir adres ya da (sınama
/// için) yerel bir JSON dosyası olabilir.
pub fn son_surum() -> Result<Surum, String> {
    let adres = std::env::var("ORHUNCA_GUNCELLEME_ADRESI")
        .unwrap_or_else(|_| format!("https://api.github.com/repos/{DEPO}/releases/latest"));
    let metin = if adres.starts_with("http://") || adres.starts_with("https://") {
        curl(&[
            "--max-time",
            "15",
            "-H",
            "Accept: application/vnd.github+json",
            "--",
            &adres,
        ])?
    } else {
        std::fs::read(&adres).map_err(|e| format!("'{adres}' okunamadı: {e}"))?
    };
    let j: serde_json::Value =
        serde_json::from_slice(&metin).map_err(|e| format!("sürüm bilgisi bozuk: {e}"))?;
    let etiket = j["tag_name"].as_str().ok_or("sürüm bilgisi bozuk")?;
    Ok(Surum {
        surum: etiket.trim_start_matches('v').to_string(),
        notlar: j["body"].as_str().unwrap_or("").to_string(),
        sayfa: j["html_url"].as_str().unwrap_or("").to_string(),
        varliklar: j["assets"]
            .as_array()
            .map(|l| {
                l.iter()
                    .filter_map(|v| {
                        Some(Varlik {
                            ad: v["name"].as_str()?.to_string(),
                            adres: v["browser_download_url"].as_str()?.to_string(),
                            boyut: v["size"].as_u64().unwrap_or(0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn parcalar(s: &str) -> Vec<u64> {
    s.trim_start_matches('v')
        .split(['.', '-', '+'])
        .take(3)
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

/// `yeni` sürümü `simdiki`den yeni mi? ("0.10.0" > "0.9.3")
pub fn daha_yeni(yeni: &str, simdiki: &str) -> bool {
    let (a, b) = (parcalar(yeni), parcalar(simdiki));
    for i in 0..3 {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kurulum {
    WindowsKurulum,
    Deb,
    Rpm,
    AppImage(PathBuf),
    MacDmg,
    /// Yalnızca `orhunca` komutu: dosyanın yeri
    Komut(PathBuf),
}

/// Bu bilgisayardaki kurulumun türü. `masaustu`: Stüdyo masaüstü uygulaması içinde mi.
pub fn kurulum_turu(masaustu: bool) -> Kurulum {
    let exe = std::env::current_exe().unwrap_or_default();
    if let Some(a) = std::env::var_os("APPIMAGE") {
        return Kurulum::AppImage(PathBuf::from(a));
    }
    let paket_ici = exe.starts_with("/usr") || exe.starts_with("/opt");
    if cfg!(windows) {
        // Kurulum dosyasıyla kurulduysa yanında Stüdyo vardır.
        let studyo = exe
            .parent()
            .is_some_and(|k| k.join("orhunca-studyo.exe").exists());
        if masaustu || studyo {
            return Kurulum::WindowsKurulum;
        }
        return Kurulum::Komut(exe);
    }
    if cfg!(target_os = "macos") {
        if masaustu || exe.to_string_lossy().contains(".app/") {
            return Kurulum::MacDmg;
        }
        return Kurulum::Komut(exe);
    }
    if masaustu || paket_ici {
        if Path::new("/usr/bin/dpkg").exists() {
            return Kurulum::Deb;
        }
        if Path::new("/usr/bin/rpm").exists() {
            return Kurulum::Rpm;
        }
    }
    Kurulum::Komut(exe)
}

/// Kurulum türü için sürümdeki dosyanın adı.
pub fn varlik_adi(k: &Kurulum) -> String {
    match k {
        Kurulum::WindowsKurulum => "Orhunca-Studyo-Windows-Kurulum.exe".into(),
        Kurulum::Deb if cfg!(target_arch = "aarch64") => "orhunca-studyo_arm64.deb".into(),
        Kurulum::Deb => "orhunca-studyo_amd64.deb".into(),
        Kurulum::Rpm => "orhunca-studyo.x86_64.rpm".into(),
        Kurulum::AppImage(_) => "Orhunca-Studyo-Linux.AppImage".into(),
        Kurulum::MacDmg => "Orhunca-Studyo-macOS.dmg".into(),
        Kurulum::Komut(_) => {
            if cfg!(windows) {
                "orhunca-windows-x86_64.zip".into()
            } else if cfg!(target_os = "macos") {
                "orhunca-macos.tar.gz".into()
            } else if cfg!(target_arch = "aarch64") {
                "orhunca-linux-aarch64.tar.gz".into()
            } else {
                "orhunca-linux-x86_64.tar.gz".into()
            }
        }
    }
}

/// SHA-256 (FIPS 180-4)
pub fn sha256(veri: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut m = veri.to_vec();
    let bit = (veri.len() as u64).wrapping_mul(8);
    m.push(0x80);
    while m.len() % 64 != 56 {
        m.push(0);
    }
    m.extend_from_slice(&bit.to_be_bytes());
    for blok in m.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                blok[4 * i],
                blok[4 * i + 1],
                blok[4 * i + 2],
                blok[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut d = h;
        for i in 0..64 {
            let s1 = d[4].rotate_right(6) ^ d[4].rotate_right(11) ^ d[4].rotate_right(25);
            let ch = (d[4] & d[5]) ^ (!d[4] & d[6]);
            let t1 = d[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = d[0].rotate_right(2) ^ d[0].rotate_right(13) ^ d[0].rotate_right(22);
            let maj = (d[0] & d[1]) ^ (d[0] & d[2]) ^ (d[1] & d[2]);
            let t2 = s0.wrapping_add(maj);
            d = [
                t1.wrapping_add(t2),
                d[0],
                d[1],
                d[2],
                d[3].wrapping_add(t1),
                d[4],
                d[5],
                d[6],
            ];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(d[i]);
        }
    }
    let mut cikti = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        cikti[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    cikti
}

fn onaltilik(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Dosyayı indirir ve sürümdeki SHA256SUMS.txt ile doğrular; yerel yolunu döndürür.
pub fn indir(s: &Surum, ad: &str) -> Result<PathBuf, String> {
    let v = s
        .varliklar
        .iter()
        .find(|v| v.ad == ad)
        .ok_or_else(|| format!("Orhunca {} sürümünde '{ad}' dosyası yok", s.surum))?;
    let ozet = s
        .varliklar
        .iter()
        .find(|v| v.ad == "SHA256SUMS.txt")
        .ok_or("sürümde SHA256SUMS.txt yok; dosya doğrulanamaz")?;
    let ozetler =
        String::from_utf8_lossy(&curl(&["--max-time", "30", "--", &ozet.adres])?).into_owned();
    let beklenen = ozetler
        .lines()
        .find_map(|l| {
            let (o, d) = l.split_once(char::is_whitespace)?;
            (d.trim().trim_start_matches('*') == ad).then(|| o.to_lowercase())
        })
        .ok_or_else(|| format!("SHA256SUMS.txt içinde '{ad}' yok"))?;
    let klasor = std::env::temp_dir().join(format!("orhunca-guncelleme-{}", s.surum));
    std::fs::create_dir_all(&klasor).map_err(|e| e.to_string())?;
    let yol = klasor.join(ad);
    let yol_m = yol.to_string_lossy().into_owned();
    curl(&["--max-time", "1800", "-o", &yol_m, "--", &v.adres])?;
    let veri = std::fs::read(&yol).map_err(|e| e.to_string())?;
    if onaltilik(&sha256(&veri)) != beklenen {
        let _ = std::fs::remove_file(&yol);
        return Err(
            "indirilen dosya doğrulanamadı (SHA-256 uyuşmuyor); güncelleme yapılmadı".into(),
        );
    }
    Ok(yol)
}

/// İndirilen dosyayı kurar. Dönen metin kullanıcıya gösterilir; `true`: uygulama
/// kapanmalı (kurulum başladı ya da dosyası değişti).
pub fn kur(k: &Kurulum, dosya: &Path) -> Result<(String, bool), String> {
    let ac = |komut: &str, args: &[&Path]| -> Result<(), String> {
        Command::new(komut)
            .args(args)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("'{komut}' çalıştırılamadı: {e}"))
    };
    match k {
        Kurulum::WindowsKurulum => {
            ac(&dosya.to_string_lossy(), &[])?;
            Ok(("Kurulum başladı; Stüdyo kapanıyor.".into(), true))
        }
        Kurulum::Deb | Kurulum::Rpm => {
            ac("xdg-open", &[dosya])?;
            Ok((
                format!(
                    "Paket yazılım kurucusunda açıldı; kurduktan sonra Stüdyo'yu yeniden başlatın. (Komutla: sudo {} {})",
                    if *k == Kurulum::Deb { "apt install" } else { "dnf install" },
                    dosya.display()
                ),
                false,
            ))
        }
        Kurulum::MacDmg => {
            ac("open", &[dosya])?;
            Ok((
                "Disk görüntüsü açıldı: Orhunca Stüdyo'yu Uygulamalar klasörüne sürükleyin.".into(),
                false,
            ))
        }
        Kurulum::AppImage(hedef) => {
            dosya_degistir(dosya, hedef)?;
            Ok(("AppImage güncellendi; Stüdyo'yu yeniden açın.".into(), true))
        }
        Kurulum::Komut(hedef) => {
            let klasor = dosya.parent().unwrap_or(Path::new(".")).join("acilan");
            let _ = std::fs::remove_dir_all(&klasor);
            std::fs::create_dir_all(&klasor).map_err(|e| e.to_string())?;
            let c = Command::new("tar")
                .arg("-xf")
                .arg(dosya)
                .arg("-C")
                .arg(&klasor)
                .status()
                .map_err(|_| "arşiv açılamadı: tar bulunamadı".to_string())?;
            if !c.success() {
                return Err("arşiv açılamadı".into());
            }
            let ad = if cfg!(windows) {
                "orhunca.exe"
            } else {
                "orhunca"
            };
            dosya_degistir(&klasor.join(ad), hedef)?;
            Ok((format!("güncellendi: {}", hedef.display()), true))
        }
    }
}

/// Çalışan dosyanın yerine yenisini koyar (Windows'ta çalışan dosya önce yeniden adlandırılır).
fn dosya_degistir(yeni: &Path, hedef: &Path) -> Result<(), String> {
    let gecici = hedef.with_extension("yeni");
    std::fs::copy(yeni, &gecici).map_err(|e| format!("'{}' yazılamadı: {e}", gecici.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&gecici, std::fs::Permissions::from_mode(0o755));
    }
    if cfg!(windows) {
        let eski = hedef.with_extension("eski.exe");
        let _ = std::fs::remove_file(&eski);
        std::fs::rename(hedef, &eski).map_err(|e| format!("eski dosya taşınamadı: {e}"))?;
    }
    std::fs::rename(&gecici, hedef).map_err(|e| format!("'{}' yazılamadı: {e}", hedef.display()))
}

/// `orhunca güncelle [--denetle]`
pub fn komut(args: &[String]) -> Result<(), String> {
    let s = son_surum()?;
    if !daha_yeni(&s.surum, crate::SURUM) {
        println!("Orhunca güncel ({}).", crate::SURUM);
        return Ok(());
    }
    println!("Yeni sürüm: Orhunca {} (kurulu: {})", s.surum, crate::SURUM);
    if args.iter().any(|a| a == "--denetle") {
        println!("Güncellemek için: orhunca güncelle");
        return Ok(());
    }
    let k = kurulum_turu(false);
    let ad = varlik_adi(&k);
    println!("İndiriliyor: {ad}");
    let dosya = indir(&s, &ad)?;
    println!("Doğrulandı (SHA-256).");
    let (mesaj, _) = kur(&k, &dosya)?;
    println!("{mesaj}");
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn surum_karsilastirma() {
        assert!(daha_yeni("0.7.0", "0.6.0"));
        assert!(daha_yeni("v0.10.0", "0.9.9"));
        assert!(daha_yeni("1.0.0", "0.99.0"));
        assert!(!daha_yeni("0.6.0", "0.6.0"));
        assert!(!daha_yeni("0.5.9", "0.6.0"));
    }

    #[test]
    fn sha256_dogru() {
        assert_eq!(
            onaltilik(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            onaltilik(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let uzun = vec![b'a'; 1_000_000];
        assert_eq!(
            onaltilik(&sha256(&uzun)),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }
}
