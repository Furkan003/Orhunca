#!/usr/bin/env python3
"""docs/basvuru.md dosyasını sitenin aranabilir başvuru sayfasına dönüştürür.

    python3 araclar/basvuru_sayfasi.py <site klasörü>/basvuru.html

docs/basvuru.md `orhunca başvuru --md` ile üretilir (tests: src/basvuru.rs belge_guncel).
"""
import html
import os
import re
import sys

KOK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def kac(s):
    return html.escape(s, quote=True)


def satir_ici(s):
    s = kac(s.replace("\\|", "|"))
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", s)
    return s


def hucreler(satir):
    # `\|` hücre ayracı değildir
    parcalar = re.split(r"(?<!\\)\|", satir.strip().strip("|"))
    return [p.strip() for p in parcalar]


def main():
    with open(os.path.join(KOK, "docs", "basvuru.md"), encoding="utf-8") as f:
        satirlar = f.read().splitlines()
    bolumler = []
    for s in satirlar:
        if s.startswith("## "):
            bolumler.append({"ad": s[3:], "not": "", "islevler": []})
        elif not bolumler or not s.strip() or s.startswith("|---") or s.startswith("| İşlev"):
            continue
        elif s.startswith("|"):
            ad, kullanim, aciklama = hucreler(s)
            bolumler[-1]["islevler"].append((ad.strip("`"), kullanim, aciklama))
        else:
            bolumler[-1]["not"] += s + " "
    icerik = []
    for b in bolumler:
        kimlik = re.sub(r"[^a-z0-9çğıöşü]+", "-", b["ad"].lower()).strip("-")
        icerik.append(f'<section class="bsv-bolum" id="{kac(kimlik)}"><h2>{kac(b["ad"])}</h2>')
        if b["not"].strip():
            icerik.append(f'<p class="bsv-not">{satir_ici(b["not"].strip())}</p>')
        for ad, kullanim, aciklama in b["islevler"]:
            icerik.append(
                f'<div class="bsv-islev" id="{kac(ad)}"><div class="bsv-kullanim">{satir_ici(kullanim)}</div>'
                f'<div class="bsv-aciklama">{satir_ici(aciklama)}</div></div>'
            )
        icerik.append("</section>")
    icindekiler = " · ".join(
        f'<a href="#{kac(re.sub(r"[^a-z0-9çğıöşü]+", "-", b["ad"].lower()).strip("-"))}">{kac(b["ad"])}</a>'
        for b in bolumler
    )
    sayfa = SAYFA.replace("{icindekiler}", icindekiler).replace("{icerik}", "\n".join(icerik))
    sayfa = sayfa.replace("{sayi}", str(sum(len(b["islevler"]) for b in bolumler)))
    with open(sys.argv[1], "w", encoding="utf-8") as f:
        f.write(sayfa)


SAYFA = """<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Yerleşik işlevler — Orhunca</title>
<meta name="description" content="Orhunca'nın yerleşik işlevleri: metin, liste, dosya, tarih, web, arayüz ve oyun işlevleri; aranabilir başvuru.">
<link rel="icon" href="marka/logo.svg" type="image/svg+xml">
<link rel="stylesheet" href="yazitipleri/yazitipleri.css">
<link rel="stylesheet" href="stil.css">
<style>
.bsv { padding-top: 48px; padding-bottom: 80px; max-width: 900px; }
.bsv h1 { font-size: clamp(30px, 4.5vw, 42px); margin: 8px 0 10px; }
.bsv-giris { color: var(--yazi2); font-size: 17px; margin: 0 0 20px; }
.bsv-giris code, .bsv-not code, .bsv-aciklama code { font-size: 14px; background: var(--pano); border: 1px solid var(--kenar2); padding: 1px 6px; border-radius: 5px; color: var(--yazi); }
.bsv-ara { width: 100%; max-width: 520px; height: 44px; padding: 0 14px; font: 16px var(--sans); color: var(--yazi); background: var(--pano); border: 1px solid var(--kenar); border-radius: 10px; }
.bsv-ara:focus { outline: none; border-color: var(--vurgu); }
.bsv-icindekiler { margin: 14px 0 8px; color: var(--soluk); font-size: 14px; line-height: 1.9; }
.bsv-icindekiler a { color: var(--yazi2); }
.bsv-bolum h2 { font-size: 22px; margin: 36px 0 8px; letter-spacing: -0.02em; }
.bsv-not { color: var(--soluk); font-size: 15px; margin: 0 0 10px; }
.bsv-islev { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 4px 20px; padding: 12px 0; border-top: 1px solid var(--kenar2); }
.bsv-kullanim { font: 14px var(--mono); color: var(--vurgu); overflow-wrap: anywhere; }
.bsv-kullanim code { background: none; border: 0; padding: 0; color: inherit; font-size: inherit; }
.bsv-aciklama { color: var(--yazi2); font-size: 15px; }
.bsv-yok { color: var(--soluk); margin-top: 24px; }
.gizli { display: none !important; }
@media (max-width: 640px) { .bsv-islev { grid-template-columns: 1fr; } }
</style>
</head>
<body>
<header class="ust"><div class="kap">
  <a class="marka" href="./"><img src="marka/logo.svg" alt="">orhunca</a>
  <nav class="gezinti"><a href="dersler/">Dersler</a><a href="basvuru.html" class="etkin">Başvuru</a><a href="dene.html">Tarayıcıda dene</a><a href="./#indir">İndir</a></nav>
  <span class="bosluk"></span>
  <a class="github" href="https://github.com/Furkan003/Orhunca"><span>GitHub</span></a>
</div></header>
<main class="kap bsv">
<h1>Yerleşik işlevler</h1>
<p class="bsv-giris">Her Orhunca programında hazır bulunan {sayi} işlev. Terminalde: <code>orhunca başvuru tarih</code>. Dilin kendisi için <a href="https://github.com/Furkan003/Orhunca/blob/HEAD/docs/dil-rehberi.md">dil rehberi</a>.</p>
<input id="ara" class="bsv-ara" type="search" placeholder="Ara: tarih, büyük harf, dosya, json…" autocomplete="off" spellcheck="false" aria-label="İşlev ara">
<div class="bsv-icindekiler">{icindekiler}</div>
{icerik}
<p class="bsv-yok gizli" id="yok">Bu aramaya uyan işlev yok.</p>
</main>
<footer class="alt"><div class="kap"><div>© Orhunca geliştiricileri</div><nav><a href="dersler/">Dersler</a><a href="dene.html">Tarayıcıda dene</a><a href="./">Ana sayfa</a></nav></div></footer>
<script>
(() => {
  const sade = t => t.toLocaleLowerCase('tr').replace(/[çğıöşüâî]/g, h => ({ ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u', â: 'a', î: 'i' })[h]).replace(/_/g, ' ');
  const ara = document.getElementById('ara');
  const islevler = [...document.querySelectorAll('.bsv-islev')].map(el => [el, sade(el.textContent)]);
  const suz = () => {
    const a = sade(ara.value.trim());
    let toplam = 0;
    for (const [el, metin] of islevler) { const u = !a || metin.includes(a); el.classList.toggle('gizli', !u); if (u) toplam++; }
    for (const b of document.querySelectorAll('.bsv-bolum')) b.classList.toggle('gizli', !b.querySelector('.bsv-islev:not(.gizli)'));
    document.getElementById('yok').classList.toggle('gizli', toplam > 0);
    history.replaceState(null, '', a ? '#ara=' + encodeURIComponent(ara.value.trim()) : location.pathname);
  };
  ara.addEventListener('input', suz);
  const m = location.hash.match(/^#ara=(.*)$/);
  if (m) { ara.value = decodeURIComponent(m[1]); suz(); }
  addEventListener('keydown', e => { if (e.key === '/' && document.activeElement !== ara) { e.preventDefault(); ara.focus(); } });
})();
</script>
</body>
</html>
"""

if __name__ == "__main__":
    main()
