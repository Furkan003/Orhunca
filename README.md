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

## Kurulum ve kullanım

Gerekenler: Rust (cargo) ve bağlama için bir C derleyicisi (`cc`; Windows hedefi için `x86_64-w64-mingw32-gcc`).

```sh
cargo build --release
./target/release/orhunca çalıştır örnekler/merhaba.ohc
./target/release/orhunca derle örnekler/asal.ohc               # → ./asal
./target/release/orhunca derle örnekler/asal.ohc --hedef windows  # → asal.exe
./target/release/orhunca denetle dosya.ohc                       # yalnızca hata denetimi
./target/release/orhunca yeni dükkan                             # yeni proje (.ohcproj)
./target/release/orhunca stüdyo                                  # geliştirme ortamı (tarayıcıda)
```

Dosya verilmezse geçerli klasördeki `.ohcproj` dosyasının `giriş` dosyası kullanılır.

## Orhunca Stüdyo

```sh
orhunca stüdyo
```

Tarayıcıda Orhunca'nın geliştirme ortamını açar: son projeler, şablon sihirbazı (Konsol Uygulaması,
Sayı Tahmin Oyunu, Kütüphane), sözdizimi renklendirmeli düzenleyici, yazarken hata gösterimi, F5 ile
derleyip çalıştırma (programın girdisi terminalden verilir), Linux/Windows için dağıtım derlemesi ve
Türkçe anahtar kelime rehberi. İnternet gerekmez; arayüz ve yazı tipleri ikili dosyanın içindedir.

![Orhunca Stüdyo — düzenleyici](docs/ekran/duzenleyici.png)

| Başlangıç | Yeni proje |
|---|---|
| ![Başlangıç](docs/ekran/baslangic.png) | ![Yeni proje](docs/ekran/yeni-proje.png) |

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
- **VS Code eklentisi:** [editors/vscode](editors/vscode) — sözdizimi renklendirme ve dil sunucusu istemcisi.
  `cd editors/vscode && npm install && npx @vscode/vsce package` ile `.vsix` oluşturulur.

## Dil rehberi (v0.1)

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

Stüdyo: `src/studyo/` (yerel HTTP sunucusu, proje/dosya API'si, program çalıştırıcı, şablonlar) ve
`studio/` (arayüz: HTML, CSS, bağımlılıksız JavaScript; `build.rs` ile ikili dosyaya gömülür).

`runtime/orhunca_rt.c`: yazdırma, metin ve liste işlemleri ile çöp toplayıcı. Toplayıcı
"tutucu" bir işaretle-süpür toplayıcıdır: yığıttaki ve yazmaçlardaki her sözcüğü olası bir
işaretçi sayar, böylece derleyicinin ürettiği koda ek bir şey gerekmez.

Testler: `cargo test` (birim testleri + `örnekler/` klasöründeki programları derleyip çalıştıran uçtan uca testler).

Yol haritası ve kararlar için: [PLAN.md](PLAN.md), sohbet özeti: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).
