#!/usr/bin/env python3
"""paketler/*.json kayıtlarından dizin.json'u üretir (Orhunca'nın okuduğu paket dizini).

Her paketin son sürümü dizine girer. Kullanım: python3 araclar/dizin.py
"""
import json
import os

KOK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def main():
    paketler = []
    klasor = os.path.join(KOK, "paketler")
    for ad in sorted(os.listdir(klasor)):
        if not ad.endswith(".json"):
            continue
        with open(os.path.join(klasor, ad), encoding="utf-8") as f:
            k = json.load(f)
        son = k["sürümler"][-1]
        paketler.append({
            "ad": k["ad"],
            "açıklama": k["açıklama"],
            "sahip": k["sahip"],
            "sürüm": son["sürüm"],
            "kaynak": son["kaynak"],
            "işleme": son["işleme"],
            "özet": son["özet"],
            "izinler": son["izinler"],
            "sürüm_sayısı": len(k["sürümler"]),
        })
    dizin = {"sürüm": 2, "paketler": paketler}
    with open(os.path.join(KOK, "dizin.json"), "w", encoding="utf-8") as f:
        json.dump(dizin, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"{len(paketler)} paket")


if __name__ == "__main__":
    main()
