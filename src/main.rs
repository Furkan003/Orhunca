//! `orhunca` komut aracı: derle, çalıştır, denetle, yeni.

use orhunca::{bicimlendirici, derleme, dil_sunucusu, etkilesim, paket, studyo, SURUM};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const YARDIM: &str = "\
Orhunca — Türkçe tabanlı programlama dili

Kullanım:
  orhunca derle [dosya.ohc] [-o çıktı] [--hedef linux|windows|web|<üçlü>]
  orhunca çalıştır [dosya.ohc] [--hedef web] [-- programın argümanları]
  orhunca denetle [dosya.ohc]
  orhunca etkileşim           (satır satır deneme: yazdığınız her satır hemen çalışır)
  orhunca paketle [dosya.ohc] [-o çıktı] [--hedef linux|windows]
  orhunca biçimlendir [dosya.ohc ...] [--denetle]
  orhunca dil-sunucusu        (düzenleyiciler için LSP, stdin/stdout)
  orhunca yeni <proje_adı> [--şablon konsol|web_sitesi|tam_yigin|web_api|...]
  orhunca paket ekle <git-adresi>[#etiket] | yükle | güncelle | kaldır <ad> | listele
  orhunca stüdyo [--kapı 7313] [--tarayıcı-açma]
  orhunca sürüm

Dosya verilmezse geçerli klasördeki .ohcproj dosyasının giriş dosyası kullanılır.
C derleyicisi gerekmez (Linux ve Windows); sistemin C derleyicisiyle bağlamak için ORHUNCA_CC=cc.
paketle: arayüz programını kendi penceresinde açılan masaüstü uygulamasına
dönüştürür (Windows: WebView2, Linux: WebKitGTK).
--hedef web: WebAssembly; tarayıcıda açılan tek bir .html dosyası (-o x.wasm: ayrı
dosyalar). 'çalıştır --hedef web' programı Node.js ile çalıştırır.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(komut) = args.first() else {
        println!("{YARDIM}");
        return ExitCode::SUCCESS;
    };
    let kalan = &args[1..];
    let sonuc = match komut.as_str() {
        "derle" => derle_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "çalıştır" | "calistir" => calistir_komutu(kalan),
        "denetle" => denetle_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "paketle" => paketle_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "etkileşim" | "etkilesim" | "repl" => etkilesim::calistir().map(|_| ExitCode::SUCCESS),
        "yeni" => yeni_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "stüdyo" | "studyo" => studyo::calistir(kalan).map(|_| ExitCode::SUCCESS),
        "biçimlendir" | "bicimlendir" => bicimlendir_komutu(kalan),
        "dil-sunucusu" | "lsp" => dil_sunucusu::calistir().map(|_| ExitCode::SUCCESS),
        "paket" => paket::komut(kalan).map(|_| ExitCode::SUCCESS),
        "sürüm" | "surum" | "--version" | "-V" => {
            println!("orhunca {SURUM}");
            Ok(ExitCode::SUCCESS)
        }
        "yardım" | "yardim" | "--help" | "-h" => {
            println!("{YARDIM}");
            Ok(ExitCode::SUCCESS)
        }
        k => Err(format!("bilinmeyen komut '{k}'\n\n{YARDIM}")),
    };
    match sonuc {
        Ok(kod) => kod,
        Err(m) => {
            eprintln!("{m}");
            ExitCode::FAILURE
        }
    }
}

struct Secenekler {
    dosya: PathBuf,
    cikti: Option<PathBuf>,
    hedef: Option<String>,
    /// `--` sonrasındaki değerler: `çalıştır` bunları programa geçirir.
    program_argumanlari: Vec<String>,
}

fn secenekleri_oku(args: &[String]) -> Result<Secenekler, String> {
    let mut dosya = None;
    let mut cikti = None;
    let mut hedef = None;
    let mut program_argumanlari = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--" => {
                program_argumanlari = args[i + 1..].to_vec();
                break;
            }
            "-o" => {
                i += 1;
                cikti = Some(PathBuf::from(
                    args.get(i).ok_or("-o sonrasında çıktı adı bekleniyordu")?,
                ));
            }
            "--hedef" => {
                i += 1;
                hedef = Some(
                    args.get(i)
                        .ok_or("--hedef sonrasında hedef adı bekleniyordu")?
                        .clone(),
                );
            }
            a if a.starts_with('-') => return Err(format!("bilinmeyen seçenek '{a}'")),
            a => dosya = Some(PathBuf::from(a)),
        }
        i += 1;
    }
    let dosya = match dosya {
        Some(d) => d,
        None => proje_girisi()?,
    };
    Ok(Secenekler {
        program_argumanlari,
        dosya,
        cikti,
        hedef,
    })
}

fn proje_girisi() -> Result<PathBuf, String> {
    let giris = derleme::proje_girisi(Path::new("."))?;
    Ok(giris
        .strip_prefix(".")
        .map(Path::to_path_buf)
        .unwrap_or(giris))
}

/// Biçimlendiricinin uyarıları (ek uyumu, "5" + 3 gibi) hata çıkışına yazılır.
fn uyarilari_yaz(dosya: &Path) {
    let Ok(kaynak) = std::fs::read_to_string(dosya) else {
        return;
    };
    for u in bicimlendirici::uyarilar(&kaynak) {
        eprintln!(
            "uyarı: {}:{}:{}: {}",
            dosya.display(),
            u.konum.satir,
            u.konum.sutun,
            u.mesaj
        );
    }
}

fn denetle_komutu(args: &[String]) -> Result<(), String> {
    let s = secenekleri_oku(args)?;
    uyarilari_yaz(&s.dosya);
    derleme::yukle(&s.dosya).map_err(|h| h.metin)?;
    println!("{}: hata yok", s.dosya.display());
    Ok(())
}

fn derle(s: &Secenekler) -> Result<PathBuf, String> {
    let cikti = s
        .cikti
        .clone()
        .unwrap_or_else(|| derleme::varsayilan_cikti(&s.dosya, s.hedef.as_deref()));
    derleme::derle(&s.dosya, &cikti, s.hedef.as_deref()).map_err(|h| h.metin)?;
    Ok(cikti)
}

fn derle_komutu(args: &[String]) -> Result<(), String> {
    // --ayıklama: Stüdyo hata ayıklayıcısı için derler (ORHUNCA_AYIKLA=<kapı>).
    let ayiklama = args.iter().any(|a| a == "--ayıklama" || a == "--ayiklama");
    let args: Vec<String> = args
        .iter()
        .filter(|a| !matches!(a.as_str(), "--ayıklama" | "--ayiklama"))
        .cloned()
        .collect();
    let s = secenekleri_oku(&args)?;
    if ayiklama {
        let cikti = s
            .cikti
            .clone()
            .unwrap_or_else(|| derleme::varsayilan_cikti(&s.dosya, None));
        derleme::derle_ayiklamali(&s.dosya, &cikti).map_err(|h| h.metin)?;
        println!("derlendi: {}", cikti.display());
        return Ok(());
    }
    let cikti = derle(&s)?;
    println!("derlendi: {}", cikti.display());
    Ok(())
}

fn paketle_komutu(args: &[String]) -> Result<(), String> {
    let s = secenekleri_oku(args)?;
    if derleme::web_hedefi_mi(s.hedef.as_deref()) {
        return Err("paketle masaüstü içindir; web için: orhunca derle --hedef web".into());
    }
    let cikti = s
        .cikti
        .clone()
        .unwrap_or_else(|| derleme::varsayilan_cikti(&s.dosya, s.hedef.as_deref()));
    derleme::paketle(&s.dosya, &cikti, s.hedef.as_deref()).map_err(|h| h.metin)?;
    println!("paketlendi: {}", cikti.display());
    Ok(())
}

fn calistir_komutu(args: &[String]) -> Result<ExitCode, String> {
    let mut s = secenekleri_oku(args)?;
    uyarilari_yaz(&s.dosya);
    if derleme::web_hedefi_mi(s.hedef.as_deref()) {
        return web_calistir(&s);
    }
    if s.hedef.is_none() && derleme::arayuz_programi_mi(&s.dosya) {
        return arayuz_ac(&s);
    }
    if s.hedef.is_some() {
        return Err(
            "'çalıştır' yalnızca bu bilgisayar için derler; --hedef ile 'derle' kullanın".into(),
        );
    }
    let gecici = derleme::gecici_klasor("calistir")?;
    s.cikti = Some(gecici.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    }));
    let yol = derle(&s)?;
    let durum = Command::new(&yol)
        .args(&s.program_argumanlari)
        .status()
        .map_err(|e| format!("program çalıştırılamadı: {e}"))?;
    let _ = std::fs::remove_dir_all(&gecici);
    Ok(ExitCode::from(durum.code().unwrap_or(1).clamp(0, 255) as u8))
}

/// Arayüz programı: tek dosyalık sayfaya derlenir ve tarayıcıda açılır.
/// `ORHUNCA_TARAYICI=0` ise yalnızca sayfanın yolu yazılır.
fn arayuz_ac(s: &Secenekler) -> Result<ExitCode, String> {
    let klasor = std::env::temp_dir().join("orhunca-arayuz");
    std::fs::create_dir_all(&klasor).map_err(|e| e.to_string())?;
    let ad = s
        .dosya
        .file_stem()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_else(|| "uygulama".into());
    let sayfa = klasor.join(format!("{ad}.html"));
    derleme::derle_web(&s.dosya, &sayfa).map_err(|h| h.metin)?;
    println!("Arayüz programı derlendi: {}", sayfa.display());
    if std::env::var("ORHUNCA_TARAYICI").as_deref() != Ok("0") {
        // xdg-open, open ve start dosya yolunu varsayılan tarayıcıyla açar.
        studyo::tarayicida_ac(&sayfa.display().to_string());
        println!("Tarayıcıda açıldı.");
    }
    Ok(ExitCode::SUCCESS)
}

/// WebAssembly'ye derler ve Node.js ile çalıştırır (tarayıcıdaki ile aynı kod).
fn web_calistir(s: &Secenekler) -> Result<ExitCode, String> {
    let wasm = derleme::wasm_uret(&s.dosya).map_err(|h| h.metin)?;
    let gecici = derleme::gecici_klasor("web")?;
    let yaz = |ad: &str, veri: &[u8]| {
        std::fs::write(gecici.join(ad), veri).map_err(|e| format!("'{ad}' yazılamadı: {e}"))
    };
    yaz("program.wasm", &wasm)?;
    yaz("orhunca_rt.wasm", derleme::WASM_CALISMA_ZAMANI)?;
    yaz("orhunca.js", derleme::WASM_YUKLEYICI.as_bytes())?;
    let node = std::env::var("ORHUNCA_NODE").unwrap_or_else(|_| "node".into());
    let durum = Command::new(&node)
        .arg(gecici.join("orhunca.js"))
        .arg(gecici.join("program.wasm"))
        .args(&s.program_argumanlari)
        .status();
    let _ = std::fs::remove_dir_all(&gecici);
    let durum = durum.map_err(|e| {
        format!(
            "Node.js ('{node}') çalıştırılamadı: {e}\nipucu: Node.js kurun ya da tarayıcıda açılacak bir sayfa üretin: orhunca derle {} --hedef web",
            s.dosya.display()
        )
    })?;
    Ok(ExitCode::from(durum.code().unwrap_or(1).clamp(0, 255) as u8))
}

/// Bir klasördeki tüm .ohc dosyaları (gizli klasörler ve derleme çıktıları hariç).
fn ohc_dosyalari(klasor: &Path, cikti: &mut Vec<PathBuf>) {
    let Ok(okunan) = std::fs::read_dir(klasor) else {
        return;
    };
    let mut girdiler: Vec<_> = okunan.filter_map(|g| g.ok().map(|g| g.path())).collect();
    girdiler.sort();
    for p in girdiler {
        let ad = p
            .file_name()
            .map(|a| a.to_string_lossy().into_owned())
            .unwrap_or_default();
        if ad.starts_with('.') || ad == "target" || ad == "cikti" {
            continue;
        }
        if p.is_dir() {
            ohc_dosyalari(&p, cikti);
        } else if p.extension().is_some_and(|u| u == "ohc") {
            cikti.push(p);
        }
    }
}

/// `--denetle`: dosyaları değiştirmez; biçimsiz dosya varsa hata koduyla çıkar.
fn bicimlendir_komutu(args: &[String]) -> Result<ExitCode, String> {
    let denetle = args.iter().any(|a| a == "--denetle");
    let mut dosyalar: Vec<PathBuf> = Vec::new();
    for a in args.iter().filter(|a| !a.starts_with("--")) {
        let p = PathBuf::from(a);
        if p.is_dir() {
            ohc_dosyalari(&p, &mut dosyalar);
        } else {
            dosyalar.push(p);
        }
    }
    if dosyalar.is_empty() {
        ohc_dosyalari(Path::new("."), &mut dosyalar);
    }
    let mut degisen = 0;
    for d in &dosyalar {
        let kaynak =
            std::fs::read_to_string(d).map_err(|e| format!("'{}' okunamadı: {e}", d.display()))?;
        let yeni = bicimlendirici::bicimlendir(&kaynak);
        for u in bicimlendirici::uyarilar(&kaynak) {
            println!(
                "{}:{}:{}: {}",
                d.display(),
                u.konum.satir,
                u.konum.sutun,
                u.mesaj
            );
        }
        if yeni != kaynak {
            degisen += 1;
            if denetle {
                println!("{}: biçimlendirilmeli", d.display());
            } else {
                std::fs::write(d, yeni)
                    .map_err(|e| format!("'{}' yazılamadı: {e}", d.display()))?;
                println!("{}: biçimlendirildi", d.display());
            }
        }
    }
    if denetle && degisen > 0 {
        return Ok(ExitCode::FAILURE);
    }
    if degisen == 0 {
        println!("{} dosya zaten düzgün.", dosyalar.len());
    }
    Ok(ExitCode::SUCCESS)
}

fn yeni_komutu(args: &[String]) -> Result<(), String> {
    use studyo::sablonlar;
    let mut ad = None;
    let mut kimlik = "konsol".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--şablon" | "--sablon" => {
                i += 1;
                kimlik = args
                    .get(i)
                    .cloned()
                    .ok_or("--şablon sonrasında şablon adı bekleniyordu")?;
            }
            a if ad.is_none() => ad = Some(a.to_string()),
            a => return Err(format!("beklenmeyen değer '{a}'")),
        }
        i += 1;
    }
    let ad = ad.ok_or("proje adı bekleniyordu: orhunca yeni <ad> [--şablon web_sitesi]")?;
    let hazir: Vec<&str> = sablonlar::SABLONLAR
        .iter()
        .filter(|s| s.yakinda.is_none())
        .map(|s| s.kimlik)
        .collect();
    let sablon = sablonlar::bul(&kimlik)
        .filter(|s| s.yakinda.is_none())
        .ok_or_else(|| {
            format!(
                "bilinmeyen şablon '{kimlik}'\nşablonlar: {}",
                hazir.join(", ")
            )
        })?;
    let klasor = PathBuf::from(&ad);
    if klasor.exists() {
        return Err(format!("'{ad}' zaten var"));
    }
    // Varsayılan konsol projesi sade; diğer şablonlar örnek içerikle gelir.
    let ornek = kimlik != "konsol";
    for dosya in sablon.dosyalar {
        let yol = klasor.join(dosya.replace("{ad}", &ad));
        if let Some(u) = yol.parent() {
            std::fs::create_dir_all(u).map_err(|e| e.to_string())?;
        }
        std::fs::write(&yol, sablonlar::icerik(sablon, dosya, &ad, ornek))
            .map_err(|e| e.to_string())?;
    }
    println!(
        "'{ad}' projesi oluşturuldu ({}).\n  cd {ad}\n  orhunca çalıştır",
        sablon.ad
    );
    if sablonlar::arayuz_mu(sablon) {
        println!("Uygulama tarayıcıda açılır (WebAssembly).");
    } else if sablonlar::web_mi(sablon) {
        println!("Sonra tarayıcıda http://localhost:3000 adresini açın.");
    }
    Ok(())
}
