//! Stüdyo'dan başlatılan programlar: çıktılarını toplar, girdi gönderir, durdurur.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Bir çalıştırmanın en fazla tutacağı çıktı (bayt).
const EN_FAZLA_CIKTI: usize = 4 * 1024 * 1024;

pub struct Calisma {
    /// (tür, metin): tür "cikti" ya da "hata"
    parcalar: Mutex<Vec<(&'static str, String)>>,
    toplam: Mutex<usize>,
    cocuk: Mutex<Child>,
    girdi: Mutex<Option<ChildStdin>>,
    /// `None`: sürüyor; `Some(kod)`: bitti (kod yoksa sinyalle sonlandı)
    sonuc: Mutex<Option<Option<i32>>>,
    baslangic: Instant,
    sure_ms: Mutex<u128>,
    ayiklayici: Option<Arc<Ayiklayici>>,
}

/// Hata ayıklanan programla bağlantı (çalışma zamanındaki ohc_ay_* ile konuşur).
pub struct Ayiklayici {
    yazici: Mutex<Option<TcpStream>>,
    durum: Mutex<AyDurumu>,
    /// Derlemedeki kaynak dosyalar (tam yollar, `Konum::dosya` sırasıyla)
    dosyalar: Vec<PathBuf>,
}

#[derive(Clone, Default)]
pub struct AyDurumu {
    pub bagli: bool,
    pub durdu: bool,
    pub neden: String,
    pub ileti: Option<String>,
    /// (işlev, dosya, satır); en üstteki ilk
    pub yigin: Vec<(String, String, usize)>,
    /// (ad, tip, değer): seçili çerçevenin değişkenleri
    pub degiskenler: Vec<(String, String, String)>,
    pub cerceve: usize,
    /// Her durakta artar (arayüz yeni durakları buna göre fark eder)
    pub surum: u64,
}

impl Ayiklayici {
    fn dosya_sirasi(&self, yol: &str) -> Option<usize> {
        let tam = std::fs::canonicalize(yol).unwrap_or_else(|_| PathBuf::from(yol));
        self.dosyalar.iter().position(|d| *d == tam)
    }

    /// `(dosya yolu, satır)` kesme noktalarını çalışma zamanının biçimine çevirir.
    fn kesmeler_satiri(&self, kesmeler: &[(String, usize)]) -> String {
        let mut s = String::from("kesmeler");
        for (d, satir) in kesmeler {
            if let Some(i) = self.dosya_sirasi(d) {
                s.push_str(&format!(" {i}:{satir}"));
            }
        }
        s
    }

    fn gonder(&self, satir: &str) -> Result<(), String> {
        let mut y = self.yazici.lock().unwrap();
        let akis = y
            .as_mut()
            .ok_or("program hata ayıklayıcıya henüz bağlanmadı")?;
        akis.write_all(format!("{satir}\n").as_bytes())
            .map_err(|_| "program bitmiş".to_string())
    }

    /// Bir olay ("dur ..." ya da "cdeg ...", "son" satırına kadar) işlenir.
    fn olay(&self, satirlar: &[String]) -> Option<(usize, usize)> {
        let mut d = self.durum.lock().unwrap();
        let mut yer = None;
        let ilk = satirlar.first()?;
        if let Some(i) = ilk.strip_prefix("cdeg ") {
            d.cerceve = i.parse().unwrap_or(0);
            d.degiskenler.clear();
        } else if let Some(neden) = ilk.strip_prefix("dur ") {
            d.durdu = true;
            d.neden = neden.to_string();
            d.ileti = None;
            d.yigin.clear();
            d.degiskenler.clear();
            d.cerceve = 0;
            d.surum += 1;
        }
        for s in &satirlar[1..] {
            if let Some(i) = s.strip_prefix("ileti ") {
                d.ileti = Some(i.to_string());
            } else if let Some(c) = s.strip_prefix("cerceve ") {
                let mut p = c.splitn(4, ' ');
                let _sira = p.next();
                let dosya: usize = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                let satir: usize = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                let islev = p.next().unwrap_or("").to_string();
                // Windows'taki tam yolların `\\?\` öneki gösterilmez.
                let yol = self
                    .dosyalar
                    .get(dosya)
                    .map(|y| {
                        let y = y.display().to_string();
                        y.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(y)
                    })
                    .unwrap_or_default();
                if yer.is_none() {
                    yer = Some((dosya, satir));
                }
                d.yigin.push((islev, yol, satir));
            } else if let Some(v) = s.strip_prefix("deg ") {
                let mut p = v.splitn(3, '\t');
                let ad = p.next().unwrap_or("").to_string();
                let tip = p.next().unwrap_or("").to_string();
                let deger = p.next().unwrap_or("").to_string();
                d.degiskenler.push((ad, tip, deger));
            }
        }
        yer
    }
}

fn tablo() -> &'static Mutex<HashMap<u64, Arc<Calisma>>> {
    static T: OnceLock<Mutex<HashMap<u64, Arc<Calisma>>>> = OnceLock::new();
    T.get_or_init(Default::default)
}

impl Calisma {
    fn ekle(&self, tur: &'static str, metin: String) {
        let mut toplam = self.toplam.lock().unwrap();
        if *toplam > EN_FAZLA_CIKTI {
            return;
        }
        *toplam += metin.len();
        let mut p = self.parcalar.lock().unwrap();
        if *toplam > EN_FAZLA_CIKTI {
            p.push(("hata", "\n… çıktı çok uzun olduğu için kısaltıldı\n".into()));
        } else {
            p.push((tur, metin));
        }
    }
}

/// Bir akışı okuyup parçalara ekler; UTF-8 karakterleri bölünmeden aktarılır.
fn akisi_oku(
    mut kaynak: impl Read + Send + 'static,
    c: Arc<Calisma>,
    tur: &'static str,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut tampon = [0u8; 8192];
        let mut artan: Vec<u8> = Vec::new();
        loop {
            match kaynak.read(&mut tampon) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    artan.extend_from_slice(&tampon[..n]);
                    let gecerli = match std::str::from_utf8(&artan) {
                        Ok(_) => artan.len(),
                        Err(e) if e.error_len().is_none() => e.valid_up_to(),
                        Err(_) => artan.len(),
                    };
                    let parca: Vec<u8> = artan.drain(..gecerli).collect();
                    if !parca.is_empty() {
                        c.ekle(tur, String::from_utf8_lossy(&parca).into_owned());
                    }
                }
            }
        }
        if !artan.is_empty() {
            c.ekle(tur, String::from_utf8_lossy(&artan).into_owned());
        }
    })
}

/// Hata ayıklamalı çalıştırma: dinleyici (kapısı ORHUNCA_AYIKLA ile programa
/// verilir), derlemedeki dosyalar ve başlangıçtaki kesme noktaları.
pub struct AyiklamaBaslangici {
    pub dinleyici: TcpListener,
    pub dosyalar: Vec<String>,
    pub kesmeler: Vec<(String, usize)>,
    /// İlk deyimde durulur (adım adım gösterim); yoksa ilk kesme noktasına kadar çalışır.
    pub ilkte_dur: bool,
}

/// Program bağlanınca kesme noktalarını gönderir ve olayları okur. İlk deyimde
/// program kendiliğinden durur; orada kesme noktası yoksa devam ettirilir.
fn ayiklayiciyi_baslat(
    a: Arc<Ayiklayici>,
    dinleyici: TcpListener,
    kesmeler: Vec<(String, usize)>,
    ilkte_dur: bool,
) {
    std::thread::spawn(move || {
        let _ = dinleyici.set_nonblocking(true);
        let baslangic = Instant::now();
        let akis = loop {
            match dinleyici.accept() {
                Ok((s, _)) => break s,
                Err(_) if baslangic.elapsed() < Duration::from_secs(30) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => return,
            }
        };
        let _ = akis.set_nonblocking(false);
        let _ = akis.set_nodelay(true);
        let Ok(yazici) = akis.try_clone() else { return };
        *a.yazici.lock().unwrap() = Some(yazici);
        a.durum.lock().unwrap().bagli = true;
        let _ = a.gonder(&a.kesmeler_satiri(&kesmeler));
        let mut okuyucu = BufReader::new(akis);
        let mut satirlar: Vec<String> = Vec::new();
        let mut ilk = true;
        let mut satir = String::new();
        loop {
            satir.clear();
            match okuyucu.read_line(&mut satir) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let s = satir.trim_end_matches(['\n', '\r']).to_string();
            if s != "son" {
                satirlar.push(s);
                continue;
            }
            let yer = a.olay(&satirlar);
            if std::mem::take(&mut ilk) {
                let kesmede = yer.is_some_and(|(d, st)| {
                    kesmeler
                        .iter()
                        .any(|(y, ks)| *ks == st && a.dosya_sirasi(y) == Some(d))
                });
                if kesmede {
                    a.durum.lock().unwrap().neden = "kesme".into();
                } else if ilkte_dur {
                    a.durum.lock().unwrap().neden = "adim".into();
                } else {
                    a.durum.lock().unwrap().durdu = false;
                    let _ = a.gonder("devam");
                }
            }
            satirlar.clear();
        }
        let mut d = a.durum.lock().unwrap();
        d.bagli = false;
        d.durdu = false;
    });
}

/// Derlenmiş programı başlatır; `silinecek` program bitince silinir.
pub fn baslat(
    program: &Path,
    klasor: &Path,
    argumanlar: &[String],
    ortam: &[(&str, String)],
    silinecek: PathBuf,
    ayiklama: Option<AyiklamaBaslangici>,
) -> Result<u64, String> {
    let mut ortam: Vec<(&str, String)> = ortam.to_vec();
    if let Some(a) = &ayiklama {
        let kapi = a.dinleyici.local_addr().map_err(|e| e.to_string())?.port();
        ortam.push(("ORHUNCA_AYIKLA", kapi.to_string()));
    }
    let mut cocuk = Command::new(program)
        .args(argumanlar)
        .envs(ortam.iter().map(|(a, d)| (*a, d.as_str())))
        .current_dir(klasor)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("program başlatılamadı: {e}"))?;
    let stdout = cocuk.stdout.take().unwrap();
    let stderr = cocuk.stderr.take().unwrap();
    let girdi = cocuk.stdin.take();
    let c = Arc::new(Calisma {
        parcalar: Mutex::new(Vec::new()),
        toplam: Mutex::new(0),
        cocuk: Mutex::new(cocuk),
        girdi: Mutex::new(girdi),
        sonuc: Mutex::new(None),
        baslangic: Instant::now(),
        sure_ms: Mutex::new(0),
        ayiklayici: ayiklama.as_ref().map(|a| {
            Arc::new(Ayiklayici {
                yazici: Mutex::new(None),
                durum: Mutex::new(AyDurumu::default()),
                dosyalar: a
                    .dosyalar
                    .iter()
                    .map(|d| std::fs::canonicalize(d).unwrap_or_else(|_| PathBuf::from(d)))
                    .collect(),
            })
        }),
    });
    if let (Some(a), Some(b)) = (&c.ayiklayici, ayiklama) {
        ayiklayiciyi_baslat(a.clone(), b.dinleyici, b.kesmeler, b.ilkte_dur);
    }
    let mut okuyucular = Some([
        akisi_oku(stdout, c.clone(), "cikti"),
        akisi_oku(stderr, c.clone(), "hata"),
    ]);

    static SAYAC: AtomicU64 = AtomicU64::new(1);
    let kimlik = SAYAC.fetch_add(1, Ordering::Relaxed);
    tablo().lock().unwrap().insert(kimlik, c.clone());

    // Bekleyici: süreç bitince sonucu kaydeder ve geçici dosyaları siler.
    std::thread::spawn(move || {
        loop {
            let durum = c.cocuk.lock().unwrap().try_wait();
            match durum {
                Ok(Some(d)) => {
                    *c.sure_ms.lock().unwrap() = c.baslangic.elapsed().as_millis();
                    // Sonuç ancak tüm çıktı okunduktan sonra yayımlanır.
                    for o in okuyucular.take().into_iter().flatten() {
                        let _ = o.join();
                    }
                    *c.sonuc.lock().unwrap() = Some(d.code());
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Err(_) => {
                    *c.sonuc.lock().unwrap() = Some(None);
                    break;
                }
            }
        }
        let _ = std::fs::remove_dir_all(&silinecek);
    });
    Ok(kimlik)
}

pub struct Durum {
    pub parcalar: Vec<(&'static str, String)>,
    pub konum: usize,
    pub bitti: bool,
    pub kod: Option<i32>,
    pub sure_ms: u128,
    pub ayiklama: Option<AyDurumu>,
}

/// `konum`dan sonraki çıktıları verir.
pub fn durum(kimlik: u64, konum: usize) -> Option<Durum> {
    let c = tablo().lock().unwrap().get(&kimlik)?.clone();
    // Sonuç, parçalardan önce okunur: "bitti" denildiğinde tüm çıktı gönderilmiş olur.
    let sonuc = *c.sonuc.lock().unwrap();
    let p = c.parcalar.lock().unwrap();
    let yeni: Vec<_> = p.iter().skip(konum).cloned().collect();
    let bitti = sonuc.is_some();
    let d = Durum {
        konum: p.len(),
        parcalar: yeni,
        bitti,
        kod: sonuc.flatten(),
        sure_ms: *c.sure_ms.lock().unwrap(),
        ayiklama: c
            .ayiklayici
            .as_ref()
            .map(|a| a.durum.lock().unwrap().clone()),
    };
    drop(p);
    if bitti {
        // Bitmiş çalıştırmalar son okumadan sonra bir süre daha tutulur.
        let tablo = tablo();
        let mut t = tablo.lock().unwrap();
        if t.len() > 32 {
            t.retain(|_, c| c.sonuc.lock().unwrap().is_none());
        }
    }
    Some(d)
}

pub fn girdi_gonder(kimlik: u64, metin: &str) -> Result<(), String> {
    let c = tablo()
        .lock()
        .unwrap()
        .get(&kimlik)
        .cloned()
        .ok_or("çalıştırma bulunamadı")?;
    let mut g = c.girdi.lock().unwrap();
    let akis = g.as_mut().ok_or("program girdi beklemiyor")?;
    akis.write_all(metin.as_bytes())
        .and_then(|_| akis.flush())
        .map_err(|_| "program girdiyi kabul etmedi (bitmiş olabilir)".to_string())
}

/// Stüdyo kapanırken çalışan tüm programlar (ör. web sunucuları) durdurulur.
pub fn hepsini_durdur() {
    let hepsi: Vec<Arc<Calisma>> = tablo().lock().unwrap().values().cloned().collect();
    for c in hepsi {
        let _ = c.cocuk.lock().unwrap().kill();
    }
}

/// Hata ayıklama komutu: devam, adim, ustunden, cik, duraklat, kesmeler, cerceve.
pub fn ayiklama_komutu(
    kimlik: u64,
    komut: &str,
    kesmeler: &[(String, usize)],
    cerceve: usize,
) -> Result<(), String> {
    let c = tablo()
        .lock()
        .unwrap()
        .get(&kimlik)
        .cloned()
        .ok_or("çalıştırma bulunamadı")?;
    let a = c
        .ayiklayici
        .as_ref()
        .ok_or("bu çalıştırma hata ayıklamalı değil")?;
    match komut {
        "devam" | "adim" | "ustunden" | "cik" => {
            let mut d = a.durum.lock().unwrap();
            if !d.durdu {
                return Err("program şu an durmuş değil".into());
            }
            d.durdu = false;
            drop(d);
            a.gonder(komut)
        }
        "duraklat" => a.gonder("duraklat"),
        "kesmeler" => a.gonder(&a.kesmeler_satiri(kesmeler)),
        "cerceve" => a.gonder(&format!("cerceve {cerceve}")),
        k => Err(format!("bilinmeyen hata ayıklama komutu '{k}'")),
    }
}

pub fn durdur(kimlik: u64) {
    if let Some(c) = tablo().lock().unwrap().get(&kimlik).cloned() {
        let _ = c.cocuk.lock().unwrap().kill();
        c.girdi.lock().unwrap().take();
    }
}
