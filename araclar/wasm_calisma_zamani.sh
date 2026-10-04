#!/bin/sh
# C çalışma zamanını WebAssembly'ye derler: runtime/wasm/orhunca_rt.wasm
#
# Gerekenler: clang ve wasm-ld (LLVM 17 ya da daha yeni; Debian/Ubuntu: clang lld).
# Derleyici bu dosyayı kendi içine gömer (src/derleme.rs); çalışma zamanı
# (runtime/orhunca_rt.c ya da runtime/wasm/) değiştiğinde yeniden çalıştırın.
set -eu
cd "$(dirname "$0")/.."

CLANG=${CLANG:-clang}
cikti=runtime/wasm/orhunca_rt.wasm

# Dışa açılan işlevler: çalışma zamanının tüm ohc_* işlevleri (web sunucusu hariç)
disa=$(grep -oE '^(int64_t|void) ohc_[a-z0-9_]+\(' runtime/orhunca_rt.c |
    sed -E 's/^(int64_t|void) (ohc_[a-z0-9_]+)\(/\2/' |
    grep -vxE 'ohc_sun|ohc_web_yol|ohc_dene|ohc_ay_gir|ohc_ay_cik|ohc_ay_satir|ohc_yigin_denetle' | sort -u | sed 's/^/-Wl,--export=/' | tr '\n' ' ')

# shellcheck disable=SC2086
"$CLANG" --target=wasm32-unknown-unknown -O2 -nostdlib -ffreestanding \
    -mbulk-memory -mnontrapping-fptoint -msign-ext -mmutable-globals \
    -Wall -Wextra -Wno-unused-parameter -Werror \
    -I runtime/wasm \
    -Wl,--no-entry -Wl,--stack-first -Wl,-z,stack-size=262144 -Wl,--strip-all \
    $disa -Wl,--export=ohc_js_ayir -Wl,--export=__stack_pointer \
    -o "$cikti" runtime/orhunca_rt.c runtime/wasm/libc.c

echo "$cikti: $(wc -c < "$cikti") bayt"
