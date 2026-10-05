#!/bin/sh
# Android kabuğunu (mobil/android) derler ve `orhunca paketle --hedef android`ın kullandığı
# runtime/kabuk/android.okb dosyasını üretir: APK'nın içindeki dosyalar (META-INF hariç),
# sıkıştırılmamış olarak tek bir dosyada. İmzalama ve ad/kimlik değişikliği paketlerken yapılır.
# Gerekenler: JDK 17+, Gradle 8.x, Android SDK (ANDROID_HOME; platform 35, build-tools).
set -eu
cd "$(dirname "$0")/.."
( cd mobil/android && gradle --no-daemon -q assembleRelease )
python3 - mobil/android/app/build/outputs/apk/release/app-release-unsigned.apk runtime/kabuk/android.okb <<'PY'
import struct, sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
cikti = bytearray(b"OHCAPK1\n")
for g in sorted(z.infolist(), key=lambda g: g.filename):
    if g.filename.startswith("META-INF/") or g.is_dir():
        continue
    ad = g.filename.encode()
    veri = z.read(g)
    cikti += struct.pack("<I", len(ad)) + ad + struct.pack("<I", len(veri)) + veri
open(sys.argv[2], "wb").write(cikti)
print(sys.argv[2], len(cikti), "bayt")
PY
