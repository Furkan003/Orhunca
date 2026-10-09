//! Orhunca derleyicisi ve araçları.
//!
//! `orhunca` komut aracı (src/main.rs) ve masaüstü Orhunca Stüdyo (masaustu/)
//! bu kütüphaneyi kullanır.

pub mod adsiz;
pub mod agac;
pub mod ajan;
pub mod android;
pub mod arayuz;
pub mod ayiklama;
pub mod ayristirici;
pub mod baglayici;
pub mod basvuru;
pub mod bicimlendirici;
pub mod bootstrap;
pub mod cevirici;
pub mod cgi;
pub mod dap;
pub mod denetci;
pub mod derleme;
pub mod dil_sunucusu;
pub mod dokum;
pub mod ekler;
pub mod etkilesim;
pub mod goc;
pub mod guncelleme;
pub mod hata;
pub mod hizli_duzelt;
pub mod ios;
pub mod mcp;
pub mod on_kutuphane;
pub mod oneriler;
pub mod paket;
pub mod paket_denetim;
pub mod php;
pub mod referans;
pub mod sablon;
pub mod sinama;
pub mod sozcuk;
pub mod studyo;
pub mod uretici;
pub mod wasm_uretici;
pub mod yayinla;
pub mod yerlesik;

/// Derleyicinin sürümü (Cargo.toml).
pub const SURUM: &str = env!("CARGO_PKG_VERSION");

/// Derleyiciyi çalıştıran iş parçacıklarının yığın boyutu. Ayrıştırıcı ve denetçi
/// özyinelemelidir; ayristirici'deki derinlik sınırları bu boyutla güvenlidir.
pub const YIGIN: usize = 256 * 1024 * 1024;

/// İşletim sisteminin güvenli rastgele sayı üretecinden bayt. WebAssembly'de (tarayıcıda
/// deneme sayfası) böyle bir üreteç yoktur; çağıran yedek yolu kullanır.
pub fn guvenli_rastgele(b: &mut [u8]) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        getrandom::getrandom(b).map_err(|e| e.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = b;
        Err("bu ortamda güvenli rastgele sayı üreteci yok".into())
    }
}

/// Alt süreç komutu. Windows'ta masaüstü Stüdyo'nun (konsolu olmayan bir pencere
/// uygulaması) başlattığı her konsol programı için boş bir siyah pencere açılır; çıktı
/// Stüdyo'ya aktarıldığından o pencere boş kalır. CREATE_NO_WINDOW bunu önler.
pub fn komut(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}
