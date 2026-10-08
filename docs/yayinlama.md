# Web programını sunucuya yayınlamak

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

## 1. Sunucu kiralamak

Herhangi bir sağlayıcıdan (Hetzner, DigitalOcean, Vultr, Natro VPS, Turhost VPS…) şu özelliklerde
bir sunucu yeterlidir:

- **İşletim sistemi:** Ubuntu 22.04/24.04 ya da Debian 12
- **İşlemci:** x86_64 (amd64). ARM sunucular şimdilik desteklenmiyor (aşağıya bakın).
- **Bellek:** 512 MB – 1 GB yeter; Orhunca programları çok az bellek kullanır.

Sağlayıcı size bir IP adresi ve `root` parolası (ya da SSH anahtarı) verir.

> Paylaşımlı hosting (cPanel/Plesk; yalnızca dosya yükleyebildiğiniz paketler) VPS değildir; orada
> `orhunca yayınla` çalışmaz.

## 2. SSH bağlantısı

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

## 3. Alan adı (isteğe bağlı)

Alan adınızın yönetim panelinde bir **A kaydı** ekleyin: ad `@` (ya da `www`), değer sunucunun IP
adresi. Kaydın yayılması birkaç dakika sürebilir. Sonra:

```sh
orhunca yayınla kok@203.0.113.5 --alan ornek.com
```

Sunucuya [Caddy](https://caddyserver.com) kurulur; gelen istekleri programınıza iletir ve Let's
Encrypt sertifikasını kendiliğinden alıp yeniler. Program yalnızca sunucunun içinden erişilebilir
(127.0.0.1), dışarıya Caddy açılır (80 ve 443).

Alan adı vermezseniz program doğrudan `http://203.0.113.5:3000` adresinde yayınlanır (HTTPS'siz).

## Seçenekler

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

## Sunucudaki ayarlar (.env)

Bilgisayarınızdaki `.env` geliştirme ayarlarıdır ve sunucuya **gönderilmez**. Sunucuda kullanılacak
ayarları (veritabanı parolası, API anahtarları) projede `.env.sunucu` dosyasına yazın; yayında
sunucuya `.env` olarak kopyalanır ve yalnızca programın okuyabileceği şekilde korunur.

```
# .env.sunucu
YONETICI_PAROLASI=uzun-ve-gizli-bir-parola
```

`.env.sunucu` dosyasını Git'e eklemeyin (`.gitignore`).

## Sunucuda ne olur?

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

## Kendiniz kurmak

Sunucu vermeden çalıştırırsanız yalnızca yayın klasörü hazırlanır:

```sh
orhunca yayınla --alan ornek.com
```

`cikti/yayın/` klasöründe Linux programı, `statik/`, hizmet dosyası ve `kur.sh` bulunur. Klasörü
sunucuya istediğiniz yolla (FileZilla/WinSCP ile SFTP, `scp`) kopyalayıp sunucuda çalıştırın:

```sh
sudo sh kur.sh
```

## Sorunlar

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

## Ortam değişkenleri

Hizmet dosyası programa şunları verir; `.env.sunucu` ile değiştirilemez:

| Değişken | Değer |
|---|---|
| `ORHUNCA_KAPI` | `--kapı` (programdaki kapının yerine geçer) |
| `ORHUNCA_ADRES` | Alan adıyla `127.0.0.1`, alan adı olmadan `0.0.0.0` |
