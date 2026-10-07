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
  ta.addEventListener('input', () => { hataSatiri = 0; ciz(); kaydetmeyiPlanla(); });
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

  // ---------------------------------------------------------------- Programlarım
  // Programlar bu cihazda, tarayıcıda saklanır: [{ ad, kod, tarih }]. Açık programın adı ayrıca
  // tutulur; yazılan her şey kısa bir gecikmeyle açık programa kaydedilir.
  let programlar = [];
  try { programlar = JSON.parse(ayarOku('programlar') || '[]'); } catch { programlar = []; }
  if (!Array.isArray(programlar)) programlar = [];
  let acikAd = ayarOku('acik') || null;
  const acikProgram = () => programlar.find((p) => p.ad === acikAd);
  function programlariYaz() {
    ayarYaz('programlar', JSON.stringify(programlar));
    ayarYaz('acik', acikAd || '');
    $('.baslik-yazi').textContent = acikAd || 'Tarayıcıda dene';
  }
  function bosAd(taban) {
    let ad = taban;
    for (let i = 2; programlar.some((p) => p.ad === ad); i++) ad = `${taban} (${i})`;
    return ad;
  }
  let kayitZamani = null;
  function programKaydet() {
    clearTimeout(kayitZamani);
    kayitZamani = null;
    let p = acikProgram();
    if (!p) {
      acikAd = bosAd('Programım');
      p = { ad: acikAd, kod: '' };
      programlar.unshift(p);
    }
    p.kod = ta.value;
    p.tarih = Date.now();
    programlariYaz();
  }
  function kaydetmeyiPlanla() {
    clearTimeout(kayitZamani);
    kayitZamani = setTimeout(programKaydet, 400);
  }
  addEventListener('pagehide', () => { if (kayitZamani) programKaydet(); });
  document.addEventListener('visibilitychange', () => { if (document.hidden && kayitZamani) programKaydet(); });
  function editoreYukle(kod) {
    durdur();
    cikti.textContent = '';
    bilgi.textContent = '';
    ta.value = kod;
    hataSatiri = 0;
    ciz();
    ta.setSelectionRange(0, 0);
  }
  function programAc(ad) {
    if (kayitZamani) programKaydet();
    const p = programlar.find((x) => x.ad === ad);
    if (!p) return;
    acikAd = ad;
    editoreYukle(p.kod);
    programlariYaz();
    $('#ornekler').value = '';
  }
  // Örnek, paylaşılan bağlantı ya da açılan dosya yeni bir program olarak açılır; kullanıcının
  // yazdıkları üzerine yazılmaz. Aynı adlı ve aynı içerikli program varsa ona geçilir.
  function yeniProgramOlarakAc(taban, kod) {
    if (kayitZamani) programKaydet();
    const ayni = programlar.find((p) => (p.ad === taban || p.ad.startsWith(taban + ' (')) && p.kod === kod);
    if (ayni) { programAc(ayni.ad); return; }
    acikAd = bosAd(taban);
    programlar.unshift({ ad: acikAd, kod, tarih: Date.now() });
    editoreYukle(kod);
    programlariYaz();
  }

  const pencere = $('#programlar');
  const tarihMetni = (t) => (t ? new Date(t).toLocaleString('tr-TR', { dateStyle: 'medium', timeStyle: 'short' }) : '');
  function listeyiCiz() {
    const l = $('#programListesi');
    l.replaceChildren();
    if (!programlar.length) {
      const li = document.createElement('li');
      li.innerHTML = '<span class="ad">Henüz kayıtlı program yok. Yazmaya başlayınca kaydedilir.</span>';
      l.appendChild(li);
      return;
    }
    for (const p of programlar) {
      const li = document.createElement('li');
      li.classList.toggle('acik', p.ad === acikAd);
      const ad = document.createElement('button');
      ad.className = 'ad';
      ad.textContent = p.ad;
      const k = document.createElement('small');
      k.textContent = tarihMetni(p.tarih);
      ad.appendChild(k);
      ad.addEventListener('click', () => { programAc(p.ad); pencere.close(); });
      const adlandir = document.createElement('button');
      adlandir.className = 'kucuk-dugme';
      adlandir.textContent = 'Adlandır';
      adlandir.addEventListener('click', () => {
        const yeni = (prompt('Programın yeni adı:', p.ad) || '').trim();
        if (!yeni || yeni === p.ad) return;
        if (programlar.some((x) => x.ad === yeni)) { alert(`'${yeni}' adlı bir program zaten var.`); return; }
        if (acikAd === p.ad) acikAd = yeni;
        p.ad = yeni;
        programlariYaz();
        listeyiCiz();
      });
      const sil = document.createElement('button');
      sil.className = 'kucuk-dugme';
      sil.textContent = 'Sil';
      sil.addEventListener('click', () => {
        if (!confirm(`'${p.ad}' silinsin mi? Bu geri alınamaz.`)) return;
        programlar = programlar.filter((x) => x !== p);
        if (acikAd === p.ad) {
          clearTimeout(kayitZamani);
          kayitZamani = null;
          acikAd = programlar.length ? programlar[0].ad : null;
          editoreYukle(programlar.length ? programlar[0].kod : '');
        }
        programlariYaz();
        listeyiCiz();
      });
      li.append(ad, adlandir, sil);
      l.appendChild(li);
    }
  }
  $('#programlarDugme').addEventListener('click', () => {
    if (kayitZamani) programKaydet();
    listeyiCiz();
    pencere.showModal();
  });
  $('#programlarKapat').addEventListener('click', () => pencere.close());
  pencere.addEventListener('click', (e) => { if (e.target === pencere) pencere.close(); });
  $('#yeniProgram').addEventListener('click', () => {
    if (kayitZamani) programKaydet();
    acikAd = bosAd('Programım');
    programlar.unshift({ ad: acikAd, kod: '', tarih: Date.now() });
    editoreYukle('');
    programlariYaz();
    $('#ornekler').value = '';
    pencere.close();
    ta.focus();
  });
  $('#programIndir').addEventListener('click', () => {
    const ad = (acikAd || 'program').replace(/[\\/:*?"<>|]+/g, '_');
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([ta.value], { type: 'text/plain;charset=utf-8' }));
    a.download = ad + '.ohc';
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  });
  $('#programYukle').addEventListener('change', async (e) => {
    const f = e.target.files[0];
    e.target.value = '';
    if (!f) return;
    yeniProgramOlarakAc(f.name.replace(/\.ohc$/i, '') || 'Programım', (await f.text()).replace(/\r\n/g, '\n'));
    pencere.close();
  });

  // ---------------------------------------------------------------- Kurulum (çevrimdışı)
  if ('serviceWorker' in navigator && /^https?:$/.test(location.protocol)) {
    navigator.serviceWorker.register('sw.js').catch(() => {});
  }
  let kurulumIstegi = null;
  addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    kurulumIstegi = e;
    $('#kur').hidden = false;
  });
  $('#kur').addEventListener('click', async () => {
    if (!kurulumIstegi) return;
    kurulumIstegi.prompt();
    await kurulumIstegi.userChoice.catch(() => {});
    kurulumIstegi = null;
    $('#kur').hidden = true;
  });
  const bagimsiz = matchMedia('(display-mode: standalone)').matches || navigator.standalone;
  if (!bagimsiz && /iPad|iPhone|iPod/.test(navigator.userAgent + (navigator.maxTouchPoints > 1 ? ' iPad' : ''))) {
    $('#kurulumBilgi').textContent += ' Ana ekrana uygulama olarak eklemek için: Paylaş → Ana Ekrana Ekle.';
  }

  // ---------------------------------------------------------------- Sembol çubuğu
  // Dokunmatik ekranlarda telefon klavyesinde zor bulunan işaretler ve Türkçe harfler.
  const semboller = $('#semboller');
  const dokunmatik = matchMedia('(pointer: coarse)');
  function cubuguKonumla() {
    const v = window.visualViewport;
    semboller.style.bottom = v ? Math.max(0, innerHeight - v.height - v.offsetTop) + 'px' : '0';
  }
  if (window.visualViewport) {
    visualViewport.addEventListener('resize', cubuguKonumla);
    visualViewport.addEventListener('scroll', cubuguKonumla);
  }
  let gizlemeZamani = null;
  ta.addEventListener('focus', () => {
    clearTimeout(gizlemeZamani);
    if (!dokunmatik.matches) return;
    semboller.hidden = false;
    document.body.classList.add('semboller-acik');
    cubuguKonumla();
  });
  ta.addEventListener('blur', () => {
    gizlemeZamani = setTimeout(() => {
      semboller.hidden = true;
      document.body.classList.remove('semboller-acik');
    }, 150);
  });
  // pointerdown'da odak düzenleyicide kalır, böylece klavye kapanmaz.
  semboller.addEventListener('pointerdown', (e) => { if (e.target.closest('button')) e.preventDefault(); });
  semboller.addEventListener('click', (e) => {
    const d = e.target.closest('button');
    if (!d) return;
    if (d.dataset.eylem === 'geri') { ta.focus(); document.execCommand('undo'); return; }
    if (d.dataset.eylem === 'calistir') { calistir(); return; }
    ta.focus();
    const secili = ta.value.slice(ta.selectionStart, ta.selectionEnd);
    if (d.dataset.kapa) {
      metinEkle(d.dataset.ekle + secili + d.dataset.kapa);
      ta.setSelectionRange(ta.selectionEnd - 1, ta.selectionEnd - 1);
    } else if (/^[)\]}]$/.test(d.dataset.ekle) && !secili && ta.value[ta.selectionStart] === d.dataset.ekle) {
      // Kendiliğinden eklenen kapanış işaretinin üzerinden geçer.
      ta.setSelectionRange(ta.selectionStart + 1, ta.selectionStart + 1);
    } else {
      metinEkle(d.dataset.ekle);
    }
  });

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
    yeniProgramOlarakAc(o.baslik, o.kod);
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

  const darEkran = matchMedia('(max-width: 860px)');
  async function calistir() {
    durdur();
    if (darEkran.matches) {
      // Telefonda klavye kapanır ve sonuç görünür.
      if (dokunmatik.matches) ta.blur();
      $('.sag').scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
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
    // Eski sürümün tek kayıtlı kodu ilk program olur.
    const eskiKod = ayarOku('kod');
    if (eskiKod && !programlar.length) {
      programlar.push({ ad: 'Programım', kod: eskiKod, tarih: Date.now() });
      acikAd = 'Programım';
      programlariYaz();
    }
    if (h.get('kod')) {
      let kod = '';
      try { kod = await ac(h.get('kod')); } catch { kod = ''; }
      yeniProgramOlarakAc('Paylaşılan program', kod);
      if (h.get('girdi')) { try { $('#girdi').value = await ac(h.get('girdi')); } catch { /* yok */ } }
    } else if (h.get('ornek') && ornekAc(h.get('ornek'))) {
      // örnek açıldı
    } else if (acikProgram()) {
      ta.value = acikProgram().kod;
      programlariYaz();
    } else {
      acikAd = null;
      ta.value = (ornekler.find((o) => o.kimlik === 'merhaba') || { kod: '"Merhaba, dünya!"\'yı yaz.\n' }).kod;
    }
    ciz();
    ta.setSelectionRange(0, 0);
    ta.focus({ preventScroll: true });
    if (h.get('kod') || h.get('ornek')) calistir();
    hazirla().catch(() => {});
  })();
})();
