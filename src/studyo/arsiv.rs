//! Doğrulanmış proje kopyaları: tamamlanmadan görünür olmaz; bağlantı izlenmez.
use super::dosya;
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

pub fn goreli(yol: &str) -> io::Result<&Path> {
    let p = Path::new(yol);
    if yol.is_empty()
        || yol.contains('\\')
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(io::Error::other("Geçersiz göreli yol."));
    }
    Ok(p)
}

pub fn baglantisiz(kok: &Path, yol: &Path) -> io::Result<()> {
    let goreli = yol.strip_prefix(kok).map_err(io::Error::other)?;
    let mut p = kok.to_path_buf();
    for c in goreli.components() {
        if !matches!(c, Component::Normal(_)) {
            return Err(io::Error::other("Proje dışı yol."));
        }
        p.push(c);
        match std::fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(io::Error::other("Bağlantılı yol işlenemez."))
            }
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub fn atlanir(ad: &str) -> bool {
    matches!(
        ad,
        ".git" | "target" | "node_modules" | "cikti" | "paketler"
    ) || ad.starts_with(".orhunca-")
        || ad.starts_with("kurtarilan-")
}

pub fn gizli(ad: &str) -> bool {
    let ad = ad.to_lowercase();
    ad == ".env"
        || ad.starts_with(".env.")
        || ad.ends_with(".pem")
        || ad.ends_with(".key")
        || ad.ends_with(".p12")
        || ad.ends_with(".keystore")
        || matches!(ad.as_str(), "veri" | "secrets" | ".aws" | ".ssh")
}

/// Tanıtıcı üzerinden kopyalar; yol değişse bile proje sınırı aşılamaz.
pub fn kopyala(kaynak: &Path, hedef: &Path, proje: bool, paylas: bool) -> io::Result<Vec<Value>> {
    use cap_std::fs::Dir;
    struct Kopya {
        dosyalar: Vec<Value>,
        boy: u64,
        proje: bool,
        paylas: bool,
    }
    impl Kopya {
        fn gez(
            &mut self,
            k: &Dir,
            ad: &Path,
            h: &Dir,
            hedef: &Path,
            goreli: &Path,
        ) -> io::Result<()> {
            let m = k.symlink_metadata(ad)?;
            if m.file_type().is_symlink() {
                return Err(io::Error::other("Bağlantı kopyalanamaz."));
            }
            if m.is_dir() {
                h.create_dir(hedef)?;
                let kaynak = k.open_dir(ad)?;
                let hedef = h.open_dir(hedef)?;
                let mut girdiler = kaynak.entries()?.collect::<Result<Vec<_>, _>>()?;
                girdiler.sort_by_key(|g| g.file_name());
                for g in girdiler {
                    let ad = g.file_name();
                    let metin = ad.to_string_lossy();
                    if (self.proje && atlanir(&metin)) || (self.paylas && gizli(&metin)) {
                        continue;
                    }
                    self.gez(
                        &kaynak,
                        Path::new(&ad),
                        &hedef,
                        Path::new(&ad),
                        &goreli.join(&ad),
                    )?;
                }
                #[cfg(unix)]
                hedef.open(".")?.sync_all()?;
            } else if m.is_file() {
                self.boy += m.len();
                if self.boy > 512 * 1024 * 1024 || self.dosyalar.len() >= 10000 {
                    return Err(io::Error::other("Kopya sınırı: 512 MiB / 10000 dosya."));
                }
                let mut f = k.open(ad)?;
                let mut veri = Vec::new();
                (&mut f)
                    .take(512 * 1024 * 1024 + 1)
                    .read_to_end(&mut veri)?;
                if veri.len() as u64 != m.len() {
                    return Err(io::Error::other("Kopyalama sırasında dosya değişti."));
                }
                let mut ayar = cap_std::fs::OpenOptions::new();
                ayar.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use cap_std::fs::OpenOptionsExt;
                    ayar.mode(0o600);
                }
                let mut f = h.open_with(hedef, &ayar)?;
                f.write_all(&veri)?;
                f.set_permissions(m.permissions())?;
                f.sync_all()?;
                let ozet = dosya::surum(&veri);
                if dosya::surum(&k.read(ad)?) != ozet {
                    return Err(io::Error::other("Kopyalama sırasında dosya değişti."));
                }
                self.dosyalar.push(json!({"yol":goreli.to_string_lossy().replace('\\',"/"),"boyut":veri.len(),"ozet":ozet}));
            } else {
                return Err(io::Error::other("Özel dosya kopyalanamaz."));
            }
            Ok(())
        }
    }
    fn ust(p: &Path) -> io::Result<(Dir, PathBuf)> {
        if let Ok(d) = dosya::kapsam(p) {
            return Ok(d);
        }
        let ad = p
            .file_name()
            .ok_or_else(|| io::Error::other("Dosya adı yok."))?;
        Ok((
            Dir::open_ambient_dir(
                p.parent().unwrap_or(Path::new(".")),
                cap_std::ambient_authority(),
            )?,
            PathBuf::from(ad),
        ))
    }
    let (k, ad) = ust(kaynak)?;
    let (h, hedef_ad) = ust(hedef)?;
    if h.symlink_metadata(&hedef_ad).is_ok() {
        return Err(io::Error::other("Hedef zaten var."));
    }
    let mut kopya = Kopya {
        dosyalar: Vec::new(),
        boy: 0,
        proje,
        paylas,
    };
    if let Err(e) = kopya.gez(&k, &ad, &h, &hedef_ad, Path::new("")) {
        let _ = h.remove_dir_all(&hedef_ad);
        let _ = h.remove_file(&hedef_ad);
        return Err(e);
    }
    Ok(kopya.dosyalar)
}

pub struct Gecici(pub PathBuf);
impl Drop for Gecici {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
