/* Orhunca Stüdyo — Yardımcılar: seçici, kaçış, biçimlendirme, ayarlar.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Yardımcılar
  // =====================================================================
  const $ = (s, k = document) => k.querySelector(s);
  const kac = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const S = (ad, sinif = '', stil = '') => `<span class="simge ${sinif}"${stil ? ` style="${stil}"` : ''}>${ad}</span>`;
  const GOKTURK = '𐰆';
  const TAURI = !!window.__TAURI__;
  const ilkBuyuk = s => s ? s.charAt(0).toLocaleUpperCase('tr') + s.slice(1) : s;
  const buyuk = s => String(s).toLocaleUpperCase('tr');
  const kucuk = s => String(s).toLocaleLowerCase('tr');
  const iki = n => String(n).padStart(2, '0');
  const tarihBicim = sn => { const d = new Date(sn * 1000); return `${iki(d.getDate())}.${iki(d.getMonth() + 1)}.${d.getFullYear()} ${iki(d.getHours())}:${iki(d.getMinutes())}`; };
  const sureBicim = ms => { const s = ms / 1000; return (s < 0.1 ? s.toFixed(2) : s.toFixed(1)).replace('.', ',') + ' sn'; };
  const boyutBicim = b => b > 1048576 ? (b / 1048576).toFixed(1).replace('.', ',') + ' MB' : Math.max(1, Math.round(b / 1024)) + ' KB';
  const uzanti = ad => { const i = ad.lastIndexOf('.'); return i > 0 ? ad.slice(i + 1).toLowerCase() : ''; };
  const sonParca = yol => yol.split(/[\\/]/).pop();
  const bekle = ms => new Promise(r => setTimeout(r, ms));
  const ayarOku = (a, v) => { try { const x = localStorage.getItem('orhunca.' + a); return x === null ? v : JSON.parse(x); } catch { return v; } };
  const ayarYaz = (a, v) => { try { localStorage.setItem('orhunca.' + a, JSON.stringify(v)); } catch { /* yok */ } };

  // Oturum anahtarı: sunucu tarayıcıyı /#anahtar=... ile açar (`#` sonrası sunucuya gitmez).
  // Açılışta açılacak proje ya da dosya (masaüstünde çift tıklanan .ohc): &ac=<tam yol>
  // Eski biçim (?anahtar=...) de okunur.
  const ADRES_PARAMETRELERI = (() => {
    const h = new URLSearchParams(location.hash.slice(1)), q = new URL(location.href).searchParams;
    return ad => h.get(ad) ?? q.get(ad);
  })();
  const ACILACAK = ADRES_PARAMETRELERI('ac');
  // Açık sekmeye yeni bir anahtarlı adres gelirse (ör. Stüdyo yeniden başlatıldı) sayfa yenilenir.
  window.addEventListener('hashchange', () => { if (/anahtar=/.test(location.hash)) location.reload(); });
  const ANAHTAR = (() => {
    let a = ADRES_PARAMETRELERI('anahtar');
    if (a) {
      try { sessionStorage.setItem('orhunca-anahtar', a); } catch { /* yok */ }
      history.replaceState(null, '', location.pathname);
    } else {
      try { a = sessionStorage.getItem('orhunca-anahtar'); } catch { /* yok */ }
    }
    return a;
  })();

  async function api(yol, govde) {
    const r = await fetch(yol, {
      method: govde === undefined ? 'GET' : 'POST',
      headers: { 'X-Orhunca-Anahtar': ANAHTAR || '', 'Content-Type': 'application/json' },
      body: govde === undefined ? undefined : JSON.stringify(govde),
    });
    const j = await r.json().catch(() => ({ hata: 'Sunucudan geçersiz yanıt geldi.' }));
    if (r.status === 403) throw new Error(j.hata || 'Erişim reddedildi.');
    return j;
  }
  const sorgu = o => Object.entries(o).map(([a, d]) => `${a}=${encodeURIComponent(d)}`).join('&');

