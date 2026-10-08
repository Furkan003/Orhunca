//! `orhunca` komut aracı: derle, çalıştır, denetle, yeni.

use orhunca::{bicimlendirici, derleme, dil_sunucusu, etkilesim, paket, studyo, SURUM};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const YARDIM: &str = "\
Orhunca — Türkçe tabanlı programlama dili

Kullanım:
  orhunca derle [dosya.ohc] [-o çıktı] [--hedef linux|windows|web|<üçlü>]
  orhunca çalıştır [dosya.ohc] [--hedef web] [-- programın argümanları]
  orhunca denetle [dosya.ohc] [--json]
  orhunca etkileşim           (satır satır deneme: yazdığınız her satır hemen çalışır)
  orhunca paketle [dosya.ohc] [-o çıktı] [--hedef linux|windows|macos|android|ios]
  orhunca biçimlendir [dosya.ohc ...] [--denetle]
  orhunca düzelt [dosya.ohc ...] [--denetle]   (eskiyen yazımları yenisine çevirir)
  orhunca dil-sunucusu        (düzenleyiciler için LSP, stdin/stdout)
  orhunca çevir [dosya.ohc] [--dil python|javascript]   (programın Python/JS karşılığı)
  orhunca sına [dosya ya da klasör] [--ad parça] [--json]   (sınamaları çalıştırır: *_sına.ohc)
  orhunca başvuru [arama] [--md | --json]   (yerleşik işlevler: ör. orhunca başvuru tarih)
  orhunca mcp                 (yapay zekâ ajanları için MCP sunucusu, stdin/stdout)
  orhunca ayıkla-dap          (düzenleyiciler için hata ayıklama bağdaştırıcısı, DAP)
  orhunca yeni <proje_adı> [--şablon konsol|web_sitesi|tam_yigin|web_api|...]
  orhunca paket ara [kelime] | ekle <ad | git-adresi>[#etiket] | yükle | güncelle | kaldır <ad> | listele
  orhunca paket bilgi <ad | git-adresi> [--json] | yayımla   (izinler, içerik özeti; paket mağazasına yayımlama)
  orhunca yayınla [kullanıcı@sunucu] [--alan ornek.com] [--kapı 3000] [--klasör /srv/ad] [--ssh-kapı 22]
  orhunca yayınla --cgi [--kaynakla]   (paylaşımlı hosting: cikti/cgi/ klasörü public_html'e yüklenir)
  orhunca stüdyo [--kapı 7313] [--tarayıcı-açma]
  orhunca güncelle [--denetle]
  orhunca sürüm

Dosya verilmezse geçerli klasördeki .ohcproj dosyasının giriş dosyası kullanılır.
C derleyicisi gerekmez (Linux ve Windows); sistemin C derleyicisiyle bağlamak için ORHUNCA_CC=cc.
paketle: arayüz programını kendi penceresinde açılan masaüstü uygulamasına
dönüştürür (Windows: WebView2, Linux: WebKitGTK); --hedef android: telefona
kurulabilen imzalı .apk (Android SDK gerekmez); --hedef ios: iPhone/iPad için Xcode
projesi (Mac'te Xcode ile ya da GitHub'da derlenir).
yayınla: web programını Linux sunucusuna (VPS) kurar; sunucuda Orhunca gerekmez.
Sunucu verilmezse yalnızca cikti/yayın/ klasörü hazırlanır.
--hedef web: WebAssembly; tarayıcıda açılan tek bir .html dosyası (-o x.wasm: ayrı
dosyalar). 'çalıştır --hedef web' programı Node.js ile çalıştırır.";

fn main() -> ExitCode {
    // Derleyici geniş yığınlı bir iş parçacığında çalışır (bkz. orhunca::YIGIN).
    std::thread::Builder::new()
        .stack_size(orhunca::YIGIN)
        .spawn(ana)
        .expect("iş parçacığı başlatılamadı")
        .join()
        .unwrap_or(ExitCode::FAILURE)
}

fn ana() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Paylaşımlı hostingde orhunca.cgi olarak: .ohc dosyalarını derleyip çalıştırır.
    if orhunca::cgi::isleyici_mi(&args) {
        return orhunca::cgi::isleyici();
    }
    let Some(komut) = args.first() else {
        println!("{YARDIM}");
        return ExitCode::SUCCESS;
    };
    let kalan = &args[1..];
    // `orhunca <komut> --help`: hiçbir şey yapmadan o komutun yardımı gösterilir.
    // `--` sonrası çalıştırılan programın argümanlarıdır, ona bakılmaz.
    if kalan
        .iter()
        .take_while(|a| *a != "--")
        .any(|a| matches!(a.as_str(), "--help" | "-h" | "--yardım" | "--yardim"))
    {
        println!("{}", komut_yardimi(komut));
        return ExitCode::SUCCESS;
    }
    let sonuc = match komut.as_str() {
        "derle" => derle_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "çalıştır" | "calistir" => calistir_komutu(kalan),
        "denetle" => denetle_komutu(kalan),
        "paketle" => paketle_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "etkileşim" | "etkilesim" | "repl" => etkilesim::calistir().map(|_| ExitCode::SUCCESS),
        "yeni" => yeni_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "stüdyo" | "studyo" => studyo::calistir(kalan).map(|_| ExitCode::SUCCESS),
        "biçimlendir" | "bicimlendir" => bicimlendir_komutu(kalan),
        "düzelt" | "duzelt" => duzelt_komutu(kalan),
        "dil-sunucusu" | "lsp" => dil_sunucusu::calistir().map(|_| ExitCode::SUCCESS),
        "mcp" => orhunca::mcp::calistir().map(|_| ExitCode::SUCCESS),
        "ayıkla-dap" | "ayikla-dap" | "dap" => orhunca::dap::calistir().map(|_| ExitCode::SUCCESS),
        "çevir" | "cevir" => cevir_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "sına" | "sina" | "test" => orhunca::sinama::komut(kalan).map(|tamam| {
            if tamam {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }),
        "başvuru" | "basvuru" => orhunca::basvuru::komut(kalan).map(|_| ExitCode::SUCCESS),
        "paket" => paket::komut(kalan).map(|_| ExitCode::SUCCESS),
        "yayınla" | "yayinla" => yayinla_komutu(kalan).map(|_| ExitCode::SUCCESS),
        "güncelle" | "guncelle" | "update" => {
            orhunca::guncelleme::komut(kalan).map(|_| ExitCode::SUCCESS)
        }
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

/// Bir komutun yardımı: genel yardımdaki o komuta ait satırlar.
fn komut_yardimi(komut: &str) -> String {
    let ascii = |m: &str| {
        m.chars()
            .map(|c| match c {
                'ç' => 'c',
                'ğ' => 'g',
                'ı' => 'i',
                'ö' => 'o',
                'ş' => 's',
                'ü' => 'u',
                c => c,
            })
            .collect::<String>()
    };
    let aranan = ascii(komut);
    let satirlar: Vec<&str> = YARDIM
        .lines()
        .filter(|s| {
            s.trim_start()
                .strip_prefix("orhunca ")
                .and_then(|k| k.split_whitespace().next())
                .is_some_and(|k| ascii(k) == aranan)
        })
        .collect();
    if satirlar.is_empty() {
        return YARDIM.to_string();
    }
    format!(
        "Kullanım:\n{}\n\nBütün komutlar için: orhunca yardım",
        satirlar.join("\n")
    )
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

fn denetle_komutu(args: &[String]) -> Result<ExitCode, String> {
    // --json: araçlar ve yapay zekâ ajanları için hatalar ve uyarılar JSON olarak yazılır.
    let json = args.iter().any(|a| a == "--json");
    let args: Vec<String> = args.iter().filter(|a| *a != "--json").cloned().collect();
    let s = secenekleri_oku(&args)?;
    if !json {
        uyarilari_yaz(&s.dosya);
        derleme::yukle(&s.dosya).map_err(|h| h.metin)?;
        println!("{}: hata yok", s.dosya.display());
        return Ok(ExitCode::SUCCESS);
    }
    let dosya = s.dosya.display().to_string();
    let uyarilar: Vec<_> = std::fs::read_to_string(&s.dosya)
        .map(|k| bicimlendirici::uyarilar(&k))
        .unwrap_or_default()
        .into_iter()
        .map(|u| {
            serde_json::json!({
                "dosya": dosya, "satir": u.konum.satir, "sutun": u.konum.sutun, "mesaj": u.mesaj
            })
        })
        .collect();
    let hatalar: Vec<_> = match derleme::yukle(&s.dosya) {
        Ok(_) => Vec::new(),
        Err(h) => vec![match h.teshis {
            Some(t) => serde_json::json!({
                "dosya": t.dosya, "satir": t.satir, "sutun": t.sutun, "mesaj": t.mesaj, "ipucu": t.ipucu
            }),
            None => serde_json::json!({
                "dosya": dosya, "satir": 0, "sutun": 0, "mesaj": h.metin, "ipucu": null
            }),
        }],
    };
    let basarili = hatalar.is_empty();
    println!(
        "{}",
        serde_json::json!({ "dosya": dosya, "basarili": basarili, "hatalar": hatalar, "uyarilar": uyarilar })
    );
    Ok(if basarili {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn yayinla_komutu(args: &[String]) -> Result<(), String> {
    let mut a = orhunca::yayinla::Ayarlar {
        sunucu: None,
        alan: None,
        kapi: 3000,
        klasor: None,
        ssh_kapi: None,
    };
    let mut dosya = None;
    let (mut cgi, mut kaynakla) = (false, false);
    let mut i = 0;
    while i < args.len() {
        if matches!(args[i].as_str(), "--cgi" | "--kaynakla") {
            cgi |= args[i] == "--cgi";
            kaynakla |= args[i] == "--kaynakla";
            i += 1;
            continue;
        }
        let deger = |i: usize| {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("{} için bir değer verin", args[i]))
        };
        match args[i].as_str() {
            "--alan" => a.alan = Some(deger(i)?.trim().trim_end_matches('/').to_lowercase()),
            "--kapı" | "--kapi" => {
                let d = deger(i)?;
                a.kapi = d
                    .parse()
                    .ok()
                    .filter(|k| *k > 0)
                    .ok_or_else(|| format!("geçersiz kapı '{d}'"))?;
            }
            "--klasör" | "--klasor" => a.klasor = Some(deger(i)?),
            "--ssh-kapı" | "--ssh-kapi" => {
                let d = deger(i)?;
                a.ssh_kapi = Some(d.parse().map_err(|_| format!("geçersiz kapı '{d}'"))?);
            }
            s if s.starts_with('-') => return Err(format!("bilinmeyen seçenek '{s}'")),
            s if s.ends_with(".ohc") => dosya = Some(PathBuf::from(s)),
            s => a.sunucu = Some(s.to_string()),
        }
        i += if args[i].starts_with('-') { 2 } else { 1 };
    }
    let giris = match dosya {
        Some(d) => d,
        None => proje_girisi()?,
    };
    if kaynakla && !cgi {
        return Err("--kaynakla yalnızca --cgi ile kullanılır".into());
    }
    if cgi {
        if a.sunucu.is_some() || a.alan.is_some() {
            return Err(
                "--cgi ile sunucu ve alan adı verilmez: cikti/cgi/ klasörünü \
                 barındırmanın dosya yöneticisi ya da FTP ile public_html klasörüne yükleyin"
                    .into(),
            );
        }
        let k = orhunca::cgi::hazirla(&giris, kaynakla)?;
        println!(
            "Paylaşımlı hosting klasörü hazır: {}\n\n\
             Klasörün İÇİNDEKİLERİ (gizli .htaccess dosyası dahil) barındırmanın public_html\n\
             klasörüne yükleyin. {} izni 755 olmalı (dosya yöneticisinde \"İzinler\").\n\
             Ayrıntılar: https://github.com/Furkan003/Orhunca/blob/HEAD/docs/yayinlama.md#paylaşımlı-hosting",
            k.display(),
            if kaynakla { "orhunca.cgi dosyasının" } else { "uygulama.cgi dosyasının" }
        );
        return Ok(());
    }
    let (yayin, ad) = orhunca::yayinla::hazirla(&giris, &a)?;
    match &a.sunucu {
        Some(s) => orhunca::yayinla::gonder(&yayin, &ad, s, &a),
        None => {
            println!(
                "Yayın klasörü hazır: {}\n\n\
                 Sunucuya göndermek için:  orhunca yayınla kullanıcı@sunucu{}\n\
                 Ya da klasörü sunucuya kendiniz kopyalayıp orada çalıştırın:  sudo sh kur.sh",
                yayin.display(),
                a.alan
                    .as_ref()
                    .map(|x| format!(" --alan {x}"))
                    .unwrap_or_default()
            );
            Ok(())
        }
    }
}

fn cevir_komutu(args: &[String]) -> Result<(), String> {
    let mut dil = orhunca::cevirici::Dil::Python;
    let mut kalan = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--dil" {
            let d = args
                .get(i + 1)
                .ok_or("--dil için bir dil verin (python ya da javascript)")?;
            dil = orhunca::cevirici::Dil::coz(d)
                .ok_or_else(|| format!("bilinmeyen dil '{d}' (python ya da javascript)"))?;
            i += 2;
        } else {
            kalan.push(args[i].clone());
            i += 1;
        }
    }
    let s = secenekleri_oku(&kalan)?;
    let p = derleme::yukle(&s.dosya).map_err(|h| h.metin)?;
    print!(
        "{}",
        orhunca::cevirici::metin(&orhunca::cevirici::cevir(&p, dil))
    );
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
        derleme::derle_ayiklamali(&s.dosya, &cikti, &[]).map_err(|h| h.metin)?;
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
    if let Some(a) = args
        .iter()
        .find(|a| a.starts_with('-') && *a != "--denetle")
    {
        return Err(format!(
            "bilinmeyen seçenek '{a}'\nKullanım: orhunca biçimlendir [dosya.ohc ...] [--denetle]"
        ));
    }
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

/// `orhunca düzelt`: eskiyen yazımları (bkz. src/goc.rs) yenisine çevirir.
fn duzelt_komutu(args: &[String]) -> Result<ExitCode, String> {
    if let Some(a) = args
        .iter()
        .find(|a| a.starts_with('-') && *a != "--denetle")
    {
        return Err(format!(
            "bilinmeyen seçenek '{a}'\nKullanım: orhunca düzelt [dosya.ohc ...] [--denetle]"
        ));
    }
    let denetle = args.iter().any(|a| a == "--denetle");
    let mut dosyalar: Vec<PathBuf> = Vec::new();
    for a in args.iter().filter(|a| !a.starts_with('-')) {
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
    let mut toplam = 0;
    for d in &dosyalar {
        let kaynak =
            std::fs::read_to_string(d).map_err(|e| format!("'{}' okunamadı: {e}", d.display()))?;
        let (yeni, n) = orhunca::goc::uygula(&kaynak, orhunca::goc::GOCLER);
        if n == 0 {
            continue;
        }
        toplam += n;
        if denetle {
            println!("{}: {n} eski yazım", d.display());
        } else {
            std::fs::write(d, yeni).map_err(|e| format!("'{}' yazılamadı: {e}", d.display()))?;
            println!("{}: {n} eski yazım güncellendi", d.display());
        }
    }
    if toplam == 0 {
        println!(
            "{} dosyada güncellenecek eski yazım yok (dil sürümü {}).",
            dosyalar.len(),
            orhunca::goc::DIL_SURUMU
        );
    } else if denetle {
        return Ok(ExitCode::FAILURE);
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
            a if a.starts_with('-') => {
                return Err(format!(
                    "bilinmeyen seçenek '{a}'\nKullanım: orhunca yeni <proje_adı> [--şablon konsol|web_sitesi|...]"
                ))
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
    // Boşluk ya da kabuk için özel karakter içeren ad tırnak içinde yazılır
    // (PowerShell, cmd ve sh'de aynı biçimde çalışır).
    let cd_adi = if ad
        .chars()
        .any(|c| c.is_whitespace() || "&()'`;$|<>^%!,".contains(c))
    {
        format!("\"{ad}\"")
    } else {
        ad.clone()
    };
    println!(
        "'{ad}' projesi oluşturuldu ({}).\n  cd {cd_adi}\n  orhunca çalıştır",
        sablon.ad
    );
    if sablonlar::arayuz_mu(sablon) {
        println!("Uygulama tarayıcıda açılır (WebAssembly).");
    } else if sablonlar::web_mi(sablon) {
        println!("Sonra tarayıcıda http://localhost:3000 adresini açın.");
    }
    Ok(())
}
