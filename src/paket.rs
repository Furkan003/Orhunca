//! Paket yöneticisi: `orhunca paket ekle | yükle | kaldır | güncelle | listele`
//!
//! Paketler Git depolarıdır; her biri bir `.ohcproj` dosyası ve giriş dosyası olan
//! bir Orhunca kütüphanesidir. Bağımlılıklar projenin `.ohcproj` dosyasındaki
//! `[bağımlılıklar]` bölümüne yazılır, kesin sürümler `orhunca.kilit` dosyasında
//! tutulur ve paketler projenin `paketler/` klasörüne kurulur:
//!
//! ```text
//! [bağımlılıklar]
//! matematik = "https://github.com/kisi/orhunca-matematik.git#v1.0"
//! ```
//!
//! Kodda `kullan "matematik"` paketin giriş dosyasını, `kullan "matematik/geometri.ohc"`
//! paketteki belirli bir dosyayı alır.
//!
//! Kaynak bir deponun alt klasörü de olabilir: `adres#etiket:klasör/yolu` (etiket boş
//! bırakılırsa varsayılan dal). Paket dizini (`kütüphaneler/dizin.json`) paketleri adla
//! bulur: `orhunca paket ara`, `orhunca paket ekle istatistik`.

use crate::paket_denetim::{self, Izin};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

const BOLUM: &str = "bağımlılıklar";
pub const KILIT_DOSYASI: &str = "orhunca.kilit";
pub const PAKET_KLASORU: &str = "paketler";

/// `.ohcproj` ve kilit dosyalarının basit biçimi: `anahtar = "değer"` satırları ve
/// `[bölüm]` başlıkları. Sıra ve yorumlar korunur.
#[derive(Debug, Clone, Default)]
pub struct AyarDosyasi {
    satirlar: Vec<Satir>,
}

#[derive(Debug, Clone)]
enum Satir {
    Bolum(String),
    Deger(String, String),
    Diger(String),
}

impl AyarDosyasi {
    pub fn coz(metin: &str) -> Self {
        let satirlar = metin
            .lines()
            .map(|s| {
                let t = s.trim();
                if let Some(b) = t.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
                    Satir::Bolum(b.trim().to_string())
                } else if let (false, Some((a, d))) = (t.starts_with('#'), t.split_once('=')) {
                    Satir::Deger(a.trim().to_string(), d.trim().trim_matches('"').to_string())
                } else {
                    Satir::Diger(s.to_string())
                }
            })
            .collect();
        AyarDosyasi { satirlar }
    }

    pub fn metin(&self) -> String {
        let mut s = String::new();
        for satir in &self.satirlar {
            match satir {
                Satir::Bolum(b) => s.push_str(&format!("[{b}]\n")),
                Satir::Deger(a, d) => {
                    s.push_str(&format!("{a} = \"{}\"\n", d.replace('"', "\\\"")))
                }
                Satir::Diger(m) => s.push_str(&format!("{m}\n")),
            }
        }
        s
    }

    /// Bir bölümün (None: en üst) değerleri
    pub fn bolum(&self, ad: Option<&str>) -> Vec<(String, String)> {
        let mut gecerli: Option<&str> = None;
        let mut cikti = Vec::new();
        for satir in &self.satirlar {
            match satir {
                Satir::Bolum(b) => gecerli = Some(b),
                Satir::Deger(a, d) if gecerli == ad => cikti.push((a.clone(), d.clone())),
                _ => {}
            }
        }
        cikti
    }

    pub fn deger(&self, ad: Option<&str>, anahtarlar: &[&str]) -> Option<String> {
        self.bolum(ad)
            .into_iter()
            .find(|(a, _)| anahtarlar.contains(&a.as_str()))
            .map(|(_, d)| d)
    }

    /// Bölümdeki anahtarı ekler ya da değiştirir; bölüm yoksa dosyanın sonuna eklenir.
    pub fn yaz(&mut self, bolum: &str, anahtar: &str, deger: &str) {
        let mut gecerli: Option<String> = None;
        let mut son_sira = None;
        for (i, satir) in self.satirlar.iter_mut().enumerate() {
            match satir {
                Satir::Bolum(b) => gecerli = Some(b.clone()),
                Satir::Deger(a, d) if gecerli.as_deref() == Some(bolum) => {
                    if a == anahtar {
                        *d = deger.to_string();
                        return;
                    }
                    son_sira = Some(i);
                }
                _ if gecerli.as_deref() == Some(bolum) && son_sira.is_none() => son_sira = Some(i),
                _ => {}
            }
        }
        let yeni = Satir::Deger(anahtar.to_string(), deger.to_string());
        match son_sira {
            Some(i) => self.satirlar.insert(i + 1, yeni),
            None => {
                if self
                    .satirlar
                    .last()
                    .is_some_and(|s| !matches!(s, Satir::Diger(m) if m.trim().is_empty()))
                {
                    self.satirlar.push(Satir::Diger(String::new()));
                }
                self.satirlar.push(Satir::Bolum(bolum.to_string()));
                self.satirlar.push(yeni);
            }
        }
    }

    pub fn sil(&mut self, bolum: &str, anahtar: &str) -> bool {
        let mut gecerli: Option<String> = None;
        let mut sira = None;
        for (i, satir) in self.satirlar.iter().enumerate() {
            match satir {
                Satir::Bolum(b) => gecerli = Some(b.clone()),
                Satir::Deger(a, _) if gecerli.as_deref() == Some(bolum) && a == anahtar => {
                    sira = Some(i)
                }
                _ => {}
            }
        }
        match sira {
            Some(i) => {
                self.satirlar.remove(i);
                true
            }
            None => false,
        }
    }
}

/// Paket kaynağı: `adres#etiket` ya da `adres#etiket:alt/klasör`. `github:kişi/depo`
/// kısaltması kabul edilir. Git dal ve etiket adlarında `:` bulunamaz.
fn kaynagi_ayir(kaynak: &str) -> (String, Option<String>) {
    let (adres, etiket, _) = kaynak_parcalari(kaynak);
    (adres, etiket)
}

fn kaynak_parcalari(kaynak: &str) -> (String, Option<String>, Option<String>) {
    let (adres, ek) = match kaynak.rsplit_once('#') {
        Some((a, e)) if !e.is_empty() => (a, Some(e)),
        _ => (kaynak, None),
    };
    let (etiket, alt) = match ek.map(|e| e.split_once(':').unwrap_or((e, ""))) {
        Some((e, a)) => (
            Some(e.to_string()).filter(|e| !e.is_empty()),
            Some(a.trim_matches('/').to_string()).filter(|a| !a.is_empty()),
        ),
        None => (None, None),
    };
    let adres = match adres.strip_prefix("github:") {
        Some(yol) => format!("https://github.com/{}.git", yol.trim_end_matches(".git")),
        None => adres.to_string(),
    };
    (adres, etiket, alt)
}

/// Etiket git'e seçenek olarak geçmesin; alt klasör deponun dışına çıkmasın.
fn kaynagi_denetle(etiket: Option<&str>, alt: Option<&str>) -> Result<(), String> {
    if let Some(e) = etiket {
        if e.starts_with('-') || e.chars().any(|c| c.is_control() || c == ' ') {
            return Err(format!("geçersiz etiket '{e}'"));
        }
    }
    if let Some(a) = alt {
        let yol = Path::new(a);
        if yol.is_absolute()
            || a.contains('\\')
            || yol
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(format!("geçersiz alt klasör '{a}'"));
        }
    }
    Ok(())
}

const ISLEME_DOSYASI: &str = ".orhunca-isleme";

fn klasoru_kopyala(kaynak: &Path, hedef: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(hedef)?;
    for g in std::fs::read_dir(kaynak)? {
        let g = g?;
        let tur = g.file_type()?;
        if tur.is_symlink() || g.file_name() == ".git" {
            continue;
        }
        let h = hedef.join(g.file_name());
        if tur.is_dir() {
            klasoru_kopyala(&g.path(), &h)?;
        } else {
            std::fs::copy(g.path(), h)?;
        }
    }
    Ok(())
}

fn git(args: &[&str], klasor: Option<&Path>) -> Result<String, String> {
    let mut k = crate::komut("git");
    k.args(args);
    if let Some(d) = klasor {
        k.current_dir(d);
    }
    let cikti = k
        .output()
        .map_err(|_| "Git bulunamadı; paketler için Git kurulu olmalı.".to_string())?;
    if !cikti.status.success() {
        return Err(String::from_utf8_lossy(&cikti.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&cikti.stdout).trim().to_string())
}

/// Paketi `hedef` klasörüne getirir ve kurulan işlemenin (commit) kimliğini döndürür.
fn getir(
    kaynak: &str,
    isleme: Option<&str>,
    hedef: &Path,
    gunluk: &mut Vec<String>,
) -> Result<String, String> {
    let (adres, etiket, alt) = kaynak_parcalari(kaynak);
    kaynagi_denetle(etiket.as_deref(), alt.as_deref())?;
    if adres.starts_with('-') {
        return Err(format!("geçersiz paket adresi '{adres}'"));
    }
    // Zaten istenen işlemede kuruluysa dokunulmaz.
    if let Some(i) = isleme {
        let kurulu = if alt.is_some() {
            std::fs::read_to_string(hedef.join(ISLEME_DOSYASI))
                .ok()
                .map(|m| m.trim().to_string())
        } else if hedef.join(".git").exists() {
            git(&["rev-parse", "HEAD"], Some(hedef)).ok()
        } else {
            None
        };
        if kurulu.as_deref() == Some(i) {
            return Ok(i.to_string());
        }
    }
    if let Some(alt) = &alt {
        return alt_klasoru_getir(
            &adres,
            isleme.map(str::to_string).or(etiket),
            alt,
            hedef,
            gunluk,
        );
    }
    if hedef.exists() {
        std::fs::remove_dir_all(hedef)
            .map_err(|e| format!("'{}' silinemedi: {e}", hedef.display()))?;
    }
    if let Some(u) = hedef.parent() {
        std::fs::create_dir_all(u).map_err(|e| e.to_string())?;
    }
    gunluk.push(format!("indiriliyor: {adres}"));
    let hedef_m = hedef.to_string_lossy().into_owned();
    git(&["clone", "--quiet", "--", &adres, &hedef_m], None)
        .map_err(|e| format!("'{adres}' indirilemedi: {e}"))?;
    let istenen = isleme.map(str::to_string).or(etiket);
    if let Some(r) = &istenen {
        git(&["checkout", "--quiet", r], Some(hedef))
            .map_err(|e| format!("'{adres}' içinde '{r}' bulunamadı: {e}"))?;
    }
    git(&["rev-parse", "HEAD"], Some(hedef))
}

/// Deponun bir alt klasöründeki paketi getirir: depo geçici olarak (yalnızca gereken
/// dosyalar indirilerek) alınır, klasör `hedef`e kopyalanır.
fn alt_klasoru_getir(
    adres: &str,
    istenen: Option<String>,
    alt: &str,
    hedef: &Path,
    gunluk: &mut Vec<String>,
) -> Result<String, String> {
    let gecici = crate::derleme::gecici_klasor("paket-depo")?;
    let depo = gecici.join("d");
    let depo_m = depo.to_string_lossy().into_owned();
    gunluk.push(format!("indiriliyor: {adres} ({alt})"));
    let sonuc = (|| {
        if git(
            &[
                "clone",
                "--quiet",
                "--filter=blob:none",
                "--no-checkout",
                "--",
                adres,
                &depo_m,
            ],
            None,
        )
        .is_err()
        {
            let _ = std::fs::remove_dir_all(&depo);
            git(
                &["clone", "--quiet", "--no-checkout", "--", adres, &depo_m],
                None,
            )
            .map_err(|e| format!("'{adres}' indirilemedi: {e}"))?;
        }
        let _ = git(
            &["sparse-checkout", "set", "--no-cone", &format!("/{alt}/")],
            Some(&depo),
        );
        let r = istenen.as_deref().unwrap_or("HEAD");
        git(&["checkout", "--quiet", r], Some(&depo))
            .map_err(|e| format!("'{adres}' içinde '{r}' bulunamadı: {e}"))?;
        let isleme = git(&["rev-parse", "HEAD"], Some(&depo))?;
        let kaynak = depo.join(alt);
        if !kaynak.is_dir() {
            return Err(format!("'{adres}' deposunda '{alt}' klasörü yok"));
        }
        if hedef.exists() {
            std::fs::remove_dir_all(hedef)
                .map_err(|e| format!("'{}' silinemedi: {e}", hedef.display()))?;
        }
        klasoru_kopyala(&kaynak, hedef).map_err(|e| format!("paket kopyalanamadı: {e}"))?;
        std::fs::write(hedef.join(ISLEME_DOSYASI), &isleme).map_err(|e| e.to_string())?;
        Ok(isleme)
    })();
    let _ = std::fs::remove_dir_all(&gecici);
    sonuc
}

/// Paket dizinindeki bir paket
#[derive(Debug, Clone, Default)]
pub struct DizinPaketi {
    pub ad: String,
    pub aciklama: String,
    /// Son sürümün kaynağı (etiketiyle): `github:kişi/depo#v1.0`
    pub kaynak: String,
    pub surum: String,
    /// Paketi yayımlayan GitHub kullanıcısı; paketi yalnızca o güncelleyebilir.
    pub sahip: String,
    /// Son sürümün işlemesi (commit); etiket sonradan taşınırsa kurulmaz.
    pub isleme: Option<String>,
    /// İçerik özeti (`sha256:...`), bkz. paket_denetim::icerik_ozeti
    pub ozet: Option<String>,
    /// Paket dizininin denetiminde koddan çıkarılan izinler
    pub izinler: Vec<String>,
}

/// Paket mağazası (GitHub'daki `orhunca-paketler` deposu); ulaşılamazsa eski dizin.
/// `ORHUNCA_PAKET_DIZINI` ile başka bir adres ya da dosya verilebilir.
pub const DIZIN_ADRESI: &str =
    "https://raw.githubusercontent.com/Furkan003/orhunca-paketler/HEAD/dizin.json";
const ESKI_DIZIN_ADRESI: &str =
    "https://raw.githubusercontent.com/Furkan003/Orhunca/HEAD/k%C3%BCt%C3%BCphaneler/dizin.json";

pub fn dizin() -> Result<Vec<DizinPaketi>, String> {
    match std::env::var("ORHUNCA_PAKET_DIZINI") {
        Ok(yer) => dizin_oku(&yer),
        Err(_) => dizin_oku(DIZIN_ADRESI).or_else(|_| dizin_oku(ESKI_DIZIN_ADRESI)),
    }
}

fn dizin_oku(yer: &str) -> Result<Vec<DizinPaketi>, String> {
    let metin = if yer.starts_with("https://") || yer.starts_with("http://") {
        let c = crate::komut("curl")
            .args([
                "-sSfL",
                "--max-time",
                "20",
                "--proto",
                "=https,http",
                "--",
                yer,
            ])
            .output()
            .map_err(|_| "paket dizini indirilemedi: curl bulunamadı".to_string())?;
        if !c.status.success() {
            return Err(format!(
                "paket dizini indirilemedi ({yer}): {}",
                String::from_utf8_lossy(&c.stderr).trim()
            ));
        }
        String::from_utf8_lossy(&c.stdout).into_owned()
    } else {
        std::fs::read_to_string(yer).map_err(|e| format!("paket dizini okunamadı ({yer}): {e}"))?
    };
    let j: serde_json::Value =
        serde_json::from_str(&metin).map_err(|e| format!("paket dizini bozuk: {e}"))?;
    let alan = |p: &serde_json::Value, a: &str| p[a].as_str().unwrap_or("").to_string();
    Ok(j["paketler"]
        .as_array()
        .map(|l| {
            l.iter()
                .map(|p| DizinPaketi {
                    ad: alan(p, "ad"),
                    aciklama: alan(p, "açıklama"),
                    kaynak: alan(p, "kaynak"),
                    surum: alan(p, "sürüm"),
                    sahip: alan(p, "sahip"),
                    isleme: Some(alan(p, "işleme")).filter(|m| !m.is_empty()),
                    ozet: Some(alan(p, "özet")).filter(|m| !m.is_empty()),
                    izinler: p["izinler"]
                        .as_array()
                        .map(|l| {
                            l.iter()
                                .filter_map(|i| i.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .filter(|p| gecerli_paket_adi(&p.ad) && !p.kaynak.is_empty())
                .collect()
        })
        .unwrap_or_default())
}

/// Ada ya da açıklamaya göre arar (Türkçe harfler sadeleştirilerek).
pub fn ara(kelime: &str) -> Result<Vec<DizinPaketi>, String> {
    let sade = |m: &str| crate::oneriler::sadelestir(m);
    let k = sade(kelime);
    Ok(dizin()?
        .into_iter()
        .filter(|p| k.is_empty() || sade(&p.ad).contains(&k) || sade(&p.aciklama).contains(&k))
        .collect())
}

/// Adres değil de yalnızca bir ad verildiyse (ör. `istatistik`) paket dizininde bulunur.
fn dizinden_coz(kok: &Path, kaynak: &str) -> Result<Option<DizinPaketi>, String> {
    if !gecerli_paket_adi(kaynak) || kok.join(kaynak).exists() {
        return Ok(None);
    }
    let paketler = dizin()?;
    match paketler.iter().find(|p| p.ad == kaynak) {
        Some(p) => Ok(Some(p.clone())),
        None => {
            let adlar = paketler.iter().map(|p| p.ad.as_str());
            let mut h = format!("'{kaynak}' paket dizininde yok");
            if let Some(b) = crate::oneriler::benzer(kaynak, adlar) {
                h.push_str(&format!("; bunu mu demek istediniz: {b}?"));
            }
            h.push_str("\npaketleri görmek için: orhunca paket ara");
            Err(h)
        }
    }
}

pub struct Proje {
    pub kok: PathBuf,
    pub dosya: PathBuf,
    pub ayarlar: AyarDosyasi,
}

impl Proje {
    pub fn ac(kok: &Path) -> Result<Proje, String> {
        let dosya = crate::derleme::proje_dosyasi(kok).ok_or_else(|| {
            format!(
                "'{}' bir Orhunca projesi değil (.ohcproj dosyası yok)",
                kok.display()
            )
        })?;
        let metin = std::fs::read_to_string(&dosya).map_err(|e| e.to_string())?;
        Ok(Proje {
            kok: kok.to_path_buf(),
            dosya,
            ayarlar: AyarDosyasi::coz(&metin),
        })
    }

    pub fn bagimliliklar(&self) -> Vec<(String, String)> {
        self.ayarlar.bolum(Some(BOLUM))
    }

    fn kaydet(&self) -> Result<(), String> {
        std::fs::write(&self.dosya, self.ayarlar.metin()).map_err(|e| e.to_string())
    }
}

/// Kilit dosyasındaki bir paketin kaydı.
#[derive(Debug, Clone, PartialEq)]
struct Kilit {
    kaynak: String,
    isleme: String,
    /// Kurulumda onaylanan izinler; yoksa (eski kilit) bilinmiyor.
    izinler: Option<String>,
    ozet: Option<String>,
}

fn kilidi_oku(kok: &Path) -> HashMap<String, Kilit> {
    let metin = std::fs::read_to_string(kok.join(KILIT_DOSYASI)).unwrap_or_default();
    let a = AyarDosyasi::coz(&metin);
    let mut sonuc = HashMap::new();
    let bolumler: Vec<String> = a
        .satirlar
        .iter()
        .filter_map(|s| match s {
            Satir::Bolum(b) => Some(b.clone()),
            _ => None,
        })
        .collect();
    for b in bolumler {
        if let (Some(k), Some(i)) = (
            a.deger(Some(&b), &["kaynak"]),
            a.deger(Some(&b), &["işleme", "isleme"]),
        ) {
            let izinler = a.deger(Some(&b), &["izinler"]);
            let ozet = a.deger(Some(&b), &["özet", "ozet"]);
            sonuc.insert(
                b,
                Kilit {
                    kaynak: k,
                    isleme: i,
                    izinler,
                    ozet,
                },
            );
        }
    }
    sonuc
}

fn kilidi_yaz(kok: &Path, kurulan: &BTreeMap<String, Kilit>) -> Result<(), String> {
    let mut s =
        String::from("# Bu dosya `orhunca paket` tarafından üretilir; elle düzenlemeyin.\n");
    for (ad, k) in kurulan {
        s.push_str(&format!(
            "\n[{ad}]\nkaynak = \"{}\"\nişleme = \"{}\"\nizinler = \"{}\"\nözet = \"{}\"\n",
            k.kaynak,
            k.isleme,
            k.izinler.as_deref().unwrap_or(""),
            k.ozet.as_deref().unwrap_or("")
        ));
    }
    std::fs::write(kok.join(KILIT_DOSYASI), s).map_err(|e| e.to_string())
}

/// Projenin .gitignore dosyasına paketler/ klasörünü ekler.
fn gitignore_guncelle(kok: &Path) {
    let yol = kok.join(".gitignore");
    if let Ok(m) = std::fs::read_to_string(&yol) {
        if !m
            .lines()
            .any(|s| s.trim() == "/paketler/" || s.trim() == "paketler/")
        {
            let ek = if m.ends_with('\n') || m.is_empty() {
                ""
            } else {
                "\n"
            };
            let _ = std::fs::write(&yol, format!("{m}{ek}/paketler/\n"));
        }
    }
}

/// İzin onayı gereken paketlerde dönen hatanın ilk satırı; komut satırı ve Stüdyo bunu
/// görünce kullanıcıya sorar ve onaylanırsa işlemi `izin_ver` ile yineler.
pub const IZIN_GEREKLI: &str = "İZİN GEREKLİ";

fn izin_hatasi(onay: &[(String, BTreeSet<Izin>)]) -> String {
    let mut m = format!("{IZIN_GEREKLI}\n");
    for (ad, izinler) in onay {
        for i in izinler {
            m.push_str(&format!("  • {ad}: {} — {}\n", i.ad(), i.aciklama()));
        }
    }
    m.trim_end().to_string()
}

/// Kurulu paketin işlemesi (commit).
fn kurulu_isleme(hedef: &Path) -> Option<String> {
    if let Ok(m) = std::fs::read_to_string(hedef.join(ISLEME_DOSYASI)) {
        return Some(m.trim().to_string());
    }
    if hedef.join(".git").exists() {
        return git(&["rev-parse", "HEAD"], Some(hedef)).ok();
    }
    None
}

/// Tüm bağımlılıkları (dolaylı olanlar dahil) `paketler/` klasörüne kurar.
/// `guncelle`: kilitteki sürümler yok sayılır, en yeni sürümler alınır. `izin_ver`: yeni
/// izin isteyen paketler (onaylanmış izinlerin dışında) kurulur; verilmezse hiçbir şey
/// değiştirilmez ve [`IZIN_GEREKLI`] ile başlayan bir hata döner.
pub fn yukle(kok: &Path, guncelle: bool, izin_ver: bool) -> Result<Vec<String>, String> {
    yukle_ic(kok, guncelle, izin_ver, &HashMap::new())
}

/// `beklenen`: paket dizininden eklenen paketlerin kayıtları (işleme ve içerik özeti
/// karşılaştırılır).
fn yukle_ic(
    kok: &Path,
    guncelle: bool,
    izin_ver: bool,
    beklenen: &HashMap<String, DizinPaketi>,
) -> Result<Vec<String>, String> {
    let proje = Proje::ac(kok)?;
    let eski_kilit = kilidi_oku(kok);
    let kilit = if guncelle {
        HashMap::new()
    } else {
        eski_kilit.clone()
    };
    let paketler = kok.join(PAKET_KLASORU);
    let mut gunluk = Vec::new();
    let mut kurulan: BTreeMap<String, Kilit> = BTreeMap::new();
    // İndirilip henüz yerine konmamış paketler: izinler onaylanınca taşınır.
    let mut bekleyen: Vec<(String, PathBuf)> = Vec::new();
    let mut onay: Vec<(String, BTreeSet<Izin>)> = Vec::new();
    let mut kuyruk: Vec<(String, String, String)> = proje
        .bagimliliklar()
        .into_iter()
        .map(|(a, k)| (a, k, proje.ad()))
        .rev()
        .collect();
    let sonuc = (|| -> Result<(), String> {
        while let Some((ad, kaynak, isteyen)) = kuyruk.pop() {
            if let Some(k) = kurulan.get(&ad) {
                if kaynagi_ayir(&k.kaynak).0 != kaynagi_ayir(&kaynak).0 {
                    return Err(format!(
                        "'{ad}' paketi iki farklı kaynaktan isteniyor: '{}' ve '{kaynak}' ({isteyen})",
                        k.kaynak
                    ));
                }
                continue;
            }
            let hedef = paketler.join(&ad);
            let kilitli = kilit.get(&ad).filter(|k| k.kaynak == kaynak);
            let (klasor, isleme) = match kilitli {
                Some(k) if kurulu_isleme(&hedef).as_deref() == Some(k.isleme.as_str()) => {
                    (hedef.clone(), k.isleme.clone())
                }
                _ => {
                    let gecici = paketler.join(format!(".yeni-{ad}"));
                    let _ = std::fs::remove_dir_all(&gecici);
                    bekleyen.push((ad.clone(), gecici.clone()));
                    let i = getir(
                        &kaynak,
                        kilitli.map(|k| k.isleme.as_str()),
                        &gecici,
                        &mut gunluk,
                    )?;
                    (gecici, i)
                }
            };
            let izinler = paket_denetim::izinler(&klasor);
            let ozet = paket_denetim::icerik_ozeti(&klasor);
            if let Some(b) = beklenen.get(&ad) {
                if b.isleme.as_ref().is_some_and(|i| *i != isleme) {
                    return Err(format!(
                        "'{ad}' paketinin etiketi paket dizinine kaydedilen işlemeyi göstermiyor \
                         (yayımlandıktan sonra değiştirilmiş olabilir); paket kurulmadı"
                    ));
                }
                if b.ozet.as_ref().is_some_and(|o| *o != ozet) {
                    return Err(format!(
                        "'{ad}' paketinin içeriği paket dizinindeki özetle uyuşmuyor; paket kurulmadı"
                    ));
                }
            }
            if let Some(k) = kilitli.filter(|k| k.isleme == isleme) {
                if k.ozet.as_ref().is_some_and(|o| !o.is_empty() && *o != ozet) {
                    gunluk.push(format!(
                        "uyarı: paketler/{ad} kilit dosyasındaki içerikten farklı (elle değiştirilmiş olabilir)"
                    ));
                }
            }
            // Daha önce onaylanan izinlerin dışındakiler onay ister. Eski kilitlerde
            // (izin kaydı yok) kurulu izinler onaylanmış sayılır.
            let onaylanmis = match eski_kilit.get(&ad) {
                Some(k) => k
                    .izinler
                    .as_deref()
                    .map(paket_denetim::izinleri_coz)
                    .unwrap_or_else(|| izinler.clone()),
                None => BTreeSet::new(),
            };
            let yeni: BTreeSet<Izin> = izinler.difference(&onaylanmis).copied().collect();
            if !yeni.is_empty() && !izin_ver {
                onay.push((ad.clone(), yeni));
            }
            gunluk.push(format!("✓ {ad} ({})", &isleme[..isleme.len().min(10)]));
            if let Ok(p) = Proje::ac(&klasor) {
                for (a, k) in p.bagimliliklar().into_iter().rev() {
                    kuyruk.push((a, k, ad.clone()));
                }
            } else {
                gunluk.push(format!(
                    "uyarı: '{ad}' paketinde .ohcproj yok; giriş dosyası {ad}.ohc varsayılır"
                ));
            }
            kurulan.insert(
                ad,
                Kilit {
                    kaynak,
                    isleme,
                    izinler: Some(paket_denetim::izin_metni(&izinler)),
                    ozet: Some(ozet),
                },
            );
        }
        Ok(())
    })();
    if sonuc.is_err() || !onay.is_empty() {
        for (_, g) in &bekleyen {
            let _ = std::fs::remove_dir_all(g);
        }
        sonuc?;
        return Err(izin_hatasi(&onay));
    }
    for (ad, gecici) in bekleyen {
        let hedef = paketler.join(&ad);
        if hedef.exists() {
            std::fs::remove_dir_all(&hedef)
                .map_err(|e| format!("'{}' silinemedi: {e}", hedef.display()))?;
        }
        std::fs::rename(&gecici, &hedef)
            .map_err(|e| format!("'{}' yerine konamadı: {e}", hedef.display()))?;
    }
    // Artık istenmeyen paketler silinir.
    if let Ok(okunan) = std::fs::read_dir(&paketler) {
        for g in okunan.filter_map(|g| g.ok()) {
            let ad = g.file_name().to_string_lossy().into_owned();
            if !kurulan.contains_key(&ad) && g.path().is_dir() {
                let _ = std::fs::remove_dir_all(g.path());
                if !ad.starts_with('.') {
                    gunluk.push(format!("kaldırıldı: {ad}"));
                }
            }
        }
    }
    for (ad, k) in &kurulan {
        if let Some(iz) = k.izinler.as_deref().filter(|i| !i.is_empty()) {
            if eski_kilit.get(ad).and_then(|e| e.izinler.as_deref()) != Some(iz) {
                gunluk.push(format!("izinler: {ad} → {iz}"));
            }
        }
    }
    kilidi_yaz(kok, &kurulan)?;
    if !kurulan.is_empty() {
        gitignore_guncelle(kok);
    }
    gunluk.push(format!("{} paket hazır.", kurulan.len()));
    Ok(gunluk)
}

impl Proje {
    fn ad(&self) -> String {
        self.ayarlar.deger(None, &["ad"]).unwrap_or_else(|| {
            self.kok
                .file_name()
                .map(|a| a.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }
}

fn gecerli_paket_adi(ad: &str) -> bool {
    !ad.is_empty()
        && ad
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

/// Paketi bağımlılıklara ekler ve kurar. Ad verilmezse paketin kendi adı kullanılır.
/// Paket izin istiyorsa `izin_ver` verilmedikçe eklenmez (bkz. [`yukle`]).
pub fn ekle(
    kok: &Path,
    kaynak: &str,
    ad: Option<&str>,
    izin_ver: bool,
) -> Result<Vec<String>, String> {
    let mut proje = Proje::ac(kok)?;
    let dizinden = dizinden_coz(kok, kaynak)?;
    let (kaynak, ad) = match &dizinden {
        Some(p) => (p.kaynak.as_str(), Some(ad.unwrap_or(&p.ad))),
        None => (kaynak, ad),
    };
    let ad = match ad {
        Some(a) => a.to_string(),
        None => {
            // Paketin adını öğrenmek için geçici olarak indirilir.
            let gecici = crate::derleme::gecici_klasor("paket")?;
            let hedef = gecici.join("p");
            let mut g = Vec::new();
            let sonuc = getir(kaynak, None, &hedef, &mut g).and_then(|_| {
                Proje::ac(&hedef)
                    .map(|p| p.ad())
                    .or_else(|_| Ok(depo_adi(kaynak)))
            });
            let _ = std::fs::remove_dir_all(&gecici);
            sonuc?
        }
    };
    if !gecerli_paket_adi(&ad) {
        return Err(format!(
            "'{ad}' geçerli bir paket adı değil; --ad ile harf, rakam, _ ve - içeren bir ad verin"
        ));
    }
    let (_, etiket, alt) = kaynak_parcalari(kaynak);
    kaynagi_denetle(etiket.as_deref(), alt.as_deref())?;
    let onceki = proje.ayarlar.clone();
    proje.ayarlar.yaz(BOLUM, &ad, kaynak);
    proje.kaydet()?;
    let mut gunluk = vec![format!("eklendi: {ad} = \"{kaynak}\"")];
    let beklenen: HashMap<String, DizinPaketi> =
        dizinden.into_iter().map(|p| (ad.clone(), p)).collect();
    match yukle_ic(kok, false, izin_ver, &beklenen) {
        Ok(g) => gunluk.extend(g),
        Err(h) => {
            // Kurulamayan (ya da izni onaylanmayan) paket proje dosyasında bırakılmaz.
            proje.ayarlar = onceki;
            let _ = proje.kaydet();
            return Err(h);
        }
    }
    gunluk.push(format!("Kullanmak için: kullan \"{ad}\""));
    Ok(gunluk)
}

fn depo_adi(kaynak: &str) -> String {
    let (adres, _) = kaynagi_ayir(kaynak);
    adres
        .trim_end_matches('/')
        .rsplit(['/', ':', '\\'])
        .next()
        .unwrap_or("paket")
        .trim_end_matches(".git")
        .trim_start_matches("orhunca-")
        .to_string()
}

pub fn kaldir(kok: &Path, ad: &str) -> Result<Vec<String>, String> {
    let mut proje = Proje::ac(kok)?;
    if !proje.ayarlar.sil(BOLUM, ad) {
        return Err(format!("'{ad}' bu projenin bağımlılıkları arasında yok"));
    }
    proje.kaydet()?;
    let mut gunluk = vec![format!("bağımlılıklardan çıkarıldı: {ad}")];
    gunluk.extend(yukle(kok, false, false)?);
    Ok(gunluk)
}

pub struct PaketBilgisi {
    pub ad: String,
    pub kaynak: String,
    pub isleme: Option<String>,
    pub kurulu: bool,
    /// Kurulumda onaylanan izinler ("dosya, ağ")
    pub izinler: String,
}

pub fn listele(kok: &Path) -> Result<Vec<PaketBilgisi>, String> {
    let proje = Proje::ac(kok)?;
    let kilit = kilidi_oku(kok);
    Ok(proje
        .bagimliliklar()
        .into_iter()
        .map(|(ad, kaynak)| PaketBilgisi {
            kurulu: kok.join(PAKET_KLASORU).join(&ad).is_dir(),
            isleme: kilit.get(&ad).map(|k| k.isleme.clone()),
            izinler: kilit
                .get(&ad)
                .and_then(|k| k.izinler.clone())
                .unwrap_or_default(),
            ad,
            kaynak,
        })
        .collect())
}

/// Bir paketin (indirilerek) incelenmesi: adı, sürümü, işlemesi, içerik özeti ve izinleri.
#[derive(Debug, Clone)]
pub struct Inceleme {
    pub ad: String,
    pub surum: String,
    pub aciklama: String,
    pub kaynak: String,
    pub isleme: String,
    pub ozet: String,
    pub izinler: BTreeSet<Izin>,
}

impl Inceleme {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "ad": self.ad,
            "sürüm": self.surum,
            "açıklama": self.aciklama,
            "kaynak": self.kaynak,
            "işleme": self.isleme,
            "özet": self.ozet,
            "izinler": self.izinler.iter().map(|i| i.ad()).collect::<Vec<_>>(),
        })
    }
}

/// Paketi geçici bir klasöre indirip inceler (paket dizininin denetimi ve `yayımla` için).
pub fn incele(kaynak: &str) -> Result<Inceleme, String> {
    let kaynak = match dizinden_coz(Path::new("."), kaynak) {
        Ok(Some(p)) => p.kaynak,
        _ => kaynak.to_string(),
    };
    let gecici = crate::derleme::gecici_klasor("paket-incele")?;
    let hedef = gecici.join("p");
    let mut g = Vec::new();
    let sonuc = getir(&kaynak, None, &hedef, &mut g).map(|isleme| {
        let proje = Proje::ac(&hedef).ok();
        let ayar = |a: &[&str]| {
            proje
                .as_ref()
                .and_then(|p| p.ayarlar.deger(None, a))
                .unwrap_or_default()
        };
        Inceleme {
            ad: proje
                .as_ref()
                .map(|p| p.ad())
                .unwrap_or_else(|| depo_adi(&kaynak)),
            surum: ayar(&["sürüm", "surum"]),
            aciklama: ayar(&["açıklama", "aciklama"]),
            kaynak: kaynak.clone(),
            isleme,
            ozet: paket_denetim::icerik_ozeti(&hedef),
            izinler: paket_denetim::izinler(&hedef),
        }
    });
    let _ = std::fs::remove_dir_all(&gecici);
    sonuc
}

/// Paket mağazasının deposu: her paketin kaydı `paketler/<ad>.json` dosyasındadır.
pub const MAGAZA_DEPOSU: &str = "Furkan003/orhunca-paketler";

/// `git remote` adresinden GitHub'daki `kişi/depo`.
fn github_deposu(adres: &str) -> Option<(String, String)> {
    let yol = adres
        .strip_prefix("https://github.com/")
        .or_else(|| adres.strip_prefix("http://github.com/"))
        .or_else(|| adres.strip_prefix("git@github.com:"))
        .or_else(|| adres.strip_prefix("ssh://git@github.com/"))?;
    let yol = yol.trim_end_matches('/').trim_end_matches(".git");
    let (kisi, depo) = yol.split_once('/')?;
    (!kisi.is_empty() && !depo.is_empty() && !depo.contains('/'))
        .then(|| (kisi.to_string(), depo.to_string()))
}

fn url_kodla(m: &str) -> String {
    m.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `orhunca paket yayımla` sonucu
pub struct Yayim {
    /// paketler/<ad>.json dosyasının yeni içeriği
    pub kayit: String,
    /// Kaydın yazıldığı yerel dosya
    pub dosya: PathBuf,
    /// Çekme isteğini (PR) açacak GitHub sayfası
    pub adres: String,
    pub yeni: bool,
    pub inceleme: Inceleme,
}

/// Paketi yayımlamaya hazırlar: etiketin GitHub'da olduğunu denetler, paketi oradan indirip
/// inceler ve paket mağazasına eklenecek kaydı üretir.
pub fn yayimla(kok: &Path, etiket: Option<&str>) -> Result<Yayim, String> {
    let proje = Proje::ac(kok)?;
    let ad = proje
        .ayarlar
        .deger(None, &["ad"])
        .ok_or("paketin .ohcproj dosyasında ad = \"...\" satırı olmalı")?;
    if !gecerli_paket_adi(&ad) || ad.chars().count() < 2 || ad.chars().count() > 40 {
        return Err(format!(
            "'{ad}' paket adı olamaz: 2-40 karakter; harf, rakam, _ ve - kullanın"
        ));
    }
    let surum = proje
        .ayarlar
        .deger(None, &["sürüm", "surum"])
        .ok_or("paketin .ohcproj dosyasında sürüm = \"1.0.0\" satırı olmalı")?;
    if surum.split('.').count() != 3 || !surum.split('.').all(|p| p.parse::<u32>().is_ok()) {
        return Err(format!("sürüm '{surum}' üç sayıdan oluşmalı (ör. 1.0.0)"));
    }
    let aciklama = proje
        .ayarlar
        .deger(None, &["açıklama", "aciklama"])
        .filter(|a| !a.trim().is_empty())
        .ok_or(
            "paketin .ohcproj dosyasına bir açıklama yazın: açıklama = \"Paket ne işe yarar\"",
        )?;
    let uzak = git(&["remote", "get-url", "origin"], Some(kok))
        .map_err(|_| "paket bir Git deposunda olmalı ve GitHub'a gönderilmiş olmalı (git remote add origin ...)".to_string())?;
    let (kisi, depo) = github_deposu(&uzak)
        .ok_or_else(|| format!("paketin deposu GitHub'da olmalı (şu an: {uzak})"))?;
    let alt = git(&["rev-parse", "--show-prefix"], Some(kok))?;
    let alt = alt.trim_end_matches('/');
    let etiket = etiket
        .map(str::to_string)
        .unwrap_or_else(|| format!("v{surum}"));
    if git(
        &[
            "rev-parse",
            "-q",
            "--verify",
            &format!("refs/tags/{etiket}"),
        ],
        Some(kok),
    )
    .is_err()
    {
        return Err(format!(
            "'{etiket}' etiketi yok. Sürümü etiketleyip GitHub'a gönderin:\n  git tag {etiket}\n  git push origin {etiket}"
        ));
    }
    let uzak_etiket = git(
        &[
            "ls-remote",
            "--tags",
            "origin",
            &format!("refs/tags/{etiket}"),
        ],
        Some(kok),
    )
    .unwrap_or_default();
    if uzak_etiket.trim().is_empty() {
        return Err(format!(
            "'{etiket}' etiketi GitHub'da yok; gönderin:\n  git push origin {etiket}"
        ));
    }
    let mut kaynak = format!("github:{kisi}/{depo}#{etiket}");
    if !alt.is_empty() {
        kaynak.push(':');
        kaynak.push_str(alt);
    }
    let inceleme = incele(&kaynak)?;
    if inceleme.ad != ad {
        return Err(format!(
            "GitHub'daki '{etiket}' etiketinde paketin adı '{}' (yereldeki: '{ad}'); etiketi güncelleyin",
            inceleme.ad
        ));
    }

    // Ad koruması: aynı ad başkasına aitse ya da çok benzer bir ad varsa yayımlanmaz.
    let dizindeki = dizin().unwrap_or_default();
    let sade = crate::oneriler::sadelestir;
    if let Some(p) = dizindeki.iter().find(|p| p.ad == ad) {
        if !p.sahip.is_empty() && !p.sahip.eq_ignore_ascii_case(&kisi) {
            return Err(format!(
                "'{ad}' adı {} kullanıcısına ait; başka bir ad seçin",
                p.sahip
            ));
        }
    } else if let Some(p) = dizindeki.iter().find(|p| sade(&p.ad) == sade(&ad)) {
        return Err(format!(
            "'{ad}' adı mağazadaki '{}' paketine çok benziyor; başka bir ad seçin",
            p.ad
        ));
    }

    // Mağazadaki kayda yeni sürüm eklenir (önceki sürümler değiştirilemez).
    let kayit_adresi = format!(
        "https://raw.githubusercontent.com/{MAGAZA_DEPOSU}/HEAD/paketler/{}.json",
        url_kodla(&ad)
    );
    let onceki = crate::komut("curl")
        .args(["-sSfL", "--max-time", "20", "--", &kayit_adresi])
        .output()
        .ok()
        .filter(|c| c.status.success())
        .and_then(|c| serde_json::from_slice::<serde_json::Value>(&c.stdout).ok());
    let yeni = onceki.is_none();
    let mut surumler = onceki
        .as_ref()
        .and_then(|o| o["sürümler"].as_array().cloned())
        .unwrap_or_default();
    if surumler.iter().any(|s| s["sürüm"] == surum.as_str()) {
        return Err(format!(
            "{surum} sürümü zaten yayımlanmış; .ohcproj dosyasındaki sürümü artırın"
        ));
    }
    surumler.push(serde_json::json!({
        "sürüm": surum,
        "kaynak": kaynak,
        "işleme": inceleme.isleme,
        "özet": inceleme.ozet,
        "izinler": inceleme.izinler.iter().map(|i| i.ad()).collect::<Vec<_>>(),
    }));
    let sahip = onceki
        .as_ref()
        .and_then(|o| o["sahip"].as_str().map(str::to_string))
        .unwrap_or_else(|| kisi.clone());
    let kayit = serde_json::json!({
        "ad": ad,
        "açıklama": aciklama,
        "sahip": sahip,
        "sürümler": surumler,
    });
    let kayit = serde_json::to_string_pretty(&kayit).map_err(|e| e.to_string())? + "\n";
    let klasor = kok.join("cikti");
    std::fs::create_dir_all(&klasor).map_err(|e| e.to_string())?;
    let dosya = klasor.join(format!("{ad}.paket.json"));
    std::fs::write(&dosya, &kayit).map_err(|e| e.to_string())?;
    let adres = if yeni {
        format!(
            "https://github.com/{MAGAZA_DEPOSU}/new/main?filename=paketler/{}.json&value={}",
            url_kodla(&ad),
            url_kodla(&kayit)
        )
    } else {
        format!(
            "https://github.com/{MAGAZA_DEPOSU}/edit/main/paketler/{}.json",
            url_kodla(&ad)
        )
    };
    Ok(Yayim {
        kayit,
        dosya,
        adres,
        yeni,
        inceleme,
    })
}

/// `kullan "matematik"` → kurulu paketin giriş dosyası; `kullan "matematik/x.ohc"` → paketteki dosya.
/// Paketler, kullanan dosyanın klasöründen yukarı doğru `paketler/` klasörlerinde aranır.
pub fn paket_yolu(klasor: &Path, kullanilan: &str) -> Option<PathBuf> {
    let normal = kullanilan.replace('\\', "/");
    let (ad, kalan) = match normal.split_once('/') {
        Some((a, k)) => (a, Some(k)),
        None => (normal.as_str(), None),
    };
    if ad.is_empty() || ad == "." || ad == ".." {
        return None;
    }
    let mut ata = Some(klasor);
    while let Some(k) = ata {
        let paket = k.join(PAKET_KLASORU).join(ad);
        if paket.is_dir() {
            return Some(match kalan {
                Some(dosya) => paket.join(dosya),
                None => crate::derleme::proje_girisi(&paket)
                    .unwrap_or_else(|_| paket.join(format!("{ad}.ohc"))),
            });
        }
        ata = k.parent();
    }
    None
}

/// İzin isteyen paketlerde kullanıcıya sorar; onaylanırsa işlemi `izin_ver` ile yineler.
fn izinle(
    izin_ver: bool,
    f: impl Fn(bool) -> Result<Vec<String>, String>,
) -> Result<Vec<String>, String> {
    use std::io::{BufRead, IsTerminal, Write};
    match f(izin_ver) {
        Err(h) if h.starts_with(IZIN_GEREKLI) => {
            let ayrinti = h[IZIN_GEREKLI.len()..].trim_start_matches('\n');
            eprintln!("Paketler şu izinleri istiyor:\n{ayrinti}");
            if !std::io::stdin().is_terminal() {
                return Err("onaylıyorsanız komutu --izin-ver ile yineleyin".into());
            }
            eprint!("Onaylıyor musunuz? [e/H] ");
            let _ = std::io::stderr().flush();
            let mut yanit = String::new();
            let _ = std::io::stdin().lock().read_line(&mut yanit);
            if matches!(yanit.trim().to_lowercase().as_str(), "e" | "evet") {
                f(true)
            } else {
                Err("iptal edildi; hiçbir şey değiştirilmedi".into())
            }
        }
        r => r,
    }
}

/// `orhunca paket ...` komutları
pub fn komut(args: &[String]) -> Result<(), String> {
    let kok = std::env::current_dir().map_err(|e| e.to_string())?;
    let yazdir = |g: Vec<String>| {
        for s in g {
            println!("{s}");
        }
    };
    let izin_ver = args.iter().any(|a| a == "--izin-ver");
    let secenek = |ad: &str| {
        args.iter()
            .position(|a| a == ad)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    };
    match args.first().map(String::as_str) {
        Some("ekle") => {
            let kaynak = args
                .get(1)
                .ok_or("paket adı ya da adresi bekleniyordu: orhunca paket ekle <ad | git-adresi>[#etiket]")?;
            let ad = secenek("--ad");
            yazdir(izinle(izin_ver, |iv| ekle(&kok, kaynak, ad, iv))?);
        }
        Some("ara") | Some("dizin") => {
            let kelime = args.get(1).map(String::as_str).unwrap_or("");
            let bulunan = ara(kelime)?;
            if bulunan.is_empty() {
                println!("'{kelime}' ile eşleşen paket yok.");
            }
            for p in &bulunan {
                let izin = if p.izinler.is_empty() {
                    String::new()
                } else {
                    format!("  [izinler: {}]", p.izinler.join(", "))
                };
                println!("{:<16} {}{izin}", p.ad, p.aciklama);
            }
            if !bulunan.is_empty() {
                println!("\nEklemek için: orhunca paket ekle <ad>");
            }
        }
        Some("bilgi") | Some("incele") => {
            let kaynak = args
                .get(1)
                .filter(|a| !a.starts_with("--"))
                .ok_or("paket adı ya da adresi bekleniyordu: orhunca paket bilgi <ad | git-adresi>[#etiket] [--json]")?;
            let i = incele(kaynak)?;
            if args.iter().any(|a| a == "--json") {
                println!("{}", i.json());
            } else {
                println!("ad:       {}\nsürüm:    {}\naçıklama: {}\nkaynak:   {}\nişleme:   {}\nözet:     {}",
                    i.ad, i.surum, i.aciklama, i.kaynak, i.isleme, i.ozet);
                if i.izinler.is_empty() {
                    println!("izinler:  yok (yalnızca hesaplama yapar)");
                } else {
                    println!("izinler:");
                    for iz in &i.izinler {
                        println!("  • {} — {}", iz.ad(), iz.aciklama());
                    }
                }
            }
        }
        Some("yayımla") | Some("yayimla") => {
            let y = yayimla(&kok, secenek("--etiket"))?;
            println!(
                "{} {} yayıma hazır (izinler: {}).\nKayıt: {}\n",
                y.inceleme.ad,
                y.inceleme.surum,
                if y.inceleme.izinler.is_empty() {
                    "yok".to_string()
                } else {
                    paket_denetim::izin_metni(&y.inceleme.izinler)
                },
                y.dosya.display()
            );
            if y.yeni {
                println!("Açılan GitHub sayfasında \"Propose new file\" ve ardından \"Create pull request\" deyin.");
            } else {
                println!("Açılan GitHub sayfasında dosyanın içeriğini yukarıdaki kayıtla değiştirip \"Propose changes\" deyin.");
            }
            println!("Paket mağazası denetimi geçince paket herkesin kullanımına açılır.\n\n{}", y.adres);
            if !args.iter().any(|a| a == "--tarayıcı-açma" || a == "--tarayici-acma") {
                crate::studyo::tarayicida_ac(&y.adres);
            }
        }
        Some("yükle") | Some("yukle") | Some("kur") => {
            yazdir(izinle(izin_ver, |iv| yukle(&kok, false, iv))?)
        }
        Some("güncelle") | Some("guncelle") => yazdir(izinle(izin_ver, |iv| yukle(&kok, true, iv))?),
        Some("kaldır") | Some("kaldir") | Some("sil") => {
            let ad = args
                .get(1)
                .ok_or("paket adı bekleniyordu: orhunca paket kaldır <ad>")?;
            yazdir(kaldir(&kok, ad)?);
        }
        Some("listele") | Some("liste") | None => {
            let liste = listele(&kok)?;
            if liste.is_empty() {
                println!(
                    "Bu projenin bağımlılığı yok. Eklemek için: orhunca paket ekle <ad | git-adresi>"
                );
            }
            for p in liste {
                let durum = if p.kurulu { "kurulu" } else { "kurulu değil" };
                let isleme = p
                    .isleme
                    .as_deref()
                    .map(|i| &i[..i.len().min(10)])
                    .unwrap_or("-");
                let izin = if p.izinler.is_empty() {
                    String::new()
                } else {
                    format!("  [izinler: {}]", p.izinler)
                };
                println!("{:<20} {:<12} {:<14} {}{izin}", p.ad, isleme, durum, p.kaynak);
            }
        }
        Some(k) => {
            return Err(format!(
                "bilinmeyen paket komutu '{k}'\nkomutlar: ara, bilgi, ekle, yükle, güncelle, kaldır, listele, yayımla"
            ))
        }
    }
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ayar_dosyasi() {
        let mut a = AyarDosyasi::coz("ad = \"x\"\n# yorum\ngiriş = \"ana.ohc\"\n");
        assert_eq!(a.deger(None, &["giriş"]).as_deref(), Some("ana.ohc"));
        a.yaz(BOLUM, "matematik", "github:kisi/matematik");
        a.yaz(BOLUM, "metin", "https://x/metin.git#v1");
        a.yaz(BOLUM, "matematik", "github:kisi/matematik#v2");
        assert_eq!(
            a.metin(),
            "ad = \"x\"\n# yorum\ngiriş = \"ana.ohc\"\n\n[bağımlılıklar]\nmatematik = \"github:kisi/matematik#v2\"\nmetin = \"https://x/metin.git#v1\"\n"
        );
        // Bölüm dışındaki ayarlar bağımlılıklara karışmaz
        assert_eq!(a.bolum(None).len(), 2);
        assert!(a.sil(BOLUM, "metin"));
        assert_eq!(a.bolum(Some(BOLUM)).len(), 1);
    }

    #[test]
    fn kaynak_ayirma() {
        assert_eq!(
            kaynagi_ayir("github:kisi/depo#v1.0"),
            (
                "https://github.com/kisi/depo.git".to_string(),
                Some("v1.0".to_string())
            )
        );
        assert_eq!(
            kaynagi_ayir("/yerel/depo"),
            ("/yerel/depo".to_string(), None)
        );
        assert_eq!(
            depo_adi("https://github.com/kisi/orhunca-matematik.git"),
            "matematik"
        );
    }
}
