#!/usr/bin/env python3
"""Orhunca logosunu (Ayraç: ‹𐰆›) yazı tiplerinden yola dönüştürüp SVG olarak üretir.

Yazı tipi gerektirmeyen SVG'ler docs/marka/ altına yazılır; simgeler bunlardan
üretilir (araclar/simgeler.sh). Gerekenler: pip install fonttools brotli
"""
import io
import os

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

KOK = os.path.join(os.path.dirname(__file__), "..")
YT = os.path.join(KOK, "studio", "yazitipleri")
CIKTI = os.path.join(KOK, "docs", "marka")

TURKUAZ = "#45d3c9"  # oklch(0.79 0.12 188)
KOYU = "#062221"
ACIK_ZEMIN_TURKUAZ = "#008e8b"  # oklch(0.58 0.11 192)


def yazitipi(ad, kalinlik=None):
    f = TTFont(os.path.join(YT, ad))
    if kalinlik and "fvar" in f:
        from fontTools.varLib.instancer import instantiateVariableFont
        f = instantiateVariableFont(f, {"wght": kalinlik})
    return f


def yol(font, metin, boyut, x, taban, aralik=0.0):
    """Metni (x, taban çizgisi) noktasından başlayan SVG yoluna çevirir; genişliği döndürür.
    `font` bir liste olabilir: harf ilkinde yoksa sonrakilerde aranır (Latin + Latin-ext)."""
    fontlar = font if isinstance(font, list) else [font]
    parcalar = []
    for c in metin:
        font = next(f for f in fontlar if ord(c) in f.getBestCmap())
        upm = font["head"].unitsPerEm
        olcek = boyut / upm
        glifler = font.getGlyphSet()
        ad = font.getBestCmap()[ord(c)]
        kalem = SVGPathPen(glifler)
        glifler[ad].draw(TransformPen(kalem, (olcek, 0, 0, -olcek, x, taban)))
        parcalar.append(kalem.getCommands())
        x += glifler[ad].width * olcek + aralik * boyut
    return " ".join(parcalar), x


def kutu(font, metin, boyut):
    """Metnin dikey sınırları (taban çizgisine göre, aşağı pozitif)."""
    from fontTools.pens.boundsPen import BoundsPen
    upm = font["head"].unitsPerEm
    glifler = font.getGlyphSet()
    cmap = font.getBestCmap()
    alt, ust = 0, 0
    for c in metin:
        b = BoundsPen(glifler)
        glifler[cmap[ord(c)]].draw(b)
        if b.bounds:
            alt = min(alt, b.bounds[1])
            ust = max(ust, b.bounds[3])
    return -ust * boyut / upm, -alt * boyut / upm


gokturk = yazitipi("noto-sans-old-turkic-400-old-turkic-0.woff2")
mono = yazitipi("jetbrains-mono-400-latin-1.woff2")
plex = yazitipi("ibm-plex-sans-600-latin-5.woff2", 600)
mono = yazitipi("jetbrains-mono-400-latin-1.woff2", 400)


def karo(zemin, yazi, x0=0, y0=0, boyut=100, ayrac=True):
    """100x100 birimlik yuvarlak köşeli karo: ‹ 𐰆 ›; (x0, y0) konumunda, `boyut` ölçeğinde."""
    s = boyut / 100
    # Ayraçlar 36, Göktürk harfi 42 (tasarımdaki gibi); ayraçlar 4 birim yukarıda.
    _, gx = yol(gokturk, "𐰆", 42, 0, 0)
    g_gen = gx
    _, a_gen = yol(mono, "‹", 36, 0, 0)
    toplam = (2 * a_gen + g_gen + 2) if ayrac else g_gen
    if not ayrac:
        # Küçük boyutlar: yalnızca harf, daha büyük
        _, g_gen = yol(gokturk, "𐰆", 56, 0, 0)
        toplam = g_gen
    x = (100 - toplam) / 2
    ust, alt = kutu(gokturk, "𐰆", 42 if ayrac else 56)
    taban = 50 - (ust + alt) / 2
    parcalar = []
    if ayrac:
        au, aa = kutu(mono, "‹", 36)
        a_taban = 50 - (au + aa) / 2 - 2
        p, x = yol(mono, "‹", 36, x, a_taban)
        parcalar.append(p)
        x += 1
        p, x = yol(gokturk, "𐰆", 42, x, taban)
        parcalar.append(p)
        x += 1
        p, x = yol(mono, "›", 36, x, a_taban)
        parcalar.append(p)
    else:
        p, x = yol(gokturk, "𐰆", 56, x, taban)
        parcalar.append(p)
    return (
        f'<g transform="translate({x0} {y0}) scale({s})">'
        f'<rect width="100" height="100" rx="24" fill="{zemin}"/>'
        f'<path fill="{yazi}" d="{" ".join(parcalar)}"/></g>'
    )


def yaz(ad, icerik, gen, yuk):
    with open(os.path.join(CIKTI, ad), "w") as f:
        f.write(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {gen:.0f} {yuk:.0f}" '
            f'width="{gen:.0f}" height="{yuk:.0f}">{icerik}</svg>\n'
        )


def yatay(zemin_karo, yazi_karo, yazi_renk):
    """Karo + "orhunca" yazısı (yükseklik 100)."""
    ust, alt = kutu(plex, "orhunca", 58)
    taban = 50 - (ust + alt) / 2
    p, son = yol(plex, "orhunca", 58, 120, taban, -0.025)
    # harf aralığı -0.025em: yazıyı hafifçe sıkıştır
    return karo(zemin_karo, yazi_karo) + f'<path fill="{yazi_renk}" d="{p}"/>', son + 4


os.makedirs(CIKTI, exist_ok=True)
yaz("logo.svg", karo(TURKUAZ, KOYU), 100, 100)
yaz("logo-kucuk.svg", karo(TURKUAZ, KOYU, ayrac=False), 100, 100)
yaz("logo-acik.svg", karo(ACIK_ZEMIN_TURKUAZ, "#f2efe8"), 100, 100)
for ad, (zk, yk, yr) in {
    "logo-yatay.svg": (TURKUAZ, KOYU, "#eef0f3"),
    "logo-yatay-acik.svg": (ACIK_ZEMIN_TURKUAZ, "#f2efe8", KOYU),
}.items():
    icerik, gen = yatay(zk, yk, yr)
    yaz(ad, icerik, gen, 100)
# Simge (1024): kenarlarda boşluklu, macOS/Windows simge ızgarasına uygun
yaz("simge.svg", karo(TURKUAZ, KOYU, 100, 100, 824), 1024, 1024)
print("docs/marka/ güncellendi")

# Paylaşım görseli (1200×630): sosyal ağlarda bağlantı önizlemesi
plex4 = [
    yazitipi("ibm-plex-sans-400-latin-1.woff2", 400),
    yazitipi("ibm-plex-sans-400-latin-ext-0.woff2", 400),
]
ust, alt = kutu(plex, "orhunca", 120)
p_ad, ad_son = yol(plex, "orhunca", 120, 0, 0, -0.025)
gen = 210 + 40 + ad_son
x0 = (1200 - gen) / 2
p_ad, _ = yol(plex, "orhunca", 120, x0 + 250, 300 - (ust + alt) / 2, -0.025)
alt_yazi = "Türkçe düşün, Türkçe kodla."
_, ay_gen = yol(plex4, alt_yazi, 44, 0, 0)
p_alt, _ = yol(plex4, alt_yazi, 44, (1200 - ay_gen) / 2, 470)
alt2 = "Okullar için ücretsiz programlama dili ve geliştirme ortamı"
_, a2 = yol(plex4, alt2, 26, 0, 0)
p_alt2, _ = yol(plex4, alt2, 26, (1200 - a2) / 2, 530)
izgara = "".join(
    f'<path d="M{x} 0V630" stroke="#1a1f26"/>' for x in range(0, 1201, 80)
) + "".join(f'<path d="M0 {y}H1200" stroke="#1a1f26"/>' for y in range(0, 631, 80))
yaz(
    "paylasim.svg",
    '<defs><radialGradient id="i" cx="50%" cy="45%" r="55%"><stop offset="0" stop-color="#45d3c9" stop-opacity=".18"/>'
    '<stop offset="1" stop-color="#45d3c9" stop-opacity="0"/></radialGradient>'
    '<radialGradient id="m" cx="50%" cy="45%" r="60%"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient>'
    '<mask id="mm"><rect width="1200" height="630" fill="url(#m)"/></mask></defs>'
    '<rect width="1200" height="630" fill="#0a0c0f"/>'
    f'<g mask="url(#mm)">{izgara}</g><rect width="1200" height="630" fill="url(#i)"/>'
    + karo(TURKUAZ, KOYU, x0, 195, 210)
    + f'<path fill="#eef0f3" d="{p_ad}"/><path fill="#c3c9d2" d="{p_alt}"/><path fill="#8b94a3" d="{p_alt2}"/>',
    1200,
    630,
)
