//! Derleme hattı: dosyaları yükleme, denetleme, makine kodu üretme ve bağlama.
//! Komut aracı ve Orhunca Stüdyo bu modülü ortak kullanır.

use crate::{agac, ayristirici, denetci, hata, sablon, sozcuk, uretici, wasm_uretici};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use target_lexicon::Triple;

const CALISMA_ZAMANI_KAYNAGI: &str = include_str!("../runtime/orhunca_rt.c");

/// WebAssembly: C çalışma zamanının wasm32 derlemesi (araclar/wasm_calisma_zamani.sh),
/// iki modülü birleştiren yükleyici ve tarayıcı sayfası.
pub const WASM_CALISMA_ZAMANI: &[u8] = include_bytes!("../runtime/wasm/orhunca_rt.wasm");
pub const WASM_YUKLEYICI: &str = include_str!("../runtime/wasm/orhunca.js");
const WASM_SAYFASI: &str = include_str!("../runtime/wasm/sayfa.html");

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
                    Some(k) => {
                        let mut h = hata::Hata::yeni(k, mesaj);
                        if yol.extension().is_none() {
                            h = h.ipucu(
                                "bir paket kullanıyorsanız önce kurun: orhunca paket ekle <git-adresi>",
                            );
                        }
                        DerlemeHatasi::konumlu(h, &dosyalar)
                    }
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
            // Önce bu dosyaya göre yol, yoksa kurulu paket: kullan "matematik"
            let dogrudan = klasor.join(&ek);
            let hedef = if dogrudan.exists() {
                dogrudan
            } else {
                crate::paket::paket_yolu(&klasor, &ek).unwrap_or(dogrudan)
            };
            kuyruk.push((hedef, Some(k)));
        }
        sozcukler.push(s);
    }
    // Görünümler: projenin görünümler/ klasöründeki .ohchtml dosyaları.
    let mut sablonlar = Vec::new();
    if let Some(klasor) = gorunum_klasoru(dosya) {
        for yol in gorunumleri_bul(&klasor) {
            let tam = std::fs::canonicalize(&yol).unwrap_or_else(|_| yol.clone());
            let kaynak = match ortulu.get(&tam) {
                Some(icerik) => icerik.clone(),
                None => std::fs::read_to_string(&yol).map_err(|e| {
                    DerlemeHatasi::duz(format!("'{}' okunamadı: {e}", yol.display()))
                })?,
            };
            let ad = yol
                .strip_prefix(&klasor)
                .unwrap_or(&yol)
                .with_extension("")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let sira = dosyalar.len();
            dosyalar.push((yol.display().to_string(), kaynak));
            let k = sablon::SablonKaynagi {
                ad,
                dosya: sira,
                kaynak: dosyalar[sira].1.clone(),
            };
            sablonlar.push(sablon::coz(&k).map_err(|h| DerlemeHatasi::konumlu(h, &dosyalar))?);
        }
    }
    let mut program = ayristirici::ayristir_cok(sozcukler, sablonlar)
        .map_err(|h| DerlemeHatasi::konumlu(h, &dosyalar))?;
    denetci::denetle(&mut program).map_err(|h| DerlemeHatasi::konumlu(h, &dosyalar))?;
    Ok(program)
}

/// Projenin kök klasörü: giriş dosyasından yukarı doğru `.ohcproj` içeren ilk
/// klasör; yoksa giriş dosyasının klasörü.
pub fn proje_koku(dosya: &Path) -> PathBuf {
    let klasor = dosya
        .parent()
        .map(Path::to_path_buf)
        .filter(|k| !k.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut k = Some(klasor.as_path());
    for _ in 0..4 {
        let Some(aday) = k else { break };
        if proje_dosyasi(aday).is_some() {
            return aday.to_path_buf();
        }
        k = aday.parent();
    }
    klasor
}

/// Görünümlerin klasörü (`<proje>/görünümler`), varsa.
pub fn gorunum_klasoru(dosya: &Path) -> Option<PathBuf> {
    let k = proje_koku(dosya).join("görünümler");
    k.is_dir().then_some(k)
}

/// Klasördeki (alt klasörler dahil) tüm `.ohchtml` dosyaları, sıralı.
pub fn gorunumleri_bul(klasor: &Path) -> Vec<PathBuf> {
    let mut sonuc = Vec::new();
    let mut bekleyen = vec![klasor.to_path_buf()];
    while let Some(k) = bekleyen.pop() {
        let Ok(girdiler) = std::fs::read_dir(&k) else {
            continue;
        };
        for g in girdiler.flatten() {
            let yol = g.path();
            if yol.is_dir() {
                bekleyen.push(yol);
            } else if yol.extension().is_some_and(|u| u == "ohchtml") {
                sonuc.push(yol);
            }
        }
    }
    sonuc.sort();
    sonuc
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
    if web_hedefi_mi(hedef) {
        return derle_web(dosya, cikti);
    }
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
    let mut komut = Command::new(&baglayici);
    komut
        .arg("-O2")
        .arg("-o")
        .arg(cikti)
        .arg(&nesne_yolu)
        .arg(&cz_yolu)
        .arg("-lm");
    if windows {
        // Web sunucusu için Windows soket kütüphanesi
        komut.arg("-lws2_32");
    }
    let sonuc = komut.output();
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

/// Çıktı adı verilmediğinde: `ana.ohc` → `ana` (Windows'ta `ana.exe`, web için `ana.html`).
pub fn varsayilan_cikti(dosya: &Path, hedef: Option<&str>) -> PathBuf {
    let kok = dosya
        .file_stem()
        .map(|k| k.to_os_string())
        .unwrap_or_else(|| "program".into());
    let mut c = PathBuf::from(kok);
    if web_hedefi_mi(hedef) {
        c.set_extension("html");
    } else if windows_mu(hedef) {
        c.set_extension("exe");
    }
    c
}

/// `--hedef web` (ya da `wasm`): WebAssembly'ye derlenir.
pub fn web_hedefi_mi(hedef: Option<&str>) -> bool {
    matches!(hedef, Some("web" | "wasm"))
}

/// Programı WebAssembly program modülüne derler.
pub fn wasm_uret(dosya: &Path) -> Result<Vec<u8>, DerlemeHatasi> {
    let program = yukle(dosya)?;
    Ok(wasm_uretici::uret(&program)
        .map_err(|e| format!("WebAssembly kod üretimi başarısız: {e}"))?)
}

/// WebAssembly'ye derler. Çıktı `.wasm` ise program modülünün yanına
/// `orhunca_rt.wasm` ve `orhunca.js` yazılır; değilse her şeyi içinde taşıyan,
/// tarayıcıda doğrudan açılan tek bir HTML sayfası.
pub fn derle_web(dosya: &Path, cikti: &Path) -> Result<(), DerlemeHatasi> {
    let wasm = wasm_uret(dosya)?;
    let yaz = |yol: &Path, veri: &[u8]| {
        std::fs::write(yol, veri).map_err(|e| format!("'{}' yazılamadı: {e}", yol.display()))
    };
    if cikti.extension().is_some_and(|u| u == "wasm") {
        let klasor = cikti
            .parent()
            .filter(|k| !k.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        yaz(cikti, &wasm)?;
        yaz(&klasor.join("orhunca_rt.wasm"), WASM_CALISMA_ZAMANI)?;
        yaz(&klasor.join("orhunca.js"), WASM_YUKLEYICI.as_bytes())?;
        return Ok(());
    }
    let baslik = dosya
        .file_stem()
        .map(|k| k.to_string_lossy().into_owned())
        .unwrap_or_else(|| "program".into());
    yaz(cikti, web_sayfasi(&baslik, &wasm).as_bytes())?;
    Ok(())
}

/// Programı ve çalışma zamanını (base64) içeren tarayıcı sayfası.
pub fn web_sayfasi(baslik: &str, wasm: &[u8]) -> String {
    let baslik = baslik
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    WASM_SAYFASI
        .replace("__BASLIK__", &baslik)
        .replace("__YUKLEYICI__", WASM_YUKLEYICI)
        .replace("__CALISMA_ZAMANI__", &base64(WASM_CALISMA_ZAMANI))
        .replace("__PROGRAM__", &base64(wasm))
}

fn base64(veri: &[u8]) -> String {
    const ABECE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(veri.len().div_ceil(3) * 4);
    for parca in veri.chunks(3) {
        let b = [
            parca[0],
            *parca.get(1).unwrap_or(&0),
            *parca.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= parca.len() {
                s.push(ABECE[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// Bir proje klasöründeki `.ohcproj` dosyasını bulur.
pub fn proje_dosyasi(klasor: &Path) -> Option<PathBuf> {
    std::fs::read_dir(klasor)
        .ok()?
        .filter_map(|g| g.ok().map(|g| g.path()))
        .find(|p| p.extension().is_some_and(|u| u == "ohcproj"))
}

/// `.ohcproj` dosyasının en üst bölümündeki `anahtar = "değer"` satırını okur.
pub fn proje_ayari(proje: &Path, anahtarlar: &[&str]) -> Option<String> {
    let icerik = std::fs::read_to_string(proje).ok()?;
    crate::paket::AyarDosyasi::coz(&icerik).deger(None, anahtarlar)
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
