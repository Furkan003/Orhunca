//! Aynı dosya sisteminde taşıma ile geri alınabilir silme; ikili dosyalar da korunur.
use super::{depo, dosya};
use serde_json::{json, Value};
use std::io;
use std::path::{Component, Path, PathBuf};

fn alan(kok: &Path) -> PathBuf {
    depo::ayar_klasoru()
        .join("silinenler")
        .join(dosya::surum(kok.to_string_lossy().as_bytes()))
}

fn gercek_klasor(p: &Path) -> io::Result<()> {
    let m = std::fs::symlink_metadata(p)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err(io::Error::other(
            "Kurtarma klasörü gerçek bir klasör olmalı.",
        ));
    }
    Ok(())
}

fn depo(kok: &Path) -> io::Result<PathBuf> {
    let p = alan(kok);
    std::fs::create_dir_all(p.parent().unwrap())?;
    match std::fs::create_dir(&p) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700))?;
            }
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    gercek_klasor(&p)?;
    Ok(p)
}

pub fn sil(kok: &Path, yol: &Path) -> io::Result<String> {
    let _kilit = dosya::kilitle(yol)?;
    let goreli = yol.strip_prefix(kok).map_err(io::Error::other)?;
    let kimlik = dosya::kimlik()?;
    let kayit = depo(kok)?.join(&kimlik);
    std::fs::create_dir(&kayit)?;
    let sonuc = (|| {
        let bilgi = json!({ "yol": goreli, "zaman": depo::simdi(), "kimlik": kimlik });
        dosya::atomik_yaz(&kayit.join("bilgi.json"), &serde_json::to_vec(&bilgi)?)?;
        // Kopyala-sil yapılmaz: başarısız taşıma asıl dosyayı korur.
        std::fs::rename(yol, kayit.join("icerik"))?;
        #[cfg(unix)]
        {
            std::fs::File::open(yol.parent().unwrap_or(kok))?.sync_all()?;
            std::fs::File::open(&kayit)?.sync_all()?;
        }
        Ok(kimlik)
    })();
    if sonuc.is_err() && !kayit.join("icerik").exists() {
        let _ = std::fs::remove_dir_all(kayit);
    }
    sonuc
}

fn kaydi_oku(kok: &Path, kimlik: &str) -> io::Result<(PathBuf, Value)> {
    if kimlik.len() != 32 || !kimlik.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(io::Error::other("Geçersiz kurtarma kaydı."));
    }
    let klasor = alan(kok);
    gercek_klasor(&klasor)?;
    let kayit = klasor.join(kimlik);
    gercek_klasor(&kayit)?;
    let bilgi = kayit.join("bilgi.json");
    if std::fs::symlink_metadata(&bilgi)?.file_type().is_symlink() {
        return Err(io::Error::other("Kurtarma bilgisi bağlantı olamaz."));
    }
    Ok((kayit, serde_json::from_slice(&std::fs::read(bilgi)?)?))
}

pub fn liste(kok: &Path) -> io::Result<Vec<Value>> {
    let klasor = alan(kok);
    if !klasor.exists() {
        return Ok(Vec::new());
    }
    gercek_klasor(&klasor)?;
    let mut liste = Vec::new();
    for g in std::fs::read_dir(klasor)? {
        let g = g?;
        let kimlik = g.file_name().to_string_lossy().into_owned();
        if let Ok((kayit, mut bilgi)) = kaydi_oku(kok, &kimlik) {
            if std::fs::symlink_metadata(kayit.join("icerik")).is_ok() {
                bilgi["kimlik"] = json!(kimlik);
                liste.push(bilgi);
            }
        }
    }
    liste.sort_by_key(|v| std::cmp::Reverse(v["zaman"].as_u64().unwrap_or(0)));
    Ok(liste)
}

pub fn hedef(kok: &Path, kimlik: &str) -> io::Result<PathBuf> {
    let (_, bilgi) = kaydi_oku(kok, kimlik)?;
    let yol = Path::new(
        bilgi["yol"]
            .as_str()
            .ok_or_else(|| io::Error::other("Kayıt yolu yok."))?,
    );
    if yol.as_os_str().is_empty() || yol.components().any(|p| !matches!(p, Component::Normal(_))) {
        return Err(io::Error::other("Geçersiz kurtarma yolu."));
    }
    Ok(kok.join(yol))
}

pub fn geri_al(kok: &Path, kimlik: &str, hedef: &Path) -> io::Result<()> {
    let (kayit, _) = kaydi_oku(kok, kimlik)?;
    let ust = hedef
        .parent()
        .ok_or_else(|| io::Error::other("Üst klasör yok."))?;
    std::fs::create_dir_all(ust)?;
    if !ust.canonicalize()?.starts_with(kok) {
        return Err(io::Error::other("Proje dışına geri yüklenemez."));
    }
    let _kilit = dosya::kilitle(hedef)?;
    if std::fs::symlink_metadata(hedef).is_ok() {
        return Err(io::Error::other(
            "Bu yolda bir dosya var; geri yükleme mevcut dosyayı ezmez.",
        ));
    }
    std::fs::rename(kayit.join("icerik"), hedef)?;
    #[cfg(unix)]
    std::fs::File::open(ust)?.sync_all()?;
    let _ = std::fs::remove_dir_all(kayit);
    Ok(())
}
