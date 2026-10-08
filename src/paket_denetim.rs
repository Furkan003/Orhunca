//! Paket güvenliği: bir paketin kullandığı izinler ve içeriğinin özeti.
//!
//! İzinler derleyici tarafından koddan çıkarılır (paketin kendi beyanına güvenilmez):
//! dosya okuma/yazma, internete bağlanma, ortam değişkenleri, C kütüphanesi çağırma ve
//! web sunucusu. Paket eklenirken ve yeni izin isteyen bir güncellemede kullanıcıya
//! gösterilip onayı alınır. İçerik özeti (SHA-256) paket dizinindeki kayıtla
//! karşılaştırılır; böylece yayımlandıktan sonra değiştirilen bir paket kurulmaz.

use crate::sozcuk::{sozcukle, Tok};
use std::collections::BTreeSet;
use std::path::Path;

/// Bir paketin isteyebileceği izinler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Izin {
    Dosya,
    Ag,
    Ortam,
    CKutuphanesi,
    Sunucu,
}

pub const IZINLER: &[Izin] = &[
    Izin::Dosya,
    Izin::Ag,
    Izin::Ortam,
    Izin::CKutuphanesi,
    Izin::Sunucu,
];

impl Izin {
    /// Kilit dosyasında ve paket dizininde kullanılan ad.
    pub fn ad(self) -> &'static str {
        match self {
            Izin::Dosya => "dosya",
            Izin::Ag => "ağ",
            Izin::Ortam => "ortam",
            Izin::CKutuphanesi => "c_kütüphanesi",
            Izin::Sunucu => "sunucu",
        }
    }

    pub fn aciklama(self) -> &'static str {
        match self {
            Izin::Dosya => "bilgisayardaki dosyaları okuyup yazabilir",
            Izin::Ag => "internete bağlanabilir (veri gönderip alabilir)",
            Izin::Ortam => "ortam değişkenlerini okuyabilir (.env'deki şifreler ve anahtarlar dahil)",
            Izin::CKutuphanesi => {
                "C kütüphanesi çağırabilir (bilgisayarda her şeyi yapabilir; yalnızca güvendiğiniz paketlere verin)"
            }
            Izin::Sunucu => "web sunucusu başlatabilir",
        }
    }

    pub fn coz(ad: &str) -> Option<Izin> {
        IZINLER.iter().copied().find(|i| i.ad() == ad)
    }
}

fn kelime_izni(k: &str) -> Option<Izin> {
    Some(match k {
        "dosya_oku" | "dosya_sil" | "dosya_taşı" | "dosya_var" | "dosyaya_ekle" | "dosyaya_yaz"
        | "csv_oku" | "csv_yaz" | "dosyaya" | "kaydet" => Izin::Dosya,
        "http_al" | "http_gönder" | "json_al" => Izin::Ag,
        "ortam" => Izin::Ortam,
        "sun" => Izin::Sunucu,
        _ => return None,
    })
}

/// Bir kaynak dosyanın kullandığı izinler.
pub fn kaynak_izinleri(kaynak: &str, izinler: &mut BTreeSet<Izin>) {
    let Ok(s) = sozcukle(kaynak) else {
        // Çözümlenemeyen dosyada kelimeler tek tek aranır (fazla izin, eksik izinden iyidir).
        metin_izinleri(kaynak, izinler);
        return;
    };
    for (i, w) in s.iter().enumerate() {
        let Tok::Kelime(k) = &w.tok else { continue };
        if let Some(iz) = kelime_izni(k) {
            izinler.insert(iz);
        }
        let sonraki = s.get(i + 1).map(|x| &x.tok);
        if k == "kütüphane" && matches!(sonraki, Some(Tok::Metin(_))) {
            izinler.insert(Izin::CKutuphanesi);
        }
        // Web yolları: al "/x": · gönder "/x": · koy · sil
        if matches!(k.as_str(), "al" | "gönder" | "koy")
            && matches!(sonraki, Some(Tok::Metin(m)) if m.starts_with('/'))
        {
            izinler.insert(Izin::Sunucu);
        }
    }
}

/// .ohchtml gibi Orhunca sözcüklerine ayrılamayan dosyalar için.
fn metin_izinleri(metin: &str, izinler: &mut BTreeSet<Izin>) {
    for k in metin.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if let Some(iz) = kelime_izni(k) {
            izinler.insert(iz);
        }
    }
    if metin.contains("kütüphane \"") {
        izinler.insert(Izin::CKutuphanesi);
    }
}

/// Paket klasöründeki kaynak dosyalar (gizli dosyalar, derleme çıktıları ve paketin kendi
/// bağımlılıkları hariç), göreli yollarıyla ve sıralı.
fn dosyalar(klasor: &Path) -> Vec<(String, std::path::PathBuf)> {
    fn gez(k: &Path, kok: &Path, l: &mut Vec<(String, std::path::PathBuf)>) {
        let Ok(g) = std::fs::read_dir(k) else { return };
        for x in g.flatten() {
            let ad = x.file_name().to_string_lossy().into_owned();
            let Ok(tur) = x.file_type() else { continue };
            if ad.starts_with('.') || tur.is_symlink() {
                continue;
            }
            let p = x.path();
            if k == kok && matches!(ad.as_str(), "paketler" | "cikti" | "target" | "veri") {
                continue;
            }
            if tur.is_dir() {
                gez(&p, kok, l);
            } else {
                let goreli = p
                    .strip_prefix(kok)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                l.push((goreli, p));
            }
        }
    }
    let mut l = Vec::new();
    gez(klasor, klasor, &mut l);
    l.sort();
    l
}

/// Paket klasörünün kullandığı izinler.
pub fn izinler(klasor: &Path) -> BTreeSet<Izin> {
    let mut iz = BTreeSet::new();
    for (ad, yol) in dosyalar(klasor) {
        let Ok(m) = std::fs::read_to_string(&yol) else {
            continue;
        };
        if ad.ends_with(".ohc") {
            kaynak_izinleri(&m, &mut iz);
        } else if ad.ends_with(".ohchtml") {
            metin_izinleri(&m, &mut iz);
        }
    }
    iz
}

/// İzinlerin kilit dosyasındaki ve paket dizinindeki yazımı: "dosya, ağ".
pub fn izin_metni(iz: &BTreeSet<Izin>) -> String {
    iz.iter().map(|i| i.ad()).collect::<Vec<_>>().join(", ")
}

pub fn izinleri_coz(m: &str) -> BTreeSet<Izin> {
    m.split(',').filter_map(|a| Izin::coz(a.trim())).collect()
}

/// Paket içeriğinin özeti: dosyaların göreli yolları ve içerikleri (satır sonları
/// birleştirilerek, böylece Windows'ta ve Linux'ta aynı) üzerinden SHA-256.
pub fn icerik_ozeti(klasor: &Path) -> String {
    let mut veri = Vec::new();
    for (ad, yol) in dosyalar(klasor) {
        let Ok(icerik) = std::fs::read(&yol) else {
            continue;
        };
        let icerik: Vec<u8> = if std::str::from_utf8(&icerik).is_ok() {
            String::from_utf8_lossy(&icerik)
                .replace("\r\n", "\n")
                .into_bytes()
        } else {
            icerik
        };
        veri.extend_from_slice(ad.as_bytes());
        veri.push(0);
        veri.extend_from_slice(icerik.len().to_string().as_bytes());
        veri.push(0);
        veri.extend_from_slice(&icerik);
    }
    let ozet = crate::guncelleme::sha256(&veri);
    format!(
        "sha256:{}",
        ozet.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    fn iz(kaynak: &str) -> String {
        let mut s = BTreeSet::new();
        kaynak_izinleri(kaynak, &mut s);
        izin_metni(&s)
    }

    #[test]
    fn izin_cikarimi() {
        assert_eq!(iz("işlev topla(a, b):\n    döndür a + b\n"), "");
        assert_eq!(iz("x = dosya_oku(\"a.txt\")\n"), "dosya");
        assert_eq!(iz("y = http_al(\"https://x\")\n"), "ağ");
        assert_eq!(iz("a = ortam(\"ANAHTAR\")\n"), "ortam");
        assert_eq!(
            iz("kütüphane \"m\":\n    işlev sqrt(x: ondalık) -> ondalık\n"),
            "c_kütüphanesi"
        );
        assert_eq!(iz("al \"/\":\n    döndür \"merhaba\"\n"), "sunucu");
        // Metin içindeki kelimeler izin sayılmaz
        assert_eq!(iz("\"dosya_oku ve http_al\"'ı yaz.\n"), "");
        assert_eq!(
            izinleri_coz("dosya, ağ, bilinmeyen"),
            [Izin::Dosya, Izin::Ag].into_iter().collect()
        );
    }

    #[test]
    fn ozet_satir_sonundan_bagimsiz() {
        let k = std::env::temp_dir().join(format!("orhunca-ozet-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&k);
        std::fs::create_dir_all(k.join("alt")).unwrap();
        std::fs::write(k.join("a.ohc"), "1'i yaz.\n2'yi yaz.\n").unwrap();
        std::fs::write(k.join("alt/b.ohc"), "x = 1\n").unwrap();
        std::fs::write(k.join(".orhunca-isleme"), "abc").unwrap();
        let ilk = icerik_ozeti(&k);
        assert!(ilk.starts_with("sha256:") && ilk.len() == 71, "{ilk}");
        std::fs::write(k.join("a.ohc"), "1'i yaz.\r\n2'yi yaz.\r\n").unwrap();
        std::fs::write(k.join(".orhunca-isleme"), "baska").unwrap();
        assert_eq!(icerik_ozeti(&k), ilk);
        std::fs::write(k.join("alt/b.ohc"), "x = 2\n").unwrap();
        assert_ne!(icerik_ozeti(&k), ilk);
        std::fs::remove_dir_all(&k).unwrap();
    }
}
