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
| 5 | Standart kütüphane, paket yöneticisi | ⏳ |
| 6 | Web sunucusu, `.ohchtml`, veritabanı/ORM | ⏳ |
| 7 | Orhunca Stüdyo (Tauri + Monaco), "Öğren" sekmesi | ⏳ |
| 8 | WebAssembly, Türkçe arayüz dili, self-hosting | ⏳ |

## v0.2'de eklenenler

- **Ondalık sayılar** (`ondalık` tipi, `3.14`). `/` her zaman ondalık verir, `//` tam bölme.
- **Kullanıcı tanımlı fiiller**: `fiil sayı'yı karele:` → `5'i karele.` Parametreler hâl
  ekleriyle eşleşir, çağrıda sıra önemsiz.
- **Otomatik bellek yönetimi**: çalışma zamanında tutucu işaretle-süpür çöp toplayıcı.

## Henüz yapılmayanlar

- **Çalışma zamanı C ile yazılı** (`runtime/orhunca_rt.c`). Bağlama zaten bir C araç zinciri
  istediğinden pratik bir seçim; self-hosting aşamasında Orhunca ile yeniden yazılabilir.
- **Modeller/yapılar yok** (`model Ürün:`), modül/içe aktarma yok.
- **Ünlü uyumu denetlenmiyor**, yalnızca kabul ediliyor. Biçimlendirici (`orhunca biçimlendir`)
  yanlış ekleri düzeltmeli (`5'a` → `5'e`).
- Fiil parametrelerinin tipi çağrılardan çıkarılmıyor; `sayı` dışındaki tipler yazılmalı.
- Listelerde indeks 0'dan başlar.

## Sıradaki adımlar (öneri sırası)

1. **Metin işlemleri** (alt metin, bölme, büyük/küçük harf — Türkçe `i/İ`, `ı/I` kurallarıyla).
2. **Modeller** (`model Ürün:` alanlarla) — web çatısının ön koşulu.
3. **Biçimlendirici** — ünlü uyumu düzeltmesi, girinti.
4. **Dil sunucusu (LSP)**: `orhunca denetle` zaten hata konumlarını veriyor; `tower-lsp` ile tanı,
   tamamlama, üzerine gelince tip.

## Çalışma şekli

- Her aşama için ayrı kısa oturum; önce bu plan, sonra net görev.
- Dil tasarımı, ayrıştırıcı, ek çözümleyici ve kod üretimi için yüksek efor; rutin işler için düşük.
- Her değişiklik `cargo test` ile doğrulanır; yeni dil özelliği için `örnekler/` klasörüne `.ohc` + `.beklenen` çifti eklenir.
