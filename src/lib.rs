//! Orhunca derleyicisi ve araçları.
//!
//! `orhunca` komut aracı (src/main.rs) ve masaüstü Orhunca Stüdyo (masaustu/)
//! bu kütüphaneyi kullanır.

pub mod agac;
pub mod ajan;
pub mod android;
pub mod arayuz;
pub mod ayristirici;
pub mod baglayici;
pub mod bicimlendirici;
pub mod denetci;
pub mod derleme;
pub mod dil_sunucusu;
pub mod dokum;
pub mod ekler;
pub mod etkilesim;
pub mod guncelleme;
pub mod hata;
pub mod ios;
pub mod mcp;
pub mod on_kutuphane;
pub mod oneriler;
pub mod paket;
pub mod sablon;
pub mod sozcuk;
pub mod studyo;
pub mod uretici;
pub mod wasm_uretici;
pub mod yerlesik;

/// Derleyicinin sürümü (Cargo.toml).
pub const SURUM: &str = env!("CARGO_PKG_VERSION");

/// Derleyiciyi çalıştıran iş parçacıklarının yığın boyutu. Ayrıştırıcı ve denetçi
/// özyinelemelidir; ayristirici'deki derinlik sınırları bu boyutla güvenlidir.
pub const YIGIN: usize = 256 * 1024 * 1024;
