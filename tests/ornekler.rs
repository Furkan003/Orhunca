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

#[test]
fn ic_ice_modeller_kaydedilip_okunur() {
    let klasor = std::env::temp_dir().join(format!("orhunca-ic-ice-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&klasor);
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join("p.ohc");
    std::fs::write(
        &dosya,
        "model Adres:\n    şehir: metin\nmodel Kişi:\n    ad: metin\n    adres: Adres\n    eski: liste<Adres>\n\
         k = Kişi(ad: \"Ali\")\nk.adres.şehir = \"Van\"\nAdres(şehir: \"Muş\")'u k.eski'ye ekle.\nk'yi kaydet.\n\
         y = Kişi.bul(1)\n(y.adres.şehir + \" \" + y.eski[0].şehir)'i yaz.\nKişi.hepsi()[0].ad'ı yaz.\n",
    )
    .unwrap();
    let c = orhunca()
        .arg("çalıştır")
        .arg(&dosya)
        .env("ORHUNCA_VERI", klasor.join("veri"))
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    assert_eq!(String::from_utf8_lossy(&c.stdout), "Van Muş\nAli\n");
    let veri = std::fs::read_to_string(klasor.join("veri/Kişi.json")).unwrap();
    assert!(veri.contains("\"adres\": {\"şehir\": \"Van\"}"), "{veri}");
    let _ = std::fs::remove_dir_all(&klasor);
}

#[test]
fn on_kutuphane_programin_tanimlarindan_etkilenmez() {
    // Programın `harfler` ve `kırp` tanımları ön kütüphanenin içini bozmaz;
    // programın `kırp`ı kendi çağrılarında yerleşiğin yerine geçer.
    let (ok, cikti, hata) = calistir(
        "işlev harfler(m: metin) -> liste<metin>:\n    döndür [\"x\"]\n\
         işlev kırp(m: metin) -> metin:\n    döndür \"kendi\"\n\
         kırp(\"  a  \")'yı yaz.\nböl(\" a  b \", \"\")'yi yaz.\nbüyük_harf(\"çiğ\")'i yaz.\n",
    );
    assert!(ok, "{hata}");
    assert_eq!(cikti, "kendi\n[\"a\", \"b\"]\nÇİĞ\n");
    let (ok, _, hata) = calistir("tekrarla(\"a\", -2)'yi yaz.\n");
    assert!(!ok);
    assert!(
        hata.contains("satır 1") && hata.contains("tekrar sayısı negatif olamaz"),
        "{hata}"
    );
}

#[test]
fn ayni_veriye_birden_cok_program_yazabilir() {
    // 1.0 test raporu, bulgu 1: aynı JSON veritabanına iki program aynı anda yazınca
    // Windows'ta biri "yazılamadı" hatasıyla duruyor, kayıtların yarısı kayboluyordu.
    let klasor = std::env::temp_dir().join(format!("orhunca-esz-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&klasor);
    std::fs::create_dir_all(&klasor).unwrap();
    let yazan = klasor.join("yazan.ohc");
    std::fs::write(
        &yazan,
        "model Not:\n    puan: sayı\nher i için 1'den 30'e kadar:\n    n = Not(puan: i)\n    n'yi kaydet.\n",
    )
    .unwrap();
    let okuyan = klasor.join("okuyan.ohc");
    std::fs::write(
        &okuyan,
        "model Not:\n    puan: sayı\ntoplam = 0\nher i için 1'den 200'e kadar:\n    toplam += uzunluk(Not.hepsi())\n",
    )
    .unwrap();
    let derle = |kaynak: &std::path::Path, ad: &str| {
        let cikti = klasor.join(format!("{ad}{}", std::env::consts::EXE_SUFFIX));
        let c = orhunca()
            .arg("derle")
            .arg(kaynak)
            .arg("-o")
            .arg(&cikti)
            .output()
            .unwrap();
        assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
        cikti
    };
    let (yazan, okuyan) = (derle(&yazan, "yazan"), derle(&okuyan, "okuyan"));
    let mut surecler = Vec::new();
    for p in [&yazan, &yazan, &yazan, &okuyan] {
        surecler.push(
            std::process::Command::new(p)
                .env("ORHUNCA_VERI", klasor.join("veri"))
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    for s in surecler {
        let c = s.wait_with_output().unwrap();
        assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    }
    let veri = std::fs::read_to_string(klasor.join("veri/Not.json")).unwrap();
    assert_eq!(veri.matches("\"kimlik\": ").count(), 90, "{veri}");
    for k in 1..=90 {
        assert!(
            veri.contains(&format!("\"kimlik\": {k},")),
            "kimlik {k} yok: {veri}"
        );
    }
    let _ = std::fs::remove_dir_all(&klasor);
}

#[test]
fn nan_ve_sonsuz_veriye_karismaz() {
    // Hata raporu B21: NaN doğrulamayı geçip kayıtta sessizce 0'a dönüşüyordu.
    let klasor = std::env::temp_dir().join(format!("orhunca-nan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&klasor);
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join("p.ohc");
    std::fs::write(
        &dosya,
        "model Ürün:\n    fiyat: ondalık, zorunlu, en_az 0\n\
         (ondalık_mı(\"NaN\"))'yı yaz.\n(ondalık_mı(\"inf\"))'i yaz.\n(ondalık_mı(\"1e999\"))'u yaz.\n\
         (ondalık(\"3,5\"))'i yaz.\n\
         ü = Ürün(fiyat: 1.0)\nher i için 1'den 400'e kadar:\n    ü.fiyat = ü.fiyat * 10.0\n\
         ü.geçerli_mi()'yi yaz.\nü.hatalar()'ı yaz.\n\
         dene:\n    ü'yü kaydet.\nyakala h:\n    h'yi yaz.\n\
         dene:\n    üs(-8.0, 0.5)'i yaz.\nyakala h:\n    h'yi yaz.\n(üs(2.0, 0.5))'i yaz.\n",
    )
    .unwrap();
    let c = orhunca()
        .arg("çalıştır")
        .arg(&dosya)
        .env("ORHUNCA_VERI", klasor.join("veri"))
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "yanlış\nyanlış\nyanlış\n3.5\nyanlış\n[\"Fiyat geçerli bir sayı olmalı\"]\n\
         'fiyat' alanı geçerli bir sayı değil (NaN ya da sonsuz); kayıt yapılmadı\n\
         negatif bir sayının kesirli üssü alınamaz (ör. üs(-8.0, 0.5))\n1.4142135623731\n"
    );
    assert!(!klasor.join("veri/Ürün.json").exists());
    let _ = std::fs::remove_dir_all(&klasor);
}

#[test]
fn yardim_secenegi_hicbir_sey_degistirmez() {
    // Hata raporu B01/B02: `biçimlendir --help` dosyaları biçimlendiriyor,
    // `yeni --help` '--help' adlı proje oluşturuyordu.
    let klasor = std::env::temp_dir().join(format!("orhunca-yardim-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&klasor);
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join("a.ohc");
    std::fs::write(&dosya, "x = 1   \n\n\n\n").unwrap();
    for args in [
        &["biçimlendir", "--help"][..],
        &["yeni", "--help"],
        &["yeni", "-h"],
        &["derle", "--yardım"],
    ] {
        let c = orhunca().args(args).current_dir(&klasor).output().unwrap();
        assert!(c.status.success(), "{args:?}");
        assert!(
            String::from_utf8_lossy(&c.stdout).contains("Kullanım"),
            "{args:?}"
        );
    }
    assert_eq!(std::fs::read_to_string(&dosya).unwrap(), "x = 1   \n\n\n\n");
    assert_eq!(std::fs::read_dir(&klasor).unwrap().count(), 1);
    let c = orhunca()
        .args(["biçimlendir", "--sil"])
        .current_dir(&klasor)
        .output()
        .unwrap();
    assert!(!c.status.success());
    // Boşluklu proje adında `cd` tırnaklı yazılır (B23).
    let c = orhunca()
        .args(["yeni", "boş luk"])
        .current_dir(&klasor)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&c.stdout).contains("cd \"boş luk\""),
        "{}",
        String::from_utf8_lossy(&c.stdout)
    );
    let _ = std::fs::remove_dir_all(&klasor);
}

#[test]
fn bilinmeyen_kacis_oldugu_gibi_kalir() {
    // Hata raporu B03: rehberdeki "\d+" gibi desen örnekleri derlenmiyordu.
    let (tamam, cikti, hata) = calistir(
        "(desen_bul(\"abc123\", \"\\d+\"))'i yaz.\n(desen_bul(\"abc123\", \"\\\\d+\"))'i yaz.\n\
         (uzunluk(\"\\q\"))'u yaz.\n\"a\\tb\"'yi yaz.\n",
    );
    assert!(tamam, "{hata}");
    assert_eq!(cikti, "123\n123\n2\na\tb\n");
}

#[test]
fn env_dosyasi_yuklenir() {
    let klasor = std::env::temp_dir().join(format!("orhunca-env-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&klasor);
    std::fs::create_dir_all(&klasor).unwrap();
    std::fs::write(
        klasor.join(".env"),
        "# gizli ayarlar\nANAHTAR=sk-123\nexport KAPI = 8080\nMESAJ=\"a # b\"\nYORUMLU=d # açıklama\nONCELIK=dosya\n",
    )
    .unwrap();
    let dosya = klasor.join("p.ohc");
    std::fs::write(
        &dosya,
        "her ad için [\"ANAHTAR\", \"KAPI\", \"MESAJ\", \"YORUMLU\", \"ONCELIK\", \"YOK\"]'dan:\n    (ad + \"=\" + ortam(ad))'yı yaz.\n",
    )
    .unwrap();
    let c = orhunca()
        .arg("çalıştır")
        .arg(&dosya)
        .current_dir(&klasor)
        .env("ONCELIK", "sistem")
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "ANAHTAR=sk-123\nKAPI=8080\nMESAJ=a # b\nYORUMLU=d\nONCELIK=sistem\nYOK=\n"
    );
    let _ = std::fs::remove_dir_all(&klasor);
}
