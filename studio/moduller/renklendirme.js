/* Orhunca Stüdyo — Orhunca sözdizimi renklendirme.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Orhunca sözdizimi renklendirme
  // =====================================================================
  const ANAHTAR_KELIMELER = new Set(['eğer', 'değilse', 'ise', 'her', 'için', 'kadar', 'işlev', 'fiil', 'döndür', 'dur', 'sürdür', 'dene', 'yakala', 'seçenek', 've', 'veya', 'değil', 'olduğu', 'sürece', 'iken', 'kullan', 'kütüphane', 'sabit', 'doğru', 'yanlış', 'ekrana', 'tıklanınca', 'değişince', 'gönderilince', 'çalınca']);
  // Arayüz dilinin kapsayıcı öğeleri (`satır:`) parantezsiz de yazılır.
  const KAPSAYICILAR = new Set(['satır', 'sütun', 'kart', 'kutu', 'ızgara']);
  const YERLESIK_FIILLER = new Set(['yaz', 'ekle', 'sırala', 'çıkar', 'kaydet']);
  // Model alanı nitelikleri ve web yolu kelimeleri (yalnızca yerinde anahtar kelimedir)
  const NITELIKLER = new Set(['zorunlu', 'en_az', 'en_fazla', 'e_posta', 'etiket', 'birincil']);
  const ROTA = /^\s*(al|gönder|koy|sil)(?=\s+")/u;
  const TIPLER = new Set(['sayı', 'ondalık', 'metin', 'mantık', 'liste', 'sözlük']);
  const YUKLEM = /^(büyük|küçük|eşit|değil)(se|sa|ken)?$/;
  const HARF = 'A-Za-zÇĞİÖŞÜçğıöşüÂâÎîÛû_';
  const SOZCUK = new RegExp(`(#.*$)|("(?:[^"\\\\]|\\\\.)*"?)|(\\d[\\d_]*(?:\\.\\d+)?)|(['’][${HARF}]+)|([${HARF}][${HARF}0-9]*)|(\\s+)|(.)`, 'gu');
  let kullaniciFiilleri = new Set();

  function fiilleriTopla() {
    kullaniciFiilleri = new Set();
    for (const s of D.sekmeler) {
      if (!s.icerik) continue;
      for (const m of s.icerik.matchAll(/^\s*fiil\b(.*?)([^\s:'’()]+)\s*(->[^:]*)?:\s*$/gmu)) kullaniciFiilleri.add(m[2]);
    }
  }

  function vurgulaSatir(satir) {
    let html = '', m;
    // `al "/ürünler":` ve `model Ürün:` satır başında anahtar kelimedir.
    const r = ROTA.exec(satir) || /^(model)(?=\s+[\p{L}_][\p{L}\p{N}_]*\s*:\s*$)/u.exec(satir)
      || /^(durum)(?=\s+[\p{L}_][\p{L}\p{N}_]*\s*[=:])/u.exec(satir) || /^(bileşen)(?=\s+[\p{L}_][\p{L}\p{N}_]*\s*\()/u.exec(satir)
      || /^(arayüz)(?=\s*:\s*$)/u.exec(satir);
    if (r) {
      html = kac(satir.slice(0, r.index + r[0].length - r[1].length)) + `<span class="k">${kac(r[1])}</span>`;
      satir = satir.slice(r.index + r[0].length);
    }
    const alanSatiri = /^\s+[\p{L}_][\p{L}\p{N}_]*\s*:\s*[\p{L}]/u.test(satir);
    SOZCUK.lastIndex = 0;
    let onceki = '';
    while ((m = SOZCUK.exec(satir))) {
      const t = m[0];
      let c = '';
      if (m[1]) c = 'c';
      else if (m[2]) c = 's';
      else if (m[3]) c = 'n';
      else if (m[4]) c = 'e';
      else if (m[5]) {
        const sonra = satir.slice(SOZCUK.lastIndex).trimStart()[0];
        if (ANAHTAR_KELIMELER.has(t) || YUKLEM.test(t) || (alanSatiri && NITELIKLER.has(t))) c = 'k';
        else if (YERLESIK_FIILLER.has(t) || kullaniciFiilleri.has(t) || /^uzunluğu/.test(t)) c = 'f';
        else if (sonra === '(' || (sonra === ':' && KAPSAYICILAR.has(t))) c = 'f';
        else if (TIPLER.has(t) && /(:|->|<|,)$/.test(onceki)) c = 't';
      }
      if (!m[6]) onceki = (onceki + t).slice(-2);
      html += c ? `<span class="${c}">${kac(t)}</span>` : kac(t);
    }
    return html;
  }

  /** Dosya uzantısına göre renklendirme dili */
  function dilBul(yol) {
    const u = uzanti(yol);
    if (u === 'ohc') return 'ohc';
    if (u === 'ohchtml') return 'ohchtml';
    if (u === 'html' || u === 'htm' || u === 'svg' || u === 'xml') return 'html';
    if (u === 'css') return 'css';
    if (u === 'js' || u === 'mjs' || u === 'json') return 'js';
    return u === 'ohcproj' ? 'ohc' : 'duz';
  }

  const sarmala = (c, t) => c ? `<span class="${c}">${kac(t)}</span>` : kac(t);

  // .ohchtml: HTML + @ ile gömülü Orhunca
  const OHCHTML_YONERGE = /^@(eğer|değilse|her|model|düzen|başlık)(?![\p{L}\p{N}_])/u;
  const HTML_SOZCUK = /(<!--.*?(?:-->|$))|(<\/?[A-Za-z][\w:-]*|\/?>)|("[^"]*"?|'[^']*'?)|(@@|@\*.*?(?:\*@|$)|@[\p{L}_][\p{L}\p{N}_]*(?:\.[\p{L}_][\p{L}\p{N}_]*)*|@\(|[{}])|([\p{L}_][\p{L}\p{N}_:-]*(?==))|(\s+)|(&\w+;)|(.)/gu;
  const GOMULU = /@@|@[\p{L}_][\p{L}\p{N}_]*(?:\.[\p{L}_][\p{L}\p{N}_]*)*/gu;

  function vurgulaHtml(satir, orhunca) {
    let html = '', m;
    const R = new RegExp(HTML_SOZCUK.source, 'gu');
    while ((m = R.exec(satir))) {
      const t = m[0];
      if (m[1]) html += sarmala('c', t);
      else if (m[2]) html += sarmala('f', t);
      else if (m[3]) {
        // Tırnak içindeki @ifadeler de vurgulanır: href="/ürünler/@ü.kimlik"
        html += orhunca ? `<span class="s">${kac(t).replace(GOMULU, x => x === '@@' ? x : `<span class="e">${x}</span>`)}</span>` : sarmala('s', t);
      } else if (m[4] && orhunca) {
        if (t.startsWith('@*')) { html += sarmala('c', t); continue; }
        const y = OHCHTML_YONERGE.exec(satir.slice(m.index));
        if (y) {
          // Yönerge ve satırın geri kalanı (son `{`'ye kadar) Orhunca kodudur.
          html += sarmala('k', y[0]);
          let kalan = satir.slice(m.index + y[0].length);
          const ac = kalan.indexOf('{');
          const kod = ac >= 0 && y[1] !== 'model' && y[1] !== 'düzen' && y[1] !== 'başlık' ? kalan.slice(0, ac) : kalan;
          html += vurgulaSatir(kod);
          if (kod.length < kalan.length) html += sarmala('k', kalan.slice(kod.length));
          break;
        }
        html += sarmala(t === '{' || t === '}' ? 'k' : 'e', t);
      } else if (m[5]) html += sarmala('t', t);
      else html += kac(t);
    }
    return html;
  }

  const CSS_SOZCUK = /(\/\*.*?(?:\*\/|$))|("[^"]*"?|'[^']*'?)|(#[0-9a-fA-F]{3,8}\b)|(-?\d*\.?\d+(?:px|em|rem|%|vh|vw|vmin|vmax|ch|s|ms|fr|deg)?\b%?)|(@[\w-]+)|([\w-]+)(?=\s*:(?!:))|([\w.#:-]+)|(\s+)|(.)/g;
  function cssParca(metin, secici) {
    let html = '', m;
    const R = new RegExp(CSS_SOZCUK.source, 'g');
    while ((m = R.exec(metin))) {
      const t = m[0];
      const c = m[1] ? 'c' : m[2] ? 's' : m[5] ? 'k' : secici ? (m[6] || m[7] ? 'f' : '') : m[3] || m[4] ? 'n' : m[6] ? 't' : '';
      html += sarmala(c, t);
    }
    return html;
  }

  /** Satırdaki ilk `{`'den öncesi seçici, sonrası özellikler sayılır. */
  function vurgulaCss(satir) {
    const i = satir.indexOf('{');
    if (i < 0 || /^\s*\/\*/.test(satir)) return cssParca(satir, false);
    return cssParca(satir.slice(0, i), true) + '{' + cssParca(satir.slice(i + 1), false);
  }

  const JS_ANAHTAR = new Set(['const', 'let', 'var', 'function', 'return', 'if', 'else', 'for', 'while', 'of', 'in', 'new', 'async', 'await', 'try', 'catch', 'finally', 'throw', 'class', 'import', 'export', 'from', 'default', 'true', 'false', 'null', 'undefined', 'this', 'typeof', 'break', 'continue', 'switch', 'case']);
  const JS_SOZCUK = /(\/\/.*$)|(`(?:[^`\\]|\\.)*`?|"(?:[^"\\]|\\.)*"?|'(?:[^'\\]|\\.)*'?)|(\b\d+(?:\.\d+)?\b)|([\p{L}_$][\p{L}\p{N}_$]*)|(\s+)|(.)/gu;
  function vurgulaJs(satir) {
    let html = '', m;
    const R = new RegExp(JS_SOZCUK.source, 'gu');
    while ((m = R.exec(satir))) {
      const t = m[0];
      let c = m[1] ? 'c' : m[2] ? 's' : m[3] ? 'n' : '';
      if (m[4]) c = JS_ANAHTAR.has(t) ? 'k' : satir.slice(R.lastIndex).trimStart()[0] === '(' ? 'f' : '';
      html += sarmala(c, t);
    }
    return html;
  }

  function vurgula(satir, dil) {
    switch (dil) {
      case 'ohc': return vurgulaSatir(satir);
      case 'ohchtml': return vurgulaHtml(satir, true);
      case 'html': return vurgulaHtml(satir, false);
      case 'css': return vurgulaCss(satir);
      case 'js': return vurgulaJs(satir);
      default: return kac(satir);
    }
  }

