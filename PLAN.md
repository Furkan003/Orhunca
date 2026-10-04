# Orhunca — Plan

Kararların gerekçesi için: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).

## Durum (v0.2)

| Aşama | İçerik | Durum |
|---|---|---|
| 0 | Depo, sohbet özeti, plan | ✅ |
| 1 | Sözcük çözümleyici + ayrıştırıcı (Türkçe söz dizimi, hâl ekleri) | ✅ |
| 2 | Değişkenler, `eğer`, döngüler, işlevler, ekrana yazma, listeler, metinler | ✅ |
| 3 | Cranelift ile çalıştırılabilir dosya: Linux ✅, Windows `.exe` (çapraz derleme, mingw) ✅ | ✅ |
| 4 | Dil sunucusu (LSP) + VS Code eklentisi | ⏳ |
| 5 | Standart kütüphane ✅, paket yöneticisi ⏳ | 🟡 |
| 6 | Web sunucusu, `.ohchtml`, veritabanı/ORM | ⏳ |
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

## Henüz yapılmayanlar

- **Çalışma zamanı C ile yazılı** (`runtime/orhunca_rt.c`). Bağlama zaten bir C araç zinciri
  istediğinden pratik bir seçim; self-hosting aşamasında Orhunca ile yeniden yazılabilir.
- **Modeller/yapılar yok** (`model Ürün:`).
- **Hata yakalama yok** (`dene / yakala`): üretilen kodun yığıtını güvenle geri sarmak için
  çalışma zamanına destek gerekiyor.
- **Ünlü uyumu denetlenmiyor**, yalnızca kabul ediliyor. Biçimlendirici (`orhunca biçimlendir`)
  yanlış ekleri düzeltmeli (`5'a` → `5'e`).
- Fiil parametrelerinin tipi çağrılardan çıkarılmıyor; `sayı` dışındaki tipler yazılmalı.
- Listelerde indeks 0'dan başlar.

## Sıradaki adımlar (öneri sırası)

1. **Dil sunucusu (LSP) + VS Code eklentisi** (Aşama 4): Stüdyo'nun denetim altyapısı hazır.
2. **Modeller** (`model Ürün:` alanlarla) — web çatısının ön koşulu.
3. **Biçimlendirici** — ünlü uyumu düzeltmesi, girinti.
4. **Dil sunucusu (LSP)**: `orhunca denetle` zaten hata konumlarını veriyor; `tower-lsp` ile tanı,
   tamamlama, üzerine gelince tip.

## Çalışma şekli

- Her aşama için ayrı kısa oturum; önce bu plan, sonra net görev.
- Dil tasarımı, ayrıştırıcı, ek çözümleyici ve kod üretimi için yüksek efor; rutin işler için düşük.
- Her değişiklik `cargo test` ile doğrulanır; yeni dil özelliği için `örnekler/` klasörüne `.ohc` + `.beklenen` çifti eklenir.
