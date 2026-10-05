/* Orhunca Stüdyo — temalar
 *
 * Bir tema (.ohctema dosyası) JSON'dur:
 *   { "orhunca_tema": 1, "ad": "...", "yazar": "...", "taban": "koyu" | "acik",
 *     "renkler": { "arka": "#0a0c0f", "vurgu": "#45d3c9", "sz-anahtar": "...", ... },
 *     "yazi": { "arayuz": "IBM Plex Sans", "kod": "JetBrains Mono", "kod_boyut": 13, "olcek": 100 },
 *     "kose": 8,
 *     "arka_plan": { "kaynak": "data:image/gif;base64,...", "tur": "resim" | "video",
 *                    "konum": "kapla" | "sigdir" | "doseme" | "ortala",
 *                    "saydamlik": 82, "bulaniklik": 0, "karartma": 25 },
 *     "ozel_css": "..." }
 * Renkler CSS değişkenleridir (stil.css :root). Verilmeyen değerler tabanın varsayılanıdır.
 */
(function () {
  'use strict';

  // Düzenlenebilen renkler: [değişken, etiket, grup]
  const DEGISKENLER = [
    ['arka', 'Uygulama arka planı', 'Arayüz'],
    ['pencere', 'Düzenleyici ve paneller', 'Arayüz'],
    ['koyu', 'Kenar çubukları ve başlık', 'Arayüz'],
    ['koyu2', 'Alt panel', 'Arayüz'],
    ['kart', 'Kartlar ve pencereler', 'Arayüz'],
    ['girdi', 'Giriş kutuları', 'Arayüz'],
    ['dugme', 'Düğmeler', 'Arayüz'],
    ['hover', 'Üzerine gelince', 'Arayüz'],
    ['cizgi', 'Ayırıcı çizgiler', 'Arayüz'],
    ['yazi', 'Yazı', 'Yazı'],
    ['yazi2', 'İkincil yazı', 'Yazı'],
    ['soluk', 'Soluk yazı', 'Yazı'],
    ['cok-soluk', 'Çok soluk yazı', 'Yazı'],
    ['vurgu', 'Vurgu rengi', 'Vurgu'],
    ['vurgu-koyu', 'Vurgunun üstündeki yazı', 'Vurgu'],
    ['hata', 'Hata', 'Vurgu'],
    ['basari', 'Başarı', 'Vurgu'],
    ['sari', 'Uyarı', 'Vurgu'],
    ['sz-duz', 'Kod: düz yazı', 'Kod'],
    ['sz-anahtar', 'Kod: anahtar kelimeler', 'Kod'],
    ['sz-islev', 'Kod: işlev ve fiiller', 'Kod'],
    ['sz-metin', 'Kod: metinler', 'Kod'],
    ['sz-sayi', 'Kod: sayılar', 'Kod'],
    ['sz-tip', 'Kod: tipler', 'Kod'],
    ['sz-ek', 'Kod: hâl ekleri', 'Kod'],
    ['sz-yorum', 'Kod: yorumlar', 'Kod'],
  ];

  const tema = (ad, taban, renkler, ek = {}) => Object.assign({ orhunca_tema: 1, ad, yazar: 'Orhunca', taban, renkler }, ek);

  const HAZIR = [
    tema('Orhunca Koyu', 'koyu', {}),
    tema('Orhunca Açık', 'acik', {}),
    tema('Gece Mavisi', 'koyu', {
      arka: '#070b16', pencere: '#0d1424', koyu: '#0a101e', koyu2: '#0b1220', kart: '#111a2e', girdi: '#131d33', dugme: '#16213a', hover: '#18243f', cizgi: '#1c2742',
      yazi: '#e3e9f5', yazi2: '#c4cee2', soluk: '#8d9ab5', 'cok-soluk': '#4a5672',
      vurgu: '#6ea8ff', 'vurgu-koyu': '#06122a', 'sz-duz': '#d6deee', 'sz-anahtar': '#6ea8ff', 'sz-islev': '#c79bff', 'sz-metin': '#9ee09e', 'sz-sayi': '#ffb86b', 'sz-tip': '#5fd4e0', 'sz-yorum': '#56647f',
    }),
    tema('Mor Gece', 'koyu', {
      arka: '#1e1f29', pencere: '#282a36', koyu: '#21222c', koyu2: '#232530', kart: '#2c2e3b', girdi: '#30323f', dugme: '#343746', hover: '#363848', cizgi: '#3a3c4e',
      yazi: '#f8f8f2', yazi2: '#e2e2dc', soluk: '#a4a8c4', 'cok-soluk': '#6272a4',
      vurgu: '#bd93f9', 'vurgu-koyu': '#1e1f29', hata: '#ff5555', basari: '#50fa7b', sari: '#f1fa8c',
      'sz-duz': '#f8f8f2', 'sz-anahtar': '#ff79c6', 'sz-islev': '#50fa7b', 'sz-metin': '#f1fa8c', 'sz-sayi': '#bd93f9', 'sz-tip': '#8be9fd', 'sz-ek': '#ffb86c', 'sz-yorum': '#6272a4',
    }),
    tema('Kuzey', 'koyu', {
      arka: '#242933', pencere: '#2e3440', koyu: '#292e39', koyu2: '#2b303b', kart: '#333a47', girdi: '#363d4b', dugme: '#3b4252', hover: '#3b4252', cizgi: '#3b4252',
      yazi: '#eceff4', yazi2: '#d8dee9', soluk: '#a3acbd', 'cok-soluk': '#616e88',
      vurgu: '#88c0d0', 'vurgu-koyu': '#1d2129', hata: '#bf616a', basari: '#a3be8c', sari: '#ebcb8b',
      'sz-duz': '#d8dee9', 'sz-anahtar': '#81a1c1', 'sz-islev': '#88c0d0', 'sz-metin': '#a3be8c', 'sz-sayi': '#b48ead', 'sz-tip': '#8fbcbb', 'sz-ek': '#d08770', 'sz-yorum': '#616e88',
    }),
    tema('Orman', 'koyu', {
      arka: '#0d1410', pencere: '#131c16', koyu: '#101812', koyu2: '#111a14', kart: '#18241c', girdi: '#1b291f', dugme: '#1e2e22', hover: '#213226', cizgi: '#22342a',
      yazi: '#e4eee6', yazi2: '#c8d8cc', soluk: '#8fa596', 'cok-soluk': '#4d6355',
      vurgu: '#7dd87d', 'vurgu-koyu': '#0b1a0d', 'sz-duz': '#d6e4d9', 'sz-anahtar': '#7dd87d', 'sz-islev': '#e0c068', 'sz-metin': '#e8a87c', 'sz-sayi': '#9ec9ff', 'sz-tip': '#7fd6c2', 'sz-yorum': '#5a7262',
    }),
    tema('Gün Batımı', 'koyu', {
      arka: '#1a0f14', pencere: '#24141b', koyu: '#1f1117', koyu2: '#211219', kart: '#2c1922', girdi: '#311c26', dugme: '#36202a', hover: '#3a222e', cizgi: '#3c2430',
      yazi: '#fbe9ee', yazi2: '#efd2da', soluk: '#c29aa6', 'cok-soluk': '#7a5562',
      vurgu: '#ff8a5c', 'vurgu-koyu': '#2a0f05', 'sz-duz': '#f4dde3', 'sz-anahtar': '#ff8a5c', 'sz-islev': '#ffc857', 'sz-metin': '#f78fb3', 'sz-sayi': '#c3a6ff', 'sz-tip': '#7fd1c7', 'sz-yorum': '#86606c',
    }),
    tema('Kâğıt', 'acik', {
      arka: '#e8e1d3', pencere: '#f8f3e8', koyu: '#efe8da', koyu2: '#ede5d6', kart: '#fbf7ee', girdi: '#f1ebdf', dugme: '#efe7d8', hover: '#e7dfcf', cizgi: '#dcd2bf',
      yazi: '#3b3128', yazi2: '#4f4338', soluk: '#7c6e60', 'cok-soluk': '#ab9e8e',
      vurgu: '#a0522d', 'vurgu-koyu': '#fbf7ee', 'sz-duz': '#3b3128', 'sz-anahtar': '#a0522d', 'sz-islev': '#2f6f8f', 'sz-metin': '#6b7d2e', 'sz-sayi': '#8f4fa0', 'sz-tip': '#1f7a6d', 'sz-yorum': '#a39482',
    }),
    tema('Yüksek Karşıtlık', 'koyu', {
      arka: '#000000', pencere: '#000000', koyu: '#000000', koyu2: '#000000', kart: '#000000', girdi: '#000000', dugme: '#000000', hover: '#1a1a1a', cizgi: '#ffffff',
      yazi: '#ffffff', yazi2: '#ffffff', soluk: '#e0e0e0', 'cok-soluk': '#bdbdbd',
      vurgu: '#ffff00', 'vurgu-koyu': '#000000', hata: '#ff6b6b', basari: '#00ff7f', sari: '#ffff00',
      'sz-duz': '#ffffff', 'sz-anahtar': '#ffff00', 'sz-islev': '#00ffff', 'sz-metin': '#7fff7f', 'sz-sayi': '#ff9cff', 'sz-tip': '#9cc9ff', 'sz-ek': '#ffd27f', 'sz-yorum': '#c0c0c0',
    }, { yazi: { kod_boyut: 15, olcek: 110 } }),
  ];

  const ARAYUZ_YAZI = ['IBM Plex Sans', 'system-ui', 'Segoe UI', 'Ubuntu', 'Noto Sans', 'Cantarell', 'Arial', 'Georgia'];
  const KOD_YAZI = ['JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Ubuntu Mono', 'DejaVu Sans Mono', 'Consolas', 'Courier New'];

  // Saydamlıkta panellerin arkasından görünen yüzeyler
  const YUZEYLER = ['arka', 'pencere', 'koyu', 'koyu2', 'kart', 'girdi', 'dugme', 'hover'];

  /** Paylaşılan temadaki özel CSS: dışarıdan kaynak yükleyemez (@import, http adresleri). */
  function guvenliCss(css) {
    return String(css || '')
      .replace(/@import[^;]*;?/gi, '')
      .replace(/url\(\s*(['"]?)\s*(?!data:)[^)]*\)/gi, 'none')
      .replace(/<\/?style/gi, '')
      .slice(0, 50000);
  }

  function dogrula(t) {
    if (!t || typeof t !== 'object' || t.orhunca_tema !== 1) throw new Error('Bu bir Orhunca tema dosyası değil.');
    const temiz = {
      orhunca_tema: 1,
      ad: String(t.ad || 'Adsız tema').slice(0, 60),
      yazar: String(t.yazar || '').slice(0, 60),
      taban: t.taban === 'acik' ? 'acik' : 'koyu',
      renkler: {},
      yazi: {},
      kose: Number.isFinite(+t.kose) ? Math.min(20, Math.max(0, +t.kose)) : undefined,
      arka_plan: null,
      ozel_css: guvenliCss(t.ozel_css),
    };
    const renkDuzgun = (r) => typeof r === 'string' && r.length < 80 && /^(#[0-9a-f]{3,8}|rgba?\([\d\s.,%/]+\)|hsla?\([\d\s.,%/deg]+\)|oklch\([\d\s.,%/]+\)|[a-z]+)$/i.test(r.trim());
    for (const [ad] of DEGISKENLER) if (renkDuzgun(t.renkler?.[ad])) temiz.renkler[ad] = t.renkler[ad].trim();
    const y = t.yazi || {};
    const yaziAdi = (a) => (typeof a === 'string' && /^[\p{L}\p{N} _-]{1,40}$/u.test(a) ? a : undefined);
    temiz.yazi = {
      arayuz: yaziAdi(y.arayuz),
      kod: yaziAdi(y.kod),
      kod_boyut: Number.isFinite(+y.kod_boyut) ? Math.min(28, Math.max(9, +y.kod_boyut)) : undefined,
      olcek: Number.isFinite(+y.olcek) ? Math.min(150, Math.max(70, +y.olcek)) : undefined,
    };
    const a = t.arka_plan;
    if (a && typeof a.kaynak === "string" && a.kaynak.length < 30e6 && /^data:(image|video)\/[a-z0-9.+-]+;base64,[A-Za-z0-9+\/=]+$/i.test(a.kaynak)) {
      temiz.arka_plan = {
        kaynak: a.kaynak,
        tur: /^data:video/i.test(a.kaynak) ? 'video' : 'resim',
        konum: ['kapla', 'sigdir', 'doseme', 'ortala'].includes(a.konum) ? a.konum : 'kapla',
        saydamlik: Math.min(100, Math.max(20, +a.saydamlik || 82)),
        bulaniklik: Math.min(30, Math.max(0, +a.bulaniklik || 0)),
        karartma: Math.min(90, Math.max(0, +a.karartma || 0)),
      };
    }
    return temiz;
  }

  let stilEtiketi = null;
  // Arka plan resmi varken yüzeylerin saydamlaştırılmadan önceki renkleri
  let saydamOncesi = {};
  let arkaPlan = null;

  /** Temayı sayfaya uygular (canlı önizleme de bununla yapılır). */
  function uygula(t) {
    const kok = document.documentElement;
    kok.dataset.tema = t.taban === 'acik' ? 'acik' : 'koyu';
    for (const [ad] of DEGISKENLER) kok.style.removeProperty('--' + ad);
    kok.style.removeProperty('--sans');
    kok.style.removeProperty('--mono');
    kok.style.removeProperty('--kose');
    kok.style.zoom = '';
    const renkler = Object.assign({}, t.renkler);
    const ap = t.arka_plan;
    saydamOncesi = {};
    for (const [ad, deger] of Object.entries(renkler)) kok.style.setProperty('--' + ad, deger);
    // Arka plan resmi: yüzeyler saydamlaşır, resim arkada görünür.
    if (ap) {
      const hesap = getComputedStyle(kok);
      for (const ad of YUZEYLER) {
        const r = renkler[ad] || hesap.getPropertyValue('--' + ad).trim();
        saydamOncesi[ad] = r;
        if (r) kok.style.setProperty('--' + ad, `color-mix(in srgb, ${r} ${ap.saydamlik}%, transparent)`);
      }
    }
    const y = t.yazi || {};
    if (y.arayuz) kok.style.setProperty('--sans', `'${y.arayuz}', system-ui, sans-serif`);
    if (y.kod) kok.style.setProperty('--mono', `'${y.kod}', ui-monospace, monospace`);
    if (y.olcek && y.olcek !== 100) kok.style.zoom = y.olcek / 100;
    if (t.kose !== undefined) kok.style.setProperty('--kose', t.kose + 'px');
    kok.classList.toggle('arka-planli', !!ap);
    kok.classList.toggle('kose-ayarli', t.kose !== undefined);

    if (!stilEtiketi) {
      stilEtiketi = document.createElement('style');
      stilEtiketi.id = 'temaOzelCss';
      document.head.appendChild(stilEtiketi);
    }
    stilEtiketi.textContent = guvenliCss(t.ozel_css);

    if (!arkaPlan) {
      arkaPlan = document.createElement('div');
      arkaPlan.className = 'tema-arka-plan';
      arkaPlan.setAttribute('aria-hidden', 'true');
      document.body.prepend(arkaPlan);
    }
    arkaPlan.innerHTML = '';
    arkaPlan.hidden = !ap;
    if (ap) {
      const boyut = { kapla: 'cover', sigdir: 'contain', doseme: 'auto', ortala: 'auto' }[ap.konum];
      const ic = document.createElement(ap.tur === 'video' ? 'video' : 'div');
      ic.className = 'tema-arka-plan-ic';
      if (ap.tur === 'video') {
        Object.assign(ic, { src: ap.kaynak, autoplay: true, loop: true, muted: true, playsInline: true });
        ic.style.objectFit = ap.konum === 'sigdir' ? 'contain' : 'cover';
      } else {
        ic.style.backgroundImage = `url("${ap.kaynak}")`;
        ic.style.backgroundSize = boyut;
        ic.style.backgroundRepeat = ap.konum === 'doseme' ? 'repeat' : 'no-repeat';
        ic.style.backgroundPosition = 'center';
      }
      ic.style.filter = ap.bulaniklik ? `blur(${ap.bulaniklik}px)` : '';
      const ort = document.createElement('div');
      ort.className = 'tema-arka-plan-ortu';
      ort.style.background = `rgba(0,0,0,${ap.karartma / 100})`;
      arkaPlan.append(ic, ort);
    }
  }

  /** Şu anki görünümden tema nesnesi (düzenleyicinin başlangıcı için). */
  function hesaplanan(t) {
    const hesap = getComputedStyle(document.documentElement);
    const renkler = {};
    for (const [ad] of DEGISKENLER) renkler[ad] = t.renkler?.[ad] || saydamOncesi[ad] || hesap.getPropertyValue('--' + ad).trim();
    return renkler;
  }

  /** Herhangi bir CSS rengini (oklch, color-mix dahil) <input type=color> için #rrggbb'ye çevirir. */
  let olcer = null;
  function onaltilik(renk) {
    if (/^#[0-9a-f]{6}$/i.test(renk)) return renk.toLowerCase();
    if (!olcer) {
      olcer = document.createElement('span');
      olcer.style.display = 'none';
      document.body.appendChild(olcer);
    }
    olcer.style.color = '';
    olcer.style.color = `color-mix(in srgb, ${renk} 100%, transparent)`;
    if (!olcer.style.color) olcer.style.color = renk;
    const c = getComputedStyle(olcer).color;
    let r, g, b;
    let m = c.match(/^rgba?\(\s*([\d.]+)[ ,]+([\d.]+)[ ,]+([\d.]+)/);
    if (m) [r, g, b] = m.slice(1, 4).map(Number);
    else if ((m = c.match(/^color\(srgb\s+([-\d.e]+)\s+([-\d.e]+)\s+([-\d.e]+)/))) [r, g, b] = m.slice(1, 4).map((x) => Number(x) * 255);
    else return '#000000';
    return '#' + [r, g, b].map((x) => Math.round(Math.min(255, Math.max(0, x))).toString(16).padStart(2, '0')).join('');
  }

  window.OrhuncaTema = { DEGISKENLER, HAZIR, ARAYUZ_YAZI, KOD_YAZI, dogrula, uygula, hesaplanan, onaltilik, guvenliCss };
})();
