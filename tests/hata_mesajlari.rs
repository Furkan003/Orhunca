//! Yeni başlayanların sık yaptığı hatalarda derleyicinin anlaşılır mesaj ve öneri
//! vermesi (başka dillerden gelen alışkanlıklar, yanlış yazılmış isimler).

use std::process::Command;

fn denetle(kaynak: &str) -> String {
    let klasor = std::env::temp_dir().join(format!("orhunca-hata-mesaji-{}", std::process::id()));
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join(format!("p{}.ohc", kaynak.len()));
    std::fs::write(&dosya, kaynak).unwrap();
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .arg("denetle")
        .arg(&dosya)
        .output()
        .unwrap();
    String::from_utf8_lossy(&c.stderr).into_owned()
}

#[test]
fn oneriler() {
    for (kaynak, beklenen) in [
        ("yaz(\"merhaba\")\n", "\"merhaba\"'yı yaz."),
        ("print(\"selam\")\n", "\"selam\"'ı yaz."),
        ("x = 5\nyaz(x + 1)\n", "(x + 1)'i yaz."),
        ("\"büyük\"ü yaz.\n", "\"büyük\"'ü şeklinde yazın"),
        ("for i in 1..3:\n    dur\n", "her i için 1'den 10'a kadar"),
        ("değişken n = 0\n", "yalnızca: x = 5"),
        ("n = tam(\"5\")\n", "sayı(\"42\")"),
        (
            "eger 3 > 2 ise:\n    dur\n",
            "bunu mu demek istediniz: eğer",
        ),
        (
            "toplam = 5\ntoplm'ı yaz.\n",
            "bunu mu demek istediniz: toplam",
        ),
        ("x = uzunlk([1, 2])\n", "bunu mu demek istediniz: uzunluk"),
        ("x = \"5\" + 3\nx'i yaz.\n", "sayı(\"5\")"),
    ] {
        let c = denetle(kaynak);
        assert!(c.contains(beklenen), "{kaynak:?}\n→ {c}");
    }
}

/// Aşırı derin ya da uzun girdi derleyiciyi çökertmez; anlaşılır hata verir ve uzun
/// satır kısaltılarak gösterilir.
#[test]
fn derinlik_sinirlari() {
    let derin = format!("x = {}1{}\n", "(".repeat(5000), ")".repeat(5000));
    let c = denetle(&derin);
    assert!(c.contains("çok fazla iç içe parantez"), "{c}");
    assert!(c.len() < 1000, "{}", c.len());

    let uzun = format!("x = 1{}\n", " + 1".repeat(100_000));
    let c = denetle(&uzun);
    assert!(c.contains("ifade çok uzun"), "{c}");

    let mut bloklar = String::new();
    for i in 0..500 {
        bloklar.push_str(&format!("{}eğer doğru ise:\n", "    ".repeat(i)));
    }
    bloklar.push_str(&format!("{}dur\n", "    ".repeat(500)));
    let c = denetle(&bloklar);
    assert!(c.contains("çok fazla iç içe blok"), "{c}");
}
