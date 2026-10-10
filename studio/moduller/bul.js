/* Orhunca Stüdyo — Dosya içinde bul (Ctrl+F) ve değiştir (Ctrl+H).
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  D.bul = { acik: false, degistir: false, metin: '', yeni: '', buyukKucuk: false, tamKelime: false, duzenli: false, sira: -1, sonuclar: [] };

  /** Aranan ifadenin düzenli ifadesi; geçersizse null. */
  function bulIfadesi() {
    const b = D.bul;
    if (!b.metin) return null;
    let kaynak = b.duzenli ? b.metin : b.metin.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    if (b.tamKelime) kaynak = `(?<![\\p{L}\\p{N}_])(?:${kaynak})(?![\\p{L}\\p{N}_])`;
    try { return new RegExp(kaynak, 'gu' + (b.buyukKucuk ? '' : 'i')); } catch { return null; }
  }

  function sonuclariBul() {
    const s = etkinSekme(), b = D.bul, ifade = bulIfadesi();
    b.sonuclar = [];
    if (!s || s.ikili || !ifade) return;
    for (const m of s.icerik.matchAll(ifade)) {
      if (!m[0].length) { if (b.sonuclar.length > 5000) break; continue; }
      b.sonuclar.push([m.index, m.index + m[0].length]);
      if (b.sonuclar.length >= 5000) break;
    }
  }

  /** İmlecin ardından gelen ilk sonuç. */
  function yakinSonuc() {
    const ta = $('#kodAlani'), b = D.bul;
    const k = ta ? ta.selectionStart : 0;
    const i = b.sonuclar.findIndex(r => r[0] >= k);
    return i < 0 ? (b.sonuclar.length ? 0 : -1) : i;
  }

  function bulKatmaniniCiz() {
    const ic = $('#kodIc'), s = etkinSekme(), b = D.bul;
    if (!ic || !s) return;
    let k = $('#bulKatmani');
    if (!k) { k = document.createElement('div'); k.id = 'bulKatmani'; ic.insertBefore(k, $('#vurguKatman')); }
    if (!b.acik || !b.sonuclar.length) { k.innerHTML = ''; return; }
    const satirBaslari = [0];
    for (let i = 0; i < s.icerik.length; i++) if (s.icerik.charCodeAt(i) === 10) satirBaslari.push(i + 1);
    const satirBul = p => { let a = 0, z = satirBaslari.length - 1; while (a < z) { const o = (a + z + 1) >> 1; if (satirBaslari[o] <= p) a = o; else z = o - 1; } return a; };
    k.innerHTML = b.sonuclar.slice(0, 2000).map(([bas, son], i) => {
      const n = satirBul(bas), sut = bas - satirBaslari[n];
      const uz = Math.max(1, Math.min(son, satirBaslari[n + 1] ?? son) - bas);
      return `<div class="bul-isaret ${i === b.sira ? 'etkin' : ''}" style="top:${4 + n * 21}px;left:${56 + sut * karakterGenisligi}px;width:${uz * karakterGenisligi}px"></div>`;
    }).join('');
  }

  function bulCubugunuCiz() {
    const bolge = $('#kodBolge'), b = D.bul;
    $('#bulCubugu')?.remove();
    if (!bolge || !b.acik || !etkinSekme() || etkinSekme().ikili) { bulKatmaniniCiz(); return; }
    const c = document.createElement('div');
    c.id = 'bulCubugu';
    c.className = 'bul-cubugu';
    const secenek = (ad, simge, baslik) => `<span class="simge bul-secenek ${b[ad] ? 'acik' : ''}" data-e="bulSecenek" data-a="${ad}" title="${baslik}">${simge}</span>`;
    c.innerHTML = `<span class="simge bul-ac-kapa" data-e="bulDegistirAcKapa" title="Değiştir (Ctrl+H)">${b.degistir ? 'expand_more' : 'chevron_right'}</span>
      <div class="bul-satirlar">
        <div class="bul-satir"><input id="bulMetin" class="metin-girdi" data-g="bulMetin" placeholder="Bul" value="${kac(b.metin)}" spellcheck="false" autocomplete="off">
          ${secenek('buyukKucuk', 'match_case', 'Büyük/küçük harf duyarlı')}${secenek('tamKelime', 'match_word', 'Yalnızca tam kelime')}${secenek('duzenli', 'code', 'Düzenli ifade')}
          <span class="bul-sayac" id="bulSayac"></span>
          <span class="simge" data-e="bulOnceki" title="Önceki (Shift+Enter)">arrow_upward</span><span class="simge" data-e="bulSonraki" title="Sonraki (Enter)">arrow_downward</span><span class="simge" data-e="bulKapat" title="Kapat (Esc)">close</span></div>
        ${b.degistir ? `<div class="bul-satir"><input id="bulYeni" class="metin-girdi" data-g="bulYeni" placeholder="Değiştir" value="${kac(b.yeni)}" spellcheck="false" autocomplete="off">
          <span class="simge" data-e="bulDegistirBir" title="Değiştir (Enter)">find_replace</span><span class="kucuk-dugme" data-e="bulHepsiniDegistir" title="Hepsini değiştir (Ctrl+Alt+Enter)">Hepsi</span></div>` : ''}
      </div>`;
    bolge.appendChild(c);
    sayaciGuncelle();
    bulKatmaniniCiz();
  }

  function sayaciGuncelle() {
    const el = $('#bulSayac'), b = D.bul;
    if (!el) return;
    el.textContent = !b.metin ? '' : !bulIfadesi() ? 'Geçersiz' : b.sonuclar.length ? `${b.sira + 1 || '?'} / ${b.sonuclar.length}${b.sonuclar.length >= 5000 ? '+' : ''}` : 'Sonuç yok';
    el.classList.toggle('yok', !!b.metin && !b.sonuclar.length);
  }

  /** Sonucu seçer ve görünür yapar; odak bul kutusunda kalır. */
  function sonucaGit(i) {
    const b = D.bul, ta = $('#kodAlani');
    if (!ta || !b.sonuclar.length) { sayaciGuncelle(); return; }
    b.sira = (i + b.sonuclar.length) % b.sonuclar.length;
    const [bas, son] = b.sonuclar[b.sira];
    ta.setSelectionRange(bas, son);
    imleciGuncelle();
    imleciGoster();
    sayaciGuncelle();
    bulKatmaniniCiz();
  }

  function aramayiYenile(git = true) {
    sonuclariBul();
    D.bul.sira = -1;
    if (git && D.bul.sonuclar.length) sonucaGit(yakinSonuc());
    else { sayaciGuncelle(); bulKatmaniniCiz(); }
  }

  function bulAc(degistir) {
    const ta = $('#kodAlani'), b = D.bul;
    if (!ta) return;
    const secili = ta.value.slice(ta.selectionStart, ta.selectionEnd);
    if (secili && !secili.includes('\n')) b.metin = secili;
    b.acik = true;
    b.degistir = degistir;
    bulCubugunuCiz();
    aramayiYenile();
    const g = $(degistir && b.metin ? '#bulYeni' : '#bulMetin');
    g?.focus(); g?.select();
  }

  function bulKapat() {
    D.bul.acik = false;
    bulCubugunuCiz();
    $('#kodAlani')?.focus();
  }

  /** Düzenleyicideki metni geri alınabilir biçimde değiştirir. */
  function araligiDegistir(ta, bas, son, metin) {
    ta.focus();
    ta.setSelectionRange(bas, son);
    metinEkle(ta, metin);
  }

  function yeniMetin(eslesen) {
    const b = D.bul;
    return b.duzenli ? eslesen.replace(new RegExp(bulIfadesi().source, bulIfadesi().flags.replace('g', '')), b.yeni) : b.yeni;
  }

  function birDegistir() {
    const ta = $('#kodAlani'), b = D.bul;
    if (!ta || !b.sonuclar.length) return;
    if (b.sira < 0) { sonucaGit(yakinSonuc()); return; }
    const [bas, son] = b.sonuclar[b.sira];
    const yeni = yeniMetin(ta.value.slice(bas, son));
    araligiDegistir(ta, bas, son, yeni);
    const sonra = bas + yeni.length;
    sonuclariBul();
    const i = b.sonuclar.findIndex(r => r[0] >= sonra);
    b.sira = -1;
    if (b.sonuclar.length) sonucaGit(i < 0 ? 0 : i); else { sayaciGuncelle(); bulKatmaniniCiz(); }
    $('#bulYeni')?.focus();
  }

  function hepsiniDegistir() {
    const ta = $('#kodAlani'), b = D.bul, ifade = bulIfadesi();
    if (!ta || !ifade || !b.sonuclar.length) return;
    const n = b.sonuclar.length;
    const yeni = b.duzenli ? ta.value.replace(ifade, b.yeni) : ta.value.replace(ifade, () => b.yeni);
    araligiDegistir(ta, 0, ta.value.length, yeni);
    aramayiYenile(false);
    bildir(`${n} yer değiştirildi.`);
    $('#bulYeni')?.focus();
  }

  // Düzenleyici yeniden çizilince (sekme değişimi) çubuk ve işaretler korunur.
  const ilkCizKod = cizKod;
  cizKod = function () {
    ilkCizKod();
    if (D.bul.acik) { sonuclariBul(); D.bul.sira = -1; bulCubugunuCiz(); }
  };
  document.addEventListener('input', e => {
    if (e.target.id === 'kodAlani' && D.bul.acik) { sonuclariBul(); D.bul.sira = Math.min(D.bul.sira, D.bul.sonuclar.length - 1); sayaciGuncelle(); bulKatmaniniCiz(); }
  });

  Object.assign(EYLEM, {
    bulAc() { bulAc(false); },
    degistirAc() { bulAc(true); },
    bulKapat,
    bulSonraki() { sonucaGit(D.bul.sira < 0 ? yakinSonuc() : D.bul.sira + 1); },
    bulOnceki() { sonucaGit(D.bul.sira < 0 ? yakinSonuc() - 1 : D.bul.sira - 1); },
    bulSecenek(ad) { D.bul[ad] = !D.bul[ad]; bulCubugunuCiz(); aramayiYenile(); $('#bulMetin')?.focus(); },
    bulDegistirAcKapa() { D.bul.degistir = !D.bul.degistir; bulCubugunuCiz(); $(D.bul.degistir ? '#bulYeni' : '#bulMetin')?.focus(); },
    bulDegistirBir: birDegistir,
    bulHepsiniDegistir: hepsiniDegistir,
  });
  Object.assign(GIRDI, {
    bulMetin(v) { D.bul.metin = v; aramayiYenile(); $('#bulMetin')?.focus(); },
    bulYeni(v) { D.bul.yeni = v; },
  });

  document.addEventListener('keydown', e => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (D.ekran !== 'duzenleyici' || D.modal) return;
    if (ctrl && !e.shiftKey && !e.altKey && (e.key === 'f' || e.key === 'F')) { e.preventDefault(); e.stopImmediatePropagation(); bulAc(false); return; }
    if (ctrl && !e.shiftKey && (e.key === 'h' || e.key === 'H')) { e.preventDefault(); e.stopImmediatePropagation(); bulAc(true); return; }
    if (e.key === 'F3' && D.bul.metin) { e.preventDefault(); if (!D.bul.acik) { D.bul.acik = true; bulCubugunuCiz(); sonuclariBul(); } e.shiftKey ? EYLEM.bulOnceki() : EYLEM.bulSonraki(); return; }
    const id = e.target.id;
    if (id === 'kodAlani' && e.key === 'Escape' && D.bul.acik && !$('#tamamla')?.offsetParent) { bulKapat(); return; }
    if (id !== 'bulMetin' && id !== 'bulYeni') return;
    if (e.key === 'Escape') { e.preventDefault(); e.stopImmediatePropagation(); bulKapat(); return; }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (id === 'bulYeni') { ctrl && e.altKey ? hepsiniDegistir() : birDegistir(); return; }
      e.shiftKey ? EYLEM.bulOnceki() : EYLEM.bulSonraki();
    }
  }, true);
