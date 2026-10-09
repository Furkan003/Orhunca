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

## Resmi paketler

Hepsi Orhunca ile yazılmıştır, dışa bağımlılığı yoktur ve aynı programda birlikte kullanılabilir.
Ayrıntılı kullanım her paketin giriş dosyasının başındaki yorumdadır; sınamalar `*_sına.ohc`
dosyalarındadır (`orhunca sına kütüphaneler`).

| Paket | Ne işe yarar | Başlıca işlevler |
|---|---|---|
| `karekod` | QR kod: SVG, terminal, `<img>` adresi | `karekod_svg`, `karekod_svg_ayarlı`, `karekod_resim_adresi`, `karekod_metin`, `karekod_matrisi`, `wifi_içeriği`, `kartvizit_içeriği`, `e_posta_içeriği`, `sms_içeriği`, `telefon_içeriği`, `konum_içeriği` |
| `barkod` | Code 128, EAN-13, EAN-8 barkodları (SVG) | `code128_svg`, `ean13_svg`, `ean8_svg`, `ean13_tamamla`, `ean_geçerli_mi` |
| `özet` | Karma ve kodlama; giriş sistemleri için şifre saklama | `sha256`, `hmac_sha256`, `base64_kodla`, `base64_çöz`, `base64_adres_kodla`, `crc32`, `şifre_karması`, `şifre_doğru_mu` |
| `giriş` | Üyelik sistemi (özet paketini kullanır) | `kayıt_ol`, `giriş_yap`, `çıkış_yap`, `giriş_yapıldı_mı`, `oturumdaki_kullanıcı`, `rolü_var_mı`, `şifre_değiştir`, `sıfırlama_anahtarı_üret`, `şifreyi_sıfırla`, `form_anahtarı`, `form_anahtarı_doğru_mu` |
| `doğrula_tr` | Türkiye'ye özgü form doğrulamaları | `tc_kimlik_geçerli_mi`, `vergi_no_geçerli_mi`, `iban_geçerli_mi`, `iban_biçimle`, `telefon_geçerli_mi`, `telefon_biçimle`, `plaka_geçerli_mi`, `posta_kodu_geçerli_mi`, `e_posta_geçerli_mi`, `url_geçerli_mi`, `kart_no_geçerli_mi`, `kart_türü`, `kart_gizle`, `şifre_eksikleri`, `şifre_gücü_yazısı` |
| `insancıl` | Sayıları ve zamanı insanların okuyacağı biçime çevirir | `sayı_yazıyla`, `para_yazıyla`, `sıra_sayısı`, `ek_ekle`, `çoğul`, `sayılı`, `liste_yazısı`, `dosya_boyutu`, `kısa_sayı`, `süre_yazısı`, `süre_önce`, `göreli_zaman`, `roma_rakamı` |
| `metin_araçları` | Metin işleme | `slug`, `ascii_yap`, `kısalt`, `başlık_biçimi`, `levenshtein`, `benzerlik`, `en_benzer`, `maskele`, `e_posta_maskele`, `satırlara_böl`, `okuma_süresi`, `html_temizle` |
| `markdown` | Markdown → güvenli HTML (blog, yorum, belge) | `markdown_html`, `markdown_düz_metin` |
| `sayfala` | Web listelerinde sayfalama | `sayfa_sayısı`, `sayfa_oku`, `sayfa_başı`, `sayfa_bilgisi`, `sayfa_numaraları`, `sayfalama_html` |
| `kimlik` | Benzersiz kimlikler | `uuid`, `uuid_geçerli_mi`, `sıralı_kimlik` (ULID), `kısa_kimlik`, `okunur_kod`, `sayı_kodu`, `koddan_sayı` |
| `sahte_veri` | Deneme ve sunum için Türkçe sahte veri | `sahte_tam_ad`, `sahte_e_posta`, `sahte_telefon`, `sahte_adres`, `sahte_şehir`, `sahte_şirket`, `sahte_geçmiş_tarih`, `sahte_doğum_tarihi`, `sahte_tc_kimlik`, `sahte_cümle`, `sahte_paragraf` |
| `günlük` | Uygulama günlüğü (log): düzeyler, dosya, JSON | `günlük_bilgi`, `günlük_uyarı`, `günlük_hata`, `günlük_ayrıntı`, `günlük_son` |
| `grafik_svg` | Web sayfası ve rapor için SVG grafikler | `çubuk_grafiği`, `çizgi_grafiği`, `halka_grafiği`, `grafik_tablosu` |
| `istatistik` | Ortalama, ortanca, standart sapma | `ortalama`, `ortanca`, `standart_sapma` … |
| `geometri` | Alan, çevre, hacim hesapları | `daire_alanı` … |

```
kullan "karekod"
kullan "özet"
kullan "grafik_svg"

karekod_svg("https://orhunca.dev")'yi "kod.svg"'ye yaz.          # QR kodu dosyaya
kayıt = şifre_karması("gizli123")                                  # veritabanına bu yazılır
şifre_doğru_mu("gizli123", kayıt)'nu yaz.                          # doğru
çubuk_grafiği("Satış", ["Oca", "Şub"], [120.0, 95.5])'yi "grafik.svg"'ye yaz.
```

Notlar:

- `özet` paketinin `şifre_karması` işlevi PBKDF2-SHA256 (20.000 tur, tahmin edilemez tuz) kullanır;
  `kimlik` paketinin kimlikleri de `güvenli_anahtar` ile üretilir. `sahte_veri` yalnızca deneme içindir.
- `giriş` paketi `Kullanıcı` modelini tanımlar (ad, e_posta, rol …). İlk kayıt olan kullanıcının rolü
  `yönetici`dir. Üst üste 5 hatalı denemede hesap 15 dakika kilitlenir; girişte oturum kimliği
  yenilenir. Şifre sıfırlama anahtarı tek kullanımlıktır ve 1 saat geçerlidir; e-postayla göndermek
  uygulamaya kalır.
- `günlük` ayarları ortam değişkenleriyle yapılır: `GUNLUK_DOSYASI`, `GUNLUK_DUZEYI`
  (`ayrıntı`, `bilgi`, `uyarı`, `hata`), `GUNLUK_BICIMI=json`, `GUNLUK_EKRAN=hayır`.
- `grafik_svg` renkleri renk körlüğüne uygun sabit sırayla verilir ve koyu temaya uyar; her grafik
  biçemini kendisi taşır (dosyaya kaydedilen `.svg` de renkli görünür). Ekran okuyucular için
  `grafik_tablosu` aynı veriyi tablo olarak verir.

## Kullanmak

Kodda `kullan "matematik"` paketin giriş dosyasını, `kullan "matematik/geometri.ohc"` paketteki bir
dosyayı alır. Paketler `paketler/` klasörüne kurulur (`.gitignore`'a eklenir); kesin sürümler
`orhunca.kilit` dosyasında tutulur. Stüdyo'da **Paketler** paneli aynı işlemleri yapar.
