//! WebAssembly hedefi (`--hedef web`): örnekler Node.js ile çalıştırılır ve
//! yerel derlemedeki `.beklenen` çıktıyı aynen vermelidir. Node.js kurulu
//! değilse çalıştırma testleri atlanır (dosya üretimi yine denetlenir).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn orhunca() -> Command {
    Command::new(env!("CARGO_BIN_EXE_orhunca"))
}

fn kok() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn node_var() -> bool {
    let var = Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|c| c.status.success());
    if !var {
        eprintln!("Node.js bulunamadı: WebAssembly çalıştırma testleri atlandı");
    }
    var
}

fn gecici(ad: &str) -> PathBuf {
    let k = std::env::temp_dir().join(format!("orhunca-wasm-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    k
}

fn web_calistir(dosya: &Path, klasor: &Path, ortam: &[(&str, &str)]) -> Output {
    let mut k = orhunca();
    k.arg("çalıştır")
        .arg(dosya)
        .arg("--hedef")
        .arg("web")
        .current_dir(klasor);
    for (a, d) in ortam {
        k.env(a, d);
    }
    k.output().unwrap()
}

#[test]
fn ornekler_webassembly_ile_ayni_ciktiyi_verir() {
    if !node_var() {
        return;
    }
    let calisma = gecici("ornekler");
    let mut sayi = 0;
    for giris in std::fs::read_dir(kok().join("örnekler")).unwrap() {
        let yol = giris.unwrap().path();
        if yol.extension().is_none_or(|u| u != "ohc") {
            continue;
        }
        let beklenen = std::fs::read_to_string(yol.with_extension("beklenen")).unwrap();
        let c = web_calistir(&yol, &calisma, &[]);
        assert!(
            c.status.success(),
            "{}: {}",
            yol.display(),
            String::from_utf8_lossy(&c.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stdout),
            beklenen,
            "{}",
            yol.display()
        );
        sayi += 1;
    }
    assert!(sayi >= 10);
    let _ = std::fs::remove_dir_all(&calisma);
}

#[test]
fn cop_toplayici_golge_yigitla_calisir() {
    if !node_var() {
        return;
    }
    let calisma = gecici("cop");
    let c = web_calistir(
        &kok().join("tests/cop_toplayici.ohc"),
        &calisma,
        &[("ORHUNCA_GC_ESIK", "2000"), ("ORHUNCA_BELLEK_RAPORU", "1")],
    );
    let hata = String::from_utf8_lossy(&c.stderr);
    assert!(c.status.success(), "{hata}");
    assert_eq!(String::from_utf8_lossy(&c.stdout), "500\n0\n");
    let toplama: u64 = hata
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    assert!(toplama > 100, "toplayıcı yeterince çalışmadı: {hata}");
}

#[test]
fn islev_cagrisi_boyunca_ara_degerler_korunur() {
    // Toplayıcı her ayırmada çalışacak kadar sık: bir Orhunca işlevi çağrılırken
    // yığıtta bekleyen metin/liste/sözlük ara değerleri gölge yığıtta olmalıdır.
    if !node_var() {
        return;
    }
    let calisma = gecici("ara");
    let beklenen = std::fs::read_to_string(kok().join("tests/ara_degerler.beklenen")).unwrap();
    let c = web_calistir(
        &kok().join("tests/ara_degerler.ohc"),
        &calisma,
        &[("ORHUNCA_GC_ESIK", "64")],
    );
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    assert_eq!(String::from_utf8_lossy(&c.stdout), beklenen);
}

#[test]
fn calisma_hatalari_ve_cikis_kodu() {
    if !node_var() {
        return;
    }
    let calisma = gecici("hatalar");
    let durumlar = [
        (
            "x = 1\ny = 0\n(x / y)'yi yaz.\n",
            1,
            "Çalışma hatası (satır 3): sıfıra bölme",
        ),
        (
            "x = 9223372036854775807\n(x + 1)'i yaz.\n",
            1,
            "tamsayı taşması",
        ),
        (
            "işlev f(n) -> sayı:\n    döndür f(n + 1) + 1\nf(0)'ı yaz.\n",
            1,
            "çok derin özyineleme",
        ),
        ("\"önce\"'yi yaz.\nçık(3)\n", 3, ""),
    ];
    for (i, (kaynak, kod, mesaj)) in durumlar.iter().enumerate() {
        let dosya = calisma.join(format!("h{i}.ohc"));
        std::fs::write(&dosya, kaynak).unwrap();
        let c = web_calistir(&dosya, &calisma, &[]);
        let hata = String::from_utf8_lossy(&c.stderr);
        assert_eq!(c.status.code(), Some(*kod), "{kaynak}: {hata}");
        assert!(hata.contains(mesaj), "{kaynak}: {hata}");
    }
}

#[test]
fn girdi_argumanlar_ve_dosyalar() {
    if !node_var() {
        return;
    }
    let calisma = gecici("girdi");
    let dosya = calisma.join("g.ohc");
    std::fs::write(
        &dosya,
        "ad = oku()\n\"Merhaba, \" + ad'ı yaz.\nargümanlar()'ı yaz.\n\
         \"not.txt\"'ye \"bir\"'i yaz.\ndosyaya_ekle(\"not.txt\", \"+iki\")\n\
         dosya_oku(\"not.txt\")'yu yaz.\ndosya_var(\"yok.txt\")'u yaz.\n",
    )
    .unwrap();
    let mut k = orhunca();
    k.arg("çalıştır")
        .arg(&dosya)
        .args(["--hedef", "web", "--", "a", "b c"])
        .current_dir(&calisma)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut surec = k.spawn().unwrap();
    use std::io::Write;
    surec
        .stdin
        .take()
        .unwrap()
        .write_all("Ayşe\n".as_bytes())
        .unwrap();
    let c = surec.wait_with_output().unwrap();
    assert!(c.status.success());
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "Merhaba, Ayşe\n[\"a\", \"b c\"]\nbir+iki\nyanlış\n"
    );
    let _ = std::fs::remove_dir_all(&calisma);
}

#[test]
fn web_sayfasi_ve_ayri_dosyalar_uretilir() {
    let calisma = gecici("sayfa");
    let c = orhunca()
        .arg("derle")
        .arg(kok().join("örnekler/merhaba.ohc"))
        .args(["--hedef", "web"])
        .current_dir(&calisma)
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let sayfa = std::fs::read_to_string(calisma.join("merhaba.html")).unwrap();
    assert!(sayfa.starts_with("<!doctype html>"));
    assert!(sayfa.contains("<title>merhaba</title>"));
    // İki wasm modülü base64 olarak gömülüdür ("\0asm" → "AGFzbQ").
    assert_eq!(sayfa.matches("'AGFzbQ").count(), 2);
    assert!(!sayfa.contains("__PROGRAM__"));

    let c = orhunca()
        .arg("derle")
        .arg(kok().join("örnekler/asal.ohc"))
        .args(["--hedef", "web", "-o", "asal.wasm"])
        .current_dir(&calisma)
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    for ad in ["asal.wasm", "orhunca_rt.wasm", "orhunca.js"] {
        assert!(calisma.join(ad).exists(), "{ad} yok");
    }
    if node_var() {
        let c = Command::new("node")
            .arg(calisma.join("orhunca.js"))
            .arg(calisma.join("asal.wasm"))
            .output()
            .unwrap();
        assert!(c.status.success());
        assert_eq!(
            String::from_utf8_lossy(&c.stdout),
            std::fs::read_to_string(kok().join("örnekler/asal.beklenen")).unwrap()
        );
    }
    let _ = std::fs::remove_dir_all(&calisma);
}

#[test]
fn web_yollari_webassemblyde_reddedilir() {
    let calisma = gecici("yol");
    let dosya = calisma.join("sunucu.ohc");
    std::fs::write(&dosya, "al \"/\":\n    döndür \"selam\"\n").unwrap();
    let c = orhunca()
        .arg("derle")
        .arg(&dosya)
        .args(["--hedef", "web"])
        .current_dir(&calisma)
        .output()
        .unwrap();
    assert!(!c.status.success());
    let hata = String::from_utf8_lossy(&c.stderr);
    assert!(hata.contains("web yolları (al \"/\")"), "{hata}");
    let _ = std::fs::remove_dir_all(&calisma);
}
