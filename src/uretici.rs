//! Kod üretici: söz dizimi ağacını Cranelift ara gösterimine, oradan da makine
//! kodu içeren bir nesne dosyasına (.o / .obj) çevirir.
//!
//! Tüm değerler 64 bitlik tamsayıdır: sayı, mantık (0/1), metin (işaretçi) ve
//! liste (işaretçi). Metin ve liste işlemleri çalışma zamanı kütüphanesine çağrıdır.

use crate::agac::*;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    types, AbiParam, Function, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind,
    UserFuncName, Value,
};
use cranelift_codegen::isa::OwnedTargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;
use target_lexicon::Triple;

const I64: types::Type = types::I64;

/// Çalışma zamanı işlevleri: ad, parametre sayısı, değer döndürür mü.
pub(crate) const CALISMA_ZAMANI: &[(&str, usize, bool)] = &[
    ("ohc_yaz", 2, false),
    ("ohc_bol", 3, true),
    ("ohc_mod", 3, true),
    ("ohc_ondalik_bol", 3, true),
    ("ohc_yuvarla", 1, true),
    ("ohc_yuvarla_basamak", 2, true),
    ("ohc_metinden_ondalik", 2, true),
    ("ohc_metin_birlestir", 2, true),
    ("ohc_metne_cevir", 2, true),
    ("ohc_metin_esit", 2, true),
    ("ohc_metin_uzunluk", 1, true),
    ("ohc_metinden_sayi", 2, true),
    ("ohc_oku", 0, true),
    ("ohc_liste_yeni", 0, true),
    ("ohc_liste_ekle", 2, false),
    ("ohc_liste_uzunluk", 1, true),
    ("ohc_liste_al", 3, true),
    ("ohc_liste_koy", 4, false),
    ("ohc_liste_sirala", 2, false),
    ("ohc_tasma", 1, false),
    ("ohc_metin_kars", 2, true),
    ("ohc_metin_harf", 3, true),
    ("ohc_sozluk_yeni", 0, true),
    ("ohc_sozluk_koy", 4, false),
    ("ohc_sozluk_al", 4, true),
    ("ohc_sozluk_icerir", 3, true),
    ("ohc_sozluk_sil", 3, false),
    ("ohc_sozluk_anahtarlar", 1, true),
    ("ohc_sozluk_degerler", 1, true),
    ("ohc_sozluk_uzunluk", 1, true),
    ("ohc_harfler", 1, true),
    ("ohc_kod", 1, true),
    ("ohc_karakter", 2, true),
    ("ohc_liste_cikar", 3, true),
    ("ohc_dosyaya_yaz", 4, false),
    ("ohc_dosya_oku", 2, true),
    ("ohc_dosya_var", 1, true),
    ("ohc_dosya_sil", 1, true),
    ("ohc_buyuk_harf", 1, true),
    ("ohc_kucuk_harf", 1, true),
    ("ohc_kirp", 1, true),
    ("ohc_metin_parca", 3, true),
    ("ohc_liste_parca", 3, true),
    ("ohc_metin_bol", 2, true),
    ("ohc_birlestir", 2, true),
    ("ohc_metin_icerir", 2, true),
    ("ohc_metin_bul", 2, true),
    ("ohc_liste_bul", 3, true),
    ("ohc_degistir", 3, true),
    ("ohc_baslar", 2, true),
    ("ohc_biter", 2, true),
    ("ohc_tekrarla", 3, true),
    ("ohc_satirlar", 1, true),
    ("ohc_sayi_mi", 1, true),
    ("ohc_ondalik_mi", 1, true),
    ("ohc_liste_sil", 3, true),
    ("ohc_liste_ters", 1, true),
    ("ohc_metin_ters", 1, true),
    ("ohc_karistir", 1, false),
    ("ohc_liste_kopya", 1, true),
    ("ohc_liste_en", 4, true),
    ("ohc_en_iki", 4, true),
    ("ohc_liste_toplam", 3, true),
    ("ohc_matematik", 3, true),
    ("ohc_logaritma_taban", 3, true),
    ("ohc_us_tam", 3, true),
    ("ohc_us", 2, true),
    ("ohc_mutlak", 2, true),
    ("ohc_rastgele", 0, true),
    ("ohc_rastgele_aralik", 3, true),
    ("ohc_zaman", 0, true),
    ("ohc_tarih", 0, true),
    ("ohc_bekle", 1, false),
    ("ohc_argumanlar", 0, true),
    ("ohc_ortam", 1, true),
    ("ohc_cik", 1, false),
    ("ohc_alan_al", 3, true),
    ("ohc_alan_koy", 4, false),
    ("ohc_json", 2, true),
    ("ohc_kacir", 1, true),
    ("ohc_para", 1, true),
    ("ohc_url_kodla", 1, true),
    ("ohc_model_hepsi", 2, true),
    ("ohc_model_yukle", 3, true),
    ("ohc_model_var", 3, true),
    ("ohc_model_sil", 3, true),
    ("ohc_model_kaydet", 2, true),
    ("ohc_model_gecerli", 1, true),
    ("ohc_model_hatalar", 1, true),
    ("ohc_model_doldur", 2, false),
    ("ohc_web_yol", 3, false),
    ("ohc_sun", 2, false),
    ("ohc_dene", 2, true),
    ("ohc_hata_mesaji", 0, true),
    ("ohc_hata_ver", 2, false),
    ("ohc_secenek_cevir", 4, true),
];

/// `dene:` bloğunun işlevinin dönüş kodları (çalışma hatasında -1).
pub(crate) const DENE_SONA_ERDI: i64 = 0;
pub(crate) const DENE_DONDUR: i64 = 1;
pub(crate) const DENE_DUR: i64 = 2;
pub(crate) const DENE_SURDUR: i64 = 3;

/// Model nesnesinde ilk alanın (kimlik) yuvası: 0 tanım, 1 bağlama hataları.
pub(crate) const ILK_ALAN: i64 = 2;

pub fn isa_kur(triple: Triple) -> Result<OwnedTargetIsa, String> {
    let mut ayarlar = settings::builder();
    ayarlar
        .set("opt_level", "speed")
        .map_err(|e| e.to_string())?;
    ayarlar.set("is_pic", "true").map_err(|e| e.to_string())?;
    let isa = cranelift_codegen::isa::lookup(triple.clone())
        .map_err(|e| format!("'{triple}' hedefi desteklenmiyor: {e}"))?;
    isa.finish(settings::Flags::new(ayarlar))
        .map_err(|e| e.to_string())
}

/// Sembol adlarında yalnızca ASCII kullanılır: `çarp` → `ohc_k_u00e7arp`.
fn sembol(ad: &str) -> String {
    let mut s = String::from("ohc_k_");
    for c in ad.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if c == '_' {
            s.push_str("__");
        } else {
            s.push_str(&format!("_u{:04x}", c as u32));
        }
    }
    s
}

struct Ortak {
    module: ObjectModule,
    calisma: HashMap<&'static str, FuncId>,
    islevler: HashMap<String, (FuncId, bool)>,
    metinler: HashMap<String, DataId>,
    modeller: HashMap<String, Model>,
    /// Üretilmeyi bekleyen `dene:` bloklarının işlevleri
    govdeler: Vec<DeneGovdesi>,
    govde_sayisi: usize,
}

/// `dene:` bloğu ayrı bir işleve çevrilir: `(çerçeve) -> dönüş kodu`. Çerçeve,
/// çevreleyen işlevin yığıtındaki yuvalardır: 0. yuva `döndür` değeri, sonrakiler
/// bloğun kullandığı değişkenler (`yakalananlar` sırasıyla). Blok bu değişkenleri
/// doğrudan çerçevede okur ve yazar; böylece hata olsa da yapılan atamalar kalır.
struct DeneGovdesi {
    id: FuncId,
    govde: Vec<Deyim>,
    yakalananlar: Vec<String>,
    /// Asıl (en dıştaki) işlevin dönüşü: `None` ana program, `Some(true)` değer döndürür.
    dis_donus: Option<bool>,
}

impl Ortak {
    fn metin_verisi(&mut self, m: &str) -> Result<DataId, String> {
        if let Some(id) = self.metinler.get(m) {
            return Ok(*id);
        }
        let ad = format!("ohc_metin_{}", self.metinler.len());
        let id = self
            .module
            .declare_data(&ad, Linkage::Local, false, false)
            .map_err(|e| e.to_string())?;
        let mut d = DataDescription::new();
        let mut baytlar = m.as_bytes().to_vec();
        baytlar.push(0);
        d.define(baytlar.into_boxed_slice());
        self.module.define_data(id, &d).map_err(|e| e.to_string())?;
        self.metinler.insert(m.to_string(), id);
        Ok(id)
    }
}

pub fn uret(p: &Program, isa: OwnedTargetIsa) -> Result<Vec<u8>, String> {
    let builder = ObjectBuilder::new(isa, "orhunca", cranelift_module::default_libcall_names())
        .map_err(|e| e.to_string())?;
    let mut ortak = Ortak {
        module: ObjectModule::new(builder),
        calisma: HashMap::new(),
        islevler: HashMap::new(),
        metinler: HashMap::new(),
        govdeler: Vec::new(),
        govde_sayisi: 0,
        modeller: p
            .modeller
            .iter()
            .map(|m| (m.ad.clone(), m.clone()))
            .collect(),
    };

    for (ad, n, doner) in CALISMA_ZAMANI {
        let mut sig = ortak.module.make_signature();
        sig.params
            .extend(std::iter::repeat_n(AbiParam::new(I64), *n));
        if *doner {
            sig.returns.push(AbiParam::new(I64));
        }
        let id = ortak
            .module
            .declare_function(ad, Linkage::Import, &sig)
            .map_err(|e| e.to_string())?;
        ortak.calisma.insert(ad, id);
    }

    let mut imzalar = Vec::new();
    for f in &p.islevler {
        let doner = f.donus.as_ref().is_some_and(|t| *t != Tip::Bos);
        let mut sig = ortak.module.make_signature();
        sig.params
            .extend(f.parametreler.iter().map(|_| AbiParam::new(I64)));
        if doner {
            sig.returns.push(AbiParam::new(I64));
        }
        let id = ortak
            .module
            .declare_function(&sembol(&f.ad), Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;
        ortak.islevler.insert(f.ad.clone(), (id, doner));
        imzalar.push(sig);
    }

    let mut ctx = ortak.module.make_context();
    let mut fctx = FunctionBuilderContext::new();
    let rotalar: Vec<(String, String, FuncId)> = p
        .islevler
        .iter()
        .filter_map(|f| {
            let r = f.rota.as_ref()?;
            Some((r.yontem.clone(), r.kalip.clone(), ortak.islevler[&f.ad].0))
        })
        .collect();

    for (f, sig) in p.islevler.iter().zip(imzalar) {
        let (id, doner) = ortak.islevler[&f.ad];
        ctx.func = Function::with_name_signature(UserFuncName::user(0, id.as_u32()), sig);
        islev_uret(
            &mut ortak,
            &mut ctx,
            &mut fctx,
            &f.yereller,
            &f.parametreler,
            &f.govde,
            Some(doner),
            &[],
        )?;
        ortak
            .module
            .define_function(id, &mut ctx)
            .map_err(|e| format!("'{}': {e:?}", f.ad))?;
        ortak.module.clear_context(&mut ctx);
        govdeleri_uret(&mut ortak, &mut ctx, &mut fctx)?;
    }

    // main: C çalışma zamanının giriş noktası.
    let mut sig = ortak.module.make_signature();
    sig.returns.push(AbiParam::new(types::I32));
    let id = ortak
        .module
        .declare_function("ohc_ana", Linkage::Export, &sig)
        .map_err(|e| e.to_string())?;
    ctx.func = Function::with_name_signature(UserFuncName::user(0, id.as_u32()), sig);
    islev_uret(
        &mut ortak,
        &mut ctx,
        &mut fctx,
        &p.ana_yereller,
        &[],
        &p.ana,
        None,
        &rotalar,
    )?;
    ortak
        .module
        .define_function(id, &mut ctx)
        .map_err(|e| format!("ana program: {e:?}"))?;
    ortak.module.clear_context(&mut ctx);
    govdeleri_uret(&mut ortak, &mut ctx, &mut fctx)?;

    let urun = ortak.module.finish();
    urun.emit().map_err(|e| e.to_string())
}

/// Bekleyen `dene:` bloklarının işlevlerini üretir (iç içe bloklar sıraya eklenir).
fn govdeleri_uret(
    ortak: &mut Ortak,
    ctx: &mut Context,
    fctx: &mut FunctionBuilderContext,
) -> Result<(), String> {
    while let Some(g) = ortak.govdeler.pop() {
        let mut sig = ortak.module.make_signature();
        sig.params.push(AbiParam::new(I64));
        sig.returns.push(AbiParam::new(I64));
        ctx.func = Function::with_name_signature(UserFuncName::user(0, g.id.as_u32()), sig);
        let mut b = FunctionBuilder::new(&mut ctx.func, fctx);
        let giris = b.create_block();
        b.append_block_params_for_function_params(giris);
        b.switch_to_block(giris);
        let cerceve = b.block_params(giris)[0];
        let mut u = Uretici {
            b,
            ortak,
            degiskenler: HashMap::new(),
            dis: g
                .yakalananlar
                .iter()
                .enumerate()
                .map(|(i, a)| (a.clone(), 8 * (i as i32 + 1)))
                .collect(),
            govde: Some((cerceve, g.dis_donus)),
            donguler: Vec::new(),
            cagri_onbellek: HashMap::new(),
            donus: Some(true),
        };
        u.blok(&g.govde)?;
        let s = u.sabit(DENE_SONA_ERDI);
        u.b.ins().return_(&[s]);
        u.b.seal_all_blocks();
        let hedef = u.ortak.module.target_config();
        u.b.finalize(hedef);
        ortak
            .module
            .define_function(g.id, ctx)
            .map_err(|e| format!("dene bloğu: {e:?}"))?;
        ortak.module.clear_context(ctx);
    }
    Ok(())
}

/// `donus`: `None` ana program (main), `Some(true)` değer döndüren işlev.
/// `rotalar`: ana programın başında çalışma zamanına kaydedilecek web yolları.
#[allow(clippy::too_many_arguments)]
fn islev_uret(
    ortak: &mut Ortak,
    ctx: &mut Context,
    fctx: &mut FunctionBuilderContext,
    yereller: &[(String, Tip)],
    parametreler: &[(String, Tip)],
    govde: &[Deyim],
    donus: Option<bool>,
    rotalar: &[(String, String, FuncId)],
) -> Result<(), String> {
    let mut b = FunctionBuilder::new(&mut ctx.func, fctx);
    let giris = b.create_block();
    b.append_block_params_for_function_params(giris);
    b.switch_to_block(giris);

    let mut degiskenler = HashMap::new();
    let sifir = b.ins().iconst(I64, 0);
    for (ad, _) in yereller {
        let v = b.declare_var(I64);
        b.def_var(v, sifir);
        degiskenler.insert(ad.clone(), v);
    }
    let parametre_degerleri = b.block_params(giris).to_vec();
    for ((ad, _), deger) in parametreler.iter().zip(parametre_degerleri) {
        b.def_var(degiskenler[ad], deger);
    }

    let mut u = Uretici {
        b,
        ortak,
        degiskenler,
        dis: HashMap::new(),
        govde: None,
        donguler: Vec::new(),
        cagri_onbellek: HashMap::new(),
        donus,
    };
    for (yontem, kalip, id) in rotalar {
        let y = u.metin_sabiti(yontem)?;
        let k = u.metin_sabiti(kalip)?;
        let fref = u.ortak.module.declare_func_in_func(*id, u.b.func);
        let f = u.b.ins().func_addr(I64, fref);
        u.cz("ohc_web_yol", &[y, k, f]);
    }
    for d in govde {
        u.deyim(d)?;
    }
    // Gövdenin sonuna düşülürse
    match donus {
        None => {
            let s = u.b.ins().iconst(types::I32, 0);
            u.b.ins().return_(&[s]);
        }
        Some(true) => {
            let s = u.b.ins().iconst(I64, 0);
            u.b.ins().return_(&[s]);
        }
        Some(false) => {
            u.b.ins().return_(&[]);
        }
    }
    u.b.seal_all_blocks();
    let hedef = u.ortak.module.target_config();
    u.b.finalize(hedef);
    Ok(())
}

struct Uretici<'a, 'b> {
    b: FunctionBuilder<'b>,
    ortak: &'a mut Ortak,
    degiskenler: HashMap<String, Variable>,
    /// `dene:` bloğunun işlevinde: çerçevedeki değişkenler (bayt ofseti)
    dis: HashMap<String, i32>,
    /// `dene:` bloğunun işlevinde: (çerçevenin adresi, asıl işlevin dönüşü)
    govde: Option<(Value, Option<bool>)>,
    /// (sürdür hedefi, dur hedefi)
    donguler: Vec<(cranelift_codegen::ir::Block, cranelift_codegen::ir::Block)>,
    cagri_onbellek: HashMap<FuncId, cranelift_codegen::ir::FuncRef>,
    donus: Option<bool>,
}

impl Uretici<'_, '_> {
    fn cagir(&mut self, id: FuncId, arg: &[Value]) -> Option<Value> {
        let fref = *self
            .cagri_onbellek
            .entry(id)
            .or_insert_with(|| self.ortak.module.declare_func_in_func(id, self.b.func));
        let inst = self.b.ins().call(fref, arg);
        self.b.inst_results(inst).first().copied()
    }

    /// Çalışma zamanı çağrısı.
    fn cz(&mut self, ad: &str, arg: &[Value]) -> Option<Value> {
        let id = self.ortak.calisma[ad];
        self.cagir(id, arg)
    }

    fn sabit(&mut self, n: i64) -> Value {
        self.b.ins().iconst(I64, n)
    }

    /// Dönüş/dur/sürdür sonrası ulaşılamayan kod için yeni bir blok açar.
    fn olu_blok(&mut self) {
        let blok = self.b.create_block();
        self.b.switch_to_block(blok);
    }

    fn blok(&mut self, govde: &[Deyim]) -> Result<(), String> {
        for d in govde {
            self.deyim(d)?;
        }
        Ok(())
    }

    fn deyim(&mut self, d: &Deyim) -> Result<(), String> {
        match d {
            Deyim::Atama { hedef, deger, .. } => {
                let v = self.ifade(deger)?;
                self.yaz(hedef, v);
            }
            Deyim::IndeksAtama {
                liste,
                indeks,
                deger,
            } => {
                let l = self.ifade(liste)?;
                let i = self.ifade(indeks)?;
                let v = self.ifade(deger)?;
                if let Tip::Sozluk(..) = liste.tip {
                    let kod = self.sabit(liste.tip.ic_kod());
                    self.cz("ohc_sozluk_koy", &[l, i, v, kod]);
                } else {
                    let s = self.sabit(indeks.konum.satir as i64);
                    self.cz("ohc_liste_koy", &[l, i, v, s]);
                }
            }
            Deyim::AlanAtama {
                nesne,
                sira,
                deger,
                konum,
                ..
            } => {
                let n = self.ifade(nesne)?;
                let v = self.ifade(deger)?;
                let yuva = self.sabit(ILK_ALAN + *sira as i64);
                let s = self.sabit(konum.satir as i64);
                self.cz("ohc_alan_koy", &[n, yuva, v, s]);
            }
            Deyim::Cikar { oge, liste } => {
                let o = self.ifade(oge)?;
                let l = self.ifade(liste)?;
                let kod = self.sabit(liste.tip.ic_kod());
                self.cz("ohc_liste_cikar", &[l, o, kod]);
            }
            Deyim::DosyayaYaz { deger, yol } => {
                let v = self.ifade(deger)?;
                let m = self.metne(v, &deger.tip);
                let y = self.ifade(yol)?;
                let kip = self.sabit(0);
                let s = self.sabit(yol.konum.satir as i64);
                self.cz("ohc_dosyaya_yaz", &[y, m, kip, s]);
            }
            Deyim::Yaz(i) => {
                let v = self.ifade(i)?;
                let kod = self.sabit(i.tip.kod());
                self.cz("ohc_yaz", &[v, kod]);
            }
            Deyim::Ekle { oge, liste } => {
                let o = self.ifade(oge)?;
                let l = self.ifade(liste)?;
                self.cz("ohc_liste_ekle", &[l, o]);
            }
            Deyim::Sirala(l) => {
                let v = self.ifade(l)?;
                let kod = match &l.tip {
                    Tip::Liste(ic) => ic.kod(),
                    _ => 0,
                };
                let kod = self.sabit(kod);
                self.cz("ohc_liste_sirala", &[v, kod]);
            }
            Deyim::Eger {
                kosul,
                govde,
                degilse,
            } => {
                let k = self.ifade(kosul)?;
                let evet = self.b.create_block();
                let hayir = self.b.create_block();
                let son = self.b.create_block();
                self.b.ins().brif(k, evet, &[], hayir, &[]);
                self.b.switch_to_block(evet);
                self.blok(govde)?;
                self.b.ins().jump(son, &[]);
                self.b.switch_to_block(hayir);
                self.blok(degilse)?;
                self.b.ins().jump(son, &[]);
                self.b.switch_to_block(son);
            }
            Deyim::Surece { kosul, govde } => {
                let bas = self.b.create_block();
                let ic = self.b.create_block();
                let son = self.b.create_block();
                self.b.ins().jump(bas, &[]);
                self.b.switch_to_block(bas);
                let k = self.ifade(kosul)?;
                self.b.ins().brif(k, ic, &[], son, &[]);
                self.b.switch_to_block(ic);
                self.donguler.push((bas, son));
                self.blok(govde)?;
                self.donguler.pop();
                self.b.ins().jump(bas, &[]);
                self.b.switch_to_block(son);
            }
            Deyim::HerAralik {
                degisken,
                bas,
                son,
                govde,
                ..
            } => {
                let ilk = self.ifade(bas)?;
                let sinir = self.ifade(son)?;
                let sinir_v = self.b.declare_var(I64);
                self.b.def_var(sinir_v, sinir);
                // Ayrı bir sayaç: gövde döngü değişkenini değiştirse de döngü bozulmaz.
                let sayac = self.b.declare_var(I64);
                self.b.def_var(sayac, ilk);
                self.dongu(
                    sayac,
                    |u| {
                        let i = u.b.use_var(sayac);
                        let s = u.b.use_var(sinir_v);
                        Ok(u.b.ins().icmp(IntCC::SignedLessThanOrEqual, i, s))
                    },
                    |u| {
                        let i = u.b.use_var(sayac);
                        u.yaz(degisken, i);
                        u.blok(govde)
                    },
                )?;
            }
            Deyim::HerListe {
                degisken,
                liste,
                govde,
                ..
            } => {
                let mut l = self.ifade(liste)?;
                // Metinde harfler, sözlükte anahtarlar gezilir.
                match liste.tip {
                    Tip::Metin => l = self.cz("ohc_harfler", &[l]).unwrap(),
                    Tip::Sozluk(..) => l = self.cz("ohc_sozluk_anahtarlar", &[l]).unwrap(),
                    _ => {}
                }
                let lv = self.b.declare_var(I64);
                self.b.def_var(lv, l);
                let sayac = self.b.declare_var(I64);
                let sifir = self.sabit(0);
                self.b.def_var(sayac, sifir);
                let satir = liste.konum.satir as i64;
                self.dongu(
                    sayac,
                    |u| {
                        let i = u.b.use_var(sayac);
                        let l = u.b.use_var(lv);
                        let n = u.cz("ohc_liste_uzunluk", &[l]).unwrap();
                        Ok(u.b.ins().icmp(IntCC::SignedLessThan, i, n))
                    },
                    |u| {
                        let i = u.b.use_var(sayac);
                        let l = u.b.use_var(lv);
                        let s = u.sabit(satir);
                        let o = u.cz("ohc_liste_al", &[l, i, s]).unwrap();
                        u.yaz(degisken, o);
                        u.blok(govde)
                    },
                )?;
            }
            Deyim::Dondur(deger, _) if self.govde.is_some() => {
                let (cerceve, dis_donus) = self.govde.unwrap();
                if let Some(i) = deger {
                    let v = self.ifade(i)?;
                    if dis_donus == Some(true) {
                        self.b.ins().store(MemFlagsData::trusted(), v, cerceve, 0);
                    }
                }
                let k = self.sabit(DENE_DONDUR);
                self.b.ins().return_(&[k]);
                self.olu_blok();
            }
            Deyim::Dur(_) | Deyim::Surdur(_)
                if self.govde.is_some() && self.donguler.is_empty() =>
            {
                // Döngü bloğun dışında: çevreleyen işlev dallanır.
                let k = self.sabit(if matches!(d, Deyim::Dur(_)) {
                    DENE_DUR
                } else {
                    DENE_SURDUR
                });
                self.b.ins().return_(&[k]);
                self.olu_blok();
            }
            Deyim::Dene {
                govde,
                degisken,
                yakala,
                ..
            } => self.dene(govde, degisken.as_deref(), yakala)?,
            Deyim::Dondur(deger, _) => {
                match (deger, self.donus) {
                    (Some(i), Some(true)) => {
                        let v = self.ifade(i)?;
                        self.b.ins().return_(&[v]);
                    }
                    (Some(i), _) => {
                        // Değeri olmayan (boş) işlev çağrısı döndürülüyor.
                        self.ifade(i)?;
                        self.b.ins().return_(&[]);
                    }
                    (None, Some(true)) => {
                        let s = self.sabit(0);
                        self.b.ins().return_(&[s]);
                    }
                    (None, _) => {
                        self.b.ins().return_(&[]);
                    }
                }
                self.olu_blok();
            }
            Deyim::Dur(_) => {
                let (_, son) = *self.donguler.last().unwrap();
                self.b.ins().jump(son, &[]);
                self.olu_blok();
            }
            Deyim::Surdur(_) => {
                let (devam, _) = *self.donguler.last().unwrap();
                self.b.ins().jump(devam, &[]);
                self.olu_blok();
            }
            Deyim::IfadeDeyimi(i) => {
                self.ifade(i)?;
            }
            Deyim::Oge(_) => {
                return Err(
                    "arayüz öğeleri yalnızca WebAssembly hedefinde (--hedef web) derlenir".into(),
                )
            }
        }
        Ok(())
    }

    fn oku(&mut self, ad: &str) -> Value {
        if let Some(v) = self.degiskenler.get(ad) {
            return self.b.use_var(*v);
        }
        let (cerceve, _) = self.govde.expect("dene bloğu dışında çerçeve değişkeni");
        let ofset = self.dis[ad];
        self.b
            .ins()
            .load(I64, MemFlagsData::trusted(), cerceve, ofset)
    }

    fn yaz(&mut self, ad: &str, v: Value) {
        if let Some(d) = self.degiskenler.get(ad) {
            self.b.def_var(*d, v);
            return;
        }
        let (cerceve, _) = self.govde.expect("dene bloğu dışında çerçeve değişkeni");
        let ofset = self.dis[ad];
        self.b
            .ins()
            .store(MemFlagsData::trusted(), v, cerceve, ofset);
    }

    /// `dene:` bloğu: blok ayrı bir işleve çevrilir ve çalışma zamanının
    /// `ohc_dene`'si ile çağrılır. Bloğun kullandığı değişkenler önce bu işlevin
    /// yığıtındaki bir çerçeveye konur, sonra oradan geri okunur.
    fn dene(
        &mut self,
        govde: &[Deyim],
        degisken: Option<&str>,
        yakala: &[Deyim],
    ) -> Result<(), String> {
        let yakalananlar: Vec<String> = gecen_adlar(govde)
            .into_iter()
            .filter(|a| self.degiskenler.contains_key(a) || self.dis.contains_key(a))
            .collect();
        let dis_donus = match self.govde {
            Some((_, d)) => d,
            None => self.donus,
        };
        let mut sig = self.ortak.module.make_signature();
        sig.params.push(AbiParam::new(I64));
        sig.returns.push(AbiParam::new(I64));
        let ad = format!("ohc_dene_{}", self.ortak.govde_sayisi);
        self.ortak.govde_sayisi += 1;
        let id = self
            .ortak
            .module
            .declare_function(&ad, Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;
        self.ortak.govdeler.push(DeneGovdesi {
            id,
            govde: govde.to_vec(),
            yakalananlar: yakalananlar.clone(),
            dis_donus,
        });

        let boyut = 8 * (yakalananlar.len() as u32 + 1);
        let yuva = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            boyut,
            3,
        ));
        let cerceve = self.b.ins().stack_addr(I64, yuva, 0);
        let sifir = self.sabit(0);
        self.b
            .ins()
            .store(MemFlagsData::trusted(), sifir, cerceve, 0);
        for (i, a) in yakalananlar.iter().enumerate() {
            let v = self.oku(a);
            self.b
                .ins()
                .store(MemFlagsData::trusted(), v, cerceve, 8 * (i as i32 + 1));
        }
        let fref = self.ortak.module.declare_func_in_func(id, self.b.func);
        let f = self.b.ins().func_addr(I64, fref);
        let kod = self.cz("ohc_dene", &[f, cerceve]).unwrap();
        // Çağrıdan sonra çerçevenin adresi yeniden alınır (değer çağrı boyunca yaşamaz).
        let cerceve = self.b.ins().stack_addr(I64, yuva, 0);
        for (i, a) in yakalananlar.iter().enumerate() {
            let v = self
                .b
                .ins()
                .load(I64, MemFlagsData::trusted(), cerceve, 8 * (i as i32 + 1));
            self.yaz(a, v);
        }

        let hata_blok = self.b.create_block();
        let diger = self.b.create_block();
        let son = self.b.create_block();
        let hata_mi = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, kod, 0);
        self.b.ins().brif(hata_mi, hata_blok, &[], diger, &[]);

        self.b.switch_to_block(hata_blok);
        if let Some(d) = degisken {
            let m = self.cz("ohc_hata_mesaji", &[]).unwrap();
            self.yaz(d, m);
        }
        self.blok(yakala)?;
        self.b.ins().jump(son, &[]);

        // Blok `döndür`, `dur` ya da `sürdür` ile bitti mi?
        self.b.switch_to_block(diger);
        let dondur = self.b.create_block();
        let kontrol = self.b.create_block();
        let d1 = self.b.ins().icmp_imm_s(IntCC::Equal, kod, DENE_DONDUR);
        self.b.ins().brif(d1, dondur, &[], kontrol, &[]);
        self.b.switch_to_block(dondur);
        let cerceve = self.b.ins().stack_addr(I64, yuva, 0);
        let deger = self.b.ins().load(I64, MemFlagsData::trusted(), cerceve, 0);
        match (self.govde, self.donus) {
            (Some((dis_cerceve, _)), _) => {
                self.b
                    .ins()
                    .store(MemFlagsData::trusted(), deger, dis_cerceve, 0);
                let k = self.sabit(DENE_DONDUR);
                self.b.ins().return_(&[k]);
            }
            (None, None) => {
                let s = self.b.ins().iconst(types::I32, 0);
                self.b.ins().return_(&[s]);
            }
            (None, Some(true)) => {
                self.b.ins().return_(&[deger]);
            }
            (None, Some(false)) => {
                self.b.ins().return_(&[]);
            }
        }
        self.b.switch_to_block(kontrol);
        let (devam_hedefi, dur_hedefi) = match self.donguler.last() {
            Some((devam, dur)) => (*devam, *dur),
            None if self.govde.is_some() => {
                let dur = self.b.create_block();
                let devam = self.b.create_block();
                self.b.switch_to_block(dur);
                let k = self.sabit(DENE_DUR);
                self.b.ins().return_(&[k]);
                self.b.switch_to_block(devam);
                let k = self.sabit(DENE_SURDUR);
                self.b.ins().return_(&[k]);
                self.b.switch_to_block(kontrol);
                (devam, dur)
            }
            // Döngü yok: blok yalnızca sona ulaşmış olabilir.
            None => (son, son),
        };
        let dur_blok = self.b.create_block();
        let d2 = self.b.ins().icmp_imm_s(IntCC::Equal, kod, DENE_DUR);
        let sonraki = self.b.create_block();
        self.b.ins().brif(d2, dur_blok, &[], sonraki, &[]);
        self.b.switch_to_block(dur_blok);
        self.b.ins().jump(dur_hedefi, &[]);
        self.b.switch_to_block(sonraki);
        let d3 = self.b.ins().icmp_imm_s(IntCC::Equal, kod, DENE_SURDUR);
        self.b.ins().brif(d3, devam_hedefi, &[], son, &[]);

        self.b.switch_to_block(son);
        Ok(())
    }

    /// Sayaçlı döngü: koşul → gövde → sayaç += 1.
    fn dongu(
        &mut self,
        sayac: Variable,
        kosul: impl FnOnce(&mut Self) -> Result<Value, String>,
        govde: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        let bas = self.b.create_block();
        let ic = self.b.create_block();
        let artir = self.b.create_block();
        let son = self.b.create_block();
        self.b.ins().jump(bas, &[]);
        self.b.switch_to_block(bas);
        let k = kosul(self)?;
        self.b.ins().brif(k, ic, &[], son, &[]);
        self.b.switch_to_block(ic);
        self.donguler.push((artir, son));
        govde(self)?;
        self.donguler.pop();
        self.b.ins().jump(artir, &[]);
        self.b.switch_to_block(artir);
        let i = self.b.use_var(sayac);
        let i = self.b.ins().iadd_imm_s(i, 1);
        self.b.def_var(sayac, i);
        self.b.ins().jump(bas, &[]);
        self.b.switch_to_block(son);
        Ok(())
    }

    /// Ondalıklar 64 bitlik tamsayı yuvalarında bit deseni olarak taşınır.
    fn f64(&mut self, v: Value) -> Value {
        self.b.ins().bitcast(types::F64, MemFlagsData::new(), v)
    }

    fn bitler(&mut self, v: Value) -> Value {
        self.b.ins().bitcast(I64, MemFlagsData::new(), v)
    }

    fn fmantik(&mut self, cc: FloatCC, a: Value, b: Value) -> Value {
        let (a, b) = (self.f64(a), self.f64(b));
        let c = self.b.ins().fcmp(cc, a, b);
        self.b.ins().uextend(I64, c)
    }

    /// Ondalık işlem: iki tarafı f64'e çevirir, sonucu bit desenine geri çevirir.
    fn fislem(&mut self, a: Value, b: Value, f: fn(&mut Self, Value, Value) -> Value) -> Value {
        let (a, b) = (self.f64(a), self.f64(b));
        let r = f(self, a, b);
        self.bitler(r)
    }

    /// Tamsayı taşmasında çalışma hatası verir.
    fn tasma_denetle(&mut self, tasti: Value, satir: i64) {
        let hata = self.b.create_block();
        let devam = self.b.create_block();
        self.b.set_cold_block(hata);
        self.b.ins().brif(tasti, hata, &[], devam, &[]);
        self.b.switch_to_block(hata);
        let s = self.sabit(satir);
        self.cz("ohc_tasma", &[s]);
        self.b.ins().jump(devam, &[]);
        self.b.switch_to_block(devam);
    }

    /// Standart kütüphane işlevleri: çoğu doğrudan çalışma zamanına çağrıdır.
    fn yerlesik(
        &mut self,
        ad: &str,
        arg: &[Ifade],
        d: &[Value],
        e: &Ifade,
    ) -> Result<Option<Value>, String> {
        let satir = self.sabit(e.konum.satir as i64);
        let t0 = arg.first().map(|a| a.tip.clone()).unwrap_or(Tip::Bos);
        let kod0 = self.sabit(t0.ic_kod());
        let metin = t0 == Tip::Metin;
        let v = match ad {
            "büyük_harf" => self.cz("ohc_buyuk_harf", d),
            "küçük_harf" => self.cz("ohc_kucuk_harf", d),
            "kırp" => self.cz("ohc_kirp", d),
            "parça" if metin => self.cz("ohc_metin_parca", d),
            "parça" => self.cz("ohc_liste_parca", d),
            "böl" => self.cz("ohc_metin_bol", d),
            "birleştir" => self.cz("ohc_birlestir", d),
            "içerir" if metin => self.cz("ohc_metin_icerir", d),
            "içerir" if matches!(t0, Tip::Sozluk(..)) => {
                self.cz("ohc_sozluk_icerir", &[d[0], d[1], kod0])
            }
            "içerir" => {
                let i = self.cz("ohc_liste_bul", &[d[0], d[1], kod0]).unwrap();
                let sifir = self.sabit(0);
                Some(self.mantik(IntCC::SignedGreaterThanOrEqual, i, sifir))
            }
            "bul" if metin => self.cz("ohc_metin_bul", d),
            "bul" => self.cz("ohc_liste_bul", &[d[0], d[1], kod0]),
            "değiştir" => self.cz("ohc_degistir", d),
            "başlar" => self.cz("ohc_baslar", d),
            "biter" => self.cz("ohc_biter", d),
            "tekrarla" => self.cz("ohc_tekrarla", &[d[0], d[1], satir]),
            "harfler" => self.cz("ohc_harfler", d),
            "kod" => self.cz("ohc_kod", d),
            "karakter" => self.cz("ohc_karakter", &[d[0], satir]),
            "satırlar" => self.cz("ohc_satirlar", d),
            "sayı_mı" => self.cz("ohc_sayi_mi", d),
            "ondalık_mı" => self.cz("ohc_ondalik_mi", d),
            "sil" if matches!(t0, Tip::Sozluk(..)) => {
                self.cz("ohc_sozluk_sil", &[d[0], d[1], kod0])
            }
            "sil" => self.cz("ohc_liste_sil", &[d[0], d[1], satir]),
            "ters" if metin => self.cz("ohc_metin_ters", d),
            "ters" => self.cz("ohc_liste_ters", d),
            "karıştır" => self.cz("ohc_karistir", d),
            "kopya" => self.cz("ohc_liste_kopya", d),
            "en_büyük" | "en_küçük" => {
                let yon = self.sabit(if ad == "en_büyük" { 1 } else { -1 });
                if d.len() == 1 {
                    self.cz("ohc_liste_en", &[d[0], kod0, yon, satir])
                } else {
                    let kod = self.sabit(t0.kod());
                    self.cz("ohc_en_iki", &[d[0], d[1], kod, yon])
                }
            }
            "toplam" => self.cz("ohc_liste_toplam", &[d[0], kod0, satir]),
            "anahtarlar" => self.cz("ohc_sozluk_anahtarlar", d),
            "değerler" => self.cz("ohc_sozluk_degerler", d),
            "dosya_oku" => self.cz("ohc_dosya_oku", &[d[0], satir]),
            "dosyaya_yaz" | "dosyaya_ekle" => {
                let kip = self.sabit((ad == "dosyaya_ekle") as i64);
                self.cz("ohc_dosyaya_yaz", &[d[0], d[1], kip, satir])
            }
            "dosya_var" => self.cz("ohc_dosya_var", d),
            "dosya_sil" => self.cz("ohc_dosya_sil", d),
            "karekök" | "sinüs" | "kosinüs" | "tanjant" | "logaritma" if d.len() == 1 => {
                let islem = ["karekök", "sinüs", "kosinüs", "tanjant", "logaritma"]
                    .iter()
                    .position(|a| *a == ad)
                    .unwrap() as i64;
                let islem = self.sabit(islem);
                self.cz("ohc_matematik", &[islem, d[0], satir])
            }
            "logaritma" => self.cz("ohc_logaritma_taban", &[d[0], d[1], satir]),
            "üs" if e.tip == Tip::Sayi => self.cz("ohc_us_tam", &[d[0], d[1], satir]),
            "üs" => self.cz("ohc_us", d),
            "mutlak" if t0 == Tip::Ondalik => {
                let f = self.f64(d[0]);
                let r = self.b.ins().fabs(f);
                Some(self.bitler(r))
            }
            "mutlak" => self.cz("ohc_mutlak", &[d[0], satir]),
            "rastgele" if d.is_empty() => self.cz("ohc_rastgele", &[]),
            "rastgele" => self.cz("ohc_rastgele_aralik", &[d[0], d[1], satir]),
            "zaman" => self.cz("ohc_zaman", &[]),
            "tarih" => self.cz("ohc_tarih", &[]),
            "bekle" => self.cz("ohc_bekle", d),
            "argümanlar" => self.cz("ohc_argumanlar", &[]),
            "json" => {
                let kod = self.sabit(t0.kod());
                self.cz("ohc_json", &[d[0], kod])
            }
            "kaçır" => {
                let m = self.metne(d[0], &t0);
                self.cz("ohc_kacir", &[m])
            }
            "para" => self.cz("ohc_para", d),
            "url_kodla" => self.cz("ohc_url_kodla", d),
            "sun" => {
                let kapi = match d.first() {
                    Some(k) => *k,
                    None => self.sabit(0),
                };
                let t = self.tanim("İstek")?;
                self.cz("ohc_sun", &[kapi, t])
            }
            "ortam" => self.cz("ohc_ortam", d),
            "çık" => self.cz("ohc_cik", d),
            "hata_ver" => self.cz("ohc_hata_ver", &[d[0], satir]),
            _ => return Err(format!("bilinmeyen işlev '{ad}'")),
        };
        Ok(v)
    }

    fn metin_sabiti(&mut self, m: &str) -> Result<Value, String> {
        let id = self.ortak.metin_verisi(m)?;
        let gv = self.ortak.module.declare_data_in_func(id, self.b.func);
        Ok(self.b.ins().symbol_value(I64, gv))
    }

    /// Modelin çalışma zamanı tanımının (alan adları, tipler, kurallar) adresi.
    fn tanim(&mut self, model: &str) -> Result<Value, String> {
        let metin = self
            .ortak
            .modeller
            .get(model)
            .ok_or_else(|| format!("tanımsız model '{model}'"))?
            .tanim_metni();
        self.metin_sabiti(&metin)
    }

    /// Model nesnesi: [tanım, bağlama hataları, kimlik, alanlar...]
    fn nesne_kur(&mut self, model: &str, degerler: &[&Ifade]) -> Result<Value, String> {
        let l = self.cz("ohc_liste_yeni", &[]).unwrap();
        let t = self.tanim(model)?;
        self.cz("ohc_liste_ekle", &[l, t]);
        let sifir = self.sabit(0);
        self.cz("ohc_liste_ekle", &[l, sifir]);
        for d in degerler {
            let v = self.ifade(d)?;
            self.cz("ohc_liste_ekle", &[l, v]);
        }
        Ok(l)
    }

    /// Alanları varsayılan değerlerinde yeni bir nesne.
    fn varsayilan_nesne(&mut self, model: &str) -> Result<Value, String> {
        let degerler: Vec<Ifade> = self
            .ortak
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

    fn metod(
        &mut self,
        e: &Ifade,
        alici: &Ifade,
        ad: &str,
        arg: &[Ifade],
    ) -> Result<Value, String> {
        let satir = self.sabit(e.konum.satir as i64);
        let Tip::Model(model) = &alici.tip else {
            return Err(format!("'{ad}' yöntemi model olmayan bir değerde"));
        };
        let model = model.clone();
        let mut d = Vec::new();
        for a in arg {
            d.push(self.ifade(a)?);
        }
        if let IfadeTuru::ModelAdi(_) = alici.tur {
            let t = self.tanim(&model)?;
            return Ok(match ad {
                "hepsi" => self.cz("ohc_model_hepsi", &[t, satir]).unwrap(),
                "var_mı" => self.cz("ohc_model_var", &[t, d[0], satir]).unwrap(),
                "sil" => self.cz("ohc_model_sil", &[t, d[0], satir]).unwrap(),
                "bul" => {
                    let n = self.varsayilan_nesne(&model)?;
                    self.cz("ohc_model_yukle", &[n, d[0], satir]).unwrap()
                }
                "formdan" => {
                    let n = self.varsayilan_nesne(&model)?;
                    self.cz("ohc_model_doldur", &[n, d[0]]);
                    n
                }
                _ => return Err(format!("bilinmeyen yöntem '{ad}'")),
            });
        }
        let n = self.ifade(alici)?;
        Ok(match ad {
            "kaydet" => self.cz("ohc_model_kaydet", &[n, satir]).unwrap(),
            "sil" => {
                let yuva = self.sabit(ILK_ALAN);
                let k = self.cz("ohc_alan_al", &[n, yuva, satir]).unwrap();
                let t = self.tanim(&model)?;
                self.cz("ohc_model_sil", &[t, k, satir]).unwrap()
            }
            "geçerli_mi" => self.cz("ohc_model_gecerli", &[n]).unwrap(),
            "hatalar" => self.cz("ohc_model_hatalar", &[n]).unwrap(),
            "json" => {
                let kod = self.sabit(6);
                self.cz("ohc_json", &[n, kod]).unwrap()
            }
            _ => return Err(format!("bilinmeyen yöntem '{ad}'")),
        })
    }

    fn mantik(&mut self, cc: IntCC, a: Value, b: Value) -> Value {
        let c = self.b.ins().icmp(cc, a, b);
        self.b.ins().uextend(I64, c)
    }

    fn metne(&mut self, v: Value, t: &Tip) -> Value {
        let kod = self.sabit(t.kod());
        self.cz("ohc_metne_cevir", &[v, kod]).unwrap()
    }

    fn ifade(&mut self, e: &Ifade) -> Result<Value, String> {
        Ok(match &e.tur {
            IfadeTuru::Sayi(n) => self.sabit(*n),
            IfadeTuru::Ondalik(n) => self.sabit(n.to_bits() as i64),
            IfadeTuru::FiilCagri(ad, _) => {
                return Err(format!("'{ad}' fiil çağrısı denetimden geçmemiş"))
            }
            IfadeTuru::Kurucu(model, alanlar) => {
                let degerler: Vec<&Ifade> = alanlar.iter().map(|(_, d)| d).collect();
                self.nesne_kur(model, &degerler)?
            }
            IfadeTuru::Alan(nesne, _, sira) => {
                let n = self.ifade(nesne)?;
                let yuva = self.sabit(ILK_ALAN + *sira as i64);
                let s = self.sabit(e.konum.satir as i64);
                self.cz("ohc_alan_al", &[n, yuva, s]).unwrap()
            }
            IfadeTuru::Metod(alici, ad, arg) => self.metod(e, alici, ad, arg)?,
            IfadeTuru::ModelAdi(m) => {
                return Err(format!("'{m}' model adı değer olarak kullanıldı"))
            }
            IfadeTuru::Mantik(m) => self.sabit(*m as i64),
            IfadeTuru::Metin(m) => {
                let id = self.ortak.metin_verisi(m)?;
                let gv = self.ortak.module.declare_data_in_func(id, self.b.func);
                self.b.ins().symbol_value(I64, gv)
            }
            IfadeTuru::Isim(ad) => self.oku(ad),
            IfadeTuru::Liste(ogeler) => {
                let l = self.cz("ohc_liste_yeni", &[]).unwrap();
                for o in ogeler {
                    let v = self.ifade(o)?;
                    self.cz("ohc_liste_ekle", &[l, v]);
                }
                l
            }
            IfadeTuru::Tekli(TekliOp::Eksi, ic) if ic.tip == Tip::Ondalik => {
                let v = self.ifade(ic)?;
                let f = self.f64(v);
                let r = self.b.ins().fneg(f);
                self.bitler(r)
            }
            IfadeTuru::Tekli(TekliOp::Eksi, ic) => {
                let v = self.ifade(ic)?;
                self.b.ins().ineg(v)
            }
            IfadeTuru::Tekli(TekliOp::Degil, ic) => {
                let v = self.ifade(ic)?;
                self.b.ins().bxor_imm_u(v, 1)
            }
            IfadeTuru::Ikili(op @ (IkiliOp::Ve | IkiliOp::Veya), sol, sag) => {
                // Kısa devre: 've' için sol yanlışsa, 'veya' için sol doğruysa sağ hesaplanmaz.
                let sonuc = self.b.declare_var(I64);
                let a = self.ifade(sol)?;
                self.b.def_var(sonuc, a);
                let sag_blok = self.b.create_block();
                let son = self.b.create_block();
                if *op == IkiliOp::Ve {
                    self.b.ins().brif(a, sag_blok, &[], son, &[]);
                } else {
                    self.b.ins().brif(a, son, &[], sag_blok, &[]);
                }
                self.b.switch_to_block(sag_blok);
                let b = self.ifade(sag)?;
                self.b.def_var(sonuc, b);
                self.b.ins().jump(son, &[]);
                self.b.switch_to_block(son);
                self.b.use_var(sonuc)
            }
            IfadeTuru::Ikili(op, sol, sag) => {
                let mut a = self.ifade(sol)?;
                let mut b = self.ifade(sag)?;
                let satir = e.konum.satir as i64;
                match op {
                    IkiliOp::Topla if e.tip == Tip::Metin => {
                        if sol.tip != Tip::Metin {
                            a = self.metne(a, &sol.tip);
                        }
                        if sag.tip != Tip::Metin {
                            b = self.metne(b, &sag.tip);
                        }
                        self.cz("ohc_metin_birlestir", &[a, b]).unwrap()
                    }
                    // Denetçi karışık işlemlerde iki tarafı da ondalığa çevirmiştir.
                    _ if sol.tip == Tip::Ondalik => match op {
                        IkiliOp::Topla => self.fislem(a, b, |u, a, b| u.b.ins().fadd(a, b)),
                        IkiliOp::Cikar => self.fislem(a, b, |u, a, b| u.b.ins().fsub(a, b)),
                        IkiliOp::Carp => self.fislem(a, b, |u, a, b| u.b.ins().fmul(a, b)),
                        IkiliOp::Bol => {
                            let s = self.sabit(satir);
                            self.cz("ohc_ondalik_bol", &[a, b, s]).unwrap()
                        }
                        IkiliOp::Esit => self.fmantik(FloatCC::Equal, a, b),
                        IkiliOp::EsitDegil => self.fmantik(FloatCC::NotEqual, a, b),
                        IkiliOp::Kucuk => self.fmantik(FloatCC::LessThan, a, b),
                        IkiliOp::Buyuk => self.fmantik(FloatCC::GreaterThan, a, b),
                        IkiliOp::KucukEsit => self.fmantik(FloatCC::LessThanOrEqual, a, b),
                        IkiliOp::BuyukEsit => self.fmantik(FloatCC::GreaterThanOrEqual, a, b),
                        _ => return Err(format!("ondalık için desteklenmeyen işlem {op:?}")),
                    },
                    IkiliOp::Topla => {
                        let (r, tasti) = self.b.ins().sadd_overflow(a, b);
                        self.tasma_denetle(tasti, satir);
                        r
                    }
                    IkiliOp::Cikar => {
                        let (r, tasti) = self.b.ins().ssub_overflow(a, b);
                        self.tasma_denetle(tasti, satir);
                        r
                    }
                    IkiliOp::Carp => {
                        let (r, tasti) = self.b.ins().smul_overflow(a, b);
                        self.tasma_denetle(tasti, satir);
                        r
                    }
                    _ if sol.tip == Tip::Metin
                        && !matches!(op, IkiliOp::Esit | IkiliOp::EsitDegil) =>
                    {
                        // Türk alfabesine göre karşılaştırma
                        let k = self.cz("ohc_metin_kars", &[a, b]).unwrap();
                        let sifir = self.sabit(0);
                        let cc = match op {
                            IkiliOp::Kucuk => IntCC::SignedLessThan,
                            IkiliOp::Buyuk => IntCC::SignedGreaterThan,
                            IkiliOp::KucukEsit => IntCC::SignedLessThanOrEqual,
                            _ => IntCC::SignedGreaterThanOrEqual,
                        };
                        self.mantik(cc, k, sifir)
                    }
                    IkiliOp::Bol | IkiliOp::TamBol | IkiliOp::Mod => {
                        let s = self.sabit(satir);
                        let ad = if *op != IkiliOp::Mod {
                            "ohc_bol"
                        } else {
                            "ohc_mod"
                        };
                        self.cz(ad, &[a, b, s]).unwrap()
                    }
                    IkiliOp::Esit | IkiliOp::EsitDegil if sol.tip.metin_gibi() => {
                        let esit = self.cz("ohc_metin_esit", &[a, b]).unwrap();
                        if *op == IkiliOp::Esit {
                            esit
                        } else {
                            self.b.ins().bxor_imm_u(esit, 1)
                        }
                    }
                    IkiliOp::Esit => self.mantik(IntCC::Equal, a, b),
                    IkiliOp::EsitDegil => self.mantik(IntCC::NotEqual, a, b),
                    IkiliOp::Kucuk => self.mantik(IntCC::SignedLessThan, a, b),
                    IkiliOp::Buyuk => self.mantik(IntCC::SignedGreaterThan, a, b),
                    IkiliOp::KucukEsit => self.mantik(IntCC::SignedLessThanOrEqual, a, b),
                    IkiliOp::BuyukEsit => self.mantik(IntCC::SignedGreaterThanOrEqual, a, b),
                    IkiliOp::Ve | IkiliOp::Veya => unreachable!(),
                }
            }
            IfadeTuru::Indeks(l, i) => {
                let lv = self.ifade(l)?;
                let iv = self.ifade(i)?;
                let s = self.sabit(e.konum.satir as i64);
                match l.tip {
                    Tip::Metin => self.cz("ohc_metin_harf", &[lv, iv, s]).unwrap(),
                    Tip::Sozluk(..) => {
                        let kod = self.sabit(l.tip.ic_kod());
                        self.cz("ohc_sozluk_al", &[lv, iv, kod, s]).unwrap()
                    }
                    _ => self.cz("ohc_liste_al", &[lv, iv, s]).unwrap(),
                }
            }
            IfadeTuru::Sozluk(ciftler) => {
                let s = self.cz("ohc_sozluk_yeni", &[]).unwrap();
                let kod = self.sabit(e.tip.ic_kod());
                for (a, d) in ciftler {
                    let av = self.ifade(a)?;
                    let dv = self.ifade(d)?;
                    self.cz("ohc_sozluk_koy", &[s, av, dv, kod]);
                }
                s
            }
            IfadeTuru::Cagri(ad, arg) => {
                let mut degerler = Vec::new();
                for a in arg {
                    degerler.push(self.ifade(a)?);
                }
                if let Some((id, doner)) = self.ortak.islevler.get(ad).copied() {
                    let v = self.cagir(id, &degerler);
                    if doner {
                        v.unwrap()
                    } else {
                        self.sabit(0)
                    }
                } else {
                    match ad.as_str() {
                        "uzunluk" if arg[0].tip == Tip::Metin => {
                            self.cz("ohc_metin_uzunluk", &degerler).unwrap()
                        }
                        "uzunluk" if matches!(arg[0].tip, Tip::Sozluk(..)) => {
                            self.cz("ohc_sozluk_uzunluk", &degerler).unwrap()
                        }
                        "uzunluk" => self.cz("ohc_liste_uzunluk", &degerler).unwrap(),
                        "metin" => self.metne(degerler[0], &arg[0].tip),
                        "sayı" if arg[0].tip == Tip::Sayi => degerler[0],
                        "sayı" if arg[0].tip == Tip::Ondalik => {
                            let f = self.f64(degerler[0]);
                            self.b.ins().fcvt_to_sint_sat(I64, f)
                        }
                        "ondalık" if arg[0].tip == Tip::Ondalik => degerler[0],
                        "ondalık" if arg[0].tip == Tip::Sayi => {
                            let f = self.b.ins().fcvt_from_sint(types::F64, degerler[0]);
                            self.bitler(f)
                        }
                        "ondalık" => {
                            let s = self.sabit(e.konum.satir as i64);
                            self.cz("ohc_metinden_ondalik", &[degerler[0], s]).unwrap()
                        }
                        "yuvarla" if degerler.len() == 2 => {
                            self.cz("ohc_yuvarla_basamak", &degerler).unwrap()
                        }
                        "yuvarla" if arg[0].tip == Tip::Sayi => degerler[0],
                        "yuvarla" => self.cz("ohc_yuvarla", &degerler).unwrap(),
                        "sayı" => {
                            let s = self.sabit(e.konum.satir as i64);
                            self.cz("ohc_metinden_sayi", &[degerler[0], s]).unwrap()
                        }
                        "oku" => self.cz("ohc_oku", &[]).unwrap(),
                        SECENEK_CEVIR => {
                            let s = self.sabit(e.konum.satir as i64);
                            self.cz(
                                "ohc_secenek_cevir",
                                &[degerler[0], degerler[1], degerler[2], s],
                            )
                            .unwrap()
                        }
                        _ => {
                            let v = self.yerlesik(ad, arg, &degerler, e)?;
                            return Ok(v.unwrap_or_else(|| self.sabit(0)));
                        }
                    }
                }
            }
        })
    }
}
