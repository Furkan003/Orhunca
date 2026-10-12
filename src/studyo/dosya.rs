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
    ozel_klasor(&klasor)?;
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

/// Kullanıcıya özel depolar; mevcut bağlantıları izleyerek sır yazılmaz.
pub fn ozel_klasor(yol: &Path) -> io::Result<()> {
    if let Some(ust) = yol.parent() {
        if !ust.exists() {
            ozel_klasor(ust)?;
        }
    }
    let mut kur = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        kur.mode(0o700);
    }
    match kur.create(yol) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e),
    }
    let m = std::fs::symlink_metadata(yol)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err(io::Error::other("Özel veri klasörü bağlantı olamaz."));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(yol, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Açılan köklerin dizin tanıtıcıları tutulur; denetimden sonra yol değiştirilse de
/// okuma/yazma bu yetki sınırının dışına çözülmez.
fn kokler() -> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, cap_std::fs::Dir>> {
    static K: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, cap_std::fs::Dir>>,
    > = std::sync::OnceLock::new();
    K.get_or_init(Default::default)
}
pub fn kok_ac(p: &Path) -> io::Result<()> {
    let mut k = kokler().lock().unwrap();
    if !k.contains_key(p) {
        k.insert(
            p.to_path_buf(),
            cap_std::fs::Dir::open_ambient_dir(p, cap_std::ambient_authority())?,
        );
    }
    Ok(())
}
pub fn sabitle(p: &Path) -> io::Result<(cap_std::fs::Dir, PathBuf)> {
    let k = kokler().lock().unwrap();
    let (kok, d) = k
        .iter()
        .filter(|(kok, _)| p.starts_with(kok))
        .max_by_key(|(kok, _)| kok.as_os_str().len())
        .ok_or_else(|| io::Error::other("Açık proje tanıtıcısı yok."))?;
    let goreli = p.strip_prefix(kok).map_err(io::Error::other)?;
    let tam = match d.canonicalize(goreli) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // Kopuk bağlantı yeni dosya sayılmaz.
            if d.symlink_metadata(goreli).is_ok() {
                return Err(e);
            }
            let ust = d.canonicalize(
                goreli
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new(".")),
            )?;
            ust.join(
                goreli
                    .file_name()
                    .ok_or_else(|| io::Error::other("Dosya adı yok."))?,
            )
        }
        Err(e) => return Err(e),
    };
    let ust = d.open_dir(
        tam.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    Ok((
        ust,
        PathBuf::from(
            tam.file_name()
                .ok_or_else(|| io::Error::other("Dosya adı yok."))?,
        ),
    ))
}
pub fn sinirli_oku(p: &Path) -> io::Result<Vec<u8>> {
    let (d, ad) = sabitle(p)?;
    d.read(ad)
}
pub fn sabit_yaz(d: &cap_std::fs::Dir, ad: &Path, veri: &[u8]) -> io::Result<()> {
    let gecici = format!(".orhunca-kayit-{}", kimlik()?);
    let sonuc = (|| {
        let mut ayar = cap_std::fs::OpenOptions::new();
        ayar.write(true).create_new(true);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            ayar.mode(0o600);
        }
        let mut f = d.open_with(&gecici, &ayar)?;
        f.write_all(veri)?;
        if let Ok(m) = d.metadata(ad) {
            f.set_permissions(m.permissions())?;
        }
        f.sync_all()?;
        drop(f);
        d.rename(&gecici, d, ad)?;
        #[cfg(unix)]
        d.open(".")?.sync_all()?;
        Ok(())
    })();
    if sonuc.is_err() {
        let _ = d.remove_file(&gecici);
    }
    sonuc
}

pub fn kapsam(p: &Path) -> io::Result<(cap_std::fs::Dir, PathBuf)> {
    let k = kokler().lock().unwrap();
    let (kok, d) = k
        .iter()
        .filter(|(kok, _)| p.starts_with(kok))
        .max_by_key(|(kok, _)| kok.as_os_str().len())
        .ok_or_else(|| io::Error::other("Açık proje tanıtıcısı yok."))?;
    let goreli = p.strip_prefix(kok).map_err(io::Error::other)?;
    if goreli
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(io::Error::other("Geçersiz proje yolu."));
    }
    Ok((
        d.try_clone()?,
        if goreli.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            goreli.to_path_buf()
        },
    ))
}

/// Linux'ta hedefi ezmeme koşulu işletim sistemi tarafından tek işlemde uygulanır.
pub fn tasi_ezmeden(
    k: &cap_std::fs::Dir,
    eski: &Path,
    h: &cap_std::fs::Dir,
    yeni: &Path,
) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        rustix::fs::renameat_with(k, eski, h, yeni, rustix::fs::RenameFlags::NOREPLACE)
            .map_err(io::Error::from)
    }
    #[cfg(not(target_os = "linux"))]
    {
        if h.symlink_metadata(yeni).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Hedef zaten var.",
            ));
        }
        if k.symlink_metadata(eski)?.is_file() {
            k.hard_link(eski, h, yeni)?;
            k.remove_file(eski)
        } else {
            k.rename(eski, h, yeni)
        }
    }
}
pub fn tasi_yollari(eski: &Path, yeni: &Path) -> io::Result<()> {
    fn ust(p: &Path) -> io::Result<(cap_std::fs::Dir, PathBuf)> {
        if let Ok(d) = kapsam(p) {
            return Ok(d);
        }
        Ok((
            cap_std::fs::Dir::open_ambient_dir(
                p.parent().ok_or_else(|| io::Error::other("Üst yol yok."))?,
                cap_std::ambient_authority(),
            )?,
            PathBuf::from(p.file_name().ok_or_else(|| io::Error::other("Ad yok."))?),
        ))
    }
    let (k, eski) = ust(eski)?;
    let (h, yeni) = ust(yeni)?;
    tasi_ezmeden(&k, &eski, &h, &yeni)
}

pub fn sinirli_metin(p: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(sinirli_oku(p.as_ref())?)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn proje_taniticisi_yol_degisse_de_disari_cikmaz() {
        let k = crate::derleme::gecici_klasor("tanimli-kok").unwrap();
        let kok = k.join("proje");
        std::fs::create_dir(&kok).unwrap();
        std::fs::write(kok.join("a"), b"proje").unwrap();
        let dis = k.join("dis");
        std::fs::create_dir(&dis).unwrap();
        std::fs::write(dis.join("a"), b"gizli").unwrap();
        kok_ac(&kok).unwrap();
        std::fs::rename(&kok, k.join("eski")).unwrap();
        std::os::unix::fs::symlink(&dis, &kok).unwrap();
        assert_eq!(sinirli_oku(&kok.join("a")).unwrap(), b"proje");
        let (d, ad) = sabitle(&kok.join("a")).unwrap();
        sabit_yaz(&d, &ad, b"yeni").unwrap();
        assert_eq!(std::fs::read(dis.join("a")).unwrap(), b"gizli");
        std::fs::remove_dir_all(k).unwrap();
    }

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
