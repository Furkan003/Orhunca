//! Yerleşik bağlayıcı: C derleyicisi ve sistem bağlayıcısı olmadan çalıştırılabilir
//! dosya üretir.
//!
//! Çalışma zamanı her hedef için önceden derlenmiş bir "çalıştırıcı" olarak
//! derleyiciye gömülüdür (`runtime/calistirici/`, araclar/calistiricilar.sh). Bu
//! modül Cranelift'in ürettiği nesne dosyasını (ELF ya da COFF) tek bir kod
//! bloğuna yerleştirir: bölümleri art arda dizer, iç yerleşimleri (çağrılar, metin
//! sabitleri) kendisi çözer; çalışma zamanı işlevleri için bloğun sonuna atlama
//! basamakları ve bir adres tablosu (GOT) ekler. Çalıştırıcı açılınca bloğu belleğe
//! yükler, yalnızca 64 bitlik mutlak adresleri yazar ve programı çalıştırır
//! (`program_yukle`, runtime/orhunca_rt.c).

use object::read::{Object, ObjectSection, ObjectSymbol, RelocationTarget};
use object::{RelocationFlags, RelocationKind, SectionKind};
use std::collections::HashMap;

/// Linux x86-64 çalıştırıcısı (glibc)
pub const LINUX_X86_64: &[u8] = include_bytes!("../runtime/calistirici/linux-x86_64");
/// Windows x86-64 çalıştırıcısı (MinGW ile derlenmiş; yalnızca sistem DLL'leri)
pub const WINDOWS_X86_64: &[u8] = include_bytes!("../runtime/calistirici/windows-x86_64.exe");

/// Hedef için gömülü çalıştırıcı (yoksa sistem bağlayıcısı kullanılır).
pub fn calistirici(triple: &target_lexicon::Triple) -> Option<&'static [u8]> {
    use target_lexicon::{Architecture, OperatingSystem};
    match (triple.architecture, triple.operating_system) {
        (Architecture::X86_64, OperatingSystem::Linux) => Some(LINUX_X86_64),
        (Architecture::X86_64, OperatingSystem::Windows) => Some(WINDOWS_X86_64),
        _ => None,
    }
}

/// Düzeltme türleri (çalıştırıcıyla ortak)
const ICE_AKTARIM: u32 = 1;
const TABAN: u32 = 2;

struct Duzeltme {
    yer: u64,
    tur: u32,
    ice: u32,
}

/// Çalıştırıcının sonuna eklenecek kod paketi.
pub struct Paket {
    kod: Vec<u8>,
    giris: u64,
    ice_aktarimlar: Vec<String>,
    duzeltmeler: Vec<Duzeltme>,
}

impl Paket {
    fn ice_sira(&mut self, ad: &str) -> u32 {
        match self.ice_aktarimlar.iter().position(|a| a == ad) {
            Some(i) => i as u32,
            None => {
                self.ice_aktarimlar.push(ad.to_string());
                (self.ice_aktarimlar.len() - 1) as u32
            }
        }
    }

    fn hizala(&mut self, hiza: u64) {
        let hiza = hiza.max(1);
        while !(self.kod.len() as u64).is_multiple_of(hiza) {
            self.kod.push(0);
        }
    }

    fn baytlar(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"OHCGRT01");
        b.extend_from_slice(&(self.kod.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.giris.to_le_bytes());
        b.extend_from_slice(&(self.ice_aktarimlar.len() as u64).to_le_bytes());
        for a in &self.ice_aktarimlar {
            b.extend_from_slice(&(a.len() as u32).to_le_bytes());
            b.extend_from_slice(a.as_bytes());
        }
        b.extend_from_slice(&(self.duzeltmeler.len() as u64).to_le_bytes());
        for d in &self.duzeltmeler {
            b.extend_from_slice(&d.yer.to_le_bytes());
            b.extend_from_slice(&d.tur.to_le_bytes());
            b.extend_from_slice(&d.ice.to_le_bytes());
        }
        b.extend_from_slice(&self.kod);
        b
    }
}

/// Yerleşimin hedefi: blok içindeki bir yer ya da çalışma zamanı işlevi.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Hedef {
    Ic(u64),
    Ice(u32),
}

/// Yükleme gerektirmeyen bölümler (çözülme bilgisi, notlar, hata ayıklama).
fn atlanir(ad: &str) -> bool {
    [
        ".eh_frame",
        ".pdata",
        ".xdata",
        ".note",
        ".debug",
        ".comment",
        ".llvm",
    ]
    .iter()
    .any(|o| ad.starts_with(o))
}

/// Nesne dosyasını kod paketine çevirir; giriş noktası `ohc_ana`.
pub fn paketle(nesne: &[u8]) -> Result<Paket, String> {
    let dosya = object::File::parse(nesne).map_err(|e| format!("nesne dosyası okunamadı: {e}"))?;
    let mut p = Paket {
        kod: Vec::new(),
        giris: 0,
        ice_aktarimlar: Vec::new(),
        duzeltmeler: Vec::new(),
    };
    // 1. Bölümleri yerleştir
    let mut bolum_yeri: HashMap<object::SectionIndex, u64> = HashMap::new();
    for b in dosya.sections() {
        let ad = b.name().unwrap_or("");
        let yuklenir = matches!(
            b.kind(),
            SectionKind::Text
                | SectionKind::Data
                | SectionKind::ReadOnlyData
                | SectionKind::ReadOnlyString
                | SectionKind::ReadOnlyDataWithRel
                | SectionKind::UninitializedData
        );
        if !yuklenir || atlanir(ad) {
            continue;
        }
        p.hizala(b.align());
        bolum_yeri.insert(b.index(), p.kod.len() as u64);
        if b.kind() == SectionKind::UninitializedData {
            p.kod.resize(p.kod.len() + b.size() as usize, 0);
        } else {
            let veri = b.data().map_err(|e| e.to_string())?;
            p.kod.extend_from_slice(veri);
            // Dosyadaki veri bölüm boyundan kısaysa sıfırla tamamla
            let eksik = (b.size() as usize).saturating_sub(veri.len());
            p.kod.resize(p.kod.len() + eksik, 0);
        }
    }

    // 2. Simgeler
    let hedef_bul = |p: &mut Paket, t: RelocationTarget| -> Result<Hedef, String> {
        match t {
            RelocationTarget::Symbol(i) => {
                let s = dosya
                    .symbol_by_index(i)
                    .map_err(|e| format!("simge okunamadı: {e}"))?;
                if s.is_undefined() {
                    let ad = s.name().map_err(|e| e.to_string())?;
                    return Ok(Hedef::Ice(p.ice_sira(ad)));
                }
                let b = s.section_index().ok_or_else(|| {
                    format!("'{}' simgesinin bölümü yok", s.name().unwrap_or("?"))
                })?;
                let taban = *bolum_yeri.get(&b).ok_or_else(|| {
                    format!("'{}' yüklenmeyen bir bölümde", s.name().unwrap_or("?"))
                })?;
                let bolum = dosya.section_by_index(b).map_err(|e| e.to_string())?;
                Ok(Hedef::Ic(taban + s.address() - bolum.address()))
            }
            RelocationTarget::Section(b) => Ok(Hedef::Ic(
                *bolum_yeri.get(&b).ok_or("yüklenmeyen bölüme yerleşim")?,
            )),
            _ => Err("desteklenmeyen yerleşim hedefi".into()),
        }
    };
    let giris = dosya
        .symbols()
        .find(|s| s.name() == Ok("ohc_ana"))
        .ok_or("nesne dosyasında ohc_ana yok")?;
    p.giris = match hedef_bul(&mut p, RelocationTarget::Symbol(giris.index()))? {
        Hedef::Ic(y) => y,
        Hedef::Ice(_) => return Err("ohc_ana tanımlı değil".into()),
    };

    // 3. Yerleşimler: önce topla (atlama basamakları ve GOT için yer ayrılacak)
    struct Yerlesim {
        yer: u64,
        tur: RelocationKind,
        boy: u8,
        ek: i64,
        hedef: Hedef,
    }
    let mut yerlesimler = Vec::new();
    for b in dosya.sections() {
        let Some(&taban) = bolum_yeri.get(&b.index()) else {
            continue;
        };
        for (ofset, r) in b.relocations() {
            let yer = taban + ofset;
            let mut tur = r.kind();
            // GOTPCRELX / REX_GOTPCRELX (ELF) ad olarak tanınmaz: GOT'a göreli sayılır.
            if tur == RelocationKind::Unknown {
                if let RelocationFlags::Elf { r_type } = r.flags() {
                    if r_type == object::elf::R_X86_64_GOTPCRELX
                        || r_type == object::elf::R_X86_64_REX_GOTPCRELX
                    {
                        tur = RelocationKind::GotRelative;
                    }
                }
            }
            let boy = r.size();
            let mut ek = r.addend();
            if r.has_implicit_addend() {
                let y = yer as usize;
                ek += match boy {
                    32 => i32::from_le_bytes(p.kod[y..y + 4].try_into().unwrap()) as i64,
                    64 => i64::from_le_bytes(p.kod[y..y + 8].try_into().unwrap()),
                    _ => 0,
                };
            }
            let hedef = hedef_bul(&mut p, r.target())?;
            yerlesimler.push(Yerlesim {
                yer,
                tur,
                boy,
                ek,
                hedef,
            });
        }
    }

    // 4. Atlama basamakları (çalışma zamanı çağrıları için: jmp [rip+0]; adres) ve GOT
    let mut basamak: HashMap<u32, u64> = HashMap::new();
    let mut got: HashMap<Hedef, u64> = HashMap::new();
    p.hizala(16);
    for y in &yerlesimler {
        if let (Hedef::Ice(i), RelocationKind::Relative | RelocationKind::PltRelative) =
            (y.hedef, y.tur)
        {
            if let std::collections::hash_map::Entry::Vacant(e) = basamak.entry(i) {
                e.insert(p.kod.len() as u64);
                p.kod.extend_from_slice(&[0xFF, 0x25, 0, 0, 0, 0]);
                p.duzeltmeler.push(Duzeltme {
                    yer: p.kod.len() as u64,
                    tur: ICE_AKTARIM,
                    ice: i,
                });
                p.kod.extend_from_slice(&[0; 8]);
                p.kod.extend_from_slice(&[0xCC, 0xCC]);
            }
        }
    }
    p.hizala(8);
    for y in &yerlesimler {
        if y.tur == RelocationKind::GotRelative && !got.contains_key(&y.hedef) {
            let yer = p.kod.len() as u64;
            got.insert(y.hedef, yer);
            let (deger, tur, ice) = match y.hedef {
                Hedef::Ic(h) => (h, TABAN, 0),
                Hedef::Ice(i) => (0, ICE_AKTARIM, i),
            };
            p.kod.extend_from_slice(&deger.to_le_bytes());
            p.duzeltmeler.push(Duzeltme { yer, tur, ice });
        }
    }

    // 5. Yerleşimleri uygula
    for y in &yerlesimler {
        let yaz32 = |p: &mut Paket, d: i64| -> Result<(), String> {
            let d = i32::try_from(d).map_err(|_| "yerleşim 32 bite sığmıyor")?;
            let i = y.yer as usize;
            p.kod[i..i + 4].copy_from_slice(&d.to_le_bytes());
            Ok(())
        };
        match (y.tur, y.boy) {
            (RelocationKind::Absolute, 64) => {
                let i = y.yer as usize;
                let (deger, tur, ice) = match y.hedef {
                    Hedef::Ic(h) => (h as i64 + y.ek, TABAN, 0),
                    Hedef::Ice(n) => (y.ek, ICE_AKTARIM, n),
                };
                p.kod[i..i + 8].copy_from_slice(&deger.to_le_bytes());
                p.duzeltmeler.push(Duzeltme {
                    yer: y.yer,
                    tur,
                    ice,
                });
            }
            (RelocationKind::Relative | RelocationKind::PltRelative, 32) => {
                let s = match y.hedef {
                    Hedef::Ic(h) => h,
                    Hedef::Ice(i) => basamak[&i],
                };
                yaz32(&mut p, s as i64 + y.ek - y.yer as i64)?;
            }
            (RelocationKind::GotRelative, 32) => {
                let g = got[&y.hedef];
                yaz32(&mut p, g as i64 + y.ek - y.yer as i64)?;
            }
            (tur, boy) => {
                return Err(format!(
                    "desteklenmeyen yerleşim: {tur:?} ({boy} bit, {:?})",
                    dosya.format()
                ))
            }
        }
    }
    Ok(p)
}

/// Çalıştırıcı + kod paketi + son (`ORHUNCA!` ve paket boyu): çalıştırılabilir dosya.
pub fn calistirilabilir(calistirici: &[u8], nesne: &[u8]) -> Result<Vec<u8>, String> {
    let paket = paketle(nesne)?.baytlar();
    let mut d = Vec::with_capacity(calistirici.len() + paket.len() + 16);
    d.extend_from_slice(calistirici);
    d.extend_from_slice(&paket);
    d.extend_from_slice(b"ORHUNCA!");
    d.extend_from_slice(&(paket.len() as u64).to_le_bytes());
    Ok(d)
}

#[cfg(test)]
mod testler {
    use std::path::Path;
    use std::str::FromStr;

    /// Bütün örnekler Linux (ELF) ve Windows (COFF) için paketlenebilir: bütün
    /// yerleşim türleri desteklenir ve giriş noktası bulunur.
    #[test]
    fn ornekler_paketlenir() {
        let kok = Path::new(env!("CARGO_MANIFEST_DIR")).join("örnekler");
        let mut sayi = 0;
        for g in std::fs::read_dir(kok).unwrap() {
            let yol = g.unwrap().path();
            if yol.extension().is_none_or(|u| u != "ohc") {
                continue;
            }
            let p = crate::derleme::yukle(&yol).unwrap();
            for hedef in ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-gnu"] {
                let triple = target_lexicon::Triple::from_str(hedef).unwrap();
                let isa = crate::uretici::isa_kur(triple).unwrap();
                let nesne = crate::uretici::uret(&p, isa).unwrap();
                let paket = super::paketle(&nesne)
                    .unwrap_or_else(|h| panic!("{} ({hedef}): {h}", yol.display()));
                assert!(paket.giris < paket.kod.len() as u64);
                assert!(paket.ice_aktarimlar.iter().all(|a| a.starts_with("ohc_")));
                sayi += 1;
            }
        }
        assert!(sayi > 20);
    }
}
