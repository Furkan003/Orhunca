# Paketler

[← README](../README.md)


Paketler Git depolarındaki Orhunca kütüphaneleridir (`.ohcproj` + giriş dosyası).
Resmi paketler [kütüphaneler/](../kütüphaneler) klasöründedir ve paket dizininden adla eklenir
(Stüdyo'da: Paketler paneli → Paket dizini).

```sh
orhunca paket ara                  # paket dizini: istatistik, geometri …
orhunca paket ekle istatistik      # dizindeki paketi adıyla ekler
orhunca paket ekle github:kisi/orhunca-matematik#v1.0   # ya da tam Git adresi / yerel yol
orhunca paket ekle github:kisi/depo#v1.0:alt/klasör     # deponun bir alt klasöründeki paket
orhunca paket yükle        # .ohcproj ve orhunca.kilit'e göre kurar (dolaylı bağımlılıklar dahil)
orhunca paket güncelle     # en yeni sürümleri alır, kilidi yeniler
orhunca paket kaldır matematik
orhunca paket listele
```

```
# uygulama.ohcproj
[bağımlılıklar]
matematik = "github:kisi/orhunca-matematik#v1.0"
```

Kodda `kullan "matematik"` paketin giriş dosyasını, `kullan "matematik/geometri.ohc"` paketteki bir
dosyayı alır. Paketler `paketler/` klasörüne kurulur (`.gitignore`'a eklenir); kesin sürümler
`orhunca.kilit` dosyasında tutulur. Stüdyo'da **Paketler** paneli aynı işlemleri yapar.
