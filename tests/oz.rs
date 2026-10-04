//! Öz-barındırmanın ilk adımı: Orhunca ile yazılmış sözcük çözümleyici
//! (öz/sözcük.ohc) derleyicinin çözümleyicisiyle (src/sozcuk.rs) aynı sözcükleri,
//! aynı konumları ve aynı hata mesajlarını üretmelidir.

use orhunca::sozcuk::{sozcukle, Tok};
use std::path::{Path, PathBuf};
use std::process::Command;

fn kok() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn gecici(ad: &str) -> PathBuf {
    let k = std::env::temp_dir().join(format!("orhunca-oz-{ad}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    k
}

/// Bir klasördeki (alt klasörler dahil) tüm .ohc dosyaları
fn ohc_dosyalari(klasor: &Path, cikti: &mut Vec<PathBuf>) {
    for g in std::fs::read_dir(klasor).unwrap() {
        let yol = g.unwrap().path();
        if yol.is_dir() {
            ohc_dosyalari(&yol, cikti);
        } else if yol.extension().is_some_and(|u| u == "ohc") {
            cikti.push(yol);
        }
    }
}

/// Derleyicinin çözümleyicisinin beklenen çıktısı: Orhunca programının yazdığı biçimde.
fn beklenen(kaynak: &str) -> Vec<String> {
    match sozcukle(kaynak) {
        Ok(sozcukler) => sozcukler
            .into_iter()
            .map(|s| {
                let (tur, deger) = match s.tok {
                    Tok::Sayi(n) => ("Sayi", n.to_string()),
                    Tok::Ondalik(f) => ("Ondalik", format!("{f:?}")),
                    Tok::Metin(m) => ("Metin", serde_json::to_string(&m).unwrap()),
                    Tok::Kelime(k) => ("Kelime", k),
                    Tok::Ek(e) => ("Ek", e),
                    Tok::Op(o) => ("Op", o.to_string()),
                    Tok::Uye => ("Uye", String::new()),
                    Tok::YeniSatir => ("YeniSatir", String::new()),
                    Tok::Girinti => ("Girinti", String::new()),
                    Tok::Cikinti => ("Cikinti", String::new()),
                    Tok::Son => ("Son", String::new()),
                };
                format!("{}:{} {tur} {deger}", s.konum.satir, s.konum.sutun)
            })
            .collect(),
        Err(h) => vec![format!(
            "{}:{} HATA {}",
            h.konum.satir, h.konum.sutun, h.mesaj
        )],
    }
}

/// Orhunca çözümleyicisinin çıktısını karşılaştırılabilir biçime getirir: ondalıklar
/// ve metinler aynı gösterime çevrilir; hata olursa yalnızca hata satırı alınır.
fn duzenle(cikti: &str) -> Vec<String> {
    let satirlar: Vec<String> = cikti
        .lines()
        .map(|s| {
            let mut p = s.splitn(3, ' ');
            let (konum, tur, deger) =
                (p.next().unwrap(), p.next().unwrap(), p.next().unwrap_or(""));
            let deger = match tur {
                "Ondalik" => format!("{:?}", deger.parse::<f64>().unwrap()),
                "Metin" => {
                    let m: String = serde_json::from_str(deger).unwrap();
                    serde_json::to_string(&m).unwrap()
                }
                _ => deger.to_string(),
            };
            format!("{konum} {tur} {deger}")
        })
        .collect();
    match satirlar.iter().find(|s| s.contains(" HATA ")) {
        Some(h) => vec![h.clone()],
        None => satirlar,
    }
}

/// Zorlu durumlar ve hatalar: her biri ayrı bir dosya olarak denenir.
const ORNEKLER: &[&str] = &[
    "x = 1_000_000\ny = 3.14\nz = 2.5_5\n5.\n",
    "\"kaçış: \\n \\t \\\" \\\\\"'i yaz.\n",
    "ü.ad'ı yaz. x.\ny = Ürün.hepsi()\n(a).b\n[1][0].c\n",
    "eğer x:\n\ty = 1\n\tz = (1 +\n2)\n    w = 3\nson = 1\n",
    "a = [1,\n     2,\n  3]\nb = {\"a\": 1}\n",
    "kitap’ı yaz.\nx'den y'ye\n# yorum\n    # girintili yorum\n\n",
    "x->y // z == w != q <= r >= s += t -= u + - * / % = < > ( ) [ ] { } , :\n",
    "_gizli = _1\nçağrı_2 = Öğe3\n",
    "5i yaz\n",
    "x = 99999999999999999999\n",
    "\"kapanmamış\n",
    "\"kötü \\q kaçış\"\n",
    "eğer x:\n        y = 1\n    z = 2\n",
    "x = (1, 2\n",
    "'i yaz\n",
    "x = 5 @ 3\n",
    "x'\n",
    "a = )\nb = 1\n",
];

#[test]
fn oz_sozcuk_cozumleyici_derleyiciyle_ayni_sozcukleri_uretir() {
    let calisma = gecici("sozcuk");
    let program = calisma.join("sozcukle");
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg("derle")
        .arg(kok().join("öz/sözcükle.ohc"))
        .arg("-o")
        .arg(&program)
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));

    let mut dosyalar = Vec::new();
    for klasor in ["örnekler", "öz", "tests", "src/studyo/sablon_dosyalari"] {
        ohc_dosyalari(&kok().join(klasor), &mut dosyalar);
    }
    for (i, o) in ORNEKLER.iter().enumerate() {
        let d = calisma.join(format!("zor{i}.ohc"));
        std::fs::write(&d, o).unwrap();
        dosyalar.push(d);
    }
    assert!(dosyalar.len() > 40, "{}", dosyalar.len());

    let mut sozcuk_sayisi = 0;
    for d in &dosyalar {
        let kaynak = std::fs::read_to_string(d).unwrap();
        let c = Command::new(&program).arg(d).output().unwrap();
        assert!(c.status.success(), "{}", d.display());
        let gercek = duzenle(&String::from_utf8_lossy(&c.stdout));
        let beklenen = beklenen(&kaynak);
        if let Some(i) =
            (0..gercek.len().max(beklenen.len())).find(|i| gercek.get(*i) != beklenen.get(*i))
        {
            panic!(
                "{}: {}. sözcükte ayrıldı\n  derleyici: {:?}\n  Orhunca:   {:?}",
                d.display(),
                i + 1,
                beklenen.get(i),
                gercek.get(i)
            );
        }
        sozcuk_sayisi += gercek.len();
    }
    assert!(sozcuk_sayisi > 5000, "{sozcuk_sayisi}");
    let _ = std::fs::remove_dir_all(&calisma);
}

#[test]
fn oz_sozcuk_cozumleyici_webassembly_ile_de_calisir() {
    if !Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|c| c.status.success())
    {
        eprintln!("Node.js bulunamadı: test atlandı");
        return;
    }
    let calisma = gecici("wasm");
    let ornek = kok().join("örnekler/standart_kutuphane.ohc");
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg("çalıştır")
        .arg(kok().join("öz/sözcükle.ohc"))
        .args(["--hedef", "web", "--"])
        .arg(&ornek)
        .current_dir(&calisma)
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let kaynak = std::fs::read_to_string(&ornek).unwrap();
    assert_eq!(
        duzenle(&String::from_utf8_lossy(&c.stdout)),
        beklenen(&kaynak)
    );
    let _ = std::fs::remove_dir_all(&calisma);
}
