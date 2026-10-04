//! Stüdyo'dan başlatılan programlar: çıktılarını toplar, girdi gönderir, durdurur.

use std::collections::HashMap;
use std::io::{Read, Write};
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

/// Derlenmiş programı başlatır; `silinecek` program bitince silinir.
pub fn baslat(
    program: &Path,
    klasor: &Path,
    argumanlar: &[String],
    ortam: &[(&str, String)],
    silinecek: PathBuf,
) -> Result<u64, String> {
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
    });
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

pub fn durdur(kimlik: u64) {
    if let Some(c) = tablo().lock().unwrap().get(&kimlik).cloned() {
        let _ = c.cocuk.lock().unwrap().kill();
        c.girdi.lock().unwrap().take();
    }
}
