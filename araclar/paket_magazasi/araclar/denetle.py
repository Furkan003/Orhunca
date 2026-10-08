#!/usr/bin/env python3
"""Paket mağazasına gelen bir çekme isteğini (PR) denetler.

Çekme isteğinin kodu çalıştırılmaz: yalnızca paketler/<ad>.json dosyaları veri olarak okunur
ve her yeni sürüm `orhunca paket bilgi` ile GitHub'dan indirilip incelenir (paket kodu
çalıştırılmaz, yalnızca ayrıştırılır).

Kurallar:
- Çekme isteği yalnızca paketler/<ad>.json dosyaları ekleyebilir ya da değiştirebilir.
- Paket adı: 2-40 karakter, harf, rakam, _ ve -. Dosya adı paketin adıdır.
- Ad koruması: yeni paketin sahibi çekme isteğini açan kişidir; var olan bir paketi yalnızca
  sahibi güncelleyebilir; mağazadaki bir ada çok benzeyen yeni ad alınamaz.
- Yayımlanmış sürümler değiştirilemez ve silinemez; yalnızca sona yeni sürüm eklenir.
- Kaynak, sahibin GitHub'daki bir deposudur (github:<sahip>/...#etiket).
- Kayıttaki işleme, içerik özeti, ad, sürüm ve izinler paketin kendisiyle aynı olmalıdır
  (izinler koddan çıkarılır; eksik izin beyan edilemez).

Kullanım (GitHub Actions): python3 araclar/denetle.py <pr-başı-git-ref> <pr-sahibi>
Çıkış kodu 0: kabul; 1: ret. Sonuç GITHUB_STEP_SUMMARY'ye ve sonuc.md'ye yazılır.
"""
import json
import os
import re
import subprocess
import sys

AD = re.compile(r"^[\w-]{2,40}$")
SURUM = re.compile(r"^\d+\.\d+\.\d+$")
IZINLER = {"dosya", "ağ", "ortam", "c_kütüphanesi", "sunucu"}


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, check=False)


def sade(ad):
    ad = ad.lower()
    for a, b in zip("çğıöşüâî", "cgiosuai"):
        ad = ad.replace(a, b)
    return ad.replace("-", "").replace("_", "")


def benzer(a, b):
    """İki ad karıştırılabilir mi (sadeleştirilince aynı ya da tek harf farklı)?"""
    a, b = sade(a), sade(b)
    if a == b:
        return True
    if min(len(a), len(b)) < 5 or abs(len(a) - len(b)) > 1:
        return False
    onceki = list(range(len(b) + 1))
    for i, x in enumerate(a, 1):
        simdiki = [i]
        for j, y in enumerate(b, 1):
            simdiki.append(min(onceki[j] + 1, simdiki[j - 1] + 1, onceki[j - 1] + (x != y)))
        onceki = simdiki
    return onceki[-1] <= 1


def oku(ref, yol):
    c = git("show", f"{ref}:{yol}")
    if c.returncode != 0:
        return None
    return json.loads(c.stdout)


def incele(kaynak):
    c = subprocess.run(["orhunca", "paket", "bilgi", kaynak, "--json"],
                       capture_output=True, text=True, check=False, timeout=600)
    if c.returncode != 0:
        raise ValueError(f"paket indirilemedi ya da incelenemedi ({kaynak}): {c.stderr.strip() or c.stdout.strip()}")
    return json.loads(c.stdout.strip().splitlines()[-1])


def kaydi_denetle(yol, yeni, eski, yazar, diger_adlar):
    ad = os.path.basename(yol)[:-5]
    h = []
    if not AD.match(ad):
        return [f"`{ad}` paket adı olamaz (2-40 karakter; harf, rakam, _ ve -)"]
    for alan in ("ad", "açıklama", "sahip", "sürümler"):
        if alan not in yeni:
            h.append(f"`{alan}` alanı eksik")
    if h:
        return h
    if yeni["ad"] != ad:
        h.append(f"kayıttaki ad (`{yeni['ad']}`) dosya adıyla (`{ad}`) aynı olmalı")
    if not isinstance(yeni["açıklama"], str) or not 3 <= len(yeni["açıklama"]) <= 200:
        h.append("açıklama 3-200 karakter olmalı")
    sahip = yeni["sahip"]
    if eski is None:
        if sahip.lower() != yazar.lower():
            h.append(f"yeni paketin sahibi çekme isteğini açan kişi olmalı (`{yazar}`)")
        for d in diger_adlar:
            if benzer(ad, d):
                h.append(f"`{ad}` adı mağazadaki `{d}` paketine çok benziyor; başka bir ad seçin")
    else:
        if eski["sahip"].lower() != sahip.lower():
            h.append("paketin sahibi değiştirilemez")
        if eski["sahip"].lower() != yazar.lower():
            h.append(f"`{ad}` paketini yalnızca sahibi (`{eski['sahip']}`) güncelleyebilir")
    surumler = yeni["sürümler"]
    if not isinstance(surumler, list) or not surumler:
        return h + ["en az bir sürüm olmalı"]
    eskiler = eski["sürümler"] if eski else []
    if surumler[:len(eskiler)] != eskiler:
        h.append("yayımlanmış sürümler değiştirilemez ve silinemez; yeni sürüm sona eklenir")
    gorulen = set()
    for s in surumler:
        if s.get("sürüm") in gorulen:
            h.append(f"{s.get('sürüm')} sürümü iki kez yazılmış")
        gorulen.add(s.get("sürüm"))
    if h:
        return h
    for s in surumler[len(eskiler):]:
        etiket = f"{s.get('sürüm')}"
        for alan in ("sürüm", "kaynak", "işleme", "özet", "izinler"):
            if alan not in s:
                h.append(f"{etiket}: `{alan}` alanı eksik")
        if h:
            continue
        if not SURUM.match(s["sürüm"]):
            h.append(f"{etiket}: sürüm üç sayıdan oluşmalı (ör. 1.0.0)")
        if not s["kaynak"].lower().startswith(f"github:{sahip.lower()}/") or "#" not in s["kaynak"]:
            h.append(f"{etiket}: kaynak sahibin GitHub deposu ve bir etiket olmalı (github:{sahip}/depo#v{s['sürüm']})")
            continue
        if not set(s["izinler"]) <= IZINLER:
            h.append(f"{etiket}: bilinmeyen izin")
        try:
            i = incele(s["kaynak"])
        except Exception as e:  # noqa: BLE001
            h.append(f"{etiket}: {e}")
            continue
        for alan in ("ad", "sürüm", "işleme", "özet"):
            if i[alan] != (ad if alan == "ad" else s[alan]):
                h.append(f"{etiket}: `{alan}` paketin kendisiyle uyuşmuyor (paket: `{i[alan]}`)")
        if sorted(i["izinler"]) != sorted(s["izinler"]):
            h.append(f"{etiket}: izinler koddan çıkarılanlarla uyuşmuyor (kod: {', '.join(i['izinler']) or 'yok'})")
    return h


def main():
    ref, yazar = sys.argv[1], sys.argv[2]
    taban = "origin/main"
    degisen = git("diff", "--name-status", f"{taban}...{ref}").stdout.splitlines()
    hatalar = []
    kabul = []
    if not degisen:
        hatalar.append("çekme isteğinde değişiklik yok")
    diger_adlar = [os.path.basename(a)[:-5] for a in os.listdir("paketler") if a.endswith(".json")]
    for satir in degisen:
        durum, yol = satir.split("\t")[0], satir.split("\t")[-1]
        if not re.match(r"^paketler/[^/]+\.json$", yol) or durum[0] not in "AM":
            hatalar.append(f"`{yol}`: çekme isteği yalnızca paketler/<ad>.json dosyaları ekleyebilir ya da değiştirebilir")
            continue
        try:
            yeni = oku(ref, yol)
            eski = oku(taban, yol)
        except json.JSONDecodeError as e:
            hatalar.append(f"`{yol}`: JSON hatası: {e}")
            continue
        ad = os.path.basename(yol)[:-5]
        try:
            h = kaydi_denetle(yol, yeni, eski, yazar, [d for d in diger_adlar if d != ad])
        except (KeyError, TypeError, AttributeError) as e:
            h = [f"kayıt biçimi hatalı ({e!r}); kaydı `orhunca paket yayımla` ile hazırlayın"]
        if h:
            hatalar += [f"`{yol}`: {x}" for x in h]
        else:
            kabul.append(f"`{ad}` {yeni['sürümler'][-1]['sürüm']} (izinler: {', '.join(yeni['sürümler'][-1]['izinler']) or 'yok'})")
    if hatalar:
        metin = "### ✗ Paket mağazası denetimi geçmedi\n\n" + "\n".join(f"- {x}" for x in hatalar)
        metin += "\n\nDüzeltip çekme isteğini güncelleyin. Kayıtları `orhunca paket yayımla` hazırlar."
    else:
        metin = "### ✓ Paket mağazası denetimi geçti\n\n" + "\n".join(f"- {x}" for x in kabul)
    with open("sonuc.md", "w", encoding="utf-8") as f:
        f.write(metin + "\n")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as f:
            f.write(metin + "\n")
    print(metin)
    sys.exit(1 if hatalar else 0)


if __name__ == "__main__":
    main()
