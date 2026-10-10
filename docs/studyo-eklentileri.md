# Stüdyo eklentileri

Stüdyo'ya kendi komutlarınızı ekleyebilirsiniz. Bir eklenti, ayar klasöründeki bir JavaScript
dosyasıdır:

| Sistem | Klasör |
|---|---|
| Windows | `%APPDATA%\Orhunca\eklentiler\<ad>\eklenti.js` |
| Linux / macOS | `~/.config/orhunca/eklentiler/<ad>/eklenti.js` |

Stüdyo açılırken eklentileri yükler. Eklentinin komutları üst çubuktaki **Eklentiler**
menüsünde görünür. İsteğe bağlı `eklenti.json` dosyasına bir açıklama yazılabilir:
`{"açıklama": "Seçili metni büyük harfe çevirir"}`.

## Örnek

```js
// ~/.config/orhunca/eklentiler/büyük_harf/eklenti.js
OrhuncaStudyo.eklenti({
  ad: 'Büyük harf',
  komutlar: [
    {
      ad: 'Seçimi büyük harfe çevir',
      calistir(api) {
        const s = api.secim();
        if (!s) return api.bildir('Önce bir metin seçin.', true);
        api.seciliyiDegistir(s.toLocaleUpperCase('tr'));
      },
    },
    {
      ad: 'Sınamaları çalıştır',
      calistir(api) { api.kabuk('orhunca sına'); },
    },
  ],
});
```

## Arayüz (`api`)

| İşlev | Ne yapar |
|---|---|
| `etkinDosya()` | Açık dosya: `{ yol, metin }` ya da `null` |
| `secim()` | Seçili metin |
| `seciliyiDegistir(metin)` | Seçimi değiştirir (Ctrl+Z ile geri alınır) |
| `metniDegistir(metin)` | Dosyanın tüm metnini değiştirir |
| `bildir(mesaj, hata)` | Alt köşede bildirim gösterir |
| `kabuk(komut)` | KABUK sekmesinde komut çalıştırır |
| `proje()` | Açık proje: `{ ad, yol }` |
| `api(yol, gövde)` | Stüdyo'nun kendi HTTP API'si, ör. `api('/api/denetle', { dosya })` |

## Güvenlik

Eklentiler Stüdyo'nun içinde, sizin yetkilerinizle çalışır: dosyalarınızı okuyup yazabilir,
kabukta komut çalıştırabilir. Yalnızca kaynağını okuduğunuz ve güvendiğiniz eklentileri kurun.
