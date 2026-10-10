# «ad»

Kayıt, giriş ve API anahtarıyla korunan bir REST API. Her kullanıcı yalnızca kendi
notlarını görür. Kayıtlar `.env` ayarıyla `veri/orhunca.sqlite` veritabanına yazılır.

## Çalıştırma

```
orhunca çalıştır
```

Sunucu http://localhost:3000 adresinde açılır. Stüdyo'da **F5** de aynı işi yapar.

## Denemek (curl)

```sh
# Kayıt ol: yanıtta API anahtarı döner
curl -X POST localhost:3000/kayıt -d 'ad=Ayşe&e_posta=ayse@örnek.com&şifre=Güçlü.Şifre42'

# Giriş yap (JSON gövde de olur)
curl -X POST localhost:3000/giriş -H 'Content-Type: application/json' \
     -d '{"e_posta": "ayse@örnek.com", "şifre": "Güçlü.Şifre42"}'

A=<yanıttaki anahtar>
curl localhost:3000/ben -H "Authorization: Bearer $A"
curl -X POST localhost:3000/notlar -H "Authorization: Bearer $A" -d 'başlık=Alışveriş&içerik=Süt, ekmek'
curl localhost:3000/notlar -H "Authorization: Bearer $A"
curl -X PUT localhost:3000/notlar/1 -H "Authorization: Bearer $A" -d 'içerik=Süt, ekmek, peynir'
curl -X DELETE localhost:3000/notlar/1 -H "Authorization: Bearer $A"
curl -X POST localhost:3000/çıkış -H "Authorization: Bearer $A"
```

## Uç noktalar

| Yöntem | Adres | Açıklama |
|---|---|---|
| POST | `/kayıt` | ad, e_posta, şifre → kullanıcı ve anahtar |
| POST | `/giriş` | e_posta, şifre → anahtar (30 gün geçerli) |
| GET | `/ben` | giriş yapan kullanıcı |
| POST | `/çıkış` | anahtarı geçersiz kılar |
| GET, POST | `/notlar` | notları listeler, yeni not ekler |
| GET, PUT, DELETE | `/notlar/{kimlik}` | tek not |

## Güvenlik

- Şifreler tuzlu karma (PBKDF2-SHA256) olarak saklanır.
- 5 hatalı denemede hesap 15 dakika kilitlenir.
- API anahtarının kendisi saklanmaz, yalnızca SHA-256 özeti tutulur.
- Gerçek sunucuda HTTPS kullanın; anahtar her istekte gönderilir.
