# Arayüz uygulamaları

> Düğmeli, girişli uygulamalar: durum, arayüz, olaylar.

Orhunca ile pencereli uygulamalar da yazılır. `durum` değişkenleri uygulamanın verisidir;
değiştiklerinde ekran kendiliğinden güncellenir. `arayüz:` bloğu ekranda ne görüneceğini anlatır:

```orhunca
durum sayaç = 0

arayüz:
    başlık("Sayaç")
    yazı("Değer: " + sayaç, boyut: 24)
    satır:
        düğme("−") tıklanınca:
            sayaç -= 1
        düğme("+") tıklanınca:
            sayaç += 1
```

Stüdyo'da F5 ile çalıştırınca uygulama sağdaki önizlemede açılır. `orhunca paketle sayaç.ohc`
ile kendi penceresinde açılan bir masaüstü uygulamasına dönüşür.

Öğeler: `başlık`, `yazı`, `düğme`, `giriş`, `onay_kutusu`, `seçim`, `kaydırıcı`, `resim`,
`bağlantı`; kapsayıcılar: `satır:`, `sütun:`, `kart:`, `ızgara(3):`. Olaylar: `tıklanınca`,
`değişince`, `gönderilince` (Enter).

```orhunca
durum ad = ""

arayüz:
    başlık("Selamlayıcı")
    giriş(ad, "Adınızı yazın")
    eğer ad ""'ye eşit değilse:
        yazı("Merhaba, " + ad + "!", renk: "yeşil", kalın: doğru)
```

## Alıştırma: Renk değiştirici

Bir düğmeye her tıklandığında yazının rengi `kırmızı` ve `mavi` arasında değişsin.

```orhunca baslangic
durum kırmızı_mı = doğru

arayüz:
    başlık("Renk değiştirici")
```

```orhunca cozum
durum kırmızı_mı = doğru

arayüz:
    başlık("Renk değiştirici")
    eğer kırmızı_mı ise:
        yazı("Merhaba!", renk: "kırmızı", boyut: 28)
    değilse:
        yazı("Merhaba!", renk: "mavi", boyut: 28)
    düğme("Rengi değiştir") tıklanınca:
        kırmızı_mı = değil kırmızı_mı
```
