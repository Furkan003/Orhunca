#!/usr/bin/env python3
"""Deneme sayfasının örnek listesi (ornekler.json): örnekler/ klasöründen ve birkaç
tanıtım örneğinden. Dosya okuyan/yazan ya da web sunucusu başlatan örnekler alınmaz."""
import json
import os
import unicodedata

KOK = os.path.join(os.path.dirname(__file__), "..")

TANITIM = [
    ("merhaba", "Başlangıç", "Merhaba, dünya", '"Merhaba, dünya!"\'yı yaz.\n'),
    ("selam", "Başlangıç", "Kendi fiilin: selamla",
     "fiil (kişi: metin)'yi selamla:\n    (\"Merhaba, \" + kişi + \"!\")'yı yaz.\n\n\"Ayşe\"'yi selamla.\n\"Mehmet\"'i selamla.\n"),
    ("notlar", "Başlangıç", "Not ortalaması",
     "# Sınıfın not ortalaması\nnotlar = [85, 92, 78, 64, 99]\ntoplam = 0\n\nher n için notlar'dan:\n    toplam += n\n\n"
     "ortalama = toplam / uzunluk(notlar)\n(\"Ortalama: \" + ortalama)'yı yaz.\n\neğer ortalama 80'den büyükse:\n"
     "    \"Sınıf çok başarılı!\"'yı yaz.\n"),
    ("girdi", "Başlangıç", "Kullanıcıdan girdi",
     "# Sağ alttaki \"Program girdisi\" kutusuna adınızı ve yaşınızı yazın.\n\"Adın ne?\"'yi yaz.\nad = oku()\n"
     "\"Kaç yaşındasın?\"'ı yaz.\nyaş = sayı(oku())\n(\"Merhaba \" + ad + \", 10 yıl sonra \" + (yaş + 10) + \" yaşında olacaksın.\")'ı yaz.\n"),
]

ADLAR = {
    "asal": "Asal sayılar", "faktoriyel": "Faktöriyel (özyineleme)", "fiiller": "Fiiller",
    "fizzbuzz": "FizzBuzz", "hata_yakalama": "Hata yakalama", "isimler": "Listeler ve metinler",
    "iç_içe_modeller": "İç içe modeller", "metin_işlemleri": "Metin işlemleri", "modeller": "Modeller",
    "ondalik": "Ondalık sayılar", "sayilar": "Sayılar", "seçenekler": "Seçenekler (numaralandırma)",
    "standart_kutuphane": "Standart kütüphane", "tip_çıkarımı": "Tip çıkarımı", "merhaba": None,
    "sayaç": "Sayaç", "yapılacaklar": "Yapılacaklar listesi", "hesap_makinesi": "Hesap makinesi",
}
ATLA = ("dosya", "sun(", "kaydet", "argümanlar(", "ortam(", "kütüphane \"")


def kimlik(ad):
    s = unicodedata.normalize("NFKD", ad.replace("ı", "i")).encode("ascii", "ignore").decode()
    return s.replace(" ", "_").lower()


def main():
    liste = [{"kimlik": k, "grup": g, "baslik": b, "kod": c} for k, g, b, c in TANITIM]
    for grup, klasor in (("Konsol", "örnekler"), ("Arayüz", "örnekler/arayüz")):
        yol = os.path.join(KOK, klasor)
        for ad in sorted(os.listdir(yol)):
            if not ad.endswith(".ohc"):
                continue
            kok = ad[:-4]
            baslik = ADLAR.get(kok, kok.replace("_", " ").capitalize())
            if baslik is None:
                continue
            kod = open(os.path.join(yol, ad), encoding="utf-8").read()
            if grup == "Konsol" and any(a in kod for a in ATLA):
                continue
            liste.append({"kimlik": kimlik(kok), "grup": grup, "baslik": baslik, "kod": kod,
                          "dosya": os.path.join(klasor, ad)})
    json.dump(liste, __import__("sys").stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
