//! Stüdyo arayüz dosyalarını (studio/) ve proje şablonlarını ikili dosyanın içine gömer.

use std::path::Path;

fn topla(kok: &Path, klasor: &Path, cikti: &mut Vec<(String, String)>) {
    let mut girdiler: Vec<_> = std::fs::read_dir(klasor)
        .unwrap()
        .filter_map(|g| g.ok())
        .collect();
    girdiler.sort_by_key(|g| g.path());
    for g in girdiler {
        let yol = g.path();
        if yol.is_dir() {
            topla(kok, &yol, cikti);
        } else {
            let goreli = yol
                .strip_prefix(kok)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            cikti.push((
                goreli,
                yol.canonicalize().unwrap().to_string_lossy().into_owned(),
            ));
        }
    }
}

/// `klasor` altındaki dosyaları `DOSYALAR` dizisi olarak `OUT_DIR/cikti` dosyasına yazar.
fn gom(klasor: &str, cikti: &str) {
    let kok = Path::new(env!("CARGO_MANIFEST_DIR")).join(klasor);
    println!("cargo:rerun-if-changed={klasor}");
    let mut dosyalar = Vec::new();
    topla(&kok, &kok, &mut dosyalar);
    let mut kod = String::from("pub const DOSYALAR: &[(&str, &[u8])] = &[\n");
    for (ad, yol) in dosyalar {
        kod.push_str(&format!("    ({ad:?}, include_bytes!({yol:?})),\n"));
    }
    kod.push_str("];\n");
    let cikti = Path::new(&std::env::var("OUT_DIR").unwrap()).join(cikti);
    std::fs::write(cikti, kod).unwrap();
}

fn main() {
    gom("studio", "studio_dosyalari.rs");
    // Stüdyo'nun yeni proje şablonlarının dosyaları
    gom("src/studyo/sablon_dosyalari", "sablon_dosyalari.rs");
}
