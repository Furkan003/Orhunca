# Modeller

> Kendi veri türlerini tanımlamak: model, alanlar, seçenekler.

**Model**, birlikte anlam taşıyan değerleri bir arada tutar. Alanların adı ve tipi yazılır:

```orhunca
model Öğrenci:
    ad: metin
    numara: sayı
    not_ortalaması: ondalık

ö = Öğrenci(ad: "Deniz", numara: 1071, not_ortalaması: 87.5)
ö.ad'ı yaz.
ö.not_ortalaması = 90.0
(ö.ad + " → " + ö.not_ortalaması)'nı yaz.
```

```cikti
Deniz
Deniz → 90.0
```

Modeller listelerde tutulabilir:

```orhunca
model Kitap:
    ad: metin
    sayfa: sayı

kitaplar = [Kitap(ad: "Kutadgu Bilig", sayfa: 680), Kitap(ad: "Dede Korkut", sayfa: 320)]
toplam_sayfa = 0
her k için kitaplar'dan:
    toplam_sayfa += k.sayfa
("Toplam sayfa: " + toplam_sayfa)'yı yaz.
```

```cikti
Toplam sayfa: 1000
```

**Seçenek** (numaralandırma), bir değerin alabileceği sabit durumları tanımlar:

```orhunca
seçenek Mevsim: İlkbahar, Yaz, Sonbahar, Kış

m = Mevsim.Kış
eğer m == Mevsim.Kış ise:
    "Kar yağabilir."'i yaz.
```

```cikti
Kar yağabilir.
```

## Alıştırma: En uzun kitap

`kitaplar` listesindeki en çok sayfalı kitabın adını yazdırın.

```orhunca baslangic
model Kitap:
    ad: metin
    sayfa: sayı

kitaplar = [Kitap(ad: "Çalıkuşu", sayfa: 512), Kitap(ad: "Sinekli Bakkal", sayfa: 448), Kitap(ad: "Saatleri Ayarlama Enstitüsü", sayfa: 560)]
```

```cikti
Saatleri Ayarlama Enstitüsü
```

```orhunca cozum
model Kitap:
    ad: metin
    sayfa: sayı

kitaplar = [Kitap(ad: "Çalıkuşu", sayfa: 512), Kitap(ad: "Sinekli Bakkal", sayfa: 448), Kitap(ad: "Saatleri Ayarlama Enstitüsü", sayfa: 560)]
en_uzun = kitaplar[0]
her k için kitaplar'dan:
    eğer k.sayfa > en_uzun.sayfa ise:
        en_uzun = k
en_uzun.ad'ı yaz.
```
