// Tarayıcıda Orhunca: derleyici WebAssembly olarak sayfada çalışır (oyun.wasm). Konsol
// programları bir iş parçacığında, arayüz programları yalıtılmış bir çerçevede çalışır.
(function () {
  'use strict';
  const $ = (s) => document.querySelector(s);
  const ta = $('#kod'), vurguKat = $('#vurguKat'), numaralar = $('#numaralar'), hataKat = $('#hataKat');
  const cikti = $('#cikti'), bilgi = $('#bilgi');
  const kodla = new TextEncoder(), coz = new TextDecoder();
  const ayarOku = (a) => { try { return localStorage.getItem('orhunca-dene.' + a); } catch { return null; } };
  const ayarYaz = (a, v) => { try { localStorage.setItem('orhunca-dene.' + a, v); } catch { /* yok */ } };

  // ---------------------------------------------------------------- Düzenleyici
  let hataSatiri = 0;
  function ciz() {
    const satirlar = ta.value.split('\n');
    vurguKat.innerHTML = OrhuncaVurgu.vurgula(ta.value) + '\n';
    if (numaralar.childElementCount !== satirlar.length) {
      numaralar.innerHTML = satirlar.map((_, i) => `<div>${i + 1}</div>`).join('');
    }
    [...numaralar.children].forEach((d, i) => d.classList.toggle('hatali', i + 1 === hataSatiri));
    hataKat.innerHTML = hataSatiri ? `<div class="hata-satiri" style="top:${14 + (hataSatiri - 1) * 22}px"></div>` : '';
    ta.style.width = Math.max(vurguKat.scrollWidth, $('#duzenleyici').clientWidth - 52) + 'px';
    ta.style.height = satirlar.length * 22 + 30 + 'px';
  }
  function metinEkle(m) {
    if (!document.execCommand('insertText', false, m)) {
      ta.setRangeText(m, ta.selectionStart, ta.selectionEnd, 'end');
      ta.dispatchEvent(new Event('input'));
    }
  }
  ta.addEventListener('input', () => { hataSatiri = 0; ciz(); ayarYaz('kod', ta.value); });
  ta.addEventListener('keydown', (e) => {
    const bas = ta.selectionStart;
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); calistir(); return; }
    if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); metinEkle('    '); return; }
    if (e.key === 'Enter') {
      // Girintiyi korur; ':' ile biten satırdan sonra bir düzey içeri girer.
      e.preventDefault();
      const satirBasi = ta.value.lastIndexOf('\n', bas - 1) + 1;
      const satir = ta.value.slice(satirBasi, bas);
      let girinti = /^\s*/.exec(satir)[0];
      if (/:\s*$/.test(satir)) girinti += '    ';
      metinEkle('\n' + girinti);
      return;
    }
    if (e.key === 'Backspace' && bas === ta.selectionEnd) {
      const satirBasi = ta.value.lastIndexOf('\n', bas - 1) + 1;
      const once = ta.value.slice(satirBasi, bas);
      if (once.length && /^ +$/.test(once) && once.length % 4 === 0) {
        e.preventDefault();
        ta.setSelectionRange(bas - 4, bas);
        metinEkle('');
      }
    }
  });
  addEventListener('resize', ciz);

  // ---------------------------------------------------------------- Örnekler
  let ornekler = [];
  const secim = $('#ornekler');
  const ornekYukle = fetch('ornekler.json').then((r) => r.json()).then((l) => {
    ornekler = l;
    let grup = null;
    for (const o of l) {
      if (!grup || grup.label !== o.grup) {
        grup = document.createElement('optgroup');
        grup.label = o.grup;
        secim.appendChild(grup);
      }
      const s = document.createElement('option');
      s.value = o.kimlik;
      s.textContent = o.baslik;
      grup.appendChild(s);
    }
  }).catch(() => {});
  function ornekAc(kimlik) {
    const o = ornekler.find((x) => x.kimlik === kimlik);
    if (!o) return false;
    ta.value = o.kod;
    hataSatiri = 0;
    ciz();
    ayarYaz('kod', ta.value);
    secim.value = kimlik;
    return true;
  }
  secim.addEventListener('change', () => { if (secim.value) { ornekAc(secim.value); calistir(); } });

  // ---------------------------------------------------------------- Derleyici
  let derleyici = null, calismaZamani = null, sayfaParcalari = null;
  async function hazirla() {
    if (derleyici) return;
    const [d, rt, yukleyici, arayuz] = await Promise.all([
      WebAssembly.instantiateStreaming
        ? WebAssembly.instantiateStreaming(fetch('calisma/oyun.wasm'), {}).catch(async () => WebAssembly.instantiate(await (await fetch('calisma/oyun.wasm')).arrayBuffer(), {}))
        : WebAssembly.instantiate(await (await fetch('calisma/oyun.wasm')).arrayBuffer(), {}),
      fetch('calisma/orhunca_rt.wasm').then((r) => r.arrayBuffer()),
      fetch('calisma/orhunca.js').then((r) => r.text()),
      fetch('calisma/arayuz.html').then((r) => r.text()),
    ]);
    derleyici = d.instance.exports;
    calismaZamani = new Uint8Array(rt);
    sayfaParcalari = { yukleyici, arayuz };
  }
  function derle(kaynak) {
    const b = kodla.encode(kaynak);
    const p = derleyici.ayir(b.length);
    new Uint8Array(derleyici.memory.buffer, p, b.length).set(b);
    const r = derleyici.derle(p, b.length);
    derleyici.birak(p, b.length);
    const bellek = derleyici.memory.buffer;
    const n = new DataView(bellek).getUint32(r, true);
    const tur = new Uint8Array(bellek, r + 4, 1)[0];
    const veri = new Uint8Array(bellek, r + 5, n - 1).slice();
    derleyici.birak(r, n);
    return { tur, veri };
  }
  function base64(b) {
    let s = '';
    for (let i = 0; i < b.length; i += 8192) s += String.fromCharCode.apply(null, b.subarray(i, i + 8192));
    return btoa(s);
  }

  // ---------------------------------------------------------------- Çalıştırma
  let isci = null, baslangic = 0, bekleyen = '', bekleyenHata = '', cizimIstendi = false;
  const cozuculer = { 1: new TextDecoder(), 2: new TextDecoder() };
  function yaz(metin, sinif) {
    const s = document.createElement('span');
    if (sinif) s.className = sinif;
    s.textContent = metin;
    cikti.appendChild(s);
  }
  function akit() {
    cizimIstendi = false;
    if (bekleyen) { yaz(bekleyen); bekleyen = ''; }
    if (bekleyenHata) { yaz(bekleyenHata, 'hata'); bekleyenHata = ''; }
    const p = $('#panelCikti');
    p.scrollTop = p.scrollHeight;
  }
  function sekme(ad) {
    $('#sekmeCikti').setAttribute('aria-selected', String(ad === 'cikti'));
    $('#sekmeUygulama').setAttribute('aria-selected', String(ad === 'uygulama'));
    $('#panelCikti').hidden = ad !== 'cikti';
    $('#panelUygulama').hidden = ad !== 'uygulama';
  }
  $('#sekmeCikti').addEventListener('click', () => sekme('cikti'));
  $('#sekmeUygulama').addEventListener('click', () => sekme('uygulama'));

  function durdur(mesaj) {
    if (isci) {
      isci.terminate();
      isci = null;
      akit();
      if (mesaj) yaz(mesaj, 'bilgi');
    }
    $('#durdur').hidden = true;
    $('#calistir').disabled = false;
  }
  $('#durdur').addEventListener('click', () => durdur('\n— Durduruldu.\n'));
  $('#calistir').addEventListener('click', () => calistir());

  async function calistir() {
    durdur();
    cikti.textContent = '';
    hataSatiri = 0;
    ciz();
    bilgi.textContent = 'derleniyor…';
    try {
      await hazirla();
    } catch (h) {
      yaz('Derleyici yüklenemedi: ' + h + '\n', 'hata');
      bilgi.textContent = '';
      return;
    }
    const t0 = performance.now();
    const { tur, veri } = derle(ta.value);
    const ms = Math.max(1, Math.round(performance.now() - t0));
    if (tur === 1) {
      const h = JSON.parse(coz.decode(veri));
      hataSatiri = h.satir || 0;
      ciz();
      sekme('cikti');
      $('#sekmeUygulama').hidden = true;
      yaz(h.mesaj.replace(/program\.ohc/g, 'kod'), 'hata');
      bilgi.textContent = 'derleme hatası';
      if (hataSatiri) {
        const d = $('#duzenleyici');
        d.scrollTop = Math.max(0, (hataSatiri - 1) * 22 - d.clientHeight / 3);
      }
      return;
    }
    if (tur === 2) {
      // Arayüz programı: tek dosyalık sayfa yalıtılmış çerçevede açılır.
      const sayfa = sayfaParcalari.arayuz
        .replace('__BASLIK__', () => 'Orhunca')
        .replace('__YUKLEYICI__', () => sayfaParcalari.yukleyici)
        .replace('__CALISMA_ZAMANI__', () => base64(calismaZamani))
        .replace('__PROGRAM__', () => base64(veri));
      const f = document.createElement('iframe');
      f.className = 'uygulama';
      f.title = 'Orhunca uygulaması';
      f.setAttribute('sandbox', 'allow-scripts allow-modals allow-forms');
      f.srcdoc = sayfa;
      $('#panelUygulama').replaceChildren(f);
      $('#sekmeUygulama').hidden = false;
      $('.girdi-kutu').hidden = true;
      sekme('uygulama');
      yaz(`✓ Arayüz programı derlendi (${ms} ms); "Uygulama" sekmesinde çalışıyor.\n`, 'tamam');
      bilgi.textContent = `derleme ${ms} ms`;
      return;
    }
    $('#sekmeUygulama').hidden = true;
    $('.girdi-kutu').hidden = false;
    sekme('cikti');
    bilgi.textContent = `derleme ${ms} ms · çalışıyor…`;
    $('#durdur').hidden = false;
    baslangic = performance.now();
    isci = new Worker('isci.js');
    isci.onmessage = (e) => {
      const m = e.data;
      if (m.tur === 'cikti') {
        const t = cozuculer[m.akis].decode(m.b, { stream: true });
        if (m.akis === 2) bekleyenHata += t; else bekleyen += t;
        if (!cizimIstendi) { cizimIstendi = true; requestAnimationFrame(akit); }
      } else if (m.tur === 'bitti') {
        akit();
        const sure = Math.round(performance.now() - baslangic);
        bilgi.textContent = `derleme ${ms} ms · çalışma ${sure} ms · çıkış kodu ${m.kod}`;
        if (!cikti.textContent) yaz('(Program bir şey yazmadı.)\n', 'bilgi');
        isci.terminate();
        isci = null;
        $('#durdur').hidden = true;
      }
    };
    isci.onerror = (e) => { yaz('Çalışma hatası: ' + e.message + '\n', 'hata'); durdur(); };
    isci.postMessage({ calismaZamani, program: veri, girdi: $('#girdi').value });
  }

  // ---------------------------------------------------------------- Paylaşma
  async function sikistir(metin) {
    if (typeof CompressionStream === 'undefined') return 'd' + base64(kodla.encode(metin));
    const akis = new Blob([metin]).stream().pipeThrough(new CompressionStream('deflate-raw'));
    return 'z' + base64(new Uint8Array(await new Response(akis).arrayBuffer()));
  }
  async function ac(k) {
    const b64 = k.slice(1).replace(/-/g, '+').replace(/_/g, '/');
    const ham = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    if (k[0] === 'd') return coz.decode(ham);
    const akis = new Blob([ham]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
    return await new Response(akis).text();
  }
  function bildir(m) {
    const d = document.createElement('div');
    d.className = 'bildirim';
    d.textContent = m;
    document.body.appendChild(d);
    setTimeout(() => d.remove(), 2200);
  }
  $('#paylas').addEventListener('click', async () => {
    const k = (await sikistir(ta.value)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
    const adres = location.href.split('#')[0] + '#kod=' + k;
    history.replaceState(null, '', adres);
    try { await navigator.clipboard.writeText(adres); bildir('Bağlantı kopyalandı.'); } catch { bildir('Bağlantı adres çubuğunda.'); }
  });

  // ---------------------------------------------------------------- Başlangıç
  (async () => {
    const h = new URLSearchParams(location.hash.slice(1));
    await ornekYukle;
    if (h.get('kod')) {
      try { ta.value = await ac(h.get('kod')); } catch { ta.value = ''; }
      if (h.get('girdi')) { try { $('#girdi').value = await ac(h.get('girdi')); } catch { /* yok */ } }
    } else if (h.get('ornek') && ornekAc(h.get('ornek'))) {
      // örnek açıldı
    } else {
      ta.value = ayarOku('kod') || (ornekler.find((o) => o.kimlik === 'merhaba') || { kod: '"Merhaba, dünya!"\'yı yaz.\n' }).kod;
    }
    ciz();
    ta.setSelectionRange(0, 0);
    ta.focus({ preventScroll: true });
    if (h.get('kod') || h.get('ornek')) calistir();
    hazirla().catch(() => {});
  })();
})();
