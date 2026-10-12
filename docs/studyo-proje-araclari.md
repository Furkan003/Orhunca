# Stüdyo proje araçları

**Proje** menüsü kurtarma, sağlık raporu, hedef profilleri, API senaryoları,
cihaz günlükleri, veritabanı geçişleri ve demo/teslim ekranlarını açar.
Kod çalıştıran işlemler projenin güvenilir olarak işaretlenmesini gerektirir.

## Kurtarma ve yedekler

Değişmiş sekmeler, otomatik kayıt kapalı olsa da yazmaya ara verildikten sonra
Stüdyo'nun özel ayar deposuna gönderilir. Sekmelerin kurtarılması diski hemen
değiştirmez; dış değişiklik varsa kaydetme öncesinde karşılaştırma açılır.
Taslaklar pencerelere özel kimlik ve sıralama kullanır; geciken eski istek yeni
içeriği ezmez. Ağ kopuksa kurtarma kaydı yazılamaz ve uyarı gösterilir.

Proje açılışında ve 10 dakikalık aralıklarla yedek alınır. **Şimdi yedek al**
manuel kopya oluşturur; **Kopyada dene** yeni `kurtarilan-…` klasörüne yükler.
Özgün dosyalar ezilmez. Yedek doğrulanmadan kurtarma yapılmaz. Yedekler en fazla
512 MiB / 10000 dosya, proje yedek deposu toplam 2 GiB olabilir. Sınırda eski
yedeklerin kullanıcı tarafından temizlenmesi gerekir; otomatik silme yapılmaz.
Git, paketler, derleme çıktıları ve önceki kurtarma kopyaları proje yedeğine alınmaz.
`.env` yedekte özel depoda korunur; teslim kopyasında korunmaz.

**Dosya → Silinenleri geri al** dosya ve klasörleri özgün yerlerine getirir.
Boyutlar gösterilir; **Kalıcı sil** onaydan sonra kurtarma kaydını kaldırır.
Farklı diske silme sırasında önce kaynak aynı diskte ayrılır; başarısız kopya
geri alınır. Kesintide ara yol kayıt bilgisine yazılmıştır. Mevcut hedef ezilmez.

Yerel geçmiş farklı hızlı kayıtları birleştirmez. Son 100 sürüm / 30 gün tutulur;
1 MiB üzerindeki dosyaların geçmişi tutulamaz. Geçmiş hatası asıl kaydı bozmaz,
uyarı olarak bildirilir.

## Hedef profilleri

Ayarlar `studyo.ayar.json` içinde saklanır. Ekrandaki **Profilleri ve API
koleksiyonunu düzenle** bölümünden düzenlenir; kayıt sürüm kontrolü kullanır.
İlk açılışta geliştirme, sınama ve teslim profilleri önerilir.

```json
{
  "profiller": [
    {
      "ad": "web",
      "hedef": "web",
      "kip": "teslim",
      "acik_ortam": { "API_ADRESI": "${API_ADRESI}" },
      "ortam": {}
    },
    {
      "ad": "sunucu",
      "hedef": "linux",
      "kip": "sınama",
      "ortam": { "API_TOKEN": "${API_TOKEN}" }
    },
    {
      "ad": "android",
      "hedef": "android",
      "kip": "teslim",
      "acik_ortam": { "API_ADRESI": "${API_ADRESI}" },
      "ortam": {}
    }
  ],
  "senaryolar": []
}
```

`.env` veya işletim sistemi ortamında `API_ADRESI` ve `API_TOKEN` tanımlanır.
Sistemde tanımlı değerler `.env` değerlerinden önce gelir. Gizli değerleri JSON'a
düz metin olarak yazmayın. `${DEGISKEN}` çalışma anında çözülür.

`acik_ortam` **istemciye teslim edilen** değerlerdir. Web/arayüz HTML'sine gömülür;
Android/iOS paketleri bu HTML'yi kullanır. Native teslim klasöründe açık `.env`
oluşturulur. Açık ayarlar gizli anahtar adına veya gizli değişkene başvuramaz.
Script etiketi içeren değerler HTML'ye güvenli biçimde kodlanır. Program bunları
`ortam("API_ADRESI")` ile okur. Kullanıcı oturum anahtarı uygulama çalışırken
alınmalıdır; derleme ayarına konmamalıdır.

`ortam` sunucuda/yerel çalıştırmada kullanılan değerlerdir; gerçek anahtarlar
teslim paketine eklenmez. **Profil ortamıyla yerelde çalıştır** bu ayarları
uygular. WebAssembly önizlemesi yalnızca `acik_ortam` değerlerini alır.

| Hedef | Çıktı |
| --- | --- |
| `web` | Tarayıcı için tek HTML + WebAssembly; native web sunucusu değildir. |
| `linux` / `windows` | Native program; arayüz paketlemek için `masaustu: true`. |
| `macos` | macOS uygulaması; macOS araçları gerekir. |
| `android` | APK; çalıştırma/günlük için `adb` gerekir. |
| `ios` | Xcode projesi; derleme/kurulum için macOS, Xcode ve ilgili imzalama gerekir. |

Sınama ve teslim kipleri kopyalanmış kaynakta sınamaları çalıştırır. Başarısız
sınama teslimi engeller; sıfır sınama **sınama yok** olarak raporlanır.
Teslim klasörü özel depoda oluşturulur; kaynak özetleri, kilitli paketler,
hedef/profil, araç sürümleri, Git işlemesi, kaydedilmemiş Git değişikliği bilgisi,
sınama sonucu ve `KURULUM.txt` içerir. Kaynak .env, veri ve özel anahtar dosyaları
paylaşılmaz. Bilinen gizli ortam değerleri veya özel anahtar içeren kaynak
saptanırsa teslim durur. Bu denetim bilinmeyen her anahtarı bulacağı anlamına gelmez;
paylaşmadan önce kaynaklar gözden geçirilmelidir.

Web/mobil çıktısı aynı açık ayarlarla yeniden üretilecekse `teslim.json` içindeki
`acik_ortam` nesnesi `ORHUNCA_ACIK_ORTAM_JSON` olarak derleme ortamına verilir.
Kaynak özetleri tekrar üretimi değerlendirmek içindir; farklı klasör, araç ve
platformlarda ikililerin bit düzeyinde aynı olacağı iddia edilmez.

## Sağlık raporu ve günlükler

Seçilen hedefe göre araçların varlığı, kurulu/eksik paketler, son derleme,
son başarılı derleme ve son sınama görülür. Çalıştırılmamış sınamalar başarılı
sayılmaz. Rapor fiziksel cihazın çalıştığını kanıtlamaz.

**Cihaz ve günlük** ekranı Android cihaz/emülatörlerini veya macOS üzerinde
iOS simülatörlerini listeler. Kimlik ve paket yolu verilerek kurma, başlatma,
durdurma ve günlük alma yapılır. iOS için derlenmiş `.app` gerekir; simülatör
açma da vardır. Canlı günlükler iki saniyede bir alınır ve panel kapanınca durur.
**Bu bilgisayar** seçeneği mevcut native çalıştırıcı/hata ayıklayıcıyı ve
terminal günlüklerini kullanır. Fiziksel iPhone kurulumu Xcode cihaz araçlarında yapılır.

Bilinen gizli ortam değerleri native programın akış parçaları arasında bölünse
de maskelenir. API başlıkları, gövde ve adresi özel geçici dosyalarda taşınır;
anahtarlar komut satırı argümanına konmaz. Maskeleme bilinmeyen, uygulamanın yeni
ürettiği her kimlik bilgisini otomatik tanımaz.

## API senaryoları

Her senaryo `ad`, `url`, `yontem`, `basliklar`, `govde`, `beklenen` HTTP kodu ve
isteğe bağlı `icerir` yanıt metni taşır. Kimlik başlıkları ortam değişkenine başvurur.

```json
{
  "ad": "Yetkili istek",
  "url": "${API_ADRESI}/api/ozel",
  "yontem": "GET",
  "basliklar": { "Authorization": "Bearer ${API_TOKEN}" },
  "beklenen": 200,
  "icerir": "başarılı"
}
```

Yetki reddi için başlıksız/yanlış kimlikli ayrı senaryoda `401` veya `403` bekleyin.
`kip: "cevrimdisi"` ve `beklenen: 0` yerel bağlantının yanıt vermeden kapanmasını;
`kip: "hatali-yanit"` ve `beklenen: 503` yerel hata sunucusunu kullanır. Bunlar
gerçek HTTP taşımasını deneyen benzetimlerdir; gerçek uzak servis veya cihaz
başarısı olarak raporlanmaz. Sonuç JSON olarak indirilebilir.

## SQLite geçişleri ve demo

Geçiş paneli SQLite içindir. Native backend için `.env` içinde
`ORHUNCA_VERITABANI=sqlite` seçin. JSON model deposu veya WebAssembly'de SQL
çalıştırma özelliğiyle karıştırılmamalıdır.

`gecisler/001_urun.sql` ileri, `001_urun.geri.sql` geri geçiştir. Önizleme mevcut
şemayı ve derlenebilen proje modellerinin eklenen/kaldırılan alanlarını gösterir.
DDL otomatik üretilmez; geçiş SQL'i geliştirici tarafından yazılır.

```sql
-- 001_urun.sql
CREATE TABLE Urun (kimlik INTEGER PRIMARY KEY, ad TEXT);
```

```sql
-- 001_urun.geri.sql
DROP TABLE Urun;
```

Önizleme şemayı değiştirmez. Uygulamadan önce `VACUUM INTO` ile tutarlı yedek
alınır; geçişler ve `orhunca_gecisler` defteri tek SQLite işleminde yazılır.
Hatalı SQL bütün işlemi geri alır. Uygulanmış dosyanın özeti değişirse, dosya
silinirse veya önizleme eskirse ilerlenmez. Geri alma son uygulanmış geçişin
`.geri.sql` dosyasını kullanır. Geçiş dosyaları işlem yönetimi içeremez.
Harici SQL motorları bu panelde desteklenmez; kendi geçiş araçları gerekir.

**Temiz demo oluştur** özgün projeden ayrı klasör oluşturur. Kişisel veri/anahtar
dosyaları çıkarılır; `demo/veri` içindeki örnek veri yüklenir. Yeniden oluşturma
temiz kopya verir. WebAssembly HTML çıktısı çevrimdışı sunulabilir; gerçek uzak
API/veritabanı kullanan akışlar için ayrıca benzetim ve yerel örnek veri seçilmelidir.

Önizleme başlığındaki Telefon/Tablet/Masaüstü düğmeleri ekran genişliğini değiştirir.
Odak işaretleri, iletişim kutusu etiketleri ve Tab odağının kutuda tutulması eklendi.
