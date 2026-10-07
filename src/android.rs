//! `orhunca paketle --hedef android`: arayüz programını Android uygulamasına (.apk)
//! dönüştürür. Android SDK gerekmez: hazır kabuk (mobil/android, gömülü olarak
//! runtime/kabuk/android.okb) kullanılır; uygulama adı, paket kimliği ve sürüm
//! manifestte değiştirilir, sayfa assets/uygulama.html olarak eklenir ve APK, APK
//! İmza Şeması v2 ile (ECDSA P-256) imzalanır.
//!
//! İmza anahtarı ilk paketlemede üretilip ayar klasöründe saklanır
//! (`android-imza.anahtar`); uygulamanın güncellemeleri aynı anahtarla imzalanmalıdır.

use crate::guncelleme::sha256;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use std::path::{Path, PathBuf};

const KABUK: &[u8] = include_bytes!("../runtime/kabuk/android.okb");

const YER_PAKET: &str = "org.orhunca.yertutucu";
const YER_AD: &str = "ORHUNCA_UYGULAMA_ADI_YER_TUTUCU";
const YER_SURUM: &str = "0.0.0-yertutucu";

/// Paketleme seçenekleri.
pub struct Uygulama<'a> {
    /// Ana ekranda görünen ad
    pub ad: &'a str,
    /// Paket kimliği (ör. org.orhunca.sayac); her uygulama için tekil olmalı
    pub kimlik: &'a str,
    pub surum: &'a str,
    /// Sayfa (derleme::web_sayfasi)
    pub sayfa: &'a [u8],
    /// Uygulama simgesi (PNG); verilmezse Orhunca simgesi
    pub simge: Option<&'a [u8]>,
}

/// Program adından geçerli bir paket kimliği: `org.orhunca.<ad>` (yalnızca a-z, 0-9, _).
pub fn varsayilan_kimlik(ad: &str) -> String {
    let mut s: String = ad
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' | 'à' | 'á' => 'a',
            'î' => 'i',
            'û' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '_',
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, 'u');
    }
    format!("org.orhunca.{s}")
}

/// Paket kimliği Android kurallarına uygun mu? (en az iki parça, harfle başlayan)
pub fn kimlik_gecerli_mi(k: &str) -> bool {
    let parcalar: Vec<&str> = k.split('.').collect();
    parcalar.len() >= 2
        && parcalar.iter().all(|p| {
            p.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

pub fn paketle(u: &Uygulama, cikti: &Path) -> Result<(), String> {
    if !kimlik_gecerli_mi(u.kimlik) {
        return Err(format!(
            "geçersiz paket kimliği '{}' (ör. org.orhunca.sayac: noktayla ayrılmış, harfle başlayan parçalar)",
            u.kimlik
        ));
    }
    let mut dosyalar = kabuk_dosyalari()?;
    for (ad, veri) in dosyalar.iter_mut() {
        if ad == "AndroidManifest.xml" {
            *veri = manifest_degistir(veri, &|s: &str| {
                if s == YER_AD {
                    Some(u.ad.to_string())
                } else if s == YER_SURUM {
                    Some(u.surum.to_string())
                } else {
                    s.strip_prefix(YER_PAKET)
                        .map(|k| format!("{}{k}", u.kimlik))
                }
            })?;
        } else if ad == "assets/uygulama.html" {
            *veri = u.sayfa.to_vec();
        } else if let (Some(simge), true) =
            (u.simge, ad.starts_with("res/") && ad.ends_with(".png"))
        {
            *veri = simge.to_vec();
        }
    }
    let zip = zip_yaz(&dosyalar);
    let anahtar = imza_anahtari()?;
    let apk = imzala(&zip, &anahtar)?;
    std::fs::write(cikti, apk).map_err(|e| format!("'{}' yazılamadı: {e}", cikti.display()))
}

fn kabuk_dosyalari() -> Result<Vec<(String, Vec<u8>)>, String> {
    let bozuk = || "gömülü Android kabuğu bozuk".to_string();
    let mut v = KABUK.strip_prefix(b"OHCAPK1\n").ok_or_else(bozuk)?;
    let mut sonuc = Vec::new();
    let oku = |v: &mut &[u8]| -> Result<Vec<u8>, String> {
        if v.len() < 4 {
            return Err(bozuk());
        }
        let n = u32::from_le_bytes(v[..4].try_into().unwrap()) as usize;
        if v.len() < 4 + n {
            return Err(bozuk());
        }
        let d = v[4..4 + n].to_vec();
        *v = &v[4 + n..];
        Ok(d)
    };
    while !v.is_empty() {
        let ad = String::from_utf8(oku(&mut v)?).map_err(|_| bozuk())?;
        let veri = oku(&mut v)?;
        sonuc.push((ad, veri));
    }
    Ok(sonuc)
}

// ---------------------------------------------------------------------------
// İkili XML (AndroidManifest.xml): dizgi havuzundaki dizgiler değiştirilir.
// Öğeler dizgilere sırayla (indisle) başvurduğu için yalnızca havuz yeniden yazılır.
// ---------------------------------------------------------------------------

fn u16_oku(v: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([v[i], v[i + 1]])
}
fn u32_oku(v: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(v[i..i + 4].try_into().unwrap())
}

pub fn manifest_degistir(
    xml: &[u8],
    degistir: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<u8>, String> {
    let bozuk = || "Android manifesti okunamadı".to_string();
    if xml.len() < 36 || u16_oku(xml, 0) != 0x0003 || u16_oku(xml, 8) != 0x0001 {
        return Err(bozuk());
    }
    let havuz_bas = 8;
    let baslik_boyu = u16_oku(xml, havuz_bas + 2) as usize;
    let havuz_boyu = u32_oku(xml, havuz_bas + 4) as usize;
    let sayi = u32_oku(xml, havuz_bas + 8) as usize;
    let bicem_sayisi = u32_oku(xml, havuz_bas + 12);
    let bayraklar = u32_oku(xml, havuz_bas + 16);
    let dizgi_bas = u32_oku(xml, havuz_bas + 20) as usize;
    let utf8 = bayraklar & 0x100 != 0;
    if bicem_sayisi != 0 || havuz_bas + havuz_boyu > xml.len() {
        return Err(bozuk());
    }
    let veri = &xml[havuz_bas + dizgi_bas..havuz_bas + havuz_boyu];
    let mut dizgiler = Vec::with_capacity(sayi);
    for i in 0..sayi {
        let k = u32_oku(xml, havuz_bas + baslik_boyu + 4 * i) as usize;
        dizgiler.push(if utf8 {
            utf8_dizgi(veri, k).ok_or_else(bozuk)?
        } else {
            utf16_dizgi(veri, k).ok_or_else(bozuk)?
        });
    }
    // Yeni havuz
    let mut yeni_veri = Vec::new();
    let mut konumlar = Vec::with_capacity(sayi);
    for d in &dizgiler {
        let d = degistir(d).unwrap_or_else(|| d.clone());
        konumlar.push(yeni_veri.len() as u32);
        if utf8 {
            uzunluk8(&mut yeni_veri, d.chars().map(char::len_utf16).sum());
            uzunluk8(&mut yeni_veri, d.len());
            yeni_veri.extend_from_slice(d.as_bytes());
            yeni_veri.push(0);
        } else {
            let birimler: Vec<u16> = d.encode_utf16().collect();
            let n = birimler.len();
            if n > 0x7fff {
                yeni_veri.extend_from_slice(&((((n >> 16) as u16) | 0x8000).to_le_bytes()));
            }
            yeni_veri.extend_from_slice(&((n & 0xffff) as u16).to_le_bytes());
            for b in birimler {
                yeni_veri.extend_from_slice(&b.to_le_bytes());
            }
            yeni_veri.extend_from_slice(&[0, 0]);
        }
    }
    while yeni_veri.len() % 4 != 0 {
        yeni_veri.push(0);
    }
    let yeni_dizgi_bas = baslik_boyu + 4 * sayi;
    let yeni_havuz_boyu = yeni_dizgi_bas + yeni_veri.len();
    let mut havuz = Vec::with_capacity(yeni_havuz_boyu);
    havuz.extend_from_slice(&xml[havuz_bas..havuz_bas + baslik_boyu]);
    havuz[4..8].copy_from_slice(&(yeni_havuz_boyu as u32).to_le_bytes());
    havuz[20..24].copy_from_slice(&(yeni_dizgi_bas as u32).to_le_bytes());
    havuz[24..28].copy_from_slice(&0u32.to_le_bytes());
    for k in konumlar {
        havuz.extend_from_slice(&k.to_le_bytes());
    }
    havuz.extend_from_slice(&yeni_veri);

    let mut sonuc = Vec::with_capacity(xml.len() + havuz.len());
    sonuc.extend_from_slice(&xml[..havuz_bas]);
    sonuc.extend_from_slice(&havuz);
    sonuc.extend_from_slice(&xml[havuz_bas + havuz_boyu..]);
    let toplam = sonuc.len() as u32;
    sonuc[4..8].copy_from_slice(&toplam.to_le_bytes());
    Ok(sonuc)
}

fn utf16_dizgi(v: &[u8], k: usize) -> Option<String> {
    let mut n = *v
        .get(k..k + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .as_ref()? as usize;
    let mut i = k + 2;
    if n & 0x8000 != 0 {
        let alt = u16::from_le_bytes([*v.get(i)?, *v.get(i + 1)?]) as usize;
        n = ((n & 0x7fff) << 16) | alt;
        i += 2;
    }
    let birimler: Vec<u16> = v
        .get(i..i + 2 * n)?
        .chunks(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    String::from_utf16(&birimler).ok()
}

fn utf8_dizgi(v: &[u8], k: usize) -> Option<String> {
    let mut i = k;
    let uzunluk = |i: &mut usize| -> Option<usize> {
        let a = *v.get(*i)? as usize;
        *i += 1;
        if a & 0x80 != 0 {
            let b = *v.get(*i)? as usize;
            *i += 1;
            Some(((a & 0x7f) << 8) | b)
        } else {
            Some(a)
        }
    };
    uzunluk(&mut i)?; // UTF-16 uzunluğu
    let n = uzunluk(&mut i)?;
    String::from_utf8(v.get(i..i + n)?.to_vec()).ok()
}

fn uzunluk8(v: &mut Vec<u8>, n: usize) {
    if n > 0x7f {
        v.push(((n >> 8) as u8) | 0x80);
    }
    v.push((n & 0xff) as u8);
}

// ---------------------------------------------------------------------------
// ZIP (sıkıştırmasız; her dosyanın verisi 4 bayt hizalı: resources.arsc için zorunlu)
// ---------------------------------------------------------------------------

fn crc32(v: &[u8]) -> u32 {
    static TABLO: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = TABLO.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, g) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb88320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *g = c;
        }
        t
    });
    !v.iter().fold(!0u32, |c, &b| {
        t[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8)
    })
}

fn zip_yaz(dosyalar: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut v = Vec::new();
    let mut merkez = Vec::new();
    for (ad, veri) in dosyalar {
        let konum = v.len() as u32;
        let crc = crc32(veri);
        let ad_b = ad.as_bytes();
        // Verinin başlangıcı 4'e bölünsün diye "ek alan" doldurulur
        let ham = v.len() + 30 + ad_b.len();
        let dolgu = (4 - ham % 4) % 4;
        v.extend_from_slice(&0x04034b50u32.to_le_bytes());
        v.extend_from_slice(&10u16.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes()); // bayraklar
        v.extend_from_slice(&0u16.to_le_bytes()); // saklanmış
        v.extend_from_slice(&0u16.to_le_bytes()); // saat
        v.extend_from_slice(&0x21u16.to_le_bytes()); // 1980-01-01
        v.extend_from_slice(&crc.to_le_bytes());
        v.extend_from_slice(&(veri.len() as u32).to_le_bytes());
        v.extend_from_slice(&(veri.len() as u32).to_le_bytes());
        v.extend_from_slice(&(ad_b.len() as u16).to_le_bytes());
        v.extend_from_slice(&(dolgu as u16).to_le_bytes());
        v.extend_from_slice(ad_b);
        v.extend(std::iter::repeat_n(0u8, dolgu));
        v.extend_from_slice(veri);

        merkez.extend_from_slice(&0x02014b50u32.to_le_bytes());
        merkez.extend_from_slice(&20u16.to_le_bytes());
        merkez.extend_from_slice(&10u16.to_le_bytes());
        merkez.extend_from_slice(&0u16.to_le_bytes());
        merkez.extend_from_slice(&0u16.to_le_bytes());
        merkez.extend_from_slice(&0u16.to_le_bytes());
        merkez.extend_from_slice(&0x21u16.to_le_bytes());
        merkez.extend_from_slice(&crc.to_le_bytes());
        merkez.extend_from_slice(&(veri.len() as u32).to_le_bytes());
        merkez.extend_from_slice(&(veri.len() as u32).to_le_bytes());
        merkez.extend_from_slice(&(ad_b.len() as u16).to_le_bytes());
        merkez.extend_from_slice(&[0; 12]); // ek, yorum, disk, iç ve dış nitelikler
        merkez.extend_from_slice(&konum.to_le_bytes());
        merkez.extend_from_slice(ad_b);
    }
    let merkez_konumu = v.len() as u32;
    v.extend_from_slice(&merkez);
    v.extend_from_slice(&0x06054b50u32.to_le_bytes());
    v.extend_from_slice(&[0; 4]);
    v.extend_from_slice(&(dosyalar.len() as u16).to_le_bytes());
    v.extend_from_slice(&(dosyalar.len() as u16).to_le_bytes());
    v.extend_from_slice(&(merkez.len() as u32).to_le_bytes());
    v.extend_from_slice(&merkez_konumu.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v
}

// ---------------------------------------------------------------------------
// APK İmza Şeması v2
// ---------------------------------------------------------------------------

const ECDSA_SHA256: u32 = 0x0201;
const V2_KIMLIGI: u32 = 0x7109_871a;

/// Ayar klasöründeki imza anahtarı; yoksa üretilir. `ORHUNCA_ANDROID_ANAHTARI` ile
/// başka bir dosya gösterilebilir.
fn imza_anahtari() -> Result<SigningKey, String> {
    let yol = std::env::var_os("ORHUNCA_ANDROID_ANAHTARI")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::studyo::ayar_klasoru().join("android-imza.anahtar"));
    if let Ok(m) = std::fs::read_to_string(&yol) {
        let b = onaltilik_coz(m.trim()).ok_or("Android imza anahtarı okunamadı")?;
        return SigningKey::from_slice(&b)
            .map_err(|_| format!("'{}' geçerli bir imza anahtarı değil", yol.display()));
    }
    let anahtar = loop {
        let mut b = [0u8; 32];
        crate::guvenli_rastgele(&mut b).map_err(|e| format!("rastgele sayı üretilemedi: {e}"))?;
        if let Ok(a) = SigningKey::from_slice(&b) {
            break a;
        }
    };
    if let Some(k) = yol.parent() {
        std::fs::create_dir_all(k).map_err(|e| e.to_string())?;
    }
    let metin: String = anahtar
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    std::fs::write(&yol, metin + "\n")
        .map_err(|e| format!("'{}' yazılamadı: {e}", yol.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o600));
    }
    eprintln!(
        "Android imza anahtarı oluşturuldu: {}\n  Bu dosyayı saklayın: uygulamanın güncellemeleri aynı anahtarla imzalanmalıdır.",
        yol.display()
    );
    Ok(anahtar)
}

fn onaltilik_coz(m: &str) -> Option<Vec<u8>> {
    if !m.len().is_multiple_of(2) {
        return None;
    }
    (0..m.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(m.get(i..i + 2)?, 16).ok())
        .collect()
}

fn uzunluklu(v: &[u8]) -> Vec<u8> {
    let mut s = (v.len() as u32).to_le_bytes().to_vec();
    s.extend_from_slice(v);
    s
}

/// İmzasız ZIP'i imzalar: merkez dizinin önüne APK imza bloğu eklenir.
pub fn imzala(zip: &[u8], anahtar: &SigningKey) -> Result<Vec<u8>, String> {
    let son = zip.len() - 22;
    if u32_oku(zip, son) != 0x06054b50 {
        return Err("ZIP sonu bulunamadı".into());
    }
    let merkez_konumu = u32_oku(zip, son + 16) as usize;
    let girdiler = &zip[..merkez_konumu];
    let merkez = &zip[merkez_konumu..son];
    // Özet hesaplanırken ZIP sonundaki merkez dizin konumu, imza bloğunun konumunu
    // (yani bloksuz hâlini) gösterir.
    let eocd = zip[son..].to_vec();
    let ozet = parcali_ozet(&[girdiler, merkez, &eocd]);

    let sertifika = sertifika(anahtar);
    let mut ozetler = (ECDSA_SHA256).to_le_bytes().to_vec();
    ozetler.extend(uzunluklu(&ozet));
    let mut imzali = uzunluklu(&uzunluklu(&ozetler));
    imzali.extend(uzunluklu(&uzunluklu(&sertifika)));
    imzali.extend(uzunluklu(&[]));
    let imza: Signature = anahtar.sign(&imzali);
    let mut imza_kaydi = ECDSA_SHA256.to_le_bytes().to_vec();
    imza_kaydi.extend(uzunluklu(imza.to_der().as_bytes()));
    let mut imzalayan = uzunluklu(&imzali);
    imzalayan.extend(uzunluklu(&uzunluklu(&imza_kaydi)));
    imzalayan.extend(uzunluklu(&acik_anahtar_bilgisi(anahtar)));
    let deger = uzunluklu(&uzunluklu(&imzalayan));

    // Blok: boyut, (uzunluk, kimlik, değer) çiftleri, boyut, sihirli dizgi
    let mut cift = ((4 + deger.len()) as u64).to_le_bytes().to_vec();
    cift.extend_from_slice(&V2_KIMLIGI.to_le_bytes());
    cift.extend(deger);
    let boyut = (cift.len() + 8 + 16) as u64;
    let mut blok = boyut.to_le_bytes().to_vec();
    blok.extend(cift);
    blok.extend_from_slice(&boyut.to_le_bytes());
    blok.extend_from_slice(b"APK Sig Block 42");

    let mut apk = Vec::with_capacity(zip.len() + blok.len());
    apk.extend_from_slice(girdiler);
    apk.extend_from_slice(&blok);
    apk.extend_from_slice(merkez);
    let mut eocd = eocd;
    eocd[16..20].copy_from_slice(&((merkez_konumu + blok.len()) as u32).to_le_bytes());
    apk.extend(eocd);
    Ok(apk)
}

/// v2 özeti: her bölüm 1 MB'lık parçalara ayrılır, parça özetlerinin özeti alınır.
fn parcali_ozet(bolumler: &[&[u8]]) -> [u8; 32] {
    const PARCA: usize = 1 << 20;
    let mut ozetler = Vec::new();
    let mut sayi = 0u32;
    for b in bolumler {
        for p in b.chunks(PARCA) {
            let mut v = vec![0xa5];
            v.extend_from_slice(&(p.len() as u32).to_le_bytes());
            v.extend_from_slice(p);
            ozetler.extend_from_slice(&sha256(&v));
            sayi += 1;
        }
    }
    let mut v = vec![0x5a];
    v.extend_from_slice(&sayi.to_le_bytes());
    v.extend(ozetler);
    sha256(&v)
}

// --- DER (ASN.1) ---

fn der(etiket: u8, icerik: &[u8]) -> Vec<u8> {
    let mut v = vec![etiket];
    let n = icerik.len();
    if n < 0x80 {
        v.push(n as u8);
    } else if n < 0x100 {
        v.extend_from_slice(&[0x81, n as u8]);
    } else {
        v.extend_from_slice(&[0x82, (n >> 8) as u8, n as u8]);
    }
    v.extend_from_slice(icerik);
    v
}
fn dizi(parcalar: &[Vec<u8>]) -> Vec<u8> {
    der(0x30, &parcalar.concat())
}
fn oid(baytlar: &[u8]) -> Vec<u8> {
    der(0x06, baytlar)
}
// 1.2.840.10045.2.1 (ecPublicKey), 1.2.840.10045.3.1.7 (P-256), 1.2.840.10045.4.3.2 (ecdsa-with-SHA256), 2.5.4.3 (CN)
const OID_EC: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
const OID_P256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
const OID_ECDSA_SHA256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
const OID_CN: &[u8] = &[0x55, 0x04, 0x03];

fn acik_anahtar_bilgisi(a: &SigningKey) -> Vec<u8> {
    let nokta = a.verifying_key().to_encoded_point(false);
    let mut bit = vec![0u8];
    bit.extend_from_slice(nokta.as_bytes());
    dizi(&[dizi(&[oid(OID_EC), oid(OID_P256)]), der(0x03, &bit)])
}

/// Kendinden imzalı X.509 sertifikası (Android yalnızca anahtarı ve imzayı denetler).
fn sertifika(a: &SigningKey) -> Vec<u8> {
    let ad = dizi(&[der(
        0x31,
        &dizi(&[oid(OID_CN), der(0x0c, b"Orhunca uygulamasi")]),
    )]);
    // Seri numarası anahtardan türetilir (aynı anahtar → aynı sertifika)
    let mut seri = sha256(a.verifying_key().to_encoded_point(false).as_bytes())[..8].to_vec();
    seri[0] &= 0x7f;
    seri[0] |= 0x01;
    let algoritma = dizi(&[oid(OID_ECDSA_SHA256)]);
    let tbs = dizi(&[
        der(0xa0, &der(0x02, &[2])),
        der(0x02, &seri),
        algoritma.clone(),
        ad.clone(),
        dizi(&[der(0x17, b"000101000000Z"), der(0x18, b"20991231235959Z")]),
        ad,
        acik_anahtar_bilgisi(a),
    ]);
    let imza: Signature = a.sign(&tbs);
    let mut bit = vec![0u8];
    bit.extend_from_slice(imza.to_der().as_bytes());
    dizi(&[tbs, algoritma, der(0x03, &bit)])
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kimlik() {
        assert_eq!(varsayilan_kimlik("Sayaç"), "org.orhunca.sayac");
        assert_eq!(
            varsayilan_kimlik("Sınıf Defteri"),
            "org.orhunca.sinif_defteri"
        );
        assert_eq!(varsayilan_kimlik("2048"), "org.orhunca.u2048");
        assert!(kimlik_gecerli_mi("org.orhunca.sayac"));
        assert!(!kimlik_gecerli_mi("sayac"));
        assert!(!kimlik_gecerli_mi("org.1abc"));
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
    }

    #[test]
    fn manifest_yeniden_yazilir() {
        let dosyalar = kabuk_dosyalari().unwrap();
        let (_, xml) = dosyalar
            .iter()
            .find(|(a, _)| a == "AndroidManifest.xml")
            .unwrap();
        let yeni = manifest_degistir(xml, &|s: &str| {
            (s == YER_AD).then(|| "Sınıf Defteri — çok uzun bir uygulama adı".to_string())
        })
        .unwrap();
        let geri = manifest_degistir(&yeni, &|_| None).unwrap();
        assert_eq!(yeni, geri);
        let metin = String::from_utf16_lossy(
            &yeni
                .chunks(2)
                .map(|b| u16::from_le_bytes([b[0], *b.get(1).unwrap_or(&0)]))
                .collect::<Vec<_>>(),
        );
        assert!(metin.contains("Sınıf Defteri"));
        assert!(!metin.contains(YER_AD));
    }
}
