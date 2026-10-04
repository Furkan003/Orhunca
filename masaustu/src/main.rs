//! Orhunca Stüdyo masaüstü uygulaması.
//!
//! Stüdyo'nun yerel sunucusunu (orhunca::studyo) bu süreç içinde başlatır ve
//! arayüzü kendi penceresinde açar; tarayıcı gerekmez. Pencere çerçevesizdir:
//! başlık çubuğu ve pencere düğmeleri Stüdyo arayüzündedir (`window.__TAURI__`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    let adres = match orhunca::studyo::arka_planda_baslat(0) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Orhunca Stüdyo başlatılamadı: {e}");
            std::process::exit(1);
        }
    };
    let url = adres.parse().expect("Stüdyo adresi geçersiz");
    tauri::Builder::default()
        .setup(move |uygulama| {
            WebviewWindowBuilder::new(uygulama, "ana", WebviewUrl::External(url))
                .title("Orhunca Stüdyo")
                .inner_size(1360.0, 880.0)
                .min_inner_size(960.0, 600.0)
                .decorations(false)
                .center()
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Orhunca Stüdyo penceresi açılamadı")
        .run(|_, olay| {
            if let tauri::RunEvent::Exit = olay {
                // Stüdyo'dan çalıştırılan web sunucuları da kapanır.
                orhunca::studyo::kapat();
            }
        });
}
