//! Hata mesajları için öneriler: başka programlama dillerinden gelen alışkanlıkların
//! Orhunca karşılıkları ve yanlış yazılmış isimler için "bunu mu demek istediniz?".

/// Başka dillerde sık kullanılan bir kelimenin Orhunca karşılığı (ipucu metni).
pub fn yabanci(ad: &str) -> Option<&'static str> {
    let k = ad.to_lowercase();
    Some(match k.as_str() {
        "print" | "println" | "printf" | "puts" | "echo" | "console" | "yazdır" | "yazdir"
        | "cout" | "write" | "writeln" => {
            "Orhunca'da ekrana yazmak bir cümledir: \"Merhaba\"'yı yaz. ya da x'i yaz."
        }
        "input" | "scanf" | "readline" | "cin" | "read" | "gets" => {
            "kullanıcıdan girdi almak için: ad = oku()   (sayı için: yaş = sayı(oku()))"
        }
        "for" | "foreach" | "döngü" | "dongu" => {
            "döngü için: her i için 1'den 10'a kadar:   ya da   her öğe için liste'den:"
        }
        "while" | "until" | "loop" => {
            "koşullu döngü için: x 10'dan küçükken:   ya da   x < 10 olduğu sürece:"
        }
        "if" | "eğerki" | "egerki" => {
            "koşul için: eğer x 5'ten büyükse:   ya da   eğer x > 5 ise:"
        }
        "else" | "elif" | "elsif" | "elseif" => {
            "koşul tutmazsa: değilse:   (zincir için: değilse eğer ...:)"
        }
        "def" | "function" | "func" | "fn" | "fun" | "fonksiyon" | "fonk" | "sub" | "proc" => {
            "işlev tanımlamak için: işlev topla(a, b):   (dönüş tipiyle: işlev topla(a, b) -> sayı:)"
        }
        "return" | "döndur" | "dondur" => "işlevden değer döndürmek için: döndür x",
        "var" | "let" | "const" | "değişken" | "degisken" | "dim" | "auto" | "int" | "float"
        | "double" | "string" | "str" | "bool" | "char" => {
            "Orhunca'da değişken tanımlamak için yalnızca: x = 5   (tipiyle: x: sayı = 5)"
        }
        "import" | "include" | "require" | "using" | "from" => {
            "başka bir dosyayı kullanmak için: kullan \"dosya.ohc\""
        }
        "class" | "struct" | "sınıf" | "sinif" | "record" => {
            "kendi veri türünüz için: model Öğrenci:   (alanlarıyla: ad: metin)"
        }
        "try" | "catch" | "except" | "finally" | "rescue" => {
            "hata yakalamak için: dene:  ...  yakala hata:  ..."
        }
        "throw" | "raise" => "hata vermek için: hata_ver(\"mesaj\")",
        "break" => "döngüden çıkmak için: dur",
        "continue" | "next" => "döngünün sonraki adımına geçmek için: sürdür",
        "true" => "Orhunca'da: doğru",
        "false" => "Orhunca'da: yanlış",
        "null" | "none" | "nil" | "undefined" => {
            "boş değer yerine boş_mu(x) ile denetleyin ya da varsayılan bir değer verin"
        }
        "and" => "Orhunca'da: ve",
        "or" => "Orhunca'da: veya",
        "not" => "Orhunca'da: değil",
        "len" | "length" | "size" | "count" => "uzunluğu bulmak için: uzunluk(x)",
        "parseint" | "tam" | "tamsayı" | "tamsayi" | "integer" | "number" => {
            "metni sayıya çevirmek için: sayı(\"42\")"
        }
        "tostring" | "to_string" => "sayıyı metne çevirmek için: metin(42)",
        "range" => "aralık için: her i için 1'den 10'a kadar:",
        "append" | "push" | "add" => "listeye eklemek için: 5'i listeye ekle.",
        "sort" => "sıralamak için: liste'yi sırala.",
        "main" => "Orhunca'da ana işlev gerekmez: dosyanın en üstündeki kod sırayla çalışır",
        _ => return None,
    })
}

/// Türkçe harfler ASCII'ye indirgenir (yanlış klavye düzeni: "eger" → "eğer").
fn sadelestir(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

fn mesafe(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut onceki: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut simdiki = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let maliyet = usize::from(ca != cb);
            simdiki[j + 1] = (onceki[j] + maliyet)
                .min(onceki[j + 1] + 1)
                .min(simdiki[j] + 1);
            // yer değiştirme (ab → ba)
            if i > 0 && j > 0 && a[i] == b[j - 1] && a[i - 1] == b[j] {
                simdiki[j + 1] = simdiki[j + 1].min(onceki[j - 1] + 1);
            }
        }
        onceki = simdiki;
    }
    onceki[b.len()]
}

/// Adaylar arasında `ad`a en çok benzeyen (Türkçe harfler sadeleştirilerek ve
/// küçük yazım farkları göz ardı edilerek); yeterince benzemiyorsa `None`.
pub fn benzer<'a>(ad: &str, adaylar: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let sade = sadelestir(ad);
    let sinir = match sade.chars().count() {
        0..=2 => 0,
        3..=5 => 1,
        _ => 2,
    };
    let mut en_iyi: Option<(usize, &str)> = None;
    for aday in adaylar {
        if aday == ad || aday.starts_with('‹') {
            continue;
        }
        let d = if sadelestir(aday) == sade {
            0
        } else {
            mesafe(&sade, &sadelestir(aday))
        };
        if d <= sinir && en_iyi.is_none_or(|(e, _)| d < e) {
            en_iyi = Some((d, aday));
        }
    }
    en_iyi.map(|(_, a)| a.to_string())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn benzerlik() {
        assert_eq!(benzer("eger", ["eğer", "her"]).as_deref(), Some("eğer"));
        assert_eq!(
            benzer("uzunlk", ["uzunluk", "kırp"]).as_deref(),
            Some("uzunluk")
        );
        assert_eq!(
            benzer("toplm", ["toplam", "sayaç"]).as_deref(),
            Some("toplam")
        );
        assert_eq!(benzer("xyz", ["toplam"]), None);
        assert_eq!(benzer("kitap", ["kırp"]), None);
        assert!(yabanci("print").is_some());
        assert!(yabanci("ayşe").is_none());
    }
}
