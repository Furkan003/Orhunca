//! Stüdyo kayıtları: içerik sürümü, süreçler arası kilit ve atomik değiştirme.
use super::depo;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn surum(veri: &[u8]) -> String {
    crate::guncelleme::sha256(veri)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn kimlik() -> io::Result<String> {
    let mut b = [0; 16];
    crate::guvenli_rastgele(&mut b).map_err(io::Error::other)?;
    Ok(b.iter().map(|b| format!("{b:02x}")).collect())
}

/// Dosya mevcut değilse gerçek üst klasör kullanılır; bağlantı takma adları aynı kilidi alır.
pub fn kilitle(yol: &Path) -> io::Result<File> {
    let tam = if yol.exists() {
        yol.canonicalize()?
    } else {
        yol.parent()
            .ok_or_else(|| io::Error::other("Üst klasör bulunamadı"))?
            .canonicalize()?
            .join(
                yol.file_name()
                    .ok_or_else(|| io::Error::other("Dosya adı yok"))?,
            )
    };
    let klasor = depo::ayar_klasoru().join("kayit-kilitleri");
    std::fs::create_dir_all(&klasor)?;
    let kilit = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(klasor.join(surum(tam.to_string_lossy().as_bytes())))?;
    kilit.lock()?;
    Ok(kilit)
}

struct Gecici(PathBuf);
impl Drop for Gecici {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Eski dosya, tüm yeni içerik diske yazılana kadar korunur. std::fs::rename,
/// Windows'ta da mevcut dosyayı değiştirir; önce hedefi silmek gerekmez.
pub fn atomik_yaz(yol: &Path, veri: &[u8]) -> io::Result<()> {
    let hedef = if yol.exists() {
        yol.canonicalize()?
    } else {
        yol.to_path_buf()
    };
    let ust = hedef
        .parent()
        .ok_or_else(|| io::Error::other("Üst klasör yok"))?;
    let gecici = Gecici(ust.join(format!(".orhunca-kayit-{}", kimlik()?)));
    let mut ayar = OpenOptions::new();
    ayar.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        ayar.mode(0o600);
    }
    let mut dosya = ayar.open(&gecici.0)?;
    dosya.write_all(veri)?;
    if let Ok(m) = std::fs::metadata(&hedef) {
        dosya.set_permissions(m.permissions())?;
    }
    dosya.sync_all()?;
    drop(dosya);
    std::fs::rename(&gecici.0, &hedef)?;
    #[cfg(unix)]
    File::open(ust)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn atomik_kayitta_okuyucu_yarim_icerik_gormez() {
        let k = std::env::temp_dir().join(format!("orhunca-atomik-{}", kimlik().unwrap()));
        std::fs::create_dir(&k).unwrap();
        let p = k.join("kod");
        let a = vec![b'a'; 128 * 1024];
        let b = vec![b'b'; 128 * 1024];
        atomik_yaz(&p, &a).unwrap();
        std::thread::scope(|scope| {
            let yazici = scope.spawn(|| {
                for i in 0..30 {
                    atomik_yaz(&p, if i % 2 == 0 { &b } else { &a }).unwrap();
                }
            });
            for _ in 0..300 {
                let veri = std::fs::read(&p).unwrap();
                assert!(veri == a || veri == b, "Eksik veya karışık içerik okundu");
            }
            yazici.join().unwrap();
        });
        // Hedef bir klasörse başarısız değiştirme onu veya içindekileri silmez;
        // geçici dosya da geride kalmaz.
        let hedef = k.join("klasor");
        std::fs::create_dir(&hedef).unwrap();
        std::fs::write(hedef.join("koru"), b"koru").unwrap();
        assert!(atomik_yaz(&hedef, b"yeni").is_err());
        assert_eq!(std::fs::read(hedef.join("koru")).unwrap(), b"koru");
        assert_eq!(std::fs::read_dir(&k).unwrap().count(), 2);
        std::fs::remove_dir_all(k).unwrap();
    }
}
