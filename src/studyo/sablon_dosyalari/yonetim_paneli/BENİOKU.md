# «ad»

Orhunca ile yazılmış bir yönetim paneli: giriş/kayıt, özet grafikleri, ürün yönetimi (arama,
sayfalama, ekle/düzenle/sil) ve kullanıcı rolleri.

## Çalıştırma

```
orhunca çalıştır
```

Tarayıcıda http://localhost:3000 adresini açın ve **Kayıt ol** ile bir hesap oluşturun; ilk hesap
yönetici olur. Orhunca Stüdyo'da **F5** sunucuyu başlatır ve sayfayı canlı önizlemede gösterir.

## Klasörler

- `sunucu.ohc` — başlangıç: paketleri ve yolları bir araya getirir
- `modeller/ürün.ohc` — Ürün modeli ve kategoriler
- `yollar/` — adresler: `hesap.ohc` (giriş, kayıt, çıkış), `panel.ohc` (özet), `ürünler.ohc`,
  `kullanıcılar.ohc`
- `görünümler/` — `.ohchtml` sayfaları; `panel_düzeni` kenar çubuklu ortak düzendir
- `statik/panel.css` — açık ve koyu tema
- `kütüphaneler/` — kullanılan Orhunca paketleri (kodlarını okuyabilirsiniz):
  `giriş` (üyelik, şifre karması, deneme sınırı), `özet` (PBKDF2, SHA-256), `grafik_svg`
  (grafikler), `sayfala` (sayfalama), `insancıl` ("5 dakika önce")

## Veritabanı

`.env` dosyasındaki `ORHUNCA_VERITABANI=sqlite` satırı kayıtları `veri/orhunca.sqlite`
veritabanına yazar (Kullanıcı ve Ürün tabloları). SQL Server, PostgreSQL ya da MySQL için yalnızca
bu satır değişir; kod aynı kalır:

```
ORHUNCA_VERITABANI=sqlserver://localhost\SQLEXPRESS/panel
ORHUNCA_VERITABANI=postgresql://kullanıcı:şifre@localhost/panel
ORHUNCA_VERITABANI=mysql://kullanıcı:şifre@localhost/panel
```

Veritabanı ve tablolar ilk çalıştırmada oluşturulur. Tabloları SSMS, pgAdmin, MySQL Workbench ya da
DB Browser for SQLite ile açabilir, kodda SQL de yazabilirsiniz:

```
sql_sorgu("SELECT kategori, COUNT(*) AS adet FROM Ürün GROUP BY kategori")
```

## Güvenlik

- Şifreler PBKDF2-SHA256 ile tuzlanıp karma olarak saklanır; şifrenin kendisi saklanmaz.
- Üst üste 5 hatalı denemede hesap 15 dakika kilitlenir.
- Girişte oturum kimliği yenilenir; formlar form anahtarıyla (CSRF) korunur.
- Yalnızca yöneticiler Kullanıcılar sayfasını görür.
