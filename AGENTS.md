# Yapay zekâ ajanları için: Orhunca deposu

Bu depo Orhunca programlama dilinin derleyicisini, çalışma zamanını ve Stüdyo'yu içerir.
Orhunca ile **program yazmak** için bu dosya değil, [docs/yapay-zeka.md](docs/yapay-zeka.md)
(`orhunca mcp`) ve dil rehberi ([docs/dil-rehberi.md](docs/dil-rehberi.md)) kullanılır.

## Yapı

- `src/`: Rust derleyicisi. Sırasıyla `sozcuk.rs` → `ayristirici.rs` → `denetci.rs` → `uretici.rs`
  (Cranelift) / `wasm_uretici.rs` dosyalarından geçer. `derleme.rs` bunları bir araya getirir.
- `src/studyo/` ve `studio/`: Stüdyo'nun sunucusu (Rust) ve arayüzü (düz JS/CSS, derleme adımı yok).
- `runtime/orhunca_rt.c`: C çalışma zamanı. `runtime/ön_kütüphane.ohc` ise Orhunca ile yazılmış
  ön kütüphanedir.
- `öz/`: Orhunca ile yazılmış ayrıştırıcı (öz-barındırma). `öz/ayrıştırıcı.ohc` dosyasındaki yerleşik
  listesi, `src/yerlesik.rs` içindeki `YERLESIKLER` ile aynı sırada olmalıdır.
- `örnekler/*.ohc` ve `.beklenen`: uçtan uca sınamalar.

## Kurallar

- Kod adları ve yorumlar Türkçe yazılır (ASCII'ye sadeleştirilmiş: `ayristirici`, `denetci`).
  Kullanıcının gördüğü her metin Türkçedir.
- `runtime/orhunca_rt.c` değişirse `araclar/calistiricilar.sh` ve `araclar/wasm_calisma_zamani.sh`
  ile hazır ikili dosyalar yeniden üretilir.

## Denetim

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- biçimlendir --denetle örnekler öz runtime/ön_kütüphane.ohc
```
