#!/bin/sh
# Kilitli Rust bağımlılıklarını güncel RustSec duyurularıyla karşılaştırır.
set -eu
cd "$(dirname "$0")/.."
if ! command -v cargo-audit >/dev/null 2>&1; then
    echo 'Önce kurun: cargo install cargo-audit --version 0.22.2 --locked' >&2
    exit 1
fi
sonuc=0
cargo audit || sonuc=1
for kilit in masaustu/Cargo.lock masaustu/kabuk/Cargo.lock web/oyun/Cargo.lock ucuncu_taraf/glib/Cargo.lock; do
    cargo audit --no-fetch --file "$kilit" || sonuc=1
done
exit "$sonuc"
