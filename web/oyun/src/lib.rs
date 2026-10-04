//! Tarayıcıda Orhunca: kaynak kodu WebAssembly programına derler (sunucu gerekmez).
//!
//! JavaScript tarafı: `ayir(n)` ile bellek alır, kaynağı yazar, `derle(adres, uzunluk)`
//! çağırır. Sonuç: [u32 uzunluk][1 bayt tür][veri]; tür 0: program (wasm), 1: hata
//! (JSON: {mesaj, satir, sutun}), 2: arayüz programı (wasm). `birak(adres)` belleği geri verir.

use std::collections::HashMap;
use std::path::PathBuf;

#[no_mangle]
pub extern "C" fn ayir(n: u32) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(n as usize + 4);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    // Kapasite, birak() için başta saklanmaz; JS aynı boyutu geri verir.
    p
}

/// # Safety
/// `adres`, `ayir(n)` ile alınmış ve `n` aynı olmalı.
#[no_mangle]
pub unsafe extern "C" fn birak(adres: *mut u8, n: u32) {
    drop(Vec::from_raw_parts(adres, 0, n as usize + 4));
}

fn sonuc(tur: u8, veri: &[u8]) -> *mut u8 {
    let mut v = Vec::with_capacity(veri.len() + 5);
    v.extend((veri.len() as u32 + 1).to_le_bytes());
    v.push(tur);
    v.extend(veri);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `adres`/`uzunluk` geçerli bir UTF-8 kaynak metnini göstermeli.
#[no_mangle]
pub unsafe extern "C" fn derle(adres: *const u8, uzunluk: u32) -> *mut u8 {
    let kaynak = String::from_utf8_lossy(std::slice::from_raw_parts(adres, uzunluk as usize));
    let yol = PathBuf::from("program.ohc");
    let ortulu = HashMap::from([(yol.clone(), kaynak.into_owned())]);
    match orhunca::derleme::yukle_ortulu(&yol, &ortulu) {
        Ok(p) => match orhunca::wasm_uretici::uret(&p) {
            Ok(w) => sonuc(if p.arayuz_programi() { 2 } else { 0 }, &w),
            Err(e) => hata(&e, 0, 0),
        },
        Err(h) => {
            let (satir, sutun) = h.teshis.as_ref().map(|t| (t.satir, t.sutun)).unwrap_or((0, 0));
            hata(&h.metin, satir, sutun)
        }
    }
}

fn hata(mesaj: &str, satir: usize, sutun: usize) -> *mut u8 {
    let j = serde_json::json!({ "mesaj": mesaj, "satir": satir, "sutun": sutun });
    sonuc(1, j.to_string().as_bytes())
}
