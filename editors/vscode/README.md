# Orhunca — VS Code eklentisi

[Orhunca](https://github.com/furkan003/orhunca) Türkçe tabanlı bir programlama dilidir. Bu eklenti
`.ohc` dosyaları ve `.ohchtml` görünümleri için şunları sağlar:

- Sözdizimi renklendirme (hâl ekleri, Türkçe anahtar kelimeler, fiiller, `model`, web yolları)
- `.ohchtml`: HTML içinde `@ürün.ad`, `@eğer … {`, `@her … {` ve `@model` renklendirmesi;
  görünümdeki hatalar proje derlenerek doğrudan dosyada gösterilir
- Yazarken hata gösterimi ve ünlü uyumu uyarıları (`5'a` → `5'e`)
- Üzerine gelince açıklama: yerleşik işlevler, anahtar kelimeler, değişken tipleri
- Tamamlama ve kod parçacıkları (`eğer`, `her … için`, `işlev`, `fiil` …)
- Biçimlendirme (Shift+Alt+F), tanıma gitme (F12), belge simgeleri
- **Orhunca: Çalıştır** (Ctrl+F5) ve **Orhunca: Stüdyo'yu aç** komutları

## Gereksinim

`orhunca` komutu kurulu olmalı (`cargo install --path .` ya da yayın dosyası). PATH'te değilse
Ayarlar → **orhunca.derleyiciYolu** ile tam yolunu verin.

## Paketleme

```sh
npm install
npx @vscode/vsce package
code --install-extension orhunca-0.5.0.vsix
```
