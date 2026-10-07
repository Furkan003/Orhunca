//! Yerel geçmiş: Stüdyo'nun kaydettiği her dosyanın son hâlleri, proje klasörünün dışında
//! (ayar klasöründe) saklanır. Kod yanlışlıkla silinse ya da bozulsa bile önceki bir hâle
//! dönülebilir; Git bilmeyen yeni başlayanlar için bir güvenlik ağıdır.
//!
//! Her dosyanın kayıtları `gecmis/<yolun özeti>/` klasöründedir: `yol.txt` asıl yolu,
//! `<milisaniye>.txt` dosyaları o anki içeriği tutar. Sık kaydetmede (otomatik kaydetme)
//! bir dakika içindeki kayıtlar tek kayıtta birleşir.

use super::depo;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bu kadar yakın kayıtlar birleşir.
const BIRLESTIRME_MS: u128 = 60_000;
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
pub fn ilk_hali_sakla(yol: &Path) {
    let k = klasor(yol);
    if !zamanlar(&k).is_empty() {
        return;
    }
    if let Ok(eski) = std::fs::read_to_string(yol) {
        let _ = yaz(&k, yol, simdi().saturating_sub(BIRLESTIRME_MS + 1), &eski);
    }
}

fn yaz(k: &Path, yol: &Path, zaman: u128, icerik: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(k)?;
    let yol_dosyasi = k.join("yol.txt");
    if !yol_dosyasi.exists() {
        std::fs::write(yol_dosyasi, yol.to_string_lossy().as_bytes())?;
    }
    std::fs::write(k.join(format!("{zaman}.txt")), icerik)
}

/// Kaydedilen içeriği geçmişe ekler. Hatalar yok sayılır: geçmiş kaydı başarısız diye
/// kullanıcının asıl kaydı engellenmez.
pub fn kaydet(yol: &Path, icerik: &str) {
    if icerik.len() > EN_BUYUK_DOSYA {
        return;
    }
    let k = klasor(yol);
    let simdi = simdi();
    let l = zamanlar(&k);
    if let Some(&son) = l.first() {
        if std::fs::read_to_string(k.join(format!("{son}.txt")))
            .ok()
            .as_deref()
            == Some(icerik)
        {
            return;
        }
        // Bir dakika içindeki kayıtlar birleşir: son kayıt güncel içerikle yer değiştirir.
        // Birleşen kayıt, kendinden önce bir kayıt varsa silinir (ilk hâl korunur).
        if simdi.saturating_sub(son) < BIRLESTIRME_MS && l.len() > 1 {
            let _ = std::fs::remove_file(k.join(format!("{son}.txt")));
        }
    }
    let _ = yaz(&k, yol, simdi, icerik);
    for (i, z) in zamanlar(&k).into_iter().enumerate() {
        if i >= EN_COK_KAYIT || simdi.saturating_sub(z) > EN_UZUN_MS {
            let _ = std::fs::remove_file(k.join(format!("{z}.txt")));
        }
    }
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
