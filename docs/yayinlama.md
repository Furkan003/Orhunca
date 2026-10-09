# Web programını sunucuya yayınlamak

Orhunca ile yazılan web siteleri iki yolla internete açılır:

- **[VPS](#1-sunucu-kiralamak)** (kiralık Linux sunucusu): `orhunca yayınla kok@sunucu` tek komutla kurar.
  En hızlısı ve en esnek olanı.
- **[Paylaşımlı hosting](#paylaşımlı-hosting)** (cPanel, Plesk; Natro, Turhost, Hostinger…):
  `orhunca yayınla --cgi` ile hazırlanan klasör dosya yöneticisi ya da FTP ile `public_html`e
  yüklenir. `.ohc` dosyaları PHP gibi de çalışabilir. CGI izni olmayan barındırmalarda program
  `orhunca yayınla --php` ile [PHP'ye çevrilir](#php-ye-çevirmek) ve MySQL ile çalışır.

## VPS

Bilgisayarınızda yazdığınız bir web sitesini ya da API'yi internete açmanın en kolay yolu kiralık bir
Linux sunucusudur (VPS). `orhunca yayınla` programı sizin bilgisayarınızda derler, sunucuya gönderir,
sürekli çalışan bir hizmet olarak kurar ve alan adı verirseniz HTTPS sertifikasını da kendiliğinden
alır. Sunucuya Orhunca kurmanız gerekmez.

[← README](../README.md) · [Web sunucusu](dil-rehberi.md#web-sunucusu)

```sh
orhunca yayınla kok@203.0.113.5 --alan ornek.com
```

```
→ kok@203.0.113.5 sunucusuna gönderiliyor (136 KB)
→ sunucuda kuruluyor
→ urunler kuruluyor: /srv/urunler
✓ Yayında: https://ornek.com
```

Güncellemek için aynı komutu yeniden çalıştırın. Sunucudaki veriler (kayıtlar, `veri/` klasörü,
yüklenen dosyalar) silinmez.

### 1. Sunucu kiralamak

Herhangi bir sağlayıcıdan (Hetzner, DigitalOcean, Vultr, Natro VPS, Turhost VPS…) şu özelliklerde
bir sunucu yeterlidir:

- **İşletim sistemi:** Ubuntu 22.04/24.04 ya da Debian 12
- **İşlemci:** x86_64 (amd64). ARM sunucular şimdilik desteklenmiyor (aşağıya bakın).
- **Bellek:** 512 MB – 1 GB yeter; Orhunca programları çok az bellek kullanır.

Sağlayıcı size bir IP adresi ve `root` parolası (ya da SSH anahtarı) verir.

> Paylaşımlı hosting (cPanel/Plesk; yalnızca dosya yükleyebildiğiniz paketler) VPS değildir; onun
> için [aşağıdaki](#paylaşımlı-hosting) `--cgi` yolunu kullanın.

### 2. SSH bağlantısı

`orhunca yayınla` sunucuya SSH ile bağlanır. Windows 10/11, macOS ve Linux'ta `ssh` komutu hazır
gelir (Windows'ta yoksa: *Ayarlar → Uygulamalar → İsteğe bağlı özellikler → OpenSSH İstemcisi*).
Önce bağlanabildiğinizi deneyin:

```sh
ssh kok@203.0.113.5
```

Her yayında parola sormaması için bir kez SSH anahtarı oluşturup sunucuya ekleyin:

```sh
ssh-keygen -t ed25519            # Enter, Enter, Enter
ssh-copy-id kok@203.0.113.5      # Windows'ta: type $env:USERPROFILE\.ssh\id_ed25519.pub | ssh kok@203.0.113.5 "mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys"
```

`root` dışında bir kullanıcıyla bağlanıyorsanız o kullanıcının `sudo` yetkisi olmalıdır; kurulum
sırasında `sudo` parolası sorulabilir.

### 3. Alan adı (isteğe bağlı)

Alan adınızın yönetim panelinde bir **A kaydı** ekleyin: ad `@` (ya da `www`), değer sunucunun IP
adresi. Kaydın yayılması birkaç dakika sürebilir. Sonra:

```sh
orhunca yayınla kok@203.0.113.5 --alan ornek.com
```

Sunucuya [Caddy](https://caddyserver.com) kurulur; gelen istekleri programınıza iletir ve Let's
Encrypt sertifikasını kendiliğinden alıp yeniler. Program yalnızca sunucunun içinden erişilebilir
(127.0.0.1), dışarıya Caddy açılır (80 ve 443).

Alan adı vermezseniz program doğrudan `http://203.0.113.5:3000` adresinde yayınlanır (HTTPS'siz).

### Seçenekler

```
orhunca yayınla [kullanıcı@sunucu] [--alan ornek.com] [--kapı 3000] [--klasör /srv/ad] [--ssh-kapı 22]
```

| Seçenek | Ne yapar |
|---|---|
| `kullanıcı@sunucu` | Verilmezse yalnızca `cikti/yayın/` klasörü hazırlanır (aşağıya bakın) |
| `--alan` | Alan adı; HTTPS için Caddy kurulur |
| `--kapı` | Programın sunucuda dinlediği kapı (varsayılan 3000). Aynı sunucuda birden çok program yayınlıyorsanız her birine ayrı kapı verin |
| `--klasör` | Sunucudaki kurulum klasörü (varsayılan `/srv/<proje adı>`) |
| `--ssh-kapı` | SSH 22 dışında bir kapıdaysa |

Komut proje klasöründe çalıştırılır; `.ohcproj` dosyasındaki giriş dosyası yayınlanır.

### Sunucudaki ayarlar (.env)

Bilgisayarınızdaki `.env` geliştirme ayarlarıdır ve sunucuya **gönderilmez**. Sunucuda kullanılacak
ayarları (veritabanı parolası, API anahtarları) projede `.env.sunucu` dosyasına yazın; yayında
sunucuya `.env` olarak kopyalanır ve yalnızca programın okuyabileceği şekilde korunur.

```
# .env.sunucu
YONETICI_PAROLASI=uzun-ve-gizli-bir-parola
```

`.env.sunucu` dosyasını Git'e eklemeyin (`.gitignore`).

### Sunucuda ne olur?

| Ne | Nerede |
|---|---|
| Program, `statik/`, `.env` | `/srv/<ad>/` |
| Programın verileri (`veri/`, kayıtlar) | `/srv/<ad>/` — yayında silinmez |
| Hizmet | `/etc/systemd/system/<ad>.service`; sunucu açılınca başlar, çökerse yeniden başlar |
| Alan adı | `/etc/caddy/Caddyfile` içinde `# orhunca: <ad>` bloğu |

Program `orhunca` adlı yetkisiz bir sistem kullanıcısıyla çalışır.

Sık kullanılan komutlar (sunucuda):

```sh
journalctl -u urunler -f          # canlı kayıtlar (istekler, hatalar)
systemctl restart urunler         # yeniden başlat
systemctl stop urunler            # durdur
```

**Yedek:** verileriniz `/srv/<ad>/` klasöründedir. Bilgisayarınıza almak için:
`scp -r kok@203.0.113.5:/srv/urunler/veri ./yedek`

### Kendiniz kurmak

Sunucu vermeden çalıştırırsanız yalnızca yayın klasörü hazırlanır:

```sh
orhunca yayınla --alan ornek.com
```

`cikti/yayın/` klasöründe Linux programı, `statik/`, hizmet dosyası ve `kur.sh` bulunur. Klasörü
sunucuya istediğiniz yolla (FileZilla/WinSCP ile SFTP, `scp`) kopyalayıp sunucuda çalıştırın:

```sh
sudo sh kur.sh
```

### Sorunlar

- **"sunucuya bağlanılamadı"**: `ssh kullanıcı@sunucu` ile bağlanabildiğinizi deneyin. Sağlayıcının
  güvenlik duvarında 22 (SSH) açık olmalı.
- **"Program başlatılamadı"**: betik son kayıtları gösterir. Çoğu zaman kapı başka bir program
  tarafından kullanılıyordur: `--kapı 3001` ile yeniden yayınlayın.
- **Site açılmıyor (alan adıyla)**: A kaydının sunucunun IP'sini gösterdiğini denetleyin
  (`ping ornek.com`). Sağlayıcının güvenlik duvarında 80 ve 443 açık olmalı. Caddy kayıtları:
  `journalctl -u caddy -n 50`.
- **ARM sunucu** (Oracle Ampere, Raspberry Pi): şimdilik program bilgisayarınızda ARM için
  derlenemiyor. Sunucuya Orhunca'yı kurup (`curl -fsSL https://furkan003.github.io/Orhunca/kur.sh | sh`)
  projeyi orada `orhunca derle` ile derleyebilirsiniz.
- **systemd olmayan sunucular** (bazı kapsayıcılar): `kur.sh` durur; programı kendiniz başlatın:
  `ORHUNCA_ADRES=0.0.0.0 ./<ad>`.

### Ortam değişkenleri

Hizmet dosyası programa şunları verir; `.env.sunucu` ile değiştirilemez:

| Değişken | Değer |
|---|---|
| `ORHUNCA_KAPI` | `--kapı` (programdaki kapının yerine geçer) |
| `ORHUNCA_ADRES` | Alan adıyla `127.0.0.1`, alan adı olmadan `0.0.0.0` |

## Paylaşımlı hosting

cPanel, Plesk ya da DirectAdmin kullanan paylaşımlı barındırma paketlerinde (Natro, Turhost,
Güzel Hosting, Hostinger…) program sürekli çalışamaz; bunun yerine her istekte web sunucusu (Apache ya
da LiteSpeed) programı CGI olarak çalıştırır. Orhunca programları bunu kendiliğinden tanır, ayrıca bir
şey yazmanız gerekmez. Veritabanlı (model kayıtları, `veri/` klasörü) ve veritabansız siteler çalışır.

Gereken: Linux barındırma (Windows/IIS barındırma desteklenmez) ve CGI izni; paketlerin çoğunda
açıktır.

### Derlenmiş program (önerilen)

```sh
orhunca yayınla --cgi
```

`cikti/cgi/` klasörü hazırlanır: `uygulama.cgi` (Linux için derlenmiş programınız), `statik/`,
`veri/` ve `.htaccess`. Klasörün **içindekileri** barındırmanın dosya yöneticisiyle ya da FTP
(FileZilla) ile `public_html` klasörüne yükleyin.

- `uygulama.cgi` dosyasının izni **755** olmalı (dosya yöneticisinde sağ tık → *İzinler*). FTP ile
  yüklerken izin bazen kaybolur.
- `.htaccess` gizli bir dosyadır; FileZilla'da *Sunucu → Gizli dosyaları göster* açık olmalı.
- Kodu değiştirince komutu yeniden çalıştırıp yalnızca `uygulama.cgi` dosyasını yeniden yükleyin;
  `veri/` klasörünün üzerine yazmayın (kayıtlar oradadır).
- Sunucuda kullanılacak ayarlar `.env.sunucu` dosyasından `.env` olarak gelir.

`.htaccess` bütün istekleri programa yönlendirir; `statik/` dosyalarını da program sunar. `veri/`,
`.env` ve oturum dosyaları dışarıdan okunamaz. cPanel'in ücretsiz SSL sertifikası (AutoSSL) için
`/.well-known/` adresleri yönlendirilmez.

### .ohc dosyalarını PHP gibi çalıştırmak

```sh
orhunca yayınla --cgi --kaynakla
```

Bu kez `.ohc` dosyalarınız ve sunucuda çalışacak `orhunca.cgi` (Linux'taki Orhunca, ~14 MB)
yüklenir. Bir `.ohc` dosyası ilk istendiğinde derlenir ve `.orhunca-onbellek/` klasöründe saklanır;
dosyayı değiştirince (dosya yöneticisinde düzenleseniz bile) kendiliğinden yeniden derlenir.

```
public_html/
├── .htaccess
├── orhunca.cgi          (izin 755)
├── index.ohc            → https://ornek.com/
├── iletisim.ohc         → https://ornek.com/iletisim.ohc
└── blog/index.ohc       → https://ornek.com/blog/
```

Sayfayı yazdırarak üreten basit bir dosya yeter:

```orhunca
"<!doctype html><h1>Merhaba</h1>"'i yaz.
her i için 1'den 3'e kadar:
    ("<p>" + metin(i) + ". satır</p>")'yi yaz.
```

- Program `yaz` ile yazdıklarını sayfa olarak gönderir (`<` ile başlıyorsa HTML, değilse düz metin).
  Bir çalışma hatası olursa hata sayfası gösterilir.
- Klasörde bir proje (`.ohcproj`) varsa bütün adresler projenin giriş dosyasındaki yollara
  (`al "/ürünler": ...`) gider; `modeller/`, `yollar/` gibi dosyalar tek başına çalıştırılamaz.
  Giriş dosyası yüklenirken `index.ohc` adını alır.
- Bilgisayarınız Windows ya da macOS ise `orhunca.cgi` GitHub'daki son sürümden indirilir.

### PHP'ye çevirmek

CGI'ye izin vermeyen, yalnızca PHP ve MySQL sunan barındırmalarda (ucuz paketlerin çoğu, Windows
olmayan her cPanel) program PHP'ye çevrilir:

```sh
orhunca yayınla --php
```

`cikti/php/` klasörü hazırlanır; **içindekileri** `public_html`e yükleyin:

```
cikti/php/
├── .htaccess            bütün adresleri index.php'ye yönlendirir
├── index.php            programınızın PHP'ye çevrilmiş hâli
├── stil.css …           statik/ klasörünüzün içindekiler
├── orhunca/
│   ├── calisma.php      Orhunca'nın PHP çalışma zamanı
│   └── ayarlar.php      veritabanı ayarları
└── veri/                SQLite veritabanı ve yüklenen dosyalar (dışarıya kapalı)
```

Gereken: PHP 8.1 ya da üstü, `mbstring` ve `pdo_mysql` (ya da `pdo_sqlite`) eklentileri; bunlar
barındırmaların hemen hepsinde açıktır. Derleyici ya da `.cgi` dosyası yüklenmez, izin (755)
ayarlamak gerekmez.

**Veritabanı.** Modeller (`Ürün.hepsi()`, `ü'yü kaydet.`) varsayılan olarak
`veri/orhunca.sqlite` dosyasında saklanır. MySQL kullanmak için barındırma panelinde (cPanel →
*MySQL Veritabanları*) bir veritabanı ve kullanıcı oluşturup `orhunca/ayarlar.php` dosyasına
yazın:

```php
<?php
return [
    'sunucu' => 'localhost',
    'veritabani' => 'kullanici_dukkan',
    'kullanici' => 'kullanici_dukkan',
    'sifre' => '...',
];
```

Her model bir tablo olur (`Ürün` → `Ürün` tablosu, `kimlik` birincil anahtar); tablolar ve sonradan
eklenen alanlar kendiliğinden oluşturulur. Bilgisayarınızdaki kayıtlar (`veri/Ürün.json` gibi)
varsayılan olarak yayına **gitmez**; deneme ya da kişisel verilerin farkında olmadan sunucuya
yüklenmemesi için. Bu kayıtlarla başlamak istiyorsanız `orhunca yayınla --php --veriyle` yazın;
tablo ilk oluşturulduğunda içeri aktarılırlar. (`--cgi` için de `--veriyle` aynı biçimde çalışır.)

**Yeniden yayınlamak.** Kodu değiştirince komutu yeniden çalıştırın ve `index.php` ile
`orhunca/calisma.php` dosyalarını yükleyin. `orhunca/ayarlar.php` ve `veri/` yeniden yayınlamada
korunur; sunucudakilerin üzerine yazmayın.

**Davranış.** Çevrilen program Orhunca'nın kendi sunucusuyla aynı yanıtları verir: yollar, yol
parametreleri, formlar ve doğrulama, JSON, yönlendirme, oturum ve çerezler, dosya yükleme,
görünümler, 404/405/500 sayfaları (500 sayfası hatanın Orhunca satırını gösterir). Sayılar,
bölme, Türkçe sıralama ve metin biçimleri de aynıdır. Farklar:

- Oturumlar PHP'nin oturum düzeniyle saklanır (çerez adı yine `orhunca_oturum`).
- C kütüphanesi çağrıları (`kütüphane` blokları) ve arayüz programları çevrilmez; arayüz programları
  için `orhunca derle --hedef web` kullanılır.
- Program bir alt klasöre (`public_html/site/`) kurulursa yönlendirmeler o klasöre göre yapılır;
  sayfalardaki elle yazılmış `/ürünler` bağlantıları ise yine alan adının köküne gider.

Bilgisayarınızda PHP varsa çeviriyi yüklemeden önce deneyebilirsiniz:

```sh
cd cikti/php
php -S localhost:8000 index.php
```

Konsol programları da çevrilebilir: `php index.php` ile çalışırlar.

### Paylaşımlı hostingde bilinmesi gerekenler

- **Oturumlar** (`istek.oturum`) `veri/.oturumlar/` klasöründe dosya olarak saklanır; 7 gün
  kullanılmayan oturum silinir.
- **Hız:** her istek programı yeniden başlatır (birkaç milisaniye). Küçük ve orta siteler için
  yeterlidir; çok ziyaretçili siteler için VPS önerilir.
- **Alt klasör:** `public_html/site/` gibi bir alt klasöre de kurulabilir; program yolları yine
  `/` ile başlar. Ancak sayfalardaki `/ürünler` gibi bağlantılar alan adının köküne gider; bu yüzden
  uygulamayı kök klasöre ya da bir alt alan adına (`magaza.ornek.com`) kurmak en kolayıdır.
- **Sorun olursa:** 500 hatasında önce `uygulama.cgi`/`orhunca.cgi` izninin 755 olduğunu, sonra
  cPanel'deki *Hatalar* (Errors) sayfasını denetleyin; programın çalışma hataları bu kayıtlara
  düşer.
