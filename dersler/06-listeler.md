# Listeler

> Birçok değeri bir arada tutmak: ekleme, sıra numarası, gezme, sıralama.

Liste köşeli parantezle yazılır. Öğelere **sıra numarasıyla** ulaşılır; ilk öğe 0'dır:

```orhunca
meyveler = ["elma", "armut", "kiraz"]
meyveler[0]'ı yaz.
uzunluk(meyveler)'i yaz.
```

```cikti
elma
3
```

Listeye eklemek, listeden çıkarmak ve sıralamak birer cümledir:

```orhunca
sayılar = [42, 7, 19]
3'ü sayılara ekle.
7'yi sayılardan çıkar.
sayıları sırala.
sayıları yaz.
```

```cikti
[3, 19, 42]
```

Listeyi baştan sona gezmek için `her ... için ...'den:` kullanılır:

```orhunca
notlar = [85, 92, 78]
her n için notlar'dan:
    eğer n 80'den büyükse:
        n'yi yaz.
```

```cikti
85
92
```

Hazır işlevler: `toplam(l)`, `en_büyük(l)`, `en_küçük(l)`, `içerir(l, x)`, `ters(l)`.

## Alıştırma: Sınıf ortalaması

`notlar` listesinin ortalamasını hesaplayın ve en yüksek notu yazdırın.

```orhunca baslangic
notlar = [70, 85, 90, 55, 100]
```

```cikti
Ortalama: 80.0
En yüksek: 100
```

```orhunca cozum
notlar = [70, 85, 90, 55, 100]
ortalama = toplam(notlar) / uzunluk(notlar)
("Ortalama: " + ortalama)'yı yaz.
("En yüksek: " + en_büyük(notlar))'i yaz.
```

## Alıştırma: Ters sıra

Kullanıcının yazdığı 3 kelimeyi bir listeye ekleyip ters sırada yazdırın.

```orhunca baslangic
kelimeler: liste<metin> = []
```

```girdi
bir
iki
üç
```

```cikti
üç
iki
bir
```

```orhunca cozum
kelimeler: liste<metin> = []
her i için 1'den 3'e kadar:
    oku()'yu kelimelere ekle.
her k için ters(kelimeler)'den:
    k'yi yaz.
```
