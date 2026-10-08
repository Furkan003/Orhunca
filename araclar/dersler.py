#!/usr/bin/env python3
"""Dersleri (dersler/*.md) web sitesi sayfalarına ve Stüdyo'nun ders verisine dönüştürür.

    python3 araclar/dersler.py <site/dersler klasörü>   # site sayfaları
    python3 araclar/dersler.py --studyo                # studio/dersler.json

Ders biçimi (Markdown): `# Başlık`, `> özet`, paragraflar, `-` listeleri, `kod`, **kalın**,
[bağlantı](adres). Kod blokları: ```orhunca (örnek), ```cikti (beklenen çıktı), ```girdi
(program girdisi). `## Alıştırma: Ad` bölümü bir görevdir: açıklama, ```orhunca baslangic,
isteğe bağlı ```girdi ve ```cikti, ```orhunca cozum. Bütün örnekler ve çözümler
tests/dersler.rs ile derlenip çalıştırılarak sınanır.
"""
import base64
import html
import json
import os
import re
import sys

KOK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
DERSLER = os.path.join(KOK, "dersler")


def kac(s):
    return html.escape(s, quote=True)


def satir_ici(s):
    s = kac(s)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", s)
    s = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', s)
    return s


def dene_adresi(kod, girdi=""):
    def b64(m):
        return "d" + base64.urlsafe_b64encode(m.encode()).decode().rstrip("=")
    adres = "kod=" + b64(kod)
    if girdi:
        adres += "&girdi=" + b64(girdi)
    return adres


def coz(metin):
    """Dersi parçalara ayırır: başlık, özet, gövde blokları ve görevler."""
    satirlar = metin.split("\n")
    ders = {"baslik": "", "ozet": "", "bloklar": [], "gorevler": []}
    hedef = ders["bloklar"]
    i = 0
    paragraf = []

    def paragrafi_bitir():
        if paragraf:
            hedef.append(("p", " ".join(paragraf)))
            paragraf.clear()

    while i < len(satirlar):
        s = satirlar[i]
        if s.startswith("```"):
            paragrafi_bitir()
            tur = s[3:].strip()
            j = i + 1
            kod = []
            while j < len(satirlar) and not satirlar[j].startswith("```"):
                kod.append(satirlar[j])
                j += 1
            hedef.append(("kod", tur, "\n".join(kod) + "\n"))
            i = j + 1
            continue
        if s.startswith("# "):
            paragrafi_bitir()
            ders["baslik"] = s[2:].strip()
        elif s.startswith("> "):
            paragrafi_bitir()
            ders["ozet"] = s[2:].strip()
        elif s.startswith("## Alıştırma:"):
            paragrafi_bitir()
            gorev = {"baslik": s.split(":", 1)[1].strip(), "bloklar": []}
            ders["gorevler"].append(gorev)
            hedef = gorev["bloklar"]
        elif s.startswith("## "):
            paragrafi_bitir()
            hedef = ders["bloklar"]
            hedef.append(("h2", s[3:].strip()))
        elif s.startswith("- "):
            paragrafi_bitir()
            if hedef and hedef[-1][0] == "ul":
                hedef[-1][1].append(s[2:].strip())
            else:
                hedef.append(("ul", [s[2:].strip()]))
        elif not s.strip():
            paragrafi_bitir()
        else:
            paragraf.append(s.strip())
        i += 1
    paragrafi_bitir()
    for g in ders["gorevler"]:
        g["aciklama"] = []
        for b in g["bloklar"]:
            if b[0] == "kod":
                tur = b[1]
                if tur == "orhunca baslangic":
                    g["baslangic"] = b[2]
                elif tur == "orhunca cozum":
                    g["cozum"] = b[2]
                elif tur == "girdi":
                    g["girdi"] = b[2]
                elif tur == "cikti":
                    g["cikti"] = b[2]
            else:
                g["aciklama"].append(b)
    return ders


def bloklari_html(bloklar, dene_oneki):
    h = []
    son_kod = None
    for i, b in enumerate(bloklar):
        if b[0] == "p":
            h.append(f"<p>{satir_ici(b[1])}</p>")
        elif b[0] == "h2":
            h.append(f"<h2>{satir_ici(b[1])}</h2>")
        elif b[0] == "ul":
            h.append("<ul>" + "".join(f"<li>{satir_ici(x)}</li>" for x in b[1]) + "</ul>")
        elif b[0] == "kod":
            tur, kod = b[1], b[2]
            if tur == "orhunca":
                # Sonraki girdi bloğu bu örneğe aittir
                girdi = ""
                for s in bloklar[i + 1:i + 3]:
                    if s[0] == "kod" and s[1] == "girdi":
                        girdi = s[2]
                son_kod = kod
                dene = (
                    f'<a class="ders-dene" href="{dene_oneki}dene.html#{dene_adresi(kod, girdi)}">Çalıştır ▸</a>'
                    if dene_oneki is not None
                    else ""
                )
                h.append(f'<div class="ders-kod">{dene}<pre data-orhunca>{kac(kod.rstrip())}</pre></div>')
            elif tur == "cikti":
                h.append(f'<div class="ders-cikti"><span>Çıktı</span><pre>{kac(kod.rstrip())}</pre></div>')
            elif tur == "girdi":
                h.append(f'<div class="ders-cikti girdi"><span>Girdi</span><pre>{kac(kod.rstrip())}</pre></div>')
    return "\n".join(h)


def dersleri_oku():
    dersler = []
    for ad in sorted(os.listdir(DERSLER)):
        if not ad.endswith(".md"):
            continue
        d = coz(open(os.path.join(DERSLER, ad), encoding="utf-8").read())
        d["kimlik"] = ad[:-3]
        d["sira"] = int(ad.split("-")[0])
        dersler.append(d)
    return dersler


SAYFA = """<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{baslik} — Orhunca dersleri</title>
<meta name="description" content="{ozet}">
<link rel="icon" href="../marka/logo.svg" type="image/svg+xml">
<link rel="stylesheet" href="../yazitipleri/yazitipleri.css">
<link rel="stylesheet" href="../stil.css">
<link rel="stylesheet" href="ders.css">
</head>
<body>
<header class="ust"><div class="kap">
  <a class="marka" href="../"><img src="../marka/logo.svg" alt="">orhunca</a>
  <nav class="gezinti"><a href="./" class="etkin">Dersler</a><a href="../basvuru.html">Başvuru</a><a href="../dene.html">Tarayıcıda dene</a><a href="../#indir">İndir</a></nav>
  <span class="bosluk"></span>
  <a class="github" href="https://github.com/Furkan003/Orhunca"><span>GitHub</span></a>
</div></header>
<main class="kap ders-duzen">
{icerik}
</main>
<footer class="alt"><div class="kap"><div>© Orhunca geliştiricileri</div><nav><a href="./">Dersler</a><a href="../dene.html">Tarayıcıda dene</a><a href="../">Ana sayfa</a></nav></div></footer>
<script src="../vurgula.js"></script>
</body>
</html>
"""

CSS = """
.ders-duzen { padding-top: 48px; padding-bottom: 80px; max-width: 860px; }
.ders-duzen h1 { font-size: clamp(32px, 4.5vw, 46px); margin: 8px 0 10px; }
.ders-ust-yazi { color: var(--vurgu); font: 600 13px var(--sans); letter-spacing: 0.08em; text-transform: uppercase; }
.ders-ozet { color: var(--yazi2); font-size: 19px; margin: 0 0 32px; }
.ders-duzen p, .ders-duzen li { color: var(--yazi2); font-size: 16.5px; }
.ders-duzen h2 { font-size: 24px; margin: 40px 0 12px; letter-spacing: -0.02em; }
.ders-duzen p code, .ders-duzen li code { font-size: 14px; background: var(--pano); border: 1px solid var(--kenar2); padding: 1px 6px; border-radius: 5px; color: var(--yazi); }
.ders-kod { position: relative; margin: 18px 0 0; }
.ders-kod pre { margin: 0; padding: 16px 18px; background: var(--arka2); border: 1px solid var(--kenar); border-radius: 12px; font-size: 14.5px; line-height: 1.7; overflow-x: auto; }
.ders-dene { position: absolute; top: 10px; right: 10px; font: 600 12.5px var(--sans); padding: 5px 10px; border-radius: 7px; background: var(--vurgu); color: var(--vurgu-koyu); }
.ders-dene:hover { color: var(--vurgu-koyu); background: var(--vurgu-acik); }
.ders-cikti { margin: 0 0 18px; border: 1px solid var(--kenar2); border-top: 0; border-radius: 0 0 12px 12px; background: var(--pano); padding: 10px 18px 12px; }
.ders-kod + .ders-cikti { margin-top: -6px; padding-top: 14px; }
.ders-cikti span { display: block; font: 600 11px var(--sans); letter-spacing: 0.08em; text-transform: uppercase; color: var(--soluk2); margin-bottom: 4px; }
.ders-cikti pre { margin: 0; font-size: 14px; color: var(--soluk); white-space: pre-wrap; }
.ders-cikti.girdi pre { color: var(--sari); }
.ders-cikti:has(+ .ders-cikti) { border-radius: 0; margin-bottom: 0; }
.ders-cikti + .ders-cikti { margin-top: 0; border-top: 1px dashed var(--kenar2); }
.gorev { margin: 44px 0 0; padding: 26px; border-radius: 16px; border: 1px solid var(--kenar); background: linear-gradient(160deg, var(--vurgu-zemin), transparent 50%), var(--pano); }
.gorev h2 { margin: 0 0 10px; }
.gorev h2 small { display: block; font: 600 12px var(--sans); color: var(--vurgu); letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 6px; }
.gorev details { margin-top: 16px; }
.gorev summary { cursor: pointer; color: var(--soluk); font-size: 14.5px; }
.gorev .eylemler { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 16px; }
.ders-gezinti { display: flex; justify-content: space-between; gap: 16px; margin-top: 56px; padding-top: 24px; border-top: 1px solid var(--kenar2); }
.ders-gezinti a { display: block; padding: 14px 18px; border-radius: 12px; border: 1px solid var(--kenar2); background: var(--pano); color: var(--yazi); max-width: 48%; }
.ders-gezinti a span { display: block; font-size: 12.5px; color: var(--soluk); }
.ders-liste { display: grid; gap: 12px; margin-top: 28px; }
.ders-liste a { display: flex; gap: 18px; align-items: center; padding: 18px 20px; border-radius: 14px; border: 1px solid var(--kenar2); background: var(--pano); color: var(--yazi); }
.ders-liste a:hover { border-color: var(--kenar); color: var(--yazi); transform: translateY(-1px); }
.ders-liste .no { flex-shrink: 0; width: 42px; height: 42px; border-radius: 10px; display: grid; place-items: center; background: var(--vurgu-zemin); color: var(--vurgu); font: 600 16px var(--mono); }
.ders-liste b { display: block; font-size: 17px; }
.ders-liste span.ozet { color: var(--soluk); font-size: 14.5px; }
.ders-liste .rozet-g { margin-left: auto; font-size: 12.5px; color: var(--soluk2); white-space: nowrap; }
"""


def gorev_html(g, sira, dene_oneki):
    h = [f'<section class="gorev"><h2><small>Alıştırma {sira}</small>{kac(g["baslik"])}</h2>']
    h.append(bloklari_html(g["aciklama"], None))
    h.append(f'<div class="ders-kod"><pre data-orhunca>{kac(g.get("baslangic", "").rstrip())}</pre></div>')
    if g.get("girdi"):
        h.append(f'<div class="ders-cikti girdi"><span>Girdi</span><pre>{kac(g["girdi"].rstrip())}</pre></div>')
    if g.get("cikti"):
        h.append(f'<div class="ders-cikti"><span>Beklenen çıktı</span><pre>{kac(g["cikti"].rstrip())}</pre></div>')
    h.append('<div class="eylemler">')
    h.append(f'<a class="dugme birincil kucuk" href="{dene_oneki}dene.html#{dene_adresi(g.get("baslangic", ""), g.get("girdi", ""))}">Tarayıcıda çöz</a>')
    h.append("</div>")
    if g.get("cozum"):
        h.append(f'<details><summary>Çözümü göster</summary><div class="ders-kod"><pre data-orhunca>{kac(g["cozum"].rstrip())}</pre></div></details>')
    h.append("</section>")
    return "\n".join(h)


def site(cikti):
    os.makedirs(cikti, exist_ok=True)
    dersler = dersleri_oku()
    with open(os.path.join(cikti, "ders.css"), "w") as f:
        f.write(CSS)
    for i, d in enumerate(dersler):
        icerik = [f'<div class="ders-ust-yazi">Ders {d["sira"]}</div><h1>{kac(d["baslik"])}</h1>',
                  f'<p class="ders-ozet">{satir_ici(d["ozet"])}</p>', bloklari_html(d["bloklar"], "../")]
        for j, g in enumerate(d["gorevler"]):
            icerik.append(gorev_html(g, j + 1, "../"))
        gez = ['<nav class="ders-gezinti">']
        gez.append(f'<a href="{dersler[i-1]["kimlik"]}.html"><span>← Önceki</span>{kac(dersler[i-1]["baslik"])}</a>' if i > 0 else "<span></span>")
        if i + 1 < len(dersler):
            gez.append(f'<a href="{dersler[i+1]["kimlik"]}.html" style="text-align:right"><span>Sonraki →</span>{kac(dersler[i+1]["baslik"])}</a>')
        gez.append("</nav>")
        icerik.append("".join(gez))
        with open(os.path.join(cikti, d["kimlik"] + ".html"), "w") as f:
            f.write(SAYFA.format(baslik=kac(d["baslik"]), ozet=kac(d["ozet"]), icerik="\n".join(icerik)))
    liste = "".join(
        f'<a href="{d["kimlik"]}.html"><span class="no">{d["sira"]}</span><span><b>{kac(d["baslik"])}</b>'
        f'<span class="ozet">{satir_ici(d["ozet"])}</span></span><span class="rozet-g">{len(d["gorevler"])} alıştırma</span></a>'
        for d in dersler
    )
    icerik = (
        '<div class="ders-ust-yazi">Orhunca öğren</div><h1>Dersler</h1>'
        '<p class="ders-ozet">Sıfırdan başlayarak programlamayı Türkçe öğrenin. Her derste örnekler ve '
        "alıştırmalar var; kodları kurulum yapmadan tarayıcıda çalıştırabilirsiniz. Aynı dersler "
        "Orhunca Stüdyo'nun <b>Dersler</b> panelinde, otomatik denetlenen alıştırmalarla da var.</p>"
        f'<div class="ders-liste">{liste}</div>'
    )
    with open(os.path.join(cikti, "index.html"), "w") as f:
        f.write(SAYFA.format(baslik="Dersler", ozet="Orhunca dersleri", icerik=icerik))


def studyo():
    dersler = []
    for d in dersleri_oku():
        dersler.append({
            "kimlik": d["kimlik"],
            "sira": d["sira"],
            "baslik": d["baslik"],
            "ozet": d["ozet"],
            "html": bloklari_html(d["bloklar"], None),
            "gorevler": [
                {
                    "baslik": g["baslik"],
                    "aciklama": bloklari_html(g["aciklama"], None),
                    "baslangic": g.get("baslangic", ""),
                    "girdi": g.get("girdi", ""),
                    "cikti": g.get("cikti"),
                    "cozum": g.get("cozum", ""),
                }
                for g in d["gorevler"]
            ],
        })
    with open(os.path.join(KOK, "studio", "dersler.json"), "w") as f:
        json.dump(dersler, f, ensure_ascii=False, indent=1)
        f.write("\n")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--studyo":
        studyo()
    else:
        site(sys.argv[1] if len(sys.argv) > 1 else "_site/dersler")
