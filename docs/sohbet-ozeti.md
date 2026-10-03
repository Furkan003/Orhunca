# Orhunca — Sohbet Özeti ve Kararlar

> Tarih: 3 Ekim 2026
> Konu: Türkçe tabanlı bir programlama dili, IDE'si ve web çatısı tasarımı
> Bu belge, sohbetin konu konu özetidir. Yeni bir oturum (ör. bulut) buradan devam edebilir.

---

## Kesinleşen kararlar (özet tablo)

| Parça | Karar |
|---|---|
| Dilin adı | **Orhunca** (Orhun Yazıtları'ndan ilham) |
| Kod dosyası | `.ohc` |
| HTML + kod (cshtml karşılığı) | `.ohchtml` |
| Proje dosyası | `.ohcproj` |
| Komut aracı | `orhunca derle`, `orhunca çalıştır`, `orhunca paket yükle`, `orhunca yeni` |
| IDE | **Orhunca Stüdyo** |
| Derleyicinin yazılacağı dil | **Rust** |
| Makine kodu üreteci | **Cranelift** (Rust ile yazılmış, C++ içermez) |
| İlk hedef çıktı | Windows `.exe` → sonra WebAssembly → Linux/Mac |
| IDE teknolojisi | **Tauri + Monaco** + Rust ile yazılmış dil sunucusu (LSP) |
| GitHub | `orhun-dili` organizasyonu, `orhunca` deposu (`github.com/orhun` başkasına ait) |

---

## 1. Başlangıç sorusu: Mevcut dillerin Türkçe versiyonu yapılabilir mi?

- Teknik olarak evet: Türkçe anahtar kelimeleri asıl dile çeviren bir **çevirici (transpiler)** ile.
- Sorunlar: kütüphaneler İngilizce (on binlerce API), hata mesajları çevrilmiş koda göre gelir, cshtml gibi karma dosyalar zor, her dil ayrı proje, ekosistem İngilizce.
- Sonuç: Mevcut dilleri çevirmek yerine **Türkçe tabanlı yeni bir dil** daha anlamlı.

## 2. Doğrudan Türkçe tabanlı bir dil

Türkçenin farkı: eklerle çalışır, fiil sona gelir. Dil bunu kullanabilir:

```
sayılar = [3, 8, 1]
5'i sayılara ekle.
sayılar'ı sırala.
her sayı için sayılardan:
    eğer sayı 4'ten büyükse:
        sayı'yı ekrana yaz.
```

Hâl ekleri parametrenin rolünü belirler:
- **-i / -ı (belirtme):** işlem yapılan nesne → `5'i`
- **-e / -a (yönelme):** hedef → `sayılara`
- **-den / -dan (ayrılma):** kaynak → `sayılardan`

Böylece parametre sırası önemsizleşir: `5'i sayılara ekle` = `sayılara 5'i ekle`.

## 3. Ek tanıma sorununun çözümü

Kilit fikir: **Derleyici tanımlı isimleri bilir**, genel morfoloji analizine gerek yoktur.

1. **Sözlük temelli çözümleme:** Kelimenin başında en uzun tanımlı ismi bul, kalanı ek tablosunda ara.
2. **Soyut ek tablosu:** `-(y)I`, `-(y)A`, `-DAn`, `-DA`, `-(y)lA` → tüm biçimler otomatik üretilir.
3. **Hoşgörülü ünlü uyumu:** Derleyici her biçimi kabul eder; editör/biçimlendirici düzeltir (`5'e`, `6'ya`, `API'ye` gibi öngörülemeyen durumlar yüzünden şart).
4. **Ünsüz yumuşaması:** Tanım anında varyantlar üretilir (`kitap` → `kitab`, `renk` → `reng`).
5. **Kesme işareti:** Sayılarda/sabitlerde ve belirsiz durumlarda zorunlu, değişkenlerde isteğe bağlı.
6. **Belirsizlikte tahmin yok:** Açık hata verilir (`kitapla` → değişken mi, `kitap'la` mı?).
7. **Sadece son hâl eki anlamlı:** Çoğul/iyelik ismin parçası (`sayılarımız` + `dan`).

Zemberek derleyiciye değil, editöre (öneri ve düzeltme için) konulabilir.

Çözümleyicinin özü:

```js
function çözümle(kelime, sözlük) {
  if (kelime.includes("'")) {
    const [kök, ek] = kelime.split("'");
    return { kök: sözlük.asılİsim(kök) ?? kök, hâl: ekTablosu[ek] ?? hata(ek) };
  }
  if (sözlük.var(kelime)) return { kök: kelime, hâl: null };
  const adaylar = [];
  for (const kök of sözlük.önekleri(kelime)) {
    const ek = kelime.slice(kök.length);
    if (ekTablosu[ek]) adaylar.push({ kök: sözlük.asılİsim(kök), hâl: ekTablosu[ek] });
  }
  if (adaylar.length === 1) return adaylar[0];
  if (adaylar.length > 1) throw belirsizlikHatası(kelime, adaylar);
  throw tanımsızİsimHatası(kelime);
}
```

## 4. Mevcut Türkçe programlama dilleri

- **Kip:** Hâl eklerini ve ünlü uyumunu tip sisteminde kullanan deneysel dil (Haskell). Araştırma amaçlı.
- **tr-lang:** Rust ile yazılmış küçük bir dil.
- GitHub'da çok sayıda bırakılmış hobi projesi.
- Farkında olmadan Türkçe "kod": Türkçe Excel formülleri (`EĞER`, `DÜŞEYARA`), Scratch.
- Sonuç: Ciddi, kapsamlı ve yaygın bir Türkçe dil **yok**, boşluk duruyor.

## 5. Kendi makine kodunu üreten dil (C++ tabanı olmadan)

- 1010'lar elle yazılmaz; **derleyici** Türkçe kodu okuyup makine kodu baytlarını üretir.
- Akış: sözcük çözümleyici → ayrıştırıcı → anlam/tip denetimi → kod üretici → `.exe`
- **Kendi kendini barındırma (self-hosting):** İlk derleyici başka bir dille yazılır, sonra dilin kendisiyle yeniden yazılır (Go, Rust, C bu yolu izledi).
- Kütüphane ekosistemi: paket yöneticisi + paket deposu + iyi standart kütüphane. Topluluk işi.
- C kütüphanelerini çağırabilme (FFI) köprü olarak eklenmeli.

Zorluk tablosu:

| Hedef | Süre (yapay zekâ yardımıyla) |
|---|---|
| Sayılar, değişkenler, `eğer`, döngü, ekrana yazma → `.exe` | Birkaç hafta |
| İşlevler, metinler, listeler, bellek yönetimi | Birkaç ay |
| Self-hosting | Aylar |
| C++ hızında, çok platformlu dil | Yıllar, ekip |

## 6. Rust mı C++ mı?

**Rust** seçildi: bellek güvenliği, `enum` + `match` ile kolay söz dizimi ağaçları, UTF-8 metinler (Türkçe karakterler sorunsuz), `cargo`, ve **Cranelift** sayesinde zincirde hiç C++ yok (LLVM C++ ile yazılmış).

Mimari:

```
dosya.ohc
   ↓
[Sözcük çözümleyici]
[Ayrıştırıcı]           ← Türkçe dilbilgisi, hâl ekleri
[Anlam ve tip denetimi] ← Türkçe hata mesajları
[Ara gösterim]
   ↓
[Cranelift]
   ↓
.exe / .wasm
```

## 7. Web desteği (HTML, CSS, cshtml karşılığı)

Üç yol:
1. **`.ohchtml` (Razor tarzı):** HTML içine Türkçe kod gömülür. Tanıdık ve hızlı.
2. **Tamamen Türkçe arayüz dili:** SwiftUI/Flutter benzeri, HTML/CSS'e derlenir. En özgün yol.
3. **Türkçe etiketli HTML/CSS (birebir çeviri):** Değeri en düşük yol, önerilmedi.

WebAssembly çıktısıyla tarayıcı tarafı da Türkçe yazılabilir, JavaScript'e gerek kalmaz.

### ASP.NET MVC benzeri web çatısı

Proje yapısı:

```
dükkan/
├── ayarlar.ohc
├── başlangıç.ohc
├── modeller/Ürün.ohc
├── denetleyiciler/ÜrünDenetleyici.ohc
├── görünümler/
│   ├── düzen.ohchtml
│   └── ürünler/liste.ohchtml
└── statik/
```

Örnek model ve denetleyici:

```
model Ürün:
    kimlik: sayı, birincil
    ad: metin, zorunlu, en_fazla 100
    fiyat: ondalık, zorunlu
    stok: sayı = 0

denetleyici ÜrünDenetleyici:
    al "/ürünler":
        ürünler = Ürün'den hepsini getir, fiyata göre sırala.
        döndür görünüm("ürünler/liste", ürünler)

    gönder "/ürünler/ekle":
        yeni = form'u Ürün'e dönüştür.
        eğer yeni geçerli değilse:
            döndür görünüm("ürünler/ekle", yeni)
        yeni'yi kaydet.
        döndür yönlendir("/ürünler")
```

Örnek görünüm (`.ohchtml`):

```html
@model liste Ürün
@düzen "düzen"

<h1>Ürünler (@model.sayı adet)</h1>
@eğer model boşsa {
    <p>Henüz ürün eklenmemiş.</p>
}
@değilse {
    <table>
    @her ürün için model'den {
        <tr><td>@ürün.ad</td><td>@ürün.fiyat.para() TL</td></tr>
    }
    </table>
}
```

Çalıştırma: `orhunca çalıştır` → `http://localhost:5000`, hot reload.

Arka plan (Rust): `tokio` + `hyper`/`axum` (sunucu), `rusqlite` (SQLite), `sqlx` (PostgreSQL/MySQL), `tiberius` (SQL Server).

Sonraki özellikler: kullanıcı girişi (`@yetki`), form doğrulama, JSON API, oturum/çerez, dosya yükleme, `orhunca yayınla`.

## 8. IDE: Orhunca Stüdyo

Kullanıcının tasarımı: Visual Studio benzeri başlangıç ekranı (son projeler, arama, hızlı işlemler) + şablon sihirbazı (Masaüstü, Konsol, Kütüphane) + "Öğren" sekmesi.

Öneriler:
- Web şablonları ekle: Web Sitesi (`.ohchtml`), Web API, Tarayıcı Uygulaması (WASM).
- Filtre tutarlılığı (Masaüstü filtresinde Kütüphane görünmemeli).
- Yol gösterimi: gerçek yol `C:\Users\...`, "Kullanıcılar" sadece Gezgin'deki görünen ad.
- "Öğren" sekmesine etkileşimli Türkçe dersler.

Teknoloji: **Tauri + Monaco** (VS Code'un editör bileşeni). Rust ile yazılan dil sunucusu hem Orhunca Stüdyo'da hem VS Code'da çalışır.

```
Orhunca Stüdyo (Tauri + Monaco)     VS Code (eklenti)
              └──────────┬──────────────┘
                 Dil Sunucusu (Rust)
                         ↓
              Derleyici (Rust + Cranelift)
```

## 9. İsim süreci

- Tasarımdaki "Nova" yer tutucuydu (ayrıca Panic'in Nova editörüyle çakışıyor).
- Önerilenler: Orhun, Yalın, Ekin, Çınar, Bilge → **Orhun** seçildi.
- `.orh` / `.orhtml` tuhaf bulundu → **Orhunca** adına geçildi.
- `.orca` reddedildi: yazılımda "Orca" çok kalabalık (Orca Security, GNOME Orca ekran okuyucu, Microsoft Orca, ORCA kuantum kimya, Hundred Rabbits Orca dili).
- Uzantılar: **`.ohc`** seçildi (kısa, `.cs`/`.py`/`.rs` gibi).
- GitHub: `github.com/orhun` ve `orhun.dev` Orhun Parmaksız'a (tanınmış Rust geliştiricisi) ait. Depo adı hesaba bağlı olduğu için `orhunca` deposu açılabilir; organizasyon adı olarak `orhun-dili` önerildi.

## 10. Bütçe ve çalışma şekli

- Bakiye: ~97 dolar. Hedef: **çalışan ilk sürüm** (tamamı değil).
- Efort: varsayılan **orta**; dil tasarımı, ayrıştırıcı, hâl eki çözümleyici ve kod üretimi için **yüksek**; basit işler için **düşük**.
- Tasarruf: her aşama için ayrı kısa oturum, önce plan belgesi, net görevler, rutin işlerde daha ucuz model.
- Otomatik yükleme (auto-reload) hesap ayarlarından kontrol edilmeli.
- Bulut: ortam Linux; Windows `.exe` için çapraz derleme gerekir. Çalışma bir GitHub deposuna gönderilmeli.

---

## Yol haritası

| Aşama | İçerik |
|---|---|
| 0 | GitHub: `orhun-dili/orhunca` deposu, bu belge + `PLAN.md` |
| 1 | Sözcük çözümleyici + ayrıştırıcı (Türkçe söz dizimi, hâl ekleri) |
| 2 | Değişkenler, `eğer`, döngü, işlev, ekrana yazma |
| 3 | Cranelift ile çalışan program dosyası (önce Linux, sonra Windows `.exe`) |
| 4 | Dil sunucusu + VS Code eklentisi |
| 5 | Standart kütüphane, paket yöneticisi |
| 6 | Web sunucusu, `.ohchtml`, veritabanı/ORM |
| 7 | Orhunca Stüdyo (Tauri + Monaco), "Öğren" sekmesi |
| 8 | WebAssembly, Türkçe arayüz dili, self-hosting |

**97 dolarlık bütçe için hedef: Aşama 0–3.**
