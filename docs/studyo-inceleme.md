# Stüdyo güvenilirlik incelemesi ve teslim durumu

12 Ekim 2026 · Taban: `dbc7174b19ac880e45cebef185e865a1d559c9f5`
Dal: `codex/guvenilirlik` · Hedef: web + mobil veya web + masaüstü bitirme projesi.
Sürüm numarası 1.2.0'dır; birleştirme ve sürüm yayını bu çalışma kapsamında yapılmaz.

Bu rapor doğrulanan düzeltmeleri, eklenen özellikleri ve çalıştırılamayan kontrolleri
listeler. Bilinmeyen bütün güvenlik açıklarının yokluğu iddia edilmez.
Kullanım ve profil örnekleri: [Stüdyo proje araçları](studyo-proje-araclari.md).

## İlk yedi düzeltme ve açık kalan 1–4

| Bulgu | Durum ve davranış |
| --- | --- |
| Kopuk bağlantıyla proje dışına yazma | Çözülemeyen mevcut bağlantı yeni dosya sayılmaz; erişim reddedilir. |
| Klasör kopyasıyla dış bağlantıları okuma | Bağlantı/özel dosya reddedilir. Kopya geçici hedefte hazırlanır; hata hedefte yarım klasör bırakmaz. |
| Proje ağacı ve yapılacak notlarında dış dosyalar | Alt girdilerde sınır kontrolü; içerik okumalarında tanıtıcıyla sınırlı okuma. |
| Sekme değişince kaçırılan otomatik kayıt | Değişen bütün açık sekmeler kaydedilir; eski proje zamanlayıcısı yeni projeye uygulanmaz. |
| Eşzamanlı kayıt | Sekme istekleri sıralanır; sunucuda süreçler arası kilit ve sürüm kontrolü uygulanır. |
| Bekleyen disk yanıtının yeni kodu ezmesi | Yanıt sonrası proje, sekme, içerik, yol ve kayıt durumu yeniden doğrulanır. |
| Yanıltıcı silme/geri alma sözü | Gerçek dosya/klasör kurtarma deposu ve geri alma ekranı vardır. |
| 1: `eşit_olmalı` yanlış başarı | Metin biçimi yerine tipli karşılaştırma; liste/sözlük/model alanları karşılaştırılır. Sözlük sırası önemli değildir. Döngüsel model çiftleri takip edilir; aynı modeldeki NaN da eşit sayılmaz. |
| 2: başka editörün değişikliğini ezme | SHA-256 sürümü gerekir. Eski sürüm 409, eksik sürüm 428 alır. Disk/yerel karşılaştırma, düzenlenebilir birleşim ve açıkça üzerine yazma vardır. |
| 3: yarım kayıt | Aynı klasörde geçici yazma, fsync ve atomik değiştirme; dosya izinleri korunur. Bul/değiştir ve yeniden adlandırma da aynı kayıt yolunu kullanır. |
| 4: silinen klasörün geri alınamaması | İkili dosyalar dahil kurtarma ve yeniden başlatma sonrası listeleme vardır. Mevcut hedef ezilmez. |

## Sonraki güvenilirlik maddeleri

| Rapor maddesi | Sonuç |
| --- | --- |
| Farklı dosya sistemine kurtarma taşıması | Aynı diskte ara taşıma, doğrulanmış kopya ve hata geri alma eklendi. Ara yol kesinti sonrası kayıtta bulunur. İki farklı dosya sisteminde 2 MiB ikili klasörün silme/geri alma denemesi geçti. |
| Kurtarma deposunun boyut/temizleme yönetimi | Silinenlerin boyutları ve onaylı kalıcı temizleme; yedekleri silme ve 2 GiB yedek kotası. Eski yedekler kullanıcı haberi olmadan silinmez. |
| Klasör kopyasında yarım hedef | Hedef yalnız tamamlanan kopyadan yayımlanır; başarısız kopya kaldırılır. |
| Denetim ile işlem arasında yol değişmesi | Proje köklerinin dizin tanıtıcıları tutulur. Okuma, kayıt, kopya ve yeni öğe oluşturma tanıtıcılardan yapılır. Linux'ta taşıma/geri alma hedefi ezmeme koşulu `RENAME_NOREPLACE` ile atomiktir. |
| Döngüsel model eşitliği | Ziyaret edilen model çiftleri ile sonlandırma; null model karşılaştırması; döngüsel hata mesajında grafiği tekrar yazdırmama. Çok derin karşılaştırmanın 512 düzey koruması kalır. |
| Döngüsel JSON'un yığın taşırması | JSON yazımı 200 düzeyde anlaşılır hata verir; bozuk veri dosyası yazılmaz. |
| Yerel geçmişin hızlı sürümleri birleştirmesi | Her farklı sürüm saklanır; hata kayıt yanıtında uyarıdır. Son 100 / 30 gün sınırı korunur. |
| Dosya sistemi/donanım güvencesi dışında güç kaybı | Senkronize atomik kayıt ve bağımsız proje yedeği vardır. Gerçek güç kesintisi/donanım denemesi yapılmadı. |

Kilit protokolüne uymayan harici programın son sürüm kontrolü ile atomik değişim
arasındaki yazması bütün işletim sistemlerinde tamamen engellenemez. Dosya sistemi
karşılaştırmalı değiştirme işlemi sunmadığından bunu mutlak garanti olarak
sunmuyoruz. Windows/macOS dizin taşımasında hedef yarışının Linux'a eşdeğer syscall
ile doğrulanması ayrıca gerekir. Kurtarma yedeği korunmalı ve çakışmalar incelenmelidir.

## Doğrulama adayı olarak bırakılmış maddeler

| Aday | Yapılan çalışma |
| --- | --- |
| Silinen JSON kimliğinin yeniden kullanılması | Model kilidi altında kalıcı en yüksek kimlik sayacı. Silmeden önce eski en yüksek değer korunur; bozuk sayaçta kayıt durur. Yeniden başlatma dahil eski kimliğin yeni hesaba verilmemesi sınandı. |
| Gizli/çıktı klasöründeki paket kaynağının taramadan kaçması | Git iç verisi ve kurulum işaretçisi dışındaki kaynaklar taranır. Okunamayan/bağlantılı paket reddedilir. Git iç verisi kaynak olarak yüklenmez. Gizli ve çıktı kaynaklarının izin/özet denemesi geçti. |
| Aynı pakette farklı sürüm isteklerinin sessiz çözülmesi | Kaynak, etiket ve alt klasör birlikte karşılaştırılır. Çakışma hata olur; yeni kilit yazılmaz. Aynı Git işlemesini gösteren iki farklı etiketin de sessizce seçilmemesi sınandı. |
| Tahmin edilebilir geçici klasörler ve sır gizliliği | Güvenli rastgele ad, create-new ve Unix 0700. Asistan ayarları özel klasörde atomik yazılır. Native çıktıda parçalar arasında bölünen bilinen gizli değerler maskelenir. API başlığı/gövdesi/adresi özel dosyalarda taşınır. |
| Bağımlılık duyurularının karşılaştırılmaması | Beş Cargo.lock, güncel RustSec veritabanıyla tarandı. Tekrarlanabilir tarama betiği ve haftalık CI eklendi. |

`glib 0.18.5` için RUSTSEC-2024-0429 bellek güvenliği hatası bulundu. GTK 3
bağımlılıkları yeni GLib sürümüne tek başına taşınamadığından, üst projedeki iki
satırlık düzeltme lisansı korunmuş kaynak kopyasına geri taşındı. Ayrıntı ve
orijinal paket özeti [üçüncü taraf notunda](../ucuncu_taraf/README.md).
Optimizasyonlu 11 iterator testi geçti. Linux hazır pencere kabuğu bu kaynakla
 yeniden üretildi; önceki ve yeni kabuğun glibc gereksinimi 2.34'tür.

Tarama bilinen güvenlik açığı kaydı bildirmedi. `proc-macro-error 1.0.4` için
RUSTSEC-2024-0370 **bakımsızlık uyarısı** devam eder; GTK 3 makro zincirinin
geçişine bağlıdır. Bu uyarı gizlenmedi ve kodun hiç açık içermediği sonucu çıkarılmadı.

## Önerilen özelliklerin uygulanması

| Öneri | Eklenen işlev |
| --- | --- |
| Kaydedilmemiş sekmeleri/çökmeyi kurtarma | Pencerelere özel sıralı taslaklar, başlangıçta kurtarma bildirimi, editörde yeniden açma ve taslak temizleme. |
| Otomatik proje yedeği ve kurtarma ekranı | 10 dakikalık yedek, manuel yedek, SHA-256 doğrulaması, özgün projeyi değiştirmeyen deneme kopyası. |
| Çakışma karşılaştırma/birleştirme | Üç içerikten ayrı satır değişikliklerini birleştirme, örtüşmede elle düzenleme, görülen sürüme bağlı kayıt. |
| Proje sağlık raporu | Hedefe göre araçlar, eksik paketler, son derleme, son başarılı derleme ve son sınama. Sıfır sınama başarılı sınama diye gösterilmez. |
| Geliştirme/sınama/teslim hedef profilleri | Sürümlü JSON ayarı; web/Linux/Windows/macOS/Android/iOS; açık API adresi ile gizli sunucu ortamı ayrı; başarısız sınamada teslim engeli. |
| Cihaz/emülatör/masaüstü günlük ve hata akışı | Android adb ve iOS simctl liste/kur/başlat/durdur/günlük; masaüstü çalıştırıcı/hata ayıklayıcı; canlı günlük ve indirilebilir sonuç. |
| Veritabanı geçişleri | SQLite şema/model alanı önizlemesi, sıralı/özetli geçiş defteri, tutarlı yedek, tek işlemde ileri/geri geçiş. |
| API senaryoları | İstek koleksiyonu, ortamdan kimlik başlıkları, beklenen durum/yanıt; yetki reddi; gerçek yerel HTTP ile çevrimdışı ve 503 benzetimi. |
| Tekrarlanabilir teslim paketi | Kaynak/paket özetleri, kilitli paketler, araç sürümü, Git işlemesi ve değişiklik bilgisi, profil/açık ayarlar, sınama özeti, kurulum yönergesi. |
| Demo profili | Kişisel veri ve anahtar içermeyen ayrı kopya; demo/veri fixture'ları; yeniden temiz kopya ve çevrimdışı HTML/benzetim akışı. |
| Erişilebilirlik ve mobil boyut önizleme | Görünür odak, dialog etiketleri ve Tab odağı; Telefon/Tablet/Masaüstü genişlikleri. |

Hedef profili varlığı o platformun SDK'sının bu makinede bulunduğu anlamına gelmez.
Veritabanı geçiş paneli SQLite içindir; diğer motorların geçiş araçları ve fiziksel
iPhone kurulumu bu panelin dışında kalır. Demo, uzak sunucunun otomatik çevrimdışı
kopyası değildir; örnek veri ve API benzetimi seçilmelidir.

## Denetim kanıtı ve sınırlar

İstenen dört komut çalıştırılır:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- biçimlendir --denetle örnekler öz runtime/ön_kütüphane.ohc
```

Teslim kodu üzerindeki son tam denetimde dört komut başarılıdır. Test koşucusu **203 başarılı,
0 başarısız** bildirdi; Orhunca biçim denetiminde **43 dosya** düzgündü.
PHP 8.4, PDO SQLite ve mbstring kurularak beş PHP testi gerçekten çalıştırıldı.
GLib düzeltmesinde ayrıca **11 test** optimizasyonlu derlemede geçti.

- Gerçek Stüdyo API'si: eski/sürümsüz/eşzamanlı kayıtlar, binary kurtarma,
  yeniden başlatma, mevcut hedefi koruma, traversal/link reddi, iki disk arasında kurtarma.
- Editor modülleri: otomatik/seri kayıt, geciken yanıt, çakışma koruma ve açıkça yazma.
- Proje araçları: yedek kopyada deneme, sıralı taslak, özel ayar reddi, web teslimi,
  model alanı önizlemesi, eskimiş plan, SQL hatasında tüm işlemin geri alınması.
- Web teslim HTML'si gerçek WebAssembly ile Node içinde çalıştırıldı: açık API
  ayarını program okudu; script etiketi içeren değer ek script oluşturmadı.
- Chromium: kurtarma, sağlık, teslim, API, geçiş, demo ve Tab odağı akışları çalıştırıldı.
- Eşitlik: hassas ondalık, koleksiyon/model, döngü/null ve NaN durumları;
  native/WebAssembly/Python/JavaScript üretimi ve bozuk JSON'u engelleme.
- JavaScript sözdizimi ve `git diff --check` denetimleri.

`runtime/orhunca_rt.c` değiştiği için Linux/Windows hazır çalıştırıcıları ve
WebAssembly çalışma zamanı ilgili betiklerle yeniden üretildi. Linux pencere
kabuğu da GLib düzeltmesiyle yeniden üretildi.

**Çalıştırılmayan kontroller:** Windows/macOS üzerinde gerçek uygulama, Android
cihazı ve iOS simülatörü/cihazı, PostgreSQL/MySQL/SQL Server bağlantılı alt senaryolar,
fiziksel güç kesintisi. Linux sanal pencere denemesi yürütme ortamının GUI izin
isteği iptal edildiği için çalıştırılmadı. Normal Cargo PATH'inde xvfb-run etkin olmadığı için pencere kontrolü testin içinden atlandı; Cargo'nun 203 sayısı bu
atlamanın gerçekten yürütüldüğü anlamına gelmez. Native kabuk derlemesi ve glibc
sürümü doğrulandı; pencerenin açıldığı iddia edilmez. Kapsamlı ekran okuyucu denemesi
yapılmadı; eklenen erişilebilirlik mekanizmaları klavye odağıyla denendi.
