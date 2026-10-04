#!/usr/bin/env python3
"""Hata ayıklayıcının çalışma zamanı tarafını sınar (CI'da Windows için):
programı `derle --ayıklama` ile derler, ORHUNCA_AYIKLA kapısından gelen bağlantıyı
kabul eder, kesme noktası / adım / çerçeve komutlarını gönderir ve olayları denetler.

Kullanım: python3 araclar/ayiklama_sinamasi.py <orhunca> <geçici klasör>
"""
import os
import socket
import subprocess
import sys

orhunca, klasor = sys.argv[1], sys.argv[2]
os.makedirs(klasor, exist_ok=True)
kaynak = os.path.join(klasor, "ay.ohc")
with open(kaynak, "w", encoding="utf-8") as f:
    f.write(
        "işlev kare(n: sayı) -> sayı:\n"
        "    sonuç = n * n\n"
        "    döndür sonuç\n"
        "\n"
        "toplam = 0\n"
        "her i için 1'den 3'e kadar:\n"
        "    toplam += kare(i)\n"
        "toplam'ı yaz.\n"
    )
program = os.path.join(klasor, "ay.exe" if os.name == "nt" else "ay")
subprocess.run([orhunca, "derle", "--ayıklama", kaynak, "-o", program], check=True)

dinleyici = socket.socket()
dinleyici.bind(("127.0.0.1", 0))
dinleyici.listen(1)
dinleyici.settimeout(30)
ortam = dict(os.environ, ORHUNCA_AYIKLA=str(dinleyici.getsockname()[1]))
surec = subprocess.Popen([program], env=ortam, stdout=subprocess.PIPE)
baglanti, _ = dinleyici.accept()
baglanti.settimeout(30)
akis = baglanti.makefile("rw", encoding="utf-8", newline="\n")


def olay():
    satirlar = []
    while True:
        s = akis.readline()
        assert s, "bağlantı kapandı"
        s = s.rstrip("\n")
        if s == "son":
            return satirlar
        satirlar.append(s)


def gonder(k):
    akis.write(k + "\n")
    akis.flush()


ilk = olay()
assert ilk[0] == "dur adim" and ilk[1] == "cerceve 0 0 5 ana", ilk
gonder("kesmeler 0:2")
gonder("devam")
d = olay()
assert d[0] == "dur kesme" and d[1] == "cerceve 0 0 2 kare" and "deg n\tsayı\t1" in d, d
gonder("cerceve 1")
c = olay()
assert c[0] == "cdeg 1" and "deg i\tsayı\t1" in c, c
gonder("kesmeler")
gonder("cik")
d = olay()
assert d[1] == "cerceve 0 0 7 ana" and "deg toplam\tsayı\t1" in d, d
gonder("devam")
cikti = surec.communicate(timeout=30)[0].decode().replace("\r", "")
assert surec.returncode == 0 and cikti == "14\n", (surec.returncode, cikti)
print("hata ayıklama sınaması geçti")
