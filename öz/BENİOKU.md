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

## İkinci adım: ayrıştırıcı

`ayrıştırıcı.ohc`, derleyicinin [src/ayristirici.rs](../src/ayristirici.rs) dosyasındaki
ayrıştırıcının karşılığıdır (yaklaşık 2000 satır Orhunca): programdaki isimlerden kurulan ek
çözümleme sözlüğü (ünsüz yumuşaması, ünlü düşmesi, belirsizlik), fiille biten cümleler, Türkçe
koşullar (`x 4'ten büyük veya eşitse`), işlevler, fiiller, modeller, seçenekler, web yolları ve
arayüz öğeleri. Söz dizimi ağacı iç içe modellerle kurulur; hatalar `hata_ver` ile atılıp en
dışta `dene / yakala` ile yakalanır.

```sh
orhunca çalıştır öz/ayrıştır.ohc -- örnekler/fiiller.ohc
```

Çıktı, derleyicinin ağaç dökümüyle ([src/dokum.rs](../src/dokum.rs)) aynı biçimdedir:

```text
işlev@4:1 karele (sayı ? Belirtme) -> -
  döndür@5:5 (ikili@5:12 Carp (isim@5:12 sayı) (isim@5:19 sayı))
işlev@10:1 düş (miktar ondalık Belirtme) (bakiye ondalık Ayrilma) -> ondalık
  eğer (ikili@11:10 Buyuk (isim@11:10 miktar) (isim@11:17 bakiye))
    yaz (metin@12:9 "Yetersiz bakiye")
```

`tests/oz.rs` iki ayrıştırıcıyı bütün örneklerde ve onlarca hatalı girdide satır satır
karşılaştırır; ayrıştırıcı kendi kaynağını da ayrıştırır.

Bu adım dilin eksiklerini de gösterdi; ayrıştırıcıyı yazmak için eklenenler: hata yakalama
(`dene / yakala`, `hata_ver`), seçenek türleri, iç içe modeller (`alt: liste<İfade>`),
indeksli birleşik atama ve parametre tipi çıkarımı.

## Sıradaki adımlar

1. Tip denetçisi ve Türkçe hata mesajları.
2. Kod üretimi: WebAssembly (ikili biçimi Orhunca ile yazmak, Cranelift'ten bağımsızdır).
3. Derleyicinin kendisini derlemesi.
