# Orhunca — Plan

Kararların gerekçesi için: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).

## Durum (v0.4)

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
| 8 | WebAssembly ✅ (`--hedef web`), Türkçe arayüz dili ✅, self-hosting ⏳ | ⏳ |

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

## Henüz yapılmayanlar

- **Çalışma zamanı C ile yazılı** (`runtime/orhunca_rt.c`). Bağlama zaten bir C araç zinciri
  istediğinden pratik bir seçim; self-hosting aşamasında Orhunca ile yeniden yazılabilir.
- Model alanları başka bir model olamaz (ilişkiler kimlik alanıyla kurulur: `yazar_kimliği: sayı`).
- Web sunucusu tek iş parçacıklıdır; oturum/çerez, dosya yükleme ve HTTPS yok (geliştirme için).
- **Hata yakalama yok** (`dene / yakala`): üretilen kodun yığıtını güvenle geri sarmak için
  çalışma zamanına destek gerekiyor.
- Ünlü uyumu derleyicide hoşgörüyle kabul edilir; biçimlendirici ve dil sunucusu uyarır ve düzeltir.
  Metinlerde (yabancı kelimeler olabileceği için) yalnızca tampon harf ve ünsüz benzeşmesi denetlenir.
- Fiil parametrelerinin tipi çağrılardan çıkarılmıyor; `sayı` dışındaki tipler yazılmalı.
- Listelerde indeks 0'dan başlar.

## Sıradaki adımlar (öneri sırası)

1. **Aşama 8**: self-hosting ilk adımı (Orhunca ile yazılmış sözcük çözümleyici).
2. Windows'ta bağlayıcı gereksinimini kaldırmak (MinGW yerine hazır bağlayıcı ve çalışma zamanı).

## Çalışma şekli

- Her aşama için ayrı kısa oturum; önce bu plan, sonra net görev.
- Dil tasarımı, ayrıştırıcı, ek çözümleyici ve kod üretimi için yüksek efor; rutin işler için düşük.
- Her değişiklik `cargo test` ile doğrulanır; yeni dil özelliği için `örnekler/` klasörüne `.ohc` + `.beklenen` çifti eklenir.
