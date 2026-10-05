# Geliştirme

Derleyiciyi kaynaktan derlemek, mimari, öz-barındırma ve düzenleyici desteği.

[← README](../README.md) · [Katkı rehberi](../CONTRIBUTING.md)

## Kaynaktan derleme ve kullanım

Gerekenler: yalnızca Rust (cargo). Programları derlemek için C derleyicisi ya da bağlayıcı
gerekmez: Linux ve Windows (x86-64) için çalışma zamanı derleyicinin içine gömülüdür, Windows `.exe`
dosyaları Linux'tan da üretilebilir. Diğer sistemlerde (macOS) ya da `ORHUNCA_CC=cc` verilince
sistemin C derleyicisiyle bağlanır.

```sh
cargo build --release
./target/release/orhunca çalıştır örnekler/merhaba.ohc
./target/release/orhunca derle örnekler/asal.ohc               # → ./asal
./target/release/orhunca derle örnekler/asal.ohc --hedef windows  # → asal.exe
./target/release/orhunca derle örnekler/asal.ohc --hedef web      # → asal.html (tarayıcıda açılır)
./target/release/orhunca paketle örnekler/arayüz/sayaç.ohc        # → sayaç (kendi penceresinde açılan uygulama)
./target/release/orhunca denetle dosya.ohc                       # yalnızca hata denetimi
./target/release/orhunca yeni dükkan                             # yeni proje (.ohcproj)
./target/release/orhunca yeni dükkan --şablon tam_yigin          # web projesi (şablonlar: web_sitesi, web_api, ...)
./target/release/orhunca stüdyo                                  # geliştirme ortamı (tarayıcıda)
```

Dosya verilmezse geçerli klasördeki `.ohcproj` dosyasının `giriş` dosyası kullanılır.

## Tarayıcıda çalıştırma (WebAssembly)

`--hedef web` programı WebAssembly'ye derler. Çıktı her şeyi içinde taşıyan tek bir HTML dosyasıdır:
çift tıklayınca tarayıcıda açılır, program çalışır ve çıktısı sayfada görünür. `oku()` bir giriş
penceresi açar (son yazılan satır soru olarak görünür); dosyalar ve model kayıtları tarayıcının yerel
deposunda (localStorage) tutulur.

```sh
orhunca derle oyun.ohc --hedef web                 # → oyun.html
orhunca derle oyun.ohc --hedef web -o oyun.wasm    # → oyun.wasm + orhunca_rt.wasm + orhunca.js
node orhunca.js oyun.wasm                          # aynı modül Node.js'te
orhunca çalıştır oyun.ohc --hedef web              # derler ve Node.js ile çalıştırır
```

- C derleyicisi gerekmez: çalışma zamanı önceden WebAssembly'ye derlenmiş olarak derleyicinin içindedir.
- Çıktı yerel derlemeyle aynıdır (ondalık biçimleri, rastgele sayılar, hata mesajları dahil); bütün
  örnekler iki yolla da test edilir.
- Web sunucusu (`al "/"` yolları, `sun()`) tarayıcıda çalışmaz; böyle programlar yerel olarak derlenir.
- Program tarayıcının ana iş parçacığında çalışır: çıktı program bitince görünür, `bekle()` sayfayı
  bekletir.

## Düzenleyici desteği

- **Biçimlendirici:** `orhunca biçimlendir [dosya.ohc]` girintiyi düzenler ve kesme işaretli eklerde
  ünlü uyumunu düzeltir (`5'a` → `5'e`, `3'den` → `3'ten`, `6'i` → `6'yı`). Sayılar okunuşlarına göre
  ek alır. `--denetle` dosyaları değiştirmeden denetler (CI için).
- **Dil sunucusu (LSP):** `orhunca dil-sunucusu` — hatalar ve yazım uyarıları, üzerine gelince açıklama
  (yerleşik işlevler, değişken tipleri), tamamlama, tanıma gitme, belge simgeleri, biçimlendirme.
  LSP destekleyen her düzenleyiciyle (VS Code, Neovim, Helix, Zed…) çalışır.
- **VS Code eklentisi:** [editors/vscode](../editors/vscode) — `.ohc` ve `.ohchtml` için sözdizimi renklendirme ve dil sunucusu istemcisi.
  `cd editors/vscode && npm install && npx @vscode/vsce package` ile `.vsix` oluşturulur.

## Öz-barındırma

Derleyicinin parçaları Orhunca'ya taşınıyor:

- [öz/sözcük.ohc](../öz/sözcük.ohc): sözcük çözümleyici (aynı sözcükler, konumlar ve hata mesajları).
- [öz/ayrıştırıcı.ohc](../öz/ayrıştırıcı.ohc): ayrıştırıcı — hâl ekleri ve ek çözümlemesi, fiil
  cümleleri, Türkçe koşullar, modeller, seçenekler, web yolları ve arayüz öğeleri. Derleyicinin
  ayrıştırıcısıyla aynı söz dizimi ağacını ve aynı hata mesajlarını üretir.

- [runtime/ön_kütüphane.ohc](../runtime/ön_kütüphane.ohc): standart kütüphanenin metin işlemleri
  (`kırp`, `böl`, `satırlar`, `bul`, `değiştir`, `tekrarla`, `ters`, `büyük_harf`, `küçük_harf`,
  `kaçır`, `url_kodla`...) C çalışma zamanından Orhunca'ya taşındı. Derleyici her programa yalnızca
  kullanılan işlevleri ekler; C'de bellek, listeler ve `kodlar` / `kodlardan` gibi temel parçalar kalır.

`tests/oz.rs` sözcük çözümleyiciyi ve ayrıştırıcıyı bütün örneklerde, şablonlarda ve onlarca hatalı girdide derleyiciyle
karşılaştırır (yerel ve WebAssembly derlemesiyle). Ayrıntılar: [öz/BENİOKU.md](../öz/BENİOKU.md).

```sh
orhunca çalıştır öz/sözcükle.ohc -- örnekler/merhaba.ohc
orhunca çalıştır öz/ayrıştır.ohc -- örnekler/fiiller.ohc
```

## Mimari

```
dosya.ohc
   ↓
[Sözcük çözümleyici]      src/sozcuk.rs     girinti, kesme işaretli ekler
[Ayrıştırıcı]             src/ayristirici.rs Türkçe cümle yapısı, hâl ekleri (src/ekler.rs)
[Anlam ve tip denetimi]   src/denetci.rs    Türkçe hata mesajları
[Kod üretici]             src/uretici.rs    Cranelift IR → nesne dosyası
                          src/wasm_uretici.rs  ya da WebAssembly modülü (--hedef web)
   ↓
[Bağlama]  src/baglayici.rs: nesne + gömülü çalıştırıcı (runtime/calistirici/) → Linux / Windows .exe
           (ORHUNCA_CC ya da macOS: nesne + runtime/orhunca_rt.c, sistemin C derleyicisiyle)
           wasm + runtime/wasm/orhunca_rt.wasm + orhunca.js → tek dosyalık HTML sayfası
```

Arayüz dili: `arayüz:` bloğu ve bileşenler sıradan işlevlere derlenir; öğeler (`src/arayuz.rs`
tablosu) çizim sırasında JavaScript'teki bir öğe ağacına eklenir ve `orhunca.js` ağacı önceki
çizimle karşılaştırarak sayfayı günceller. Olay blokları ayrı işlevlere çevrilir; çizimde yakalanan
değerlerle bir olay listesine kaydedilir ve olay olunca tablo üzerinden çağrılır. `durum`
değişkenleri gölge yığıtın dibindeki genel bölgede durur (toplayıcı her zaman tarar).

WebAssembly: program modülü, C çalışma zamanının wasm32 derlemesini (`runtime/wasm/orhunca_rt.wasm`;
`araclar/wasm_calisma_zamani.sh` ile clang'la derlenir, küçük C kütüphanesi `runtime/wasm/libc.c`)
içe aktarır: belleği ve `ohc_*` işlevlerini. `runtime/wasm/orhunca.js` iki modülü birleştirir ve
çıktı, girdi, dosya ve zaman isteklerini tarayıcıda ya da Node.js'te karşılar; Wasm bağlayıcısı
gerekmez. WebAssembly'de yığıt taranamadığı için derleyici metin, liste, sözlük ve model değerlerini
bellekteki bir "gölge yığıtta" tutar (bir Orhunca işlevi çağrılırken yaşayan ara değerler dahil);
toplayıcı yalnızca işlev girişlerinde ve döngü başlarında çalışır ve gölge yığıtı tarar.

Yerleşik bağlayıcı: çalışma zamanı her hedef için önceden derlenmiş bir "çalıştırıcı"dır
(`araclar/calistiricilar.sh`, gcc ve MinGW ile). Derleyici Cranelift'in nesne dosyasını (ELF ya da
COFF) tek bir kod bloğuna yerleştirir, iç yerleşimleri çözer, çalışma zamanı işlevleri için atlama
basamakları ve adres tablosu ekler ve bloğu çalıştırıcının sonuna yazar. Program açılınca çalıştırıcı
bloğu belleğe yükler, çalışma zamanı işlevlerinin adreslerini yazar ve çalıştırır.

Görünümler: `src/sablon.rs` `.ohchtml` dosyalarını parçalara ayırır; ayrıştırıcı onları metin
döndüren işlevlere çevirir. Web yolları da `istek` alıp `Yanıt` döndüren işlevlerdir; ana program
başlarken çalışma zamanına kaydedilir.

Stüdyo: `src/studyo/` (yerel HTTP sunucusu, proje/dosya API'si, program çalıştırıcı, şablonlar —
web şablonlarının dosyaları `src/studyo/sablon_dosyalari/`) ve `studio/` (arayüz: HTML, CSS,
bağımlılıksız JavaScript; `build.rs` ile ikili dosyaya gömülür).

`runtime/orhunca_rt.c`: yazdırma, metin, liste ve sözlük işlemleri, çöp toplayıcı, model kayıtları
(JSON), doğrulama ve olay döngülü HTTP/1.1 sunucusu (Windows'ta Winsock; HTTPS için OpenSSL çalışırken yüklenir). Toplayıcı "tutucu" bir
işaretle-süpür toplayıcıdır: yığıttaki ve yazmaçlardaki her sözcüğü olası bir işaretçi sayar, böylece
derleyicinin ürettiği koda ek bir şey gerekmez. Bir web isteğindeki çalışma hatası isteğin başına
geri sarılır (`__builtin_setjmp`), sunucu durmaz.

Testler: `cargo test` (birim testleri, `örnekler/` klasöründeki programları derleyip çalıştıran uçtan
uca testler, gerçek HTTP istekleriyle web çatısı testi `tests/web.rs`, Stüdyo testleri, örnekleri
WebAssembly'ye derleyip Node.js ile çalıştıran `tests/wasm.rs`).

Yol haritası ve kararlar için: [PLAN.md](../PLAN.md), sohbet özeti: [docs/sohbet-ozeti.md](sohbet-ozeti.md).
