# Stüdyo: güvenilirlik incelemesi ve geliştirme sırası

11 Ekim 2026 · Taban: `dbc7174b19ac880e45cebef185e865a1d559c9f5`
Dal: `codex/guvenilirlik` · Hedef: web ve mobil veya web ve masaüstü bitirme projesi.

Bu çalışma kapsamlı bir güvenlik garantisi değildir. Öncelikli incelemede doğrulanan
sorunlar düzeltildi; aşağıda kapsam, doğrulama ve devam işleri ayrılmıştır.

## Tamamlanan düzeltmeler

| Bulgu | Sonuç |
| --- | --- |
| Proje dışına yazan kopuk sembolik bağlantı | Mevcut ama çözülemeyen bağlantı yeni dosya sayılmıyor; erişim reddediliyor. |
| Klasör kopyasında proje dışı bağlantılar | Alt girdiler denetleniyor; bağlantılar ve özel dosyalar reddediliyor. Kendi içine kopyalama takma yollarla da engelleniyor. |
| Proje ağacı ve yapılacak notlarında dış dosyalar | Alt girdilerde de erişim sınırı denetleniyor. |
| Sekme değiştirince kaçırılan otomatik kayıt | Etkin sekme yerine değişmiş açık sekmeler kaydediliyor. Eski proje zamanlayıcısı yeni projeye uygulanmıyor. |
| Eşzamanlı kayıtlar | Aynı sekmede istekler sıraya giriyor; sunucuda kayıt kilidi ve sürüm kontrolü var. |
| Bekleyen disk yanıtının yeni yazılan kodu ezmesi | Yeniden yükleme öncesinde proje, sekme, yol, içerik ve kayıt durumu yeniden doğrulanıyor. |
| Yanıltıcı silme/geri alma sözü | Gerçek kurtarma alanı ve **Dosya → Silinenleri geri al** menüsü eklendi. |
| Açık madde 1: `eşit_olmalı` yanlış başarı | Karşılaştırma artık metne dönüşüme dayanmıyor. Sayılar gerçek değerleriyle; liste, sözlük ve modeller yapısal karşılaştırılıyor. Sözlükte ekleme sırası sonucu değiştirmiyor. |
| Açık madde 2: dış değişikliği ezme | Dosya okumaları SHA-256 sürümü veriyor; kayıt bu sürümü gerektiriyor. Eski sürüm HTTP 409, eksik sürüm 428 alıyor. Editörde iki içerik gösteriliyor; diskten yükleme veya görülen sürümün üstüne açıkça kaydetme seçenekleri var. |
| Açık madde 3: kayıt sırasında dosyanın yarım kalması | İçerik aynı klasörde geçici dosyaya yazılıyor, senkronize ediliyor, ardından tek yeniden adlandırmayla değiştiriliyor. Projede bul/değiştir ve yeniden adlandırma da aynı kayıt yolunu kullanıyor. Yazma başarısızsa eski dosya korunuyor. |
| Açık madde 4: silinen klasörü geri alma | Dosya/klasör, ikili içerik dahil Stüdyo ayar klasöründeki kurtarma alanına taşınıyor. Kayıtlar yeniden başlatmadan sonra da listeleniyor. Geri yükleme mevcut dosyayı ezmiyor. |

## Doğrulama

İstenen denetimler: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `cargo run -- biçimlendir --denetle örnekler öz runtime/ön_kütüphane.ohc`.
Dördü de başarıyla tamamlandı. `cargo test` toplam 194 başarılı, 0 başarısız
sonuç bildirdi; aşağıdaki ortam kaynaklı iç atlamalar bu sayıdan ayrılmalıdır.
Orhunca biçim denetiminde 43 dosya zaten düzgündü. JavaScript sözdizimi denetimleri
ve `git diff --check` de geçti.

Eklenen regresyonlar:

- Gerçek Stüdyo API'sinde eski sürüm, sürümsüz kayıt, silinmiş dosya ve eşzamanlı kayıtlar.
- Kopuk bağlantıyla dışarı yazma, klasör kopyalayarak dışarı okuma ve kendi içine kopyalama.
- 2 MiB ikili dosya içeren klasörün silinmesi, yeniden başlatma sonrası kurtarılması;
  mevcut hedef ve geçersiz kurtarma kimliğinin reddi.
- Atomik kayıt sırasında okuyucunun yalnızca tam eski veya tam yeni içerik görmesi;
  başarısız değiştirmede asıl içeriğin ve geçici dosya temizliğinin korunması.
- Gerçek JavaScript modüllerinde otomatik kayıt, seri kayıt, geciken disk yanıtı,
  çakışmanın korunması ve açıkça üzerine yazma (Node.js senaryoları `cargo test`e bağlı).
- Metinde aynı görünen farklı ondalıkların ve iç içe koleksiyonların eşit sayılmaması;
  eşit koleksiyon/model değerlerinin geçmesi.
- Eşitlik davranışının yerel hedef, WebAssembly, Python ve JavaScript çıktılarında korunması.

`runtime/orhunca_rt.c` değişmedi; yeni eşitlik yardımcıları Rust derleyicisinde sıradan
Orhunca ifadelerine indirgeniyor. Hazır C çalışma zamanı ikililerini yenilemek gerekmedi.

## Sınırlar ve kalan işler

- Gerçek Windows/macOS ve Android/iOS cihaz denemeleri yapılmadı. Tarayıcıda görsel ve
  klavye kullanım testi ayrıca yapılmalı; Node testleri gerçek DOM/WebView testi değildir.
- Test koşucusunun başarılı saydığı bazı kontroller içinden atlayabiliyor: mevcut ortamda
  dört PHP testi ve bir masaüstü pencere testi araç eksikliği nedeniyle çalışmıyor.
  PostgreSQL/MySQL/SQL Server senaryoları bağlantı ayarları olmadığında çalışmıyor.
- Kurtarma alanı Stüdyo ayar klasöründedir; Git'e veya teslim paketine kullanıcı dosyalarının
  kopyasını eklemez. Farklı dosya sisteminden taşıma desteklenmez: silme hata verir ve
  asıl dosya korunur. Kurtarma kayıtları otomatik temizlenmez; boyut/temizleme yönetimi ileride eklenmeli.
- Klasör kopyası bağlantıyla karşılaşırsa hata verir; daha önce kopyalanmış normal dosyalar
  hedefte kalabilir. Bağlantı kontrolü ile dosya işlemi arasında kötü niyetli yerel süreçlerin
  yol değiştirmesine karşı ek işletim sistemi koruması ayrı çalışmadır.
- Sürüm kontrolü ve kilit Stüdyo istemcilerini korur. Kilide uymayan harici programın son
  kontrol ile yeniden adlandırma arasında yazması bütünüyle engellenemez.
- Atomik kayıt mevcut dosyanın izin bitlerini korur. Dosya sistemi/donanımın senkronizasyon
  garantileri dışındaki güç kesintilerine karşı ayrıca yedek gerekir; gerçek güç kesintisi denenmedi.
- Çok derin/döngüsel yapısal eşitlik 128 düzeyden sonra anlaşılır hata verir; yanlış başarı
  vermez. Döngüsel nesne grafiklerinin tam karşılaştırması ayrı bir geliştirmedir.
- Yerel geçmiş hâlâ bir dakika içindeki kayıtları birleştirir ve geçmiş yazma hatalarını
  sessiz geçebilir. Kurtarma alanı bu geçmiş mekanizmasından bağımsızdır.

## Daha fazla doğrulama gerektiren adaylar

Bu maddeler kesinleşmiş güvenlik açığı olarak değerlendirilmemelidir:

- JSON modellerinde en büyük kimlikten sonraki kimliğin üretilmesi ve silinen kullanıcı
  kimliğinin tekrar kullanılması: eski oturum/API anahtarı veya ilişkili kayıtların yeni
  hesaba bağlanıp bağlanmadığı kontrol edilmeli (`runtime/orhunca_rt.c`, hesap şablonları).
- Paket izin/özet denetiminde gizli dosyalar ve bazı çıktı klasörleri dışarıda bırakılıyor.
  Derleyicinin bunlardan içe aktarma yapmasıyla denetim aşılıp aşılmadığı doğrulanmalı.
- Aynı paket kaynağına farklı sürüm etiketleri isteyen bağımlılıklarda sürüm çakışmasının
  sessizce çözülmesi incelenmeli (`src/paket.rs`).
- Tahmin edilebilir geçici klasörler ve varsayılan dosya izinlerinin ortak okul
  bilgisayarlarında asistan konuşması/kaynak kod gizliliğine etkisi incelenmeli.
- Güncel bağımlılık güvenlik duyurularıyla kapsamlı karşılaştırma yapılmadı;
  bir CVE taramasından temiz sonuç alındığı iddia edilmemelidir.

## Önerilen özellikler ve sıra

### 1. Projeyi güvenle yazabilmek

- **Çökmeden kurtarma:** kaydedilmemiş sekmeleri kurtarma, otomatik proje yedeği,
  geri yüklemeyi deneyebileceğin bir kurtarma ekranı.
- **Çakışma çözümü:** iki pencerede veya başka editörde değişen dosyalar için
  karşılaştırma/birleştirme; eski içerikle sessizce ezmeyi engelleme.
- **Proje sağlık raporu:** seçilen hedefe göre araçlar, eksik bağımlılıklar,
  çalıştırılan/atlanmış testler ve son başarılı derleme.

### 2. Web + mobil/masaüstü geliştirmeyi kolaylaştırmak

- **Hedef profilleri:** mevcut paketleme komutlarını geliştirme/test/teslim profillerinde
  toplama; API adresi ve ortam ayarlarını hedef bazında yönetme; gizli değerleri çıktılardan gizleme.
- **Cihazda hata ayıklama akışı:** mobil cihaz/emülatör ve masaüstü için bağlantı,
  canlı günlükler, hata raporu ve yeniden başlatmayı tek yerden yönetme.
- **Veritabanı geçişleri:** model değişikliğini önizleme, sürümlü şema geçişi,
  yedek alma/geri dönme ve ayrı deneme verisi. Mevcut veritabanı panelinin tamamlayıcısı.
- **API senaryoları:** HTTP isteği koleksiyonları, kimlik doğrulama ve yetki reddi
  senaryoları, çevrimdışı/hatalı yanıt denemeleri.

### 3. Bitirme projesini teslim etmek

- **Tekrarlanabilir teslim paketi:** sürüm, kullanılan araçlar, kaynak işlem kimliği,
  kurulum yönergesi ve test özetiyle web/mobil/masaüstü çıktıları.
- **Demo profili:** örnek veriyi güvenli sıfırlama, internet olmadan sunum akışı,
  kişisel anahtar içermeyen paylaşılabilir proje ayarları.
- **Erişilebilirlik kontrolleri:** klavyeyle bütün temel işlemler, görünür odak,
  ekran okuyucu etiketleri ve mobil ekran boyutlarında önizleme.

Git paneli, yerel geçmiş, otomatik kayıt, hata ayıklayıcı, sınamalar ve paketleme zaten var.
Öneriler bunların üzerine güvenilirlik ve hedefler arasında tutarlı kullanım eklemeyi amaçlıyor.

