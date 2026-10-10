//! Sınamalar (testler): `doğrula` ve `eşit_olmalı` ile `orhunca sına`.
//!
//! - `doğrula(koşul)` · `doğrula(koşul, açıklama)`: koşul yanlışsa çalışma hatası.
//! - `eşit_olmalı(gerçek, beklenen)`: iki değer farklıysa ikisini de gösteren çalışma hatası.
//!
//! İkisi de ayrıştırmadan hemen sonra sıradan Orhunca deyimlerine çevrilir
//! (`eğer ... ise: hata_ver(...)`); böylece her tiple çalışırlar, tip denetimi ve bütün
//! hedefler (yerel, WebAssembly, Python/JS çevirisi) ayrıca bir şey bilmek zorunda kalmaz.
//! Programın kendi `doğrula` işlevi ya da fiili varsa ona dokunulmaz.
//!
//! `orhunca sına`: adı `_sına.ohc` ile biten (ya da `sınama_` ile başlayan) dosyalarda ve
//! `sınamalar/` klasöründe, adı `sına_` ile başlayan parametresiz işlevleri bulur ve her birini
//! ayrı ayrı çalıştırır. `sına_` işlevi olmayan bir sınama dosyası, üst düzeydeki
//! `eşit_olmalı`/`doğrula` satırlarıyla birlikte tek bir sınama olarak çalışır (yeni
//! başlayanlar için en kısa yol).

use crate::agac::{Deyim, Ifade, IfadeTuru, IkiliOp, Program, TekliOp};
use crate::hata::{Hata, Konum};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const DOGRULA: &str = "doğrula";
pub const ESIT_OLMALI: &str = "eşit_olmalı";

fn ifade(tur: IfadeTuru, k: Konum) -> Ifade {
    Ifade::yeni(tur, k)
}

fn metin(m: String, k: Konum) -> Ifade {
    ifade(IfadeTuru::Metin(m), k)
}

fn topla(a: Ifade, b: Ifade) -> Ifade {
    let k = a.konum;
    ifade(
        IfadeTuru::Ikili(IkiliOp::Topla, Box::new(a), Box::new(b)),
        k,
    )
}

fn cagri(ad: &str, args: Vec<Ifade>, k: Konum) -> Ifade {
    ifade(IfadeTuru::Cagri(ad.into(), args), k)
}

fn hata_ver(mesaj: Ifade) -> Deyim {
    let k = mesaj.konum;
    Deyim::IfadeDeyimi(cagri("hata_ver", vec![mesaj], k))
}

struct Indirgeyici<'a> {
    dosyalar: &'a [(String, String)],
    dogrula: bool,
    esit: bool,
}

impl Indirgeyici<'_> {
    /// Hata mesajının başı: `hesap_sına.ohc:12`
    fn yer(&self, k: Konum) -> String {
        let ad = self
            .dosyalar
            .get(k.dosya)
            .map(|d| {
                Path::new(&d.0)
                    .file_name()
                    .map(|a| a.to_string_lossy().into_owned())
                    .unwrap_or_else(|| d.0.clone())
            })
            .unwrap_or_default();
        format!("{ad}:{}", k.satir)
    }

    fn satir(&self, k: Konum) -> String {
        self.dosyalar
            .get(k.dosya)
            .and_then(|d| d.1.lines().nth(k.satir.saturating_sub(1)))
            .unwrap_or("")
            .trim()
            .to_string()
    }

    fn blok(&self, govde: &mut Vec<Deyim>) -> Result<(), Hata> {
        let eski = std::mem::take(govde);
        for mut d in eski {
            match &mut d {
                Deyim::Eger {
                    govde: a,
                    degilse: b,
                    ..
                } => {
                    self.blok(a)?;
                    self.blok(b)?;
                }
                Deyim::Surece { govde: a, .. }
                | Deyim::HerAralik { govde: a, .. }
                | Deyim::HerListe { govde: a, .. } => self.blok(a)?,
                Deyim::Dene {
                    govde: a,
                    yakala: b,
                    ..
                } => {
                    self.blok(a)?;
                    self.blok(b)?;
                }
                _ => {}
            }
            match d {
                Deyim::IfadeDeyimi(Ifade {
                    tur: IfadeTuru::Cagri(ad, args),
                    konum,
                    ..
                }) if (ad == DOGRULA && self.dogrula) || (ad == ESIT_OLMALI && self.esit) => {
                    govde.extend(self.indir(&ad, args, konum)?);
                }
                d => govde.push(d),
            }
        }
        Ok(())
    }

    fn indir(&self, ad: &str, mut args: Vec<Ifade>, k: Konum) -> Result<Vec<Deyim>, Hata> {
        let yer = self.yer(k);
        if ad == DOGRULA {
            if !(1..=2).contains(&args.len()) {
                return Err(Hata::yeni(
                    k,
                    "doğrula bir koşul alır: doğrula(x > 0) ya da doğrula(x > 0, \"açıklama\")",
                ));
            }
            let aciklama = (args.len() == 2).then(|| args.pop().unwrap());
            let kosul = args.pop().unwrap();
            let mut mesaj = metin(format!("{yer}: doğrulanamadı: {}", self.satir(k)), k);
            if let Some(a) = aciklama {
                mesaj = topla(topla(mesaj, metin(" — ".into(), k)), a);
            }
            return Ok(vec![Deyim::Eger {
                kosul: ifade(IfadeTuru::Tekli(TekliOp::Degil, Box::new(kosul)), k),
                govde: vec![hata_ver(mesaj)],
                degilse: vec![],
            }]);
        }
        if args.len() != 2 {
            return Err(Hata::yeni(
                k,
                "eşit_olmalı iki değer alır: eşit_olmalı(gerçek, beklenen)",
            ));
        }
        // Değerler bir kez hesaplanır (yan etkileri iki kez olmasın).
        let beklenen = args.pop().unwrap();
        let gercek = args.pop().unwrap();
        let g_ad = format!("sınama_gerçek_{}_{}", k.satir, k.sutun);
        let b_ad = format!("sınama_beklenen_{}_{}", k.satir, k.sutun);
        let isim = |a: &str| ifade(IfadeTuru::Isim(a.into()), k);
        // Karşılaştırma metin hâlleriyle yapılır: listeler, sözlükler ve modeller de
        // karşılaştırılabilsin. İki değerin aynı tipte olması, ikisini bir listeye koyarak
        // denetlenir (eşit_olmalı(1, "1") derleme hatası verir).
        let tip_denetimi = Deyim::Atama {
            hedef: format!("sınama_tipler_{}_{}", k.satir, k.sutun),
            tip: None,
            deger: ifade(IfadeTuru::Liste(vec![isim(&g_ad), isim(&b_ad)]), k),
            konum: k,
        };
        let metni = |a: &str| cagri("metin", vec![isim(a)], k);
        let mesaj = topla(
            topla(
                topla(
                    metin(format!("{yer}: beklenen "), k),
                    cagri("metin", vec![isim(&b_ad)], k),
                ),
                metin(", bulunan ".into(), k),
            ),
            cagri("metin", vec![isim(&g_ad)], k),
        );
        Ok(vec![
            Deyim::Atama {
                hedef: g_ad.clone(),
                tip: None,
                deger: gercek,
                konum: k,
            },
            Deyim::Atama {
                hedef: b_ad.clone(),
                tip: None,
                deger: beklenen,
                konum: k,
            },
            tip_denetimi,
            Deyim::Eger {
                kosul: ifade(
                    IfadeTuru::Ikili(
                        IkiliOp::EsitDegil,
                        Box::new(metni(&g_ad)),
                        Box::new(metni(&b_ad)),
                    ),
                    k,
                ),
                govde: vec![hata_ver(mesaj)],
                degilse: vec![],
            },
        ])
    }
}

/// `eşit_olmalı`nın tip denetimi (iki değeri bir listeye koymak) hata verirse mesajı
/// kullanıcının yazdığına göre söyler.
pub fn hatayi_acikla(mut h: Hata, dosyalar: &[(String, String)]) -> Hata {
    let satir = dosyalar
        .get(h.konum.dosya)
        .and_then(|d| d.1.lines().nth(h.konum.satir.saturating_sub(1)))
        .unwrap_or("");
    if let Some(tipler) = h
        .mesaj
        .strip_prefix("listenin tüm öğeleri aynı tipte olmalı ")
    {
        if satir.contains(&format!("{ESIT_OLMALI}(")) {
            h.mesaj = format!("eşit_olmalı: gerçek ve beklenen değer aynı tipte olmalı {tipler}");
            h.ipucu = Some("gerekirse dönüştürün: eşit_olmalı(metin(x), \"5\")".into());
            h.oneri = None;
        }
    }
    h
}

/// `doğrula` ve `eşit_olmalı` çağrılarını sıradan deyimlere çevirir (tip denetiminden önce).
pub fn indir(p: &mut Program, dosyalar: &[(String, String)]) -> Result<(), Hata> {
    let tanimli = |ad: &str| p.islevler.iter().any(|f| f.ad == ad);
    let i = Indirgeyici {
        dosyalar,
        dogrula: !tanimli(DOGRULA),
        esit: !tanimli(ESIT_OLMALI),
    };
    if !i.dogrula && !i.esit {
        return Ok(());
    }
    for f in p.islevler.iter_mut() {
        i.blok(&mut f.govde)?;
    }
    i.blok(&mut p.ana)
}

// ---------------------------------------------------------------------------
// orhunca sına
// ---------------------------------------------------------------------------

const ISARET: &str = "⟦orhunca-sına⟧";

fn sinama_dosyasi_mi(yol: &Path) -> bool {
    if yol.extension().and_then(|e| e.to_str()) != Some("ohc") {
        return false;
    }
    let ad = yol
        .file_stem()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_default();
    ad.ends_with("_sına")
        || ad.ends_with("_sina")
        || ad.starts_with("sınama_")
        || ad.starts_with("sinama_")
        || yol
            .parent()
            .and_then(|k| k.file_name())
            .is_some_and(|k| k == "sınamalar" || k == "sinamalar")
}

/// Klasördeki sınama dosyaları (gizli klasörler, derleme çıktıları ve paketler hariç).
pub fn dosyalari_bul(kok: &Path) -> Vec<PathBuf> {
    fn gez(k: &Path, l: &mut Vec<PathBuf>) {
        let Ok(g) = std::fs::read_dir(k) else { return };
        for x in g.flatten() {
            let ad = x.file_name().to_string_lossy().into_owned();
            if ad.starts_with('.') || matches!(ad.as_str(), "cikti" | "target" | "paketler") {
                continue;
            }
            let p = x.path();
            if p.is_dir() {
                gez(&p, l);
            } else if sinama_dosyasi_mi(&p) {
                l.push(p);
            }
        }
    }
    let mut l = Vec::new();
    if kok.is_file() {
        l.push(kok.to_path_buf());
    } else {
        gez(kok, &mut l);
    }
    l.sort();
    l
}

#[derive(Debug, Clone)]
pub struct Sonuc {
    pub ad: String,
    pub satir: usize,
    pub gecti: bool,
    pub mesaj: String,
    pub cikti: String,
}

#[derive(Debug, Clone)]
pub struct DosyaSonucu {
    pub dosya: PathBuf,
    pub sinamalar: Vec<Sonuc>,
    /// Derleme hatası ya da programın beklenmedik biçimde bitmesi
    pub hata: Option<String>,
    pub sure: Duration,
}

/// Dosyadaki sınama işlevleri: (ad, satır).
fn sinama_islevleri(p: &Program, suzgec: Option<&str>) -> Vec<(String, usize)> {
    p.islevler
        .iter()
        .filter(|f| {
            f.konum.dosya == 0
                && f.parametreler.is_empty()
                && f.haller.is_empty()
                && f.rota.is_none()
                && (f.ad.starts_with("sına_") || f.ad.starts_with("sina_"))
                && suzgec.is_none_or(|s| match s.strip_prefix('=') {
                    Some(tam) => f.ad == tam,
                    None => f.ad.contains(s),
                })
        })
        .map(|f| (f.ad.clone(), f.konum.satir))
        .collect()
}

/// Bir sınama dosyasını çalıştırır: her sınama işlevi `dene:` içinde çağrılır.
pub fn dosyayi_sina(dosya: &Path, suzgec: Option<&str>, sure_siniri: Duration) -> DosyaSonucu {
    let bas = Instant::now();
    let mut sonuc = DosyaSonucu {
        dosya: dosya.to_path_buf(),
        sinamalar: vec![],
        hata: None,
        sure: Duration::ZERO,
    };
    let bitir = |mut s: DosyaSonucu, hata: Option<String>| {
        s.hata = hata;
        s.sure = bas.elapsed();
        s
    };
    let program = match crate::derleme::yukle(dosya) {
        Ok(p) => p,
        Err(h) => return bitir(sonuc, Some(h.metin)),
    };
    let islevler = sinama_islevleri(&program, suzgec);
    if islevler.is_empty() {
        // `sına_` işlevi yoksa dosyanın kendisi tek bir sınamadır (üst düzeyde eşit_olmalı…).
        let ad = dosya
            .file_stem()
            .map(|a| a.to_string_lossy().into_owned())
            .unwrap_or_default();
        let uyuyor = suzgec.is_none_or(|s| match s.strip_prefix('=') {
            Some(tam) => ad == tam,
            None => ad.contains(s),
        });
        if !uyuyor || program.ana.is_empty() {
            return bitir(sonuc, None);
        }
        return match ust_duzey_sina(dosya, sure_siniri) {
            Ok(t) => {
                sonuc.sinamalar.push(t);
                bitir(sonuc, None)
            }
            Err(e) => bitir(sonuc, Some(e)),
        };
    }
    let kaynak = match std::fs::read_to_string(dosya) {
        Ok(k) => k,
        Err(e) => return bitir(sonuc, Some(e.to_string())),
    };
    let mut ek = format!("\n\n# {ISARET}\n");
    for (ad, _) in &islevler {
        ek.push_str(&format!(
            "dene:\n    {ad}()\n    \"{ISARET} geçti {ad}\"'ı yaz.\nyakala sınama_hatası:\n    (\"{ISARET} kaldı {ad} \" + sınama_hatası)'yı yaz.\n"
        ));
    }
    let mut ortulu = std::collections::HashMap::new();
    let tam = std::fs::canonicalize(dosya).unwrap_or_else(|_| dosya.to_path_buf());
    ortulu.insert(tam, format!("{}{ek}", kaynak.trim_end()));
    let gecici = match crate::derleme::gecici_klasor("sina") {
        Ok(k) => k,
        Err(e) => return bitir(sonuc, Some(e)),
    };
    let program_yolu = gecici.join(if cfg!(windows) {
        "sinama.exe"
    } else {
        "sinama"
    });
    if let Err(h) = crate::derleme::derle_ortulu(dosya, &ortulu, &program_yolu) {
        let _ = std::fs::remove_dir_all(&gecici);
        return bitir(sonuc, Some(h.metin));
    }
    let klasor = dosya
        .parent()
        .filter(|k| !k.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let calisma = calistir(&program_yolu, klasor, sure_siniri);
    let _ = std::fs::remove_dir_all(&gecici);
    let (cikti, kod, sure_doldu) = match calisma {
        Ok(c) => c,
        Err(e) => return bitir(sonuc, Some(e)),
    };
    let mut tampon = String::new();
    for satir in cikti.lines() {
        let Some(geri) = satir.strip_prefix(ISARET) else {
            tampon.push_str(satir);
            tampon.push('\n');
            continue;
        };
        let geri = geri.trim_start();
        let (gecti, geri) = match geri.split_once(' ') {
            Some(("geçti", g)) => (true, g),
            Some(("kaldı", g)) => (false, g),
            _ => continue,
        };
        let (ad, mesaj) = geri.split_once(' ').unwrap_or((geri, ""));
        let satir_no = islevler
            .iter()
            .find(|(a, _)| a == ad)
            .map(|(_, s)| *s)
            .unwrap_or(0);
        sonuc.sinamalar.push(Sonuc {
            ad: ad.to_string(),
            satir: satir_no,
            gecti,
            mesaj: mesaj.to_string(),
            cikti: std::mem::take(&mut tampon),
        });
    }
    // Sonucu gelmeyen sınamalar: program çık() ile ya da çökerek bitti.
    let neden = if sure_doldu {
        format!("süre doldu ({} sn)", sure_siniri.as_secs())
    } else {
        format!("program beklenmedik biçimde bitti (çıkış kodu {kod})")
    };
    for (ad, satir) in &islevler {
        if !sonuc.sinamalar.iter().any(|s| &s.ad == ad) {
            sonuc.sinamalar.push(Sonuc {
                ad: ad.clone(),
                satir: *satir,
                gecti: false,
                mesaj: neden.clone(),
                cikti: std::mem::take(&mut tampon),
            });
        }
    }
    bitir(sonuc, None)
}

/// `sına_` işlevi olmayan sınama dosyası: program olduğu gibi çalışır; hatasız biterse geçer.
fn ust_duzey_sina(dosya: &Path, sure_siniri: Duration) -> Result<Sonuc, String> {
    let gecici = crate::derleme::gecici_klasor("sina")?;
    let program_yolu = gecici.join(if cfg!(windows) {
        "sinama.exe"
    } else {
        "sinama"
    });
    if let Err(h) = crate::derleme::derle_ortulu(dosya, &Default::default(), &program_yolu) {
        let _ = std::fs::remove_dir_all(&gecici);
        return Err(h.metin);
    }
    let klasor = dosya
        .parent()
        .filter(|k| !k.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let calisma = calistir(&program_yolu, klasor, sure_siniri);
    let _ = std::fs::remove_dir_all(&gecici);
    let (cikti, kod, sure_doldu) = calisma?;
    let ad = dosya
        .file_stem()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Çalışma hatasının mesajı: "Çalışma hatası (satır 3): hesap_sına.ohc:3: beklenen 5, bulunan 4"
    let hata_satiri = cikti
        .lines()
        .rev()
        .find(|s| s.starts_with("Çalışma hatası"));
    let mesaj = match hata_satiri {
        Some(h) => h.split_once("): ").map_or(h, |(_, m)| m).to_string(),
        None if sure_doldu => format!("süre doldu ({} sn)", sure_siniri.as_secs()),
        None if kod != 0 => format!("program beklenmedik biçimde bitti (çıkış kodu {kod})"),
        None => String::new(),
    };
    let satir = hata_satiri
        .and_then(|h| h.strip_prefix("Çalışma hatası (satır "))
        .and_then(|h| h.split(')').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);
    let gecti = mesaj.is_empty();
    let cikti = cikti
        .lines()
        .filter(|s| Some(*s) != hata_satiri)
        .map(|s| format!("{s}\n"))
        .collect();
    Ok(Sonuc {
        ad,
        satir,
        gecti,
        mesaj,
        cikti,
    })
}

/// Programı çalıştırır; (çıktı ve hata akışı, çıkış kodu, süre doldu mu).
/// Her sınama kendi boş veri klasörüyle çalışır: modellerin kayıtları projenin gerçek `veri/`
/// klasörüne ya da veritabanı sunucusuna dokunmaz ve sınamalar birbirini etkilemez.
fn calistir(program: &Path, klasor: &Path, sinir: Duration) -> Result<(String, i32, bool), String> {
    static SIRA: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let veri = std::env::temp_dir().join(format!(
        "orhunca-sinama-verisi-{}-{}",
        std::process::id(),
        SIRA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let sonuc = calistir_verisiyle(program, klasor, sinir, &veri);
    let _ = std::fs::remove_dir_all(&veri);
    sonuc
}

fn calistir_verisiyle(
    program: &Path,
    klasor: &Path,
    sinir: Duration,
    veri: &Path,
) -> Result<(String, i32, bool), String> {
    use std::io::Read;
    let mut c = crate::komut(program)
        .current_dir(klasor)
        .env("ORHUNCA_VERI", veri)
        // Sunucu veritabanı (PostgreSQL, MySQL, SQL Server) ayarlıysa sınamalar onun yerine
        // geçici klasördeki SQLite dosyasını kullanır.
        .env("ORHUNCA_SINAMA", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("sınama programı başlatılamadı: {e}"))?;
    let mut cikti_akisi = c.stdout.take().unwrap();
    let mut hata_akisi = c.stderr.take().unwrap();
    let o = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = cikti_akisi.read_to_end(&mut v);
        v
    });
    let h = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = hata_akisi.read_to_end(&mut v);
        v
    });
    let bas = Instant::now();
    let (kod, doldu) = loop {
        if let Some(d) = c.try_wait().map_err(|e| e.to_string())? {
            break (d.code().unwrap_or(-1), false);
        }
        if bas.elapsed() > sinir {
            let _ = c.kill();
            let _ = c.wait();
            break (-1, true);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut metin = String::from_utf8_lossy(&o.join().unwrap_or_default()).into_owned();
    let hata = String::from_utf8_lossy(&h.join().unwrap_or_default()).into_owned();
    if !hata.trim().is_empty() {
        metin.push_str(&hata);
    }
    Ok((metin, kod, doldu))
}

fn goreli_yol(p: &Path, kok: &Path) -> String {
    let p = p.strip_prefix(kok).unwrap_or(p);
    let p = p.strip_prefix(".").unwrap_or(p);
    p.to_string_lossy().replace('\\', "/")
}

pub fn json_sonuc(l: &[DosyaSonucu], kok: &Path) -> Value {
    let goreli = |p: &Path| goreli_yol(p, kok);
    let gecen = l
        .iter()
        .flat_map(|d| &d.sinamalar)
        .filter(|s| s.gecti)
        .count();
    let kalan = l
        .iter()
        .flat_map(|d| &d.sinamalar)
        .filter(|s| !s.gecti)
        .count()
        + l.iter().filter(|d| d.hata.is_some()).count();
    json!({
        "gecti": gecen,
        "kaldi": kalan,
        "dosyalar": l.iter().map(|d| json!({
            "dosya": goreli(&d.dosya),
            "hata": d.hata,
            "sure_ms": d.sure.as_millis() as u64,
            "sinamalar": d.sinamalar.iter().map(|s| json!({
                "ad": s.ad, "satir": s.satir, "gecti": s.gecti, "mesaj": s.mesaj, "cikti": s.cikti,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// Projedeki sınamalar (çalıştırmadan): Stüdyo'nun Sınamalar paneli için.
pub fn listele(kok: &Path) -> Value {
    let dosyalar: Vec<Value> = dosyalari_bul(kok)
        .into_iter()
        .map(|d| {
            let (sinamalar, hata) = match crate::derleme::yukle(&d) {
                Ok(p) => (
                    sinama_islevleri(&p, None)
                        .into_iter()
                        .map(|(ad, satir)| json!({ "ad": ad, "satir": satir }))
                        .collect(),
                    None,
                ),
                Err(h) => (vec![], Some(h.metin)),
            };
            json!({ "dosya": goreli_yol(&d, kok), "sinamalar": sinamalar, "hata": hata })
        })
        .collect();
    json!({ "dosyalar": dosyalar })
}

/// Stüdyo: projedeki bütün sınamalar, bir dosya ya da tek bir sınama.
pub fn calistir_json(kok: &Path, dosya: Option<&Path>, ad: Option<&str>) -> Value {
    let dosyalar = match dosya {
        Some(d) => vec![d.to_path_buf()],
        None => dosyalari_bul(kok),
    };
    let sonuclar: Vec<DosyaSonucu> = dosyalar
        .iter()
        .map(|d| dosyayi_sina(d, ad, Duration::from_secs(60)))
        .collect();
    json_sonuc(&sonuclar, kok)
}

/// `orhunca sına [dosya ya da klasör ...] [--ad parça] [--json]`
pub fn komut(args: &[String]) -> Result<bool, String> {
    let mut yollar = Vec::new();
    let mut suzgec = None;
    let mut json_cikti = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json_cikti = true,
            "--ad" => {
                i += 1;
                suzgec = Some(args.get(i).ok_or("--ad bir sınama adı (ya da parçası) alır")?.clone());
            }
            a if a.starts_with("--") => {
                return Err(format!(
                    "bilinmeyen seçenek '{a}'\nKullanım: orhunca sına [dosya ya da klasör ...] [--ad parça] [--json]"
                ))
            }
            a => yollar.push(PathBuf::from(a)),
        }
        i += 1;
    }
    let kok = std::env::current_dir().unwrap_or_default();
    if yollar.is_empty() {
        yollar.push(PathBuf::from("."));
    }
    let mut dosyalar = Vec::new();
    for y in &yollar {
        if !y.exists() {
            return Err(format!("'{}' bulunamadı", y.display()));
        }
        dosyalar.extend(dosyalari_bul(y));
    }
    if dosyalar.is_empty() {
        let m = "Sınama bulunamadı.\nSınama dosyalarının adı _sına.ohc ile biter (ya da sınamalar/ klasöründedir). \
                 En kısa yol: hesap_sına.ohc dosyasına eşit_olmalı(2 + 3, 5) yazın. Birden çok sınama için \
                 adı sına_ ile başlayan işlevler yazın:\n\n    işlev sına_toplama():\n        eşit_olmalı(2 + 3, 5)";
        if json_cikti {
            println!("{}", json_sonuc(&[], &kok));
            return Ok(true);
        }
        return Err(m.into());
    }
    let bas = Instant::now();
    let mut sonuclar = Vec::new();
    for d in &dosyalar {
        let s = dosyayi_sina(d, suzgec.as_deref(), Duration::from_secs(60));
        if !json_cikti {
            yazdir(&s, &kok);
        }
        sonuclar.push(s);
    }
    let j = json_sonuc(&sonuclar, &kok);
    let basarili = j["kaldi"] == 0;
    if json_cikti {
        println!("{j}");
    } else {
        let toplam = j["gecti"].as_u64().unwrap_or(0) + j["kaldi"].as_u64().unwrap_or(0);
        println!(
            "\n{toplam} sınama: {} geçti, {} kaldı ({:.1} sn)",
            j["gecti"],
            j["kaldi"],
            bas.elapsed().as_secs_f64()
        );
    }
    Ok(basarili)
}

fn yazdir(s: &DosyaSonucu, kok: &Path) {
    println!("{}", goreli_yol(&s.dosya, kok));
    if let Some(h) = &s.hata {
        for satir in h.lines() {
            println!("  ✗ {satir}");
        }
        return;
    }
    if s.sinamalar.is_empty() {
        println!("  (sına_ ile başlayan işlev yok)");
    }
    for t in &s.sinamalar {
        if t.gecti {
            println!("  ✓ {}", t.ad);
        } else {
            println!("  ✗ {}\n      {}", t.ad, t.mesaj);
            for satir in t.cikti.lines() {
                println!("      │ {satir}");
            }
        }
    }
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn dosya_adlari() {
        assert!(sinama_dosyasi_mi(Path::new("a/hesap_sına.ohc")));
        assert!(sinama_dosyasi_mi(Path::new("hesap_sina.ohc")));
        assert!(sinama_dosyasi_mi(Path::new("p/sınamalar/hesap.ohc")));
        assert!(sinama_dosyasi_mi(Path::new("sınama_hesap.ohc")));
        assert!(!sinama_dosyasi_mi(Path::new("hesap.ohc")));
        assert!(!sinama_dosyasi_mi(Path::new("hesap_sına.txt")));
    }
}
