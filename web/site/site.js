// Tanıtım sayfası: işletim sistemine göre indirme, son sürüm bilgisi, örnek sekmeleri.
(function () {
  'use strict';
  const DEPO = 'Furkan003/Orhunca';
  const indirmeAdresi = (ad) => `https://github.com/${DEPO}/releases/latest/download/${ad}`;
  const $$ = (s, k = document) => [...k.querySelectorAll(s)];

  // Menü (dar ekran)
  const menu = document.getElementById('menuDugme');
  const gezinti = document.getElementById('gezinti');
  menu?.addEventListener('click', () => {
    const acik = gezinti.classList.toggle('acik');
    menu.setAttribute('aria-expanded', String(acik));
  });
  $$('#gezinti a').forEach((a) => a.addEventListener('click', () => gezinti.classList.remove('acik')));

  // İndirme bağlantıları
  $$('[data-dosya]').forEach((a) => { a.href = indirmeAdresi(a.dataset.dosya); });

  // İşletim sistemi
  function sistemBul() {
    const ua = navigator.userAgent;
    const p = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || '';
    if (/Win/i.test(p) || /Windows/i.test(ua)) return 'windows';
    if (/Mac/i.test(p) || /Mac OS X/i.test(ua)) return /iPhone|iPad/.test(ua) ? null : 'macos';
    if (/Linux|X11|CrOS/i.test(p + ua) && !/Android/i.test(ua)) return 'linux';
    return null;
  }
  const ADLAR = { windows: 'Windows', linux: 'Linux · Pardus', macos: 'macOS' };
  const ANA_DOSYA = {
    windows: 'Orhunca-Studyo-Windows-Kurulum.exe',
    linux: 'orhunca-studyo_amd64.deb',
    macos: 'Orhunca-Studyo-macOS.dmg',
  };
  function sistemSec(s) {
    $$('.sistem').forEach((d) => d.setAttribute('aria-selected', String(d.dataset.sistem === s)));
    $$('[data-sistem-pano]').forEach((p) => { p.hidden = p.dataset.sistemPano !== s; });
  }
  $$('.sistem').forEach((d) => d.addEventListener('click', () => sistemSec(d.dataset.sistem)));
  const sistem = sistemBul();
  if (sistem) {
    sistemSec(sistem);
    const g = document.getElementById('girisIndir');
    if (g) {
      g.href = indirmeAdresi(ANA_DOSYA[sistem]);
      document.getElementById('girisIndirAlt').textContent = ADLAR[sistem] + ' için · ücretsiz';
    }
  }

  // Kopyala düğmeleri
  $$('.kopyala').forEach((d) => d.addEventListener('click', async () => {
    const metin = d.parentElement.querySelector('code').textContent;
    try { await navigator.clipboard.writeText(metin); d.textContent = 'Kopyalandı'; } catch { d.textContent = 'Seçip kopyalayın'; }
    setTimeout(() => { d.textContent = 'Kopyala'; }, 1600);
  }));

  // Örnek sekmeleri
  const ORNEK_DOSYA = { fiil: 'selam.ohc', arayuz: 'sayaç.ohc', web: 'okul.ohc' };
  const ORNEK_DENE = { fiil: 'selam', arayuz: 'sayac', web: null };
  $$('.sekme[data-ornek]').forEach((d) => d.addEventListener('click', () => {
    const o = d.dataset.ornek;
    $$('.sekme[data-ornek]').forEach((x) => x.setAttribute('aria-selected', String(x === d)));
    $$('[data-ornek-kod]').forEach((p) => { p.hidden = p.dataset.ornekKod !== o; });
    $$('[data-ornek-aciklama]').forEach((p) => { p.hidden = p.dataset.ornekAciklama !== o; });
    document.getElementById('ornekAd').textContent = ORNEK_DOSYA[o];
    const dene = document.getElementById('ornekDene');
    dene.hidden = !ORNEK_DENE[o];
    if (ORNEK_DENE[o]) dene.href = 'dene.html#ornek=' + ORNEK_DENE[o];
  }));

  // Son sürüm: sürüm numarası, tarih ve dosya boyutları (GitHub API; olmazsa sessizce geçilir)
  const boyut = (b) => (b > 1048576 ? (b / 1048576).toFixed(1) + ' MB' : Math.max(1, Math.round(b / 1024)) + ' KB');
  fetch(`https://api.github.com/repos/${DEPO}/releases/latest`, { headers: { Accept: 'application/vnd.github+json' } })
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((s) => {
      const tarih = new Date(s.published_at).toLocaleDateString('tr-TR', { day: 'numeric', month: 'long', year: 'numeric' });
      const bilgi = document.getElementById('surumBilgi');
      if (bilgi) bilgi.innerHTML = `Son sürüm <b>${s.tag_name}</b> · ${tarih} · <a href="${s.html_url}">Sürüm notları</a>`;
      const rozet = document.getElementById('surumRozet');
      if (rozet) rozet.textContent = `${s.tag_name} yayımlandı · ücretsiz`;
      const varliklar = new Map(s.assets.map((a) => [a.name, a]));
      $$('.dosya[data-dosya]').forEach((d) => {
        const a = varliklar.get(d.dataset.dosya);
        const b = d.querySelector('.boyut');
        if (a && b) b.textContent = boyut(a.size);
      });
    })
    .catch(() => {});
})();
