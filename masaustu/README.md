# Orhunca Stüdyo — masaüstü uygulaması

Orhunca Stüdyo'yu tarayıcı gerektirmeyen, kendi penceresinde açılan bir masaüstü uygulaması
olarak paketler ([Tauri 2](https://tauri.app)). Arayüz `orhunca stüdyo` ile aynıdır
(`studio/` klasörü); uygulama Stüdyo'nun yerel sunucusunu kendi içinde başlatır ve pencerede
gösterir. Pencere çerçevesizdir: başlık çubuğu ve küçült/büyüt/kapat düğmeleri Stüdyo
arayüzündedir, başlık çubuğundan sürüklenir.

![Orhunca Stüdyo masaüstü uygulaması (Linux, WebKitGTK)](../docs/ekran/masaustu.png)

## Çalıştırma ve paketleme

```sh
cd masaustu
cargo run --release                  # geliştirme sırasında doğrudan çalıştırma
cargo install tauri-cli --version "^2" --locked
cargo tauri build                    # kurulum dosyaları: target/release/bundle/
```

| Sistem | Gerekenler | Çıktı |
|---|---|---|
| Linux | `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev` | `orhunca-studyo_0.4.0_amd64.deb`, `.AppImage`, `.rpm` |
| Windows | WebView2 (Windows 10/11'de hazır) | `.msi`, kurulum `.exe` |
| macOS (deneysel) | Xcode komut satırı araçları | `.app`, `.dmg` |

GitHub'da **Masaüstü** iş akışı (`.github/workflows/masaustu.yml`) üç sistem için kurulum
dosyalarını üretir; sürüm etiketlerinde (`v0.4.0`) ve elle başlatıldığında çalışır.

## Kullanıcının bilgisayarında

Orhunca programları derlenirken bir C bağlayıcısı kullanılır:

- **Linux:** `gcc` (`.deb` paketi bağımlılık olarak kurar).
- **Windows:** [MinGW-w64](https://www.mingw-w64.org/) (`gcc` PATH'te olmalı ya da `ORHUNCA_CC`
  ile yolu verilmeli).

## Nasıl çalışır

- `src/main.rs`: `orhunca::studyo::arka_planda_baslat(0)` boş bir kapıda Stüdyo sunucusunu başlatır
  ve oturum anahtarlı adresi verir; pencere bu adresi açar.
- `capabilities/studyo.json`: yerel sunucudan yüklenen arayüzün yalnızca pencereyi küçültme,
  büyütme, kapatma ve sürükleme izni vardır.
- Uygulama kapanınca Stüdyo'dan çalıştırılan programlar (ör. web sunucuları) da durdurulur.
- "Önizlemeyi tarayıcıda aç" sistemin varsayılan tarayıcısını kullanır.
- Linux paket adı ASCII olmak zorunda olduğundan `tauri.linux.conf.json` paketi `orhunca-studyo`
  adıyla üretir; menüde görünen ad (`orhunca-studyo.desktop`) yine "Orhunca Stüdyo"dur.
