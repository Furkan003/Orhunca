//! Uçtan uca testler: örnek programları gerçek çalıştırılabilir dosyaya derler,
//! çalıştırır ve çıktıyı `.beklenen` dosyasıyla karşılaştırır.

use std::path::Path;
use std::process::Command;

fn orhunca() -> Command {
    Command::new(env!("CARGO_BIN_EXE_orhunca"))
}

#[test]
fn ornekler_beklenen_ciktiyi_verir() {
    let klasor = Path::new(env!("CARGO_MANIFEST_DIR")).join("örnekler");
    let mut sayi = 0;
    for giris in std::fs::read_dir(&klasor).unwrap() {
        let yol = giris.unwrap().path();
        if yol.extension().is_none_or(|u| u != "ohc") {
            continue;
        }
        let beklenen = std::fs::read_to_string(yol.with_extension("beklenen"))
            .unwrap_or_else(|_| panic!("{} için .beklenen dosyası yok", yol.display()));
        // Örnekler dosya yazabilir; geçici bir klasörde çalıştırılır.
        let calisma = std::env::temp_dir().join(format!("orhunca-ornek-{}", std::process::id()));
        std::fs::create_dir_all(&calisma).unwrap();
        let cikti = orhunca()
            .arg("çalıştır")
            .arg(&yol)
            .current_dir(&calisma)
            .output()
            .unwrap();
        assert!(
            cikti.status.success(),
            "{}: {}",
            yol.display(),
            String::from_utf8_lossy(&cikti.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&cikti.stdout),
            beklenen,
            "{}",
            yol.display()
        );
        sayi += 1;
    }
    assert!(sayi > 0);
}

fn calistir(kaynak: &str) -> (bool, String, String) {
    let klasor = std::env::temp_dir().join(format!(
        "orhunca-test-{}-{}",
        std::process::id(),
        kaynak.len()
    ));
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join(format!(
        "t{}.ohc",
        kaynak.bytes().map(|b| b as usize).sum::<usize>()
    ));
    std::fs::write(&dosya, kaynak).unwrap();
    let c = orhunca().arg("çalıştır").arg(&dosya).output().unwrap();
    let _ = std::fs::remove_file(&dosya);
    (
        c.status.success(),
        String::from_utf8_lossy(&c.stdout).into(),
        String::from_utf8_lossy(&c.stderr).into(),
    )
}

#[test]
fn derleme_hatasi_turkce() {
    let (ok, _, hata) = calistir("x = 5\neğer x 4'e büyükse:\n    x'i yaz.\n");
    assert!(!ok);
    assert!(hata.contains("ayrılma hâlinde (-den)"), "{hata}");
    assert!(hata.contains("2 | eğer x 4'e büyükse:"), "{hata}");
}

#[test]
fn calisma_hatasi_satir_numarasi() {
    let (ok, _, hata) = calistir("x = 1\ny = 0\nx / y'yi yaz.\n");
    assert!(!ok);
    assert!(
        hata.contains("satır 3") && hata.contains("sıfıra bölme"),
        "{hata}"
    );

    let (ok, _, hata) = calistir("l = [1, 2]\nl[2]'yi yaz.\n");
    assert!(!ok);
    assert!(hata.contains("liste sınırı aşıldı"), "{hata}");
}

#[test]
fn dongu_dur_surdur_ve_kisa_devre() {
    let kaynak = "\
i = 0
doğru olduğu sürece:
    i += 1
    eğer i 2'ye eşitse:
        sürdür
    eğer i 5'ten büyükse:
        dur
    i'yi yaz.
l = []
eğer uzunluk(l) > 0 ve l[0] == 1 ise:
    \"olmamalı\"'yı yaz.
";
    let (ok, cikti, hata) = calistir(kaynak);
    assert!(ok, "{hata}");
    assert_eq!(cikti, "1\n3\n4\n5\n");
}

#[test]
fn cop_toplayici_canli_nesneleri_korur() {
    // Eşik çok küçük tutulur: toplayıcı binlerce kez çalışır, hiçbir canlı
    // metin ya da liste kaybolmamalıdır.
    let dosya = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cop_toplayici.ohc");
    let c = orhunca()
        .arg("çalıştır")
        .arg(&dosya)
        .env("ORHUNCA_GC_ESIK", "2000")
        .env("ORHUNCA_BELLEK_RAPORU", "1")
        .output()
        .unwrap();
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
fn bellek_geri_verilir() {
    // 500 bin döngüde metin ve liste üretilir; canlı bellek küçük kalmalıdır.
    let kaynak = "\
i = 0
toplam = 0
i 500_000'den küçükken:
    s = \"kayıt-\" + i
    l = [i, i + 1]
    toplam += uzunluk(s) + uzunluk(l)
    i += 1
toplam'ı yaz.
";
    let klasor = std::env::temp_dir().join(format!("orhunca-bellek-{}", std::process::id()));
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join("bellek.ohc");
    std::fs::write(&dosya, kaynak).unwrap();
    let c = orhunca()
        .arg("çalıştır")
        .arg(&dosya)
        .env("ORHUNCA_GC_ESIK", "1000000")
        .env("ORHUNCA_BELLEK_RAPORU", "1")
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&klasor);
    let hata = String::from_utf8_lossy(&c.stderr);
    assert!(c.status.success(), "{hata}");
    let beklenen: usize = (0..500_000)
        .map(|i: usize| 6 + i.to_string().len() + 2)
        .sum();
    assert_eq!(String::from_utf8_lossy(&c.stdout), format!("{beklenen}\n"));
    let en_yuksek: u64 = hata
        .split_whitespace()
        .rev()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap();
    assert!(en_yuksek < 4_000_000, "bellek geri verilmiyor: {hata}");
}

#[test]
fn fiil_hatalari_turkce() {
    let (ok, _, hata) = calistir("fiil x'i y'ye böl:\n    döndür x / y\n10'u böl.\n");
    assert!(!ok);
    assert!(
        hata.contains("yönelme (-e) hâlinde bir öğe bekliyor"),
        "{hata}"
    );
    assert!(hata.contains("kullanım: x'i y'ye böl."), "{hata}");

    let (ok, _, hata) = calistir("x = 1\nx = 2.5\n");
    assert!(!ok);
    assert!(hata.contains("x = 0.0"), "{hata}");
}

#[test]
fn coklu_dosya_kullan() {
    let klasor = Path::new(env!("CARGO_MANIFEST_DIR")).join("örnekler/çoklu_dosya");
    let c = orhunca()
        .arg("çalıştır")
        .arg(klasor.join("ana.ohc"))
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let beklenen = std::fs::read_to_string(klasor.join("ana.beklenen")).unwrap();
    assert_eq!(String::from_utf8_lossy(&c.stdout), beklenen);
}

#[test]
fn programa_arguman_gecirilir() {
    let (ok, cikti, hata) = {
        let klasor = std::env::temp_dir().join(format!("orhunca-arg-{}", std::process::id()));
        std::fs::create_dir_all(&klasor).unwrap();
        let dosya = klasor.join("arg.ohc");
        std::fs::write(&dosya, "argümanlar()'ı yaz.\nçık(4)\n").unwrap();
        let c = orhunca()
            .arg("çalıştır")
            .arg(&dosya)
            .args(["--", "bir", "iki kelime"])
            .output()
            .unwrap();
        (
            c.status.code() == Some(4),
            String::from_utf8_lossy(&c.stdout).to_string(),
            String::from_utf8_lossy(&c.stderr).to_string(),
        )
    };
    assert!(ok, "{hata}");
    assert_eq!(cikti, "[\"bir\", \"iki kelime\"]\n");
}

#[test]
fn standart_kutuphane_hatalari() {
    let (ok, _, hata) = calistir("x = 4_000_000_000_000_000_000\n(x * 3)'ü yaz.\n");
    assert!(!ok);
    assert!(hata.contains("tamsayı taşması"), "{hata}");

    let (ok, _, hata) = calistir("s = {\"a\": 1}\ns[\"b\"]'yi yaz.\n");
    assert!(!ok);
    assert!(hata.contains("\"b\" anahtarı yok"), "{hata}");

    let (ok, _, hata) = calistir("böl(\"a\")'yı yaz.\n");
    assert!(!ok);
    assert!(hata.contains("kullanım: böl(metin, ayraç)"), "{hata}");
}

#[test]
fn tipi_yazilmis_degiskenler_ve_karakter_kodlari() {
    let (ok, cikti, hata) = calistir(
        "isimler: liste<metin> = []\n\"Ayşe\"'yi isimler'e ekle.\nsayaç: sözlük<metin, sayı> = {}\nsayaç[\"a\"] = 2\nisimler'i yaz.\nsayaç'ı yaz.\nkod(\"ç\")'yi yaz.\nkarakter(kod(\"Ğ\") + 1)'i yaz.\noran: ondalık = 3\noran'ı yaz.\n",
    );
    assert!(ok, "{hata}");
    assert_eq!(cikti, "[\"Ayşe\"]\n{\"a\": 2}\n231\nğ\n3.0\n");
    let (ok, _, hata) = calistir("x: sayı = \"beş\"\n");
    assert!(!ok);
    assert!(
        hata.contains("'x' sayı olarak tanımlandı; değeri metin"),
        "{hata}"
    );
}

#[test]
fn secenek_hatalari_turkce() {
    let on = "seçenek Renk: kırmızı, mavi\nseçenek Boy: kısa, uzun\n";
    for (kaynak, beklenen) in [
        (
            "Renk.mor'u yaz.\n",
            "'Renk' türünde 'mor' diye bir değer yok",
        ),
        ("(Renk.mavi == Boy.uzun)'u yaz.\n", "eşitlik"),
        ("(Renk.mavi == \"mavi\")'yı yaz.\n", "eşitlik"),
        ("x = Renk\n", "'Renk' bir seçenek türü"),
        ("Renk.sil()'i yaz.\n", "yalnızca Renk.hepsi()"),
        ("r: Renk = \"mavi\"\n", "Renk"),
    ] {
        let (ok, _, hata) = calistir(&format!("{on}{kaynak}"));
        assert!(!ok, "{kaynak}");
        assert!(hata.contains(beklenen), "{kaynak}: {hata}");
    }
    let (ok, _, hata) = calistir("seçenek Renk: kırmızı, kırmızı\n");
    assert!(!ok && hata.contains("iki kez"), "{hata}");
    let (ok, _, hata) = calistir("seçenek Renk: a\nRenk(\"b\")'yi yaz.\n");
    assert!(
        !ok && hata.contains("'b' bir Renk değeri değil (değerler: a)"),
        "{hata}"
    );
}

#[test]
fn indeksli_birlesik_atama() {
    let (ok, cikti, hata) = calistir(
        "l = [1, 2]\nl[1] += 5\nl[0] -= 1\ns = {\"a\": 1}\ns[\"a\"] += 2\nl'yi yaz.\ns'yi yaz.\n",
    );
    assert!(ok, "{hata}");
    assert_eq!(cikti, "[0, 7]\n{\"a\": 3}\n");
}
