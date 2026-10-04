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
```

Dosya verilmezse geçerli klasördeki `.ohcproj` dosyasının `giriş` dosyası kullanılır.

## Dil rehberi (v0.1)

### Değerler ve tipler

| Tip | Örnek |
|---|---|
| `sayı` (64 bit tamsayı) | `42`, `-7`, `1_000_000` |
| `ondalık` (64 bit kayan nokta) | `3.14`, `-0.5`, `19.99` |
| `metin` | `"Merhaba"` (`\n`, `\t`, `\"`) |
| `mantık` | `doğru`, `yanlış` |
| `liste<T>` | `[3, 8, 1]`, `["a", "b"]`, `[]` |

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
5'i sayılara ekle.
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

### Yerleşik işlevler

`uzunluk(x)` (liste ya da metin, karakter sayısı) · `metin(x)` · `sayı(x)` (ondalığı keser) ·
`ondalık(x)` (`"3,5"` gibi virgüllü metinleri de okur) · `yuvarla(x)` (2.5 → 3) ·
`yuvarla(x, 2)` (2 basamağa) · `oku()` (klavyeden bir satır)

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

`runtime/orhunca_rt.c`: yazdırma, metin ve liste işlemleri ile çöp toplayıcı. Toplayıcı
"tutucu" bir işaretle-süpür toplayıcıdır: yığıttaki ve yazmaçlardaki her sözcüğü olası bir
işaretçi sayar, böylece derleyicinin ürettiği koda ek bir şey gerekmez.

Testler: `cargo test` (birim testleri + `örnekler/` klasöründeki programları derleyip çalıştıran uçtan uca testler).

Yol haritası ve kararlar için: [PLAN.md](PLAN.md), sohbet özeti: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).
