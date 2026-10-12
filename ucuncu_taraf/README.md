# Üçüncü taraf GLib düzeltmesi

GTK 3 / WebKitGTK kullanan Tauri ve Wry, GLib 0.18 zincirini gerektirir.
GLib 0.20 sürümüne tek başına yükseltmek bu bağımlılıklarla uyumlu değildir.
Bu dizin crates.io `glib 0.18.5` kaynağının lisansları korunmuş kopyasıdır.
İndirilmiş paketin SHA-256 özeti: `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`.

Kaynak kodda yalnızca `src/variant_iter.rs` içindeki `VariantStrIter::impl_get` değiştirildi:
C işlevinin yazdığı işaretçi `let mut p` olarak tanımlanır ve `&mut p` verilir.
Bu, [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
ve [üst proje düzeltmesi](https://github.com/gtk-rs/gtk-rs-core/pull/1343) ile aynıdır.
Paket sürümü değiştirilerek tarama sonucu gizlenmez; sürüme göre yapılan taramalarda
uyarı hâlâ görülebilir. GTK zinciri güncel GLib'e geçince bu kopya kaldırılmalıdır.
Üst proje kaynaklarının özgün adları ve yorumları korunur.

Doğrulama: GLib/GObject geliştirme araçlarıyla
`cargo test --release --manifest-path ucuncu_taraf/glib/Cargo.toml --lib variant_iter`.

Lisans metni değiştirilmeden sondaki fazladan boş satır normalleştirildi;
Cargo.lock optimizasyonlu testin bağımlılıklarını sabitlemek için tutulur.
