# Öz-barındırma: Orhunca ile yazılmış Orhunca

Bir dilin olgunlaştığının işaretlerinden biri, derleyicisinin o dille yazılabilmesidir (Go, Rust ve
C bu yolu izledi). Orhunca'nın derleyicisi şimdilik Rust ile yazılı; bu klasör onun parçalarının
Orhunca'ya taşındığı yerdir.

## İlk adım: sözcük çözümleyici

`sözcük.ohc`, derleyicinin [src/sozcuk.rs](../src/sozcuk.rs) dosyasındaki sözcük çözümleyicinin
(lexer) Orhunca ile yazılmış karşılığıdır: girinti blokları, kesme işaretli ekler (`5'i`), sayılar
(`1_000`, `3.14`), metinler ve kaçış dizileri, üye noktası (`ürün.ad`) ve işleçler. Aynı
sözcükleri, aynı satır/sütun konumlarıyla ve aynı Türkçe hata mesajlarıyla üretir.

```sh
orhunca çalıştır öz/sözcükle.ohc -- örnekler/merhaba.ohc
```

```text
2:1 Metin "Merhaba, dünya!"
2:18 Ek yı
2:22 Kelime ekrana
2:29 Kelime yaz
2:32 Op .
2:33 YeniSatir
3:1 Son
```

`tests/oz.rs` iki çözümleyiciyi bütün örnek programlarda, şablonlarda ve hatalı girdilerde
sözcük sözcük karşılaştırır; Orhunca çözümleyicisi WebAssembly'ye derlenince de aynı sonucu verir.

Bu iş dile iki eklemeyi gerektirdi: tipi yazılmış değişkenler (`çıktı: liste<Sözcük> = []`) ve
karakter kodları (`kod("A")` = 65, `karakter(231)` = "ç").

## Sıradaki adımlar

1. Ayrıştırıcı (hâl ekleri, cümle yapısı) ve söz dizimi ağacı modelleri.
2. Tip denetçisi ve Türkçe hata mesajları.
3. Kod üretimi: WebAssembly (ikili biçimi Orhunca ile yazmak, Cranelift'ten bağımsızdır).
4. Derleyicinin kendisini derlemesi.

Eksikleri: Orhunca'da henüz hata yakalama (`dene / yakala`) ve sözcük türleri için numaralandırma
(enum) yok; hatalar `HATA` türünde bir sözcükle, türler metinle gösteriliyor.
