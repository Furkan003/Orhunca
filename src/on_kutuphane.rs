//! Standart kütüphanenin Orhunca ile yazılmış parçası (runtime/ön_kütüphane.ohc).
//!
//! Dosya ayrı ayrıştırılır (isimleri programın ek çözümleme sözlüğüne karışmaz) ve
//! işlevleri `‹öz›` önekiyle programa eklenir. Denetçi `kırp`, `böl`, `değiştir` gibi
//! yerleşik çağrıları bu işlevlere bağlar; yalnızca kullanılanlar derlenir. Dosyanın
//! içindeki çağrılar programın tanımlarına değil, doğrudan yerleşiklere ve dosyanın
//! kendi işlevlerine gider.

use crate::agac::*;
use std::sync::OnceLock;

pub const KAYNAK: &str = include_str!("../runtime/ön_kütüphane.ohc");

/// Ön kütüphane işlevlerinin adlarının öneki (kaynakta yazılamaz).
pub const ON_EK: &str = "‹öz›";
/// Ön kütüphanedeki yerleşik işlev çağrılarının öneki: programın aynı adlı
/// tanımları bu çağrıları gölgelemez.
pub const YERLESIK_ON_EK: &str = "‹yerleşik›";
/// `hata_ver_satırda(mesaj, satır)`: çağıranın satırıyla çalışma hatası.
pub const HATA_SATIRDA: &str = "‹hata_satırda›";
/// Ön kütüphanenin derlemedeki dosya sırası (hata gösterimi için).
pub const DOSYA: usize = usize::MAX - 1;

/// Yerleşik çağrının ön kütüphanedeki karşılığı: (işlev, çağıranın satırı eklenir mi).
pub fn esle(ad: &str, tipler: &[Tip]) -> Option<(&'static str, bool)> {
    use Tip::*;
    Some(match (ad, tipler) {
        ("kırp", [Metin]) => ("kırp", false),
        ("başlar", [Metin, Metin]) => ("başlar", false),
        ("biter", [Metin, Metin]) => ("biter", false),
        ("böl", [Metin, Metin]) => ("böl", false),
        ("satırlar", [Metin]) => ("satırlar", false),
        ("bul", [Metin, Metin]) => ("metin_bul", false),
        ("içerir", [Metin, Metin]) => ("metin_içerir", false),
        ("değiştir", [Metin, Metin, Metin]) => ("değiştir", false),
        ("tekrarla", [Metin, Sayi]) => ("tekrarla", true),
        ("ters", [Metin]) => ("metin_ters", false),
        ("büyük_harf", [Metin]) => ("büyük_harf", false),
        ("küçük_harf", [Metin]) => ("küçük_harf", false),
        ("kaçır", [_]) => ("kaçır", false),
        ("url_kodla", [Metin]) => ("url_kodla", false),
        _ => return None,
    })
}

/// Ön kütüphanenin işlevleri (adları önekli, çağrıları bağlanmış).
pub fn islevler() -> Vec<Islev> {
    static ISLEVLER: OnceLock<Vec<Islev>> = OnceLock::new();
    ISLEVLER
        .get_or_init(|| {
            let mut sozcukler = crate::sozcuk::sozcukle(KAYNAK).expect("ön kütüphane sözcükleri");
            for s in sozcukler.iter_mut() {
                s.konum.dosya = DOSYA;
            }
            let p = crate::ayristirici::ayristir_cok(vec![sozcukler], Vec::new())
                .unwrap_or_else(|h| panic!("ön kütüphane ayrıştırılamadı: {h}"));
            let adlar: Vec<String> = p.islevler.iter().map(|f| f.ad.clone()).collect();
            p.islevler
                .into_iter()
                .map(|mut f| {
                    f.ad = format!("{ON_EK}{}", f.ad);
                    ifadeleri_gez(&mut f.govde, &mut |e| {
                        if let IfadeTuru::Cagri(ad, _) = &mut e.tur {
                            *ad = if adlar.contains(ad) {
                                format!("{ON_EK}{ad}")
                            } else if ad == "hata_ver_satırda" {
                                HATA_SATIRDA.to_string()
                            } else {
                                format!("{YERLESIK_ON_EK}{ad}")
                            };
                        }
                    });
                    f
                })
                .collect()
        })
        .clone()
}

pub fn on_kutuphane_mi(ad: &str) -> bool {
    ad.starts_with(ON_EK)
}
