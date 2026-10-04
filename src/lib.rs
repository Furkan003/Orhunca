//! Orhunca derleyicisi ve araçları.
//!
//! `orhunca` komut aracı (src/main.rs) ve masaüstü Orhunca Stüdyo (masaustu/)
//! bu kütüphaneyi kullanır.

pub mod agac;
pub mod ayristirici;
pub mod bicimlendirici;
pub mod denetci;
pub mod derleme;
pub mod dil_sunucusu;
pub mod ekler;
pub mod hata;
pub mod paket;
pub mod sablon;
pub mod sozcuk;
pub mod studyo;
pub mod uretici;
pub mod yerlesik;

/// Derleyicinin sürümü (Cargo.toml).
pub const SURUM: &str = env!("CARGO_PKG_VERSION");
