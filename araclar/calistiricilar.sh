#!/bin/sh
# Hazır çalıştırıcıları derler: runtime/calistirici/linux-x86_64 ve windows-x86_64.exe
#
# Çalıştırıcı, C çalışma zamanının (runtime/orhunca_rt.c) programın makine kodunu
# kendi dosyasının sonundan yükleyen biçimidir. Derleyici bunları kendi içine gömer
# (src/baglayici.rs); kullanıcının bilgisayarında C derleyicisi gerekmez. Çalışma
# zamanı değiştiğinde yeniden çalıştırın.
#
# Gerekenler: gcc (Linux) ve x86_64-w64-mingw32-gcc (Debian/Ubuntu: gcc-mingw-w64-x86-64).
set -eu
cd "$(dirname "$0")/.."

gecici=$(mktemp -d)
trap 'rm -rf "$gecici"' EXIT

# Çalışma zamanının bütün ohc_* işlevleri ve Cranelift'in çağırabileceği C işlevleri
{
    echo "/* araclar/calistiricilar.sh üretti */"
    echo "static const IslevAdresi CALISTIRICI_ISLEVLERI[] = {"
    grep -oE '^(int64_t|void) ohc_[a-z0-9_]+\(' runtime/orhunca_rt.c |
        sed -E 's/^(int64_t|void) (ohc_[a-z0-9_]+)\(/\2/' | sort -u |
        while read -r ad; do echo "    {\"$ad\", (void *)$ad},"; done
    for ad in memcpy memmove memset fmod floor ceil trunc nearbyint sqrt; do
        echo "    {\"$ad\", (void *)$ad},"
    done
    echo "};"
} > "$gecici/calistirici_islevleri.h"

mkdir -p runtime/calistirici
BAYRAKLAR="-O2 -s -DORHUNCA_CALISTIRICI -I $gecici -Wall -Wno-unused-function"

${CC:-gcc} $BAYRAKLAR -o runtime/calistirici/linux-x86_64 runtime/orhunca_rt.c -lm -ldl
${MINGW_CC:-x86_64-w64-mingw32-gcc} $BAYRAKLAR -static-libgcc -o runtime/calistirici/windows-x86_64.exe \
    runtime/orhunca_rt.c -lws2_32

for d in runtime/calistirici/linux-x86_64 runtime/calistirici/windows-x86_64.exe; do
    echo "$d: $(wc -c < "$d") bayt"
done
