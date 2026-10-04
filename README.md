# Orhunca

Türkçe tabanlı, derlenen bir programlama dili. Hâl ekleri parametrenin rolünü belirler,
fiil sona gelir; derleyici Rust ile yazılmıştır ve **Cranelift** ile doğrudan makine kodu
üretir (zincirde C++ yok).

```
sayılar = [3, 8, 1]
5'i sayılara ekle.
sayılar'ı sırala.
her sayı için sayılardan:
    eğer sayı 4'ten büyükse:
        sayı'yı ekrana yaz.
```

Web uygulamaları da aynı dille yazılır (modeller, yollar, `.ohchtml` görünümleri):

```
model Ürün:
    ad: metin, zorunlu, en_fazla 80
    fiyat: ondalık, en_az 0

al "/ürünler":
    döndür görünüm("ürünler", Ürün.hepsi())

gönder "/ürünler":
    ü = Ürün.formdan(istek)
    eğer değil ü.geçerli_mi() ise:
        döndür görünüm("ürün_formu", ü, ü.hatalar())
    ü'yü kaydet.
    döndür yönlendir("/ürünler")
```

## Kurulum ve kullanım

Gerekenler: Rust (cargo) ve bağlama için bir C derleyicisi (`cc`; Windows hedefi için `x86_64-w64-mingw32-gcc`).

```sh
cargo build --release
./target/release/orhunca çalıştır örnekler/merhaba.ohc
./target/release/orhunca derle örnekler/asal.ohc               # → ./asal
./target/release/orhunca derle örnekler/asal.ohc --hedef windows  # → asal.exe
./target/release/orhunca denetle dosya.ohc                       # yalnızca hata denetimi
./target/release/orhunca yeni dükkan                             # yeni proje (.ohcproj)
./target/release/orhunca yeni dükkan --şablon tam_yigin          # web projesi (şablonlar: web_sitesi, web_api, ...)
./target/release/orhunca stüdyo                                  # geliştirme ortamı (tarayıcıda)
```

Dosya verilmezse geçerli klasördeki `.ohcproj` dosyasının `giriş` dosyası kullanılır.

## Orhunca Stüdyo

```sh
orhunca stüdyo
```

Tarayıcıda Orhunca'nın geliştirme ortamını açar: son projeler, şablon sihirbazı (Konsol Uygulaması,
Sayı Tahmin Oyunu, Kütüphane; Boş Web Sayfası, Web Sitesi, Web Uygulaması, Açılış Sayfası, Web API,
Tam Yığın Uygulama), sözdizimi renklendirmeli düzenleyici (`.ohc`, `.ohchtml`, CSS, JavaScript),
yazarken hata gösterimi, F5 ile derleyip çalıştırma (programın girdisi terminalden verilir),
Linux/Windows için dağıtım derlemesi ve Türkçe anahtar kelime rehberi. İnternet gerekmez; arayüz ve
yazı tipleri ikili dosyanın içindedir.

Web projelerinde F5 sunucuyu başlatır ve sayfa sağdaki **canlı önizlemede** açılır. Kaydettiğinizde
sunucu yeniden derlenir ve önizleme bulunduğu adreste yenilenir (yalnızca `statik/` dosyası
değiştiyse sayfa yenilenir). Stüdyo kapanınca başlattığı sunucular da kapanır.

![Orhunca Stüdyo — canlı önizleme](docs/ekran/canli-onizleme.png)

![Orhunca Stüdyo — düzenleyici](docs/ekran/duzenleyici.png)

| Başlangıç | Yeni proje |
|---|---|
| ![Başlangıç](docs/ekran/baslangic.png) | ![Yeni proje](docs/ekran/yeni-proje.png) |

**Masaüstü uygulaması:** [masaustu/](masaustu) Stüdyo'yu tarayıcısız, kendi penceresinde açan Tauri
uygulamasıdır (`cd masaustu && cargo run --release`; kurulum dosyaları için `cargo tauri build`).

Sunucu yalnızca `127.0.0.1` üzerinden erişilebilir ve her oturumda rastgele bir anahtar üretir;
başka web sitelerinin dosyalarınıza ya da derleyiciye erişmesi engellenir. Tasarım kaynağı:
[docs/tasarim/](docs/tasarim/).

## Paketler

Paketler Git depolarındaki Orhunca kütüphaneleridir (`.ohcproj` + giriş dosyası).

```sh
orhunca paket ekle github:kisi/orhunca-matematik#v1.0   # ya da tam Git adresi / yerel yol
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

## Düzenleyici desteği

- **Biçimlendirici:** `orhunca biçimlendir [dosya.ohc]` girintiyi düzenler ve kesme işaretli eklerde
  ünlü uyumunu düzeltir (`5'a` → `5'e`, `3'den` → `3'ten`, `6'i` → `6'yı`). Sayılar okunuşlarına göre
  ek alır. `--denetle` dosyaları değiştirmeden denetler (CI için).
- **Dil sunucusu (LSP):** `orhunca dil-sunucusu` — hatalar ve yazım uyarıları, üzerine gelince açıklama
  (yerleşik işlevler, değişken tipleri), tamamlama, tanıma gitme, belge simgeleri, biçimlendirme.
  LSP destekleyen her düzenleyiciyle (VS Code, Neovim, Helix, Zed…) çalışır.
- **VS Code eklentisi:** [editors/vscode](editors/vscode) — `.ohc` ve `.ohchtml` için sözdizimi renklendirme ve dil sunucusu istemcisi.
  `cd editors/vscode && npm install && npx @vscode/vsce package` ile `.vsix` oluşturulur.

## Dil rehberi

### Değerler ve tipler

| Tip | Örnek |
|---|---|
| `sayı` (64 bit tamsayı) | `42`, `-7`, `1_000_000` |
| `ondalık` (64 bit kayan nokta) | `3.14`, `-0.5`, `19.99` |
| `metin` | `"Merhaba"` (`\n`, `\t`, `\"`) |
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
işlev topla(a, b):               # parametreler varsayılan olarak sayı
    döndür a + b

işlev selamla(ad: metin) -> metin:
    döndür "Merhaba, " + ad

topla(2, 3)'ü yaz.
```
İşlevler yalnızca kendi parametrelerini ve yerel değişkenlerini görür. Özyineleme desteklenir.

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

### Standart kütüphane

| Alan | İşlevler |
|---|---|
| Dönüşüm | `uzunluk` `metin` `sayı` `ondalık` `yuvarla(x)` `yuvarla(x, 2)` `sayı_mı` `ondalık_mı` |
| Metin | `büyük_harf` `küçük_harf` (Türkçe i/İ, ı/I) `kırp` `parça(m, baş, uzunluk)` `böl` `birleştir` `içerir` `bul` `değiştir` `başlar` `biter` `tekrarla` `harfler` `satırlar` `ters` · `m[i]` · `<` `>` Türk alfabesine göre |
| Liste | `sil(l, sıra)` `içerir` `bul` `parça` `ters` `kopya` `karıştır` `en_büyük` `en_küçük` `toplam` |
| Sözlük | `s[a]` `içerir` `sil` `anahtarlar` `değerler` `uzunluk` |
| Dosya | `dosya_oku` `dosyaya_yaz` `dosyaya_ekle` `dosya_var` `dosya_sil` |
| Matematik | `karekök` `üs` `mutlak` `sinüs` `kosinüs` `tanjant` `logaritma(x)` `logaritma(x, taban)` `rastgele()` `rastgele(a, b)` |
| Zaman ve sistem | `zaman()` `tarih()` `bekle(saniye)` `oku()` `argümanlar()` `ortam(ad)` `çık(kod)` |

- Metin üzerinde `her harf için metinden:` harf harf, sözlük üzerinde anahtar anahtar gezer.
- Programa argüman: `orhunca çalıştır dosya.ohc -- bir iki`
- Tamsayı taşması çalışma hatası verir; çok büyük değerler için ondalık kullanın.
- `ORHUNCA_TOHUM=42` rastgele sayıları tekrarlanabilir yapar.

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

- `istek`: `yöntem`, `yol`, `sorgu` (`istek.sorgu["q"]`), `form`, `parametreler`, `gövde`, `başlıklar`.
- Yanıtlar: `yanıt(404, "...")`, `yanıt(200, metin, "text/plain")`, `yönlendir("/")`,
  `json_yanıtı(değer, 201)`; `y.başlıklar["X-Ad"] = "değer"` ile başlık eklenir.
- `statik/` klasöründeki dosyalar olduğu gibi sunulur. Yol bulunamazsa Türkçe 404 sayfası verilir.
- Bir istek çalışma hatasına yol açarsa tarayıcıya hatanın satırını gösteren 500 sayfası gider;
  sunucu çalışmayı sürdürür.
- Program yol tanımlıyorsa sonunda sunucu kendiliğinden başlar (`sun()`; varsayılan kapı 3000,
  `sun(8080)` ya da `ORHUNCA_KAPI` ile değişir). Yalnızca bu bilgisayardan erişilir;
  `ORHUNCA_ADRES=0.0.0.0` ağa açar.
- Yardımcılar: `kaçır(metin)` (HTML'e güvenli), `para(12.5)` → "12,50", `url_kodla(metin)`.

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

## Mimari

```
dosya.ohc
   ↓
[Sözcük çözümleyici]      src/sozcuk.rs     girinti, kesme işaretli ekler
[Ayrıştırıcı]             src/ayristirici.rs Türkçe cümle yapısı, hâl ekleri (src/ekler.rs)
[Anlam ve tip denetimi]   src/denetci.rs    Türkçe hata mesajları
[Kod üretici]             src/uretici.rs    Cranelift IR → nesne dosyası
   ↓
[Bağlama]  nesne + runtime/orhunca_rt.c → Linux çalıştırılabilir dosyası / Windows .exe
```

Görünümler: `src/sablon.rs` `.ohchtml` dosyalarını parçalara ayırır; ayrıştırıcı onları metin
döndüren işlevlere çevirir. Web yolları da `istek` alıp `Yanıt` döndüren işlevlerdir; ana program
başlarken çalışma zamanına kaydedilir.

Stüdyo: `src/studyo/` (yerel HTTP sunucusu, proje/dosya API'si, program çalıştırıcı, şablonlar —
web şablonlarının dosyaları `src/studyo/sablon_dosyalari/`) ve `studio/` (arayüz: HTML, CSS,
bağımlılıksız JavaScript; `build.rs` ile ikili dosyaya gömülür).

`runtime/orhunca_rt.c`: yazdırma, metin, liste ve sözlük işlemleri, çöp toplayıcı, model kayıtları
(JSON), doğrulama ve tek iş parçacıklı HTTP/1.1 sunucusu (Windows'ta Winsock). Toplayıcı "tutucu" bir
işaretle-süpür toplayıcıdır: yığıttaki ve yazmaçlardaki her sözcüğü olası bir işaretçi sayar, böylece
derleyicinin ürettiği koda ek bir şey gerekmez. Bir web isteğindeki çalışma hatası isteğin başına
geri sarılır (`__builtin_setjmp`), sunucu durmaz.

Testler: `cargo test` (birim testleri, `örnekler/` klasöründeki programları derleyip çalıştıran uçtan
uca testler, gerçek HTTP istekleriyle web çatısı testi `tests/web.rs`, Stüdyo testleri).

Yol haritası ve kararlar için: [PLAN.md](PLAN.md), sohbet özeti: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).
