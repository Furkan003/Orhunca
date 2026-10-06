# Orhunca — Plan

Kararların gerekçesi için: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).

## Durum (v0.5)

| Aşama | İçerik | Durum |
|---|---|---|
| 0 | Depo, sohbet özeti, plan | ✅ |
| 1 | Sözcük çözümleyici + ayrıştırıcı (Türkçe söz dizimi, hâl ekleri) | ✅ |
| 2 | Değişkenler, `eğer`, döngüler, işlevler, ekrana yazma, listeler, metinler | ✅ |
| 3 | Cranelift ile çalıştırılabilir dosya: Linux ✅, Windows `.exe` (çapraz derleme, mingw) ✅ | ✅ |
| 4 | Biçimlendirici ✅, dil sunucusu (LSP) ✅, VS Code eklentisi ✅ | ✅ |
| 5 | Standart kütüphane ✅, paket yöneticisi ✅ (Git tabanlı, kilit dosyalı) | ✅ |
| 6 | Modeller ✅, JSON veri deposu ✅, web sunucusu ✅, `.ohchtml` görünümleri ✅, Stüdyo web şablonları ve canlı önizleme ✅ | ✅ |
| 7 | Orhunca Stüdyo: arayüz ✅ (tarayıcıda, `orhunca stüdyo`), Tauri masaüstü uygulaması ✅ (`masaustu/`) | ✅ |
| 8 | WebAssembly ✅ (`--hedef web`), Türkçe arayüz dili ✅, öz-barındırmanın ilk adımı ✅ (Orhunca ile sözcük çözümleyici) | ✅ |
| 9 | Hata yakalama, seçenekler, iç içe modeller, tip çıkarımı, Orhunca'da ayrıştırıcı ve metin kütüphanesi, eşzamanlı web sunucusu, C derleyicisiz derleme, masaüstü paketleme, hata ayıklayıcı | ✅ |

## v0.2'de eklenenler

- **Ondalık sayılar** (`ondalık` tipi, `3.14`). `/` her zaman ondalık verir, `//` tam bölme.
- **Kullanıcı tanımlı fiiller**: `fiil sayı'yı karele:` → `5'i karele.` Parametreler hâl
  ekleriyle eşleşir, çağrıda sıra önemsiz.
- **Otomatik bellek yönetimi**: çalışma zamanında tutucu işaretle-süpür çöp toplayıcı.

## v0.3'te eklenenler (Aşama 5a – standart kütüphane)

- ~50 yerleşik işlev: metin, liste, sözlük, dosya, matematik, zaman, sistem.
- `sözlük<A, D>` tipi, `{"a": 1}` yazımı, `s[a] = d`.
- `kullan "dosya.ohc"` ile birden fazla dosya; hatalar doğru dosyayı gösterir.
- `sabit AD = değer`; hazır `pi`.
- `çıkar` fiili, dosyaya yazan `yaz`, metin karşılaştırma (`<`), metinde harf gezme.
- Tamsayı taşması denetimi; programa argüman geçirme (`--`).

## Orhunca Stüdyo (Aşama 7, ilk sürüm)

- Tasarım: `docs/tasarim/Orhunca Başlangıç.dc.html`. Görünüm birebir uygulandı; içerik gerçek dile
  uyarlandı (`.ohc`/`.ohcproj`, gerçek söz dizimi, çalışan şablonlar).
- Düzenleyici bağımlılıksız: sözdizimi renklendirme, otomatik girinti, yorum satırı, hata çizgileri.
  Monaco gerekirse ileride takılabilir; arayüz aynı API'yi kullanır.
- Web/sunucu şablonları "Aşama 6'da geliyor" olarak gösterilir; canlı önizleme paneli web desteğiyle gelecek.
- Tauri: aynı `studio/` arayüzü `masaustu/` uygulamasında çerçevesiz bir pencerede açılır; pencere
  düğmeleri ve başlık çubuğundan sürükleme `__TAURI__` ile çalışır. Kurulum dosyaları (deb, AppImage,
  rpm, msi, exe, dmg) `.github/workflows/masaustu.yml` ile üretilir.

## v0.4'te eklenenler (Aşama 6 – web)

- **Modeller**: `model Ürün:` alanlar, varsayılanlar, `zorunlu`/`en_az`/`en_fazla`/`e_posta`/`etiket`
  kuralları, Türkçe doğrulama mesajları, `ürün.ad` erişimi (ekli yazım: `ürün.fiyatı`).
- **Veri deposu**: `veri/<Model>.json` — `kaydet`, `hepsi`, `bul`, `var_mı`, `sil`. Dosya tabanlı ve
  okunabilir; her işlem dosyayı okuyup yazar (küçük uygulamalar için). Büyük veriler için SQLite
  ileride eklenebilir.
- **Web sunucusu** (çalışma zamanında, C): yollar (`al`/`gönder`/`koy`/`sil`), `{kimlik: sayı}`
  parametreleri, form/JSON bağlama, JSON yanıtlar, `statik/`, 404/405/500 sayfaları, Windows desteği.
- **Görünümler**: `.ohchtml` → derleme zamanında işleve çevrilir; düzen, başlık, parça görünümler.
- **Stüdyo**: altı web şablonu, canlı önizleme (kaydedince sunucu yeniden derlenir; sayfa bulunduğu
  adreste yenilenir), `.ohchtml`/CSS/JS renklendirme; `orhunca yeni <ad> --şablon <şablon>`.
- Ek çözümleyicide ünlü düşmesi: `isim` → `ismi`, `metin` → `metni`.

## Aşama 8a – WebAssembly

- `--hedef web`: tek dosyalık HTML sayfası (iki wasm modülü base64 olarak gömülü); `-o x.wasm` ile ayrı
  dosyalar; `çalıştır --hedef web` Node.js ile çalıştırır.
- C çalışma zamanı aynı kaynaktan wasm32'ye derlenir (`#ifdef __wasm__`): sistem işleri küçük bir C
  kütüphanesiyle (`runtime/wasm/libc.c`) JavaScript'e devredilir. `%g` biçimi JavaScript'te tam ondalık
  açılımla (BigInt) yapılır; çıktılar glibc ile aynıdır. 128 bitlik çarpma kaldırıldı (wasm32'de
  derleyici kütüphanesi gerekirdi); taşma denetimi her iki hedefte aynı taşınabilir kodla yapılır.
- Çöp toplayıcı: yığıt taranamadığından gölge yığıt + güvenli noktalar. Derleyici, bir Orhunca işlevi
  çağrılırken canlı kalan yönetilen ara değerleri gölge yığıta kaydeder (`tests/ara_degerler.ohc`
  toplayıcı her ayırmada çalışırken bunu sınar).
- Yapılmayanlar: web sunucusu (tarayıcıda anlamsız), programın ayrı bir iş parçacığında (Web Worker)
  çalışması — şimdilik ana iş parçacığında, çıktı program bitince görünür.

## Aşama 8b – Türkçe arayüz dili

- `durum` (her yerden görülen, değişince arayüzü yeniden çizdiren değişkenler), `arayüz:` bloğu,
  `bileşen Ad(...):`, 20 öğe (`src/arayuz.rs`), olaylar (`tıklanınca`, `değişince`, `gönderilince`,
  `çalınca`), durum değişkenine/liste öğesine/model alanına bağlanan girişler, adlı seçenekler.
- Çizim: program her olaydan sonra arayüz işlevini çalıştırır, JavaScript öğe ağacını kurar ve
  önceki ağaçla karşılaştırarak DOM'u günceller (sıraya göre; odak ve imleç korunur, kullanıcının
  yazmakta olduğu değer program değiştirmedikçe ezilmez).
- Olay blokları kapanış (closure) gibi çalışır: çevredeki yerel değişkenler çizim anında yakalanır ve
  yalnızca okunur; kalıcı değişiklikler durum değişkenlerine yapılır.
- Stüdyo: "Arayüz Uygulaması" şablonu, F5 ile yalıtılmış canlı önizleme, kaydedince yeniden derleme.
  `orhunca çalıştır` arayüz programını tarayıcıda açar.
- Testler: örnekler Node.js'te DOM olmadan olay tetiklenerek sınanır (`tests/arayuz_senaryolari.js`,
  toplayıcı her ayırmada çalışırken); hata mesajları için ayrı test.
- Yapılmayanlar: anahtarlı liste karşılaştırması (öğeler sıraya göre eşleşir), yerel (Tauri)
  pencerede paketleme, animasyonlar, gezinme (çok sayfalı arayüz).

## Aşama 8c – Öz-barındırmanın ilk adımı

- `öz/sözcük.ohc`: derleyicinin sözcük çözümleyicisinin Orhunca ile yazılmış karşılığı; bütün
  örneklerde, şablonlarda ve hatalı girdilerde aynı sözcükleri, konumları ve hata mesajlarını
  üretir (`tests/oz.rs`; yerel ve WebAssembly derlemesiyle).
- Dile eklenenler: tipi yazılmış değişkenler (`çıktı: liste<Sözcük> = []`), `kod(m)` ve
  `karakter(n)`.
- Sıradaki adımlar `öz/BENİOKU.md`'de: ayrıştırıcı, tip denetçisi, WebAssembly kod üretimi.

## Aşama 9 – Eksiklerin giderilmesi

### 9a – Hata yakalama
- `dene:` / `yakala hata:` blokları ve `hata_ver(mesaj)`. Blok ayrı bir işleve çevrilir; bloğun
  kullandığı değişkenler çevreleyen işlevin çerçevesinde durur (hatadan önceki atamalar kalır).
  Yerelde çalışma zamanı `setjmp/longjmp` ile, WebAssembly'de yükleyici bir JavaScript
  istisnasıyla geri sarar. `döndür`, `dur`, `sürdür` ve iç içe bloklar desteklenir.

### 9b – Seçenek türleri (numaralandırma)
- `seçenek Renk: kırmızı, yeşil, mavi` (tek satır ya da blok); `Renk.kırmızı`, `Renk.hepsi()`,
  `Renk("mavi")` (geçersizse çalışma hatası). Tip denetimi ayrı türdür; çalışma zamanında değer
  adıdır (metin). Model alanı (varsayılan ilk değer, form doğrulaması), sözlük anahtarı, arayüzde
  `seçim` bağlama. Ayrıca `liste[i] += 1` gibi indeksli birleşik atama.

### 9c – İç içe modeller
- Model alanı başka bir model, `liste<Model>` ya da `sözlük<metin, Model>` olabilir. Varsayılan:
  iç modelin varsayılan nesnesi; kendi modeline dönen zincirlerde (`sonraki: Düğüm`) boş
  (`boş_mu(x)`). Kayıtlarda iç nesneler gömülü JSON'dur; doğrulama iç modellere iner. Çalışma
  zamanı iç modelin tanımını adla bulur (tanım metninde `@Model` sütunu, programın başında kayıt).

### 9d – Parametre tipi çıkarımı
- İşlev gövdeleri ilk çağrıldıklarında denetlenir (önce ana program); tipi yazılmayan parametre
  (işlev, fiil, bileşen) ilk çağrıdaki değerin tipini alır, hiç çağrılmayan işlevde sayıdır.
  Dönüş tipleri de böylece çağrıdan önce bilinir (yalnızca özyinelemede varsayılan kalır).

### 9e – Öz-barındırma: ayrıştırıcı
- `öz/ayrıştırıcı.ohc`: derleyicinin ayrıştırıcısının Orhunca ile yazılmış karşılığı (ek
  çözümleme, cümleler, koşullar, tanımlar, arayüz). `src/dokum.rs` ağaç dökümüyle karşılaştırılır:
  bütün örnekler, şablonlar ve ~80 hatalı girdi; yerel ve WebAssembly.

### 9f – Çalışma zamanının bir kısmı Orhunca'da
- `runtime/ön_kütüphane.ohc`: metin işlemleri (kırp, başlar, biter, böl, satırlar, bul, içerir,
  değiştir, tekrarla, ters, büyük/küçük harf, kaçır, url_kodla) C'den taşındı. Ayrı ayrıştırılır
  (isimleri programa karışmaz), `‹öz›` önekiyle eklenir; yalnızca çağrılanlar derlenir. İçindeki
  yerleşik çağrıları programın tanımları gölgelemez.
- Yeni temel parçalar: `kodlar(m)` ve `kodlardan(l)` (karakter kodu listeleri; doğrusal zaman).
- Büyük/küçük harf dönüşümü düzeltildi (Latin Genişletilmiş-A çiftleri) ve Yunan, Kiril eklendi.

### 9g – Web sunucusu
- Olay döngüsü (poll / WSAPoll): engelsiz soketler, aynı anda birçok bağlantı, açık kalan
  (keep-alive) ve ardışık (pipelined) istekler, parçalı (chunked) gövde, zaman aşımları.
- `istek.çerezler`, `istek.oturum` (sunucuda JSON olarak saklanan oturum; rastgele 128 bit kimlik,
  HttpOnly/SameSite/Secure çerez), `y.çerezler`.
- Dosya yükleme: multipart/form-data → `istek.dosyalar` (`YüklenenDosya`), güvenli dosya adları;
  `dosya_taşı`.
- HTTPS: OpenSSL çalışma anında yüklenir (dlopen / LoadLibrary), `ORHUNCA_SERTIFIKA`,
  `ORHUNCA_ANAHTAR`. Testler: tests/web_sunucu.rs (keep-alive, yavaş istemci, oturum, yükleme, HTTPS).

### 9h – C derleyicisi olmadan derleme
- `runtime/calistirici/`: çalışma zamanının önceden derlenmiş "çalıştırıcı" biçimi (Linux x86-64,
  Windows x86-64; `araclar/calistiricilar.sh`). `src/baglayici.rs` nesne dosyasını (ELF/COFF) tek
  bloğa yerleştirir (iç yerleşimler, atlama basamakları, GOT) ve çalıştırıcının sonuna ekler;
  çalıştırıcı bloğu belleğe yükleyip çalıştırır. Derleme çok hızlandı (çalışma zamanı her seferinde
  derlenmiyor). macOS ve `ORHUNCA_CC` için sistem bağlayıcısı yolu sürer.
- CI: çalıştırıcılar kaynaktan yeniden derlenip testler onlarla çalışır; Windows işi örnekleri
  C derleyicisi olmadan derleyip çalıştırır.

### 9i – Masaüstü paketleme (`orhunca paketle`)
- `masaustu/kabuk/`: küçük pencere kabuğu (wry/tao; Windows'ta WebView2, Linux'ta WebKitGTK).
  Derleyici arayüz programının tek dosyalık sayfasını kabuğun sonuna ekler
  ("OHCKABUK" yükü + "ORHUNCA!" sonu); kabuk sayfayı `orhunca://` şemasından kendi penceresinde açar.
- Programın dosyaları `window.orhuncaKabukDepo` ile kullanıcının veri klasöründe `depo.json`
  dosyasında kalıcıdır. Sınama kipi (`ORHUNCA_KABUK_SINAMA=1`) sayfa metnini yazıp kapanır.
- Hazır kabuklar `runtime/kabuk/`: Linux (Ubuntu 22.04'te, glibc 2.34+) ve Windows (MSVC; WebView2
  yükleyicisi ve C çalışma zamanı gömülü, yalnızca sistem DLL'leri). CI kabukları kaynaktan derleyip
  gerçek pencerede sınar; Windows işi `orhunca paketle` çıktısını açar. Stüdyo: Çalıştır menüsü.

### 9j – Stüdyo hata ayıklayıcısı
- `derle --ayıklama` (Stüdyo'da F6): her deyimden önce `ohc_ay_satir(satır, dosya, yuvalar, tanım)`,
  işlev başında/sonunda `ohc_ay_gir`/`ohc_ay_cik`. Değişkenlerin değerleri yığıttaki bir yuvaya
  yazılır; tanım metni adları, tip kodlarını ve tip adlarını taşır (`dene:` gövdesinde çerçeve).
- Çalışma zamanı `ORHUNCA_AYIKLA` kapısından Stüdyo'ya bağlanır; satır tabanlı iletişim:
  kesmeler, devam, adim, ustunden, cik, cerceve, duraklat. Çalışırken de kesme noktaları değişir.
  Yakalanmamış çalışma hatasında program o satırda durur; yakalanan hatada yığın derinliği geri alınır.
- Stüdyo: satır numarasında kesme noktaları (F9), durulan satırın vurgusu, değişkenler, çağrı yığını
  (çerçeve seçimi), araç çubuğu (F5/F10/F11/⇧F11, duraklat). Test: tests/studyo.rs.

## Aşama 10 – Okullar için hazır ürün (v0.6)

### 10a – Marka
- "Ayraç" logosu (‹𐰆›), simgeler (`araclar/logo_uret.py` → `docs/marka/`), Stüdyo açılış animasyonu
  (Oyma → Ayraç → Yazı → Kod; tıklayınca ya da tuşla geçilir, azaltılmış hareket ayarına uyar).

### 10b – Sürüm hattı ve kurulum dosyaları
- `.github/workflows/surum.yml`: `orhunca` komutu (Linux glibc 2.17+, Windows, macOS evrensel) ve
  Stüdyo kurulum dosyaları: Windows `.exe` (NSIS) ve `.msi` (Türkçe), Linux `.deb`, `.rpm`,
  `.AppImage`, macOS `.dmg`. Kurulum `orhunca` komutunu PATH'e ekler, `.ohc`/`.ohcproj` dosyalarını
  Stüdyo'ya bağlar. `v*` etiketiyle GitHub Releases'e SHA256SUMS ile yayımlanır.
- `kurulum/kur.sh`, `kurulum/kur.ps1`: tek satırlık kurulum.

### 10c – Web sitesi ve tarayıcıda deneme
- `web/site` (GitHub Pages): indirme bölümü, derleyicinin WebAssembly'si ile tarayıcıda deneme
  (konsol programları Web Worker'da, arayüz programları yalıtılmış çerçevede; paylaşım bağlantısı).

### 10d – Hata mesajları
- Başka dillerden gelen alışkanlıklar (`print(...)`, `for`, `def` ...) için Orhunca karşılığı;
  yanlış yazılmış isimlere "bunu mu demek istediniz?" (Türkçe harfler sadeleştirilerek).

### 10e – Stüdyo kolaylıkları
- `orhunca etkileşim` (REPL), açık tema, otomatik tamamlama (doğru hâl ekiyle), adım adım gösterim,
  erişilebilirlik (klavye, ekran okuyucu).

### 10f – Öğrenme
- `dersler/`: 13 ders; Stüdyo'da Dersler paneli, alıştırmalar çalıştırılıp çıktıyla denetlenir.
  Günlük hayattan örnekler (not ortalaması, alışveriş listesi, birim çevirici, kelime sayacı).

### 10g – Kütüphane
- Tarih (`bugün`, `gün_ekle`, `gün_farkı`, `haftanın_günü`, `tarih_yazısı`), desenler (düzenli
  ifadeler; Orhunca ile yazılmış geri izlemeli makine), CSV, `json_al`, HTTP (`http_al`, `http_gönder`).
- Arayüz öğeleri: `tablo`, `grafik` (çubuk, çizgi, pasta), `sekmeler`, `iletişim_kutusu`.
- Paket dizini (`kütüphaneler/dizin.json`, `orhunca paket ara`, `orhunca paket ekle istatistik`),
  resmi paketler: istatistik, geometri; depo alt klasöründen paket.

### 10h – Güvenlik ve dayanıklılık
- Bulanık sınama (`tests/bulanik.rs`); derin/uzun girdi sınırları; derleyici geniş yığında.
- Web sunucusu: statik dosyalarda gizli dosya ve aygıt adı reddi, parçalı gövde taşması, yavaş
  gönderim zaman aşımı, bağlantı ve oturum sınırları. Stüdyo: `..` reddi, başlık sınırı, zaman aşımı.
- Sonsuz özyineleme yerelde ve tarayıcıda yakalanabilir çalışma hatası.

## Aşama 11 – Yayına hazırlık

### 11a – Otomatik güncelleme
- `orhunca güncelle [--denetle]` ve Stüdyo'da yeni sürüm bildirimi: GitHub Releases'ten indirme,
  SHA256SUMS ile doğrulama, kurulum türüne göre (NSIS, .deb, .rpm, AppImage, .dmg, komut) kurma.
  Okullar `ORHUNCA_GUNCELLEME=kapali` ile kapatabilir.

### 11b – ARM
- Cranelift ARM64 kod üretimi; Linux aarch64 sürümü (Raspberry Pi 4/5, ARM dizüstüler; programlar
  sistemdeki C derleyicisiyle bağlanır), CI'da ARM işi.

### 11c – Topluluk
- MIT lisansı, katkı rehberi, davranış kuralları, güvenlik politikası, hata/öneri şablonları,
  yeni README ve ekran görüntüleri.

### 11e – Temalar
- Stüdyo'nun bütün renkleri, yazı tipleri, ölçeği; resim/GIF/video arka plan; `.ohctema` dosyasıyla
  dışa/içe aktarma; `temalar/` topluluk galerisi.

### 11f – Yapay zekâ
- `orhunca mcp`: MCP sunucusu (rehber, denetle, çalıştır, biçimlendir); `llms.txt`, `llms-full.txt`.
- Stüdyo asistanı: Anthropic, OpenAI, Gemini, OpenRouter, Groq, Mistral, DeepSeek, xAI ve yerel
  modeller (Ollama'nın kendi API'si, LM Studio ve OpenAI uyumlu her sunucu); model sağlayıcının
  listesinden seçilir ya da elle yazılır. Asistan kodu kendisi denetleyip çalıştırır, dosya
  değişikliğini öneri olarak verir; araç kullanamayan modellerle düz sohbete geçer.
- "Kendi ajanınızı bağlayın": Claude Code, Codex, Gemini CLI, Cursor, VS Code, Claude Desktop,
  Windsurf, Cline, Continue, Zed, OpenCode için hazır MCP yapılandırması; projeye AGENTS.md.
  Masaüstü uygulaması da `orhunca-studyo mcp` olarak MCP sunucusu çalıştırır.
  Okullar `ORHUNCA_YAPAY_ZEKA=kapali` ile kapatabilir. Bkz. [docs/yapay-zeka.md](docs/yapay-zeka.md).

## Aşama 12 – Telefon, çeviri, oyun

### 12a – Android
- `orhunca paketle --hedef android`: hazır WebView kabuğu (mobil/android → runtime/kabuk/android.okb),
  ikili manifestte ad/kimlik/sürüm değişimi, APK İmza Şeması v2 (ECDSA P-256). SDK gerekmez.
  CI'da gerçek emülatörde açılıp düğmesine dokunularak sınanır.

### 12b – iOS
- `orhunca paketle --hedef ios`: WKWebView'lu Xcode projesi (mobil/ios), GitHub'da imzasız .ipa
  üreten iş akışı. CI'da simülatörde açılır.

### 12c – Telefon komutları
- `titret`, `paylaş`, `bildirim_gönder` (Android/iOS köprüsü; tarayıcıda Web API'leri).

### 12d – Başka dillerde
- `orhunca çevir --dil python|javascript` ve Stüdyo'da yan yana görünüm (satır satır eşleşmiş).
  Örneklerin Python/JS karşılıkları çalıştırılıp çıktıları sınanır.

### 12e – 2B oyunlar
- `oyun_alanı(g, y) her_karede:` (requestAnimationFrame), çizim (`temizle`, `dikdörtgen`, `daire`,
  `çizgi`, `yazı_çiz`, `resim_çiz`), giriş (`tuş_basılı`, `fare_x/y`, `fare_basılı`; dokunma),
  `ses`. Stüdyo'da "2B Oyun" şablonu; örnek: örnekler/oyunlar/top_yakala.ohc.

## Henüz yapılmayanlar

- Çalışma zamanının çekirdeği (bellek ve çöp toplayıcı, listeler, sözlükler, dosyalar, ağ, JSON,
  modeller) C ile yazılı (`runtime/orhunca_rt.c`); metin işlemleri Orhunca'dadır.
- Web sunucusu yolları tek iş parçacığında sırayla çalıştırır (bağlantılar olay döngüsünde
  eşzamanlıdır); oturumlar sunucunun belleğindedir (yeniden başlatınca silinir).
- Ünlü uyumu derleyicide hoşgörüyle kabul edilir; biçimlendirici ve dil sunucusu uyarır ve düzeltir.
  Metinlerde (yabancı kelimeler olabileceği için) yalnızca tampon harf ve ünsüz benzeşmesi denetlenir.
- Listelerde indeks 0'dan başlar.

## Sıradaki adımlar (öneri sırası)

1. Öz-barındırma: tip denetçisinin Orhunca'ya taşınması (`öz/BENİOKU.md`).
2. Hata ayıklayıcıda koşullu kesme noktaları ve değişkenlerin ağaç görünümü.
3. macOS için hazır çalıştırıcı (şimdilik macOS'ta derleme sistemdeki C derleyicisiyle yapılır).
4. Kurulum dosyalarının imzalanması (Windows SmartScreen ve macOS Gatekeeper uyarıları için).
5. Döngüde metin büyütmenin (`m = m + ...`) hızlandırılması.

## Çalışma şekli

- Her aşama için ayrı kısa oturum; önce bu plan, sonra net görev.
- Dil tasarımı, ayrıştırıcı, ek çözümleyici ve kod üretimi için yüksek efor; rutin işler için düşük.
- Her değişiklik `cargo test` ile doğrulanır; yeni dil özelliği için `örnekler/` klasörüne `.ohc` + `.beklenen` çifti eklenir.
