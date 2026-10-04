//! `orhunca` komut aracı: derle, çalıştır, denetle, yeni.

mod agac;
mod ayristirici;
mod denetci;
mod derleme;
mod ekler;
mod hata;
mod sozcuk;
mod studyo;
mod uretici;
mod yerlesik;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const SURUM: &str = env!("CARGO_PKG_VERSION");

const YARDIM: &str = "\
Orhunca — Türkçe tabanlı programlama dili

Kullanım:
  orhunca derle [dosya.ohc] [-o çıktı] [--hedef linux|windows|<üçlü>]
  orhunca çalıştır [dosya.ohc] [-- programın argümanları]
  orhunca denetle [dosya.ohc]
  orhunca yeni <proje_adı>
  orhunca stüdyo [--kapı 7313] [--tarayıcı-açma]
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
        "stüdyo" | "studyo" => studyo::calistir(kalan).map(|_| ExitCode::SUCCESS),
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

fn denetle_komutu(args: &[String]) -> Result<(), String> {
    let s = secenekleri_oku(args)?;
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
