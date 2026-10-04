# Koşullar

> eğer, değilse eğer, değilse; karşılaştırmalar ve mantık.

Program bir koşula göre farklı şeyler yapabilir. `eğer` satırı `:` ile biter; koşul doğruysa
altındaki **içeri girintili** satırlar çalışır (girinti 4 boşluktur):

```orhunca
puan = 72
eğer puan 50'den büyükse:
    "Geçti"'yi yaz.
değilse:
    "Kaldı"'yı yaz.
```

```cikti
Geçti
```

Karşılaştırmalar Türkçe yazılabilir: `x 5'ten büyükse`, `x 5'ten küçükse`, `x 5'e eşitse`,
`x 5'e eşit değilse`, `x 5'ten büyük veya eşitse`. Simgelerle de yazılabilir:
`eğer x > 5 ise:` — simgeler `==`, `!=`, `<`, `>`, `<=`, `>=`.

Birden çok durum için `değilse eğer` kullanılır:

```orhunca
sıcaklık = 18
eğer sıcaklık 25'ten büyükse:
    "Sıcak"'ı yaz.
değilse eğer sıcaklık 10'dan büyükse:
    "Ilık"'ı yaz.
değilse:
    "Soğuk"'u yaz.
```

```cikti
Ilık
```

Koşullar `ve`, `veya`, `değil` ile birleşir:

```orhunca
yaş = 15
bilet_var = doğru
eğer yaş >= 12 ve bilet_var ise:
    "Girebilirsin."'i yaz.
```

```cikti
Girebilirsin.
```

## Alıştırma: Tek mi çift mi?

Okunan sayı çiftse `çift`, tekse `tek` yazdırın. İpucu: çift sayıların 2'ye bölümünden kalan
0'dır (`n % 2`).

```orhunca baslangic
n = sayı(oku())
```

```girdi
7
```

```cikti
tek
```

```orhunca cozum
n = sayı(oku())
eğer n % 2 == 0 ise:
    "çift"'i yaz.
değilse:
    "tek"'i yaz.
```

## Alıştırma: Not harfi

Okunan notu harfe çevirin: 85 ve üstü `A`, 70 ve üstü `B`, 50 ve üstü `C`, altı `F`.

```orhunca baslangic
puan = sayı(oku())
```

```girdi
78
```

```cikti
B
```

```orhunca cozum
puan = sayı(oku())
eğer puan >= 85 ise:
    "A"'yı yaz.
değilse eğer puan >= 70 ise:
    "B"'yi yaz.
değilse eğer puan >= 50 ise:
    "C"'yi yaz.
değilse:
    "F"'yi yaz.
```
