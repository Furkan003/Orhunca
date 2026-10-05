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
  // ---------------------------------------------------------------------
  // Görsel etkiler (azaltılmış hareket tercihine uyulur)
  // ---------------------------------------------------------------------
  const azHareket = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const kac = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

  // Kaydırınca beliren bölümler
  if (!azHareket && 'IntersectionObserver' in window) {
    document.documentElement.classList.add('belirme');
    const izle = new IntersectionObserver((girdiler) => {
      girdiler.forEach((g) => {
        if (!g.isIntersecting) return;
        // Aynı satırdaki kartlar sırayla belirir
        const kardesler = [...g.target.parentElement.children].filter((x) => x.classList.contains('belir'));
        g.target.style.transitionDelay = Math.min(kardesler.indexOf(g.target), 5) * 70 + 'ms';
        g.target.classList.add('gorundu');
        izle.unobserve(g.target);
      });
    }, { rootMargin: '0px 0px -8% 0px' });
    $$('.belir').forEach((e) => izle.observe(e));
  }

  // Giriş: kod yazılıyormuş gibi görünür, ardından çıktı gelir
  const kod = document.getElementById('girisKod');
  const cikti = document.getElementById('girisCikti');
  if (kod && cikti && window.OrhuncaVurgu && !azHareket) {
    const tam = kod.textContent;
    const ciktiSatirlar = cikti.innerHTML.split('\n');
    kod.style.minHeight = kod.offsetHeight + 'px';
    cikti.style.minHeight = cikti.offsetHeight + 'px';
    cikti.innerHTML = '';
    let i = 0;
    const yaz = () => {
      // Girinti ve satır sonları tek adımda; diğer karakterler tek tek
      do { i++; } while (i < tam.length && (tam[i - 1] === ' ' && tam[i] === ' '));
      kod.innerHTML = window.OrhuncaVurgu.vurgula(tam.slice(0, i)) + '<span class="imlec"></span>';
      if (i < tam.length) setTimeout(yaz, tam[i - 1] === '\n' ? 90 : 12 + Math.random() * 18);
      else setTimeout(calistir, 500);
    };
    const calistir = () => {
      kod.innerHTML = window.OrhuncaVurgu.vurgula(tam);
      document.querySelector('.calistir-rozet')?.classList.add('basildi');
      ciktiSatirlar.forEach((s, n) => setTimeout(() => {
        cikti.innerHTML = ciktiSatirlar.slice(0, n + 1).join('\n');
      }, 250 + n * 380));
    };
    kod.innerHTML = '<span class="imlec"></span>';
    setTimeout(yaz, 600);
  }

  // Hâl ekleri: örnek cümleler
  const CUMLELER = [
    { parcalar: [['5', "'i", 'b'], ['sayılar', 'a', 'y'], ['ekle', '.', 'f']], aciklama: '<b>Neyi?</b> 5\'i · <b>nereye?</b> sayılara · <b>ne yap?</b> ekle. Listenin sonuna 5 eklenir.' },
    { parcalar: [['sayılar', 'a', 'y'], ['5', "'i", 'b'], ['ekle', '.', 'f']], aciklama: 'Sıra önemsiz: görevi ekler anlattığı için bu da aynı cümle.' },
    { parcalar: [['5', "'i", 'b'], ['sayılar', 'dan', 'a'], ['çıkar', '.', 'f']], aciklama: '<b>Nereden?</b> Ayrılma hâli (-dan, -den) kaynağı gösterir: 5 listeden çıkarılır.' },
    { parcalar: [['"Ayşe"', "'yi", 'b'], ['selamla', '.', 'f']], aciklama: 'Kendi fiillerini de tanımlarsın: <code>fiil (kişi: metin)\'yi selamla:</code>' },
    { parcalar: [['eğer x', '', 'k'], ['10', "'dan", 'a'], ['büyük', 'se:', 'f']], aciklama: 'Koşullar da Türkçe: <b>10\'dan büyükse</b>. Simgelerle de yazabilirsin: <code>eğer x > 10 ise:</code>' },
  ];
  const HAL = { b: ['Neyi?', 'belirtme'], y: ['Nereye?', 'yönelme'], a: ['Nereden?', 'ayrılma'], f: ['Ne yap?', 'yüklem'], k: ['', ''] };
  const cumle = document.getElementById('cumle');
  if (cumle) {
    const aciklama = document.getElementById('cumleAciklama');
    const noktalar = document.getElementById('cumleNoktalar');
    let sira = 0, zaman = null;
    noktalar.innerHTML = CUMLELER.map((_, n) => `<button role="tab" aria-label="Örnek ${n + 1}" data-n="${n}"></button>`).join('');
    const goster = (n) => {
      sira = n;
      const c = CUMLELER[n];
      cumle.classList.remove('yeni'); void cumle.offsetWidth; cumle.classList.add('yeni');
      cumle.innerHTML = c.parcalar.map(([kok, ek, tur], j) => `<span class="parca hal-${tur}" style="animation-delay:${j * 90}ms">
        <span class="parca-metin"><span class="kok">${kac(kok)}</span><span class="ek">${kac(ek)}</span></span>
        ${HAL[tur][0] ? `<span class="parca-etiket"><b>${HAL[tur][0]}</b>${HAL[tur][1]}</span>` : '<span class="parca-etiket"></span>'}</span>`).join('');
      aciklama.innerHTML = c.aciklama;
      $$('button', noktalar).forEach((b, j) => b.setAttribute('aria-selected', String(j === n)));
    };
    const ileri = () => goster((sira + 1) % CUMLELER.length);
    const baslat = () => { if (!azHareket) { clearInterval(zaman); zaman = setInterval(ileri, 4200); } };
    noktalar.addEventListener('click', (e) => { const b = e.target.closest('button'); if (b) { goster(+b.dataset.n); baslat(); } });
    document.getElementById('cumleKart').addEventListener('mouseenter', () => clearInterval(zaman));
    document.getElementById('cumleKart').addEventListener('mouseleave', baslat);
    goster(0); baslat();
  }

  // Stüdyo vitrini
  const resim = document.getElementById('vitrinResim');
  if (resim) {
    const sekmeler = $$('[data-vitrin]');
    let zaman = null, kullanici = false;
    const sec = (d) => {
      sekmeler.forEach((x) => x.setAttribute('aria-selected', String(x === d)));
      resim.classList.add('soluk');
      const yeni = new Image();
      yeni.src = 'ekran/' + d.dataset.vitrin + '.png';
      const bitir = () => { resim.src = yeni.src; resim.alt = 'Orhunca Stüdyo: ' + d.querySelector('b').textContent; resim.classList.remove('soluk'); };
      yeni.decode ? yeni.decode().then(bitir, bitir) : (yeni.onload = bitir);
    };
    sekmeler.forEach((d) => d.addEventListener('click', () => { kullanici = true; clearInterval(zaman); sec(d); }));
    // Görünürken kendiliğinden ilerler; kullanıcı seçince durur
    if (!azHareket && 'IntersectionObserver' in window) {
      new IntersectionObserver(([g]) => {
        clearInterval(zaman);
        if (g.isIntersecting && !kullanici) zaman = setInterval(() => {
          const n = sekmeler.findIndex((x) => x.getAttribute('aria-selected') === 'true');
          sec(sekmeler[(n + 1) % sekmeler.length]);
        }, 4500);
      }, { threshold: 0.4 }).observe(resim);
    }
  }
})();
