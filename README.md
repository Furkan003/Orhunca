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
| `metin` | `"Merhaba"` (`\n`, `\t`, `\"`) |
| `mantık` | `doğru`, `yanlış` |
| `liste<T>` | `[3, 8, 1]`, `["a", "b"]`, `[]` |

Tipler çıkarılır; bir değişkenin tipi sonradan değişemez.

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
Aritmetik: `+ - * / %` · Karşılaştırma: `== != < > <= >=` · Mantık: `ve`, `veya`, `değil`

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

### Yerleşik işlevler

`uzunluk(x)` (liste ya da metin, karakter sayısı) · `metin(x)` · `sayı(x)` · `oku()` (klavyeden bir satır)

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

Testler: `cargo test` (birim testleri + `örnekler/` klasöründeki programları derleyip çalıştıran uçtan uca testler).

Yol haritası ve kararlar için: [PLAN.md](PLAN.md), sohbet özeti: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).
