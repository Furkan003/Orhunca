/* Orhunca Stüdyo — Gezgin işlemleri: sağ tık menüsü, silme, adlandırma, taşıma, kopyalama,
 * yeni öğe şablonları ve Stüdyo dışında değişen dosyaları fark etme.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Sağ tık menüsü (gezgin, sekmeler ve başka yerler için ortak)
  // =====================================================================
  let sagMenuIslevleri = [];
  /** ogeler: [etiket, işlev, kısayol?] ya da '-' (ayraç) */
  function sagMenuAc(e, ogeler) {
    e.preventDefault();
    sagMenuKapat();
    sagMenuIslevleri = [];
    const m = document.createElement('div');
    m.className = 'acilir-menu sag-menu';
    m.innerHTML = ogeler.filter(Boolean).map(o => {
      if (o === '-') return '<div class="acilir-ayrac"></div>';
      sagMenuIslevleri.push(o[1]);
      return `<div class="acilir-oge" data-sag="${sagMenuIslevleri.length - 1}"><span>${kac(o[0])}</span>${o[2] ? `<span class="kisayol">${kac(o[2])}</span>` : ''}</div>`;
    }).join('');
    document.body.appendChild(m);
    const r = m.getBoundingClientRect();
    m.style.left = Math.min(e.clientX, innerWidth - r.width - 6) + 'px';
    m.style.top = Math.min(e.clientY, innerHeight - r.height - 6) + 'px';
  }
  function sagMenuKapat() { document.querySelectorAll('.sag-menu').forEach(m => m.remove()); }
  document.addEventListener('mousedown', e => {
    const o = e.target.closest('.sag-menu [data-sag]');
    if (o) { e.preventDefault(); const f = sagMenuIslevleri[+o.dataset.sag]; sagMenuKapat(); f?.(); return; }
    if (!e.target.closest('.sag-menu')) sagMenuKapat();
  }, true);
  document.addEventListener('keydown', e => { if (e.key === 'Escape') sagMenuKapat(); }, true);
  window.addEventListener('blur', sagMenuKapat);

  // =====================================================================
  // Dosya işlemleri
  // =====================================================================
  const ustKlasor = yol => yol.includes('/') ? yol.slice(0, yol.lastIndexOf('/')) : '';
  const birlestir = (k, ad) => k ? k + '/' + ad : ad;
  const kopyalanacakYer = (k, yol) => birlestir(k, sonParca(yol));

  /** Açık sekmeler ve açık/kapalı klasör bilgisi yeni yola taşınır. */
  function yollariTasi(eski, yeni) {
    const cevir = y => y === eski ? yeni : y.startsWith(eski + '/') ? yeni + y.slice(eski.length) : y;
    D.sekmeler.forEach(s => { s.yol = cevir(s.yol); });
    if (D.etkin) D.etkin = cevir(D.etkin);
    D.kapaliKlasorler = new Set([...D.kapaliKlasorler].map(cevir));
  }

  /** Aynı adlı dosya varsa "ad kopya.ohc", "ad kopya 2.ohc" … */
  function bosAd(k, ad) {
    const var_ = y => D.agac.some(g => g.yol === y);
    if (!var_(birlestir(k, ad))) return birlestir(k, ad);
    const n = ad.lastIndexOf('.'), kok = n > 0 ? ad.slice(0, n) : ad, uzanti = n > 0 ? ad.slice(n) : '';
    for (let i = 1; ; i++) {
      const y = birlestir(k, `${kok} kopya${i > 1 ? ' ' + i : ''}${uzanti}`);
      if (!var_(y)) return y;
    }
  }

  async function agaciTazele() { await agaciYukle(); guncelle('yan', 'sekmeler', 'kod', 'durum'); }

  async function ogeSil(yol) {
    const g = D.agac.find(x => x.yol === yol);
    const ne = g?.klasor ? `“${sonParca(yol)}” klasörü ve içindeki her şey` : `“${sonParca(yol)}” dosyası`;
    if (!(await onayla(`${ne} silinsin mi? Dosyaların son hâli Dosya geçmişinden geri alınabilir.`, { baslik: 'Sil', dugme: 'Sil', tehlikeli: true }))) return;
    const r = await api('/api/dosya/sil', { yol: tamYol(yol) }).catch(x => ({ hata: x.message }));
    if (r.hata) return bildir(r.hata, true);
    D.sekmeler = D.sekmeler.filter(s => s.yol !== yol && !s.yol.startsWith(yol + '/'));
    if (D.etkin && !D.sekmeler.some(s => s.yol === D.etkin)) D.etkin = D.sekmeler[0]?.yol || null;
    await agaciTazele();
  }

  async function ogeTasi(yol, yeni, kopya = false) {
    if (!yeni || yeni === yol) return false;
    const r = await api(kopya ? '/api/dosya/kopyala' : '/api/dosya/tasi', { yol: tamYol(yol), yeni: tamYol(yeni) }).catch(x => ({ hata: x.message }));
    if (r.hata) { bildir(r.hata, true); return false; }
    if (!kopya) yollariTasi(yol, yeni);
    await agaciTazele();
    return true;
  }

  async function ogeAdlandir(yol) {
    const ad = await metinSor('Yeni ad', sonParca(yol), { baslik: 'Yeniden adlandır', dugme: 'Adlandır' });
    const temiz = ad?.trim();
    if (!temiz || temiz === sonParca(yol)) return;
    if (/[\\/]/.test(temiz) || temiz === '..' || temiz === '.') return bildir('Ad, / ya da \\ içeremez.', true);
    await ogeTasi(yol, birlestir(ustKlasor(yol), temiz));
  }

  async function yapistir(hedefKlasor) {
    const p = D.pano;
    if (!p) return;
    if (p.kes) {
      if (await ogeTasi(p.yol, kopyalanacakYer(hedefKlasor, p.yol))) D.pano = null;
    } else {
      await ogeTasi(p.yol, bosAd(hedefKlasor, sonParca(p.yol)), true);
    }
  }

  function panoyaYaz(metin) {
    navigator.clipboard?.writeText(metin).then(() => bildir('Panoya kopyalandı.'), () => bildir(metin));
  }

  function gezginMenusu(e, yol) {
    const g = yol ? D.agac.find(x => x.yol === yol) : null;
    const klasor = g ? (g.klasor ? yol : ustKlasor(yol)) : '';
    sagMenuAc(e, [
      g && !g.klasor && ['Aç', () => dosyaAc(yol)],
      ['Yeni öğe…', () => yeniOgeModal(klasor), 'Ctrl+Shift+A'],
      ['Yeni dosya…', () => EYLEM.yeniDosyaModal(klasor)],
      ['Yeni klasör…', () => EYLEM.yeniKlasorModal(klasor)],
      '-',
      g && ['Kes', () => { D.pano = { yol, kes: true }; }, 'Ctrl+X'],
      g && ['Kopyala', () => { D.pano = { yol, kes: false }; }, 'Ctrl+C'],
      D.pano && ['Yapıştır', () => yapistir(klasor), 'Ctrl+V'],
      g && ['Çoğalt', () => ogeTasi(yol, bosAd(ustKlasor(yol), sonParca(yol)), true)],
      g && '-',
      g && ['Yeniden adlandır…', () => ogeAdlandir(yol), 'F2'],
      g && ['Sil', () => ogeSil(yol), 'Delete'],
      '-',
      ['Yolu kopyala', () => panoyaYaz(yol ? tamYol(yol) : D.proje.yol)],
      g && ['Göreli yolu kopyala', () => panoyaYaz(yol)],
      ['Dosya gezgininde göster', async () => { const r = await api('/api/dosya/goster', { yol: yol ? tamYol(yol) : D.proje.yol }).catch(x => ({ hata: x.message })); if (r.hata) bildir(r.hata, true); }],
    ]);
  }

  document.addEventListener('contextmenu', e => {
    const oge = e.target.closest('.agac-oge');
    if (oge) { gezginMenusu(e, oge.dataset.a); return; }
    if (e.target.closest('.agac, .proje-baslik')) gezginMenusu(e, '');
  });

  // Gezginde klavye: Delete, F2, Ctrl+C/X/V
  document.addEventListener('keydown', e => {
    const oge = e.target.closest?.('.agac-oge');
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && e.shiftKey && (e.key === 'A' || e.key === 'a') && D.ekran === 'duzenleyici' && D.proje) {
      e.preventDefault(); yeniOgeModal(oge ? (oge.classList.contains('klasor') ? oge.dataset.a : ustKlasor(oge.dataset.a)) : ''); return;
    }
    if (!oge) return;
    const yol = oge.dataset.a, klasor = oge.classList.contains('klasor') ? yol : ustKlasor(yol);
    if (e.key === 'Delete') { e.preventDefault(); ogeSil(yol); }
    else if (e.key === 'F2') { e.preventDefault(); ogeAdlandir(yol); }
    else if (ctrl && (e.key === 'c' || e.key === 'C')) { D.pano = { yol, kes: false }; }
    else if (ctrl && (e.key === 'x' || e.key === 'X')) { D.pano = { yol, kes: true }; }
    else if (ctrl && (e.key === 'v' || e.key === 'V')) { e.preventDefault(); yapistir(klasor); }
  });

  // Sürükle-bırak ile taşıma (Ctrl basılıysa kopyalama)
  document.addEventListener('dragstart', e => {
    const oge = e.target.closest?.('.agac-oge');
    if (!oge) return;
    e.dataTransfer.setData('text/orhunca-yol', oge.dataset.a);
    e.dataTransfer.effectAllowed = 'copyMove';
  });
  const birakmaHedefi = e => {
    const oge = e.target.closest?.('.agac-oge');
    if (oge) return oge.classList.contains('klasor') ? oge.dataset.a : ustKlasor(oge.dataset.a);
    return e.target.closest?.('.agac, .proje-baslik') ? '' : null;
  };
  document.addEventListener('dragover', e => {
    if (!e.dataTransfer.types.includes('text/orhunca-yol')) return;
    const h = birakmaHedefi(e);
    if (h === null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = e.ctrlKey ? 'copy' : 'move';
    document.querySelectorAll('.birakma').forEach(x => x.classList.remove('birakma'));
    (e.target.closest('.agac-oge.klasor') || e.target.closest('.agac'))?.classList.add('birakma');
  });
  document.addEventListener('dragleave', e => { if (!e.relatedTarget?.closest?.('.agac')) document.querySelectorAll('.birakma').forEach(x => x.classList.remove('birakma')); });
  document.addEventListener('drop', e => {
    const yol = e.dataTransfer.getData('text/orhunca-yol');
    document.querySelectorAll('.birakma').forEach(x => x.classList.remove('birakma'));
    if (!yol) return;
    const h = birakmaHedefi(e);
    if (h === null) return;
    e.preventDefault();
    if (h === yol || h.startsWith(yol + '/')) return;
    if (e.ctrlKey) ogeTasi(yol, bosAd(h, sonParca(yol)), true);
    else if (ustKlasor(yol) !== h) ogeTasi(yol, kopyalanacakYer(h, yol));
  });

  // =====================================================================
  // Yeni öğe ekle (Visual Studio'daki Add → New Item)
  // =====================================================================
  const ADI = ad => ad.charAt(0).toLocaleUpperCase('tr') + ad.slice(1);
  const OGE_SABLONLARI = [
    { kimlik: 'kod', ad: 'Kod dosyası', simge: 'description', uzanti: '.ohc', aciklama: 'Boş bir Orhunca dosyası.', icerik: ad => `# ${ad}\n\n` },
    { kimlik: 'model', ad: 'Model (sınıf)', simge: 'data_object', uzanti: '.ohc', aciklama: 'Alanları ve işlevleri olan bir veri türü. Web projelerinde veritabanına kaydedilir.',
      icerik: ad => `# ${ADI(ad)} modeli. Başka dosyadan: kullan "${ad}.ohc"\n\nmodel ${ADI(ad)}:\n    ad: metin, zorunlu, en_fazla 100\n    oluşturulma: metin\n\n# ${ADI(ad)} için kısa bir açıklama metni.\nişlev ${ad}_özeti(k: ${ADI(ad)}) -> metin:\n    döndür k.ad\n` },
    { kimlik: 'secenek', ad: 'Seçenek (enum)', simge: 'list', uzanti: '.ohc', aciklama: 'Belirli değerlerden birini alan tür.',
      icerik: ad => `# ${ADI(ad)} seçenekleri. Kullanım: ${ADI(ad)}.birinci\n\nseçenek ${ADI(ad)}: birinci, ikinci, üçüncü\n` },
    { kimlik: 'sinama', ad: 'Sınama', simge: 'science', uzanti: '_sına.ohc', aciklama: 'Adı sına_ ile başlayan işlevler sınamadır. Sınamalar panelinden çalışır.',
      icerik: ad => `# ${ad} sınamaları. Denetimler: eşit_olmalı(gerçek, beklenen), doğrula(koşul)\n# kullan "${ad}.ohc"\n\nişlev sına_örnek():\n    eşit_olmalı(1 + 1, 2)\n` },
    { kimlik: 'yol', ad: 'Web yolu', simge: 'route', uzanti: '.ohc', aciklama: 'Web sunucusuna yeni sayfalar ya da API uçları ekler.',
      icerik: ad => `# /${ad} yolları. sunucu.ohc içinde: kullan "${ad}.ohc"\n\nal "/${ad}":\n    döndür json_yanıtı({"ileti": "${ad} çalışıyor"})\n\ngönder "/${ad}":\n    döndür json_yanıtı({"alındı": değer(istek.form, "ad", "")}, 201)\n` },
    { kimlik: 'gorunum', ad: 'Görünüm (sayfa)', simge: 'web', uzanti: '.ohchtml', aciklama: 'Web sayfası şablonu. Yolda görünüm("ad", değer) ile gösterilir.',
      icerik: ad => `@model metin\n<!DOCTYPE html>\n<html lang="tr">\n<head>\n    <meta charset="utf-8">\n    <meta name="viewport" content="width=device-width, initial-scale=1">\n    <title>${ADI(ad)}</title>\n</head>\n<body>\n    <h1>${ADI(ad)}</h1>\n    <p>@model</p>\n</body>\n</html>\n` },
    { kimlik: 'bilesen', ad: 'Arayüz bileşeni', simge: 'widgets', uzanti: '.ohc', aciklama: 'Arayüzde tekrar kullanılan parça. ' + 'Kullanım: kullan "ad.ohc", sonra Ad("...").',
      icerik: ad => `# ${ADI(ad)} bileşeni. Arayüzde: ${ADI(ad)}("Başlık")\n\nbileşen ${ADI(ad)}(baslik_: metin):\n    alt_başlık(baslik_)\n    yazı("İçerik")\n` },
    { kimlik: 'css', ad: 'Stil dosyası (CSS)', simge: 'palette', uzanti: '.css', aciklama: 'Web sayfaları için stil.', icerik: () => `/* Stiller */\nbody {\n    font-family: system-ui, sans-serif;\n}\n` },
    { kimlik: 'js', ad: 'JavaScript', simge: 'javascript', uzanti: '.js', aciklama: 'Tarayıcıda çalışan betik. js_yükle ile de yüklenebilir.', icerik: () => `'use strict';\n\n` },
    { kimlik: 'json', ad: 'JSON verisi', simge: 'data_array', uzanti: '.json', aciklama: 'Ayar ya da veri dosyası.', icerik: () => `{\n}\n` },
    { kimlik: 'md', ad: 'Belge (Markdown)', simge: 'article', uzanti: '.md', aciklama: 'Açıklama ya da not.', icerik: ad => `# ${ADI(ad)}\n\n` },
  ];

  function yeniOgeModal(klasor = '') {
    if (!D.proje) return;
    D.menu = null;
    D.modal = { tur: 'yeniOge', sablon: 'model', ad: '', klasor };
    katmanlariCiz();
    setTimeout(() => $('#yeniOgeAdi')?.focus(), 30);
  }

  function yeniOgeYolu(m) {
    const s = OGE_SABLONLARI.find(x => x.kimlik === m.sablon) || OGE_SABLONLARI[0];
    let ad = (m.ad.trim() || 'yeni').replace(/\.[^.]*$/, '');
    if (s.uzanti === '_sına.ohc') ad = ad.replace(/_sına$/, '');
    const klasor = m.klasor.trim().replace(/\\/g, '/').replace(/^\/+|\/+$/g, '');
    return { s, ad, klasor, yol: birlestir(klasor, ad + s.uzanti) };
  }

  function cizYeniOge(m) {
    const { s, yol: dosya } = yeniOgeYolu(m);
    return `<div class="yeni-oge">
      <div class="yeni-oge-liste">${OGE_SABLONLARI.map(x => `<div class="yeni-oge-sablon ${x.kimlik === s.kimlik ? 'secili' : ''}" data-e="yeniOgeSec" data-ee="yeniOgeOlustur" data-a="${x.kimlik}">${S(x.simge)}<span>${kac(x.ad)}</span><span class="uzanti">${kac(x.uzanti)}</span></div>`).join('')}</div>
      <div class="yeni-oge-sag"><div class="panel-not">${kac(s.aciklama)}</div>
        <div class="alan"><label>Ad</label><input id="yeniOgeAdi" data-g="yeniOgeAdi" class="metin-girdi" value="${kac(m.ad)}" placeholder="ör. ürün" spellcheck="false"></div>
        <div class="alan"><label>Klasör</label><input data-g="yeniOgeKlasor" class="metin-girdi" value="${kac(m.klasor)}" placeholder="(proje kökü)" spellcheck="false"></div>
        <div class="panel-not">Oluşturulacak: <span class="mono" id="yeniOgeYol">${kac(dosya)}</span></div>
        ${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}</div></div>`;
  }

  async function yeniOgeOlustur() {
    const m = D.modal;
    if (m?.tur !== 'yeniOge') return;
    const { s, ad, klasor, yol } = yeniOgeYolu(m);
    if (!m.ad.trim() || /[\\/]/.test(m.ad) || klasor.split('/').includes('..')) { m.hata = 'Geçerli bir ad girin.'; katmanlariCiz(); return; }
    const r = await api('/api/dosya/yeni', { yol: tamYol(yol), klasor: false });
    if (r.hata) { m.hata = r.hata; katmanlariCiz(); return; }
    const kayit = await api('/api/dosya', { yol: tamYol(yol), icerik: s.icerik(ad) });
    if (kayit.hata) bildir(kayit.hata, true);
    D.modal = null;
    await agaciYukle();
    await dosyaAc(yol, false);
    guncelle('yan', 'sekmeler', 'kod', 'katman', 'durum');
  }

  Object.assign(EYLEM, {
    yeniOgeModal(k) { yeniOgeModal(k || ''); },
    yeniOgeSec(k) { D.modal.sablon = k; D.modal.hata = null; katmanlariCiz(); $('#yeniOgeAdi')?.focus(); },
    yeniOgeOlustur(k) { if (k) D.modal.sablon = k; yeniOgeOlustur(); },
    yeniDosyaModal(k) { if (!D.proje) return; D.modal = { tur: 'yeniDosya', ad: k ? k + '/' : '', klasor: false }; guncelleVeyaCiz(); $('#yeniDosyaAdi')?.focus(); },
    yeniKlasorModal(k) { if (!D.proje) return; D.modal = { tur: 'yeniDosya', ad: k ? k + '/' : '', klasor: true }; guncelleVeyaCiz(); $('#yeniDosyaAdi')?.focus(); },
  });
  Object.assign(GIRDI, {
    yeniOgeAdi(v) { D.modal.ad = v; const y = $('#yeniOgeYol'); if (y) y.textContent = yeniOgeYolu(D.modal).yol; },
    yeniOgeKlasor(v) { D.modal.klasor = v; const y = $('#yeniOgeYol'); if (y) y.textContent = yeniOgeYolu(D.modal).yol; },
  });
  document.addEventListener('keydown', e => {
    if (e.target.id === 'yeniOgeAdi' && e.key === 'Enter') { e.preventDefault(); yeniOgeOlustur(); }
  });

  // =====================================================================
  // Stüdyo dışında değişen dosyalar
  // =====================================================================
  let disDenetimSuruyor = false;
  async function disDegisiklikleriDenetle() {
    if (disDenetimSuruyor || D.ekran !== 'duzenleyici' || !D.proje || document.hidden) return;
    const sekmeler = D.sekmeler.filter(s => !s.ikili && s.zaman);
    if (!sekmeler.length) return;
    disDenetimSuruyor = true;
    try {
      const r = await api('/api/dosya/zamanlar', { yollar: sekmeler.map(s => tamYol(s.yol)) }).catch(() => ({}));
      for (const s of sekmeler) {
        const z = r.zamanlar?.[tamYol(s.yol)];
        if (z === undefined || !s.zaman || z === s.zaman) continue;
        if (z === 0) { s.zaman = 0; bildir(`“${sonParca(s.yol)}” Stüdyo dışında silindi ya da taşındı.`, true); continue; }
        const kirli = s.icerik !== s.kayitli;
        if (kirli && !(await onayla(`“${sonParca(s.yol)}” Stüdyo dışında değişti. Diskteki hâli yüklensin mi? (Hayır derseniz sizin değişiklikleriniz korunur.)`, { baslik: 'Dosya değişti', dugme: 'Diskten yükle' }))) { s.zaman = z; continue; }
        const d = await api('/api/dosya?' + sorgu({ yol: tamYol(s.yol) })).catch(x => ({ hata: x.message }));
        if (d.hata || d.ikili) continue;
        s.icerik = s.kayitli = d.icerik; s.zaman = d.zaman;
        if (!kirli) bildir(`“${sonParca(s.yol)}” dışarıda değişti; yeniden yüklendi.`);
        if (s.yol === D.etkin) guncelle('kod', 'sekmeler');
        else guncelle('sekmeler');
      }
    } finally { disDenetimSuruyor = false; }
  }
  window.addEventListener('focus', () => { if (D.ekran === 'duzenleyici' && D.proje) { disDegisiklikleriDenetle(); agaciYukle().then(() => D.yanPanel === 'gezgin' && cizYanPanel()); } });
  setInterval(disDegisiklikleriDenetle, 4000);
