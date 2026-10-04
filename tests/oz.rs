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

/// Ayrıştırıcının hata yolları: her biri ayrı bir dosya olarak denenir.
const HATALI_PROGRAMLAR: &[&str] = &[
    "x = 5\neğer x 4'e büyükse:\n    x'i yaz.\n",
    "x = 5\neğer x 4'ten büyükken:\n    x'i yaz.\n",
    "x = 1\nx 10'dan küçükse:\n    x += 1\n",
    "x'i yaz\n",
    "5'i yaz bakalım\n",
    "x = 3\nx'i y'ye ekle.\n",
    "l = []\n5'i l'ye ekle\n",
    "l = []\n5'i l'den ekle.\n",
    "x = 1\nx yaz.\n",
    "x = 1\nx'i koş.\n",
    "değilse:\n    x = 1\n",
    "yakala h:\n    x = 1\n",
    "dene:\n    x = 1\nx = 2\n",
    "işlev f(a, b:\n    döndür a\n",
    "işlev uzunluk(a):\n    döndür a\n",
    "işlev f(a: tamsayı):\n    döndür a\n",
    "fiil x'i kare:\n    döndür x * x\n5'i karele.\n",
    "fiil x'in karele:\n    döndür x\n",
    "fiil x'i y'yi topla:\n    döndür x\n",
    "fiil x yaz:\n    döndür x\n",
    "model Ürün:\n    ad: metin\n    ad: sayı\n",
    "model Ürün:\nx = 1\n",
    "model İstek:\n    a: sayı\n",
    "model Ürün:\n    ad: metin, zorla\n",
    "model Ürün:\n    kimlik: metin\n",
    "model Ürün:\n    ad: metin, en_az x\n",
    "model Ürün:\n    ad: metin, etiket \"\"\n",
    "seçenek Renk: a, a\n",
    "seçenek Renk:\nx = 1\n",
    "al \"ürünler\":\n    döndür 1\n",
    "al \"/a/{x y}\":\n    döndür 1\n",
    "al \"/a/{x: ondalık}\":\n    döndür 1\n",
    "al \"/a/b{x}\":\n    döndür 1\n",
    "x = (1 + 2\n",
    "x = [1, 2\ny = 3\n",
    "x = 5 +\n",
    "x = y\n",
    "kitap = 1\nkitapla = 2\nkitapla'yı yaz.\nkitapla yaz.\n",
    "x = 1\nx'xyz yaz.\n",
    "l = [1]\nl'nin boyu'nu yaz.\n",
    "l = [1]\nl'nin uzunluğu'nu yaz.\n",
    "x = 1\nx'i'yi yaz.\n",
    "model Ürün:\n    ad: metin\nÜrün'ü yaz.\n",
    "model Ürün:\n    ad: metin\nü = Ürün(\"a\")\n",
    "model Ürün:\n    ad: metin\nü = Ürün(ad: 1, ad: 2)\n",
    "seçenek Renk: a\nRenk'i yaz.\n",
    "durum x = 1\ndurum x = 2\n",
    "arayüz:\n    düğme(\"a\") basılınca:\n        x = 1\n",
    "arayüz:\n    düğme(renk: \"a\", \"b\")\n",
    "arayüz:\n    düğme(\"a\", renk: 1, renk: 2)\n",
    "arayüz:\n    yazı(\"a\")\narayüz:\n    yazı(\"b\")\n",
    "her i için 1'e 10'a kadar:\n    i'yi yaz.\n",
    "her i için 1'den 10'dan kadar:\n    i'yi yaz.\n",
    "x = 1\neğer x:\nx = 2\n",
    "eğer doğru ise:\n    işlev f():\n        döndür 1\n",
    "  x = 1\n",
    "x = 1\nx 2'ye eşit değilse ve x 3'ten büyük veya eşitse:\n    x'i yaz.\n",
    "x = 1\neğer x 2'ye eşitken:\n    x'i yaz.\n",
    "x = 1\nx'i:\n",
    "fiil x'i karele:\n    döndür x * x\ny = 5'i karele'yi\n",
    "fiil x'i karele:\n    döndür x * x\ny = 5'i 3\n",
    "y = 5'i\n",
    "x = 1\nx.y.z = 3\n",
    "x = 1\n(x)'e = 3\n",
    "sabit A = 1\nsabit A = 2\n",
    "kullan 5\n",
    "x: liste<metin = []\n",
    "x: sözlük<metin> = {}\n",
    "x = Ürün.hepsi()\n",
    "x = 1\nx'den 5'e yaz.\n",
    "uzunluğu'nu yaz.\n",
    "dur\n",
    "x = 1\n5'i x'e yaz yaz.\n",
    "x = {1: 2, 3}\n",
    "işlev f() -> liste<:\n    döndür 1\n",
    "x = 1\nx.\n",
    "x = 1\nx.5 = 2\n",
    "\"kapanmamış\n",
    "x = 5 @ 3\n",
];

#[test]
fn oz_ayristirici_derleyiciyle_ayni_agaci_uretir() {
    let calisma = gecici("ayristirici");
    let program = calisma.join("ayrıştır");
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg("derle")
        .arg(kok().join("öz/ayrıştır.ohc"))
        .arg("-o")
        .arg(&program)
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));

    let mut dosyalar = Vec::new();
    for klasor in ["örnekler", "öz", "tests", "src/studyo/sablon_dosyalari"] {
        ohc_dosyalari(&kok().join(klasor), &mut dosyalar);
    }
    let ornek_sayisi = dosyalar.len();
    for (i, o) in HATALI_PROGRAMLAR.iter().chain(ORNEKLER).enumerate() {
        let d = calisma.join(format!("hatali{i}.ohc"));
        std::fs::write(&d, o).unwrap();
        dosyalar.push(d);
    }
    let mut hata_sayisi = 0;
    for d in &dosyalar {
        let kaynak = std::fs::read_to_string(d).unwrap();
        let c = Command::new(&program).arg(d).output().unwrap();
        assert!(c.status.success(), "{}", d.display());
        let cikti = String::from_utf8_lossy(&c.stdout);
        // `yaz` sona bir satır sonu ekler.
        let gercek = cikti.strip_suffix('\n').unwrap_or(&cikti);
        let beklenen = orhunca::dokum::kaynak(&kaynak);
        if gercek != beklenen {
            let satir = gercek
                .lines()
                .zip(beklenen.lines())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            panic!(
                "{}: {}. satırda ayrıldı\n  derleyici: {:?}\n  Orhunca:   {:?}",
                d.display(),
                satir + 1,
                beklenen.lines().nth(satir),
                gercek.lines().nth(satir)
            );
        }
        hata_sayisi += beklenen.starts_with("HATA@") as usize;
    }
    // Hatalı programların çoğu ayrıştırmada hata verir (bir kısmı ancak denetçide).
    assert!(
        hata_sayisi > HATALI_PROGRAMLAR.len() * 9 / 10,
        "{hata_sayisi}"
    );
    assert!(ornek_sayisi > 40, "{ornek_sayisi}");
    let _ = std::fs::remove_dir_all(&calisma);
}

#[test]
fn oz_ayristiricinin_tablolari_guncel() {
    let kaynak = std::fs::read_to_string(kok().join("öz/ayrıştırıcı.ohc")).unwrap();
    let liste = |islev: &str| -> Vec<String> {
        let bas = kaynak.find(&format!("işlev {islev}()")).unwrap();
        let satir = kaynak[bas..].lines().nth(1).unwrap();
        satir
            .split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect()
    };
    let yerlesikler: Vec<String> = orhunca::yerlesik::YERLESIKLER
        .iter()
        .map(|y| y.ad.to_string())
        .collect();
    assert_eq!(liste("standart_kütüphane"), yerlesikler);
    let ogeler: Vec<String> = orhunca::arayuz::OGELER
        .iter()
        .map(|o| o.ad.to_string())
        .collect();
    assert_eq!(liste("arayüz_öğeleri"), ogeler);
    let olaylar: Vec<String> = orhunca::arayuz::OLAYLAR
        .iter()
        .map(|(o, _)| o.to_string())
        .collect();
    assert_eq!(liste("arayüz_olayları"), olaylar);
}

#[test]
fn oz_ayristirici_webassembly_ile_de_calisir() {
    if !Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|c| c.status.success())
    {
        eprintln!("Node.js bulunamadı: test atlandı");
        return;
    }
    let calisma = gecici("ayristirici-wasm");
    // Ayrıştırıcının kendi kaynağı (en büyük dosya) ve bir hata yolu
    let hatali = calisma.join("hatali.ohc");
    std::fs::write(&hatali, HATALI_PROGRAMLAR[37]).unwrap();
    for dosya in [kok().join("öz/ayrıştırıcı.ohc"), hatali] {
        let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
            .arg("çalıştır")
            .arg(kok().join("öz/ayrıştır.ohc"))
            .args(["--hedef", "web", "--"])
            .arg(&dosya)
            .current_dir(&calisma)
            .output()
            .unwrap();
        assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
        let cikti = String::from_utf8_lossy(&c.stdout);
        let kaynak = std::fs::read_to_string(&dosya).unwrap();
        assert_eq!(
            cikti.strip_suffix('\n').unwrap_or(&cikti),
            orhunca::dokum::kaynak(&kaynak)
        );
    }
    let _ = std::fs::remove_dir_all(&calisma);
}
