# Web sitesi

> Web sunucusu: yollar, modeller, JSON.

Orhunca ile bir web sitesi ya da API yazmak birkaç satırdır. `al "/yol":` tarayıcının o adrese
yaptığı isteği karşılar; döndürülen değer sayfada görünür. `sun(3000)` sunucuyu başlatır:

```orhunca
al "/":
    döndür "Merhaba, web!"

al "/saat":
    döndür "Şu an: " + tarih()

sun(3000)
```

Stüdyo'da F5'e basınca sunucu başlar ve sayfa canlı önizlemede açılır. Kendi bilgisayarınızda
tarayıcıda `http://localhost:3000` adresini de açabilirsiniz.

Modeller kalıcı olarak saklanır ve kendiliğinden JSON'a çevrilir:

```orhunca
model Not:
    başlık: metin
    içerik: metin

al "/notlar":
    döndür Not.hepsi()

gönder "/notlar":
    n = Not.formdan(istek)
    n'yi kaydet.
    döndür n

sun(3000)
```

Daha fazlası için Stüdyo'daki **Web Sitesi**, **Web API** ve **Tam Yığın** şablonlarına bakın:
sayfa şablonları (`.ohchtml`), oturumlar, dosya yükleme ve doğrulama kuralları.

## Alıştırma: Kişisel sayfa

`/` adresinde adınızı, `/hobiler` adresinde hobilerinizi listeleyen bir site yazın.

```orhunca baslangic
al "/":
    döndür "..."

sun(3000)
```

```orhunca cozum
al "/":
    döndür "Merhaba, ben Ece!"

al "/hobiler":
    döndür ["kitap okumak", "satranç", "kodlama"]

sun(3000)
```
