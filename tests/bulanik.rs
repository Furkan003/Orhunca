//! Bulanık sınama (fuzz): örnek programlar rastgele bozulur; sözcük çözümleyici,
//! ayrıştırıcı, denetçi, biçimlendirici ve kod üreticiler hiçbir girdide çökmemeli
//! (hata mesajı vermeli). Stüdyo ve dil sunucusu derleyiciyi kendi sürecinde
//! çalıştırdığından buradaki bir çökme düzenleyiciyi de düşürür.
//!
//! Varsayılan: kısa tur (CI). Uzun tur: ORHUNCA_BULANIK=200000 cargo test --release --test bulanik

use orhunca::{ayristirici, bicimlendirici, denetci, sablon, sozcuk, uretici, wasm_uretici};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

/// Küçük, tekrarlanabilir rastgele sayı üreteci (xorshift)
struct Rastgele(u64);
impl Rastgele {
    fn sonraki(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn az(&mut self, n: usize) -> usize {
        (self.sonraki() % n.max(1) as u64) as usize
    }
}

const PARCALAR: &[&str] = &[
    "'",
    "'yı",
    "'a",
    "'dan",
    "'e kadar",
    ":",
    "\n",
    "\n    ",
    "    ",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    ",",
    ".",
    "\"",
    "\\",
    "#",
    "=",
    "==",
    "+",
    "-",
    "*",
    "/",
    "//",
    "%",
    "<",
    ">",
    "->",
    "eğer",
    "ise",
    "değilse",
    "her",
    "için",
    "olduğu sürece",
    "işlev",
    "fiil",
    "döndür",
    "model",
    "seçenek",
    "dene",
    "yakala",
    "arayüz",
    "durum",
    "bileşen",
    "kullan",
    "sabit",
    "yaz",
    "ekle",
    "sırala",
    "ve",
    "veya",
    "değil",
    "doğru",
    "yanlış",
    "boş",
    "sayı",
    "metin",
    "liste<sayı>",
    "sözlük<metin, sayı>",
    "0",
    "-1",
    "9223372036854775807",
    "1.5",
    "1e400",
    "x",
    "ö",
    "düğme(\"a\") tıklanınca:",
    "al \"/\":",
    "@",
    "ﬁ",
    "\u{0}",
    "\t",
    "\r",
    "@eğer",
    "@her",
    "@son",
    "@model",
    "@düzen",
    "@(",
    "@{",
    "<",
    "/>",
    "</p>",
    "@@",
];

fn sablonlar(klasor: &Path, v: &mut Vec<String>) {
    for g in std::fs::read_dir(klasor).unwrap().filter_map(|g| g.ok()) {
        let y = g.path();
        if y.is_dir() {
            sablonlar(&y, v);
        } else if y.extension().is_some_and(|u| u == "ohchtml") {
            v.push(std::fs::read_to_string(y).unwrap());
        }
    }
}

fn ornekler() -> Vec<String> {
    let kok = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut v = Vec::new();
    for klasor in [
        "örnekler",
        "örnekler/arayüz",
        "öz",
        "kütüphaneler/istatistik",
    ] {
        let Ok(d) = std::fs::read_dir(kok.join(klasor)) else {
            continue;
        };
        let mut yollar: Vec<_> = d.filter_map(|g| g.ok().map(|g| g.path())).collect();
        yollar.sort();
        for y in yollar {
            if y.extension().is_some_and(|u| u == "ohc") {
                v.push(std::fs::read_to_string(y).unwrap());
            }
        }
    }
    assert!(v.len() > 20);
    v
}

fn boz(r: &mut Rastgele, kaynak: &str) -> String {
    let mut k: Vec<char> = kaynak.chars().collect();
    for _ in 0..1 + r.az(4) {
        let i = r.az(k.len() + 1);
        match r.az(5) {
            0 if !k.is_empty() => {
                let n = 1 + r.az(12);
                let son = (i + n).min(k.len());
                k.drain(i.min(son)..son);
            }
            1 => {
                let p = PARCALAR[r.az(PARCALAR.len())];
                k.splice(i..i, p.chars());
            }
            2 if !k.is_empty() => {
                // bir parçayı başka yere kopyala
                let a = r.az(k.len());
                let b = (a + 1 + r.az(40)).min(k.len());
                let parca: Vec<char> = k[a..b].to_vec();
                let j = r.az(k.len() + 1);
                k.splice(j..j, parca);
            }
            3 if !k.is_empty() => {
                let j = r.az(k.len());
                k[j] = PARCALAR[r.az(PARCALAR.len())].chars().next().unwrap_or(' ');
            }
            _ => {
                // satırı kopyala ya da sil
                let satirlar: Vec<&str> = kaynak.lines().collect();
                if !satirlar.is_empty() {
                    let s = satirlar[r.az(satirlar.len())];
                    k.splice(i..i, format!("{s}\n").chars());
                }
            }
        }
    }
    k.into_iter().collect()
}

/// Bir girdiyi bütün aşamalardan geçirir; çökerse çöken aşamanın adını döndürür.
fn dene(kaynak: &str) -> Result<(), &'static str> {
    let calis =
        |ad: &'static str, f: &mut dyn FnMut()| catch_unwind(AssertUnwindSafe(f)).map_err(|_| ad);
    calis("biçimlendirici", &mut || {
        let _ = bicimlendirici::bicimlendir(kaynak);
        let _ = bicimlendirici::uyarilar(kaynak);
    })?;
    let mut program = None;
    calis("sözcük/ayrıştırıcı/denetçi", &mut || {
        let Ok(s) = sozcuk::sozcukle(kaynak) else {
            return;
        };
        let Ok(mut p) = ayristirici::ayristir_cok(vec![s], Vec::new()) else {
            return;
        };
        if denetci::denetle(&mut p).is_ok() {
            program = Some(p);
        }
    })?;
    if let Some(p) = program {
        // Arayüz programları yalnızca WebAssembly'ye derlenir (derleme.rs).
        if !p.arayuz_programi() {
            calis("kod üretici", &mut || {
                let isa = uretici::isa_kur(target_lexicon::Triple::host()).unwrap();
                let _ = uretici::uret(&p, isa);
            })?;
        }
        calis("wasm üretici", &mut || {
            let _ = wasm_uretici::uret(&p);
        })?;
    }
    Ok(())
}

#[test]
fn bozuk_girdilerde_cokmez() {
    let tur: usize = std::env::var("ORHUNCA_BULANIK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1500);
    let tohum: u64 = std::env::var("ORHUNCA_BULANIK_TOHUM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5eed_0e7c_a11d);
    let ornekler = ornekler();
    let mut gorunumler = Vec::new();
    sablonlar(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/studyo/sablon_dosyalari"),
        &mut gorunumler,
    );
    assert!(gorunumler.len() > 5);
    // Panik mesajları sınama çıktısını doldurmasın; yalnızca ilk çöken girdi gösterilir.
    let mesajlar = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let m2 = mesajlar.clone();
    std::panic::set_hook(Box::new(move |b| m2.lock().unwrap().push(b.to_string())));
    let mut r = Rastgele(tohum);
    let mut cokenler = Vec::new();
    for i in 0..tur {
        let kaynak = boz(&mut r, &ornekler[i % ornekler.len()]);
        let gorunum = boz(&mut r, &gorunumler[i % gorunumler.len()]);
        let sonuc = dene(&kaynak).and_then(|_| {
            catch_unwind(AssertUnwindSafe(|| {
                let _ = sablon::coz(&sablon::SablonKaynagi {
                    ad: "g".into(),
                    dosya: 0,
                    kaynak: gorunum.clone(),
                });
            }))
            .map_err(|_| "şablon")
        });
        if let Err(asama) = sonuc {
            let kaynak = if asama == "şablon" { gorunum } else { kaynak };
            cokenler.push((asama, kaynak));
            if cokenler.len() >= 5 {
                break;
            }
        }
    }
    let _ = std::panic::take_hook();
    if let Some((asama, kaynak)) = cokenler.first() {
        let yol = std::env::temp_dir().join("orhunca-bulanik-coken.ohc");
        std::fs::write(&yol, kaynak).unwrap();
        panic!(
            "{} girdi çöktü; ilki {asama} aşamasında, girdi: {}\n{}",
            cokenler.len(),
            yol.display(),
            mesajlar.lock().unwrap().join("\n")
        );
    }
}
