# «ad»

Bir çözüm: aynı kodu paylaşan iki uygulama.

| Klasör | Ne |
|---|---|
| `ortak/` | Hesap kodu ve sınamaları; iki uygulama da `kullan "../ortak/hesap.ohc"` ile alır |
| `web/` | Web sunucusu (sayfa ve JSON API); sayfaları `görünümler/` klasöründe |
| `mobil/` | Türkçe arayüz diliyle telefon uygulaması |

## Çalıştırma

```sh
orhunca çalıştır                                   # web sunucusu: http://localhost:3000
orhunca çalıştır mobil/uygulama.ohc                # telefon uygulaması tarayıcıda
orhunca paketle mobil/uygulama.ohc --hedef android # .apk
orhunca sına ortak                                 # ortak kodun sınamaları
```

Ortak koddaki bir düzeltme iki uygulamaya birden yansır. Yeni bir parça (ör. masaüstü
uygulaması) eklemek için yeni bir klasör açıp aynı `kullan` satırını yazmanız yeterli.
