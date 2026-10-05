#!/bin/sh
# Android emülatöründe uçtan uca sınama: APK kurulur, sınama kipinde açılır (kabuk sayfadaki
# yazıyı logcat'e yazar, "+ 1" düğmesine basar), ekranda sayacın arttığı görülür.
#   araclar/android_sinamasi.sh sayac.apk org.orhunca.sayac
set -eu
apk=$1; paket=$2
bekle_yazi() {
    i=0
    while [ $i -lt 45 ]; do
        if adb logcat -d -s OrhuncaSinama:I | grep -q "$1"; then echo "✓ ekranda: $1"; return 0; fi
        sleep 2; i=$((i + 1))
    done
    echo "✗ ekranda bulunamadı: $1"
    adb logcat -d -s OrhuncaSinama:I Orhunca:I | tail -20
    adb logcat -d | grep -iE "AndroidRuntime|chromium" | tail -30
    adb shell screencap -p /sdcard/ekran.png && adb pull /sdcard/ekran.png android-ekran.png >/dev/null || true
    return 1
}
adb install -r "$apk"
adb logcat -c
adb shell am start -W -n "$paket/org.orhunca.kabuk.AnaEtkinlik" --ez orhunca_sinama true --es tikla "'+ 1'"
bekle_yazi "Şu anki değer: 0"
bekle_yazi "Şu anki değer: 1"
adb shell screencap -p /sdcard/ekran.png && adb pull /sdcard/ekran.png android-ekran.png >/dev/null
echo "Android sınaması başarılı"
