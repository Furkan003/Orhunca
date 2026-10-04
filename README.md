<p align="center">
  <a href="https://furkan003.github.io/Orhunca/"><img src="docs/marka/logo-yatay.svg" alt="Orhunca" height="72"></a>
</p>

<h3 align="center">Türkçe düşün, Türkçe kodla.</h3>

<p align="center">
  Türkçenin hâl ekleriyle yazılan, derlenen bir programlama dili ve okullar için ücretsiz geliştirme ortamı.
</p>

<p align="center">
  <a href="https://furkan003.github.io/Orhunca/#indir"><b>İndir</b></a> ·
  <a href="https://furkan003.github.io/Orhunca/dene.html"><b>Tarayıcıda dene</b></a> ·
  <a href="https://furkan003.github.io/Orhunca/dersler/"><b>Dersler</b></a> ·
  <a href="https://github.com/Furkan003/Orhunca/releases">Sürümler</a>
</p>

<p align="center">
  <a href="https://github.com/Furkan003/Orhunca/actions/workflows/ci.yml"><img src="https://github.com/Furkan003/Orhunca/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Furkan003/Orhunca/releases/latest"><img src="https://img.shields.io/github/v/release/Furkan003/Orhunca?label=s%C3%BCr%C3%BCm&color=45d3c9" alt="Sürüm"></a>
  <img src="https://img.shields.io/badge/Windows%20%C2%B7%20Linux%20%C2%B7%20Pardus%20%C2%B7%20macOS-0f1216" alt="Windows · Linux · Pardus · macOS">
</p>

<p align="center">
  <img src="docs/ekran/duzenleyici.png" alt="Orhunca Stüdyo" width="860">
</p>

```orhunca
fiil (kişi: metin)'yi selamla:
    ("Merhaba, " + kişi + "!")'yı yaz.

"Ayşe"'yi selamla.

notlar = [85, 92, 78, 64, 99]
her n için notlar'dan:
    eğer n 90'dan büyükse:
        n'yi yaz.
```

Hâl ekleri parametrenin rolünü belirler, fiil sona gelir. Derleyici Rust ile yazılmıştır ve
**Cranelift** ile doğrudan makine kodu üretir; programlar tek dosyalık çalıştırılabilir dosyaya,
tarayıcı için WebAssembly'ye ya da kendi penceresinde açılan masaüstü uygulamasına derlenir.

## İndir

| Sistem | Orhunca Stüdyo (önerilen) | Yalnızca `orhunca` komutu |
|---|---|---|
| **Windows 10/11** | [Kurulum (.exe)](https://github.com/Furkan003/Orhunca/releases/latest/download/Orhunca-Studyo-Windows-Kurulum.exe) · [MSI (okul/kurumsal)](https://github.com/Furkan003/Orhunca/releases/latest/download/Orhunca-Studyo-Windows.msi) | [.zip](https://github.com/Furkan003/Orhunca/releases/latest/download/orhunca-windows-x86_64.zip) · `irm https://furkan003.github.io/Orhunca/kur.ps1 \| iex` |
| **Pardus, Ubuntu, Debian** | [.deb](https://github.com/Furkan003/Orhunca/releases/latest/download/orhunca-studyo_amd64.deb) · [AppImage](https://github.com/Furkan003/Orhunca/releases/latest/download/Orhunca-Studyo-Linux.AppImage) · [.rpm](https://github.com/Furkan003/Orhunca/releases/latest/download/orhunca-studyo.x86_64.rpm) | [.tar.gz](https://github.com/Furkan003/Orhunca/releases/latest/download/orhunca-linux-x86_64.tar.gz) · `curl -fsSL https://furkan003.github.io/Orhunca/kur.sh \| sh` |
| **macOS 10.15+** | [.dmg](https://github.com/Furkan003/Orhunca/releases/latest/download/Orhunca-Studyo-macOS.dmg) (Apple işlemcili ve Intel) | [.tar.gz](https://github.com/Furkan003/Orhunca/releases/latest/download/orhunca-macos.tar.gz) |

Stüdyo kurulumları `orhunca` komutunu da kurar. Kurulum yapmadan denemek için:
**[tarayıcıda dene](https://furkan003.github.io/Orhunca/dene.html)**.

Web uygulamaları da aynı dille yazılır (modeller, yollar, `.ohchtml` görünümleri):

```orhunca
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

## Türkçe arayüz dili

Düğmeli, giriş kutulu, listeli uygulamalar Orhunca ile yazılır; JavaScript gerekmez. Program
WebAssembly'ye derlenir ve tarayıcıda (ya da Stüdyo'nun canlı önizlemesinde) çalışır.

```orhunca
durum sayaç = 0
durum adım = 1

arayüz:
    başlık("Sayaç")
    yazı("Şu anki değer: " + sayaç, boyut: 24)
    satır:
        düğme("− " + adım) tıklanınca:
            sayaç -= adım
        düğme("+ " + adım) tıklanınca:
            sayaç += adım
    kaydırıcı(adım, 1, 10)
    eğer sayaç 10'dan büyükse:
        yazı("On'u geçtin!", renk: "kırmızı", kalın: doğru)
```

```sh
orhunca çalıştır sayaç.ohc               # sayfayı derler ve tarayıcıda açar
orhunca derle sayaç.ohc --hedef web       # → sayaç.html (tek dosya)
```

- **`durum ad = değer`** — uygulamanın değişkenleri; programın her yerinden (işlevler dahil)
  görülür ve değiştirilir. Tipi ilk değerden çıkarılır; boş liste için yazılır:
  `durum işler: liste<İş> = []`.
- **`arayüz:`** — ekranda görünenler. Her olaydan sonra yeniden çizilir; `eğer`, `her ... için` ve
  yerel değişkenler kullanılabilir, yalnızca değişen yerler sayfada güncellenir.
- **Olaylar:** `düğme("Ekle") tıklanınca:` gibi bir bloğun içinde sıradan Orhunca kodu yazılır.
  Bloğun kullandığı çevre değişkenleri (ör. döngü değişkeni) öğe çizilirken yakalanır:
  `her iş için işler'den:` içindeki her "Sil" düğmesi kendi işini siler.
- **Bağlama:** `giriş(ad)`, `onay_kutusu(iş.bitti, "Bitti")`, `seçim(şehir, [...])`,
  `kaydırıcı(ses, 0, 100)` değeri bir durum değişkenine (ya da liste öğesine, model alanına) bağlar:
  kullanıcı değiştirince değişken güncellenir.
- **`bileşen Kart(başlık: metin):`** — arayüzün yeniden kullanılan parçası; `Kart("...")` diye çağrılır.

| Öğe | Örnek | Olaylar |
|---|---|---|
| `başlık`, `alt_başlık`, `yazı` | `yazı("Toplam: " + toplam)` | tıklanınca |
| `düğme` | `düğme("Kaydet") tıklanınca:` | tıklanınca |
| `giriş`, `metin_alanı` | `giriş(ad, "Adınız", tür: "şifre")` | değişince, gönderilince (Enter) |
| `onay_kutusu`, `seçim`, `kaydırıcı` | `seçim(şehir, ["Ankara", "İzmir"])` | değişince |
| `resim`, `bağlantı`, `ilerleme`, `ayraç`, `boşluk` | `bağlantı("Orhunca", "https://...")` | tıklanınca |
| `satır:`, `sütun:`, `kart:`, `kutu:`, `ızgara(3):` | içine öğe alan kapsayıcılar | tıklanınca |
| `zamanlayıcı(1)` | `zamanlayıcı(1) çalınca:` (her saniye) | çalınca |
| `tablo` | `tablo([["Ad", "Not"], ["Ayşe", "90"]])` (ilk satır başlık) | — |
| `grafik` | `grafik(notlar, adlar, "çubuk")` (`"çizgi"`, `"pasta"`) | — |
| `sekmeler` | `sekmeler(sekme, ["Genel", "Ayarlar"])` + `eğer sekme == "Genel" ise:` | değişince |
| `iletişim_kutusu:` | `iletişim_kutusu(açık, "Emin misiniz?"):` (açık doğruyken görünür; kapatılınca yanlış olur) | değişince |

Seçenekler bütün öğelerde kullanılabilir: `renk`, `arka`, `boyut`, `kalın`, `eğik`, `hizala`
(`"sol"`, `"orta"`, `"sağ"`), `genişlik`, `yükseklik`, `boşluk`, `iç_boşluk`, `köşe`, `kenarlık`,
`ipucu`, `etkin`, `gizli`, `sınıf`. Renkler Türkçe yazılabilir (`"kırmızı"`, `"lacivert"`,
`"açık_gri"` …) ya da CSS biçiminde (`"#3366ff"`). Örnekler: [örnekler/arayüz/](örnekler/arayüz)
(sayaç, yapılacaklar listesi, hesap makinesi, sınıf defteri: tablo, grafik, sekmeler ve iletişim kutusu).

![Orhunca Stüdyo — arayüz uygulaması canlı önizlemede](docs/ekran/arayuz.png)

### Masaüstü uygulaması olarak paketleme

```sh
orhunca paketle sayaç.ohc                    # Linux → ./sayaç
orhunca paketle sayaç.ohc --hedef windows    # Windows → sayaç.exe (Linux'tan da üretilebilir)
```

Çıkan tek dosya, uygulamayı tarayıcı olmadan **kendi penceresinde** açar (Windows'ta sistemdeki
WebView2, Linux'ta WebKitGTK; Windows 10/11'de ve masaüstü Linux dağıtımlarında hazır bulunur).
Programın dosyaları (`dosyaya_yaz`, `dosya_oku` …) kullanıcının veri klasöründe kalıcı olarak
saklanır (`%APPDATA%\orhunca-<ad>`, `~/.local/share/orhunca-<ad>`). Stüdyo'da: Çalıştır menüsü →
*Masaüstü uygulaması*. Pencere kabuğunun kaynağı [masaustu/kabuk/](masaustu/kabuk) (wry/tao);
derleyici hazır kabukları içinde taşır, ek bir araç gerekmez.

## Öz-barındırma

Derleyicinin parçaları Orhunca'ya taşınıyor:

- [öz/sözcük.ohc](öz/sözcük.ohc): sözcük çözümleyici (aynı sözcükler, konumlar ve hata mesajları).
- [öz/ayrıştırıcı.ohc](öz/ayrıştırıcı.ohc): ayrıştırıcı — hâl ekleri ve ek çözümlemesi, fiil
  cümleleri, Türkçe koşullar, modeller, seçenekler, web yolları ve arayüz öğeleri. Derleyicinin
  ayrıştırıcısıyla aynı söz dizimi ağacını ve aynı hata mesajlarını üretir.

- [runtime/ön_kütüphane.ohc](runtime/ön_kütüphane.ohc): standart kütüphanenin metin işlemleri
  (`kırp`, `böl`, `satırlar`, `bul`, `değiştir`, `tekrarla`, `ters`, `büyük_harf`, `küçük_harf`,
  `kaçır`, `url_kodla`...) C çalışma zamanından Orhunca'ya taşındı. Derleyici her programa yalnızca
  kullanılan işlevleri ekler; C'de bellek, listeler ve `kodlar` / `kodlardan` gibi temel parçalar kalır.

`tests/oz.rs` sözcük çözümleyiciyi ve ayrıştırıcıyı bütün örneklerde, şablonlarda ve onlarca hatalı girdide derleyiciyle
karşılaştırır (yerel ve WebAssembly derlemesiyle). Ayrıntılar: [öz/BENİOKU.md](öz/BENİOKU.md).

```sh
orhunca çalıştır öz/sözcükle.ohc -- örnekler/merhaba.ohc
orhunca çalıştır öz/ayrıştır.ohc -- örnekler/fiiller.ohc
```

## Orhunca Stüdyo

```sh
orhunca stüdyo
```

Tarayıcıda Orhunca'nın geliştirme ortamını açar: son projeler, şablon sihirbazı (Konsol Uygulaması,
Sayı Tahmin Oyunu, Kütüphane; Boş Web Sayfası, Web Sitesi, Web Uygulaması, Açılış Sayfası, Web API,
Tam Yığın Uygulama; Arayüz Uygulaması), sözdizimi renklendirmeli düzenleyici (`.ohc`, `.ohchtml`, CSS, JavaScript),
yazarken hata gösterimi, F5 ile derleyip çalıştırma (programın girdisi terminalden verilir),
kesme noktalı hata ayıklayıcı, Linux/Windows için dağıtım derlemesi ve masaüstü paketleme ve Türkçe anahtar kelime rehberi. İnternet gerekmez; arayüz ve
yazı tipleri ikili dosyanın içindedir.

Web projelerinde F5 sunucuyu başlatır ve sayfa sağdaki **canlı önizlemede** açılır. Kaydettiğinizde
sunucu yeniden derlenir ve önizleme bulunduğu adreste yenilenir (yalnızca `statik/` dosyası
değiştiyse sayfa yenilenir). Stüdyo kapanınca başlattığı sunucular da kapanır. Arayüz
uygulamalarında F5 programı WebAssembly'ye derler ve uygulama önizlemede (Stüdyo'dan yalıtılmış bir
çerçevede) çalışır; kaydettiğinizde yeniden derlenir.

**Hata ayıklama:** satır numarasına tıklayarak (ya da F9) kesme noktası koyun, F6 ile programı
hata ayıklayarak başlatın. Program kesme noktasında durur; durulan satır vurgulanır, yan panelde
o anki **değişkenler** (tipleri ve değerleriyle; listeler, sözlükler ve modeller dahil) ve **çağrı
yığını** görünür; yığındaki bir işleve tıklayınca onun değişkenleri gösterilir. F5 devam, F10
üstünden adım, F11 içine adım, ⇧F11 dışına adım; çalışırken *Duraklat* ile program bulunduğu
yerde durdurulur ve kesme noktaları program çalışırken de eklenip kaldırılabilir. Yakalanmamış
bir çalışma hatasında program kapanmadan önce hatanın olduğu satırda durur.

![Orhunca Stüdyo — hata ayıklama](docs/ekran/hata-ayiklama.png)

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

### Standart kütüphane

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

Yol haritası ve kararlar için: [PLAN.md](PLAN.md), sohbet özeti: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).
