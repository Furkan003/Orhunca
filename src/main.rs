//! `orhunca` komut aracı: derle, çalıştır, denetle, yeni.

mod agac;
mod ayristirici;
mod denetci;
mod ekler;
mod hata;
mod sozcuk;
mod uretici;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::str::FromStr;
use target_lexicon::Triple;

const CALISMA_ZAMANI_KAYNAGI: &str = include_str!("../runtime/orhunca_rt.c");
const SURUM: &str = env!("CARGO_PKG_VERSION");

const YARDIM: &str = "\
Orhunca — Türkçe tabanlı programlama dili

Kullanım:
  orhunca derle [dosya.ohc] [-o çıktı] [--hedef linux|windows|<üçlü>]
  orhunca çalıştır [dosya.ohc]
  orhunca denetle [dosya.ohc]
  orhunca yeni <proje_adı>
  orhunca sürüm

Dosya verilmezse geçerli klasördeki .ohcproj dosyasının giriş dosyası kullanılır.
Bağlayıcıyı değiştirmek için ORHUNCA_CC ortam değişkenini kullanın.";

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
        "yeni" => yeni_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "paket" => Err("paket yöneticisi henüz hazır değil (yol haritası: Aşama 5)".into()),
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
}

fn secenekleri_oku(args: &[String]) -> Result<Secenekler, String> {
    let mut dosya = None;
    let mut cikti = None;
    let mut hedef = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
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
        dosya,
        cikti,
        hedef,
    })
}

/// Geçerli klasördeki `.ohcproj` dosyasından giriş dosyasını bulur.
fn proje_girisi() -> Result<PathBuf, String> {
    let proje = std::fs::read_dir(".")
        .map_err(|e| e.to_string())?
        .filter_map(|g| g.ok().map(|g| g.path()))
        .find(|p| p.extension().is_some_and(|u| u == "ohcproj"))
        .ok_or("derlenecek dosya verilmedi ve bu klasörde .ohcproj dosyası yok")?;
    let icerik = std::fs::read_to_string(&proje).map_err(|e| e.to_string())?;
    for satir in icerik.lines() {
        if let Some((anahtar, deger)) = satir.split_once('=') {
            if matches!(anahtar.trim(), "giriş" | "giris") {
                return Ok(PathBuf::from(deger.trim().trim_matches('"')));
            }
        }
    }
    Ok(PathBuf::from("ana.ohc"))
}

/// Kaynağı okur, ayrıştırır ve denetler.
fn on_derle(dosya: &Path) -> Result<agac::Program, String> {
    let kaynak = std::fs::read_to_string(dosya)
        .map_err(|e| format!("'{}' okunamadı: {e}", dosya.display()))?;
    let ad = dosya.display().to_string();
    let goster = |h: hata::Hata| h.goster(&ad, &kaynak);
    let sozcukler = sozcuk::sozcukle(&kaynak).map_err(goster)?;
    let mut program = ayristirici::ayristir(sozcukler).map_err(goster)?;
    denetci::denetle(&mut program).map_err(goster)?;
    Ok(program)
}

fn denetle_komutu(args: &[String]) -> Result<(), String> {
    let s = secenekleri_oku(args)?;
    on_derle(&s.dosya)?;
    println!("{}: hata yok", s.dosya.display());
    Ok(())
}

fn hedef_uclusu(hedef: Option<&str>) -> Result<Triple, String> {
    match hedef {
        None => Ok(Triple::host()),
        Some("linux") => Ok(Triple::from_str("x86_64-unknown-linux-gnu").unwrap()),
        Some("windows") => Ok(Triple::from_str("x86_64-pc-windows-gnu").unwrap()),
        Some(t) => Triple::from_str(t).map_err(|e| format!("geçersiz hedef '{t}': {e}")),
    }
}

fn derle(s: &Secenekler) -> Result<PathBuf, String> {
    let program = on_derle(&s.dosya)?;
    let triple = hedef_uclusu(s.hedef.as_deref())?;
    let windows = triple.operating_system == target_lexicon::OperatingSystem::Windows;
    let isa = uretici::isa_kur(triple)?;
    let nesne = uretici::uret(&program, isa).map_err(|e| format!("kod üretimi başarısız: {e}"))?;

    let cikti = match &s.cikti {
        Some(c) => c.clone(),
        None => {
            let kok = s
                .dosya
                .file_stem()
                .map(|k| k.to_os_string())
                .unwrap_or_else(|| "program".into());
            let mut c = PathBuf::from(kok);
            if windows {
                c.set_extension("exe");
            }
            c
        }
    };

    let gecici = std::env::temp_dir().join(format!("orhunca-{}", std::process::id()));
    std::fs::create_dir_all(&gecici).map_err(|e| e.to_string())?;
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
    let durum = Command::new(&baglayici)
        .arg("-O2")
        .arg("-o")
        .arg(&cikti)
        .arg(&nesne_yolu)
        .arg(&cz_yolu)
        .status()
        .map_err(|e| format!("bağlayıcı '{baglayici}' çalıştırılamadı: {e}\nipucu: bir C derleyicisi kurun ya da ORHUNCA_CC ile belirtin"))?;
    let _ = std::fs::remove_dir_all(&gecici);
    if !durum.success() {
        return Err(format!("bağlama başarısız ({baglayici})"));
    }
    Ok(cikti)
}

fn derle_komutu(args: &[String]) -> Result<(), String> {
    let s = secenekleri_oku(args)?;
    let cikti = derle(&s)?;
    println!("derlendi: {}", cikti.display());
    Ok(())
}

fn calistir_komutu(args: &[String]) -> Result<ExitCode, String> {
    let mut s = secenekleri_oku(args)?;
    if s.hedef.is_some() {
        return Err(
            "'çalıştır' yalnızca bu bilgisayar için derler; --hedef ile 'derle' kullanın".into(),
        );
    }
    let gecici = std::env::temp_dir().join(format!("orhunca-calistir-{}", std::process::id()));
    std::fs::create_dir_all(&gecici).map_err(|e| e.to_string())?;
    s.cikti = Some(gecici.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    }));
    let yol = derle(&s)?;
    let durum = Command::new(&yol)
        .status()
        .map_err(|e| format!("program çalıştırılamadı: {e}"))?;
    let _ = std::fs::remove_dir_all(&gecici);
    Ok(ExitCode::from(durum.code().unwrap_or(1).clamp(0, 255) as u8))
}

fn yeni_komutu(args: &[String]) -> Result<(), String> {
    let ad = args
        .first()
        .ok_or("proje adı bekleniyordu: orhunca yeni <ad>")?;
    let klasor = PathBuf::from(ad);
    if klasor.exists() {
        return Err(format!("'{ad}' zaten var"));
    }
    std::fs::create_dir_all(&klasor).map_err(|e| e.to_string())?;
    std::fs::write(
        klasor.join(format!("{ad}.ohcproj")),
        format!("ad = \"{ad}\"\nsürüm = \"0.1.0\"\ngiriş = \"ana.ohc\"\n"),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        klasor.join("ana.ohc"),
        "\"Merhaba, dünya!\"'yı ekrana yaz.\n",
    )
    .map_err(|e| e.to_string())?;
    println!("'{ad}' projesi oluşturuldu.\n  cd {ad}\n  orhunca çalıştır");
    Ok(())
}
