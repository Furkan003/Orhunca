/* Orhunca Stüdyo — Ekstralar: resim, Markdown, CSV/JSON önizleme; sorun süzgeci; kod
 * parçacıkları; YAPILACAKLAR; yer imleri; birden çok kabuk; dosya karşılaştırma; sınama kapsamı.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  const RESIM = /\.(png|jpe?g|gif|webp|svg|bmp|ico)$/i;
  const ONIZLENIR = /\.(md|csv|tsv|json|svg)$/i;
  D.onizlenen = new Set();

  // =====================================================================
  // Önizleme: resim (ana alanda), Markdown / CSV / JSON (sağ bölmede)
  // =====================================================================
  const resimAdresleri = new Map();
  async function resimAdresi(yol) {
    if (resimAdresleri.has(yol)) return resimAdresleri.get(yol);
    const r = await fetch('/api/ham?' + sorgu({ yol: tamYol(yol) }), { headers: { 'X-Orhunca-Anahtar': ANAHTAR || '' } });
    if (!r.ok) throw new Error((await r.json().catch(() => ({}))).hata || 'Resim okunamadı.');
    const a = URL.createObjectURL(await r.blob());
    resimAdresleri.set(yol, a);
    return a;
  }

  function mdHtml(md) {
    const satir = t => kac(t)
      .replace(/!\[([^\]]*)\]\(([^)\s]+)\)/g, (_, a, u) => /^https?:/.test(u) ? `<img alt="${a}" src="${u}">` : `<span class="md-resim">[resim: ${a || u}]</span>`)
      .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
      .replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/(^|[^*])\*([^*]+)\*/g, '$1<i>$2</i>').replace(/`([^`]+)`/g, '<code>$1</code>');
    const satirlar = md.split(/\r?\n/);
    let html = '', liste = null, kod = null, tablo = null;
    const kapat = () => { if (liste) { html += `</${liste}>`; liste = null; } if (tablo) { html += '</table>'; tablo = null; } };
    for (const l of satirlar) {
      if (kod !== null) { if (/^\s*```/.test(l)) { html += `<pre><code>${kac(kod)}</code></pre>`; kod = null; } else kod += (kod ? '\n' : '') + l; continue; }
      if (/^\s*```/.test(l)) { kapat(); kod = ''; continue; }
      let m;
      if (/^\s*\|.*\|\s*$/.test(l)) {
        if (/^\s*\|?\s*:?-{3,}/.test(l)) continue;
        const h = l.trim().replace(/^\||\|$/g, '').split('|').map(x => satir(x.trim()));
        if (!tablo) { kapat(); html += '<table><tr>' + h.map(x => `<th>${x}</th>`).join('') + '</tr>'; tablo = true; }
        else html += '<tr>' + h.map(x => `<td>${x}</td>`).join('') + '</tr>';
        continue;
      }
      if ((m = l.match(/^\s*[-*+]\s+(?:\[([ xX])\]\s+)?(.*)/))) { if (liste !== 'ul') { kapat(); html += '<ul>'; liste = 'ul'; } html += `<li>${m[1] !== undefined ? `<input type="checkbox" disabled ${m[1] !== ' ' ? 'checked' : ''}> ` : ''}${satir(m[2])}</li>`; continue; }
      if ((m = l.match(/^\s*\d+[.)]\s+(.*)/))) { if (liste !== 'ol') { kapat(); html += '<ol>'; liste = 'ol'; } html += `<li>${satir(m[1])}</li>`; continue; }
      kapat();
      if ((m = l.match(/^(#{1,6})\s+(.*)/))) html += `<h${m[1].length}>${satir(m[2])}</h${m[1].length}>`;
      else if ((m = l.match(/^>\s?(.*)/))) html += `<blockquote>${satir(m[1])}</blockquote>`;
      else if (/^\s*(-{3,}|\*{3,})\s*$/.test(l)) html += '<hr>';
      else if (l.trim()) html += `<p>${satir(l)}</p>`;
    }
    if (kod !== null) html += `<pre><code>${kac(kod)}</code></pre>`;
    kapat();
    return html;
  }

  /** CSV/TSV satırları (tırnaklı alanlar desteklenir; ayraç , ; ya da sekme). */
  function csvOku(metin, ayrac) {
    const ilk = metin.split('\n')[0] || '';
    ayrac ||= ['\t', ';', ','].map(a => [a, ilk.split(a).length]).sort((a, b) => b[1] - a[1])[0][0];
    const satirlar = [];
    let alan = '', satir = [], tirnak = false;
    for (let i = 0; i < metin.length; i++) {
      const c = metin[i];
      if (tirnak) { if (c === '"' && metin[i + 1] === '"') { alan += '"'; i++; } else if (c === '"') tirnak = false; else alan += c; }
      else if (c === '"') tirnak = true;
      else if (c === ayrac) { satir.push(alan); alan = ''; }
      else if (c === '\n') { satir.push(alan.replace(/\r$/, '')); satirlar.push(satir); satir = []; alan = ''; }
      else alan += c;
      if (satirlar.length > 2000) break;
    }
    if (alan || satir.length) { satir.push(alan); satirlar.push(satir); }
    return satirlar;
  }
  function tabloHtml(basliklar, satirlar) {
    return `<div class="tablo-bilgi">${satirlar.length} satır · ${basliklar.length} sütun</div><table class="veri-tablosu"><tr>${basliklar.map(b => `<th>${kac(b)}</th>`).join('')}</tr>${satirlar.slice(0, 1000).map(s => `<tr>${basliklar.map((_, i) => `<td>${kac(s[i] ?? '')}</td>`).join('')}</tr>`).join('')}</table>`;
  }
  function jsonHtml(metin) {
    let v;
    try { v = JSON.parse(metin); } catch (e) { return `<div class="modal-hata">JSON okunamadı: ${kac(e.message)}</div>`; }
    if (!Array.isArray(v) && v && typeof v === 'object') { const d = Object.values(v).find(Array.isArray); if (d) v = d; }
    if (Array.isArray(v) && v.length && v.every(x => x && typeof x === 'object' && !Array.isArray(x))) {
      const b = [...new Set(v.flatMap(Object.keys))];
      return tabloHtml(b, v.map(x => b.map(k => typeof x[k] === 'object' ? JSON.stringify(x[k]) : String(x[k] ?? ''))));
    }
    return `<pre class="json-agac">${kac(JSON.stringify(v, null, 2))}</pre>`;
  }
  function onizlemeHtml(s) {
    if (/\.md$/i.test(s.yol)) return `<div class="md-onizleme">${mdHtml(s.icerik)}</div>`;
    // SVG <img> içinde gösterilir: içindeki betikler çalışmaz.
    if (/\.svg$/i.test(s.yol)) return `<div class="resim-onizleme"><img src="data:image/svg+xml;charset=utf-8,${encodeURIComponent(s.icerik)}" alt=""></div>`;
    if (/\.json$/i.test(s.yol)) return jsonHtml(s.icerik);
    const l = csvOku(s.icerik, /\.tsv$/i.test(s.yol) ? '\t' : '');
    return l.length ? tabloHtml(l[0], l.slice(1)) : '<div class="panel-not">Dosya boş.</div>';
  }

  function onizlemeBolmesi() {
    const bolge = $('#kodBolge'), s = etkinSekme();
    $('#onizlemeBolmesi')?.remove();
    if (!bolge || !s || s.ikili || D.bolme || !D.onizlenen.has(s.yol)) return;
    bolge.classList.add('bolunmus');
    const d = document.createElement('div');
    d.id = 'onizlemeBolmesi';
    d.className = 'ikinci-bolge onizleme-bolmesi';
    d.innerHTML = `<div class="ikinci-baslik"><span class="ikinci-sec">Önizleme: ${kac(sonParca(s.yol))}</span><span class="simge" data-e="onizlemeKapat" title="Önizlemeyi kapat">close</span></div><div class="onizleme-ic">${onizlemeHtml(s)}</div>`;
    bolge.appendChild(d);
  }
  let onizlemeZamani = null;
  document.addEventListener('input', e => {
    if (e.target.id !== 'kodAlani' || !$('#onizlemeBolmesi')) return;
    clearTimeout(onizlemeZamani);
    onizlemeZamani = setTimeout(() => { const s = etkinSekme(), ic = $('#onizlemeBolmesi .onizleme-ic'); if (s && ic) { const y = ic.scrollTop; ic.innerHTML = onizlemeHtml(s); ic.scrollTop = y; } }, 250);
  });

  function onizlemeDugmesi() {
    const k = $('#kirinti'), s = etkinSekme();
    if (!k) return;
    k.querySelector('.kirinti-dugmeleri')?.remove();
    if (!s || s.ikili || !ONIZLENIR.test(s.yol)) return;
    const d = document.createElement('span');
    d.className = 'kirinti-dugmeleri';
    const acik = D.onizlenen.has(s.yol);
    d.innerHTML = `<span class="kirinti-dugme ${acik ? 'acik' : ''}" data-e="onizlemeAcKapa" title="${/\.(md|svg)$/i.test(s.yol) ? 'Önizleme' : 'Tablo olarak göster'}">${S(/\.(md|svg)$/i.test(s.yol) ? 'preview' : 'table_view')}${/\.(md|svg)$/i.test(s.yol) ? 'Önizle' : 'Tablo'}</span>`;
    k.appendChild(d);
  }

  const onizlemesizCizKod = cizKod;
  cizKod = function () {
    const s = etkinSekme();
    if (s?.ikili && RESIM.test(s.yol)) {
      const kap = $('#kodBolge');
      kap.classList.remove('bolunmus');
      kap.innerHTML = `<div class="resim-onizleme"><div class="donen kucuk"></div></div>`;
      resimAdresi(s.yol).then(a => {
        const k = $('.resim-onizleme');
        if (!k || etkinSekme() !== s) return;
        k.innerHTML = `<img src="${a}" alt="${kac(s.yol)}"><div class="resim-bilgi" id="resimBilgi">${kac(sonParca(s.yol))}</div>`;
        const img = k.querySelector('img');
        img.onload = () => { const b = $('#resimBilgi'); if (b) b.textContent = `${sonParca(s.yol)} · ${img.naturalWidth} × ${img.naturalHeight}`; };
      }).catch(e => { const k = $('.resim-onizleme'); if (k) k.innerHTML = `<div class="panel-not">${kac(e.message)}</div>`; });
      onizlemeDugmesi();
      return;
    }
    onizlemesizCizKod();
    onizlemeBolmesi();
    onizlemeDugmesi();
  };
  const ilkCizSekmeler = cizSekmeler;
  cizSekmeler = function () { ilkCizSekmeler(); onizlemeDugmesi(); };

  // =====================================================================
  // Kod parçacıkları: anahtar sözcük + Tab
  // =====================================================================
  const PARCACIKLAR = {
    'işlev': 'işlev ${ad}(${değer}: sayı) -> sayı:\n    döndür ${değer}',
    'eğer': 'eğer ${koşul} ise:\n    $0',
    'eğerd': 'eğer ${koşul} ise:\n    $0\ndeğilse:\n    ',
    'her': 'her ${öğe} için ${liste}\'den:\n    $0',
    'sürece': '${koşul} olduğu sürece:\n    $0',
    'dene': 'dene:\n    $0\nyakala hata:\n    hata\'yı yaz.',
    'model': 'model ${Ad}:\n    ad: metin, zorunlu\n    $0',
    'seçenek': 'seçenek ${Ad}: birinci, ikinci',
    'al': 'al "/${yol}":\n    döndür json_yanıtı({"ileti": "$0"})',
    'gönder': 'gönder "/${yol}":\n    döndür json_yanıtı({"alındı": değer(istek.form, "ad", "")}, 201)',
    'sına': 'işlev sına_${ad}():\n    eşit_olmalı(${gerçek}, ${beklenen})',
    'arayüz': 'arayüz:\n    başlık("${Başlık}")\n    düğme("Tıkla") tıklanınca:\n        $0',
    'bileşen': 'bileşen ${Ad}(başlık_: metin):\n    alt_başlık(başlık_)\n    $0',
    'yaz': '"${metin}"\'i yaz.',
  };
  /** Parçacığı yerleştirir; ilk ${...} seçili gelir, Tab sonrakine geçer. */
  let parcacikDurakları = [];
  function parcacikAc(ta, anahtar) {
    const sablon = PARCACIKLAR[anahtar];
    const bas = ta.selectionStart - anahtar.length;
    const satirBas = ta.value.lastIndexOf('\n', bas - 1) + 1;
    const girinti = ta.value.slice(satirBas, bas).match(/^\s*/)[0];
    let metin = '', duraklar = [];
    const parcalar = sablon.replace(/\n/g, '\n' + girinti).split(/(\$\{[^}]+\}|\$0)/);
    for (const p of parcalar) {
      const m = p.match(/^\$\{([^}]+)\}$/);
      if (m) { duraklar.push([bas + metin.length, bas + metin.length + m[1].length]); metin += m[1]; }
      else if (p === '$0') duraklar.push([bas + metin.length, bas + metin.length, true]);
      else metin += p;
    }
    ta.setSelectionRange(bas, ta.selectionStart);
    metinEkle(ta, metin);
    parcacikDurakları = duraklar.sort((a, b) => (a[2] ? 1 : 0) - (b[2] ? 1 : 0) || a[0] - b[0]);
    const ilk = parcacikDurakları.shift();
    if (ilk) ta.setSelectionRange(ilk[0], ilk[1]);
    imleciGuncelle();
  }
  let parcacikUzunluk = 0;
  document.addEventListener('keydown', e => {
    const ta = e.target;
    if (ta.id !== 'kodAlani' || e.key !== 'Tab' || e.shiftKey || e.ctrlKey || e.altKey) return;
    if ($('#tamamla')?.offsetParent) return;
    const s = etkinSekme();
    if (!s?.yol.endsWith('.ohc')) return;
    // Parçacığın sonraki durağı
    if (parcacikDurakları.length) {
      const d = parcacikDurakları.shift();
      e.preventDefault(); e.stopImmediatePropagation();
      ta.setSelectionRange(Math.min(d[0], ta.value.length), Math.min(d[1], ta.value.length));
      imleciGuncelle();
      return;
    }
    if (ta.selectionStart !== ta.selectionEnd) return;
    const once = ta.value.slice(0, ta.selectionStart);
    const m = once.match(new RegExp(`(?:^|[^${HARF}0-9])([${HARF}]+)$`, 'u'));
    if (!m || !PARCACIKLAR[m[1]]) return;
    const satirOncesi = once.slice(once.lastIndexOf('\n') + 1, once.length - m[1].length);
    if (satirOncesi.trim()) return;
    e.preventDefault(); e.stopImmediatePropagation();
    parcacikAc(ta, m[1]);
    parcacikUzunluk = ta.value.length;
  }, true);
  // Durakta yazıldıkça sonraki durakların yeri kayar.
  document.addEventListener('input', e => {
    if (e.target.id !== 'kodAlani' || !parcacikDurakları.length) return;
    const ta = e.target, fark = ta.value.length - parcacikUzunluk;
    parcacikUzunluk = ta.value.length;
    const imlec = ta.selectionStart - Math.max(fark, 0);
    for (const d of parcacikDurakları) if (d[0] >= imlec) { d[0] += fark; d[1] += fark; }
  });
  document.addEventListener('mousedown', e => { if (e.target.id === 'kodAlani') parcacikDurakları = []; });

  // =====================================================================
  // Sorunlar süzgeci
  // =====================================================================
  D.sorunSuzgec = { tur: 'hepsi', dosya: false, metin: '' };
  function sorunGorunur(h, uyari) {
    const f = D.sorunSuzgec;
    if (f.tur === 'hata' && uyari) return false;
    if (f.tur === 'uyari' && !uyari) return false;
    if (f.dosya) { const s = etkinSekme(); if (!s || !h.dosya || normal(h.dosya) !== normal(tamYol(s.yol))) return false; }
    if (f.metin && !kucuk(h.mesaj + ' ' + (h.dosya || '')).includes(kucuk(f.metin))) return false;
    return true;
  }
  function sorunSuzgecCubugu() {
    const f = D.sorunSuzgec;
    const tur = (k, ad) => `<span class="${f.tur === k ? 'secili' : ''}" data-e="sorunTur" data-a="${k}">${ad}</span>`;
    return `<div class="sorun-suzgec"><div class="tema-secim">${tur('hepsi', 'Tümü')}${tur('hata', `Hatalar (${D.sorunlar.length})`)}${tur('uyari', `Uyarılar (${D.uyarilar.length})`)}</div>
      <label class="ara-secenek"><input type="checkbox" data-e="sorunDosya" ${f.dosya ? 'checked' : ''}>Yalnızca açık dosya</label>
      <input id="sorunMetin" class="metin-girdi" data-g="sorunMetin" placeholder="Süz" value="${kac(f.metin)}" spellcheck="false"></div>`;
  }

  // =====================================================================
  // YAPILACAKLAR ve YER İMLERİ (alt panel sekmeleri)
  // =====================================================================
  D.yerimleri = ayarOku('yerimleri', {});
  D.yapilacaklar = null;
  async function yapilacaklariYukle() {
    const r = await api('/api/yapilacaklar?' + sorgu({ kok: D.proje.yol })).catch(e => ({ hata: e.message }));
    D.yapilacaklar = r.notlar || [];
    if (D.altSekme === 'yapilacaklar') cizAltPanel();
  }
  function yapilacaklarHtml() {
    if (!D.yapilacaklar) { yapilacaklariYukle(); return '<div class="tl dim">Taranıyor…</div>'; }
    if (!D.yapilacaklar.length) return '<div class="tl dim">Not yok. Kodda # YAPILACAK: ya da # TODO: diye başlayan yorumlar burada listelenir.</div>';
    return D.yapilacaklar.map(n => `<div class="sorun bilgi-satiri" data-e="konumaGit" data-a="${kac(n.dosya)}|${n.satir}"><span class="not-tur t-${kac(n.tur)}">${kac(n.tur)}</span><div><span>${kac(n.metin.replace(/^(#|\/\/|\/\*|<!--)\s*/, ''))}</span><span class="yer">${kac(n.dosya)}:${n.satir}</span></div></div>`).join('');
  }
  function yerimleriHtml() {
    const l = Object.entries(D.yerimleri).filter(([, s]) => s.length).flatMap(([d, s]) => s.map(n => [d, n]));
    if (!l.length) return '<div class="tl dim">Yer imi yok. Satırda Ctrl+Alt+K ile ekleyin; Ctrl+Alt+L / Ctrl+Alt+J ile sonrakine / öncekine gidin.</div>';
    return l.map(([d, n]) => {
      const g = goreliYol(d);
      const sekme = D.sekmeler.find(s => s.yol === g);
      const metin = sekme ? (sekme.icerik.split('\n')[n - 1] || '').trim() : '';
      return `<div class="sorun bilgi-satiri" data-e="konumaGit" data-a="${kac(g)}|${n}">${S('bookmark')}<div><span>${kac(metin || g)}</span><span class="yer">${kac(g)}:${n}</span></div><span class="duzelt" data-e="yerimiSil" data-a="${kac(d)}|${n}">Kaldır</span></div>`;
    }).join('');
  }
  function yerimiDegistir() {
    const s = etkinSekme();
    if (!s || s.ikili) return;
    const d = normal(tamYol(s.yol)), n = D.imlec.satir;
    const l = D.yerimleri[d] ||= [];
    const i = l.indexOf(n);
    if (i >= 0) l.splice(i, 1); else { l.push(n); l.sort((a, b) => a - b); }
    ayarYaz('yerimleri', D.yerimleri);
    isaretleriCiz();
    if (D.altSekme === 'yerimleri') cizAltPanel();
  }
  async function yerimineGit(yon) {
    const l = Object.entries(D.yerimleri).flatMap(([d, s]) => s.map(n => [goreliYol(d), n])).filter(([g]) => D.agac.some(x => x.yol === g));
    if (!l.length) return bildir('Yer imi yok (Ctrl+Alt+K ile ekleyin).');
    const sira = l.sort((a, b) => a[0].localeCompare(b[0]) || a[1] - b[1]);
    const simdi = [D.etkin, D.imlec.satir];
    const k = (x) => x[0].localeCompare(simdi[0]) || x[1] - simdi[1];
    const hedef = yon > 0 ? (sira.find(x => k(x) > 0) || sira[0]) : ([...sira].reverse().find(x => k(x) < 0) || sira[sira.length - 1]);
    if (hedef[0] !== D.etkin) await dosyaAc(hedef[0]);
    satiraGit(hedef[1]);
  }
  const ilkIsaretler = isaretleriCiz;
  isaretleriCiz = function () {
    ilkIsaretler();
    const s = etkinSekme(), no = $('#satirNolari');
    if (!s || !no) return;
    const tam = normal(tamYol(s.yol));
    const yi = D.yerimleri[tam] || [];
    const kap = D.kapsamGoster && D.kapsam?.[s.yol];
    [...no.children].forEach((d, i) => {
      d.classList.toggle('yerimi', yi.includes(i + 1));
      d.classList.toggle('kapsanmadi', !!kap && kap.calismayan.includes(i + 1));
    });
  };

  // Alt panel: yeni sekmeler, sorun süzgeci ve kabuk seçicisi
  const ilkCizAltPanel = cizAltPanel;
  cizAltPanel = function () {
    if (D.altSekme === 'yapilacaklar' || D.altSekme === 'yerimleri') {
      const sakla = D.altSekme;
      D.altSekme = 'cikti';
      const eskiCikti = D.cikti;
      D.cikti = [];
      ilkCizAltPanel();
      D.cikti = eskiCikti;
      D.altSekme = sakla;
      $('#terminal').innerHTML = sakla === 'yapilacaklar' ? yapilacaklarHtml() : yerimleriHtml();
    } else if (D.altSekme === 'sorunlar') {
      const [s, u] = [D.sorunlar, D.uyarilar];
      ilkCizAltPanel();
      const t = $('#terminal');
      t.insertAdjacentHTML('afterbegin', sorunSuzgecCubugu());
      t.querySelectorAll('.sorun').forEach(el => {
        const uyari = el.classList.contains('uyari'), i = +el.dataset.a;
        el.hidden = !sorunGorunur(uyari ? u[i] : s[i], uyari);
      });
      t.scrollTop = 0;
      if (document.activeElement?.id !== 'sorunMetin' && sorunOdak) { const g = $('#sorunMetin'); g?.focus(); g?.setSelectionRange(g.value.length, g.value.length); sorunOdak = false; }
    } else {
      ilkCizAltPanel();
      if (D.altSekme === 'kabuk') $('#terminal')?.insertAdjacentHTML('afterbegin', kabukSecici());
    }
    // Ek sekmeler
    const sekmeler = $('#altPanel .alt-sekmeler');
    if (sekmeler && !sekmeler.querySelector('[data-a="yapilacaklar"]')) {
      sekmeler.querySelector('[data-a="kabuk"]')?.insertAdjacentHTML('afterend', '<span data-e="altSekme" data-a="yapilacaklar">YAPILACAKLAR</span><span data-e="altSekme" data-a="yerimleri">YER İMLERİ</span>');
      sekmeler.querySelectorAll('[data-e="altSekme"]').forEach(x => x.classList.toggle('etkin', x.dataset.a === D.altSekme));
    }
  };
  let sorunOdak = false;

  // =====================================================================
  // Birden çok kabuk
  // =====================================================================
  D.kabuklar = [];
  function kabukSecici() {
    if (!D.kabuklar.length) return '';
    return `<div class="kabuk-secici">${D.kabuklar.map((k, i) => `<span class="${k === D.kabuk ? 'secili' : ''}" data-e="kabukSec" data-a="${i}">${S('terminal')}${i + 1}${k.bitti ? ' (kapandı)' : ''}<span class="simge kapat" data-e="kabukKapat" data-a="${i}" title="Kabuğu kapat">close</span></span>`).join('')}<span class="simge" data-e="kabukYeni" title="Yeni kabuk">add</span></div>`;
  }
  kabukAc = async function () {
    if (D.kabukAciliyor || !D.proje) return;
    if (D.kabuk && !D.kabuk.bitti) return;
    if (!(await guvenSor('Kabuk açmak'))) return;
    D.kabukAciliyor = true;
    const r = await api('/api/kabuk', { kok: D.proje.yol }).catch(x => ({ hata: x.message }));
    D.kabukAciliyor = false;
    if (r.hata) { D.kabukSatir.push({ t: r.hata, c: 'err' }); cizAltPanel(); return; }
    const k = { kimlik: r.kimlik, konum: 0, satirlar: [{ t: `Kabuk açıldı: ${D.proje.yol}`, c: 'dim' }], bitti: false };
    D.kabuklar.push(k);
    D.kabuk = k; D.kabukSatir = k.satirlar;
    cizAltPanel();
    while (D.kabuklar.includes(k)) {
      const c = await api('/api/cikti?' + sorgu({ kimlik: k.kimlik, konum: k.konum })).catch(() => null);
      if (!c || c.hata) break;
      k.konum = c.konum;
      for (const p of c.parcalar) k.satirlar.push({ t: p.t, c: p.tur === 'hata' ? 'err' : '' });
      if (k.satirlar.length > 2000) k.satirlar.splice(0, k.satirlar.length - 2000);
      if (c.bitti) { k.satirlar.push({ t: '— Kabuk kapandı', c: 'dim' }); k.bitti = true; }
      if ((c.parcalar.length || c.bitti) && D.kabuk === k && D.altSekme === 'kabuk' && D.ekran === 'duzenleyici') cizAltPanel();
      if (c.bitti) break;
      await bekle(c.parcalar.length ? 30 : 120);
    }
  };
  kabukKomutu = async function (komut) {
    if (!D.kabuk || D.kabuk.bitti) { D.kabuk = null; await kabukAc(); if (!D.kabuk) return; }
    const k = D.kabuk;
    if (komut.trim()) { D.kabukGecmis.push(komut); if (D.kabukGecmis.length > 100) D.kabukGecmis.shift(); }
    D.kabukSira = null;
    if (/^\s*(clear|cls|temizle)\s*$/.test(komut)) { k.satirlar.length = 0; cizAltPanel(); return; }
    k.satirlar.push({ t: '$ ' + komut, c: 'girdi-yanki' });
    cizAltPanel();
    $('#kabukGirdi')?.focus();
    const r = await api('/api/girdi', { kimlik: k.kimlik, metin: komut + '\n' }).catch(x => ({ hata: x.message }));
    if (r.hata) bildir(r.hata, true);
  };

  // =====================================================================
  // İki dosyayı karşılaştırma
  // =====================================================================
  /** Satır farkı (en uzun ortak alt dizi); git'in birleşik fark biçiminde. */
  function birlesikFark(a, b, adA, adB) {
    const A = a.split('\n'), B = b.split('\n');
    if (A.length * B.length > 9e6) return `--- ${adA}\n+++ ${adB}\n@@ Dosyalar karşılaştırma için çok büyük @@`;
    const n = A.length, m = B.length, L = new Uint32Array((n + 1) * (m + 1));
    for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--)
      L[i * (m + 1) + j] = A[i] === B[j] ? L[(i + 1) * (m + 1) + j + 1] + 1 : Math.max(L[(i + 1) * (m + 1) + j], L[i * (m + 1) + j + 1]);
    const ops = [];
    let i = 0, j = 0;
    while (i < n || j < m) {
      if (i < n && j < m && A[i] === B[j]) { ops.push([' ', A[i], i, j]); i++; j++; }
      else if (j < m && (i >= n || L[i * (m + 1) + j + 1] >= L[(i + 1) * (m + 1) + j])) { ops.push(['+', B[j], i, j]); j++; }
      else { ops.push(['-', A[i], i, j]); i++; }
    }
    let cikti = `--- ${adA}\n+++ ${adB}\n`;
    const degisen = ops.map((o, k) => o[0] !== ' ' ? k : -1).filter(k => k >= 0);
    if (!degisen.length) return cikti + '@@ Dosyalar aynı @@';
    let k = 0;
    while (k < degisen.length) {
      let bas = Math.max(0, degisen[k] - 3), son = degisen[k];
      while (k + 1 < degisen.length && degisen[k + 1] - son <= 7) son = degisen[++k];
      son = Math.min(ops.length - 1, son + 3);
      k++;
      const parca = ops.slice(bas, son + 1);
      const aBas = parca[0][2] + 1, bBas = parca[0][3] + 1;
      cikti += `@@ -${aBas},${parca.filter(o => o[0] !== '+').length} +${bBas},${parca.filter(o => o[0] !== '-').length} @@\n` + parca.map(o => o[0] + o[1]).join('\n') + '\n';
    }
    return cikti;
  }
  async function karsilastir(a, b) {
    const oku = async y => { const s = D.sekmeler.find(x => x.yol === y); if (s && !s.ikili) return s.icerik; const r = await api('/api/dosya?' + sorgu({ yol: tamYol(y) })); if (r.hata || r.ikili) throw new Error(r.hata || 'Metin dosyası değil.'); return r.icerik; };
    try {
      const [x, y] = await Promise.all([oku(a), oku(b)]);
      D.modal = { tur: 'fark', yol: `${a} ↔ ${b}`, isleme: true, metin: birlesikFark(x, y, a, b) };
      katmanlariCiz();
    } catch (e) { bildir(e.message, true); }
  }
  document.addEventListener('contextmenu', e => {
    const oge = e.target.closest('.agac-oge:not(.klasor)');
    if (!oge) return;
    // gezgin.js'in menüsü açıldıktan sonra karşılaştırma seçenekleri eklenir.
    setTimeout(() => {
      const m = $('.sag-menu');
      if (!m) return;
      const yol = oge.dataset.a;
      const ekle = (etiket, f) => { sagMenuIslevleri.push(f); m.insertAdjacentHTML('beforeend', `<div class="acilir-oge" data-sag="${sagMenuIslevleri.length - 1}"><span>${kac(etiket)}</span></div>`); };
      m.insertAdjacentHTML('beforeend', '<div class="acilir-ayrac"></div>');
      ekle('Karşılaştırma için seç', () => { D.karsilastirilan = yol; bildir(`“${sonParca(yol)}” seçildi; şimdi öteki dosyada “Seçilenle karşılaştır”.`); });
      if (D.karsilastirilan && D.karsilastirilan !== yol) ekle(`“${sonParca(D.karsilastirilan)}” ile karşılaştır`, () => karsilastir(D.karsilastirilan, yol));
      const r = m.getBoundingClientRect();
      if (r.bottom > innerHeight - 6) m.style.top = Math.max(6, innerHeight - r.height - 6) + 'px';
    }, 0);
  });

  // =====================================================================
  // Sınama kapsamı
  // =====================================================================
  D.kapsam = null;
  D.kapsamGoster = false;
  async function kapsamiOlc() {
    if (!D.proje || D.sinamaMesgul) return;
    if (!(await guvenSor('Sınamaları çalıştırmak'))) return;
    if (!(await tumunuKaydet())) return;
    D.sinamaMesgul = { dosya: '', ad: '' };
    cizYanPanel();
    bildir('Sınamalar kapsam ölçümüyle çalışıyor…');
    const r = await api('/api/sina', { kok: D.proje.yol, kapsam: true }).catch(e => ({ hata: e.message }));
    D.sinamaMesgul = null;
    if (r.hata) { bildir(r.hata, true); cizYanPanel(); return; }
    for (const d of r.dosyalar) {
      delete D.sinamaSonuc[d.dosya + '|'];
      if (d.hata) D.sinamaSonuc[d.dosya + '|'] = d.hata;
      for (const t of d.sinamalar) D.sinamaSonuc[d.dosya + '|' + t.ad] = t;
    }
    D.kapsam = {};
    for (const k of r.kapsam || []) D.kapsam[k.dosya] = k;
    D.kapsamGoster = true;
    const top = (r.kapsam || []).reduce((a, k) => [a[0] + k.kapsanan, a[1] + k.toplam], [0, 0]);
    bildir(`Kapsam: %${top[1] ? Math.floor(top[0] * 100 / top[1]) : 0} (${top[0]}/${top[1]} satır). Çalışmayan satırlar düzenleyicide kırmızı işaretli.`);
    cizYanPanel();
    isaretleriCiz();
  }
  function kapsamHtml() {
    if (!D.kapsam) return '';
    const l = Object.values(D.kapsam);
    if (!l.length) return '';
    return `<div class="git-bolum" style="margin-top:14px"><span class="esnek">Kapsam</span><span class="simge" title="${D.kapsamGoster ? 'Düzenleyicideki işaretleri gizle' : 'Düzenleyicide göster'}" data-e="kapsamGosterDegistir">${D.kapsamGoster ? 'visibility' : 'visibility_off'}</span></div>`
      + l.map(k => `<div class="kapsam-oge" data-e="dosyaAc" data-a="${kac(k.dosya)}" title="Çalışmayan satırlar: ${kac(k.calismayan.slice(0, 30).join(', '))}"><span class="esnek">${kac(k.dosya)}</span><span class="kapsam-cubuk"><span style="width:${k.yuzde}%" class="${k.yuzde >= 80 ? 'iyi' : k.yuzde >= 50 ? 'orta' : 'dusuk'}"></span></span><span class="mono">%${k.yuzde}</span></div>`).join('');
  }
  const ilkSinamaPaneli = sinamaPaneli;
  sinamaPaneli = function () {
    return `<div class="panel-dugme" data-e="kapsamiOlc" title="Sınamaları çalıştırır ve kodun hangi satırlarının denendiğini gösterir">${S('checklist')}Kapsamı ölç</div>` + ilkSinamaPaneli() + kapsamHtml();
  };

  Object.assign(EYLEM, {
    onizlemeAcKapa() { const s = etkinSekme(); if (!s) return; D.onizlenen.has(s.yol) ? D.onizlenen.delete(s.yol) : D.onizlenen.add(s.yol); D.bolme = null; guncelle('kod'); },
    onizlemeKapat() { const s = etkinSekme(); if (s) D.onizlenen.delete(s.yol); guncelle('kod'); },
    sorunTur(k) { D.sorunSuzgec.tur = k; cizAltPanel(); },
    sorunDosya() { D.sorunSuzgec.dosya = !D.sorunSuzgec.dosya; cizAltPanel(); },
    yerimiDegistir, yerimiSonraki() { yerimineGit(1); }, yerimiOnceki() { yerimineGit(-1); },
    yerimiSil(a, el, e) { e?.stopPropagation(); const i = a.lastIndexOf('|'), d = a.slice(0, i), n = +a.slice(i + 1); D.yerimleri[d] = (D.yerimleri[d] || []).filter(x => x !== n); ayarYaz('yerimleri', D.yerimleri); cizAltPanel(); isaretleriCiz(); },
    yapilacaklariYenile() { D.yapilacaklar = null; cizAltPanel(); },
    kabukSec(i) { const k = D.kabuklar[+i]; if (k) { D.kabuk = k; D.kabukSatir = k.satirlar; cizAltPanel(); } },
    kabukYeni() { D.kabuk = null; D.kabukSatir = []; kabukAc(); },
    async kabukKapat(i, el, e) {
      e?.stopPropagation();
      const k = D.kabuklar[+i];
      if (!k) return;
      if (!k.bitti) await api('/api/durdur', { kimlik: k.kimlik }).catch(() => {});
      D.kabuklar.splice(+i, 1);
      if (D.kabuk === k) { D.kabuk = D.kabuklar[D.kabuklar.length - 1] || null; D.kabukSatir = D.kabuk?.satirlar || []; }
      if (!D.kabuk) { D.kabukSatir = [{ t: 'Kabuk kapatıldı. Yeni kabuk için + düğmesine basın ya da komut yazın.', c: 'dim' }]; D.kabuk = { bitti: true, satirlar: D.kabukSatir }; }
      cizAltPanel();
    },
    kapsamiOlc,
    kapsamGosterDegistir() { D.kapsamGoster = !D.kapsamGoster; cizYanPanel(); isaretleriCiz(); },
  });
  Object.assign(GIRDI, {
    sorunMetin(v) { D.sorunSuzgec.metin = v; sorunOdak = true; cizAltPanel(); },
  });
  // Alt panelde YAPILACAKLAR sekmesine her geçişte yeniden taranır.
  const ilkAltSekme = EYLEM.altSekme;
  EYLEM.altSekme = function (a, ...k) { if (a === 'yapilacaklar') D.yapilacaklar = null; return ilkAltSekme(a, ...k); };

  document.addEventListener('keydown', e => {
    if (D.ekran !== 'duzenleyici' || D.modal) return;
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase();
    if (ctrl && e.altKey && k === 'k') { e.preventDefault(); yerimiDegistir(); }
    else if (ctrl && e.altKey && k === 'l') { e.preventDefault(); yerimineGit(1); }
    else if (ctrl && e.altKey && k === 'j') { e.preventDefault(); yerimineGit(-1); }
  });
