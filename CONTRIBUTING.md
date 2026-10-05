# Orhunca'ya katkı

Orhunca'yı birlikte geliştirdiğimiz için teşekkürler! Hata bildirmek, ders yazmak, örnek
eklemek, belge düzeltmek ya da kod göndermek — her katkı değerli.

## Nereden başlamalı?

- **Hata bildirmek:** [yeni bir konu açın](https://github.com/Furkan003/Orhunca/issues/new/choose)
  ve "Hata bildirimi" şablonunu doldurun. Hatayı gösteren en kısa `.ohc` programı çok yardımcı olur.
- **Fikir önermek:** "Öneri" şablonunu kullanın; önce benzer bir konu olup olmadığına bakın.
- **Ders ya da örnek yazmak:** [dersler/](dersler) ve [örnekler/](örnekler) klasörleri. Ders biçimi
  [araclar/dersler.py](araclar/dersler.py) içinde açıklanır; her örneğin yanında beklenen çıktısı
  (`.beklenen`) bulunur.
- **Paket yayımlamak:** Orhunca paketleri Git depolarıdır; paket dizinine eklemek için
  [kütüphaneler/dizin.json](kütüphaneler/dizin.json) dosyasına bir satır ekleyen çekme isteği açın.
- **Tema paylaşmak:** Stüdyo'da *Ayarlar → Görünüm → Dışa aktar* ile `.ohctema` dosyası oluşturup
  [temalar/](temalar) klasörüne ekleyen bir çekme isteği açın.

## Geliştirme ortamı

```sh
git clone https://github.com/Furkan003/Orhunca && cd Orhunca
cargo build            # derleyici + Stüdyo
cargo test             # bütün sınamalar
cargo run -- stüdyo    # Stüdyo'yu kaynaktan aç
```

Gerekenler: Rust (kararlı sürüm). Çalışma zamanı (`runtime/orhunca_rt.c`) değişirse hazır
çalıştırıcılar `araclar/calistiricilar.sh`, WebAssembly çalışma zamanı
`araclar/wasm_calisma_zamani.sh` ile yeniden derlenir. Ayrıntılar: [docs/gelistirme.md](docs/gelistirme.md).

## Çekme isteği göndermeden önce

1. `cargo fmt` ve `cargo clippy --all-targets -- -D warnings` temiz olmalı.
2. `cargo test` geçmeli.
3. Yeni bir dil özelliği için `örnekler/` klasörüne `.ohc` + `.beklenen` çifti ekleyin.
4. Orhunca kaynakları biçimli olmalı: `cargo run -- biçimlendir --denetle örnekler öz`.
5. Kullanıcıya görünen bütün metinler (hata mesajları, arayüz) Türkçedir.

Kod adları Türkçedir (ASCII'ye sadeleştirilmiş: `ayristirici`, `denetci`); yorumlar Türkçe yazılır.

## Davranış kuralları

Bu proje [davranış kurallarına](CODE_OF_CONDUCT.md) uyar. Okullarda kullanılan bir proje olduğu
için herkese — özellikle öğrencilere ve yeni başlayanlara — saygılı ve sabırlı olalım.

## Lisans

Gönderdiğiniz katkılar projenin [MIT lisansı](LICENSE) ile dağıtılır.
