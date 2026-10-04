# Sözlükler ve metinler

> Anahtar–değer çiftleri; metinlerle çalışmak.

**Sözlük**, her anahtara bir değer bağlar. Süslü parantezle yazılır:

```orhunca
stok = {"elma": 12, "armut": 5}
stok["kiraz"] = 30
stok["elma"] = stok["elma"] - 2
her meyve için stok'tan:
    (meyve + ": " + stok[meyve])'yi yaz.
```

```cikti
elma: 10
armut: 5
kiraz: 30
```

`içerir(s, anahtar)` anahtar var mı diye bakar, `sil(s, anahtar)` siler.

Metinler için hazır işlevler vardır: `büyük_harf`, `küçük_harf` (Türkçe i/İ, ı/I doğru
çevrilir), `uzunluk`, `böl`, `içerir`, `değiştir`, `kırp`, `ters`:

```orhunca
cümle = "Orhun yazıtları Türkçenin ilk yazılı belgeleridir"
büyük_harf("istanbul")'u yaz.
uzunluk(cümle)'yi yaz.
kelimeler = böl(cümle, " ")
uzunluk(kelimeler)'i yaz.
değiştir(cümle, "ilk", "en eski")'yi yaz.
```

```cikti
İSTANBUL
49
6
Orhun yazıtları Türkçenin en eski yazılı belgeleridir
```

## Alıştırma: Harf sayacı

Okunan kelimede her harfin kaç kez geçtiğini sayıp, harflerin ilk görülme sırasıyla yazdırın.
Bir metnin harflerini `her h için kelime'den:` ile gezebilirsiniz.

```orhunca baslangic
kelime = oku()
sayaç: sözlük<metin, sayı> = {}
```

```girdi
ankara
```

```cikti
a: 3
n: 1
k: 1
r: 1
```

```orhunca cozum
kelime = oku()
sayaç: sözlük<metin, sayı> = {}
her h için kelime'den:
    eğer içerir(sayaç, h) ise:
        sayaç[h] = sayaç[h] + 1
    değilse:
        sayaç[h] = 1
her h için sayaç'tan:
    (h + ": " + sayaç[h])'yi yaz.
```
