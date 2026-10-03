# Orhunca — Plan

Kararların gerekçesi için: [docs/sohbet-ozeti.md](docs/sohbet-ozeti.md).

## Durum (v0.1)

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

## v0.1'de bilinçli olarak yapılmayanlar

- **Bellek geri verilmiyor.** Metin ve listeler `malloc` ile ayrılır, serbest bırakılmaz. Kısa programlar için sorun değil; ileride referans sayımı ya da bölge tabanlı bellek.
- **Çalışma zamanı C ile yazılı** (`runtime/orhunca_rt.c`, ~250 satır). Bağlama zaten bir C araç zinciri istediğinden pratik bir seçim; self-hosting aşamasında Orhunca ile yeniden yazılabilir.
- **Ondalık sayılar yok** (`ondalık` tipi). Cranelift `f64` destekler; eklemesi kolay.
- **Kullanıcı tanımlı fiiller yok.** İşlevler şimdilik `topla(2, 3)` biçiminde çağrılır. Hedef: `işlev (sayı'yı) karele:` gibi hâl ekli parametre tanımları ve `5'i karele.` çağrısı.
- **Modeller/yapılar yok** (`model Ürün:`), modül/içe aktarma yok.
- **Ünlü uyumu denetlenmiyor**, yalnızca kabul ediliyor. Biçimlendirici (`orhunca biçimlendir`) yanlış ekleri düzeltmeli (`5'a` → `5'e`).
- Listelerde indeks 0'dan başlar.

## Sıradaki adımlar (öneri sırası)

1. **Ondalık sayılar** ve `metin` işlemleri (alt metin, bölme, büyük/küçük harf — Türkçe `i/İ`, `ı/I` kurallarıyla).
2. **Kullanıcı tanımlı fiiller** (hâl ekli parametreler) — dilin en özgün özelliği.
3. **Modeller** (`model Ürün:` alanlarla) — web çatısının ön koşulu.
4. **Biçimlendirici** — ünlü uyumu düzeltmesi, girinti.
5. **Dil sunucusu (LSP)**: `orhunca denetle` zaten hata konumlarını veriyor; `tower-lsp` ile tanı, tamamlama, üzerine gelince tip.
6. Bellek yönetimi.

## Çalışma şekli

- Her aşama için ayrı kısa oturum; önce bu plan, sonra net görev.
- Dil tasarımı, ayrıştırıcı, ek çözümleyici ve kod üretimi için yüksek efor; rutin işler için düşük.
- Her değişiklik `cargo test` ile doğrulanır; yeni dil özelliği için `örnekler/` klasörüne `.ohc` + `.beklenen` çifti eklenir.
