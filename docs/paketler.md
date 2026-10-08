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

## Paket mağazası ve güvenlik

Paket mağazası GitHub'daki [orhunca-paketler](https://github.com/Furkan003/orhunca-paketler)
deposudur; sunucu yoktur. `orhunca paket ara` ve Stüdyo'nun paket dizini oradan okur.

- **İzinler:** Orhunca bir paketin dosya okuyup yazdığını (`dosya`), internete bağlandığını
  (`ağ`), ortam değişkenlerini okuduğunu (`ortam`), C kütüphanesi çağırdığını (`c_kütüphanesi`)
  ya da web sunucusu başlattığını (`sunucu`) kodundan kendisi çıkarır. Paket eklenirken izinler
  gösterilir ve onayınız istenir; bir güncelleme yeni izin isterse yeniden sorulur, onaylamazsanız
  hiçbir şey değişmez. Komut satırında soru sorulamıyorsa `--izin-ver` ile onaylanır.
- **Değişmeyen sürümler:** mağazadaki her sürümün işlemesi (commit) ve içerik özeti (SHA-256)
  kayıtlıdır; etiket başka koda taşınmışsa ya da içerik değişmişse paket kurulmaz. Onaylanan
  izinler ve özetler `orhunca.kilit` dosyasına yazılır.
- **Kurulumda kod çalışmaz:** paketlerin kurulum betiği yoktur.

```sh
orhunca paket bilgi istatistik     # sürüm, işleme, içerik özeti ve izinler
```

### Paket yayımlamak

Paketin `.ohcproj` dosyasında `ad`, `sürüm` ve `açıklama` olsun, paket GitHub'da herkese açık
bir depoda dursun:

```sh
git tag v1.0.0 && git push origin v1.0.0
orhunca paket yayımla
```

Orhunca paketi GitHub'dan indirip inceler, mağaza kaydını hazırlar ve kaydı ekleyen çekme isteği
(pull request) sayfasını açar. Otomatik denetim kuralları (ad koruması, değişmeyen sürümler,
izinlerin koddan doğrulanması) geçince paket birkaç dakika içinde yayına girer. Ayrıntılar:
mağaza deposunun BENİOKU dosyası.

## Kullanmak

Kodda `kullan "matematik"` paketin giriş dosyasını, `kullan "matematik/geometri.ohc"` paketteki bir
dosyayı alır. Paketler `paketler/` klasörüne kurulur (`.gitignore`'a eklenir); kesin sürümler
`orhunca.kilit` dosyasında tutulur. Stüdyo'da **Paketler** paneli aynı işlemleri yapar.
