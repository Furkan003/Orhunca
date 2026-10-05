#!/bin/sh
# Android emülatöründe uçtan uca sınama: APK kurulur, açılır, ekrandaki yazı okunur,
# "Artır" düğmesine dokunulur ve sayacın arttığı görülür.
#   araclar/android_sinamasi.sh sayac.apk org.orhunca.sayac
set -eu
apk=$1; paket=$2
ekran() { adb shell uiautomator dump /sdcard/ekran.xml >/dev/null 2>&1 || true; adb shell cat /sdcard/ekran.xml 2>/dev/null || true; }
bekle_yazi() {
    i=0
    while [ $i -lt 30 ]; do
        if ekran | grep -q "$1"; then echo "✓ ekranda: $1"; return 0; fi
        sleep 2; i=$((i + 1))
    done
    echo "✗ ekranda bulunamadı: $1"; ekran | head -c 4000; echo
    adb logcat -d | grep -iE "chromium|orhunca|AndroidRuntime" | tail -40
    return 1
}
adb install -r "$apk"
adb shell am start -W -n "$paket/org.orhunca.kabuk.AnaEtkinlik"
bekle_yazi "Şu anki değer: 0"
# Düğmenin ekrandaki konumu (bounds="[x1,y1][x2,y2]") bulunur ve ortasına dokunulur
konum=$(ekran | tr '>' '\n' | grep 'text="Artır"' | head -1 | sed -n 's/.*bounds="\[\([0-9]*\),\([0-9]*\)\]\[\([0-9]*\),\([0-9]*\)\]".*/\1 \2 \3 \4/p')
[ -n "$konum" ] || { echo "✗ Artır düğmesi bulunamadı"; ekran | head -c 4000; exit 1; }
set -- $konum
adb shell input tap $(( ($1 + $3) / 2 )) $(( ($2 + $4) / 2 ))
bekle_yazi "Şu anki değer: 1"
adb shell screencap -p /sdcard/ekran.png && adb pull /sdcard/ekran.png android-ekran.png >/dev/null
echo "Android sınaması başarılı"
