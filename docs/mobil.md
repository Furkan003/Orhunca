# Telefon uygulamaları (Android ve iOS)

Orhunca'da yazılan bir arayüz programı, tek kodla hem Android hem iPhone/iPad uygulamasına
dönüşür.

[← README](../README.md) · [Arayüz dili](arayuz.md)

```orhunca
durum sayaç = 0

arayüz:
    başlık("Sayaç: " + sayaç)
    düğme("Artır") tıklanınca:
        sayaç += 1
        titret(30)
```

## Telefonda ve tablette kod yazmak

Kod yazmak için bilgisayar gerekmez: [tarayıcıda dene](https://furkan003.github.io/Orhunca/dene.html)
sayfası telefonda ve tablette de çalışır ve uygulama olarak kurulabilir.

- **Kurulum.** Android'de (Chrome) **Programlarım → Uygulama olarak kur** ya da tarayıcı menüsünden
  *Ana ekrana ekle*. iPhone/iPad'de (Safari) *Paylaş → Ana Ekrana Ekle*.
- **İnternetsiz.** Sayfa bir kez açıldıktan sonra derleyici cihazda saklanır; internet olmadan da
  kod yazılıp çalıştırılır. Programlar hiçbir yere gönderilmez.
- **Sembol çubuğu.** Kod yazarken klavyenin üstünde telefon klavyesinde zor bulunan işaretler
  (`'`, `:`, `"`, parantezler, girinti) ve Türkçe harfler (ç, ğ, ı, ö, ş, ü) çıkar. ▶ ile çalıştırılır.
- **Programlarım.** Yazdıklarınız cihazda kendiliğinden kaydedilir. Birden çok program tutulabilir,
  adlandırılabilir, `.ohc` dosyası olarak indirilip bilgisayardaki Stüdyo'da açılabilir.

## Android

```sh
orhunca paketle sayaç.ohc --hedef android        # → sayaç.apk
```

Stüdyo'da: **Çalıştır paneli → Telefon (Android .apk)**.

- Android SDK ya da Android Studio gerekmez. Hazır kabuk kullanılır; birkaç saniyede imzalı
  bir `.apk` dosyası üretilir.
- **Telefona kurmak:** `.apk` dosyasını telefona gönderin (e-posta, USB, Drive…) ve açın. Android
  ilk seferde "bilinmeyen kaynaklardan yüklemeye izin ver" ister.
- **İmza anahtarı:** İlk paketlemede oluşturulur ve ayar klasöründe saklanır
  (Linux `~/.config/orhunca/android-imza.anahtar`, Windows `%APPDATA%\Orhunca\android-imza.anahtar`).
  Uygulamanın yeni sürümünü aynı telefona kurabilmek için anahtar aynı kalmalıdır; bu dosyayı
  yedekleyin.
- Android 7.0 ve sonrasında çalışır.

## iPhone ve iPad

```sh
orhunca paketle sayaç.ohc --hedef ios            # → sayaç-ios/ (Xcode projesi)
```

Apple, iPhone uygulamalarının yalnızca macOS'taki Xcode ile derlenmesine ve Apple hesabıyla
imzalanmasına izin verir. Bu yüzden Orhunca çalıştırılmaya hazır bir Xcode projesi üretir:

- **Mac'iniz varsa:** `Uygulama.xcodeproj` dosyasını Xcode ile açın. *Signing & Capabilities*
  bölümünde Apple hesabınızı seçin ve iPhone'unuzu bağlayıp ▶ düğmesine basın. Ücretsiz Apple
  hesabı kendi telefonunuz için yeterlidir.
- **Mac'iniz yoksa:** Klasörü bir GitHub deposuna yükleyin. İçindeki `.github/workflows/ios.yml`
  GitHub'ın Mac'lerinde imzasız bir `.ipa` dosyası üretir. Bu dosya AltStore ya da Sideloadly ile
  iPhone'a kurulabilir.
- **App Store:** Apple Developer Program üyeliği gerekir (yıllık ücretli). Xcode'da
  *Product → Archive → Distribute App* yolunu izleyin.

## Telefon komutları

| Komut | Ne yapar |
|---|---|
| `titret(200)` | Telefonu 200 milisaniye titretir |
| `paylaş("Puanım: " + puan)` | Paylaşma penceresini açar (WhatsApp, e-posta…) |
| `bildirim_gönder("Başlık", "Metin")` | Bildirim gösterir; ilk seferde izin ister |

Bu komutlar tarayıcıda da çalışır (destekleyen tarayıcılarda). Bilgisayar programında etkisizdir.

## Kamera, karekod, konum ve dosya

Bu dört öğe birer düğme olarak görünür. Sonuç gelince bağlı değişken güncellenir ve
`değişince:` bloğu çalışır. İzin ilk kullanımda sorulur.

| Öğe | Değişkene yazılan |
|---|---|
| `kamera(fotoğraf, "Fotoğraf çek")` | Çekilen fotoğraf (resim adresi; `resim(fotoğraf)` ile gösterilir) |
| `karekod_okuyucu(kod, "Karekod okut")` | Okunan karekod ya da barkod metni |
| `konum(yer, "Konumumu bul")` | `"enlem,boylam"` (ör. `"41.008200,28.978400"`; `böl(yer, ",")` ile ayrılır) |
| `dosya_seç(içerik, "Dosya seç", tür: ".csv,.txt")` | Metin dosyasının içeriği; resim ve diğer dosyalarda resim adresi |

```orhunca
durum kod = ""
arayüz:
    karekod_okuyucu(kod, "Ürün okut") değişince:
        titret(100)
    yazı("Okunan: " + kod)
```

Karekod okuma tarayıcının `BarcodeDetector` desteğini kullanır: Android uygulamasında,
Chrome ve Edge'de çalışır. Kamera ve konum, tarayıcıda yalnızca `https://` ya da
`localhost` adreslerinde açılır.

## Uygulama ayarları

Proje dosyasına (`.ohcproj`) eklenebilir. Hepsi isteğe bağlıdır:

```
ad = "Sınıf Defteri"            # ana ekrandaki ad (yoksa dosya adı)
sürüm = "1.2.0"
paket_kimliği = "org.okulum.sinif_defteri"   # her uygulama için tekil; yoksa org.orhunca.<ad>
simge = "simge.png"             # PNG; iOS için 1024×1024 ve saydamlıksız
```

Proje klasöründe `simge.png` dosyası varsa simge olarak o kullanılır.

## Nasıl çalışır?

Program WebAssembly'ye derlenir ve tek bir sayfa olur. Bu sayfa telefonun yerleşik tarayıcı
motorunda (Android WebView, iOS WKWebView) tam ekran açılır. Telefon komutları, kabuktaki küçük
bir köprü aracılığıyla telefonun kendi özelliklerine ulaşır. Kabukların kaynağı
[mobil/](../mobil) klasöründedir.
