/* Orhunca Stüdyo — Kod alanı: çizim, imleç, kesme noktaları ve tuşlar.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  const etkinSekme = () => D.sekmeler.find(s => s.yol === D.etkin);
  let karakterGenisligi = 7.8;

  function cizKod() {
    const kap = $('#kodBolge'), s = etkinSekme();
    document.documentElement.style.setProperty('--kod-boyut', D.yaziBoyutu + 'px');
    if (!s) {
      kap.innerHTML = `<div class="bos-duzenleyici"><div class="gokturk">${GOKTURK}</div><div class="kisayollar">Çalıştır<span>F5</span><br>Denetle<span>F7</span><br>Kaydet<span>Ctrl+S</span></div></div>`;
      return;
    }
    if (s.ikili) {
      kap.innerHTML = `<div class="bos-duzenleyici">${S('draft', '', 'font-size:40px')}<div>Bu dosya metin değil; düzenleyicide açılamaz.</div></div>`;
      return;
    }
    kap.innerHTML = `<div class="kod-kap" id="kodKap"><div class="kod-ic" id="kodIc">
      <div class="etkin-satir" id="etkinSatir"></div><div class="ayiklama-satir" id="ayiklamaSatir" hidden></div><div id="hataKatmani"></div><div id="parantezKatmani"></div>
      <div class="satir-nolari" id="satirNolari"></div>
      <pre class="vurgu-katman" id="vurguKatman"></pre>
      <textarea class="kod-alani" id="kodAlani" spellcheck="false" autocapitalize="off" autocomplete="off" autocorrect="off" wrap="off"></textarea>
    </div></div>`;
    const ta = $('#kodAlani');
    ta.value = s.icerik;
    // karakter genişliği (hata çizgileri için)
    const olcu = document.createElement('span');
    olcu.textContent = 'x'.repeat(100);
    olcu.style.cssText = 'position:absolute;visibility:hidden;white-space:pre';
    $('#kodIc').appendChild(olcu);
    karakterGenisligi = olcu.getBoundingClientRect().width / 100 || 7.8;
    olcu.remove();
    vurguyuGuncelle();
    $('#kodKap').scrollTop = s.kaydirma?.y || 0;
    $('#kodKap').scrollLeft = s.kaydirma?.x || 0;
    ta.focus({ preventScroll: true });
    ta.setSelectionRange(s.secim?.[0] || 0, s.secim?.[1] || 0);
    imleciGuncelle();
    duzenleyiciOlaylari(ta);
  }

  function vurguyuGuncelle() {
    const ta = $('#kodAlani'), s = etkinSekme();
    if (!ta || !s) return;
    const satirlar = s.icerik.split('\n');
    const dil = dilBul(s.yol);
    $('#vurguKatman').innerHTML = satirlar.map(x => vurgula(x, dil)).join('\n') + '\n';
    const no = $('#satirNolari');
    if (no.childElementCount !== satirlar.length) {
      no.innerHTML = satirlar.map((_, i) => `<div data-e="kesmeDegistir" data-a="${i + 1}" title="Kesme noktası ekle/kaldır (F9)">${i + 1}</div>`).join('');
    }
    const pre = $('#vurguKatman');
    ta.style.width = Math.max(pre.scrollWidth, $('#kodKap').clientWidth - 56) + 'px';
    ta.style.height = (satirlar.length * 21 + 8) + 'px';
    isaretleriCiz();
  }

  /** Hata çizgileri ve satır numarası işaretleri */
  function isaretleriCiz() {
    const katman = $('#hataKatmani'), s = etkinSekme();
    if (!katman || !s) return;
    const tam = normal(tamYol(s.yol));
    const satirlar = s.icerik.split('\n');
    const benim = D.sorunlar.filter(h => h.dosya && normal(h.dosya) === tam && h.satir > 0);
    const uyarilar = D.uyarilar.filter(h => normal(h.dosya) === tam);
    katman.innerHTML = benim.map(h => {
      const metin = satirlar[h.satir - 1] ?? '';
      const bas = Math.max(0, h.sutun - 1);
      const uz = Math.max(2, metin.length - bas);
      return `<div class="hata-cizgi" style="top:${4 + (h.satir - 1) * 21}px;left:${56 + bas * karakterGenisligi}px;width:${uz * karakterGenisligi}px"></div>`;
    }).join('') + uyarilar.map(h => `<div class="hata-cizgi uyari" style="top:${4 + (h.satir - 1) * 21}px;left:${56 + (h.sutun - 1) * karakterGenisligi}px;width:${Math.max(1, h.uzunluk) * karakterGenisligi}px"></div>`).join('');
    const no = $('#satirNolari');
    const kesmeler = D.kesmeler[tam] || [];
    if (no) [...no.children].forEach((d, i) => {
      const a = kesmeler.includes(i + 1) ? D.kesmeAyar[tam + ':' + (i + 1)] : null;
      d.classList.toggle('hatali', benim.some(h => h.satir === i + 1));
      d.classList.toggle('kesme', kesmeler.includes(i + 1));
      d.classList.toggle('kosullu', !!a?.kosul && !a?.gunluk);
      d.classList.toggle('gunluk', !!a?.gunluk);
    });
    // Hata ayıklayıcının durduğu satır (seçili çağrı çerçevesi)
    const ay = $('#ayiklamaSatir'), yer = ayiklamaYeri();
    if (ay) {
      ay.hidden = !(yer && normal(yer.dosya) === tam);
      if (!ay.hidden) {
        ay.style.top = (4 + (yer.satir - 1) * 21) + 'px';
        ay.classList.toggle('hata', D.calisma.ay.neden === 'hata');
      }
    }
  }

  /** Durulan yer: { dosya, satir } (program durmuşsa) */
  function ayiklamaYeri() {
    const ay = D.calisma?.ay;
    if (!ay?.durdu) return null;
    return ay.yigin[ay.cerceve] || ay.yigin[0] || null;
  }

  function kesmeListesi() {
    return Object.entries(D.kesmeler).flatMap(([dosya, l]) => l.map(satir => ({ dosya, satir, ...(D.kesmeAyar[dosya + ':' + satir] || {}) })));
  }

  /** Kesme noktası ayarı: koşul ve günlük mesajı (satır numarasına sağ tık). */
  function kesmeAyarAc(satir) {
    const s = etkinSekme();
    if (!s || !satir) return;
    const tam = normal(tamYol(s.yol)), a = D.kesmeAyar[tam + ':' + satir] || {};
    D.modal = { tur: 'kesmeAyar', dosya: tam, satir, kosul: a.kosul || '', gunluk: a.gunluk || '' };
    katmanlariCiz();
    setTimeout(() => $('#kesmeKosul')?.focus(), 30);
  }

  function kesmeDegistir(satir) {
    const s = etkinSekme();
    if (!s || !satir) return;
    const tam = normal(tamYol(s.yol));
    const l = new Set(D.kesmeler[tam] || []);
    if (l.has(satir)) { l.delete(satir); delete D.kesmeAyar[tam + ':' + satir]; ayarYaz('kesmeAyar', D.kesmeAyar); } else l.add(satir);
    if (l.size) D.kesmeler[tam] = [...l].sort((a, b) => a - b); else delete D.kesmeler[tam];
    ayarYaz('kesmeler', D.kesmeler);
    isaretleriCiz();
    if (D.calisma?.ayikla) api('/api/ayikla', { kimlik: D.calisma.kimlik, komut: 'kesmeler', kesmeler: kesmeListesi() }).catch(() => null);
    if (D.yanPanel === 'calistir') cizYanPanel();
  }

  async function ayiklamaKomutu(komut) {
    const c = D.calisma;
    if (!c?.ayikla) return;
    if (komut !== 'duraklat') {
      if (!c.ay?.durdu) return;
      c.ay.durdu = false;
      guncelle('isaretler', 'yan');
    }
    const r = await api('/api/ayikla', { kimlik: c.kimlik, komut }).catch(e => ({ hata: e.message }));
    if (r.hata) bildir(r.hata, true);
  }

  async function cerceveSec(i) {
    const c = D.calisma;
    if (!c?.ay?.durdu) return;
    c.ay.cerceve = i;
    await api('/api/ayikla', { kimlik: c.kimlik, komut: 'cerceve', cerceve: i }).catch(() => null);
    await durulanYereGit();
  }

  /** Durulan dosyayı açar ve satırı gösterir. */
  async function durulanYereGit() {
    const yer = ayiklamaYeri();
    if (!yer || !D.proje) return;
    const goreli = goreliYol(yer.dosya);
    if (D.etkin !== goreli) await dosyaAc(goreli);
    else isaretleriCiz();
    const kap = $('#kodKap');
    if (kap) {
      const y = 4 + (yer.satir - 1) * 21;
      if (y < kap.scrollTop || y > kap.scrollTop + kap.clientHeight - 42) kap.scrollTop = Math.max(0, y - kap.clientHeight / 3);
    }
    guncelle('yan', 'isaretler');
  }

  function imleciGuncelle() {
    const ta = $('#kodAlani'), s = etkinSekme();
    if (!ta || !s) return;
    const once = ta.value.slice(0, ta.selectionStart);
    const satir = once.split('\n').length;
    const sutun = ta.selectionStart - once.lastIndexOf('\n');
    D.imlec = { satir, sutun };
    s.secim = [ta.selectionStart, ta.selectionEnd];
    $('#etkinSatir').style.top = (4 + (satir - 1) * 21) + 'px';
    const no = $('#satirNolari');
    no.querySelector('.etkin')?.classList.remove('etkin');
    no.children[satir - 1]?.classList.add('etkin');
    const d = $('#durumImlec');
    if (d) d.textContent = `Satır ${satir}, Sütun ${sutun}`;
    parantezleriCiz(ta);
  }

  const ACAN = { '(': ')', '[': ']', '{': '}' };
  const KAPAYAN = { ')': '(', ']': '[', '}': '{' };

  /** Metin ve yorum dışındaki parantezlerin konumları (metinler `"` ile, yorumlar `#` ile). */
  function parantezKonumlari(v) {
    const sonuc = [];
    let metinde = false, yorumda = false;
    for (let i = 0; i < v.length; i++) {
      const c = v[i];
      if (yorumda) { if (c === '\n') yorumda = false; continue; }
      if (metinde) { if (c === '\\') i++; else if (c === '"' || c === '\n') metinde = false; continue; }
      if (c === '"') metinde = true;
      else if (c === '#') yorumda = true;
      else if (ACAN[c] || KAPAYAN[c]) sonuc.push(i);
    }
    return sonuc;
  }

  /** İmlecin yanındaki parantezle eşini işaretler. */
  function parantezleriCiz(ta) {
    const katman = $('#parantezKatmani');
    if (!katman) return;
    katman.innerHTML = '';
    const v = ta.value, i = ta.selectionStart;
    if (ta.selectionEnd !== i || v.length > 200000) return;
    const konumlar = parantezKonumlari(v);
    const yer = konumlar.includes(i - 1) && KAPAYAN[v[i - 1]] ? i - 1 : konumlar.includes(i) ? i : konumlar.includes(i - 1) ? i - 1 : -1;
    if (yer < 0) return;
    const sira = konumlar.indexOf(yer);
    let derinlik = 0, es = -1;
    if (ACAN[v[yer]]) {
      for (let k = sira; k < konumlar.length; k++) {
        const c = v[konumlar[k]];
        if (ACAN[c]) derinlik++; else derinlik--;
        if (derinlik === 0) { es = ACAN[v[yer]] === c ? konumlar[k] : -2; break; }
      }
    } else {
      for (let k = sira; k >= 0; k--) {
        const c = v[konumlar[k]];
        if (KAPAYAN[c]) derinlik++; else derinlik--;
        if (derinlik === 0) { es = KAPAYAN[v[yer]] === c ? konumlar[k] : -2; break; }
      }
    }
    const kutu = (p, sinif) => {
      const once = v.slice(0, p), satir = once.split('\n').length, sutun = p - once.lastIndexOf('\n') - 1;
      return `<div class="parantez ${sinif}" style="top:${4 + (satir - 1) * 21}px;left:${56 + sutun * karakterGenisligi}px;width:${karakterGenisligi}px"></div>`;
    };
    katman.innerHTML = es >= 0 ? kutu(yer, '') + kutu(es, '') : kutu(yer, 'eslesmeyen');
  }

  /** Seçili satırları (ya da imlecin satırını) bir satır yukarı/aşağı taşır. */
  function satirlariTasi(ta, yon) {
    const v = ta.value;
    const bas = v.lastIndexOf('\n', ta.selectionStart - 1) + 1;
    let son = v.indexOf('\n', ta.selectionEnd - (ta.selectionEnd > ta.selectionStart && v[ta.selectionEnd - 1] === '\n' ? 1 : 0));
    if (son < 0) son = v.length;
    const parca = v.slice(bas, son), sb = ta.selectionStart - bas, se = ta.selectionEnd - bas;
    if (yon < 0) {
      if (bas === 0) return;
      const ust = v.lastIndexOf('\n', bas - 2) + 1;
      const onceki = v.slice(ust, bas - 1);
      ta.setSelectionRange(ust, son);
      metinEkle(ta, parca + '\n' + onceki);
      ta.setSelectionRange(ust + sb, ust + se);
    } else {
      if (son >= v.length) return;
      let alt = v.indexOf('\n', son + 1);
      if (alt < 0) alt = v.length;
      const sonraki = v.slice(son + 1, alt);
      ta.setSelectionRange(bas, alt);
      metinEkle(ta, sonraki + '\n' + parca);
      const yeni = bas + sonraki.length + 1;
      ta.setSelectionRange(yeni + sb, yeni + se);
    }
  }

  /** Görünür alanın dışına çıkan imleci kaydırır. */
  function imleciGoster() {
    const kap = $('#kodKap'), ta = $('#kodAlani');
    if (!kap || !ta) return;
    const y = 4 + (D.imlec.satir - 1) * 21, x = 56 + (D.imlec.sutun - 1) * karakterGenisligi;
    if (y < kap.scrollTop) kap.scrollTop = y - 8;
    else if (y + 21 > kap.scrollTop + kap.clientHeight) kap.scrollTop = y + 21 - kap.clientHeight + 8;
    if (x < kap.scrollLeft + 56) kap.scrollLeft = Math.max(0, x - 80);
    else if (x > kap.scrollLeft + kap.clientWidth - 24) kap.scrollLeft = x - kap.clientWidth + 80;
  }

  function metinEkle(ta, metin) {
    // execCommand geri alma geçmişini korur
    if (!document.execCommand('insertText', false, metin)) {
      ta.setRangeText(metin, ta.selectionStart, ta.selectionEnd, 'end');
      ta.dispatchEvent(new Event('input'));
    }
  }

  /** Seçili satırları dönüştürür (girinti, yorum). */
  function satirlariDonustur(ta, f) {
    const v = ta.value;
    const bas = v.lastIndexOf('\n', ta.selectionStart - 1) + 1;
    let son = v.indexOf('\n', ta.selectionEnd - (ta.selectionEnd > ta.selectionStart && v[ta.selectionEnd - 1] === '\n' ? 1 : 0));
    if (son < 0) son = v.length;
    const eski = v.slice(bas, son).split('\n');
    const yeni = f(eski);
    ta.setSelectionRange(bas, son);
    metinEkle(ta, yeni.join('\n'));
    ta.setSelectionRange(bas, bas + yeni.join('\n').length);
  }

  function yorumYap() {
    const ta = $('#kodAlani');
    if (!ta) return;
    satirlariDonustur(ta, sat => {
      const hepsi = sat.filter(l => l.trim()).every(l => /^\s*#/.test(l));
      return sat.map(l => !l.trim() ? l : hepsi ? l.replace(/^(\s*)# ?/, '$1') : l.replace(/^(\s*)/, '$1# '));
    });
  }

  function satiriCogalt() {
    const ta = $('#kodAlani');
    if (!ta) return;
    const v = ta.value, bas = v.lastIndexOf('\n', ta.selectionStart - 1) + 1;
    let son = v.indexOf('\n', ta.selectionStart);
    if (son < 0) son = v.length;
    const satir = v.slice(bas, son), konum = ta.selectionStart;
    ta.setSelectionRange(son, son);
    metinEkle(ta, '\n' + satir);
    ta.setSelectionRange(konum + satir.length + 1, konum + satir.length + 1);
  }

  let olaylarBagli = new WeakSet();
  function duzenleyiciOlaylari(ta) {
    if (olaylarBagli.has(ta)) return;
    olaylarBagli.add(ta);
    ta.addEventListener('input', (e) => {
      const s = etkinSekme();
      if (!s) return;
      if (turkceHarfleriDuzelt(ta, e)) return;
      const onceKirli = s.icerik !== s.kayitli;
      s.icerik = ta.value;
      if (/^\s*fiil\b/m.test(s.icerik) || kullaniciFiilleri.size) fiilleriTopla();
      vurguyuGuncelle();
      imleciGuncelle();
      imleciGoster();
      if (onceKirli !== (s.icerik !== s.kayitli)) cizSekmeler();
      if (D.yanPanel === 'yapi') cizYanPanel();
      denetlemeyiPlanla();
      kaydetmeyiPlanla();
      tamamlamayiGuncelle(ta);
    });
    ta.addEventListener('blur', () => setTimeout(() => tamamlamaKapat(), 150));
    ta.addEventListener('click', () => tamamlamaKapat());
    ['keyup', 'click', 'select', 'focus'].forEach(o => ta.addEventListener(o, imleciGuncelle));
    $('#kodKap').addEventListener('scroll', () => {
      const s = etkinSekme(), k = $('#kodKap');
      if (s && k) s.kaydirma = { x: k.scrollLeft, y: k.scrollTop };
      // Satır numaraları yatay kaydırmada yerinde kalır.
      const n = $('#satirNolari');
      if (n && k) { n.style.transform = k.scrollLeft ? `translateX(${k.scrollLeft}px)` : ''; n.classList.toggle('kaydirilmis', k.scrollLeft > 0); }
    });
    // Kodun altındaki boş alana tıklanınca düzenleyiciye odaklanılır, imleç sona gider.
    $('#kodKap').addEventListener('mousedown', e => {
      if (e.button !== 0 || e.target === ta || e.target.closest('.satir-nolari, #tamamla')) return;
      e.preventDefault();
      ta.focus({ preventScroll: true });
      ta.setSelectionRange(ta.value.length, ta.value.length);
      imleciGuncelle();
    });
    ta.addEventListener('mousemove', e => {
      const kap = $('#kodKap').getBoundingClientRect(), k = $('#kodKap');
      const satir = Math.floor((e.clientY - kap.top + k.scrollTop - 4) / 21) + 1;
      const s = etkinSekme();
      if (!s) return;
      const tam = normal(tamYol(s.yol));
      const sutun = Math.floor((e.clientX - kap.left + k.scrollLeft - 56) / karakterGenisligi) + 1;
      const h = D.sorunlar.find(x => x.dosya && normal(x.dosya) === tam && x.satir === satir)
        || D.uyarilar.find(x => normal(x.dosya) === tam && x.satir === satir && sutun >= x.sutun - 2 && sutun <= x.sutun + x.uzunluk);
      if (h) { baloncukGoster(h, e.clientX, e.clientY); return; }
      // Yerleşik işlevlerin açıklaması
      const metin = s.icerik.split('\n')[satir - 1] || '';
      const kelime = kelimeBul(metin, sutun - 1);
      const y = kelime && D.yerlesikler.find(x => x.ad === kelime);
      if (y) baloncukGoster({ bilgi: true, mesaj: y.kullanim, ipucu: y.aciklama }, e.clientX, e.clientY);
      else if (kelime) kullaniciBilgisi(satir, sutun, e.clientX, e.clientY);
      else baloncukGoster(null);
    });
    ta.addEventListener('mouseleave', () => { bilgiZamanlayici && clearTimeout(bilgiZamanlayici); baloncukGoster(null); });
    ta.addEventListener('keydown', e => {
      const v = ta.value, bas = ta.selectionStart, son = ta.selectionEnd;
      // Kendiliğinden açılan listede Enter yeni satırdır (yazılan kelime sessizce değişmesin);
      // öneriyi Tab seçer. Oklarla seçim yapıldıysa ya da liste Ctrl+Boşluk ile açıldıysa Enter da seçer.
      if (TAMAMLA && e.key === 'Enter' && !TAMAMLA.secildi) tamamlamaKapat();
      if (TAMAMLA && ['ArrowDown', 'ArrowUp', 'Enter', 'Tab', 'Escape'].includes(e.key)) {
        e.preventDefault();
        if (e.key === 'Escape') tamamlamaKapat();
        else if (e.key === 'ArrowDown') { TAMAMLA.secili = (TAMAMLA.secili + 1) % TAMAMLA.liste.length; TAMAMLA.secildi = true; tamamlamaCiz(); }
        else if (e.key === 'ArrowUp') { TAMAMLA.secili = (TAMAMLA.secili + TAMAMLA.liste.length - 1) % TAMAMLA.liste.length; TAMAMLA.secildi = true; tamamlamaCiz(); }
        else tamamlamaUygula(ta, TAMAMLA.secili);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === ' ') { e.preventDefault(); tamamlamayiGuncelle(ta, true); return; }
      // Türkçe klavyesi olmayanlar için: Alt+C/G/I/O/S/U → ç ğ ı ö ş ü (Shift ile büyük harf).
      // AltGr (Ctrl+Alt) dokunulmaz; Türkçe klavyedeki işaretler ona bağlıdır.
      if (e.altKey && !e.ctrlKey && !e.metaKey && TURKCE_HARF_TUSLARI[e.code]) {
        e.preventDefault();
        metinEkle(ta, TURKCE_HARF_TUSLARI[e.code][e.shiftKey ? 1 : 0]);
        return;
      }
      // Otomatik kapatma: ( [ { " eşini ekler; seçili metni sarar; kapayanın üstünden geçer.
      if (!e.ctrlKey && !e.metaKey && !e.altKey && e.key.length === 1) {
        const sonraki = v[son] ?? '';
        if ((KAPAYAN[e.key] || e.key === '"') && bas === son && sonraki === e.key) {
          e.preventDefault();
          ta.setSelectionRange(bas + 1, bas + 1);
          imleciGuncelle();
          return;
        }
        const es = ACAN[e.key] || (e.key === '"' ? '"' : null);
        if (es) {
          const secili = v.slice(bas, son);
          const once = v[bas - 1] ?? '';
          const tirnakUygun = e.key !== '"' || !/[\p{L}\p{N}_"]/u.test(once);
          if (secili && !secili.includes('\n')) {
            e.preventDefault();
            metinEkle(ta, e.key + secili + es);
            ta.setSelectionRange(bas + 1, bas + 1 + secili.length);
            return;
          }
          if (!secili && tirnakUygun && (sonraki === '' || /[\s)\]},.:]/.test(sonraki))) {
            e.preventDefault();
            metinEkle(ta, e.key + es);
            ta.setSelectionRange(bas + 1, bas + 1);
            imleciGuncelle();
            return;
          }
        }
      }
      if (e.altKey && !e.ctrlKey && !e.metaKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
        e.preventDefault();
        satirlariTasi(ta, e.key === 'ArrowUp' ? -1 : 1);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === 'g' || e.key === 'G')) {
        e.preventDefault();
        D.modal = { tur: 'satiraGit', satir: '' };
        katmanlariCiz();
        setTimeout(() => $('#satirGirdi')?.focus(), 30);
        return;
      }
      if (e.key === 'Tab') {
        e.preventDefault();
        if (e.shiftKey) satirlariDonustur(ta, sat => sat.map(l => l.replace(/^( {1,4}|\t)/, '')));
        else if (v.slice(bas, son).includes('\n')) satirlariDonustur(ta, sat => sat.map(l => '    ' + l));
        else metinEkle(ta, '    ');
      } else if (e.key === 'Enter' && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        const satirBas = v.lastIndexOf('\n', bas - 1) + 1;
        const satir = v.slice(satirBas, bas);
        let girinti = satir.match(/^[ \t]*/)[0];
        if (/:\s*(#.*)?$/.test(satir.replace(/"(?:[^"\\]|\\.)*"/g, '""'))) girinti += '    ';
        metinEkle(ta, '\n' + girinti);
      } else if (e.key === 'Backspace' && bas === son && bas > 0 && ((ACAN[v[bas - 1]] && v[bas] === ACAN[v[bas - 1]]) || (v[bas - 1] === '"' && v[bas] === '"'))) {
        // Boş parantez çiftini birlikte siler
        e.preventDefault();
        ta.setSelectionRange(bas - 1, bas + 1);
        if (!document.execCommand('delete')) metinEkle(ta, '');
      } else if (e.key === 'Backspace' && bas === son && bas > 0) {
        const satirBas = v.lastIndexOf('\n', bas - 1) + 1;
        const once = v.slice(satirBas, bas);
        if (once.length > 0 && /^ +$/.test(once) && once.length % 4 === 0) {
          e.preventDefault();
          ta.setSelectionRange(bas - 4, bas);
          // insertText('') imleci bir önceki satıra atabiliyor; silme komutu kullanılır.
          if (!document.execCommand('delete')) metinEkle(ta, '');
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key === '/') {
        e.preventDefault(); yorumYap();
      } else if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'D' || e.key === 'd')) {
        e.preventDefault(); satiriCogalt();
      } else if ((e.ctrlKey || e.metaKey) && (e.key === 'l' || e.key === 'L')) {
        e.preventDefault();
        const sb = v.lastIndexOf('\n', bas - 1) + 1; let ss = v.indexOf('\n', bas); if (ss < 0) ss = v.length;
        ta.setSelectionRange(sb, Math.min(v.length, ss + 1));
      }
    });
  }
