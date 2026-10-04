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

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Paket kaynağı: `adres#etiket`. `github:kişi/depo` kısaltması kabul edilir.
fn kaynagi_ayir(kaynak: &str) -> (String, Option<String>) {
    let (adres, etiket) = match kaynak.rsplit_once('#') {
        Some((a, e)) if !e.is_empty() => (a, Some(e.to_string())),
        _ => (kaynak, None),
    };
    let adres = match adres.strip_prefix("github:") {
        Some(yol) => format!("https://github.com/{}.git", yol.trim_end_matches(".git")),
        None => adres.to_string(),
    };
    (adres, etiket)
}

fn git(args: &[&str], klasor: Option<&Path>) -> Result<String, String> {
    let mut k = Command::new("git");
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
    let (adres, etiket) = kaynagi_ayir(kaynak);
    // Zaten istenen işlemede kuruluysa dokunulmaz.
    if let (Some(i), true) = (isleme, hedef.join(".git").exists()) {
        if git(&["rev-parse", "HEAD"], Some(hedef)).ok().as_deref() == Some(i) {
            return Ok(i.to_string());
        }
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

fn kilidi_oku(kok: &Path) -> HashMap<String, (String, String)> {
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
            sonuc.insert(b, (k, i));
        }
    }
    sonuc
}

fn kilidi_yaz(kok: &Path, kurulan: &BTreeMap<String, (String, String)>) -> Result<(), String> {
    let mut s =
        String::from("# Bu dosya `orhunca paket` tarafından üretilir; elle düzenlemeyin.\n");
    for (ad, (kaynak, isleme)) in kurulan {
        s.push_str(&format!(
            "\n[{ad}]\nkaynak = \"{kaynak}\"\nişleme = \"{isleme}\"\n"
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

/// Tüm bağımlılıkları (dolaylı olanlar dahil) `paketler/` klasörüne kurar.
/// `guncelle`: kilitteki sürümler yok sayılır, en yeni sürümler alınır.
pub fn yukle(kok: &Path, guncelle: bool) -> Result<Vec<String>, String> {
    let proje = Proje::ac(kok)?;
    let kilit = if guncelle {
        HashMap::new()
    } else {
        kilidi_oku(kok)
    };
    let mut gunluk = Vec::new();
    let mut kurulan: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut kuyruk: Vec<(String, String, String)> = proje
        .bagimliliklar()
        .into_iter()
        .map(|(a, k)| (a, k, proje.ad()))
        .rev()
        .collect();
    while let Some((ad, kaynak, isteyen)) = kuyruk.pop() {
        if let Some((k, _)) = kurulan.get(&ad) {
            if kaynagi_ayir(k).0 != kaynagi_ayir(&kaynak).0 {
                return Err(format!(
                    "'{ad}' paketi iki farklı kaynaktan isteniyor: '{k}' ve '{kaynak}' ({isteyen})"
                ));
            }
            continue;
        }
        let hedef = kok.join(PAKET_KLASORU).join(&ad);
        let kilitli = kilit
            .get(&ad)
            .filter(|(k, _)| *k == kaynak)
            .map(|(_, i)| i.as_str());
        let isleme = getir(&kaynak, kilitli, &hedef, &mut gunluk)?;
        gunluk.push(format!("✓ {ad} ({})", &isleme[..isleme.len().min(10)]));
        if let Ok(p) = Proje::ac(&hedef) {
            for (a, k) in p.bagimliliklar().into_iter().rev() {
                kuyruk.push((a, k, ad.clone()));
            }
        } else {
            gunluk.push(format!(
                "uyarı: '{ad}' paketinde .ohcproj yok; giriş dosyası {ad}.ohc varsayılır"
            ));
        }
        kurulan.insert(ad, (kaynak, isleme));
    }
    // Artık istenmeyen paketler silinir.
    if let Ok(okunan) = std::fs::read_dir(kok.join(PAKET_KLASORU)) {
        for g in okunan.filter_map(|g| g.ok()) {
            let ad = g.file_name().to_string_lossy().into_owned();
            if !kurulan.contains_key(&ad) && g.path().is_dir() {
                let _ = std::fs::remove_dir_all(g.path());
                gunluk.push(format!("kaldırıldı: {ad}"));
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
pub fn ekle(kok: &Path, kaynak: &str, ad: Option<&str>) -> Result<Vec<String>, String> {
    let mut proje = Proje::ac(kok)?;
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
    proje.ayarlar.yaz(BOLUM, &ad, kaynak);
    proje.kaydet()?;
    let mut gunluk = vec![format!("eklendi: {ad} = \"{kaynak}\"")];
    gunluk.extend(yukle(kok, false)?);
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
    gunluk.extend(yukle(kok, false)?);
    Ok(gunluk)
}

pub struct PaketBilgisi {
    pub ad: String,
    pub kaynak: String,
    pub isleme: Option<String>,
    pub kurulu: bool,
}

pub fn listele(kok: &Path) -> Result<Vec<PaketBilgisi>, String> {
    let proje = Proje::ac(kok)?;
    let kilit = kilidi_oku(kok);
    Ok(proje
        .bagimliliklar()
        .into_iter()
        .map(|(ad, kaynak)| PaketBilgisi {
            kurulu: kok.join(PAKET_KLASORU).join(&ad).is_dir(),
            isleme: kilit.get(&ad).map(|(_, i)| i.clone()),
            ad,
            kaynak,
        })
        .collect())
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

/// `orhunca paket ...` komutları
pub fn komut(args: &[String]) -> Result<(), String> {
    let kok = std::env::current_dir().map_err(|e| e.to_string())?;
    let yazdir = |g: Vec<String>| {
        for s in g {
            println!("{s}");
        }
    };
    match args.first().map(String::as_str) {
        Some("ekle") => {
            let kaynak = args
                .get(1)
                .ok_or("paket adresi bekleniyordu: orhunca paket ekle <git-adresi>[#etiket]")?;
            let ad = args
                .iter()
                .position(|a| a == "--ad")
                .and_then(|i| args.get(i + 1));
            yazdir(ekle(&kok, kaynak, ad.map(String::as_str))?);
        }
        Some("yükle") | Some("yukle") | Some("kur") => yazdir(yukle(&kok, false)?),
        Some("güncelle") | Some("guncelle") => yazdir(yukle(&kok, true)?),
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
                    "Bu projenin bağımlılığı yok. Eklemek için: orhunca paket ekle <git-adresi>"
                );
            }
            for p in liste {
                let durum = if p.kurulu { "kurulu" } else { "kurulu değil" };
                let isleme = p
                    .isleme
                    .as_deref()
                    .map(|i| &i[..i.len().min(10)])
                    .unwrap_or("-");
                println!("{:<20} {:<12} {:<14} {}", p.ad, isleme, durum, p.kaynak);
            }
        }
        Some(k) => {
            return Err(format!(
                "bilinmeyen paket komutu '{k}'\nkomutlar: ekle, yükle, güncelle, kaldır, listele"
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
