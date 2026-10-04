// Orhunca Stüdyo açılış animasyonu (docs/marka/ tasarımı): Göktürk harfi taşa oyulur
// gibi belirir, ‹ › ayraçları kapanır, "orhunca" yazılır, ilk Türkçe komut çalışır.
// 1920×1080'lik bir sahne pencereye sığdırılır; her kare yalnızca T'den (saniye)
// hesaplanır. Tıklama ya da tuş animasyonu geçer. Hareketi azaltma tercihi
// açıksa ya da ayarlardan kapatıldıysa gösterilmez.
(function () {
  'use strict';
  let ayar = true;
  try { ayar = JSON.parse(localStorage.getItem('orhunca.acilis') ?? 'true'); } catch { /* yok */ }
  const azHareket = window.matchMedia && matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (!ayar || azHareket || /[?&]acilis=0\b/.test(location.search)) return;

  // Bölümler: Oyma 1.0 · Ayraç 0.6 · Yazı 1.0 · Kod 1.4 · Bekle 0.6
  const C = { Oyma: 0, Ayrac: 1.0, Yazi: 1.6, Kod: 2.6, Bekle: 4.0 };
  const SURE = 4.6;
  const TURKUAZ = 'oklch(0.79 0.12 188)';

  const Y = {
    dogrusal: t => t,
    sinus: t => -(Math.cos(Math.PI * t) - 1) / 2,
    kubik: t => (t < 0.5 ? 4 * t * t * t : (t - 1) * (2 * t - 2) * (2 * t - 2) + 1),
    dortlu: t => 1 - Math.pow(1 - t, 4),
  };
  const tw = (T, a, b, ease, from = 0, to = 1) => {
    if (T <= a) return from;
    if (T >= b) return to;
    return from + (to - from) * ease((T - a) / (b - a));
  };

  const KOD = [
    ['"Merhaba, dünya"', '#e6e9ee'],
    ["'yı", '#8b94a3'],
    [' yaz', TURKUAZ],
    ['.', '#e6e9ee'],
  ];
  const KOD_METNI = KOD.map(p => p[0]).join('');
  const KELIME = 'orhunca';

  const kok = document.createElement('div');
  kok.className = 'acilis';
  kok.setAttribute('role', 'presentation');
  kok.innerHTML = `
    <div class="acilis-isik"></div><div class="acilis-izgara"></div>
    <div class="acilis-sahne">
      <div class="acilis-ust">
        <div class="acilis-satir">
          <div class="acilis-karo">
            <span class="acilis-ayrac sol">‹</span>
            <span class="acilis-harf">
              <span class="acilis-oyma">𐰆</span><span class="acilis-dolgu">𐰆</span><span class="acilis-karo-imlec"></span>
            </span>
            <span class="acilis-ayrac sag">›</span>
          </div>
          <div class="acilis-kelime"><span class="acilis-yazi"></span><span class="acilis-imlec"></span></div>
        </div>
      </div>
      <div class="acilis-panel">
        <div class="acilis-panel-baslik"><i></i><i></i><i></i><span>merhaba.ohc</span></div>
        <div class="acilis-kod"><span class="acilis-no">1</span><span class="acilis-kod-metin"></span></div>
        <div class="acilis-cikti">→ Merhaba, dünya</div>
      </div>
    </div>
    <div class="acilis-karartma"></div>`;
  document.body.appendChild(kok);
  const $ = s => kok.querySelector(s);
  const el = {
    sahne: $('.acilis-sahne'), isik: $('.acilis-isik'), izgara: $('.acilis-izgara'),
    ust: $('.acilis-ust'), karo: $('.acilis-karo'), sol: $('.sol'), sag: $('.sag'),
    oyma: $('.acilis-oyma'), dolgu: $('.acilis-dolgu'), karoImlec: $('.acilis-karo-imlec'),
    yazi: $('.acilis-yazi'), imlec: $('.acilis-imlec'), panel: $('.acilis-panel'),
    kod: $('.acilis-kod-metin'), cikti: $('.acilis-cikti'), karartma: $('.acilis-karartma'),
  };

  function olcekle() {
    const s = Math.min(innerWidth / 1920, innerHeight / 1080);
    el.sahne.style.transform = `translate(-50%, -50%) scale(${s})`;
  }
  olcekle();
  addEventListener('resize', olcekle);

  let sonKod = -1;
  function ciz(T) {
    const g = tw(T, 0, 1.2, Y.dogrusal);
    el.isik.style.background = `radial-gradient(circle at 50% 46%, oklch(0.79 0.12 188 / ${0.14 * g + 0.03 * Math.sin(T * 2)}) 0, transparent 46%)`;
    el.izgara.style.opacity = g;
    const blink = Math.floor(T * 2.4) % 2 === 0 ? 1 : 0.1;
    // Oyma: harf alttan yukarı çizgi çizgi belirir, sonra dolar
    const oyma = tw(T, C.Oyma, C.Oyma + 0.8, Y.sinus);
    el.karo.style.opacity = tw(T, 0, 0.3, Y.dogrusal);
    el.oyma.style.clipPath = `inset(0 0 ${Math.max(0, 100 - oyma * 100)}% 0)`;
    el.dolgu.style.opacity = tw(T, C.Oyma + 0.55, C.Oyma + 0.9, Y.dogrusal);
    // Ayraçlar dışarıdan kapanır; harfin içinde imleç yanıp söner
    const br = tw(T, C.Ayrac, C.Ayrac + 0.45, Y.kubik);
    el.sol.style.transform = `translateX(${(1 - br) * -70}px)`;
    el.sag.style.transform = `translateX(${(1 - br) * 70}px)`;
    el.sol.style.opacity = el.sag.style.opacity = br;
    const karoImlec = T >= C.Ayrac + 0.3 && T < C.Yazi;
    el.karoImlec.style.display = karoImlec ? 'block' : 'none';
    el.karoImlec.style.opacity = blink;
    // Yazı: orhunca daktilo gibi
    const n = Math.floor(tw(T, C.Yazi, C.Yazi + 0.75, Y.dogrusal) * KELIME.length + 0.0001);
    el.yazi.textContent = KELIME.slice(0, n);
    el.imlec.style.display = T >= C.Yazi ? 'inline-block' : 'none';
    el.imlec.style.opacity = T < C.Kod + 0.4 ? 1 : blink;
    // Kod: logo yukarı kayar, düzenleyici paneli gelir, ilk komut yazılıp çalışır
    el.ust.style.transform = `translateY(${tw(T, C.Kod, C.Kod + 0.5, Y.kubik, 0, -150)}px)`;
    const panel = tw(T, C.Kod, C.Kod + 0.5, Y.dortlu);
    el.panel.style.opacity = panel;
    el.panel.style.transform = `translateY(${(1 - panel) * 30}px)`;
    const cn = Math.floor(tw(T, C.Kod + 0.3, C.Kod + 1.0, Y.dogrusal) * KOD_METNI.length);
    if (cn !== sonKod) {
      sonKod = cn;
      let kalan = cn, h = '';
      for (const [p, renk] of KOD) {
        if (kalan <= 0) break;
        const parca = p.slice(0, kalan);
        kalan -= parca.length;
        h += `<span style="color:${renk}">${parca.replace(/&/g, '&amp;').replace(/</g, '&lt;')}</span>`;
      }
      el.kod.innerHTML = h + '<span class="acilis-kod-imlec">▍</span>';
    }
    const ki = el.kod.lastChild;
    if (ki) ki.style.opacity = blink;
    el.cikti.style.opacity = tw(T, C.Kod + 1.0, C.Kod + 1.1, Y.dogrusal);
    el.karartma.style.opacity = tw(T, C.Bekle + 0.15, C.Bekle + 0.5, Y.dogrusal);
  }

  let bitti = false, baslangic = null, gec = null;
  function bitir() {
    if (bitti) return;
    bitti = true;
    kok.classList.add('kapaniyor');
    setTimeout(() => kok.remove(), 260);
    removeEventListener('resize', olcekle);
    removeEventListener('keydown', gecTus, true);
  }
  function gecTus(e) { e.preventDefault(); e.stopPropagation(); bitir(); }
  kok.addEventListener('click', bitir);
  addEventListener('keydown', gecTus, true);

  function kare(ts) {
    if (bitti) return;
    if (baslangic === null) baslangic = ts;
    const T = (ts - baslangic) / 1000;
    if (T >= SURE) { ciz(SURE); bitir(); return; }
    ciz(T);
    requestAnimationFrame(kare);
  }
  ciz(0);
  // Yazı tipleri yüklenince başlar (en çok 600 ms beklenir).
  const basla = () => { if (gec !== null) return; gec = 1; requestAnimationFrame(kare); };
  if (document.fonts && document.fonts.load) {
    Promise.all([
      document.fonts.load("42px 'Noto Sans Old Turkic'", '𐰆'),
      document.fonts.load("600 150px 'IBM Plex Sans'", KELIME),
      document.fonts.load("38px 'JetBrains Mono'", KOD_METNI),
    ]).then(basla, basla);
    setTimeout(basla, 600);
  } else basla();
})();
