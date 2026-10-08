# Orhunca dil rehberi

Orhunca'nın söz dizimi ve standart kütüphanesi. Adım adım öğrenmek için [dersler](../dersler) daha uygundur.

[← README](../README.md)


### Değerler ve tipler

| Tip | Örnek |
|---|---|
| `sayı` (64 bit tamsayı) | `42`, `-7`, `1_000_000` |
| `ondalık` (64 bit kayan nokta) | `3.14`, `-0.5`, `19.99` |
| `metin` | `"Merhaba"` (`\n`, `\t`, `\"`, `\\`; başka kaçışlar olduğu gibi kalır: `"\d+"`) |
| `mantık` | `doğru`, `yanlış` |
| `liste<T>` | `[3, 8, 1]`, `["a", "b"]`, `[]` |
| `sözlük<A, D>` | `{"elma": 5, "armut": 3}`, `{}` (anahtarlar sayı ya da metin) |

Tipler çıkarılır; bir değişkenin tipi sonradan değişemez. Sayılar gerektiğinde kendiliğinden
ondalığa çevrilir (`2 * 1.5`, ondalık parametreye `3` vermek), tersi yapılmaz: ondalık
biriktirecek bir değişkeni `toplam = 0.0` diye başlatın.

### Hâl ekleri

| Hâl | Ekler | Rolü |
|---|---|---|
| Belirtme | `-(y)ı/i/u/ü` | işlem yapılan nesne: `5'i`, `sayıları` |
| Yönelme | `-(y)a/e` | hedef: `sayılara`, `10'a` |
| Ayrılma | `-dan/den/tan/ten` | kaynak, karşılaştırma: `sayılardan`, `4'ten` |
| İlgi | `-(n)ın/in/un/ün` | `sayıların uzunluğu` |

- Sayılarda, metinlerde ve parantezli ifadelerde **kesme işareti zorunludur**: `5'i`, `"Merhaba"'yı`, `(a + b)'yi`.
- Değişkenlerde isteğe bağlıdır: `sayılara` = `sayılar'a`. Derleyici programdaki tanımlı isimleri bilir; en uzun ismi bulup kalanı ek tablosunda arar.
- Ünsüz yumuşaması tanınır: `kitap` → `kitabı`, `renk` → `rengi`, `çocuk` → `çocuğa`.
- Ünlü uyumu hoşgörüyle karşılanır (`5'a` de kabul edilir).
- Birden fazla okuma mümkünse derleyici tahmin yürütmez, hata verir (`kitapla`: değişken mi, `kitap'la` mı?).
- Ek, ifadenin tamamına aittir: `a + b'yi yaz.` → `(a + b)` yazılır.
- Parametre sırası önemsizdir: `5'i sayılara ekle.` = `sayılara 5'i ekle.`

### Cümleler (fiiller)

```
x'i ekrana yaz.          # "ekrana" isteğe bağlı: x'i yaz.
metni "notlar.txt"'ye yaz.   # yönelme hâlinde bir dosya yolu verilirse dosyaya yazar
5'i sayılara ekle.
5'i sayılardan çıkar.    # ilk eşleşeni listeden siler
sayıları sırala.         # metinler Türk alfabesine göre sıralanır (ç, ğ, ı, ö, ş, ü)
```

### Atama ve işlemler

```
x = 10
x += 1
l[0] = 99
"Toplam: " + x'i yaz.    # metin + sayı otomatik metne çevrilir
isimler: liste<metin> = []    # tipi yazılmış tanım (boş liste ve sözlükler için)
```
- `/` her zaman ondalık sonuç verir: `7 / 2` → `3.5`
- `//` tam bölmedir, aşağı yuvarlar: `7 // 2` → `3`, `-7 // 2` → `-4`
- `%` kalanın işareti bölenle aynıdır: `-7 % 3` → `2`
Aritmetik: `+ - * / // %` · Karşılaştırma: `== != < > <= >=` · Mantık: `ve`, `veya`, `değil`

### Koşullar

```
eğer x 4'ten büyükse:
    ...
değilse eğer x 4'e eşitse:
    ...
değilse:
    ...
```
Türkçe karşılaştırmalar: `x 4'ten büyükse`, `x 4'ten küçükse`, `x 4'e eşitse`, `x 4'e eşit değilse`,
`x 4'ten büyük veya eşitse`. Simgelerle de yazılabilir: `eğer x > 4 ve y < 2 ise:` (`ise` isteğe bağlı).

### Döngüler

```
her i için 1'den 10'a kadar:     # 1 ve 10 dahil
    ...
her sayı için sayılardan:
    ...
x 10'dan küçük olduğu sürece:
    ...
x 10'dan küçükken:
    ...
```
`dur` döngüden çıkar, `sürdür` sonraki adıma geçer.

### İşlevler

```
işlev topla(a, b):
    döndür a + b

işlev selamla(ad: metin) -> metin:
    döndür "Merhaba, " + ad

topla(2, 3)'ü yaz.
```
İşlevler yalnızca kendi parametrelerini ve yerel değişkenlerini görür. Özyineleme desteklenir.
Tipi yazılmayan parametre, işlevin ilk çağrısındaki değerin tipini alır (`selamla2(ad)` →
`selamla2("Ali")` ile metin); hiç çağrılmayan işlevde sayıdır. Sonraki çağrılar aynı tipte
olmalıdır; farklı tipler (ör. hem sayı hem ondalık) için tipi tanımda yazın.

### Kendi fiilleriniz

Parametreler hâl ekleriyle tanımlanır, son kelime fiilin adıdır. Çağrıda sıra önemsizdir;
hangi değerin hangi parametreye gideceğini ekler belirler.

```
fiil sayı'yı karele:
    döndür sayı * sayı

fiil (kişi: metin)'yi selamla:
    "Merhaba, " + kişi'yi yaz.

fiil (miktar: ondalık)'ı (bakiye: ondalık)'dan düş -> ondalık:
    döndür bakiye - miktar

"Ayşe"'yi selamla.                    # cümle olarak
kare = 7'yi karele                    # değer döndüren fiil
bakiye = 30'u bakiye'den düş
bakiye = bakiye'den 25.5'i düş        # aynı çağrı, farklı sıra
eğer 5'i karele 20'den büyükse:       # koşulda
    ...
```
- Tipi yazılmayan parametreler `sayı`dır; başka tip için `(ad: metin)'i` yazın.
- Her parametre farklı bir hâl eki almalıdır (belirtme, yönelme, ayrılma, bulunma, vasıta).
- Fiiller parantezle de çağrılabilir: `karele(4)`.

### Sözlükler

```
stok = {"elma": 12, "armut": 5}
stok["kiraz"] = 30
stok["elma"] = stok["elma"] - 2
eğer içerir(stok, "armut") ise:
    sil(stok, "armut")
her meyve için stoktan:          # anahtarlar ekleme sırasıyla gezilir
    meyve + ": " + stok[meyve]'yi yaz.
```

### Birden fazla dosya ve sabitler

```
kullan "araçlar/fiyat.ohc"       # yol, bu dosyanın klasörüne göredir
sabit KDV_ORANI = 0.20           # her yerden (işlevlerden de) görülür
```
`kullan` ile eklenen dosyalarda yalnızca işlev, fiil ve sabit tanımları olabilir. `pi` hazır bir sabittir.

### C kütüphaneleri

Bilgisayarda kurulu bir C kütüphanesinin işlevleri, imzaları `kütüphane` bloğuna yazılarak
Orhunca işlevi gibi çağrılır (gövde yazılmaz):

```
kütüphane "m":
    işlev pow(taban: ondalık, üs: ondalık) -> ondalık

kütüphane "c":
    işlev strlen(m: metin) -> sayı
    işlev abs(x: sayı32) -> sayı32
    işlev getenv(ad: metin) -> metin

pow(2.0, 10.0)'u yaz.          # 1024.0
strlen("Orhunca")'yı yaz.      # 7
```

| Orhunca | C |
|---|---|
| `sayı` | `int64_t` (`long long`) |
| `sayı32` | `int` |
| `ondalık` | `double` |
| `mantık` | `bool` (C'de `int` döndüren "doğru mu" işlevleri için `sayı32` kullanın) |
| `metin` | `const char *` (UTF-8) |
| dönüş yazılmazsa | `void` |

- Kütüphane adı kısa yazılır: `"m"` Linux'ta `libm.so`, macOS'ta `libm.dylib`, Windows'ta `m.dll`
  olarak aranır (çalışılan klasörde de). `"c"` ve `"m"` Windows'ta `msvcrt.dll`'dir. Tam dosya
  adı da yazılabilir: `kütüphane "libcurl.so.4":`.
- Kütüphane ilk çağrıda yüklenir; bulunamazsa ya da işlev yoksa çalışma hatası verilir.
- Yalnızca bilgisayarda çalışan programlarda kullanılır (web hedefinde ve arayüz programlarında
  olmaz). Python karşılığında `ctypes` ile çağrılır.
- İşaretçi, yapı (struct) ve geri çağırma (callback) isteyen işlevler şimdilik çağrılamaz.

### Standart kütüphane

Her işlevin kullanımı ve açıklaması: [yerleşik işlevler başvurusu](basvuru.md) (terminalde `orhunca başvuru <kelime>`).

| Alan | İşlevler |
|---|---|
| Dönüşüm | `uzunluk` `metin` `sayı` `ondalık` `yuvarla(x)` `yuvarla(x, 2)` `sayı_mı` `ondalık_mı` |
| Metin | `büyük_harf` `küçük_harf` (Türkçe i/İ, ı/I) `kırp` `parça(m, baş, uzunluk)` `böl` `birleştir` `içerir` `bul` `değiştir` `başlar` `biter` `tekrarla` `harfler` `satırlar` `ters` `kod` `karakter` `kodlar` `kodlardan` · `m[i]` · `<` `>` Türk alfabesine göre |
| Liste | `sil(l, sıra)` `içerir` `bul` `parça` `ters` `kopya` `karıştır` `en_büyük` `en_küçük` `toplam` |
| Sözlük | `s[a]` `içerir` `sil` `anahtarlar` `değerler` `uzunluk` |
| Dosya | `dosya_oku` `dosyaya_yaz` `dosyaya_ekle` `dosya_var` `dosya_sil` |
| Matematik | `karekök` `üs` `mutlak` `sinüs` `kosinüs` `tanjant` `logaritma(x)` `logaritma(x, taban)` `rastgele()` `rastgele(a, b)` |
| Zaman ve sistem | `zaman()` `tarih()` `bekle(saniye)` `oku()` `argümanlar()` `ortam(ad)` `çık(kod)` `hata_ver(mesaj)` |
| Tarih | `bugün()` `saat()` `gün_ekle(t, n)` `gün_farkı(t1, t2)` `haftanın_günü(t)` `tarih_yazısı(t)` — tarihler `"2026-10-04"` ya da `"04.10.2026"` |
| Desenler | `eşleşir(m, desen)` (metnin tamamı) `desen_bul` `eşleşmeler` `desen_değiştir` `desen_böl` — `. \d \w \s [a-z] [^0-9] * + ? {n,m} ( \| ) ^ $ \b`; `\w` Türkçe harfleri tanır |
| CSV ve JSON | `csv_oku(metin)` → `liste<liste<metin>>` (ayraç `,` `;` ya da sekme) `csv_yaz(tablo)` · `json_al(json, "öğrenciler.0.ad")` |
| İnternet | `http_al(adres)` `http_gönder(adres, gövde)` — hata durumunda (bağlantı yok, HTTP 404 …) çalışma hatası; `dene:` ile yakalanır |

- Metin üzerinde `her harf için metinden:` harf harf, sözlük üzerinde anahtar anahtar gezer.
- Programa argüman: `orhunca çalıştır dosya.ohc -- bir iki`
- Tamsayı taşması çalışma hatası verir; çok büyük değerler için ondalık kullanın.
- `ORHUNCA_TOHUM=42` rastgele sayıları tekrarlanabilir yapar.
- **`.env` dosyası:** program başlarken çalıştığı klasördeki `.env` dosyasını okur; `AD=değer`
  satırları `ortam("AD")` ile alınır (`#` yorum, `export`, tırnaklı değer desteklenir). Bilgisayarda
  zaten tanımlı değişkenler önceliklidir. API anahtarı ve şifreleri koda değil buraya yazın: Stüdyo'nun
  yeni projelerdeki `.gitignore` dosyası `.env`'yi Git'e almaz, web sunucusu noktayla başlayan
  dosyaları dışarı vermez, masaüstü/telefon paketlerine de girmez. `ORHUNCA_KAPI` gibi ayarlar da
  `.env`'ye yazılabilir.

### Bellek

Bellek otomatik yönetilir: artık kullanılmayan metin ve listeler çöp toplayıcı tarafından geri
verilir. `ORHUNCA_BELLEK_RAPORU=1` ortam değişkeniyle program sonunda kaç kez toplama yapıldığı ve
en yüksek canlı bellek yazdırılır.

### Modeller

Alanları, varsayılan değerleri ve kuralları olan veri tipleri. Her modelin bir `kimlik` alanı vardır
(kaydedilince verilir).

```
model Kitap:
    ad: metin, zorunlu, en_fazla 60
    sayfa: sayı = 100, en_az 1
    fiyat: ondalık, en_az 0
    e_posta: metin, e_posta, etiket "E-posta"
    etiketler: liste<metin>

k = Kitap(ad: "Nutuk", sayfa: 600, fiyat: 150)   # verilmeyen alanlar varsayılanını alır
k.ad'ı yaz.
k.sayfa += 4
k.fiyatı yaz.                 # alan adına da ek gelebilir
eğer değil k.geçerli_mi() ise:
    k.hatalar()'ı yaz.        # ["Ad boş bırakılamaz", "Fiyat en az 0 olmalı"]
```

Nitelikler: `zorunlu`, `en_az N`, `en_fazla N` (metinde karakter, listede öğe sayısı), `e_posta`
(biçim denetimi), `etiket "Görünen ad"` (hata mesajlarında). Hata mesajlarında alan adındaki `_`
boşluk olur: `doğum_tarihi` → "Doğum tarihi".

Bir alan başka bir model, model listesi ya da sözlüğü olabilir:

```
model Adres:
    şehir: metin, zorunlu

model Müşteri:
    ad: metin
    adres: Adres                 # başta varsayılan bir Adres nesnesi
    eski_adresler: liste<Adres>

m = Müşteri(ad: "Ayşe")
m.adres.şehir = "Kars"
m'yi kaydet.                     # iç nesneler kaydın içine gömülü saklanır
m.hatalar()'ı yaz.               # iç modelin hataları da: "Adres: Şehir boş bırakılamaz"

model Düğüm:
    değer: sayı
    sonraki: Düğüm               # kendi modeline dönen alan başta boştur
eğer boş_mu(d.sonraki) ise: ...
```

**Model işlevleri.** Modelin içinde tanımlanan işlevler o modelin nesneleriyle çağrılır;
nesnenin kendisi `bu`dur:

```
model Hesap:
    sahip: metin
    bakiye: ondalık

    işlev yatır(miktar: ondalık):
        bu.bakiye += miktar

    işlev özet() -> metin:
        döndür bu.sahip + ": " + para(bu.bakiye) + " TL"

h = Hesap(sahip: "Ayşe")
h.yatır(150)
h.özet()'i yaz.                  # Ayşe: 150,00 TL
```
- Alanlar `bu.alan` ile okunur ve değiştirilir; değişiklik nesnenin kendisinde olur.
- Model işlevleri birbirini (`bu.özet()`) ve başka modellerin işlevlerini çağırabilir.
- Alanla aynı adı ve yerleşik yöntem adlarını (`kaydet`, `sil`, `geçerli_mi`, `hatalar`, `json`,
  `hepsi`, `bul`, `var_mı`, `formdan`) alamaz.
- Python/JavaScript çevirisinde sınıfın yöntemleri olur (`bu` → `self` / `this`).

### Seçenekler (numaralandırma)

```
seçenek Renk: kırmızı, yeşil, mavi

seçenek Gün:
    pazartesi, salı, çarşamba
    perşembe, cuma

r = Renk.yeşil
eğer r == Renk.kırmızı ise:
    "dur"'u yaz.
Renk.hepsi()'ni yaz.            # ["kırmızı", "yeşil", "mavi"]
seçilen = Renk("mavi")          # metinden; geçersizse çalışma hatası

model Araba:
    renk: Renk                  # varsayılan: ilk değer
```
Seçenek değerleri yalnızca aynı türden değerlerle karşılaştırılabilir; metne eklenebilir, yazılabilir,
sözlük anahtarı ve liste öğesi olabilir. Kayıtlarda ve JSON'da adıyla saklanır; formdan gelen
geçersiz değer `hatalar()` listesine girer. Arayüzde: `seçim(renk, Renk.hepsi())`.

### Kalıcı kayıtlar

Modeller `veri/<Model>.json` dosyasına kaydedilir (okunabilir JSON; `ORHUNCA_VERI` ile klasör değişir):

```
k'yı kaydet.                  # yeni kayda sıradaki kimlik verilir; var olan kayıt güncellenir
Kitap.hepsi()                 # liste<Kitap>
Kitap.bul(3)                  # yoksa kimliği 0 olan yeni bir nesne
Kitap.var_mı(3)  ·  Kitap.sil(3)  ·  k'yı sil.
k.json()  ·  json(değer)      # JSON metni
```

### Web sunucusu

```
al "/":                                   # GET
    döndür "<h1>Merhaba</h1>"             # metin → HTML yanıtı

al "/ürünler/{kimlik: sayı}":             # yol parametresi yerel değişken olur
    döndür Ürün.bul(kimlik)               # model, liste, sözlük → JSON yanıtı

gönder "/ürünler":                        # POST (koy: PUT, sil: DELETE)
    ü = Ürün.formdan(istek)               # form ya da JSON gövdesi alanlara bağlanır
    ...
    döndür yönlendir("/ürünler")
```

- `istek`: `yöntem`, `yol`, `sorgu` (`istek.sorgu["q"]`), `form`, `parametreler`, `gövde`, `başlıklar`,
  `çerezler`, `oturum`, `dosyalar`.
- Yanıtlar: `yanıt(404, "...")`, `yanıt(200, metin, "text/plain")`, `yönlendir("/")`,
  `json_yanıtı(değer, 201)`; `y.başlıklar["X-Ad"] = "değer"` ile başlık eklenir.
- `statik/` klasöründeki dosyalar olduğu gibi sunulur. Yol bulunamazsa Türkçe 404 sayfası verilir.
- Bir istek çalışma hatasına yol açarsa tarayıcıya hatanın satırını gösteren 500 sayfası gider;
  sunucu çalışmayı sürdürür.
- Program yol tanımlıyorsa sonunda sunucu kendiliğinden başlar (`sun()`; varsayılan kapı 3000,
  `sun(8080)` ya da `ORHUNCA_KAPI` ile değişir). Yalnızca bu bilgisayardan erişilir;
  `ORHUNCA_ADRES=0.0.0.0` ağa açar.
- Yardımcılar: `kaçır(metin)` (HTML'e güvenli), `para(12.5)` → "12,50", `url_kodla(metin)`.

**Oturum ve çerezler.** `istek.oturum` her tarayıcıya ayrı, sunucuda saklanan bir
`sözlük<metin, metin>`dir; yazılan değerler sonraki isteklerde gelir (oturum çerezi `HttpOnly`,
`SameSite=Lax`; HTTPS'te `Secure`). Boşaltılınca oturum silinir. Gelen çerezler `istek.çerezler`de,
gönderilecekler `y.çerezler["tema"] = "koyu"` ile (boş değer çerezi siler).

```
gönder "/giriş":
    eğer istek.form["şifre"] == "gizli" ise:
        istek.oturum["kullanıcı"] = istek.form["ad"]
    döndür yönlendir("/")

al "/çıkış":
    istek.oturum = {}
    döndür yönlendir("/")
```

**Dosya yükleme.** `<form enctype="multipart/form-data">` ile gönderilen dosyalar
`veri/yüklemeler/` klasörüne kaydedilir; `istek.dosyalar["alan"]` bir `YüklenenDosya`dır
(`ad`, `tür`, `yol`, `boyut`), metin alanları `istek.form`a girer. `dosya_taşı(d.yol, "statik/...")`
ile kalıcı bir yere taşınabilir. Gövde sınırı 32 MB'tır (`ORHUNCA_EN_BUYUK_GOVDE`).

**Eşzamanlılık.** Sunucu tek iş parçacığında bir olay döngüsüyle çalışır: bütün bağlantılar
birlikte okunup yazılır, yavaş bir istemci ötekileri bekletmez, HTTP/1.1 bağlantıları açık kalır
(parçalı gövdeler de desteklenir). Yollar sırayla çalışır; bu sayede kayıtlar ve bellek güvendedir.

**HTTPS.** `ORHUNCA_SERTIFIKA=sertifika.pem ORHUNCA_ANAHTAR=anahtar.pem` verilirse sunucu HTTPS
konuşur. Sistemdeki OpenSSL (1.1 ya da 3) çalışırken yüklenir; derlemede ek bir şey gerekmez.

**Yayınlamak.** `orhunca yayınla kullanıcı@sunucu --alan ornek.com` programı bir Linux sunucusuna
(VPS) kurar, hizmet olarak başlatır ve HTTPS sertifikasını kendiliğinden alır. Paylaşımlı
hostingde (cPanel) `orhunca yayınla --cgi` ile program CGI olarak çalışır; `--kaynakla` ile `.ohc`
dosyaları PHP gibi çalışır. Yalnızca PHP ve MySQL sunan barındırmalarda `orhunca yayınla --php`
programı PHP'ye çevirir. Bkz. [sunucuya yayınlamak](yayinlama.md).

### Görünümler (.ohchtml)

`görünümler/` klasöründeki dosyalar derlemeye katılır ve `görünüm("ad", değer)` ile kullanılır:

```
@model liste<Ürün>
@düzen "düzen"
@başlık "Ürünler"
<h1>@uzunluk(model) ürün</h1>
@eğer uzunluk(model) 0'a eşitse {
    <p>Henüz ürün yok.</p>
} @değilse {
    <ul>
    @her ü için model'den {
        <li><a href="/ürünler/@ü.kimlik">@ü.ad</a> @para(ü.fiyat) TL</li>
    }
    </ul>
}
```

- `@ifade` ve `@(ifade)` değeri HTML'e kaçırarak yazar; `@ham(ifade)` kaçırmaz.
- `@model (ürün: Ürün, hatalar: liste<metin>)` birden çok değer alır: `görünüm("form", ü, [])`.
- `@düzen "düzen"`: sayfa `görünümler/düzen.ohchtml` içine, onun `@içerik` yazdığı yere yerleşir;
  `@başlık` düzene `başlık` olarak geçer.
- Parça görünümler: `@görünüm("parçalar/kart", ü)`. Yorum: `@* ... *@`, `@` için `@@`.
- Görünümdeki hatalar görünüm dosyasının satırını gösterir.

**Bootstrap.** Düzenin `<head>` bölümüne `@bootstrap` yazılınca [Bootstrap 5](https://getbootstrap.com)
sayfaya eklenir. Sınıflar İngilizce (`class="btn btn-primary"`) ya da Türkçe yazılır: `sınıf="..."`
içindeki Türkçe adlar çevrilir, bilinmeyenler (kendi sınıflarınız) olduğu gibi kalır.

```
<nav sınıf="gezinme arka-koyu gezinme-koyu"><a sınıf="gezinme-marka" href="/">Dükkan</a></nav>
<div sınıf="kapsayıcı"><div sınıf="satır"><div sınıf="sütun-orta-4">
  <div sınıf="kart gölge-küçük"><div sınıf="kart-gövde">
    <h5 sınıf="kart-başlık">@ü.ad</h5>
    <a sınıf="düğme düğme-birincil" href="/sepet">Sepete ekle</a>
  </div></div>
</div></div></div>
```

| Türkçe | Bootstrap |
|---|---|
| `düğme`, `düğme-birincil`, `düğme-çerçeve-tehlike`, `düğme-büyük`/`düğme-küçük` | `btn`, `btn-primary`, `btn-outline-danger`, `btn-lg`/`btn-sm` |
| renkler: `birincil` `ikincil` `başarı` `tehlike` `dikkat` `bilgi` `açık` `koyu` | `primary` `secondary` `success` `danger` `warning` `info` `light` `dark` |
| `kapsayıcı`, `satır`, `sütun-orta-6` (`küçük`/`orta`/`büyük`/`geniş` = sm/md/lg/xl) | `container`, `row`, `col-md-6` |
| `kart`, `kart-gövde`, `kart-başlık`, `kart-metin`, `kart-üst`, `kart-alt` | `card`, `card-body`, `card-title`, `card-text`, `card-header`, `card-footer` |
| `uyarı uyarı-başarı`, `rozet`, `tablo tablo-çizgili`, `form-denetim`, `form-etiket` | `alert alert-success`, `badge`, `table table-striped`, `form-control`, `form-label` |
| `gezinme`, `gezinme-marka`, `menü`, `menü-bağlantı`, `arka-koyu`, `metin-orta`, `gölge`, `esnek`, `gizli` | `navbar`, `navbar-brand`, `nav`, `nav-link`, `bg-dark`, `text-center`, `shadow`, `d-flex`, `d-none` |

Boşluk ve benzeri yardımcı sınıflar İngilizce yazılır (`mt-3`, `p-2`). Bootstrap internetten
(jsDelivr) bütünlük özetiyle yüklenir; internetsiz çalışacak sayfalarda `statik/` klasörüne
indirip `<link>` ile ekleyin.

### Hata mesajları

```
hata: karşılaştırılan değer ayrılma hâlinde (-den) olmalı
  --> t.ohc:2:8
  |
2 | eğer x 4'e büyükse:
  |        ^
ipucu: x 4'ten büyükse
```
Çalışma hataları da Türkçedir ve satır numarası verir (sıfıra bölme, liste sınırı...).

### Hata yakalama

```
dene:
    yaş = sayı(girdi)
    eğer yaş < 0 ise:
        hata_ver("yaş eksi olamaz")     # kendi hatanız
yakala hata:
    ("Geçersiz giriş: " + hata)'yı yaz.  # hata: mesaj (metin)
```
`dene:` bloğunda bir çalışma hatası (dönüştürme, sıfıra bölme, liste sınırı, dosya, `hata_ver`...)
olursa blok orada bırakılır ve `yakala` bloğu çalışır. Hatadan önce yapılan atamalar kalır.
Değişken adı isteğe bağlıdır (`yakala:`). `dene` blokları iç içe yazılabilir; blokta `döndür`,
`dur` ve `sürdür` kullanılabilir. Web yolunda yakalanmayan hata yine 500 sayfası verir.

### Sınamalar

Programın doğru çalıştığını denetleyen küçük programlar. Adı `_sına.ohc` ile biten dosyalara (ya da
`sınamalar/` klasörüne) yazılır; adı `sına_` ile başlayan parametresiz her işlev bir sınamadır:

```
# hesap_sına.ohc
kullan "hesap.ohc"

işlev sına_toplama():
    eşit_olmalı(topla(2, 3), 5)
    eşit_olmalı(büyük_harf("ıi"), "Iİ")

işlev sına_ortalama():
    doğrula(ortalama([1, 2, 3]) == 2.0)
    doğrula(uzunluk(notlar) > 0, "not listesi boş olmamalı")
```

- `eşit_olmalı(gerçek, beklenen)`: değerler farklıysa ikisini de gösterir
  (`hesap_sına.ohc:5: beklenen 5, bulunan 4`). Her tiple çalışır: sayı, metin, liste...
- `doğrula(koşul)` · `doğrula(koşul, açıklama)`: koşul yanlışsa satırı gösterir.

`orhunca sına` projedeki bütün sınamaları çalıştırır; her sınama ayrı denenir, birinin hatası
ötekileri durdurmaz. `orhunca sına hesap_sına.ohc --ad toplama` yalnızca adında "toplama" geçenleri,
`--json` sonucu makinenin okuyacağı biçimde verir. Bir sınama kalırsa çıkış kodu 1'dir (CI için).
Stüdyo'da soldaki **Sınamalar** paneli sınamaları listeler; hepsini, bir dosyayı ya da tek bir
sınamayı çalıştırır ve kalanların mesajını gösterir. `doğrula` ve `eşit_olmalı` sınama dosyası
dışında da kullanılabilir.
