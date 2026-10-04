# Hata yakalama

> Programın çökmesini önlemek: dene, yakala, hata_ver.

Bazı işlemler çalışırken hata verebilir: sıfıra bölme, sayıya çevrilemeyen bir metin, listede
olmayan bir sıra numarası. `dene:` bloğunda hata olursa program durmaz, `yakala` bloğu çalışır:

```orhunca
girdiler = ["42", "kırk", "7"]
her g için girdiler'den:
    dene:
        n = sayı(g)
        (n * 2)'yi yaz.
    yakala hata:
        ("Sayı değil: " + g)'yi yaz.
```

```cikti
84
Sayı değil: kırk
14
```

`yakala hata:` satırındaki `hata` değişkeni hatanın mesajını tutar. Kendi hatanızı
`hata_ver("mesaj")` ile verebilirsiniz:

```orhunca
işlev yaş_kontrol(yaş: sayı):
    eğer yaş < 0 veya yaş > 150 ise:
        hata_ver("geçersiz yaş: " + yaş)
    ("Yaş tamam: " + yaş)'ı yaz.

dene:
    yaş_kontrol(30)
    yaş_kontrol(-4)
yakala hata:
    hata'yı yaz.
```

```cikti
Yaş tamam: 30
geçersiz yaş: -4
```

## Alıştırma: Güvenli bölme

İki sayı okuyun ve birinciyi ikinciye tam bölün (`//`). İkinci sayı 0 ise program çökmesin,
`Sıfıra bölünemez` yazsın.

```orhunca baslangic
a = sayı(oku())
b = sayı(oku())
(a // b)'yi yaz.
```

```girdi
10
0
```

```cikti
Sıfıra bölünemez
```

```orhunca cozum
a = sayı(oku())
b = sayı(oku())
dene:
    (a // b)'yi yaz.
yakala:
    "Sıfıra bölünemez"'i yaz.
```
