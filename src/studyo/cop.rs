//! Aynı dosya sisteminde taşıma ile geri alınabilir silme; ikili dosyalar da korunur.
use super::{arsiv, depo, dosya};
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
    dosya::ozel_klasor(p.parent().unwrap())?;
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
        let ara = yol.with_file_name(format!(".orhunca-silinecek-{kimlik}"));
        let bilgi = json!({ "yol": goreli, "zaman": depo::simdi(), "kimlik": kimlik, "ara_yol": ara.strip_prefix(kok).map_err(io::Error::other)? });
        dosya::atomik_yaz(&kayit.join("bilgi.json"), &serde_json::to_vec(&bilgi)?)?;
        match dosya::tasi_yollari(yol, &kayit.join("icerik")) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                // Önce aynı diskte ayır. Kesinti olursa bilgi.json ara yolu gösterir.
                dosya::tasi_yollari(yol, &ara)?;
                if let Err(e) = arsiv::kopyala(&ara, &kayit.join("icerik"), false, false) {
                    if std::fs::symlink_metadata(yol).is_err() {
                        let _ = dosya::tasi_yollari(&ara, yol);
                    }
                    return Err(e);
                }
                kaldir(&ara)?;
            }
            Err(e) => return Err(e),
        }
        #[cfg(unix)]
        {
            std::fs::File::open(yol.parent().unwrap_or(kok))?.sync_all()?;
            std::fs::File::open(&kayit)?.sync_all()?;
        }
        Ok(kimlik.clone())
    })();
    if sonuc.is_err()
        && !kayit.join("icerik").exists()
        && !yol
            .with_file_name(format!(".orhunca-silinecek-{kimlik}"))
            .exists()
    {
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
            if std::fs::symlink_metadata(kayit.join("icerik")).is_ok()
                || ara_yol(kok, &bilgi).is_ok_and(|p| p.exists())
            {
                bilgi["boyut"] = json!(boyut(&kayit.join("icerik"))?);
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
    let (d, ad) = dosya::kapsam(hedef)?;
    if let Some(ust) = ad.parent().filter(|p| !p.as_os_str().is_empty()) {
        d.create_dir_all(ust)?;
    }
    if !ust.canonicalize()?.starts_with(kok) {
        return Err(io::Error::other("Proje dışına geri yüklenemez."));
    }
    let _kilit = dosya::kilitle(hedef)?;
    if std::fs::symlink_metadata(hedef).is_ok() {
        return Err(io::Error::other(
            "Bu yolda bir dosya var; geri yükleme mevcut dosyayı ezmez.",
        ));
    }
    let (_, bilgi) = kaydi_oku(kok, kimlik)?;
    let ara = ara_yol(kok, &bilgi).ok();
    let kaynak = if kayit.join("icerik").exists() {
        kayit.join("icerik")
    } else {
        ara.clone()
            .filter(|p| p.exists())
            .ok_or_else(|| io::Error::other("Kurtarma içeriği bulunamadı."))?
    };
    match dosya::tasi_yollari(&kaynak, hedef) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            let gecici = hedef.with_file_name(format!(".orhunca-kurtar-{}", dosya::kimlik()?));
            arsiv::kopyala(&kaynak, &gecici, false, false)?;
            if let Err(e) = dosya::tasi_yollari(&gecici, hedef) {
                let _ = kaldir(&gecici);
                return Err(e);
            }
            kaldir(&kaynak)?;
        }
        Err(e) => return Err(e),
    }
    if let Some(ara) = ara.filter(|p| p.exists()) {
        kaldir(&ara)?;
    }
    #[cfg(unix)]
    std::fs::File::open(ust)?.sync_all()?;
    let _ = std::fs::remove_dir_all(kayit);
    Ok(())
}

fn kaldir(p: &Path) -> io::Result<()> {
    if let Ok((d, ad)) = dosya::kapsam(p) {
        return if d.symlink_metadata(&ad)?.is_dir() {
            d.remove_dir_all(ad)
        } else {
            d.remove_file(ad)
        };
    }
    if std::fs::symlink_metadata(p)?.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    }
}
fn ara_yol(kok: &Path, bilgi: &Value) -> io::Result<PathBuf> {
    let p = kok.join(arsiv::goreli(
        bilgi["ara_yol"]
            .as_str()
            .ok_or_else(|| io::Error::other("Ara yol yok."))?,
    )?);
    arsiv::baglantisiz(kok, &p)?;
    if !p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .starts_with(".orhunca-silinecek-")
    {
        return Err(io::Error::other("Geçersiz ara yol."));
    }
    Ok(p)
}
fn boyut(p: &Path) -> io::Result<u64> {
    let m = match std::fs::symlink_metadata(p) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    if !m.is_dir() || m.file_type().is_symlink() {
        return Ok(m.len());
    }
    let mut toplam = 0;
    for g in std::fs::read_dir(p)? {
        toplam += boyut(&g?.path())?;
    }
    Ok(toplam)
}
pub fn kalici_sil(kok: &Path, kimlik: &str) -> io::Result<()> {
    let (kayit, bilgi) = kaydi_oku(kok, kimlik)?;
    if ara_yol(kok, &bilgi).is_ok_and(|p| p.exists()) {
        return Err(io::Error::other("Yarım kalan taşıma önce geri alınmalı."));
    }
    std::fs::remove_dir_all(kayit)
}
