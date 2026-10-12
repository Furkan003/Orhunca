//! Yerel geçmiş: Stüdyo'nun kaydettiği her dosyanın son hâlleri, proje klasörünün dışında
//! (ayar klasöründe) saklanır. Kod yanlışlıkla silinse ya da bozulsa bile önceki bir hâle
//! dönülebilir; Git bilmeyen yeni başlayanlar için bir güvenlik ağıdır.
//!
//! Her dosyanın kayıtları `gecmis/<yolun özeti>/` klasöründedir: `yol.txt` asıl yolu,
//! `<milisaniye>.txt` dosyaları o anki içeriği tutar. Sık kaydetmede (otomatik kaydetme)
//! kayıtlar birleştirilmez; son 100 sürüm korunur.

use super::depo;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Dosya başına en çok bu kadar kayıt tutulur.
const EN_COK_KAYIT: usize = 100;
/// Bundan eski kayıtlar silinir.
const EN_UZUN_MS: u128 = 30 * 24 * 60 * 60 * 1000;
/// Bundan büyük dosyaların geçmişi tutulmaz.
const EN_BUYUK_DOSYA: usize = 1024 * 1024;

fn simdi() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|s| s.as_millis())
        .unwrap_or(0)
}

/// Yolun 64 bitlik FNV-1a özeti; klasör adı olarak kullanılır.
fn ozet(yol: &Path) -> String {
    let tam = std::fs::canonicalize(yol).unwrap_or_else(|_| yol.to_path_buf());
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in tam.to_string_lossy().as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn klasor(yol: &Path) -> PathBuf {
    depo::ayar_klasoru().join("gecmis").join(ozet(yol))
}

/// Kayıtların zamanları, yeniden eskiye.
fn zamanlar(k: &Path) -> Vec<u128> {
    let mut l: Vec<u128> = std::fs::read_dir(k)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|g| {
            g.file_name()
                .to_str()?
                .strip_suffix(".txt")?
                .parse::<u128>()
                .ok()
        })
        .collect();
    l.sort_unstable_by(|a, b| b.cmp(a));
    l
}

/// Dosya yazılmadan önce çağrılır: geçmişi hiç yoksa diskteki ilk hâli saklanır
/// (Stüdyo'da yapılan ilk değişiklikten önceki hâle de dönülebilsin).
pub fn ilk_hali_sakla(yol: &Path) -> std::io::Result<()> {
    match std::fs::read_to_string(yol) {
        Ok(eski) => kaydet(yol, &eski),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn yaz(k: &Path, yol: &Path, zaman: u128, icerik: &str) -> std::io::Result<()> {
    super::dosya::ozel_klasor(k)?;
    let yol_dosyasi = k.join("yol.txt");
    if !yol_dosyasi.exists() {
        super::dosya::atomik_yaz(&yol_dosyasi, yol.to_string_lossy().as_bytes())?;
    }
    super::dosya::atomik_yaz(&k.join(format!("{zaman}.txt")), icerik.as_bytes())
}

/// Her farklı sürümü saklar. Hata API yanıtında uyarı olarak gösterilir.
pub fn kaydet(yol: &Path, icerik: &str) -> std::io::Result<()> {
    if icerik.len() > EN_BUYUK_DOSYA {
        return Err(std::io::Error::other(
            "1 MiB üzerindeki dosyanın yerel geçmişi tutulamadı.",
        ));
    }
    let k = klasor(yol);
    let l = zamanlar(&k);
    if let Some(&son) = l.first() {
        if std::fs::read_to_string(k.join(format!("{son}.txt")))?.as_str() == icerik {
            return Ok(());
        }
    }
    let zaman = simdi().max(l.first().copied().unwrap_or(0) + 1);
    yaz(&k, yol, zaman, icerik)?;
    for (i, z) in zamanlar(&k).into_iter().enumerate() {
        if i >= EN_COK_KAYIT || zaman.saturating_sub(z) > EN_UZUN_MS {
            std::fs::remove_file(k.join(format!("{z}.txt")))?;
        }
    }
    Ok(())
}

/// Dosyanın kayıtları (yeniden eskiye): (zaman, bayt).
pub fn liste(yol: &Path) -> Vec<(u128, u64)> {
    let k = klasor(yol);
    zamanlar(&k)
        .into_iter()
        .map(|z| {
            let boyut = std::fs::metadata(k.join(format!("{z}.txt")))
                .map(|m| m.len())
                .unwrap_or(0);
            (z, boyut)
        })
        .collect()
}

pub fn oku(yol: &Path, zaman: u128) -> Option<String> {
    std::fs::read_to_string(klasor(yol).join(format!("{zaman}.txt"))).ok()
}
