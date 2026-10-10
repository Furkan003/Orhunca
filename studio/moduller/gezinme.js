/* Orhunca Stüdyo — Kodda gezinme: tanıma git (F12), üzerine gelince bilgi, parametre yardımı,
 * hızlı dosya açma (Ctrl+P), komut paleti (Ctrl+Shift+P), sekme menüsü, kapanan sekmeyi
 * geri açma ve ekranı bölme.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Üzerine gelince bilgi (kullanıcının tanımladığı işlev, model, değişken)
  // =====================================================================
  let bilgiZamanlayici = null, bilgiBekleyen = '';
  const bilgiOnbellek = new Map();
  function bilgiAyikla(md) {
    if (!md) return null;
    const m = String(md).match(/```orhunca\n([\s\S]*?)\n```\n?([\s\S]*)/);
    return m ? { bilgi: true, mesaj: m[1], ipucu: m[2].trim() } : { bilgi: true, mesaj: String(md).replace(/\*\*/g, ''), ipucu: '' };
  }
  async function bilgiAl(s, satir, sutun) {
    const anahtar = `${s.yol}|${satir}|${sutun}|${s.icerik.length}`;
    if (bilgiOnbellek.has(anahtar)) return bilgiOnbellek.get(anahtar);
    const r = await api('/api/bilgi', { dosya: tamYol(s.yol), icerik: s.icerik, satir: satir - 1, sutun: sutun - 1 }).catch(() => ({}));
    const b = bilgiAyikla(r.metin);
    if (bilgiOnbellek.size > 300) bilgiOnbellek.clear();
    bilgiOnbellek.set(anahtar, b);
    return b;
  }
  function kullaniciBilgisi(satir, sutun, x, y) {
    const s = etkinSekme();
    clearTimeout(bilgiZamanlayici);
    if (!s || !s.yol.endsWith('.ohc')) { baloncukGoster(null); return; }
    const anahtar = `${satir}|${sutun}`;
    if (anahtar === bilgiBekleyen) return;
    bilgiBekleyen = anahtar;
    baloncukGoster(null);
    bilgiZamanlayici = setTimeout(async () => {
      const b = await bilgiAl(s, satir, sutun);
      if (bilgiBekleyen === anahtar && etkinSekme() === s) baloncukGoster(b, x, y);
    }, 400);
  }

  // =====================================================================
  // Tanıma git (F12, Ctrl+tıklama) ve geri dönme (Alt+←)
  // =====================================================================
  const gezinmeGecmisi = [];
  async function tanimaGit(satir = D.imlec.satir, sutun = D.imlec.sutun) {
    const s = etkinSekme();
    if (!s || s.ikili || !s.yol.endsWith('.ohc')) return;
    const r = await api('/api/tanim', { dosya: tamYol(s.yol), icerik: s.icerik, satir: satir - 1, sutun: sutun - 1 }).catch(e => ({ hata: e.message }));
    if (r.hata) return bildir(r.hata, true);
    if (r.yok) return bildir('Burada tanımı bulunabilecek bir isim yok.');
    gezinmeGecmisi.push({ yol: s.yol, satir: D.imlec.satir, sutun: D.imlec.sutun });
    if (gezinmeGecmisi.length > 50) gezinmeGecmisi.shift();
    const hedef = goreliYol(r.dosya);
    if (hedef !== s.yol) await dosyaAc(hedef);
    satiraGit(r.satir, r.sutun);
  }
  async function geriGit() {
    const g = gezinmeGecmisi.pop();
    if (!g) return;
    if (g.yol !== D.etkin) await dosyaAc(g.yol);
    satiraGit(g.satir, g.sutun);
  }
  document.addEventListener('click', e => {
    if (e.target.id !== 'kodAlani' || !(e.ctrlKey || e.metaKey)) return;
    const ta = e.target, once = ta.value.slice(0, ta.selectionStart);
    tanimaGit(once.split('\n').length, ta.selectionStart - once.lastIndexOf('\n'));
  });

  // =====================================================================
  // Parametre yardımı: işlev( yazınca beklenen değerler
  // =====================================================================
  /** İmlecin içinde bulunduğu çağrı: { ad, adKonum, arg } */
  function cagriBul(metin, konum) {
    let derinlik = 0, arg = 0, tirnak = false;
    for (let i = konum - 1; i >= 0 && metin[i] !== '\n'; i--) {
      const c = metin[i];
      if (c === '"') { tirnak = !tirnak; continue; }
      if (tirnak) continue;
      if (c === ')' || c === ']' || c === '}') derinlik++;
      else if (c === '(' || c === '[' || c === '{') {
        if (derinlik === 0) {
          if (c !== '(') return null;
          const m = metin.slice(Math.max(0, i - 80), i).match(new RegExp(`([${HARF}][${HARF}0-9]*)$`, 'u'));
          return m ? { ad: m[1], adKonum: i - m[1].length, arg } : null;
        }
        derinlik--;
      } else if (c === ',' && derinlik === 0) arg++;
    }
    return null;
  }
  /** "ad(a: metin, b: sayı) -> x" imzasında `arg` sıradaki parametre kalın gösterilir. */
  function imzaHtml(imza, arg) {
    const bas = imza.indexOf('(');
    if (bas < 0) return kac(imza);
    let d = 0, son = -1;
    const parcalar = [];
    let onceki = bas + 1;
    for (let i = bas; i < imza.length; i++) {
      const c = imza[i];
      if (c === '(' || c === '<' || c === '[') d++;
      else if (c === ')' || c === '>' || c === ']') { d--; if (d === 0) { son = i; break; } }
      else if (c === ',' && d === 1) { parcalar.push(imza.slice(onceki, i)); onceki = i + 1; }
    }
    if (son < 0) return kac(imza);
    parcalar.push(imza.slice(onceki, son));
    const p = parcalar.map((x, i) => i === arg ? `<b>${kac(x.trim())}</b>` : kac(x.trim())).join(', ');
    return kac(imza.slice(0, bas + 1)) + p + kac(imza.slice(son));
  }
  let imzaZamanlayici = null;
  function parametreYardimi() {
    clearTimeout(imzaZamanlayici);
    imzaZamanlayici = setTimeout(async () => {
      const ta = $('#kodAlani'), s = etkinSekme();
      const kutu = () => { let k = $('#imzaYardimi'); if (!k && $('#kodIc')) { k = document.createElement('div'); k.id = 'imzaYardimi'; k.className = 'imza-yardimi'; $('#kodIc').appendChild(k); } return k; };
      const gizle = () => $('#imzaYardimi')?.remove();
      if (!ta || !s || document.activeElement !== ta || ta.selectionStart !== ta.selectionEnd) return gizle();
      const c = cagriBul(ta.value, ta.selectionStart);
      if (!c || /^(eğer|değilse|her|döndür|yaz|ise)$/.test(c.ad)) return gizle();
      let imza = D.yerlesikler.find(x => x.ad === c.ad)?.kullanim;
      if (!imza && s.yol.endsWith('.ohc')) {
        const once = ta.value.slice(0, c.adKonum);
        const b = await bilgiAl(s, once.split('\n').length, c.adKonum - once.lastIndexOf('\n'));
        imza = b?.mesaj?.split('\n')[0];
        if (imza && !imza.includes('(')) imza = null;
      }
      if (!imza) return gizle();
      const k = kutu();
      if (!k) return;
      k.innerHTML = imzaHtml(imza.replace(/^\s*(işlev|bileşen)\s+/, '').replace(/:\s*$/, ''), c.arg);
      k.style.top = (4 + (D.imlec.satir - 1) * 21 - 26) + 'px';
      k.style.left = (56 + Math.max(0, D.imlec.sutun - 1 - c.ad.length) * karakterGenisligi) + 'px';
      if (D.imlec.satir === 1) k.style.top = '27px';
    }, 120);
  }
  document.addEventListener('input', e => { if (e.target.id === 'kodAlani') parametreYardimi(); });
  document.addEventListener('keyup', e => { if (e.target.id === 'kodAlani' && /Arrow|Home|End/.test(e.key)) parametreYardimi(); });
  document.addEventListener('focusout', e => { if (e.target.id === 'kodAlani') setTimeout(() => { if (document.activeElement?.id !== 'kodAlani') $('#imzaYardimi')?.remove(); }, 100); });

  // =====================================================================
  // Hızlı aç (Ctrl+P) ve komut paleti (Ctrl+Shift+P)
  // =====================================================================
  function komutlar() {
    const l = [];
    for (const m of MENULER) for (const o of m.ogeler) if (o !== '-' && EYLEM[o[2]]) l.push({ etiket: `${m.ad}: ${o[0].replace(/…$/, '')}`, kisayol: o[1], eylem: o[2] });
    const goruldu = new Set();
    return l.filter(k => !goruldu.has(k.eylem) && goruldu.add(k.eylem));
  }
  /** Bulanık eşleşme puanı (küçük daha iyi); eşleşmezse null. */
  function puan(metin, q) {
    const m = kucuk(metin), ad = kucuk(sonParca(metin));
    if (!q) return 0;
    let i = ad.indexOf(q);
    if (i >= 0) return i + (ad.length - q.length) / 100;
    i = m.indexOf(q);
    if (i >= 0) return 50 + i;
    let j = 0;
    for (const c of m) if (c === q[j]) j++;
    return j === q.length ? 200 + m.length : null;
  }
  function hizliSonuclar(m) {
    const g = m.metin;
    if (g.startsWith(':')) { const n = parseInt(g.slice(1), 10); return n > 0 ? [{ etiket: `${n}. satıra git`, satir: n }] : []; }
    if (g.startsWith('>')) {
      const q = kucuk(g.slice(1).trim());
      return komutlar().map(k => ({ ...k, p: puan(k.etiket, q) })).filter(k => k.p !== null).sort((a, b) => a.p - b.p).slice(0, 60);
    }
    const q = kucuk(g.trim());
    const son = D.sonAcilanlar || [];
    return D.agac.filter(x => !x.klasor).map(x => ({ etiket: x.yol, yol: x.yol, p: puan(x.yol, q) }))
      .filter(x => x.p !== null).map(x => ({ ...x, p: q ? x.p : -(1000 - (son.indexOf(x.yol) + 1 || 1000)) }))
      .sort((a, b) => a.p - b.p).slice(0, 60);
  }
  function hizliListeCiz() {
    const m = D.modal, l = $('#hizliListe');
    if (!l || m?.tur !== 'hizli') return;
    m.sonuclar = hizliSonuclar(m);
    m.secili = Math.min(m.secili, Math.max(0, m.sonuclar.length - 1));
    l.innerHTML = m.sonuclar.map((x, i) => `<div class="hizli-oge ${i === m.secili ? 'secili' : ''}" data-e="hizliSec" data-a="${i}">${x.yol ? `<span>${kac(sonParca(x.yol))}</span><span class="hizli-yol">${kac(x.yol)}</span>` : `<span>${kac(x.etiket)}</span>${x.kisayol ? `<span class="kisayol">${kac(x.kisayol)}</span>` : ''}`}</div>`).join('')
      || '<div class="panel-not" style="padding:10px">Sonuç yok.</div>';
    l.querySelector('.secili')?.scrollIntoView({ block: 'nearest' });
  }
  function hizliAc(onek = '') {
    if (D.ekran !== 'duzenleyici' || !D.proje) return;
    D.menu = null;
    D.modal = { tur: 'hizli', metin: onek, secili: 0, sonuclar: [] };
    katmanlariCiz();
    const g = $('#hizliGirdi');
    g?.focus(); g?.setSelectionRange(onek.length, onek.length);
    hizliListeCiz();
  }
  function cizHizli(m) {
    return `<div class="ortu saydam ust" data-e="modalDis"><div class="modal hizli" data-e="hic">
      <input id="hizliGirdi" class="metin-girdi" data-g="hizliGirdi" value="${kac(m.metin)}" placeholder="Dosya adı yazın · > komut · : satır" spellcheck="false" autocomplete="off">
      <div class="hizli-liste" id="hizliListe"></div></div></div>`;
  }
  async function hizliCalistir(i) {
    const m = D.modal, x = m?.sonuclar?.[i];
    if (!x) return;
    D.modal = null;
    katmanlariCiz();
    if (x.yol) await dosyaAc(x.yol);
    else if (x.satir) satiraGit(x.satir);
    else if (x.eylem) EYLEM[x.eylem]?.();
  }
  document.addEventListener('keydown', e => {
    if (e.target.id !== 'hizliGirdi') return;
    const m = D.modal;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); m.secili = Math.max(0, Math.min(m.sonuclar.length - 1, m.secili + (e.key === 'ArrowDown' ? 1 : -1))); hizliListeCiz(); }
    else if (e.key === 'Enter') { e.preventDefault(); hizliCalistir(m.secili); }
  });

  // =====================================================================
  // Sekmeler: sağ tık menüsü, kapanan sekmeyi geri açma
  // =====================================================================
  const kapananlar = [];
  const tekKapat = EYLEM.sekmeKapat;
  EYLEM.sekmeKapat = async (yol, el, e) => {
    const vardi = D.sekmeler.some(s => s.yol === yol);
    await tekKapat(yol, el, e || { stopPropagation() {} });
    if (vardi && !D.sekmeler.some(s => s.yol === yol)) {
      kapananlar.push(yol);
      if (D.bolme === yol) { D.bolme = null; guncelle('kod'); }
    }
  };
  async function sekmeleriKapat(kapatilacak) {
    if (!kapatilacak.length) return;
    const kirli = kapatilacak.filter(s => !s.ikili && s.icerik !== s.kayitli);
    if (kirli.length && !(await tumunuKaydet()) && !(await onayla(`${kirli.length} dosyada kaydedilmemiş değişiklik var. Yine de kapatılsın mı?`, { dugme: 'Kaydetmeden kapat', tehlikeli: true }))) return;
    const etkinIndeks = D.sekmeler.findIndex(s => s.yol === D.etkin);
    for (const s of kapatilacak) kapananlar.push(s.yol);
    D.sekmeler = D.sekmeler.filter(s => !kapatilacak.includes(s));
    if (!D.sekmeler.some(s => s.yol === D.etkin)) D.etkin = (D.sekmeler[etkinIndeks] || D.sekmeler[D.sekmeler.length - 1])?.yol || null;
    if (D.bolme && !D.sekmeler.some(s => s.yol === D.bolme)) D.bolme = null;
    guncelle('sekmeler', 'kod', 'yan', 'durum');
  }
  async function kapananiAc() {
    while (kapananlar.length) {
      const y = kapananlar.pop();
      if (D.agac.some(g => g.yol === y)) { await dosyaAc(y); return; }
    }
  }
  document.addEventListener('contextmenu', e => {
    const sekme = e.target.closest('.sekme[data-a]');
    if (!sekme) return;
    const yol = sekme.dataset.a, i = D.sekmeler.findIndex(s => s.yol === yol);
    sagMenuAc(e, [
      ['Kapat', () => EYLEM.sekmeKapat(yol), 'Ctrl+W'],
      ['Diğerlerini kapat', () => sekmeleriKapat(D.sekmeler.filter(s => s.yol !== yol))],
      ['Sağdakileri kapat', () => sekmeleriKapat(D.sekmeler.slice(i + 1))],
      ['Kaydedilmişleri kapat', () => sekmeleriKapat(D.sekmeler.filter(s => s.icerik === s.kayitli))],
      ['Tümünü kapat', () => sekmeleriKapat([...D.sekmeler])],
      '-',
      ['Kapanan sekmeyi geri aç', kapananiAc, 'Ctrl+⇧+T'],
      ['Yana böl', () => ekraniBol(yol), 'Ctrl+\\'],
      '-',
      ['Yolu kopyala', () => panoyaYaz(tamYol(yol))],
      ['Gezginde göster', () => { const p = yol.split('/'); for (let k = 1; k < p.length; k++) D.kapaliKlasorler.delete(p.slice(0, k).join('/')); D.yanPanel = 'gezgin'; guncelle('etkinlik', 'yan'); setTimeout(() => document.querySelector(`.agac-oge[data-a="${CSS.escape(yol)}"]`)?.scrollIntoView({ block: 'center' }), 30); }],
      ['Dosya gezgininde göster', () => api('/api/dosya/goster', { yol: tamYol(yol) })],
    ]);
  });
  // Son açılan dosyalar (Ctrl+P'de önce gelir)
  const ilkDosyaAc = dosyaAc;
  dosyaAc = async function (yol, ciz_ = true) {
    await ilkDosyaAc(yol, ciz_);
    D.sonAcilanlar = [yol, ...(D.sonAcilanlar || []).filter(y => y !== yol)].slice(0, 30);
  };

  // =====================================================================
  // Ekranı bölme: sağda ikinci bir düzenleyici
  // =====================================================================
  D.bolme = null;
  function ekraniBol(yol) {
    const s = D.sekmeler.find(x => x.yol === (yol || D.etkin));
    if (!s || s.ikili) return;
    D.bolme = D.bolme === s.yol && !yol ? null : s.yol;
    guncelle('kod');
  }
  function ikinciDuzenleyici() {
    const bolge = $('#kodBolge');
    $('#ikinciBolge')?.remove();
    if (!bolge) return;
    const s = D.bolme && D.sekmeler.find(x => x.yol === D.bolme);
    bolge.classList.toggle('bolunmus', !!s && !!etkinSekme());
    if (!s || !etkinSekme()) return;
    const d = document.createElement('div');
    d.id = 'ikinciBolge';
    d.className = 'ikinci-bolge';
    d.innerHTML = `<div class="ikinci-baslik"><span class="ikinci-sec" title="Bu dosyayı ana düzenleyicide aç">${kac(s.yol)}</span><span class="simge" data-e="bolmeKapat" title="Bölmeyi kapat">close</span></div>
      <div class="kod-kap ikinci-kap"><div class="kod-ic"><div class="satir-nolari ikinci-nolar"></div><pre class="vurgu-katman"></pre><textarea class="kod-alani" spellcheck="false" autocapitalize="off" autocomplete="off" wrap="off"></textarea></div></div>`;
    bolge.appendChild(d);
    const ta = d.querySelector('textarea'), pre = d.querySelector('pre'), nolar = d.querySelector('.ikinci-nolar');
    const yenile = () => {
      const satirlar = s.icerik.split('\n');
      pre.innerHTML = satirlar.map(x => vurgula(x, dilBul(s.yol))).join('\n') + '\n';
      if (nolar.childElementCount !== satirlar.length) nolar.innerHTML = satirlar.map((_, i) => `<div>${i + 1}</div>`).join('');
      ta.style.height = (satirlar.length * 21 + 8) + 'px';
      ta.style.width = Math.max(pre.scrollWidth, d.clientWidth - 56) + 'px';
    };
    ta.value = s.icerik;
    yenile();
    d.yenile = () => { if (ta.value !== s.icerik) { const [a, b] = [ta.selectionStart, ta.selectionEnd]; ta.value = s.icerik; ta.setSelectionRange(a, b); } yenile(); };
    ta.addEventListener('input', () => {
      const onceKirli = s.icerik !== s.kayitli;
      s.icerik = ta.value;
      yenile();
      if (D.etkin === s.yol) { const ana = $('#kodAlani'); if (ana) { const [a, b] = [ana.selectionStart, ana.selectionEnd]; ana.value = s.icerik; ana.setSelectionRange(a, b); vurguyuGuncelle(); } }
      if (onceKirli !== (s.icerik !== s.kayitli)) cizSekmeler();
      kaydetmeyiPlanla();
      denetlemeyiPlanla();
    });
    ta.addEventListener('keydown', e => {
      if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); metinEkle(ta, '    '); }
      else if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) { e.preventDefault(); e.stopPropagation(); kaydet(s); }
    });
    d.querySelector('.ikinci-sec').addEventListener('click', () => { D.etkin = s.yol; guncelle('sekmeler', 'kod', 'yan', 'durum'); });
  }
  const bolmesizCizKod = cizKod;
  cizKod = function () { bolmesizCizKod(); ikinciDuzenleyici(); };
  document.addEventListener('input', e => {
    if (e.target.id === 'kodAlani' && D.bolme === D.etkin) $('#ikinciBolge')?.yenile?.();
  });

  Object.assign(EYLEM, {
    tanimaGit() { tanimaGit(); },
    geriGit,
    hizliAc() { hizliAc(''); },
    komutPaleti() { hizliAc('>'); },
    hizliSec(i) { hizliCalistir(+i); },
    kapananiAc,
    ekraniBol() { ekraniBol(); },
    bolmeKapat() { D.bolme = null; guncelle('kod'); },
  });
  Object.assign(GIRDI, {
    hizliGirdi(v) { D.modal.metin = v; D.modal.secili = 0; hizliListeCiz(); },
  });

  document.addEventListener('keydown', e => {
    if (D.ekran !== 'duzenleyici' || !D.proje) return;
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase();
    if (ctrl && e.shiftKey && k === 'p') { e.preventDefault(); hizliAc('>'); return; }
    if (e.key === 'F1' && !D.modal) { e.preventDefault(); hizliAc('>'); return; }
    if (D.modal) return;
    if (ctrl && !e.shiftKey && k === 'p') { e.preventDefault(); hizliAc(''); return; }
    if (ctrl && e.shiftKey && k === 't') { e.preventDefault(); kapananiAc(); return; }
    if (ctrl && !e.shiftKey && k === 'w' && D.etkin) { e.preventDefault(); EYLEM.sekmeKapat(D.etkin); return; }
    if (ctrl && e.key === '\\') { e.preventDefault(); ekraniBol(); return; }
    if (e.key === 'F12' && !e.shiftKey && e.target.id === 'kodAlani') { e.preventDefault(); tanimaGit(); return; }
    if (e.altKey && e.key === 'ArrowLeft' && e.target.id === 'kodAlani') { e.preventDefault(); geriGit(); return; }
    if (e.key === 'Escape' && $('#imzaYardimi')) $('#imzaYardimi').remove();
  });
