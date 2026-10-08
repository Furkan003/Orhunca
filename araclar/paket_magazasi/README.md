# Orhunca paket mağazası

[Orhunca](https://github.com/Furkan003/Orhunca) paketlerinin dizini. Sunucu yoktur: her paketin
kaydı bu depodaki `paketler/<ad>.json` dosyasıdır; paketlerin kendisi yazarlarının GitHub
depolarındadır. Orhunca ve Orhunca Stüdyo paketleri buradaki `dizin.json` dosyasından bulur.

```sh
orhunca paket ara              # mağazadaki paketler
orhunca paket ekle istatistik  # paketi projeye ekler
```

Stüdyo'da: **Paketler** paneli → paket dizini.

## Güvenlik

- **İzinler koddan çıkarılır.** Orhunca, paketin dosya okuyup yazdığını, internete bağlandığını,
  ortam değişkenlerini (şifreler, anahtarlar) okuduğunu, C kütüphanesi çağırdığını ya da web
  sunucusu başlattığını kodundan kendisi bulur. Paket eklenirken bu izinler gösterilir ve onayınız
  istenir. Bir güncelleme yeni izin isterse yeniden sorulur; onaylamazsanız eski sürüm kalır.
- **Yayımlanan sürüm değişmez.** Her sürümün işlemesi (commit) ve içerik özeti (SHA-256)
  kaydedilir. Yazar etiketi sonradan başka bir koda taşırsa ya da içerik değişirse Orhunca paketi
  kurmaz. Kurulan sürümler projenin `orhunca.kilit` dosyasına yazılır.
- **Ad koruması.** Bir paketi yalnızca onu ilk yayımlayan GitHub kullanıcısı güncelleyebilir;
  mağazadaki bir ada çok benzeyen yeni adlar (`istatistikk` gibi) alınamaz.
- **Kurulumda kod çalışmaz.** Paketlerin kurulum betiği yoktur; paket kodu yalnızca sizin
  programınız çalışınca çalışır.

## Paket yayımlamak

1. Paketiniz GitHub'da herkese açık bir depoda (ya da bir deponun alt klasöründe) olsun ve
   `.ohcproj` dosyasında `ad`, `sürüm` ve `açıklama` yazsın:

   ```
   ad = "renkler"
   sürüm = "1.0.0"
   giriş = "renkler.ohc"
   açıklama = "Renk adları ve renk karışımları"
   ```

2. Sürümü etiketleyip gönderin:

   ```sh
   git tag v1.0.0
   git push origin v1.0.0
   ```

3. Paketin klasöründe:

   ```sh
   orhunca paket yayımla
   ```

   Orhunca paketi GitHub'dan indirip inceler, kaydı hazırlar ve bu depoda kaydı ekleyen bir
   çekme isteği (pull request) sayfası açar. **Propose new file** → **Create pull request** deyin.

4. Denetim kendiliğinden çalışır. Kurallara uyan kayıt birkaç dakika içinde birleştirilir ve
   paket herkesin kullanımına açılır; uymayan kayıtta nedeni çekme isteğine yazılır.

Yeni bir sürüm için `.ohcproj` dosyasındaki sürümü artırın, yeni etiketi gönderin ve
`orhunca paket yayımla` komutunu yineleyin.

## Kurallar

- Paket adı 2-40 karakterdir; harf, rakam, `_` ve `-` içerir.
- Kayıt yalnızca `paketler/<ad>.json` dosyasını ekler ya da değiştirir.
- Yayımlanmış sürümler değiştirilemez ve silinemez; yeni sürüm sona eklenir.
- Kaynak, paket sahibinin GitHub deposu ve bir etikettir (`github:kişi/depo#v1.0.0`).
- Kayıttaki işleme, içerik özeti, ad, sürüm ve izinler paketin kendisiyle aynı olmalıdır.
- Kötü amaçlı paketler (izinlerini kötüye kullanan, kullanıcıyı yanıltan) mağazadan çıkarılır.
  Bildirmek için bir konu (issue) açın.

## Kayıt biçimi

```json
{
  "ad": "renkler",
  "açıklama": "Renk adları ve renk karışımları",
  "sahip": "ayse",
  "sürümler": [
    {
      "sürüm": "1.0.0",
      "kaynak": "github:ayse/orhunca-renkler#v1.0.0",
      "işleme": "3f2a…",
      "özet": "sha256:…",
      "izinler": []
    }
  ]
}
```

`dizin.json` bu kayıtlardan `araclar/dizin.py` ile üretilir; elle düzenlenmez.
