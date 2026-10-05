# Oyun yapmak

Orhunca'da 2B oyunlar `oyun_alanı` öğesiyle yapılır. Oyun tarayıcıda, Stüdyo'nun canlı
önizlemesinde, masaüstü uygulamasında ve telefonda (Android/iOS) aynı kodla çalışır.

[← README](../README.md) · [Arayüz dili](arayuz.md)

```orhunca
durum x = 240.0
durum y = 160.0

arayüz:
    oyun_alanı(480, 320) her_karede:
        eğer tuş_basılı("sağ") ise:
            x += 4
        eğer tuş_basılı("sol") ise:
            x -= 4
        temizle("#101820")
        daire(x, y, 20, "turuncu")
```

Stüdyo'da: **Yeni proje → 2B Oyun**. Hazır örnek: [örnekler/oyunlar/top_yakala.ohc](../örnekler/oyunlar/top_yakala.ohc).

## Oyun döngüsü

`oyun_alanı(genişlik, yükseklik) her_karede:` bloğu saniyede yaklaşık 60 kez çalışır. Her
karede önce oyunun durumunu güncelleyin (konumlar, puan), sonra sahneyi baştan çizin. Oyunun
değişkenleri `durum` ile tanımlanır ve kareler arasında korunur.

Koordinatlar piksel cinsindendir. (0, 0) sol üst köşedir; x sağa, y aşağı doğru artar.

## Çizim

| Komut | Ne yapar |
|---|---|
| `temizle(renk)` | Bütün alanı boyar (her karenin başında) |
| `dikdörtgen(x, y, genişlik, yükseklik, renk)` | Dolu dikdörtgen |
| `daire(x, y, yarıçap, renk)` | Dolu daire |
| `çizgi(x1, y1, x2, y2, renk)` | Çizgi |
| `yazı_çiz(metin, x, y, renk)` · `yazı_çiz(metin, x, y, renk, boyut)` | Yazı (boyut piksel; varsayılan 16) |
| `resim_çiz(adres, x, y, genişlik, yükseklik)` | Resim (ör. statik/ klasöründeki bir PNG ya da bir internet adresi) |

Renkler Türkçe adlarla (`"kırmızı"`, `"turuncu"`, `"turkuaz"`, `"lacivert"` …) ya da `"#3366ff"`
biçiminde yazılır.

## Giriş ve ses

| Komut | Ne yapar |
|---|---|
| `tuş_basılı("sol")` | Tuş basılı mı? `"sol"`, `"sağ"`, `"yukarı"`, `"aşağı"`, `"boşluk"`, `"enter"`, harfler, rakamlar |
| `fare_x()`, `fare_y()` | Farenin ya da parmağın oyun alanındaki konumu |
| `fare_basılı()` | Fare basılı mı ya da ekrana dokunuluyor mu? |
| `ses(frekans, süre)` | Kısa bir ses: `ses(660, 0.1)` |

Oyun alanına `tıklanınca:` bloğu da eklenebilir.

## Çarpışma

Çarpışma için kendi fiilinizi yazabilirsiniz:

```orhunca
işlev çarpışıyor(x1: ondalık, y1: ondalık, x2: ondalık, y2: ondalık, mesafe: ondalık) -> mantık:
    dx = x1 - x2
    dy = y1 - y2
    döndür dx * dx + dy * dy < mesafe * mesafe
```

## Telefonda

Oyunu telefona taşımak için `orhunca paketle oyun.ohc --hedef android` yeterli. Telefonda
`fare_x()`, `fare_y()` ve `fare_basılı()` dokunmayla çalışır; `titret(30)` ile titreşim
ekleyebilirsiniz. Ayrıntılar: [Telefon uygulamaları](mobil.md).
