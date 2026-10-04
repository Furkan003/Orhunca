//! Derleme hattı: dosyaları yükleme, denetleme, makine kodu üretme ve bağlama.
//! Komut aracı ve Orhunca Stüdyo bu modülü ortak kullanır.

use crate::{agac, ayristirici, denetci, hata, sozcuk, uretici};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use target_lexicon::Triple;

const CALISMA_ZAMANI_KAYNAGI: &str = include_str!("../runtime/orhunca_rt.c");

/// Derleme sırasında oluşan hata: biçimlenmiş metin ve (varsa) konum bilgisi.
#[derive(Debug, Clone)]
pub struct DerlemeHatasi {
    /// Komut satırında gösterilecek, kaynak satırını içeren metin.
    pub metin: String,
    pub teshis: Option<Teshis>,
}

/// Düzenleyicide gösterilecek tanı.
#[derive(Debug, Clone)]
pub struct Teshis {
    pub dosya: String,
    pub satir: usize,
    pub sutun: usize,
    pub mesaj: String,
    pub ipucu: Option<String>,
}

impl DerlemeHatasi {
    fn duz(metin: impl Into<String>) -> Self {
        DerlemeHatasi {
            metin: metin.into(),
            teshis: None,
        }
    }

    fn konumlu(h: hata::Hata, dosyalar: &[(String, String)]) -> Self {
        let dosya = dosyalar
            .get(h.konum.dosya)
            .map(|d| d.0.clone())
            .unwrap_or_default();
        DerlemeHatasi {
            metin: h.goster(dosyalar),
            teshis: Some(Teshis {
                dosya,
                satir: h.konum.satir,
                sutun: h.konum.sutun,
                mesaj: h.mesaj,
                ipucu: h.ipucu,
            }),
        }
    }
}

impl From<String> for DerlemeHatasi {
    fn from(m: String) -> Self {
        DerlemeHatasi::duz(m)
    }
}

/// Ana dosyayı ve `kullan` ile eklenen tüm dosyaları okur, ayrıştırır ve denetler.
pub fn yukle(dosya: &Path) -> Result<agac::Program, DerlemeHatasi> {
    yukle_ortulu(dosya, &HashMap::new())
}

/// `ortulu`daki dosyalar diskten değil bellekten okunur (düzenleyicide kaydedilmemiş
/// değişiklikler). Anahtarlar tam (canonical) yollardır.
pub fn yukle_ortulu(
    dosya: &Path,
    ortulu: &HashMap<PathBuf, String>,
) -> Result<agac::Program, DerlemeHatasi> {
    let mut dosyalar: Vec<(String, String)> = Vec::new();
    let mut yollar: Vec<PathBuf> = Vec::new();
    let mut sozcukler = Vec::new();
    let mut kuyruk = vec![(dosya.to_path_buf(), None::<hata::Konum>)];
    while let Some((yol, nereden)) = kuyruk.pop() {
        let tam = std::fs::canonicalize(&yol).unwrap_or_else(|_| yol.clone());
        if yollar.contains(&tam) {
            continue;
        }
        let okunan = match ortulu.get(&tam) {
            Some(icerik) => Ok(icerik.clone()),
            None => std::fs::read_to_string(&yol),
        };
        let kaynak = match okunan {
            Ok(k) => k,
            Err(e) => {
                let neden = match e.kind() {
                    std::io::ErrorKind::NotFound => "dosya bulunamadı".to_string(),
                    std::io::ErrorKind::PermissionDenied => "okuma izni yok".to_string(),
                    _ => e.to_string(),
                };
                let mesaj = format!("'{}' okunamadı: {neden}", yol.display());
                return Err(match nereden {
                    Some(k) => DerlemeHatasi::konumlu(hata::Hata::yeni(k, mesaj), &dosyalar),
                    None => DerlemeHatasi::duz(mesaj),
                });
            }
        };
        let sira = dosyalar.len();
        dosyalar.push((yol.display().to_string(), kaynak));
        yollar.push(tam);
        let mut s = match sozcuk::sozcukle(&dosyalar[sira].1) {
            Ok(s) => s,
            Err(mut h) => {
                h.konum.dosya = sira;
                return Err(DerlemeHatasi::konumlu(h, &dosyalar));
            }
        };
        for t in s.iter_mut() {
            t.konum.dosya = sira;
        }
        let klasor = yol.parent().map(Path::to_path_buf).unwrap_or_default();
        for (ek, k) in ayristirici::kullanilanlar(&s).into_iter().rev() {
            kuyruk.push((klasor.join(ek), Some(k)));
        }
        sozcukler.push(s);
    }
    let mut program =
        ayristirici::ayristir_cok(sozcukler).map_err(|h| DerlemeHatasi::konumlu(h, &dosyalar))?;
    denetci::denetle(&mut program).map_err(|h| DerlemeHatasi::konumlu(h, &dosyalar))?;
    Ok(program)
}

pub fn hedef_uclusu(hedef: Option<&str>) -> Result<Triple, String> {
    match hedef {
        None => Ok(Triple::host()),
        Some("linux") => Ok(Triple::from_str("x86_64-unknown-linux-gnu").unwrap()),
        Some("windows") => Ok(Triple::from_str("x86_64-pc-windows-gnu").unwrap()),
        Some(t) => Triple::from_str(t).map_err(|e| format!("geçersiz hedef '{t}': {e}")),
    }
}

pub fn windows_mu(hedef: Option<&str>) -> bool {
    hedef_uclusu(hedef)
        .map(|t| t.operating_system == target_lexicon::OperatingSystem::Windows)
        .unwrap_or(false)
}

/// Aynı süreçte aynı anda birden fazla derleme yapılabilir (Stüdyo); her birine
/// ayrı bir geçici klasör verilir.
pub fn gecici_klasor(ad: &str) -> Result<PathBuf, String> {
    static SAYAC: AtomicU64 = AtomicU64::new(0);
    let n = SAYAC.fetch_add(1, Ordering::Relaxed);
    let k = std::env::temp_dir().join(format!("orhunca-{ad}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&k).map_err(|e| format!("geçici klasör oluşturulamadı: {e}"))?;
    Ok(k)
}

/// `dosya`yı `cikti` adlı çalıştırılabilir dosyaya derler.
pub fn derle(dosya: &Path, cikti: &Path, hedef: Option<&str>) -> Result<(), DerlemeHatasi> {
    let program = yukle(dosya)?;
    let triple = hedef_uclusu(hedef)?;
    let windows = triple.operating_system == target_lexicon::OperatingSystem::Windows;
    let isa = uretici::isa_kur(triple)?;
    let nesne = uretici::uret(&program, isa).map_err(|e| format!("kod üretimi başarısız: {e}"))?;

    let gecici = gecici_klasor("derleme")?;
    let nesne_yolu = gecici.join(if windows { "program.obj" } else { "program.o" });
    let cz_yolu = gecici.join("orhunca_rt.c");
    std::fs::write(&nesne_yolu, nesne).map_err(|e| e.to_string())?;
    std::fs::write(&cz_yolu, CALISMA_ZAMANI_KAYNAGI).map_err(|e| e.to_string())?;

    let baglayici = std::env::var("ORHUNCA_CC").unwrap_or_else(|_| {
        if windows && !cfg!(windows) {
            "x86_64-w64-mingw32-gcc".into()
        } else {
            "cc".into()
        }
    });
    let sonuc = Command::new(&baglayici)
        .arg("-O2")
        .arg("-o")
        .arg(cikti)
        .arg(&nesne_yolu)
        .arg(&cz_yolu)
        .arg("-lm")
        .output();
    let _ = std::fs::remove_dir_all(&gecici);
    let sonuc = sonuc.map_err(|e| {
        format!(
            "bağlayıcı '{baglayici}' çalıştırılamadı: {e}\nipucu: bir C derleyicisi kurun (Linux: gcc, Windows: MinGW-w64) ya da ORHUNCA_CC ile belirtin"
        )
    })?;
    if !sonuc.status.success() {
        return Err(DerlemeHatasi::duz(format!(
            "bağlama başarısız ({baglayici}):\n{}",
            String::from_utf8_lossy(&sonuc.stderr)
        )));
    }
    Ok(())
}

/// Çıktı adı verilmediğinde: `ana.ohc` → `ana` (Windows'ta `ana.exe`).
pub fn varsayilan_cikti(dosya: &Path, hedef: Option<&str>) -> PathBuf {
    let kok = dosya
        .file_stem()
        .map(|k| k.to_os_string())
        .unwrap_or_else(|| "program".into());
    let mut c = PathBuf::from(kok);
    if windows_mu(hedef) {
        c.set_extension("exe");
    }
    c
}

/// Bir proje klasöründeki `.ohcproj` dosyasını bulur.
pub fn proje_dosyasi(klasor: &Path) -> Option<PathBuf> {
    std::fs::read_dir(klasor)
        .ok()?
        .filter_map(|g| g.ok().map(|g| g.path()))
        .find(|p| p.extension().is_some_and(|u| u == "ohcproj"))
}

/// `.ohcproj` dosyasındaki `anahtar = "değer"` satırını okur.
pub fn proje_ayari(proje: &Path, anahtarlar: &[&str]) -> Option<String> {
    let icerik = std::fs::read_to_string(proje).ok()?;
    icerik.lines().find_map(|satir| {
        let (a, d) = satir.split_once('=')?;
        anahtarlar
            .contains(&a.trim())
            .then(|| d.trim().trim_matches('"').to_string())
    })
}

/// Proje klasörünün giriş dosyası (`giriş = "ana.ohc"`, yoksa `ana.ohc`).
pub fn proje_girisi(klasor: &Path) -> Result<PathBuf, String> {
    let proje = proje_dosyasi(klasor).ok_or_else(|| {
        format!(
            "derlenecek dosya verilmedi ve '{}' klasöründe .ohcproj dosyası yok",
            klasor.display()
        )
    })?;
    let giris = proje_ayari(&proje, &["giriş", "giris"]).unwrap_or_else(|| "ana.ohc".into());
    Ok(klasor.join(giris))
}
