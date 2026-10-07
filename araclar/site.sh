#!/bin/sh
# Web sitesini (GitHub Pages) derler: araclar/site.sh [çıktı klasörü, varsayılan _site]
#
# - web/site/: tanıtım ve indirme sayfası, tarayıcıda deneme sayfası
# - web/oyun/: derleyicinin WebAssembly biçimi (deneme sayfası kodu tarayıcıda derler)
# - dersler/: dersler (Markdown → HTML, araclar/dersler.py)
# Gerekenler: Rust (wasm32-unknown-unknown hedefi), Python 3.
set -eu
cd "$(dirname "$0")/.."
CIKTI=${1:-_site}
rm -rf "$CIKTI"
mkdir -p "$CIKTI/calisma" "$CIKTI/marka" "$CIKTI/ekran"

( cd web/oyun && cargo build --release --target wasm32-unknown-unknown )
cp web/oyun/target/wasm32-unknown-unknown/release/orhunca_oyun.wasm "$CIKTI/calisma/oyun.wasm"
cp runtime/wasm/orhunca_rt.wasm runtime/wasm/orhunca.js runtime/wasm/arayuz.html "$CIKTI/calisma/"

cp web/site/* "$CIKTI/"
cp -r studio/yazitipleri "$CIKTI/yazitipleri"
cp docs/marka/*.svg "$CIKTI/marka/"
if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w 1200 docs/marka/paylasim.svg -o "$CIKTI/marka/paylasim.png"
    # Kurulabilir web uygulamasının (manifest.webmanifest) simgeleri
    rsvg-convert -w 192 -h 192 docs/marka/simge.svg -o "$CIKTI/marka/simge-192.png"
    rsvg-convert -w 512 -h 512 docs/marka/simge.svg -o "$CIKTI/marka/simge-512.png"
fi
# Çevrimdışı önbelleğin sürümü: her derlemede değişir, eski önbellek silinir.
SURUM=$(git rev-parse --short HEAD 2>/dev/null || date +%s)-$(date +%s)
sed -i.yedek "s/__SURUM__/$SURUM/" "$CIKTI/sw.js" && rm -f "$CIKTI/sw.js.yedek"
cp docs/ekran/*.png "$CIKTI/ekran/"
{ cat docs/ajan-girisi.md; echo; cat docs/dil-rehberi.md; echo; cat docs/arayuz.md; } > "$CIKTI/llms-full.txt"
cp kurulum/kur.sh kurulum/kur.ps1 "$CIKTI/"
python3 araclar/site_ornekleri.py > "$CIKTI/ornekler.json"
python3 araclar/dersler.py "$CIKTI/dersler"
touch "$CIKTI/.nojekyll"
du -sh "$CIKTI"
