# Orhunca özeti (yapay zekâ ajanları için)

Orhunca Türkçe bir programlama dilidir (.ohc). Bloklar 4 boşluk girintiyle yazılır, blok açan satır
`:` ile biter. Cümleler nokta ile biter, atamalarda nokta yoktur. Yorum: `#`. Kodu yazdıktan sonra
mutlaka denetleyin; hata mesajlarındaki `ipucu:` satırı doğru yazımı gösterir. Kullanıcıya Türkçe
yanıt verin.

Hâl ekleri: belirtme `'ı/i/u/ü` (nesne), yönelme `'a/e` (hedef), ayrılma `'dan/den/tan/ten`
(kaynak, karşılaştırma), ilgi `'ın/in`. Sayı, metin ve parantezli ifadelerde kesme işareti
zorunludur ve ek ünlü uyumuna uyar: `5'i`, `10'u`, `3'ü`, `6'yı`, `"a"'yı`, `(a + b)'yi`, `4'ten`,
`10'a`. Değişkenlerde kesme isteğe bağlıdır: `sayıları` = `sayılar'ı`. Ek, ifadenin tamamına
aittir: `a + b'yi yaz.` → `(a + b)` yazılır.

```orhunca
ad = "Ayşe"
yaş = 20
fiyat = 19.99
notlar = [80, 95, 70]
stok = {"elma": 5}
isimler: liste<metin> = []

"Merhaba, " + ad'ı yaz.
5'i notlara ekle.
notları sırala.
uzunluk(notlar)'ı yaz.
stok["armut"] = 3

eğer yaş 18'den büyükse:
    "yetişkin"'i yaz.
değilse eğer yaş 18'e eşitse:
    "tam 18"'i yaz.
değilse:
    "çocuk"'u yaz.
eğer yaş >= 18 ve fiyat < 50 ise:
    "uygun"'u yaz.

her i için 1'den 5'e kadar:
    i'yi yaz.
her not için notlardan:
    eğer not 90'dan küçükse:
        sürdür
    not'u yaz.
x = 0
x 3'ten küçük olduğu sürece:
    x += 1

işlev topla(a, b):
    döndür a + b

işlev selamla(kişi: metin) -> metin:
    döndür "Merhaba, " + kişi

topla(2, 3)'ü yaz.

fiil sayı'yı karele:
    döndür sayı * sayı

kare = 7'yi karele
kare'yi yaz.

dene:
    sayı("abc")'yi yaz.
yakala hata:
    ("Hata: " + hata)'yı yaz.
```

- Tipler: `sayı` (tamsayı), `ondalık`, `metin`, `mantık` (`doğru`/`yanlış`), `liste<T>`,
  `sözlük<A, D>`. Değişkenin tipi değişemez; ondalık biriktirecekseniz `toplam = 0.0` ile başlatın.
- `/` ondalık verir (`7 / 2` → 3.5), `//` tam bölme, `%` kalan. Mantık: `ve`, `veya`, `değil`.
- Karşılaştırma: `x 4'ten büyükse`, `küçükse`, `x 4'e eşitse`, `eşit değilse`,
  `4'ten büyük veya eşitse` ya da simgelerle `eğer x > 4 ise:`. Döngüden çıkış `dur`.
- Liste/metin: `l[0]`, `uzunluk`, `içerir`, `bul`, `sil(l, sıra)`, `5'i l'den çıkar.`, `toplam`,
  `en_büyük`, `en_küçük`, `ters`, `parça(m, baş, uzunluk)`, `böl`, `birleştir`, `kırp`,
  `büyük_harf`, `küçük_harf`, `değiştir`.
- Dönüşüm: `metin(x)`, `sayı("42")`, `ondalık("3.5")`, `yuvarla(x, 2)`. Girdi: `oku()`.
  Rastgele: `rastgele(1, 6)`. Sabit: `sabit KDV = 0.20`. Başka dosya: `kullan "araçlar.ohc"`.
- Model (veri tipi): `model Kitap:` altında `ad: metin, zorunlu` gibi alanlar;
  `k = Kitap(ad: "Nutuk")`, `k.ad`, `k'yı kaydet.`, `Kitap.hepsi()`.
- Arayüz programı (tarayıcı, masaüstü, telefon): `durum sayaç = 0` ve `arayüz:` bloğu;
  `düğme("Artır") tıklanınca:`, `yazı(...)`, `giriş(ad)`.
- Web sunucusu: `al "/":` bloğunda `döndür "<h1>Merhaba</h1>"`; POST için `gönder "/yol":`.

Ayrıntılar (standart kütüphane, modeller, seçenekler, web, görünümler, arayüz öğeleri, oyunlar)
rehberin bölümlerindedir; emin olmadığınız konuda ilgili bölümü okuyun.
