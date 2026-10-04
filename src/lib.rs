//! Orhunca derleyicisi ve araçları.
//!
//! `orhunca` komut aracı (src/main.rs) ve masaüstü Orhunca Stüdyo (masaustu/)
//! bu kütüphaneyi kullanır.

pub mod agac;
pub mod arayuz;
pub mod ayristirici;
pub mod baglayici;
pub mod bicimlendirici;
pub mod denetci;
pub mod derleme;
pub mod dil_sunucusu;
pub mod dokum;
pub mod ekler;
pub mod hata;
pub mod on_kutuphane;
pub mod paket;
pub mod sablon;
pub mod sozcuk;
pub mod studyo;
pub mod uretici;
pub mod wasm_uretici;
pub mod yerlesik;

/// Derleyicinin sürümü (Cargo.toml).
pub const SURUM: &str = env!("CARGO_PKG_VERSION");
