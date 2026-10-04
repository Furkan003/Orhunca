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
| 7 | Orhunca Stüdyo: arayüz ✅ (tarayıcıda, `orhunca stüdyo`), Tauri masaüstü paketi ⏳ | 🟡 |
| 8 | WebAssembly, Türkçe arayüz dili, self-hosting | ⏳ |

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
- Tauri: aynı `studio/` arayüzü pencereye sarılacak; pencere düğmeleri (`__TAURI__`) hazır.

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

1. **Tauri masaüstü paketi** (Aşama 7'nin kalanı): Stüdyo'yu pencereli uygulama olarak paketlemek.
2. **Aşama 8**: WebAssembly çıktısı, Türkçe arayüz dili, self-hosting.

## Çalışma şekli

- Her aşama için ayrı kısa oturum; önce bu plan, sonra net görev.
- Dil tasarımı, ayrıştırıcı, ek çözümleyici ve kod üretimi için yüksek efor; rutin işler için düşük.
- Her değişiklik `cargo test` ile doğrulanır; yeni dil özelliği için `örnekler/` klasörüne `.ohc` + `.beklenen` çifti eklenir.
