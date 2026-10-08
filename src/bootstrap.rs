//! Bootstrap desteği (.ohchtml görünümleri).
//!
//! - `@bootstrap` (düzenin `<head>` bölümünde): Bootstrap 5'in CSS ve JavaScript dosyalarını
//!   bütünlük özetleriyle (SRI) sayfaya ekler.
//! - `sınıf="düğme düğme-birincil"`: Türkçe Bootstrap sınıfları çevrilir ve `class` olur
//!   (`class="btn btn-primary"`). Bilinmeyen sınıflar olduğu gibi kalır; `class="..."` hiç
//!   değiştirilmez (İngilizce sınıflar ve projenin kendi sınıfları için).

pub const SURUM: &str = "5.3.3";

/// `@bootstrap` yönergesinin yerine yazılan HTML.
pub const BASLIK: &str = "<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css\" integrity=\"sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH\" crossorigin=\"anonymous\">\n    <script defer src=\"https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js\" integrity=\"sha384-YvpcrYf0tY3lHB60NNkmXc5s9fDVZLESaAA55NDzOxhy9GkcIdslK1eN7N6jIeHz\" crossorigin=\"anonymous\"></script>";

/// Tek başına çevrilen sınıflar (yardımcı sınıflar ve parçalardan çıkmayanlar).
const TAM: &[(&str, &str)] = &[
    ("esnek", "d-flex"),
    ("gizli", "d-none"),
    ("blok", "d-block"),
    ("kalın", "fw-bold"),
    ("eğik", "fst-italic"),
    ("metin-orta", "text-center"),
    ("metin-sol", "text-start"),
    ("metin-sağ", "text-end"),
    ("genişlik-100", "w-100"),
    ("gölge", "shadow"),
    ("gölge-küçük", "shadow-sm"),
    ("gölge-büyük", "shadow-lg"),
    ("yuvarlak", "rounded"),
    ("kart-üst", "card-header"),
    ("kart-alt", "card-footer"),
    ("kart-resim-üst", "card-img-top"),
    ("menü", "nav"),
    ("menü-bağlantı", "nav-link"),
    ("menü-öğe", "nav-item"),
    ("gezinme-açılır", "navbar-collapse"),
    ("gezinme-düğme", "navbar-toggler"),
    ("form-denetim", "form-control"),
    ("form-onay-giriş", "form-check-input"),
    ("form-onay-etiket", "form-check-label"),
    ("liste-grup", "list-group"),
    ("liste-grup-öğe", "list-group-item"),
    ("ilerleme-çubuk", "progress-bar"),
    ("satır-ortala", "align-items-center"),
    ("iki-yana", "justify-content-between"),
    ("ortala", "justify-content-center"),
    ("büyük-yazı", "lead"),
    ("ekran-1", "display-1"),
    ("ekran-4", "display-4"),
];

/// Sınıf adının parçaları: `düğme-çerçeve-tehlike` → `btn-outline-danger`.
const PARCALAR: &[(&str, &str)] = &[
    ("düğme", "btn"),
    ("kapsayıcı", "container"),
    ("akışkan", "fluid"),
    ("satır", "row"),
    ("sütun", "col"),
    ("kart", "card"),
    ("gövde", "body"),
    ("başlık", "title"),
    ("metin", "text"),
    ("uyarı", "alert"),
    ("rozet", "badge"),
    ("tablo", "table"),
    ("çizgili", "striped"),
    ("kenarlı", "bordered"),
    ("üzerinde", "hover"),
    ("gezinme", "navbar"),
    ("marka", "brand"),
    ("bağlantı", "link"),
    ("form", "form"),
    ("etiket", "label"),
    ("seçim", "select"),
    ("onay", "check"),
    ("giriş", "input"),
    ("grup", "group"),
    ("sayfalama", "pagination"),
    ("ilerleme", "progress"),
    ("kenarlık", "border"),
    ("arka", "bg"),
    ("birincil", "primary"),
    ("ikincil", "secondary"),
    ("başarı", "success"),
    ("tehlike", "danger"),
    ("dikkat", "warning"),
    ("bilgi", "info"),
    ("açık", "light"),
    ("koyu", "dark"),
    ("beyaz", "white"),
    ("siyah", "black"),
    ("çerçeve", "outline"),
    ("çerçeveli", "outline"),
    ("küçük", "sm"),
    ("orta", "md"),
    ("büyük", "lg"),
    ("geniş", "xl"),
    ("kapat", "close"),
];

/// İngilizcesi olduğu gibi kalan parçalar (kırılma noktaları, sayılar).
fn degismez(p: &str) -> bool {
    matches!(p, "sm" | "md" | "lg" | "xl" | "xxl" | "auto") || p.chars().all(|c| c.is_ascii_digit())
}

/// Tek bir Türkçe sınıf adının Bootstrap karşılığı; bilinmiyorsa kendisi.
pub fn sinif(ad: &str) -> String {
    if let Some((_, k)) = TAM.iter().find(|(t, _)| *t == ad) {
        return k.to_string();
    }
    let mut cevrilen = Vec::new();
    let mut turkce = false;
    for p in ad.split('-') {
        if let Some((_, k)) = PARCALAR.iter().find(|(t, _)| *t == p) {
            cevrilen.push(*k);
            turkce = true;
        } else if degismez(p) {
            cevrilen.push(p);
        } else {
            return ad.to_string();
        }
    }
    if turkce {
        cevrilen.join("-")
    } else {
        ad.to_string()
    }
}

/// Görünümün HTML metnindeki `sınıf="..."` özniteliklerini `class="..."`a çevirir.
pub fn siniflari_cevir(metin: &str) -> String {
    const OZNITELIK: &str = "sınıf=\"";
    if !metin.contains(OZNITELIK) {
        return metin.to_string();
    }
    let mut sonuc = String::with_capacity(metin.len());
    let mut kalan = metin;
    while let Some(i) = kalan.find(OZNITELIK) {
        // Öznitelik adının başı: önünde boşluk olmalı (ör. `bilgisınıf="` değil)
        let once = &kalan[..i];
        if !once.ends_with(char::is_whitespace) {
            sonuc.push_str(&kalan[..i + OZNITELIK.len()]);
            kalan = &kalan[i + OZNITELIK.len()..];
            continue;
        }
        sonuc.push_str(once);
        sonuc.push_str("class=\"");
        let govde = &kalan[i + OZNITELIK.len()..];
        // Öznitelik bu parçada kapanmıyorsa (içinde @değer var) sondaki yarım kelime çevrilmez.
        let (deger, kapandi) = match govde.find('"') {
            Some(j) => (&govde[..j], true),
            None => (govde, false),
        };
        let mut parcalar: Vec<&str> = deger.split(' ').collect();
        let yarim = if !kapandi && !deger.ends_with(' ') {
            parcalar.pop()
        } else {
            None
        };
        let cevrilen: Vec<String> = parcalar
            .iter()
            .map(|p| {
                if p.is_empty() {
                    String::new()
                } else {
                    sinif(p)
                }
            })
            .collect();
        sonuc.push_str(&cevrilen.join(" "));
        if let Some(y) = yarim {
            if !cevrilen.is_empty() {
                sonuc.push(' ');
            }
            sonuc.push_str(y);
        }
        kalan = &govde[deger.len()..];
    }
    sonuc.push_str(kalan);
    sonuc
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn siniflar() {
        assert_eq!(sinif("düğme"), "btn");
        assert_eq!(sinif("düğme-birincil"), "btn-primary");
        assert_eq!(sinif("düğme-çerçeve-tehlike"), "btn-outline-danger");
        assert_eq!(sinif("düğme-büyük"), "btn-lg");
        assert_eq!(sinif("sütun-md-6"), "col-md-6");
        assert_eq!(sinif("sütun-orta-6"), "col-md-6");
        assert_eq!(sinif("kapsayıcı-akışkan"), "container-fluid");
        assert_eq!(sinif("kart-gövde"), "card-body");
        assert_eq!(sinif("arka-koyu"), "bg-dark");
        assert_eq!(sinif("metin-orta"), "text-center");
        assert_eq!(sinif("form-denetim"), "form-control");
        // İngilizce ve bilinmeyenler olduğu gibi
        assert_eq!(sinif("btn-primary"), "btn-primary");
        assert_eq!(sinif("benim-sınıfım"), "benim-sınıfım");
        assert_eq!(sinif("mt-3"), "mt-3");
    }

    #[test]
    fn oznitelik() {
        assert_eq!(
            siniflari_cevir("<a sınıf=\"düğme düğme-birincil mt-3\" href=\"/\">"),
            "<a class=\"btn btn-primary mt-3\" href=\"/\">"
        );
        assert_eq!(
            siniflari_cevir("<p class=\"düğme\">"),
            "<p class=\"düğme\">"
        );
        // Değerin ortasında @ifade: sondaki yarım kelime dokunulmadan kalır.
        assert_eq!(
            siniflari_cevir("<div sınıf=\"uyarı uyarı-"),
            "<div class=\"alert uyarı-"
        );
        assert_eq!(siniflari_cevir("<div sınıf=\"kart "), "<div class=\"card ");
    }
}
