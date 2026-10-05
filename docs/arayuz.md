# Türkçe arayüz dili

Pencereli ve tarayıcıda çalışan uygulamalar: `durum`, `arayüz:`, düğmeler, tablolar, grafikler.

[← README](../README.md)


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
`"açık_gri"` …) ya da CSS biçiminde (`"#3366ff"`). Örnekler: [örnekler/arayüz/](../örnekler/arayüz)
(sayaç, yapılacaklar listesi, hesap makinesi, sınıf defteri: tablo, grafik, sekmeler ve iletişim kutusu).

![Orhunca Stüdyo — arayüz uygulaması canlı önizlemede](ekran/arayuz.png)

### Masaüstü uygulaması olarak paketleme

```sh
orhunca paketle sayaç.ohc                    # Linux → ./sayaç
orhunca paketle sayaç.ohc --hedef windows    # Windows → sayaç.exe (Linux'tan da üretilebilir)
```

Çıkan tek dosya, uygulamayı tarayıcı olmadan **kendi penceresinde** açar (Windows'ta sistemdeki
WebView2, Linux'ta WebKitGTK; Windows 10/11'de ve masaüstü Linux dağıtımlarında hazır bulunur).
Programın dosyaları (`dosyaya_yaz`, `dosya_oku` …) kullanıcının veri klasöründe kalıcı olarak
saklanır (`%APPDATA%\orhunca-<ad>`, `~/.local/share/orhunca-<ad>`). Stüdyo'da: Çalıştır menüsü →
*Masaüstü uygulaması*. Pencere kabuğunun kaynağı [masaustu/kabuk/](../masaustu/kabuk) (wry/tao);
derleyici hazır kabukları içinde taşır, ek bir araç gerekmez.

### Telefon uygulaması

Arayüz programları Android ve iPhone uygulamasına da dönüşür:
`orhunca paketle uygulama.ohc --hedef android` (ya da `--hedef ios`). Telefona özel
komutlar `titret`, `paylaş` ve `bildirim_gönder`'dir. Ayrıntılar: [Telefon uygulamaları](mobil.md).
