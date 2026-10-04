#!/bin/sh
# Hazır pencere kabuklarını derler: runtime/kabuk/linux-x86_64 ve windows-x86_64.exe
#
# Kabuk (masaustu/kabuk), `orhunca paketle` ile üretilen masaüstü uygulamalarının
# gövdesidir; derleyici bunları kendi içine gömer (src/derleme.rs). Kabuk
# değiştiğinde yeniden üretin.
#
# Linux kabuğu bu bilgisayarda derlenir (libwebkit2gtk-4.1-dev gerekir; geniş
# uyumluluk için eski glibc'li bir sistemde, ör. Ubuntu 22.04, derleyin).
# Windows kabuğu MSVC ile derlenmelidir (WebView2 yükleyicisi ve C çalışma zamanı
# exe'ye gömülür): Windows'ta bu betiği çalıştırın ya da CI'daki "kabuk" işinin
# çıktısını (kabuk-windows-x86_64.exe) runtime/kabuk/ altına koyun.
set -eu
cd "$(dirname "$0")/.."
mkdir -p runtime/kabuk
( cd masaustu/kabuk && cargo build --release )
case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) cp masaustu/kabuk/target/release/orhunca-kabuk.exe runtime/kabuk/windows-x86_64.exe ;;
    *) cp masaustu/kabuk/target/release/orhunca-kabuk runtime/kabuk/linux-x86_64 ;;
esac
ls -l runtime/kabuk
