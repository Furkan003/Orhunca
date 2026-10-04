//! WebAssembly kod üretici: söz dizimi ağacını bir Wasm modülüne çevirir.
//!
//! Üretilen "program modülü", C çalışma zamanının wasm32 derlemesini
//! (`runtime/orhunca_rt.wasm`) kullanır: onun belleğini ve `ohc_*` işlevlerini
//! içe aktarır. İki modülü JavaScript yükleyicisi (`runtime/wasm/orhunca.js`)
//! birleştirir; Wasm bağlayıcısı gerekmez.
//!
//! Çöp toplayıcı: WebAssembly'de yığıt ve yerel değişkenler taranamadığı için
//! metin, liste, sözlük ve model değerleri bellekteki "gölge yığıtta" tutulur.
//! Her işlevin gölge yığıtta bir çerçevesi vardır: bu tiplerdeki değişkenler ve
//! bir Orhunca işlevi çağrılırken saklanması gereken ara değerler oradadır.
//! Toplama yalnızca güvenli noktalarda (işlev girişleri, döngü başları) yapılır;
//! o anda canlı olan her değer gölge yığıttadır. Sayı, ondalık ve mantık
//! değerleri Wasm yerel değişkenlerinde durur.

use crate::agac::*;
use crate::arayuz::{self, Beklenen};
use crate::denetci::OLAY_DEGERI;
use crate::hata::Konum;
use crate::on_kutuphane::{HATA_SATIRDA, YERLESIK_ON_EK};
use crate::uretici::{
    CALISMA_ZAMANI, DENE_DONDUR, DENE_DUR, DENE_SONA_ERDI, DENE_SURDUR, ILK_ALAN,
};
use std::borrow::Cow;
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataCountSection, DataSection, ElementSection, Elements,
    EntityType, ExportKind, ExportSection, Function, FunctionSection, GlobalSection, GlobalType,
    ImportSection, Instruction as K, MemArg, MemoryType, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

/// Yalnızca WebAssembly'de kullanılan çalışma zamanı işlevleri: ad, parametre
/// sayısı, değer döndürür mü.
const WASM_EKLERI: &[(&str, usize, bool)] = &[
    ("ohc_wasm_ayir", 1, true),
    ("ohc_wasm_golge", 1, true),
    ("ohc_wasm_bayrak", 0, true),
    ("ohc_guvenli_nokta", 1, false),
    ("ohc_yigin_tasti", 0, false),
    ("ohc_carp", 3, true),
    ("ohc_wasm_metin", 1, true),
];

/// Arayüz programlarının JavaScript'ten içe aktardığı işlevler (`ui` modülü):
/// çizim sırasında öğe ağacını kurar. Değerler bellekteki metinlerin adresidir.
const UI_ISLEVLERI: &[(&str, usize)] = &[("ac", 1), ("ozellik", 2), ("olay", 2), ("kapat", 0)];

/// Olay işlevlerinin ilk parametresi: çizim anında yakalanan değerlerin listesi.
const YAKALANANLAR: &str = "‹yakalananlar›";

/// Web sunucusu ve hata ayıklama kancaları WebAssembly'de yoktur; `dene:` bloklarını
/// JavaScript çalıştırır.
const YALNIZ_YEREL: &[&str] = &[
    "ohc_web_yol",
    "ohc_sun",
    "ohc_dene",
    "ohc_ay_gir",
    "ohc_ay_cik",
    "ohc_ay_satir",
    "ohc_yigin_denetle",
];

/// `dene:` bloklarının işlevlerinin tek parametresi: çevreleyen işlevin çerçevesi.
const DENE_CERCEVESI: &str = "‹çerçeve›";

/// Gölge yığıtın boyutu (bayt; çalışma zamanının ayırıcısı ikinin kuvvetlerine yuvarlar).
const GOLGE_BOYUTU: i64 = (1 << 20) - 64;

/// Genel değişkenler (globals)
const G_TEPE: u32 = 0; // gölge yığıtın tepesi (i32)
const G_SINIR: u32 = 1; // gölge yığıtın sonu (i32)
const G_BAYRAK: u32 = 2; // "toplama gerekli" bayrağının adresi (i32)
const G_METINLER: u32 = 3; // sabit metinlerin bellekteki başı (i64)
const G_GENEL: u32 = 4; // durum değişkenlerinin ve olay listesinin yuvaları (i32)
const G_METIN_BOYU: u32 = 5; // sabit metinlerin toplam boyu (i32, değişmez; en son bilinir)

/// Bir değişkenin ya da ara değerin yeri.
#[derive(Clone, Copy, Debug)]
enum Yer {
    /// Gölge yığıt çerçevesindeki sıra (8 baytlık yuva).
    Yuva(u32),
    /// Wasm yerel değişkeni (i64).
    Yerel(u32),
    /// `dene:` bloğunun işlevinde: çevreleyen işlevin çerçevesindeki yuva (bayt ofseti).
    Dis(u32),
}

/// Çalışma zamanı çağrısının bir değeri.
enum Arg<'e> {
    /// Hesaplanacak ifade
    I(&'e Ifade),
    /// Hesaplanıp metne çevrilecek ifade
    Metne(&'e Ifade),
    /// Sayı sabiti
    S(i64),
    /// Sabit metin (ör. modelin tanımı)
    Metin(String),
    /// Önceden hesaplanmış ara değer
    Hazir(Yer),
}

/// Çöp toplayıcının yönettiği bir değer mi (metin, liste, sözlük, model)?
fn yonetilen(t: &Tip) -> bool {
    !matches!(t, Tip::Sayi | Tip::Ondalik | Tip::Mantik | Tip::Bos)
}

/// Değerlendirilmesi gözlenebilir bir etki bırakmayan ifade: sırası değişebilir.
fn yan_etkisiz(e: &Ifade) -> bool {
    matches!(
        e.tur,
        IfadeTuru::Isim(_)
            | IfadeTuru::Sayi(_)
            | IfadeTuru::Ondalik(_)
            | IfadeTuru::Mantik(_)
            | IfadeTuru::Metin(_)
    )
}

fn bellek(ofset: u32) -> MemArg {
    MemArg {
        offset: ofset as u64,
        align: 3,
        memory_index: 0,
    }
}

struct Ortak {
    turler: TypeSection,
    tur_sirasi: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    calisma: HashMap<&'static str, (u32, bool)>,
    islevler: HashMap<String, (u32, bool)>,
    metinler: Vec<u8>,
    metin_yeri: HashMap<String, u32>,
    modeller: HashMap<String, Model>,
    /// Durum değişkenlerinin genel bölgedeki yuvası
    genel: HashMap<String, u32>,
    /// Çizimde kaydedilen olayların listesinin genel bölgedeki yuvası
    olay_yuvasi: u32,
    /// `ui` modülünden içe aktarılan işlevler
    ui: HashMap<&'static str, u32>,
    /// Öğenin (konum, olay adı) → olay işlevinin tablodaki sırası
    olay_sirasi: HashMap<(Konum, String), u32>,
    /// JavaScript'ten içe aktarılan `dn.dene(tablodaki sıra, çerçeve) -> kod`
    dene: Option<u32>,
    /// `dene:` bloklarının işlevleri: tablodaki ilk sıra ve ilk işlev sırası
    govde_tablo_basi: u32,
    govde_islev_basi: u32,
    /// Üretilmeyi bekleyen ve toplam `dene:` blokları
    govdeler: Vec<DeneGovdesi>,
    govde_sayisi: u32,
}

/// `dene:` bloğu ayrı bir işleve çevrilir: `(çerçeve) -> dönüş kodu`. Çerçeve,
/// çevreleyen işlevin gölge yığıttaki yuvalarıdır: 0. yuva `döndür` değeri,
/// sonrakiler bloğun kullandığı değişkenler. Blok bu değişkenleri doğrudan
/// çerçevede okur ve yazar. Çalışma hatası JavaScript istisnasıdır; yükleyici
/// (orhunca.js) onu yakalayıp -1 döndürür.
struct DeneGovdesi {
    govde: Vec<Deyim>,
    yakalananlar: Vec<String>,
    /// Asıl (en dıştaki) işlevin dönüşü: `None` ana program, `Some(true)` değer döndürür.
    dis_donus: Option<bool>,
}

impl Ortak {
    fn tur_ekle(&mut self, p: &[ValType], r: &[ValType]) -> u32 {
        let anahtar = (p.to_vec(), r.to_vec());
        if let Some(t) = self.tur_sirasi.get(&anahtar) {
            return *t;
        }
        let sira = self.tur_sirasi.len() as u32;
        self.turler
            .ty()
            .function(p.iter().copied(), r.iter().copied());
        self.tur_sirasi.insert(anahtar, sira);
        sira
    }

    /// `n` adet i64 alan, `doner` ise i64 döndüren işlev tipi.
    fn tur(&mut self, n: usize, doner: bool) -> u32 {
        let sonuc: &[ValType] = if doner { &[ValType::I64] } else { &[] };
        self.tur_ekle(&vec![ValType::I64; n], sonuc)
    }

    fn metin(&mut self, m: &str) -> u32 {
        if let Some(y) = self.metin_yeri.get(m) {
            return *y;
        }
        let y = self.metinler.len() as u32;
        self.metinler.extend_from_slice(m.as_bytes());
        self.metinler.push(0);
        self.metin_yeri.insert(m.to_string(), y);
        y
    }

    fn tanim(&self, model: &str) -> Result<String, String> {
        Ok(self
            .modeller
            .get(model)
            .ok_or_else(|| format!("tanımsız model '{model}'"))?
            .tanim_metni())
    }
}

/// Programı WebAssembly modülüne (ikili) çevirir.
pub fn uret(p: &Program) -> Result<Vec<u8>, String> {
    if let Some(f) = p.islevler.iter().find(|f| f.rota.is_some()) {
        let r = f.rota.as_ref().unwrap();
        let soz = match r.yontem.as_str() {
            "GET" => "al",
            "POST" => "gönder",
            "PUT" => "koy",
            _ => "sil",
        };
        return Err(format!(
            "web yolları ({soz} \"{}\") WebAssembly hedefinde kullanılamaz: tarayıcıda sunucu \
             çalışmaz. Sunucu programını bu bilgisayar için derleyin (--hedef olmadan).",
            r.kalip
        ));
    }
    let mut o = Ortak {
        turler: TypeSection::new(),
        tur_sirasi: HashMap::new(),
        calisma: HashMap::new(),
        islevler: HashMap::new(),
        metinler: Vec::new(),
        metin_yeri: HashMap::new(),
        modeller: p
            .modeller
            .iter()
            .map(|m| (m.ad.clone(), m.clone()))
            .collect(),
        genel: p
            .durumlar
            .iter()
            .enumerate()
            .map(|(i, d)| (d.ad.clone(), i as u32))
            .collect(),
        olay_yuvasi: p.durumlar.len() as u32,
        ui: HashMap::new(),
        olay_sirasi: HashMap::new(),
        dene: None,
        govde_tablo_basi: 0,
        govde_islev_basi: 0,
        govdeler: Vec::new(),
        govde_sayisi: 0,
    };
    let dene_kullanilir = dene_var(&p.ana) || p.islevler.iter().any(|f| dene_var(&f.govde));
    let arayuz_var = p.islevler.iter().any(|f| f.arayuz && f.ad == ARAYUZ_ISLEVI);
    // Olay blokları ayrı işlevlere çevrilir; çizimde tabloya göre çağrılırlar.
    let mut olaylar = Vec::new();
    for f in p.islevler.iter().filter(|f| f.arayuz) {
        olaylari_topla(&f.govde, &mut olaylar, &mut o.olay_sirasi);
    }

    let mut ice = ImportSection::new();
    ice.import(
        "rt",
        "memory",
        EntityType::Memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );
    let mut sira = 0u32;
    for (ad, n, doner) in CALISMA_ZAMANI.iter().chain(WASM_EKLERI) {
        if YALNIZ_YEREL.contains(ad) {
            continue;
        }
        let t = o.tur(*n, *doner);
        ice.import("rt", ad, EntityType::Function(t));
        o.calisma.insert(ad, (sira, *doner));
        sira += 1;
    }
    if arayuz_var {
        for (ad, n) in UI_ISLEVLERI {
            let t = o.tur_ekle(&vec![ValType::I32; *n], &[]);
            ice.import("ui", ad, EntityType::Function(t));
            o.ui.insert(ad, sira);
            sira += 1;
        }
    }
    if dene_kullanilir {
        let t = o.tur_ekle(&[ValType::I32, ValType::I64], &[ValType::I64]);
        ice.import("dn", "dene", EntityType::Function(t));
        o.dene = Some(sira);
        sira += 1;
    }

    let mut islevler = FunctionSection::new();
    for f in p.islevler.iter().chain(&olaylar) {
        let doner = f.donus.as_ref().is_some_and(|t| *t != Tip::Bos);
        let t = o.tur(f.parametreler.len(), doner);
        islevler.function(t);
        o.islevler.insert(f.ad.clone(), (sira, doner));
        sira += 1;
    }
    let ana_sirasi = sira;
    let t = o.tur(0, false);
    islevler.function(t);
    let olay_turu = o.tur(2, false);
    if arayuz_var {
        islevler.function(t); // ohc_ciz
        let t = o.tur_ekle(&[ValType::I32, ValType::I32], &[]);
        islevler.function(t); // ohc_olay
    }
    o.govde_tablo_basi = olaylar.len() as u32;
    o.govde_islev_basi = ana_sirasi + if arayuz_var { 3 } else { 1 };

    let mut kod = CodeSection::new();
    for f in p.islevler.iter().chain(&olaylar) {
        let (_, doner) = o.islevler[&f.ad];
        let govde = islev_uret(
            &mut o,
            &f.yereller,
            &f.parametreler,
            &f.govde,
            Some(doner),
            None,
        )
        .map_err(|e| format!("'{}': {e}", f.ad))?;
        kod.function(&govde);
    }
    // Ana program en sona: sabit metinlerin tamamı ancak o zaman bilinir.
    // Durum değişkenleri programın başında ilk değerlerini alır.
    let mut ana_govde: Vec<Deyim> = p
        .durumlar
        .iter()
        .map(|d| Deyim::Atama {
            hedef: d.ad.clone(),
            tip: None,
            deger: d.deger.clone(),
            konum: d.konum,
        })
        .collect();
    ana_govde.extend(p.ana.iter().cloned());
    let ana = islev_uret(&mut o, &p.ana_yereller, &[], &ana_govde, None, None)?;
    kod.function(&ana);
    if arayuz_var {
        kod.function(&ciz_islevi(&o));
        kod.function(&olay_islevi(&o, olay_turu));
    }
    // `dene:` blokları (sırayla; iç içe olanlar sona eklenir)
    let mut uretilen = 0;
    while uretilen < o.govdeler.len() {
        let g = std::mem::replace(
            &mut o.govdeler[uretilen],
            DeneGovdesi {
                govde: Vec::new(),
                yakalananlar: Vec::new(),
                dis_donus: None,
            },
        );
        let parametre = [(DENE_CERCEVESI.to_string(), Tip::Sayi)];
        let f = islev_uret(
            &mut o,
            &parametre,
            &parametre,
            &g.govde,
            Some(true),
            Some((&g.yakalananlar, g.dis_donus)),
        )?;
        let t = o.tur(1, true);
        islevler.function(t);
        kod.function(&f);
        uretilen += 1;
    }
    let govde_islevleri: Vec<u32> = (0..o.govde_sayisi)
        .map(|i| o.govde_islev_basi + i)
        .collect();
    let tablo_var = arayuz_var || !govde_islevleri.is_empty();

    let mut genel = GlobalSection::new();
    let metin_boyu = o.metinler.len() as i32;
    for tur in [
        ValType::I32,
        ValType::I32,
        ValType::I32,
        ValType::I64,
        ValType::I32,
    ] {
        let ilk = if tur == ValType::I32 {
            ConstExpr::i32_const(0)
        } else {
            ConstExpr::i64_const(0)
        };
        genel.global(
            GlobalType {
                val_type: tur,
                mutable: true,
                shared: false,
            },
            &ilk,
        );
    }
    genel.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: false,
            shared: false,
        },
        &ConstExpr::i32_const(metin_boyu),
    );
    let mut disa = ExportSection::new();
    disa.export("ohc_ana", ExportKind::Func, ana_sirasi);
    if arayuz_var {
        disa.export("ohc_ciz", ExportKind::Func, ana_sirasi + 1);
        disa.export("ohc_olay", ExportKind::Func, ana_sirasi + 2);
    }
    if !govde_islevleri.is_empty() {
        disa.export("ohc_tablo", ExportKind::Table, 0);
    }

    // Olay ve `dene:` işlevleri tablosu (call_indirect)
    let mut olay_islevleri: Vec<u32> = olaylar.iter().map(|f| o.islevler[&f.ad].0).collect();
    olay_islevleri.extend(&govde_islevleri);
    let mut tablo = TableSection::new();
    tablo.table(TableType {
        element_type: RefType::FUNCREF,
        table64: false,
        minimum: olay_islevleri.len() as u64,
        maximum: Some(olay_islevleri.len() as u64),
        shared: false,
    });
    let mut ogeler = ElementSection::new();
    ogeler.active(
        Some(0),
        &ConstExpr::i32_const(0),
        Elements::Functions(Cow::Borrowed(&olay_islevleri)),
    );

    let mut veri = DataSection::new();
    veri.passive(o.metinler.iter().copied());

    let mut m = Module::new();
    m.section(&o.turler).section(&ice).section(&islevler);
    if tablo_var {
        m.section(&tablo);
    }
    m.section(&genel).section(&disa);
    if tablo_var && !olay_islevleri.is_empty() {
        m.section(&ogeler);
    }
    m.section(&DataCountSection { count: 1 })
        .section(&kod)
        .section(&veri);
    Ok(m.finish())
}

/// Arayüz işlevlerindeki olay bloklarını (ve bağlı öğelerin güncelleme bloklarını)
/// birer işleve çevirir: `(yakalananlar, değer)` alırlar; önce yakalanan
/// değerleri yerel değişkenlere açar, sonra bloğu çalıştırırlar.
fn olaylari_topla(
    govde: &[Deyim],
    olaylar: &mut Vec<Islev>,
    sira: &mut HashMap<(Konum, String), u32>,
) {
    for d in govde {
        match d {
            Deyim::Oge(o) => {
                for olay in o.baglama.iter().chain(&o.olay) {
                    sira.insert((o.konum, olay.ad.clone()), olaylar.len() as u32);
                    olaylar.push(olay_islevi_kur(olay, olaylar.len()));
                }
                olaylari_topla(&o.cocuklar, olaylar, sira);
            }
            Deyim::Eger { govde, degilse, .. } => {
                olaylari_topla(govde, olaylar, sira);
                olaylari_topla(degilse, olaylar, sira);
            }
            Deyim::Surece { govde, .. }
            | Deyim::HerAralik { govde, .. }
            | Deyim::HerListe { govde, .. } => olaylari_topla(govde, olaylar, sira),
            Deyim::Dene { govde, yakala, .. } => {
                olaylari_topla(govde, olaylar, sira);
                olaylari_topla(yakala, olaylar, sira);
            }
            _ => {}
        }
    }
}

fn olay_islevi_kur(olay: &Olay, no: usize) -> Islev {
    let k = olay.konum;
    let yakalanan_tipi = Tip::Liste(Box::new(Tip::Bilinmeyen));
    let parametreler = vec![
        (YAKALANANLAR.to_string(), yakalanan_tipi),
        (OLAY_DEGERI.to_string(), Tip::Metin),
    ];
    let mut yereller = parametreler.clone();
    yereller.extend(olay.yakalananlar.iter().cloned());
    yereller.extend(
        olay.yereller
            .iter()
            .filter(|(a, _)| a != OLAY_DEGERI)
            .cloned(),
    );
    let mut govde: Vec<Deyim> = olay
        .yakalananlar
        .iter()
        .enumerate()
        .map(|(i, (ad, tip))| Deyim::Atama {
            hedef: ad.clone(),
            tip: None,
            deger: Ifade {
                tur: IfadeTuru::Indeks(
                    Box::new(Ifade {
                        tur: IfadeTuru::Isim(YAKALANANLAR.into()),
                        konum: k,
                        tip: parametreler[0].1.clone(),
                    }),
                    Box::new(Ifade {
                        tur: IfadeTuru::Sayi(i as i64),
                        konum: k,
                        tip: Tip::Sayi,
                    }),
                ),
                konum: k,
                tip: tip.clone(),
            },
            konum: k,
        })
        .collect();
    govde.extend(olay.govde.iter().cloned());
    Islev {
        ad: format!("‹olay›{no}"),
        parametreler,
        haller: Vec::new(),
        donus: Some(Tip::Bos),
        govde,
        konum: k,
        yereller,
        rota: None,
        arayuz: false,
    }
}

/// `ohc_ciz()`: olay listesini sıfırlar ve arayüzü çizer (JavaScript ağacı kurar).
fn ciz_islevi(o: &Ortak) -> Function {
    let mut f = Function::new([]);
    for k in [
        K::GlobalGet(G_GENEL),
        K::Call(o.calisma["ohc_liste_yeni"].0),
        K::I64Store(bellek(o.olay_yuvasi * 8)),
        K::Call(o.islevler[ARAYUZ_ISLEVI].0),
        K::End,
    ] {
        f.instruction(&k);
    }
    f
}

/// `ohc_olay(sıra, değer)`: çizimde kaydedilen olayın işlevini, yakalanan
/// değerler ve kullanıcının girdiği değerle (metin ya da 0) çağırır.
fn olay_islevi(o: &Ortak, olay_turu: u32) -> Function {
    let liste_al = o.calisma["ohc_liste_al"].0;
    let mut f = Function::new([(1, ValType::I32)]);
    let kayit = |tek: bool| {
        let mut v = vec![
            K::GlobalGet(G_GENEL),
            K::I64Load(bellek(o.olay_yuvasi * 8)),
            K::LocalGet(0),
            K::I64ExtendI32U,
            K::I64Const(1),
            K::I64Shl,
        ];
        if tek {
            v.extend([K::I64Const(1), K::I64Or]);
        }
        v.extend([K::I64Const(0), K::Call(liste_al)]);
        v
    };
    let mut kod = kayit(false);
    kod.extend([K::I32WrapI64, K::LocalSet(2)]);
    kod.extend(kayit(true));
    kod.extend([
        K::LocalGet(1),
        K::I64ExtendI32U,
        K::LocalGet(2),
        K::CallIndirect {
            type_index: olay_turu,
            table_index: 0,
        },
        K::End,
    ]);
    for k in &kod {
        f.instruction(k);
    }
    f
}

/// `donus`: `None` ana program, `Some(true)` değer döndüren işlev.
/// `dene_govdesi`: `dene:` bloğunun işlevi ise çerçevedeki değişkenler ve asıl
/// işlevin dönüşü.
fn islev_uret(
    o: &mut Ortak,
    yereller: &[(String, Tip)],
    parametreler: &[(String, Tip)],
    govde: &[Deyim],
    donus: Option<bool>,
    dene_govdesi: Option<(&[String], Option<bool>)>,
) -> Result<Function, String> {
    let param_sayisi = parametreler.len() as u32;
    let mut u = Uretici {
        o,
        kod: Vec::new(),
        degiskenler: HashMap::new(),
        param_sayisi,
        cerceve: param_sayisi,
        yerel_sayisi: 0,
        bos_yereller: Vec::new(),
        yuva_sayisi: 0,
        bos_yuvalar: Vec::new(),
        derinlik: 0,
        donguler: Vec::new(),
        donus,
        govde: dene_govdesi.map(|(_, d)| d),
        k: [0; 3],
    };
    u.k = [u.yeni_yerel(), u.yeni_yerel(), u.yeni_yerel()];

    // Yönetilen parametreler giriş kısmında yuvalarına kopyalanır.
    let mut kopyalar = Vec::new();
    for (ad, tip) in yereller {
        let param = parametreler.iter().position(|(p, _)| p == ad);
        let yer = match (param, yonetilen(tip)) {
            (Some(i), false) => Yer::Yerel(i as u32),
            (Some(i), true) => {
                let y = u.yuva_al();
                kopyalar.push((i as u32, y));
                Yer::Yuva(y)
            }
            (None, false) => Yer::Yerel(u.yeni_yerel()),
            (None, true) => Yer::Yuva(u.yuva_al()),
        };
        u.degiskenler.insert(ad.clone(), yer);
    }
    if let Some((yakalananlar, _)) = dene_govdesi {
        for (i, ad) in yakalananlar.iter().enumerate() {
            u.degiskenler
                .insert(ad.clone(), Yer::Dis(8 * (i as u32 + 1)));
        }
    }

    if donus.is_none() {
        // İç içe modellerin tanımları çalışma zamanında adla bulunur.
        let mut adlar: Vec<String> = u.o.modeller.keys().cloned().collect();
        adlar.sort();
        for ad in adlar {
            let t = u.o.tanim(&ad)?;
            u.yap("ohc_model_tanimla", &[Arg::Metin(t)])?;
        }
    }
    for d in govde {
        u.deyim(d)?;
    }
    // Gövdenin sonuna düşülürse
    u.cerceveyi_birak();
    if donus == Some(true) {
        u.kod.push(K::I64Const(DENE_SONA_ERDI));
    }

    // Giriş: çerçeveyi kur, sıfırla, parametreleri yerleştir, güvenli nokta.
    let mut giris: Vec<K<'static>> = Vec::new();
    if donus.is_none() {
        u.ana_hazirlik(&mut giris);
    }
    let boyut = (u.yuva_sayisi * 8) as i32;
    giris.extend([
        K::GlobalGet(G_TEPE),
        K::LocalTee(u.cerceve),
        K::I32Const(boyut),
        K::I32Add,
        K::GlobalSet(G_TEPE),
    ]);
    if boyut > 0 {
        let tasti = u.o.calisma["ohc_yigin_tasti"].0;
        giris.extend([
            K::GlobalGet(G_TEPE),
            K::GlobalGet(G_SINIR),
            K::I32GtU,
            K::If(BlockType::Empty),
            K::Call(tasti),
            K::End,
            K::LocalGet(u.cerceve),
            K::I32Const(0),
            K::I32Const(boyut),
            K::MemoryFill(0),
        ]);
    }
    for (param, yuva) in kopyalar {
        giris.extend([
            K::LocalGet(u.cerceve),
            K::LocalGet(param),
            K::I64Store(bellek(yuva * 8)),
        ]);
    }
    giris.extend(u.guvenli_nokta_kodu());

    let mut f = Function::new([(1, ValType::I32), (u.yerel_sayisi, ValType::I64)]);
    for k in giris.iter().chain(&u.kod) {
        f.instruction(k);
    }
    f.instruction(&K::End);
    Ok(f)
}

struct Uretici<'a> {
    o: &'a mut Ortak,
    kod: Vec<K<'static>>,
    degiskenler: HashMap<String, Yer>,
    param_sayisi: u32,
    /// Çerçevenin başını tutan i32 yereli
    cerceve: u32,
    yerel_sayisi: u32,
    bos_yereller: Vec<u32>,
    yuva_sayisi: u32,
    bos_yuvalar: Vec<u32>,
    /// Açık blok (block/loop/if) sayısı
    derinlik: u32,
    /// Döngüler: (sürdür hedefinin derinliği, dur hedefinin derinliği)
    donguler: Vec<(u32, u32)>,
    donus: Option<bool>,
    /// `dene:` bloğunun işlevinde: asıl işlevin dönüşü
    govde: Option<Option<bool>>,
    /// Düz kod parçalarında (iç içe değerlendirme olmadan) kullanılan yereller
    k: [u32; 3],
}

impl Uretici<'_> {
    fn yeni_yerel(&mut self) -> u32 {
        // 0..param: parametreler, param: çerçeve (i32), sonra i64 yereller
        let y = self.param_sayisi + 1 + self.yerel_sayisi;
        self.yerel_sayisi += 1;
        y
    }

    fn yerel_al(&mut self) -> u32 {
        match self.bos_yereller.pop() {
            Some(y) => y,
            None => self.yeni_yerel(),
        }
    }

    fn yuva_al(&mut self) -> u32 {
        match self.bos_yuvalar.pop() {
            Some(y) => y,
            None => {
                self.yuva_sayisi += 1;
                self.yuva_sayisi - 1
            }
        }
    }

    fn birak(&mut self, y: Yer) {
        match y {
            Yer::Yuva(s) => self.bos_yuvalar.push(s),
            Yer::Yerel(l) => self.bos_yereller.push(l),
            Yer::Dis(_) => {}
        }
    }

    /// `dene:` bloğunun işlevinde çevreleyen çerçevenin adresi (i32)
    fn dis_cerceve(&mut self) {
        self.e(K::LocalGet(0));
        self.e(K::I32WrapI64);
    }

    fn e(&mut self, k: K<'static>) {
        self.kod.push(k);
    }

    fn sabit(&mut self, n: i64) {
        self.e(K::I64Const(n));
    }

    fn ac(&mut self, k: K<'static>) -> u32 {
        self.e(k);
        self.derinlik += 1;
        self.derinlik
    }

    fn kapat(&mut self) {
        self.e(K::End);
        self.derinlik -= 1;
    }

    /// Derinliği `hedef` olan bloğa dallanma
    fn dallan(&mut self, hedef: u32, kosullu: bool) {
        let n = self.derinlik - hedef;
        self.e(if kosullu { K::BrIf(n) } else { K::Br(n) });
    }

    /// Yığıttaki değeri bir ara değer yerine kaydeder; `korunmali` ise gölge
    /// yığıta (bir Orhunca çağrısı boyunca yaşayacak yönetilen değer).
    fn sakla(&mut self, korunmali: bool) -> Yer {
        if korunmali {
            let y = self.yuva_al();
            let k = self.k[0];
            self.e(K::LocalSet(k));
            self.e(K::LocalGet(self.cerceve));
            self.e(K::LocalGet(k));
            self.e(K::I64Store(bellek(y * 8)));
            Yer::Yuva(y)
        } else {
            let l = self.yerel_al();
            self.e(K::LocalSet(l));
            Yer::Yerel(l)
        }
    }

    fn yukle(&mut self, y: Yer) {
        match y {
            Yer::Yuva(s) => {
                self.e(K::LocalGet(self.cerceve));
                self.e(K::I64Load(bellek(s * 8)));
            }
            Yer::Yerel(l) => self.e(K::LocalGet(l)),
            Yer::Dis(o) => {
                self.dis_cerceve();
                self.e(K::I64Load(bellek(o)));
            }
        }
    }

    fn degisken_oku(&mut self, ad: &str) -> Result<(), String> {
        if let Some(y) = self.degiskenler.get(ad).copied() {
            self.yukle(y);
            return Ok(());
        }
        let g = *self
            .o
            .genel
            .get(ad)
            .ok_or_else(|| format!("tanımsız değişken '{ad}'"))?;
        self.e(K::GlobalGet(G_GENEL));
        self.e(K::I64Load(bellek(g * 8)));
        Ok(())
    }

    /// `deger` yığıta bir değer koyar; değer değişkene yazılır.
    fn degiskene_yaz(
        &mut self,
        ad: &str,
        deger: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        let Some(y) = self.degiskenler.get(ad).copied() else {
            let g = *self
                .o
                .genel
                .get(ad)
                .ok_or_else(|| format!("tanımsız değişken '{ad}'"))?;
            self.e(K::GlobalGet(G_GENEL));
            deger(self)?;
            self.e(K::I64Store(bellek(g * 8)));
            return Ok(());
        };
        match y {
            Yer::Yuva(s) => {
                self.e(K::LocalGet(self.cerceve));
                deger(self)?;
                self.e(K::I64Store(bellek(s * 8)));
            }
            Yer::Yerel(l) => {
                deger(self)?;
                self.e(K::LocalSet(l));
            }
            Yer::Dis(o) => {
                self.dis_cerceve();
                deger(self)?;
                self.e(K::I64Store(bellek(o)));
            }
        }
        Ok(())
    }

    fn cerceveyi_birak(&mut self) {
        self.e(K::LocalGet(self.cerceve));
        self.e(K::GlobalSet(G_TEPE));
    }

    /// Toplama gerekiyorsa çöp toplayıcıyı çalıştırır.
    fn guvenli_nokta_kodu(&self) -> [K<'static>; 7] {
        [
            K::GlobalGet(G_BAYRAK),
            K::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            K::If(BlockType::Empty),
            K::GlobalGet(G_TEPE),
            K::I64ExtendI32U,
            K::Call(self.o.calisma["ohc_guvenli_nokta"].0),
            K::End,
        ]
    }

    fn guvenli_nokta(&mut self) {
        let k = self.guvenli_nokta_kodu();
        self.kod.extend(k);
    }

    /// Programın başında: sabit metinleri belleğe kopyalar, gölge yığıtı kurar.
    fn ana_hazirlik(&mut self, giris: &mut Vec<K<'static>>) {
        let c = |ad: &str| self.o.calisma[ad].0;
        // Metinlerin boyu ancak tüm işlevler (`dene:` blokları dahil) üretilince bilinir.
        giris.extend([
            K::GlobalGet(G_METIN_BOYU),
            K::I64ExtendI32U,
            K::I64Const(1),
            K::I64Add,
            K::Call(c("ohc_wasm_ayir")),
            K::GlobalSet(G_METINLER),
            K::GlobalGet(G_METINLER),
            K::I32WrapI64,
            K::I32Const(0),
            K::GlobalGet(G_METIN_BOYU),
            K::MemoryInit {
                mem: 0,
                data_index: 0,
            },
            K::DataDrop(0),
            K::I64Const(GOLGE_BOYUTU),
            K::Call(c("ohc_wasm_golge")),
            K::I32WrapI64,
            K::GlobalSet(G_TEPE),
            K::GlobalGet(G_TEPE),
            K::I32Const(GOLGE_BOYUTU as i32),
            K::I32Add,
            K::GlobalSet(G_SINIR),
            K::Call(c("ohc_wasm_bayrak")),
            K::I32WrapI64,
            K::GlobalSet(G_BAYRAK),
            // Genel bölge: durum değişkenleri ve olay listesi (gölge yığıtın dibinde,
            // çöp toplayıcı her zaman tarar).
            K::GlobalGet(G_TEPE),
            K::GlobalSet(G_GENEL),
            K::GlobalGet(G_TEPE),
            K::I32Const(((self.o.olay_yuvasi + 1) * 8) as i32),
            K::I32Add,
            K::GlobalSet(G_TEPE),
        ]);
    }

    fn metin_sabiti(&mut self, m: &str) {
        let y = self.o.metin(m);
        self.e(K::GlobalGet(G_METINLER));
        self.e(K::I64Const(y as i64));
        self.e(K::I64Add);
    }

    /// İfadenin değerlendirilmesi bir güvenli noktaya (Orhunca işlev çağrısı)
    /// ulaşabilir mi?
    fn tetikler(&self, e: &Ifade) -> bool {
        match &e.tur {
            IfadeTuru::Cagri(ad, arg) => {
                self.o.islevler.contains_key(ad) || arg.iter().any(|a| self.tetikler(a))
            }
            IfadeTuru::FiilCagri(..) => true,
            IfadeTuru::Liste(o) => o.iter().any(|a| self.tetikler(a)),
            IfadeTuru::Sozluk(c) => c.iter().any(|(a, d)| self.tetikler(a) || self.tetikler(d)),
            IfadeTuru::Ikili(_, a, b) | IfadeTuru::Indeks(a, b) => {
                self.tetikler(a) || self.tetikler(b)
            }
            IfadeTuru::Tekli(_, a) | IfadeTuru::Alan(a, ..) => self.tetikler(a),
            IfadeTuru::Kurucu(_, alanlar) => alanlar.iter().any(|(_, a)| self.tetikler(a)),
            IfadeTuru::Metod(a, _, arg) => self.tetikler(a) || arg.iter().any(|x| self.tetikler(x)),
            IfadeTuru::Sayi(_)
            | IfadeTuru::Ondalik(_)
            | IfadeTuru::Metin(_)
            | IfadeTuru::Mantik(_)
            | IfadeTuru::Isim(_)
            | IfadeTuru::ModelAdi(_) => false,
        }
    }

    fn arg_tetikler(&self, a: &Arg) -> bool {
        match a {
            Arg::I(e) | Arg::Metne(e) => self.tetikler(e),
            _ => false,
        }
    }

    /// Bu değer hesaplandıktan sonra bir güvenli noktadan geçilecekse gölge
    /// yığıtta saklanmalı mı? Değişkenler zaten oradadır, sabitler yönetilmez.
    fn korunmali(&self, a: &Arg) -> bool {
        match a {
            Arg::I(e) => yonetilen(&e.tip) && !yan_etkisiz(e),
            // Metne çevirme yeni bir metin oluşturur; metin değişkeni ya da sabiti zaten güvende.
            Arg::Metne(e) => e.tip != Tip::Metin || !yan_etkisiz(e),
            _ => false,
        }
    }

    fn arg_yukle(&mut self, a: &Arg) -> Result<(), String> {
        match a {
            Arg::I(e) => self.ifade(e)?,
            Arg::Metne(e) => {
                self.ifade(e)?;
                if e.tip != Tip::Metin {
                    self.sabit(e.tip.kod());
                    self.cz("ohc_metne_cevir");
                }
            }
            Arg::S(n) => self.sabit(*n),
            Arg::Metin(m) => self.metin_sabiti(m),
            Arg::Hazir(y) => self.yukle(*y),
        }
        Ok(())
    }

    /// Değerleri sırayla yığıta koyar. Sonra gelen bir değer bir güvenli noktaya
    /// ulaşabiliyorsa, ondan önce hesaplanan yönetilen değerler o sırada gölge
    /// yığıtta saklanır.
    fn argumanlar(&mut self, args: &[Arg]) -> Result<(), String> {
        let son = args.iter().rposition(|a| self.arg_tetikler(a));
        let k = match son {
            Some(k) if k > 0 => k,
            _ => {
                for a in args {
                    self.arg_yukle(a)?;
                }
                return Ok(());
            }
        };
        let mut saklanan = Vec::new();
        for (i, a) in args[..=k].iter().enumerate() {
            if matches!(a, Arg::I(_) | Arg::Metne(_)) {
                self.arg_yukle(a)?;
                let korunmali = i < k && self.korunmali(a);
                saklanan.push(Some(self.sakla(korunmali)));
            } else {
                saklanan.push(None);
            }
        }
        for (a, s) in args[..=k].iter().zip(saklanan) {
            match s {
                Some(y) => {
                    self.yukle(y);
                    self.birak(y);
                }
                None => self.arg_yukle(a)?,
            }
        }
        for a in &args[k + 1..] {
            self.arg_yukle(a)?;
        }
        Ok(())
    }

    /// Çalışma zamanı işlevini çağırır; değer döndürüyorsa `true`.
    fn cz(&mut self, ad: &str) -> bool {
        let (sira, doner) = self.o.calisma[ad];
        self.e(K::Call(sira));
        doner
    }

    /// İfade içinde çalışma zamanı çağrısı: yığıta her zaman bir değer koyar.
    fn cagri(&mut self, ad: &str, args: &[Arg]) -> Result<(), String> {
        self.argumanlar(args)?;
        if !self.cz(ad) {
            self.sabit(0);
        }
        Ok(())
    }

    /// Deyim olarak çalışma zamanı çağrısı: yığıtta değer bırakmaz.
    fn yap(&mut self, ad: &str, args: &[Arg]) -> Result<(), String> {
        self.argumanlar(args)?;
        if self.cz(ad) {
            self.e(K::Drop);
        }
        Ok(())
    }

    /// `ilk` önce hesaplanmalı ama `ikinci`den sonra yığıta konmalı (yerel
    /// derleyicideki değerlendirme sırası). İkinci yan etkisizse sıra değişebilir.
    fn once_hesapla<'e>(
        &mut self,
        ilk: Arg<'e>,
        ikinci: &Ifade,
    ) -> Result<(Arg<'e>, Option<Yer>), String> {
        if yan_etkisiz(ikinci) {
            return Ok((ilk, None));
        }
        self.arg_yukle(&ilk)?;
        let korunmali = self.korunmali(&ilk) && self.tetikler(ikinci);
        let y = self.sakla(korunmali);
        Ok((Arg::Hazir(y), Some(y)))
    }

    fn blok(&mut self, govde: &[Deyim]) -> Result<(), String> {
        for d in govde {
            self.deyim(d)?;
        }
        Ok(())
    }

    /// Mantık değerini i32 koşula çevirir.
    fn kosul(&mut self, e: &Ifade) -> Result<(), String> {
        self.ifade(e)?;
        self.sabit(0);
        self.e(K::I64Ne);
        Ok(())
    }

    fn deyim(&mut self, d: &Deyim) -> Result<(), String> {
        match d {
            Deyim::Atama { hedef, deger, .. } => {
                self.degiskene_yaz(hedef, |u| u.ifade(deger))?;
            }
            Deyim::IndeksAtama {
                liste,
                indeks,
                deger,
            } => {
                if let Tip::Sozluk(..) = liste.tip {
                    let kod = liste.tip.ic_kod();
                    self.yap(
                        "ohc_sozluk_koy",
                        &[Arg::I(liste), Arg::I(indeks), Arg::I(deger), Arg::S(kod)],
                    )?;
                } else {
                    let s = indeks.konum.satir as i64;
                    self.yap(
                        "ohc_liste_koy",
                        &[Arg::I(liste), Arg::I(indeks), Arg::I(deger), Arg::S(s)],
                    )?;
                }
            }
            Deyim::AlanAtama {
                nesne,
                sira,
                deger,
                konum,
                ..
            } => {
                self.yap(
                    "ohc_alan_koy",
                    &[
                        Arg::I(nesne),
                        Arg::S(ILK_ALAN + *sira as i64),
                        Arg::I(deger),
                        Arg::S(konum.satir as i64),
                    ],
                )?;
            }
            Deyim::Cikar { oge, liste } => {
                let (o, y) = self.once_hesapla(Arg::I(oge), liste)?;
                let kod = liste.tip.ic_kod();
                self.yap("ohc_liste_cikar", &[Arg::I(liste), o, Arg::S(kod)])?;
                if let Some(y) = y {
                    self.birak(y);
                }
            }
            Deyim::DosyayaYaz { deger, yol } => {
                let (m, y) = self.once_hesapla(Arg::Metne(deger), yol)?;
                let s = yol.konum.satir as i64;
                self.yap("ohc_dosyaya_yaz", &[Arg::I(yol), m, Arg::S(0), Arg::S(s)])?;
                if let Some(y) = y {
                    self.birak(y);
                }
            }
            Deyim::Yaz(i) => {
                self.yap("ohc_yaz", &[Arg::I(i), Arg::S(i.tip.kod())])?;
            }
            Deyim::Ekle { oge, liste } => {
                let (o, y) = self.once_hesapla(Arg::I(oge), liste)?;
                self.yap("ohc_liste_ekle", &[Arg::I(liste), o])?;
                if let Some(y) = y {
                    self.birak(y);
                }
            }
            Deyim::Sirala(l) => {
                let kod = match &l.tip {
                    Tip::Liste(ic) => ic.kod(),
                    _ => 0,
                };
                self.yap("ohc_liste_sirala", &[Arg::I(l), Arg::S(kod)])?;
            }
            Deyim::Eger {
                kosul,
                govde,
                degilse,
            } => {
                self.kosul(kosul)?;
                self.ac(K::If(BlockType::Empty));
                self.blok(govde)?;
                if !degilse.is_empty() {
                    self.e(K::Else);
                    self.blok(degilse)?;
                }
                self.kapat();
            }
            Deyim::Surece { kosul, govde } => {
                let son = self.ac(K::Block(BlockType::Empty));
                let bas = self.ac(K::Loop(BlockType::Empty));
                self.guvenli_nokta();
                self.kosul(kosul)?;
                self.e(K::I32Eqz);
                self.dallan(son, true);
                self.donguler.push((bas, son));
                self.blok(govde)?;
                self.donguler.pop();
                self.dallan(bas, false);
                self.kapat();
                self.kapat();
            }
            Deyim::HerAralik {
                degisken,
                bas,
                son,
                govde,
                ..
            } => {
                // Ayrı bir sayaç: gövde döngü değişkenini değiştirse de döngü bozulmaz.
                let sayac = self.yerel_al();
                let sinir = self.yerel_al();
                self.ifade(bas)?;
                self.e(K::LocalSet(sayac));
                self.ifade(son)?;
                self.e(K::LocalSet(sinir));
                self.dongu(
                    |u| {
                        u.e(K::LocalGet(sayac));
                        u.e(K::LocalGet(sinir));
                        u.e(K::I64GtS);
                        Ok(())
                    },
                    |u| {
                        u.degiskene_yaz(degisken, |u| {
                            u.e(K::LocalGet(sayac));
                            Ok(())
                        })
                    },
                    sayac,
                    govde,
                )?;
                self.bos_yereller.extend([sinir, sayac]);
            }
            Deyim::HerListe {
                degisken,
                liste,
                govde,
                ..
            } => {
                // Metinde harfler, sözlükte anahtarlar gezilir.
                self.ifade(liste)?;
                match liste.tip {
                    Tip::Metin => {
                        self.cz("ohc_harfler");
                    }
                    Tip::Sozluk(..) => {
                        self.cz("ohc_sozluk_anahtarlar");
                    }
                    _ => {}
                }
                let l = self.sakla(true);
                let sayac = self.yerel_al();
                self.sabit(0);
                self.e(K::LocalSet(sayac));
                let satir = liste.konum.satir as i64;
                self.dongu(
                    |u| {
                        u.e(K::LocalGet(sayac));
                        u.yukle(l);
                        u.cz("ohc_liste_uzunluk");
                        u.e(K::I64GeS);
                        Ok(())
                    },
                    |u| {
                        u.degiskene_yaz(degisken, |u| {
                            u.yukle(l);
                            u.e(K::LocalGet(sayac));
                            u.sabit(satir);
                            u.cz("ohc_liste_al");
                            Ok(())
                        })
                    },
                    sayac,
                    govde,
                )?;
                self.birak(l);
                self.bos_yereller.push(sayac);
            }
            Deyim::Dondur(deger, _) if self.govde.is_some() => {
                if let Some(i) = deger {
                    if self.govde == Some(Some(true)) {
                        self.dis_cerceve();
                        self.ifade(i)?;
                        self.e(K::I64Store(bellek(0)));
                    } else {
                        self.ifade(i)?;
                        self.e(K::Drop);
                    }
                }
                self.cerceveyi_birak();
                self.sabit(DENE_DONDUR);
                self.e(K::Return);
            }
            Deyim::Dur(_) | Deyim::Surdur(_)
                if self.govde.is_some() && self.donguler.is_empty() =>
            {
                // Döngü bloğun dışında: çevreleyen işlev dallanır.
                self.cerceveyi_birak();
                self.sabit(if matches!(d, Deyim::Dur(_)) {
                    DENE_DUR
                } else {
                    DENE_SURDUR
                });
                self.e(K::Return);
            }
            Deyim::Dene {
                govde,
                degisken,
                yakala,
                ..
            } => self.dene(govde, degisken.as_deref(), yakala)?,
            Deyim::Dondur(deger, _) => {
                match (deger, self.donus) {
                    (Some(i), Some(true)) => self.ifade(i)?,
                    (Some(i), _) => {
                        // Değeri olmayan (boş) işlev çağrısı döndürülüyor.
                        self.ifade(i)?;
                        self.e(K::Drop);
                    }
                    (None, Some(true)) => self.sabit(0),
                    (None, _) => {}
                }
                self.cerceveyi_birak();
                self.e(K::Return);
            }
            Deyim::Dur(_) => {
                let (_, son) = *self.donguler.last().ok_or("döngü dışında 'dur'")?;
                self.dallan(son, false);
            }
            Deyim::Surdur(_) => {
                let (devam, _) = *self.donguler.last().ok_or("döngü dışında 'sürdür'")?;
                self.dallan(devam, false);
            }
            Deyim::IfadeDeyimi(i) => {
                self.ifade(i)?;
                self.e(K::Drop);
            }
            Deyim::Oge(o) => self.oge(o)?,
        }
        Ok(())
    }

    /// `dene:` bloğu: blok ayrı bir işleve çevrilir, JavaScript yükleyicisi
    /// (`dn.dene`) onu tablodan çağırır. Bloğun kullandığı değişkenler önce gölge
    /// yığıttaki bitişik yuvalara konur, sonra oradan geri okunur.
    fn dene(
        &mut self,
        govde: &[Deyim],
        degisken: Option<&str>,
        yakala: &[Deyim],
    ) -> Result<(), String> {
        let ice = self.o.dene.ok_or("dene içe aktarılmamış")?;
        let yakalananlar: Vec<String> = gecen_adlar(govde)
            .into_iter()
            .filter(|a| self.degiskenler.contains_key(a))
            .collect();
        let tablo = self.o.govde_tablo_basi + self.o.govde_sayisi;
        self.o.govde_sayisi += 1;
        self.o.govdeler.push(DeneGovdesi {
            govde: govde.to_vec(),
            yakalananlar: yakalananlar.clone(),
            dis_donus: self.govde.unwrap_or(self.donus),
        });

        // Bitişik yuvalar (yeniden kullanılmaz)
        let taban = self.yuva_sayisi;
        self.yuva_sayisi += yakalananlar.len() as u32 + 1;
        self.e(K::LocalGet(self.cerceve));
        self.sabit(0);
        self.e(K::I64Store(bellek(taban * 8)));
        for (i, a) in yakalananlar.iter().enumerate() {
            self.e(K::LocalGet(self.cerceve));
            self.degisken_oku(a)?;
            self.e(K::I64Store(bellek((taban + 1 + i as u32) * 8)));
        }
        // Hata olursa gölge yığıtın tepesi geri alınır.
        let tepe = self.yerel_al();
        self.e(K::GlobalGet(G_TEPE));
        self.e(K::I64ExtendI32U);
        self.e(K::LocalSet(tepe));
        self.e(K::I32Const(tablo as i32));
        self.e(K::LocalGet(self.cerceve));
        self.e(K::I32Const((taban * 8) as i32));
        self.e(K::I32Add);
        self.e(K::I64ExtendI32U);
        self.e(K::Call(ice));
        let kod = self.yerel_al();
        self.e(K::LocalSet(kod));
        self.e(K::LocalGet(tepe));
        self.e(K::I32WrapI64);
        self.e(K::GlobalSet(G_TEPE));
        for (i, a) in yakalananlar.iter().enumerate() {
            let c = self.cerceve;
            self.degiskene_yaz(a, |u| {
                u.e(K::LocalGet(c));
                u.e(K::I64Load(bellek((taban + 1 + i as u32) * 8)));
                Ok(())
            })?;
        }

        self.e(K::LocalGet(kod));
        self.sabit(0);
        self.e(K::I64LtS);
        self.ac(K::If(BlockType::Empty));
        if let Some(d) = degisken {
            self.degiskene_yaz(d, |u| {
                u.cz("ohc_hata_mesaji");
                Ok(())
            })?;
        }
        self.blok(yakala)?;
        self.e(K::Else);
        // döndür
        self.e(K::LocalGet(kod));
        self.sabit(DENE_DONDUR);
        self.e(K::I64Eq);
        self.ac(K::If(BlockType::Empty));
        match (self.govde, self.donus) {
            (Some(_), _) => {
                self.dis_cerceve();
                self.e(K::LocalGet(self.cerceve));
                self.e(K::I64Load(bellek(taban * 8)));
                self.e(K::I64Store(bellek(0)));
                self.cerceveyi_birak();
                self.sabit(DENE_DONDUR);
            }
            (None, Some(true)) => {
                self.e(K::LocalGet(self.cerceve));
                self.e(K::I64Load(bellek(taban * 8)));
                self.cerceveyi_birak();
            }
            (None, _) => self.cerceveyi_birak(),
        }
        self.e(K::Return);
        self.kapat();
        // dur / sürdür (döngü bloğun dışındaysa)
        for (k, dur) in [(DENE_DUR, true), (DENE_SURDUR, false)] {
            let hedef = self
                .donguler
                .last()
                .map(|(devam, son)| if dur { *son } else { *devam });
            if hedef.is_none() && self.govde.is_none() {
                continue;
            }
            self.e(K::LocalGet(kod));
            self.sabit(k);
            self.e(K::I64Eq);
            self.ac(K::If(BlockType::Empty));
            match hedef {
                Some(h) => self.dallan(h, false),
                None => {
                    self.cerceveyi_birak();
                    self.sabit(k);
                    self.e(K::Return);
                }
            }
            self.kapat();
        }
        self.kapat();
        self.bos_yereller.extend([kod, tepe]);
        Ok(())
    }

    fn ui(&mut self, ad: &str) {
        let sira = self.o.ui[ad];
        self.e(K::Call(sira));
    }

    /// Yığıta bir metnin adresini (i32) koyar.
    fn metin_adresi(&mut self, m: &str) {
        self.metin_sabiti(m);
        self.e(K::I32WrapI64);
    }

    /// Öğenin bir özelliği: ad ve metne çevrilmiş değer JavaScript ağacına gider.
    fn ozellik(&mut self, ad: &str, deger: &Ifade, liste: bool) -> Result<(), String> {
        self.metin_adresi(ad);
        if liste {
            self.ifade(deger)?;
            self.sabit(deger.tip.kod());
            self.cz("ohc_json");
        } else {
            self.arg_yukle(&Arg::Metne(deger))?;
        }
        self.e(K::I32WrapI64);
        self.ui("ozellik");
        Ok(())
    }

    /// Arayüz öğesi: JavaScript'teki ağaca bir düğüm açar, özelliklerini ve
    /// olaylarını ekler, içindekileri çizer ve düğümü kapatır.
    fn oge(&mut self, o: &Oge) -> Result<(), String> {
        let tanim = arayuz::oge(&o.ad).ok_or_else(|| format!("bilinmeyen öğe '{}'", o.ad))?;
        self.metin_adresi(&o.ad);
        self.ui("ac");
        for (a, (ad, beklenen)) in o.argumanlar.iter().zip(tanim.degerler) {
            self.ozellik(
                ad,
                a,
                matches!(beklenen, Beklenen::MetinListesi | Beklenen::Veri),
            )?;
        }
        for (ad, d) in &o.secenekler {
            self.ozellik(ad, d, false)?;
        }
        for olay in o.baglama.iter().chain(&o.olay) {
            self.olay_kaydet(o, olay)?;
        }
        self.blok(&o.cocuklar)?;
        self.ui("kapat");
        Ok(())
    }

    /// Olayı bu çizimin olay listesine ekler: [işlevin tablodaki sırası,
    /// yakalanan değerler]. JavaScript, olay olunca listedeki sırayla `ohc_olay`ı çağırır.
    fn olay_kaydet(&mut self, o: &Oge, olay: &Olay) -> Result<(), String> {
        let tablo = *self
            .o
            .olay_sirasi
            .get(&(o.konum, olay.ad.clone()))
            .ok_or("olay işlevi bulunamadı")?;
        // Yakalanan değerlerin listesi (güvenli nokta yok: yerelde durabilir)
        let yakalanan = if olay.yakalananlar.is_empty() {
            None
        } else {
            self.cz("ohc_liste_yeni");
            let l = self.sakla(false);
            for (ad, _) in &olay.yakalananlar {
                self.yukle(l);
                self.degisken_oku(ad)?;
                self.cz("ohc_liste_ekle");
            }
            Some(l)
        };
        let g = self.o.olay_yuvasi * 8;
        // Olayın sırası: listenin o anki uzunluğunun yarısı
        self.metin_adresi(match olay.ad.as_str() {
            "bağ" => "bağ",
            a => a,
        });
        self.e(K::GlobalGet(G_GENEL));
        self.e(K::I64Load(bellek(g)));
        self.cz("ohc_liste_uzunluk");
        self.sabit(1);
        self.e(K::I64ShrU);
        self.e(K::I32WrapI64);
        // Listeye ekle
        self.e(K::GlobalGet(G_GENEL));
        self.e(K::I64Load(bellek(g)));
        self.sabit(tablo as i64);
        self.cz("ohc_liste_ekle");
        self.e(K::GlobalGet(G_GENEL));
        self.e(K::I64Load(bellek(g)));
        match yakalanan {
            Some(l) => {
                self.yukle(l);
                self.birak(l);
            }
            None => self.sabit(0),
        }
        self.cz("ohc_liste_ekle");
        self.ui("olay");
        Ok(())
    }

    /// Sayaçlı döngü: `bitti` yığıta i32 koyar (doğruysa çıkılır), `ata` döngü
    /// değişkenini yazar; gövdeden sonra sayaç bir artar.
    fn dongu(
        &mut self,
        bitti: impl FnOnce(&mut Self) -> Result<(), String>,
        ata: impl FnOnce(&mut Self) -> Result<(), String>,
        sayac: u32,
        govde: &[Deyim],
    ) -> Result<(), String> {
        let son = self.ac(K::Block(BlockType::Empty));
        let bas = self.ac(K::Loop(BlockType::Empty));
        self.guvenli_nokta();
        bitti(self)?;
        self.dallan(son, true);
        ata(self)?;
        let devam = self.ac(K::Block(BlockType::Empty));
        self.donguler.push((devam, son));
        self.blok(govde)?;
        self.donguler.pop();
        self.kapat();
        self.e(K::LocalGet(sayac));
        self.sabit(1);
        self.e(K::I64Add);
        self.e(K::LocalSet(sayac));
        self.dallan(bas, false);
        self.kapat();
        self.kapat();
        Ok(())
    }

    /// Yığıttaki iki i64'ü ondalık olarak işler: `islem` iki f64'ü bir f64'e çevirir.
    fn ondalik_islem(&mut self, islem: K<'static>) {
        let b = self.k[1];
        self.e(K::LocalSet(b));
        self.e(K::F64ReinterpretI64);
        self.e(K::LocalGet(b));
        self.e(K::F64ReinterpretI64);
        self.e(islem);
    }

    /// Tamsayı toplama/çıkarma/çarpma; taşmada çalışma hatası.
    fn tamsayi_islem(&mut self, op: IkiliOp, satir: i64) {
        let [a, b, r] = self.k;
        self.e(K::LocalSet(b));
        self.e(K::LocalSet(a));
        if op == IkiliOp::Carp {
            // İki taraf da 32 bite sığıyorsa çarpım taşamaz; yoksa çalışma zamanı denetler.
            for y in [a, b] {
                self.e(K::LocalGet(y));
                self.sabit(0x8000_0000);
                self.e(K::I64Add);
            }
            self.e(K::I64Or);
            self.sabit(32);
            self.e(K::I64ShrU);
            self.e(K::I64Eqz);
            self.e(K::If(BlockType::Result(ValType::I64)));
            self.e(K::LocalGet(a));
            self.e(K::LocalGet(b));
            self.e(K::I64Mul);
            self.e(K::Else);
            self.e(K::LocalGet(a));
            self.e(K::LocalGet(b));
            self.sabit(satir);
            self.cz("ohc_carp");
            self.e(K::End);
            return;
        }
        self.e(K::LocalGet(a));
        self.e(K::LocalGet(b));
        self.e(if op == IkiliOp::Topla {
            K::I64Add
        } else {
            K::I64Sub
        });
        self.e(K::LocalTee(r));
        // Toplama: (a ^ r) & (b ^ r) < 0; çıkarma: (a ^ b) & (a ^ r) < 0
        if op == IkiliOp::Topla {
            self.e(K::LocalGet(a));
            self.e(K::I64Xor);
            self.e(K::LocalGet(b));
            self.e(K::LocalGet(r));
            self.e(K::I64Xor);
        } else {
            self.e(K::LocalGet(a));
            self.e(K::I64Xor);
            self.e(K::LocalGet(a));
            self.e(K::LocalGet(b));
            self.e(K::I64Xor);
        }
        self.e(K::I64And);
        self.sabit(0);
        self.e(K::I64LtS);
        self.e(K::If(BlockType::Empty));
        self.sabit(satir);
        self.cz("ohc_tasma");
        self.e(K::End);
        self.e(K::LocalGet(r));
    }

    /// i32 karşılaştırma sonucunu mantık değerine (i64 0/1) çevirir.
    fn mantik(&mut self, k: K<'static>) {
        self.e(k);
        self.e(K::I64ExtendI32U);
    }

    fn ifade(&mut self, e: &Ifade) -> Result<(), String> {
        let satir = e.konum.satir as i64;
        match &e.tur {
            IfadeTuru::Sayi(n) => self.sabit(*n),
            IfadeTuru::Ondalik(n) => self.sabit(n.to_bits() as i64),
            IfadeTuru::Mantik(m) => self.sabit(*m as i64),
            IfadeTuru::Metin(m) => self.metin_sabiti(m),
            IfadeTuru::Isim(ad) => self.degisken_oku(ad)?,
            IfadeTuru::FiilCagri(ad, _) => {
                return Err(format!("'{ad}' fiil çağrısı denetimden geçmemiş"))
            }
            IfadeTuru::ModelAdi(m) => {
                return Err(format!("'{m}' model adı değer olarak kullanıldı"))
            }
            IfadeTuru::Kurucu(model, alanlar) if alanlar.is_empty() => {
                self.varsayilan_nesne(model)?;
            }
            IfadeTuru::Kurucu(model, alanlar) => {
                let degerler: Vec<&Ifade> = alanlar.iter().map(|(_, d)| d).collect();
                self.nesne_kur(model, &degerler)?;
            }
            IfadeTuru::Alan(nesne, _, sira) => {
                self.cagri(
                    "ohc_alan_al",
                    &[
                        Arg::I(nesne),
                        Arg::S(ILK_ALAN + *sira as i64),
                        Arg::S(satir),
                    ],
                )?;
            }
            IfadeTuru::Metod(alici, ad, arg) => self.metod(e, alici, ad, arg)?,
            IfadeTuru::Liste(ogeler) => {
                self.cz("ohc_liste_yeni");
                let korunmali = ogeler.iter().any(|o| self.tetikler(o));
                let l = self.sakla(korunmali);
                for o in ogeler {
                    self.yap("ohc_liste_ekle", &[Arg::Hazir(l), Arg::I(o)])?;
                }
                self.yukle(l);
                self.birak(l);
            }
            IfadeTuru::Sozluk(ciftler) => {
                self.cz("ohc_sozluk_yeni");
                let korunmali = ciftler
                    .iter()
                    .any(|(a, d)| self.tetikler(a) || self.tetikler(d));
                let s = self.sakla(korunmali);
                let kod = e.tip.ic_kod();
                for (a, d) in ciftler {
                    self.yap(
                        "ohc_sozluk_koy",
                        &[Arg::Hazir(s), Arg::I(a), Arg::I(d), Arg::S(kod)],
                    )?;
                }
                self.yukle(s);
                self.birak(s);
            }
            IfadeTuru::Tekli(TekliOp::Eksi, ic) if ic.tip == Tip::Ondalik => {
                self.ifade(ic)?;
                self.e(K::F64ReinterpretI64);
                self.e(K::F64Neg);
                self.e(K::I64ReinterpretF64);
            }
            IfadeTuru::Tekli(TekliOp::Eksi, ic) => {
                self.sabit(0);
                self.ifade(ic)?;
                self.e(K::I64Sub);
            }
            IfadeTuru::Tekli(TekliOp::Degil, ic) => {
                self.ifade(ic)?;
                self.sabit(1);
                self.e(K::I64Xor);
            }
            IfadeTuru::Ikili(op @ (IkiliOp::Ve | IkiliOp::Veya), sol, sag) => {
                // Kısa devre: 've' için sol yanlışsa, 'veya' için sol doğruysa sağ hesaplanmaz.
                self.kosul(sol)?;
                self.ac(K::If(BlockType::Result(ValType::I64)));
                if *op == IkiliOp::Ve {
                    self.ifade(sag)?;
                    self.e(K::Else);
                    self.sabit(0);
                } else {
                    self.sabit(1);
                    self.e(K::Else);
                    self.ifade(sag)?;
                }
                self.kapat();
            }
            IfadeTuru::Ikili(op, sol, sag) => self.ikili(e, *op, sol, sag)?,
            IfadeTuru::Indeks(l, i) => match l.tip {
                Tip::Metin => {
                    self.cagri("ohc_metin_harf", &[Arg::I(l), Arg::I(i), Arg::S(satir)])?
                }
                Tip::Sozluk(..) => {
                    let kod = l.tip.ic_kod();
                    self.cagri(
                        "ohc_sozluk_al",
                        &[Arg::I(l), Arg::I(i), Arg::S(kod), Arg::S(satir)],
                    )?
                }
                _ => self.cagri("ohc_liste_al", &[Arg::I(l), Arg::I(i), Arg::S(satir)])?,
            },
            IfadeTuru::Cagri(ad, arg) => self.cagri_ifadesi(e, ad, arg)?,
        }
        Ok(())
    }

    fn ikili(&mut self, e: &Ifade, op: IkiliOp, sol: &Ifade, sag: &Ifade) -> Result<(), String> {
        let satir = e.konum.satir as i64;
        let iki = [Arg::I(sol), Arg::I(sag)];
        match op {
            IkiliOp::Topla if e.tip == Tip::Metin => {
                self.cagri("ohc_metin_birlestir", &[Arg::Metne(sol), Arg::Metne(sag)])?;
            }
            // Denetçi karışık işlemlerde iki tarafı da ondalığa çevirmiştir.
            _ if sol.tip == Tip::Ondalik => match op {
                IkiliOp::Topla | IkiliOp::Cikar | IkiliOp::Carp => {
                    self.argumanlar(&iki)?;
                    self.ondalik_islem(match op {
                        IkiliOp::Topla => K::F64Add,
                        IkiliOp::Cikar => K::F64Sub,
                        _ => K::F64Mul,
                    });
                    self.e(K::I64ReinterpretF64);
                }
                IkiliOp::Bol => {
                    self.cagri(
                        "ohc_ondalik_bol",
                        &[Arg::I(sol), Arg::I(sag), Arg::S(satir)],
                    )?;
                }
                IkiliOp::Esit
                | IkiliOp::EsitDegil
                | IkiliOp::Kucuk
                | IkiliOp::Buyuk
                | IkiliOp::KucukEsit
                | IkiliOp::BuyukEsit => {
                    self.argumanlar(&iki)?;
                    self.ondalik_islem(match op {
                        IkiliOp::Esit => K::F64Eq,
                        IkiliOp::EsitDegil => K::F64Ne,
                        IkiliOp::Kucuk => K::F64Lt,
                        IkiliOp::Buyuk => K::F64Gt,
                        IkiliOp::KucukEsit => K::F64Le,
                        _ => K::F64Ge,
                    });
                    self.e(K::I64ExtendI32U);
                }
                _ => return Err(format!("ondalık için desteklenmeyen işlem {op:?}")),
            },
            IkiliOp::Topla | IkiliOp::Cikar | IkiliOp::Carp => {
                self.argumanlar(&iki)?;
                self.tamsayi_islem(op, satir);
            }
            _ if sol.tip == Tip::Metin && !matches!(op, IkiliOp::Esit | IkiliOp::EsitDegil) => {
                // Türk alfabesine göre karşılaştırma
                self.cagri("ohc_metin_kars", &iki)?;
                self.sabit(0);
                self.mantik(match op {
                    IkiliOp::Kucuk => K::I64LtS,
                    IkiliOp::Buyuk => K::I64GtS,
                    IkiliOp::KucukEsit => K::I64LeS,
                    _ => K::I64GeS,
                });
            }
            IkiliOp::Bol | IkiliOp::TamBol | IkiliOp::Mod => {
                let ad = if op == IkiliOp::Mod {
                    "ohc_mod"
                } else {
                    "ohc_bol"
                };
                self.cagri(ad, &[Arg::I(sol), Arg::I(sag), Arg::S(satir)])?;
            }
            IkiliOp::Esit | IkiliOp::EsitDegil if sol.tip.metin_gibi() => {
                self.cagri("ohc_metin_esit", &iki)?;
                if op == IkiliOp::EsitDegil {
                    self.sabit(1);
                    self.e(K::I64Xor);
                }
            }
            _ => {
                self.argumanlar(&iki)?;
                self.mantik(match op {
                    IkiliOp::Esit => K::I64Eq,
                    IkiliOp::EsitDegil => K::I64Ne,
                    IkiliOp::Kucuk => K::I64LtS,
                    IkiliOp::Buyuk => K::I64GtS,
                    IkiliOp::KucukEsit => K::I64LeS,
                    IkiliOp::BuyukEsit => K::I64GeS,
                    _ => unreachable!(),
                });
            }
        }
        Ok(())
    }

    fn cagri_ifadesi(&mut self, e: &Ifade, ad: &str, arg: &[Ifade]) -> Result<(), String> {
        let satir = e.konum.satir as i64;
        // Ön kütüphanenin yerleşik çağrısı: programın aynı adlı işlevi değil
        let (ad, kullanici) = match ad.strip_prefix(YERLESIK_ON_EK) {
            Some(y) => (y, None),
            None => (ad, self.o.islevler.get(ad).copied()),
        };
        if let Some((sira, doner)) = kullanici {
            let a: Vec<Arg> = arg.iter().map(Arg::I).collect();
            self.argumanlar(&a)?;
            self.e(K::Call(sira));
            if !doner {
                self.sabit(0);
            }
            return Ok(());
        }
        let t0 = arg.first().map(|a| a.tip.clone()).unwrap_or(Tip::Bos);
        match ad {
            "uzunluk" => {
                let f = match t0 {
                    Tip::Metin => "ohc_metin_uzunluk",
                    Tip::Sozluk(..) => "ohc_sozluk_uzunluk",
                    _ => "ohc_liste_uzunluk",
                };
                self.cagri(f, &[Arg::I(&arg[0])])?;
            }
            "metin" => self.arg_yukle(&Arg::Metne(&arg[0]))?,
            "sayı" if t0 == Tip::Sayi => self.ifade(&arg[0])?,
            "sayı" if t0 == Tip::Ondalik => {
                self.ifade(&arg[0])?;
                self.e(K::F64ReinterpretI64);
                self.e(K::I64TruncSatF64S);
            }
            "sayı" => self.cagri("ohc_metinden_sayi", &[Arg::I(&arg[0]), Arg::S(satir)])?,
            "ondalık" if t0 == Tip::Ondalik => self.ifade(&arg[0])?,
            "ondalık" if t0 == Tip::Sayi => {
                self.ifade(&arg[0])?;
                self.e(K::F64ConvertI64S);
                self.e(K::I64ReinterpretF64);
            }
            "ondalık" => self.cagri("ohc_metinden_ondalik", &[Arg::I(&arg[0]), Arg::S(satir)])?,
            "yuvarla" if arg.len() == 2 => {
                self.cagri("ohc_yuvarla_basamak", &[Arg::I(&arg[0]), Arg::I(&arg[1])])?
            }
            "yuvarla" if t0 == Tip::Sayi => self.ifade(&arg[0])?,
            "yuvarla" => self.cagri("ohc_yuvarla", &[Arg::I(&arg[0])])?,
            "oku" => self.cagri("ohc_oku", &[])?,
            SECENEK_CEVIR => self.cagri(
                "ohc_secenek_cevir",
                &[
                    Arg::I(&arg[0]),
                    Arg::I(&arg[1]),
                    Arg::I(&arg[2]),
                    Arg::S(satir),
                ],
            )?,
            _ => self.yerlesik(e, ad, arg, &t0)?,
        }
        Ok(())
    }

    /// Standart kütüphane işlevleri: çoğu doğrudan çalışma zamanına çağrıdır.
    fn yerlesik(&mut self, e: &Ifade, ad: &str, arg: &[Ifade], t0: &Tip) -> Result<(), String> {
        let satir = Arg::S(e.konum.satir as i64);
        let kod0 = Arg::S(t0.ic_kod());
        let metin = *t0 == Tip::Metin;
        let sozluk = matches!(t0, Tip::Sozluk(..));
        let mut d: Vec<Arg> = arg.iter().map(Arg::I).collect();
        match ad {
            "parça" if metin => self.cagri("ohc_metin_parca", &d)?,
            "parça" => self.cagri("ohc_liste_parca", &d)?,
            "birleştir" => self.cagri("ohc_birlestir", &d)?,
            "içerir" if sozluk => {
                d.push(kod0);
                self.cagri("ohc_sozluk_icerir", &d)?
            }
            "içerir" => {
                d.push(kod0);
                self.cagri("ohc_liste_bul", &d)?;
                self.sabit(0);
                self.mantik(K::I64GeS);
            }
            "bul" => {
                d.push(kod0);
                self.cagri("ohc_liste_bul", &d)?
            }
            "harfler" => self.cagri("ohc_harfler", &d)?,
            "kod" => self.cagri("ohc_kod", &d)?,
            "kodlar" => self.cagri("ohc_kodlar", &d)?,
            "kodlardan" => {
                d.push(satir);
                self.cagri("ohc_kodlardan", &d)?
            }
            "karakter" => {
                d.push(satir);
                self.cagri("ohc_karakter", &d)?
            }
            "sayı_mı" => self.cagri("ohc_sayi_mi", &d)?,
            "ondalık_mı" => self.cagri("ohc_ondalik_mi", &d)?,
            "sil" if sozluk => {
                d.push(kod0);
                self.cagri("ohc_sozluk_sil", &d)?
            }
            "sil" => {
                d.push(satir);
                self.cagri("ohc_liste_sil", &d)?
            }
            "ters" => self.cagri("ohc_liste_ters", &d)?,
            "karıştır" => self.cagri("ohc_karistir", &d)?,
            "kopya" => self.cagri("ohc_liste_kopya", &d)?,
            "en_büyük" | "en_küçük" => {
                let yon = Arg::S(if ad == "en_büyük" { 1 } else { -1 });
                if d.len() == 1 {
                    d.extend([kod0, yon, satir]);
                    self.cagri("ohc_liste_en", &d)?
                } else {
                    d.extend([Arg::S(t0.kod()), yon]);
                    self.cagri("ohc_en_iki", &d)?
                }
            }
            "toplam" => {
                d.extend([kod0, satir]);
                self.cagri("ohc_liste_toplam", &d)?
            }
            "anahtarlar" => self.cagri("ohc_sozluk_anahtarlar", &d)?,
            "değerler" => self.cagri("ohc_sozluk_degerler", &d)?,
            "dosya_oku" => {
                d.push(satir);
                self.cagri("ohc_dosya_oku", &d)?
            }
            "dosyaya_yaz" | "dosyaya_ekle" => {
                let kip = Arg::S((ad == "dosyaya_ekle") as i64);
                d.extend([kip, satir]);
                self.cagri("ohc_dosyaya_yaz", &d)?
            }
            "dosya_var" => self.cagri("ohc_dosya_var", &d)?,
            "dosya_sil" => self.cagri("ohc_dosya_sil", &d)?,
            "dosya_taşı" => self.cagri("ohc_dosya_tasi", &d)?,
            "karekök" | "sinüs" | "kosinüs" | "tanjant" | "logaritma" if d.len() == 1 => {
                let islem = ["karekök", "sinüs", "kosinüs", "tanjant", "logaritma"]
                    .iter()
                    .position(|a| *a == ad)
                    .unwrap() as i64;
                d.insert(0, Arg::S(islem));
                d.push(satir);
                self.cagri("ohc_matematik", &d)?
            }
            "logaritma" => {
                d.push(satir);
                self.cagri("ohc_logaritma_taban", &d)?
            }
            "üs" if e.tip == Tip::Sayi => {
                d.push(satir);
                self.cagri("ohc_us_tam", &d)?
            }
            "üs" => self.cagri("ohc_us", &d)?,
            "mutlak" if *t0 == Tip::Ondalik => {
                self.ifade(&arg[0])?;
                self.e(K::F64ReinterpretI64);
                self.e(K::F64Abs);
                self.e(K::I64ReinterpretF64);
            }
            "mutlak" => {
                d.push(satir);
                self.cagri("ohc_mutlak", &d)?
            }
            "rastgele" if d.is_empty() => self.cagri("ohc_rastgele", &d)?,
            "rastgele" => {
                d.push(satir);
                self.cagri("ohc_rastgele_aralik", &d)?
            }
            "zaman" => self.cagri("ohc_zaman", &d)?,
            "tarih" => self.cagri("ohc_tarih", &d)?,
            "bekle" => self.cagri("ohc_bekle", &d)?,
            "argümanlar" => self.cagri("ohc_argumanlar", &d)?,
            "json" => {
                d.push(Arg::S(t0.kod()));
                self.cagri("ohc_json", &d)?
            }
            "para" => self.cagri("ohc_para", &d)?,
            "sun" => {
                return Err(
                    "sun() WebAssembly hedefinde kullanılamaz: tarayıcıda web sunucusu çalışmaz"
                        .into(),
                )
            }
            "ortam" => self.cagri("ohc_ortam", &d)?,
            "http_al" | "http_gönder" => {
                let yontem = (ad == "http_gönder") as i64;
                if yontem == 0 {
                    d.push(Arg::S(0));
                }
                d.insert(0, Arg::S(yontem));
                d.push(satir);
                self.cagri("ohc_http", &d)?
            }
            "çık" => self.cagri("ohc_cik", &d)?,
            "boş_mu" => {
                self.ifade(&arg[0])?;
                self.mantik(K::I64Eqz);
            }
            HATA_SATIRDA => self.cagri("ohc_hata_ver", &d)?,
            "hata_ver" => {
                d.push(satir);
                self.cagri("ohc_hata_ver", &d)?
            }
            _ => return Err(format!("bilinmeyen işlev '{ad}'")),
        }
        Ok(())
    }

    /// Model nesnesi: [tanım, bağlama hataları, kimlik, alanlar...]
    fn nesne_kur(&mut self, model: &str, degerler: &[&Ifade]) -> Result<(), String> {
        let tanim = self.o.tanim(model)?;
        self.cz("ohc_liste_yeni");
        let korunmali = degerler.iter().any(|d| self.tetikler(d));
        let n = self.sakla(korunmali);
        self.yap("ohc_liste_ekle", &[Arg::Hazir(n), Arg::Metin(tanim)])?;
        self.yap("ohc_liste_ekle", &[Arg::Hazir(n), Arg::S(0)])?;
        for d in degerler {
            self.yap("ohc_liste_ekle", &[Arg::Hazir(n), Arg::I(d)])?;
        }
        self.yukle(n);
        self.birak(n);
        Ok(())
    }

    /// Alanları varsayılan değerlerinde yeni bir nesne.
    fn varsayilan_nesne(&mut self, model: &str) -> Result<(), String> {
        let degerler: Vec<Ifade> = self
            .o
            .modeller
            .get(model)
            .ok_or_else(|| format!("tanımsız model '{model}'"))?
            .alanlar
            .iter()
            .map(|a| a.ilk_deger())
            .collect();
        let r: Vec<&Ifade> = degerler.iter().collect();
        self.nesne_kur(model, &r)
    }

    fn metod(&mut self, e: &Ifade, alici: &Ifade, ad: &str, arg: &[Ifade]) -> Result<(), String> {
        let satir = Arg::S(e.konum.satir as i64);
        let Tip::Model(model) = &alici.tip else {
            return Err(format!("'{ad}' yöntemi model olmayan bir değerde"));
        };
        let model = model.clone();
        if let IfadeTuru::ModelAdi(_) = alici.tur {
            let t = self.o.tanim(&model)?;
            match ad {
                "hepsi" => self.cagri("ohc_model_hepsi", &[Arg::Metin(t), satir])?,
                "var_mı" => {
                    self.cagri("ohc_model_var", &[Arg::Metin(t), Arg::I(&arg[0]), satir])?
                }
                "sil" => self.cagri("ohc_model_sil", &[Arg::Metin(t), Arg::I(&arg[0]), satir])?,
                "bul" => {
                    // Önce kimlik, sonra varsayılan nesne (güvenli noktasız).
                    self.ifade(&arg[0])?;
                    let k = self.sakla(false);
                    self.varsayilan_nesne(&model)?;
                    self.yukle(k);
                    self.birak(k);
                    self.arg_yukle(&satir)?;
                    self.cz("ohc_model_yukle");
                }
                "formdan" => {
                    self.ifade(&arg[0])?;
                    let istek = self.sakla(false);
                    self.varsayilan_nesne(&model)?;
                    let n = self.sakla(false);
                    self.yap("ohc_model_doldur", &[Arg::Hazir(n), Arg::Hazir(istek)])?;
                    self.yukle(n);
                    self.birak(n);
                    self.birak(istek);
                }
                _ => return Err(format!("bilinmeyen yöntem '{ad}'")),
            }
            return Ok(());
        }
        match ad {
            "kaydet" => self.cagri("ohc_model_kaydet", &[Arg::I(alici), satir])?,
            "sil" => {
                let t = self.o.tanim(&model)?;
                self.metin_sabiti(&t);
                self.cagri(
                    "ohc_alan_al",
                    &[
                        Arg::I(alici),
                        Arg::S(ILK_ALAN),
                        Arg::S(e.konum.satir as i64),
                    ],
                )?;
                self.arg_yukle(&satir)?;
                self.cz("ohc_model_sil");
            }
            "geçerli_mi" => self.cagri("ohc_model_gecerli", &[Arg::I(alici)])?,
            "hatalar" => self.cagri("ohc_model_hatalar", &[Arg::I(alici)])?,
            "json" => self.cagri("ohc_json", &[Arg::I(alici), Arg::S(6)])?,
            _ => return Err(format!("bilinmeyen yöntem '{ad}'")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use std::collections::HashMap;
    use std::path::Path;
    use wasmparser::{CompositeInnerType, ExternalKind, Parser, Payload, TypeRef, Validator};

    /// İşlev tipleri (parametre ve sonuç sayısı), içe ve dışa aktarılanlar.
    struct Ozet {
        ice: Vec<(String, String, (usize, usize))>,
        disa: HashMap<String, (usize, usize)>,
    }

    fn ozet(wasm: &[u8]) -> Ozet {
        let mut turler = Vec::new();
        let mut islev_turu = Vec::new();
        let mut ice = Vec::new();
        let mut disa_sira = Vec::new();
        for parca in Parser::new(0).parse_all(wasm) {
            match parca.unwrap() {
                Payload::TypeSection(r) => {
                    for grup in r {
                        for t in grup.unwrap().types() {
                            if let CompositeInnerType::Func(f) = &t.composite_type.inner {
                                turler.push((f.params().len(), f.results().len()));
                            }
                        }
                    }
                }
                Payload::ImportSection(r) => {
                    for i in r.into_imports() {
                        let i = i.unwrap();
                        if let TypeRef::Func(t) = i.ty {
                            islev_turu.push(t);
                            ice.push((i.module.to_string(), i.name.to_string(), t));
                        }
                    }
                }
                Payload::FunctionSection(r) => {
                    for t in r {
                        islev_turu.push(t.unwrap());
                    }
                }
                Payload::ExportSection(r) => {
                    for d in r {
                        let d = d.unwrap();
                        if d.kind == ExternalKind::Func {
                            disa_sira.push((d.name.to_string(), d.index));
                        }
                    }
                }
                _ => {}
            }
        }
        Ozet {
            ice: ice
                .into_iter()
                .map(|(m, a, t)| (m, a, turler[t as usize]))
                .collect(),
            disa: disa_sira
                .into_iter()
                .map(|(a, i)| (a, turler[islev_turu[i as usize] as usize]))
                .collect(),
        }
    }

    fn ornek_modulleri() -> Vec<(String, Vec<u8>)> {
        let kok = Path::new(env!("CARGO_MANIFEST_DIR")).join("örnekler");
        let mut sonuc = Vec::new();
        for klasor in [kok.clone(), kok.join("arayüz")] {
            for g in std::fs::read_dir(klasor).unwrap() {
                let yol = g.unwrap().path();
                if yol.extension().is_some_and(|u| u == "ohc") {
                    let p = crate::derleme::yukle(&yol).unwrap();
                    let wasm = super::uret(&p).unwrap();
                    sonuc.push((yol.display().to_string(), wasm));
                }
            }
        }
        sonuc
    }

    #[test]
    fn uretilen_moduller_gecerli() {
        let moduller = ornek_modulleri();
        assert!(moduller.len() >= 13);
        for (ad, wasm) in moduller {
            Validator::new()
                .validate_all(&wasm)
                .unwrap_or_else(|h| panic!("{ad}: geçersiz wasm: {h}"));
        }
    }

    #[test]
    fn calisma_zamani_tum_iceri_aktarilanlari_saglar() {
        let rt = ozet(crate::derleme::WASM_CALISMA_ZAMANI);
        for (ad, wasm) in ornek_modulleri() {
            for (modul, isim, tur) in ozet(&wasm).ice {
                if modul == "ui" || modul == "dn" {
                    // Arayüz ve `dene:` işlevleri JavaScript'ten gelir (orhunca.js).
                    continue;
                }
                assert_eq!(modul, "rt", "{ad}");
                assert_eq!(
                    rt.disa.get(&isim),
                    Some(&tur),
                    "{ad}: çalışma zamanı '{isim}' işlevini bu tiple dışa aktarmıyor \
                     (araclar/wasm_calisma_zamani.sh ile yeniden derleyin)"
                );
            }
        }
    }

    #[test]
    fn calisma_zamani_guncel() {
        // runtime/orhunca_rt.c'deki her ohc_* işlevi (web sunucusu hariç)
        // gömülü wasm çalışma zamanında olmalı.
        let kaynak = include_str!("../runtime/orhunca_rt.c");
        let rt = ozet(crate::derleme::WASM_CALISMA_ZAMANI);
        let mut eksik = Vec::new();
        for satir in kaynak.lines() {
            let Some(geri) = satir
                .strip_prefix("int64_t ")
                .or_else(|| satir.strip_prefix("void "))
            else {
                continue;
            };
            let Some(ad) = geri.split('(').next().filter(|a| a.starts_with("ohc_")) else {
                continue;
            };
            if !super::YALNIZ_YEREL.contains(&ad) && !rt.disa.contains_key(ad) {
                eksik.push(ad.to_string());
            }
        }
        assert!(
            eksik.is_empty(),
            "wasm çalışma zamanı eski, eksik: {eksik:?} (araclar/wasm_calisma_zamani.sh)"
        );
    }
}
