//! Hata ayıklama bağdaştırıcısı (Debug Adapter Protocol): `orhunca ayıkla-dap`.
//!
//! VS Code gibi düzenleyiciler stdin/stdout üzerinden DAP konuşur; bağdaştırıcı programı
//! hata ayıklamalı derler, çalışma zamanının TCP protokolüyle (bkz. runtime/orhunca_rt.c,
//! "Hata ayıklama") konuşur ve olayları DAP'a çevirir. Koşullu kesme ve günlük noktaları
//! derlemede programa yerleştirilir (ayiklama.rs).

use crate::ayiklama::{KosulluKesme, GIZLI_DEGISKEN, KAYDIRMA};
use crate::dil_sunucusu::{gonder, mesaj_oku};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

enum Ileti {
    Dap(Value),
    DapBitti,
    /// Çalışma zamanından bir olay ("son" satırına kadar)
    Olay(Vec<String>),
    Cikti(&'static str, String),
    /// Zaman aşımı: programın bitip bitmediğine bakılır.
    Tik,
}

#[derive(Default)]
struct Durak {
    neden: String,
    ileti: Option<String>,
    /// (işlev, dosya sırası, satır)
    yigin: Vec<(String, usize, usize)>,
    /// çerçeve → (ad, tip, değer)
    degiskenler: HashMap<usize, Vec<(String, String, String)>>,
}

struct Oturum {
    seq: i64,
    kesmeler: HashMap<String, Vec<KosulluKesme>>,
    dosyalar: Vec<PathBuf>,
    soket: Option<TcpStream>,
    cocuk: Option<std::process::Child>,
    ilk: bool,
    girişte_dur: bool,
    durak: Option<Durak>,
    /// Değişkenleri beklenen istekler: (istek, çerçeve)
    bekleyen: Vec<(Value, usize)>,
    istenen_cerceve: Option<usize>,
    gecici: Option<PathBuf>,
}

impl Oturum {
    fn yolla(&mut self, mut m: Value) {
        self.seq += 1;
        m["seq"] = json!(self.seq);
        gonder(&m);
    }

    fn yanit(&mut self, istek: &Value, govde: Value) {
        let m = json!({
            "type": "response", "request_seq": istek["seq"], "success": true,
            "command": istek["command"], "body": govde,
        });
        self.yolla(m);
    }

    fn hata(&mut self, istek: &Value, mesaj: &str) {
        let m = json!({
            "type": "response", "request_seq": istek["seq"], "success": false,
            "command": istek["command"], "message": mesaj,
        });
        self.yolla(m);
    }

    fn olay(&mut self, ad: &str, govde: Value) {
        self.yolla(json!({ "type": "event", "event": ad, "body": govde }));
    }

    fn cikti(&mut self, tur: &str, metin: &str) {
        self.olay("output", json!({ "category": tur, "output": metin }));
    }

    fn komut(&mut self, k: &str) {
        if let Some(s) = self.soket.as_mut() {
            let _ = s.write_all(format!("{k}\n").as_bytes());
        }
    }

    fn sira(&self, yol: &str) -> Option<usize> {
        let t = std::fs::canonicalize(yol).unwrap_or_else(|_| yol.into());
        self.dosyalar.iter().position(|d| *d == t)
    }

    fn kesmeler_satiri(&self) -> String {
        let mut s = String::from("kesmeler");
        for (yol, l) in &self.kesmeler {
            if let Some(i) = self.sira(yol) {
                for k in l {
                    if let Some(satir) = k.calisma_zamani_satiri() {
                        s.push_str(&format!(" {i}:{satir}"));
                    }
                }
            }
        }
        s
    }

    /// Bir çalışma zamanı olayı.
    fn olay_isle(&mut self, satirlar: Vec<String>) {
        let Some(ilk) = satirlar.first() else { return };
        if let Some(c) = ilk.strip_prefix("cdeg ") {
            let cerceve: usize = c.trim().parse().unwrap_or(0);
            let degler = degiskenler(&satirlar[1..]);
            if let Some(d) = self.durak.as_mut() {
                d.degiskenler.insert(cerceve, degler);
            }
            self.bekleyenleri_yanitla();
            return;
        }
        let Some(neden) = ilk.strip_prefix("dur ") else {
            return;
        };
        let mut d = Durak {
            neden: neden.to_string(),
            ..Default::default()
        };
        for s in &satirlar[1..] {
            if let Some(i) = s.strip_prefix("ileti ") {
                d.ileti = Some(i.to_string());
            } else if let Some(c) = s.strip_prefix("cerceve ") {
                let mut p = c.splitn(4, ' ');
                let _ = p.next();
                let dosya = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                let mut satir: usize = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                if satir > KAYDIRMA {
                    satir -= KAYDIRMA;
                }
                d.yigin
                    .push((p.next().unwrap_or("").to_string(), dosya, satir));
            }
        }
        d.degiskenler.insert(0, degiskenler(&satirlar[1..]));
        let ilk_durak = std::mem::take(&mut self.ilk);
        let kesmede = d.neden == "kesme";
        self.istenen_cerceve = None;
        self.durak = Some(d);
        if ilk_durak && !kesmede && !self.girişte_dur {
            self.durak = None;
            self.komut("devam");
            return;
        }
        let (neden, aciklama) = match self.durak.as_ref().unwrap().neden.as_str() {
            "kesme" => ("breakpoint", "kesme noktası"),
            "hata" => ("exception", "çalışma hatası"),
            "duraklat" => ("pause", "duraklatıldı"),
            _ if ilk_durak => ("entry", "başlangıç"),
            _ => ("step", "adım"),
        };
        let ileti = self.durak.as_ref().unwrap().ileti.clone();
        self.olay(
            "stopped",
            json!({ "reason": neden, "description": aciklama, "text": ileti, "threadId": 1, "allThreadsStopped": true }),
        );
    }

    fn bekleyenleri_yanitla(&mut self) {
        let bekleyen = std::mem::take(&mut self.bekleyen);
        for (istek, cerceve) in bekleyen {
            let hazir = self
                .durak
                .as_ref()
                .and_then(|d| d.degiskenler.get(&cerceve))
                .cloned();
            match hazir {
                Some(l) => {
                    let v: Vec<Value> = l
                        .iter()
                        .map(|(ad, tip, deger)| json!({ "name": ad, "value": deger, "type": tip, "variablesReference": 0 }))
                        .collect();
                    self.yanit(&istek, json!({ "variables": v }));
                }
                None => self.bekleyen.push((istek, cerceve)),
            }
        }
    }

    fn baslat(&mut self, ayar: &Value, gonderici: &Sender<Ileti>) -> Result<(), String> {
        let program = ayar["program"].as_str().unwrap_or("").to_string();
        let mut dosya = PathBuf::from(&program);
        if program.is_empty() || dosya.is_dir() {
            let klasor = if program.is_empty() {
                std::env::current_dir().unwrap_or_default()
            } else {
                dosya.clone()
            };
            dosya = crate::derleme::proje_girisi(&klasor)?;
        }
        let gecici = crate::derleme::gecici_klasor("dap")?;
        let cikti = gecici.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        self.gecici = Some(gecici);
        let kesmeler: Vec<KosulluKesme> = self.kesmeler.values().flatten().cloned().collect();
        let dosyalar =
            crate::derleme::derle_ayiklamali(&dosya, &cikti, &kesmeler).map_err(|h| h.metin)?;
        self.dosyalar = dosyalar
            .iter()
            .map(|d| std::fs::canonicalize(d).unwrap_or_else(|_| d.into()))
            .collect();
        let dinleyici = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let kapi = dinleyici.local_addr().map_err(|e| e.to_string())?.port();
        let klasor = ayar["cwd"]
            .as_str()
            .map(PathBuf::from)
            .or_else(|| dosya.parent().map(|p| p.to_path_buf()))
            .filter(|k| !k.as_os_str().is_empty())
            .unwrap_or_else(|| ".".into());
        let argumanlar: Vec<String> = ayar["args"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let mut cocuk = crate::komut(&cikti)
            .args(&argumanlar)
            .current_dir(&klasor)
            .env("ORHUNCA_AYIKLA", kapi.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("program başlatılamadı: {e}"))?;
        for (akis, tur) in [
            (
                Box::new(cocuk.stdout.take().unwrap()) as Box<dyn Read + Send>,
                "stdout",
            ),
            (Box::new(cocuk.stderr.take().unwrap()), "stderr"),
        ] {
            let g = gonderici.clone();
            std::thread::spawn(move || {
                let mut akis = akis;
                let mut tampon = [0u8; 4096];
                while let Ok(n) = akis.read(&mut tampon) {
                    if n == 0 {
                        break;
                    }
                    let _ = g.send(Ileti::Cikti(
                        tur,
                        String::from_utf8_lossy(&tampon[..n]).into_owned(),
                    ));
                }
            });
        }
        self.cocuk = Some(cocuk);
        // Çalışma zamanının bağlanmasını bekle
        dinleyici.set_nonblocking(true).map_err(|e| e.to_string())?;
        let bas = std::time::Instant::now();
        let akis = loop {
            match dinleyici.accept() {
                Ok((s, _)) => break s,
                Err(_) if bas.elapsed() < Duration::from_secs(30) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => return Err("program hata ayıklayıcıya bağlanmadı".into()),
            }
        };
        let _ = akis.set_nonblocking(false);
        let _ = akis.set_nodelay(true);
        let okuyucu = akis.try_clone().map_err(|e| e.to_string())?;
        self.soket = Some(akis);
        let s = self.kesmeler_satiri();
        self.komut(&s);
        let g = gonderici.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(okuyucu);
            let mut satirlar = Vec::new();
            let mut satir = String::new();
            loop {
                satir.clear();
                match r.read_line(&mut satir) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let s = satir.trim_end_matches(['\n', '\r']).to_string();
                if s == "son" {
                    let _ = g.send(Ileti::Olay(std::mem::take(&mut satirlar)));
                } else {
                    satirlar.push(s);
                }
            }
        });
        Ok(())
    }

    fn istek(&mut self, m: Value, gonderici: &Sender<Ileti>) -> bool {
        let komut = m["command"].as_str().unwrap_or("").to_string();
        let a = &m["arguments"];
        match komut.as_str() {
            "initialize" => {
                self.yanit(
                    &m,
                    json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsConditionalBreakpoints": true,
                        "supportsLogPoints": true,
                        "supportsTerminateRequest": true,
                    }),
                );
                self.olay("initialized", json!({}));
            }
            "launch" => {
                self.girişte_dur = a["stopOnEntry"] == true;
                self.istenen_cerceve = None;
                // Asıl başlatma configurationDone'da (kesme noktaları gelince)
                self.yanit(&m, json!({}));
                self.bekleyen.push((m.clone(), usize::MAX));
            }
            "setBreakpoints" => {
                let yol = a["source"]["path"].as_str().unwrap_or("").to_string();
                let l: Vec<KosulluKesme> = a["breakpoints"]
                    .as_array()
                    .map(|l| {
                        l.iter()
                            .map(|b| KosulluKesme {
                                dosya: yol.clone(),
                                satir: b["line"].as_u64().unwrap_or(0) as usize,
                                kosul: b["condition"]
                                    .as_str()
                                    .filter(|c| !c.trim().is_empty())
                                    .map(str::to_string),
                                gunluk: b["logMessage"]
                                    .as_str()
                                    .filter(|c| !c.trim().is_empty())
                                    .map(str::to_string),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let yanit: Vec<Value> = l
                    .iter()
                    .map(|k| json!({ "verified": true, "line": k.satir }))
                    .collect();
                let kosullu_degisti = self.soket.is_some()
                    && l.iter().any(|k| k.kosul.is_some() || k.gunluk.is_some());
                self.kesmeler.insert(yol, l);
                self.yanit(&m, json!({ "breakpoints": yanit }));
                if self.soket.is_some() {
                    let s = self.kesmeler_satiri();
                    self.komut(&s);
                }
                if kosullu_degisti {
                    self.cikti("console", "Koşul ve günlük noktası değişiklikleri hata ayıklama yeniden başlatılınca geçerli olur.\n");
                }
            }
            "configurationDone" => {
                self.yanit(&m, json!({}));
                let ayar = self
                    .bekleyen
                    .iter()
                    .position(|(_, c)| *c == usize::MAX)
                    .map(|i| self.bekleyen.remove(i).0["arguments"].clone())
                    .unwrap_or_default();
                if let Err(e) = self.baslat(&ayar, gonderici) {
                    self.cikti("stderr", &format!("{e}\n"));
                    self.olay("terminated", json!({}));
                }
            }
            "threads" => self.yanit(&m, json!({ "threads": [{ "id": 1, "name": "ana" }] })),
            "stackTrace" => {
                let cerceveler: Vec<Value> = self
                    .durak
                    .as_ref()
                    .map(|d| {
                        d.yigin
                            .iter()
                            .enumerate()
                            .map(|(i, (ad, dosya, satir))| {
                                let yol = self.dosyalar.get(*dosya).map(|p| p.display().to_string()).unwrap_or_default();
                                let yol = yol.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(yol);
                                json!({ "id": i, "name": ad, "line": satir, "column": 1,
                                    "source": { "name": std::path::Path::new(&yol).file_name().map(|a| a.to_string_lossy().into_owned()), "path": yol } })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let n = cerceveler.len();
                self.yanit(&m, json!({ "stackFrames": cerceveler, "totalFrames": n }));
            }
            "scopes" => {
                let cerceve = a["frameId"].as_u64().unwrap_or(0) as usize;
                self.yanit(&m, json!({ "scopes": [{ "name": "Değişkenler", "variablesReference": cerceve + 1, "expensive": false }] }));
            }
            "variables" => {
                let cerceve = a["variablesReference"]
                    .as_u64()
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                let var = self
                    .durak
                    .as_ref()
                    .is_some_and(|d| d.degiskenler.contains_key(&cerceve));
                self.bekleyen.push((m.clone(), cerceve));
                if !var && self.istenen_cerceve != Some(cerceve) {
                    self.istenen_cerceve = Some(cerceve);
                    self.komut(&format!("cerceve {cerceve}"));
                }
                self.bekleyenleri_yanitla();
            }
            "continue" | "next" | "stepIn" | "stepOut" => {
                let k = match komut.as_str() {
                    "continue" => "devam",
                    "next" => "ustunden",
                    "stepIn" => "adim",
                    _ => "cik",
                };
                self.durak = None;
                self.bekleyen.retain(|(_, c)| *c == usize::MAX);
                self.komut(k);
                self.yanit(&m, json!({ "allThreadsContinued": true }));
            }
            "pause" => {
                self.komut("duraklat");
                self.yanit(&m, json!({}));
            }
            "evaluate" => {
                // İfade değerlendirme yok: değişken adı ise değeri verilir.
                let ifade = a["expression"].as_str().unwrap_or("").trim().to_string();
                let cerceve = a["frameId"].as_u64().unwrap_or(0) as usize;
                let deger = self
                    .durak
                    .as_ref()
                    .and_then(|d| d.degiskenler.get(&cerceve))
                    .and_then(|l| l.iter().find(|(ad, _, _)| *ad == ifade))
                    .map(|(_, _, d)| d.clone());
                match deger {
                    Some(d) => self.yanit(&m, json!({ "result": d, "variablesReference": 0 })),
                    None => self.hata(&m, "yalnızca bu çerçevedeki değişken adları gösterilebilir"),
                }
            }
            "disconnect" | "terminate" => {
                if let Some(c) = self.cocuk.as_mut() {
                    let _ = c.kill();
                }
                self.yanit(&m, json!({}));
                if komut == "disconnect" {
                    return false;
                }
            }
            _ => self.hata(&m, &format!("desteklenmeyen istek: {komut}")),
        }
        true
    }
}

fn degiskenler(satirlar: &[String]) -> Vec<(String, String, String)> {
    satirlar
        .iter()
        .filter_map(|s| s.strip_prefix("deg "))
        .filter_map(|v| {
            let mut p = v.splitn(3, '\t');
            let ad = p.next()?.to_string();
            (ad != GIZLI_DEGISKEN).then(|| {
                (
                    ad,
                    p.next().unwrap_or("").to_string(),
                    p.next().unwrap_or("").to_string(),
                )
            })
        })
        .collect()
}

pub fn calistir() -> Result<(), String> {
    let (gonderici, alici) = channel();
    let g = gonderici.clone();
    std::thread::spawn(move || {
        let mut okuyucu = BufReader::new(std::io::stdin().lock());
        while let Some(m) = mesaj_oku(&mut okuyucu) {
            if g.send(Ileti::Dap(m)).is_err() {
                return;
            }
        }
        let _ = g.send(Ileti::DapBitti);
    });
    let mut o = Oturum {
        seq: 0,
        kesmeler: HashMap::new(),
        dosyalar: vec![],
        soket: None,
        cocuk: None,
        ilk: true,
        girişte_dur: false,
        durak: None,
        bekleyen: vec![],
        istenen_cerceve: None,
        gecici: None,
    };
    let mut bitti_bekleniyor = false;
    while let Ok(i) = alici
        .recv_timeout(Duration::from_millis(200))
        .or_else(|e| match e {
            std::sync::mpsc::RecvTimeoutError::Timeout => Ok(Ileti::Tik),
            std::sync::mpsc::RecvTimeoutError::Disconnected => Err(()),
        })
    {
        match i {
            Ileti::Dap(m) => {
                if !o.istek(m, &gonderici) {
                    break;
                }
            }
            Ileti::DapBitti => break,
            Ileti::Olay(s) => o.olay_isle(s),
            Ileti::Cikti(tur, m) => o.cikti(tur, &m),
            Ileti::Tik => {
                // Program bitti mi? (zaman aşımında bakılır)
                if bitti_bekleniyor {
                    continue;
                }
                let kod = o
                    .cocuk
                    .as_mut()
                    .and_then(|c| c.try_wait().ok().flatten())
                    .map(|d| d.code().unwrap_or(-1));
                if let Some(kod) = kod {
                    // Kalan çıktılar için kısa bir bekleme
                    std::thread::sleep(Duration::from_millis(100));
                    while let Ok(Ileti::Cikti(tur, m)) = alici.try_recv() {
                        o.cikti(tur, &m);
                    }
                    o.olay("exited", json!({ "exitCode": kod }));
                    o.olay("terminated", json!({}));
                    bitti_bekleniyor = true;
                }
            }
        }
    }
    if let Some(c) = o.cocuk.as_mut() {
        let _ = c.kill();
    }
    if let Some(g) = &o.gecici {
        let _ = std::fs::remove_dir_all(g);
    }
    Ok(())
}
