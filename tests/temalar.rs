//! temalar/ (Stüdyo'nun topluluk galerisi): dizindeki her tema dosyası var, geçerli ve küçük.

#[test]
fn galeri_gecerli() {
    let kok = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("temalar");
    let dizin: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(kok.join("dizin.json")).unwrap()).unwrap();
    let temalar = dizin["temalar"].as_array().unwrap();
    assert!(!temalar.is_empty());
    for g in temalar {
        let dosya = g["dosya"].as_str().unwrap();
        assert!(
            dosya.ends_with(".ohctema")
                && dosya
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.'),
            "{dosya}: adda yalnızca küçük harf, rakam ve - olmalı"
        );
        let metin = std::fs::read_to_string(kok.join(dosya)).unwrap();
        assert!(metin.len() < 2 * 1024 * 1024, "{dosya} 2 MB'tan büyük");
        let t: serde_json::Value = serde_json::from_str(&metin).unwrap();
        assert_eq!(t["orhunca_tema"], 1, "{dosya}");
        assert_eq!(t["ad"], g["ad"], "{dosya}: dizindeki ad farklı");
        if let Some(k) = t["arka_plan"]["kaynak"].as_str() {
            assert!(
                k.starts_with("data:image/") && k.contains(";base64,"),
                "{dosya}"
            );
        }
        assert!(
            !t["ozel_css"].as_str().unwrap_or("").contains("@import"),
            "{dosya}"
        );
    }
}
