# Döngüler

> Bir işi tekrar tekrar yapmak: her ... için, ... iken, dur ve sürdür.

Bir sayı aralığında dönmek için `her ... için ...'den ...'e kadar:` yazılır; iki uç da dahildir:

```orhunca
her i için 1'den 5'e kadar:
    (i + ". tekrar")'ı yaz.
```

```cikti
1. tekrar
2. tekrar
3. tekrar
4. tekrar
5. tekrar
```

Bir koşul doğru olduğu sürece dönmek için `...iken:` ya da `... olduğu sürece:` yazılır:

```orhunca
para = 100
gün = 0
para 0'dan büyükken:
    para -= 30
    gün += 1
(gün + " günde para bitti.")'yi yaz.
```

```cikti
4 günde para bitti.
```

`dur` döngüden hemen çıkar, `sürdür` döngünün sonraki adımına geçer:

```orhunca
her n için 1'den 10'a kadar:
    eğer n % 2 == 0 ise:
        sürdür
    eğer n 7'den büyükse:
        dur
    n'yi yaz.
```

```cikti
1
3
5
7
```

## Alıştırma: Çarpım tablosu

Okunan sayının 1'den 5'e kadar çarpım tablosunu `3 x 1 = 3` biçiminde yazdırın.

```orhunca baslangic
n = sayı(oku())
```

```girdi
3
```

```cikti
3 x 1 = 3
3 x 2 = 6
3 x 3 = 9
3 x 4 = 12
3 x 5 = 15
```

```orhunca cozum
n = sayı(oku())
her i için 1'den 5'e kadar:
    (n + " x " + i + " = " + (n * i))'yi yaz.
```

## Alıştırma: 1'den 100'e toplam

1'den 100'e kadar olan sayıların toplamını bir döngüyle hesaplayıp yazdırın.

```orhunca baslangic
toplam = 0
```

```cikti
5050
```

```orhunca cozum
toplam = 0
her i için 1'den 100'e kadar:
    toplam += i
toplam'ı yaz.
```
