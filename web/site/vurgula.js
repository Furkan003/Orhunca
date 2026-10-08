// Orhunca sözdizimi renklendirmesi (site ve deneme sayfası): anahtar kelimeler,
// metinler, sayılar, yorumlar, ekler ('yi, 'den) ve işlev çağrıları.
(function (kok) {
  'use strict';
  const ANAHTAR = new Set([
    'eğer', 'değilse', 'ise', 'her', 'için', 'kadar', 'işlev', 'fiil', 'kullan', 'kütüphane', 'sabit', 'döndür', 'dur',
    'sürdür', 've', 'veya', 'değil', 'doğru', 'yanlış', 'olduğu', 'sürece', 'iken', 'yaz', 'ekle', 'sırala',
    'çıkar', 'ekrana', 'dene', 'yakala', 'seçenek', 'model', 'durum', 'arayüz', 'bileşen', 'tıklanınca',
    'değişince', 'gönderilince', 'çalınca', 'al', 'gönder', 'koy', 'sil', 'kaydet', 'boş',
  ]);
  const TIPLER = new Set(['sayı', 'ondalık', 'metin', 'mantık', 'liste', 'sözlük']);
  const kac = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const HARF = /[\p{L}_]/u;
  const KELIME = /[\p{L}\p{N}_]/u;

  function satir(s) {
    let o = '', i = 0;
    while (i < s.length) {
      const c = s[i];
      if (c === '#') { o += `<span class="k-y">${kac(s.slice(i))}</span>`; break; }
      if (c === '"') {
        let j = i + 1;
        while (j < s.length && s[j] !== '"') { if (s[j] === '\\') j++; j++; }
        o += `<span class="k-m">${kac(s.slice(i, j + 1))}</span>`;
        i = j + 1;
        continue;
      }
      if (c === "'" && i > 0 && /[\p{L}\p{N}_)\]"]/u.test(s[i - 1])) {
        let j = i + 1;
        while (j < s.length && KELIME.test(s[j])) j++;
        o += `<span class="k-e">${kac(s.slice(i, j))}</span>`;
        i = j;
        continue;
      }
      if (/[0-9]/.test(c) && !(i > 0 && KELIME.test(s[i - 1]))) {
        let j = i;
        while (j < s.length && /[0-9.]/.test(s[j])) j++;
        o += `<span class="k-s">${s.slice(i, j)}</span>`;
        i = j;
        continue;
      }
      if (HARF.test(c)) {
        let j = i;
        while (j < s.length && KELIME.test(s[j])) j++;
        const k = s.slice(i, j);
        if (ANAHTAR.has(k)) o += `<span class="k-a">${k}</span>`;
        else if (TIPLER.has(k) && s[j] !== '(') o += `<span class="k-a">${k}</span>`;
        else if (s[j] === '(') o += `<span class="k-f">${k}</span>`;
        else o += kac(k);
        i = j;
        continue;
      }
      o += kac(c);
      i++;
    }
    return o;
  }

  function vurgula(metin) {
    return metin.split('\n').map(satir).join('\n');
  }
  kok.OrhuncaVurgu = { vurgula, satir };
  if (typeof document !== 'undefined') {
    document.querySelectorAll('pre[data-orhunca]').forEach((p) => { p.innerHTML = vurgula(p.textContent); });
  }
})(typeof globalThis !== 'undefined' ? globalThis : this);
