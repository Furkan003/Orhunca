# Topluluk temaları

Orhunca Stüdyo'nun *Ayarlar → Görünüm ve temalar → Topluluk* sekmesinde görünen temalar.

## Kendi temanızı paylaşın

1. Stüdyo'da *Ayarlar → Görünüm ve temalar*'ı açın, renkleri, yazı tiplerini ve isterseniz arka
   plan resmini (resim, GIF ya da kısa video) ayarlayın.
2. **Dışa aktar** ile `.ohctema` dosyasını kaydedin.
3. Dosyayı bu klasöre ekleyip `dizin.json` dosyasına bir satır ekleyen bir çekme isteği açın:

```json
{ "ad": "Temanın adı", "yazar": "GitHub adınız", "dosya": "temanin-adi.ohctema",
  "renkler": { "arka": "#...", "pencere": "#...", "vurgu": "#...", "sz-anahtar": "#...", "sz-metin": "#...", "sz-islev": "#..." } }
```

Dosya adında yalnızca küçük harf, rakam ve `-` kullanın. Arka plan resmi temanın içine gömülür;
galeri için 2 MB'tan küçük tutun. Temalar dış adres yükleyemez (özel CSS'te `@import` ve `http`
adresleri çalışmaz).

Kendi bilgisayarınızda bir temayı paylaşmak için `.ohctema` dosyasını göndermeniz yeterli: alan kişi
*İçe aktar* ile açar.
