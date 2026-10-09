/* Orhunca Stüdyo — Türkçe klavye yardımı ve otomatik tamamlama.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // ---- Türkçe klavyesi olmayanlar için yardım
  const TURKCE_HARF_TUSLARI = { KeyC: ['ç', 'Ç'], KeyG: ['ğ', 'Ğ'], KeyI: ['ı', 'İ'], KeyO: ['ö', 'Ö'], KeyS: ['ş', 'Ş'], KeyU: ['ü', 'Ü'] };
  const ASCII_HARF = { ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u', Ç: 'C', Ğ: 'G', İ: 'I', Ö: 'O', Ş: 'S', Ü: 'U' };
  const asciiYap = (m) => m.replace(/[çğıöşüÇĞİÖŞÜ]/g, (h) => ASCII_HARF[h]);
  /** Türkçe harfsiz yazılmış anahtar kelime ve yerleşik adları: `eger` → `eğer`. */
  function asciiKarsiliklar() {
    const m = new Map();
    for (const a of [...ANAHTAR_KELIMELER, ...D.yerlesikler.map((y) => y.ad)]) {
      const k = asciiYap(a);
      if (k !== a) m.set(k, a);
    }
    // ASCII hâli de bir ad olan (iki yazımı da geçerli) kelimeler düzeltilmez.
    for (const a of [...ANAHTAR_KELIMELER, ...D.yerlesikler.map((y) => y.ad)]) m.delete(a);
    return m;
  }
  /**
   * Kelimeden sonra boşluk, `:`, `(`, `.`, `,` ya da satır sonu yazılınca Türkçe harfsiz yazılmış
   * anahtar kelimeyi düzeltir (`eger ` → `eğer `). Dosyada aynı adla tanımlanmış bir isim varsa,
   * metin ve yorum içindeyse dokunulmaz. Ctrl+Z düzeltmeyi geri alır.
   */
  function turkceHarfleriDuzelt(ta, e) {
    if (e.inputType !== 'insertText' || !e.data || !/^[\s:(.,]/.test(e.data)) return false;
    if (dilBul(etkinSekme()?.yol || '') !== 'ohc') return false;
    const v = ta.value, son = ta.selectionStart - e.data.length;
    if (son < 0 || ta.selectionStart !== ta.selectionEnd) return false;
    const once = v.slice(v.lastIndexOf('\n', son - 1) + 1, son);
    const tirnak = (once.replace(/\\./g, '').match(/"/g) || []).length;
    if (tirnak % 2 === 1 || /#/.test(once.replace(/"(?:[^"\\]|\\.)*"/g, '""'))) return false;
    const m = /(?:^|[^\p{L}\p{N}_'’])([A-Za-z_][A-Za-z0-9_]*)$/u.exec(once);
    if (!m) return false;
    const dogru = asciiKarsiliklar().get(m[1]);
    if (!dogru) return false;
    // Kullanıcı bu adı kendisi tanımlamışsa (ör. `icin = 5`) düzeltilmez.
    const ad = m[1].replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const tanim = new RegExp(`^\\s*${ad}\\s*(:[^=\\n]*)?=(?!=)|(işlev|her|fiil)\\s+${ad}(?![\\p{L}\\p{N}_])`, 'mu');
    if (tanim.test(v.slice(0, son - m[1].length) + v.slice(son + e.data.length))) return false;
    const bas = son - m[1].length, imlec = ta.selectionStart;
    ta.setSelectionRange(bas, son);
    metinEkle(ta, dogru);
    ta.setSelectionRange(imlec, imlec);
    return true;
  }

  // ---- Otomatik tamamlama: isimler (anahtar kelimeler, yerleşikler, dosyadaki
  // isimler) ve kesme işaretinden sonra ünlü uyumuna uygun ekler.
  let TAMAMLA = null, tamamlamaSayaci = 0;
  const KELIME_RE = new RegExp(`[${HARF}_][${HARF}0-9_]*`, 'gu');
  function tamamlamaKapat() { TAMAMLA = null; $('#tamamla')?.remove(); }
  async function tamamlamayiGuncelle(ta, zorla = false) {
    const v = ta.value, k = ta.selectionStart;
    if (k !== ta.selectionEnd || dilBul(etkinSekme()?.yol || '') !== 'ohc') { tamamlamaKapat(); return; }
    const satirBas = v.lastIndexOf('\n', k - 1) + 1;
    const once = v.slice(satirBas, k);
    // Metin ve yorum içinde önerilmez
    const tirnak = (once.replace(/\\./g, '').match(/"/g) || []).length;
    if (tirnak % 2 === 1 || /#/.test(once.replace(/"(?:[^"\\]|\\.)*"/g, '""'))) { tamamlamaKapat(); return; }
    const sayac = ++tamamlamaSayaci;
    // 1) Ek: `x'` → x'in doğru ekleri
    const ek = /((?:"(?:[^"\\]|\\.)*")|[\p{L}\p{N}_]+|\))['’]([\p{L}]*)$/u.exec(once);
    if (ek) {
      let ifade = ek[1];
      if (ifade === ')') {
        // parantezin açılışını bul
        let d = 0, i = once.length - ek[0].length;
        for (; i >= 0; i--) { if (once[i] === ')') d++; else if (once[i] === '(' && --d === 0) break; }
        ifade = once.slice(Math.max(0, i), once.length - ek[0].length + 1);
      }
      const r = await api('/api/ekler?' + sorgu({ ifade })).catch(() => null);
      if (!r || sayac !== tamamlamaSayaci) return;
      const yazilan = ek[2];
      const liste = r.ekler.filter(x => x.ek.startsWith(yazilan) && x.ek !== yazilan).map(x => ({ ad: x.ek, yazilan, ayrinti: x.hal, tur: 'ek' }));
      tamamlamaAc(ta, liste, k - yazilan.length);
      return;
    }
    // 2) İsim: en az 2 harf (Ctrl+Boşluk ile 0 harf)
    const kelime = /[\p{L}_][\p{L}\p{N}_]*$/u.exec(once);
    const onek = kelime ? kelime[0] : '';
    if (!zorla && onek.length < 2) { tamamlamaKapat(); return; }
    const adlar = new Map();
    for (const a of ANAHTAR_KELIMELER) adlar.set(a, 'anahtar kelime');
    for (const y of D.yerlesikler) adlar.set(y.ad, y.kullanim);
    for (const m of v.matchAll(KELIME_RE)) if (!adlar.has(m[0]) && m[0].length > 1) adlar.set(m[0], 'isim');
    // Türkçe harfsiz yazılan önek de eşleşir: `deg` → `değilse`, `eger` → `eğer`.
    const asciiOnek = asciiYap(onek);
    const liste = [...adlar].filter(([a]) => a !== onek && (a.startsWith(onek) || asciiYap(a).startsWith(asciiOnek)))
      .sort((a, b) => (a[0].startsWith(onek) ? 0 : 1) - (b[0].startsWith(onek) ? 0 : 1) || (a[1] === 'isim' ? 0 : 1) - (b[1] === 'isim' ? 0 : 1) || a[0].length - b[0].length || a[0].localeCompare(b[0], 'tr'))
      .slice(0, 8).map(([ad, ayrinti]) => ({ ad, yazilan: onek, ayrinti, tur: 'isim' }));
    tamamlamaAc(ta, liste, k - onek.length);
  }
  function tamamlamaAc(ta, liste, bas) {
    if (!liste.length) { tamamlamaKapat(); return; }
    TAMAMLA = { liste, secili: 0, bas };
    tamamlamaCiz();
  }
  function tamamlamaCiz() {
    const ta = $('#kodAlani');
    if (!TAMAMLA || !ta) return;
    let kutu = $('#tamamla');
    if (!kutu) { kutu = document.createElement('div'); kutu.id = 'tamamla'; kutu.className = 'tamamla'; $('#kodIc').appendChild(kutu); }
    const once = ta.value.slice(0, TAMAMLA.bas);
    const satir = once.split('\n').length, sutun = TAMAMLA.bas - once.lastIndexOf('\n') - 1;
    kutu.style.top = (4 + satir * 21) + 'px';
    kutu.style.left = (56 + sutun * karakterGenisligi) + 'px';
    kutu.innerHTML = TAMAMLA.liste.map((x, i) => `<div class="tamamla-oge ${i === TAMAMLA.secili ? 'secili' : ''}" data-i="${i}"><span class="tamamla-ad">${x.tur === 'ek' ? "'" : ''}${kac(x.ad)}</span><span class="tamamla-ayrinti">${kac(x.ayrinti)}</span></div>`).join('');
    kutu.querySelectorAll('.tamamla-oge').forEach(o => o.addEventListener('mousedown', e => { e.preventDefault(); tamamlamaUygula(ta, +o.dataset.i); }));
  }
  function tamamlamaUygula(ta, i) {
    const x = TAMAMLA?.liste[i];
    if (!x) return;
    const k = ta.selectionStart;
    ta.setSelectionRange(k - x.yazilan.length, k);
    tamamlamaKapat();
    metinEkle(ta, x.ad);
    tamamlamaKapat();
  }

  /** Satırdaki `sira` konumundaki kelime */
  function kelimeBul(satir, sira) {
    const re = new RegExp(`[${HARF}][${HARF}0-9]*`, 'gu');
    let m;
    while ((m = re.exec(satir))) if (sira >= m.index && sira < m.index + m[0].length) return m[0];
    return null;
  }

  function baloncukGoster(h, x, y) {
    let b = $('#baloncuk');
    if (!h) { b?.remove(); return; }
    if (!b) { b = document.createElement('div'); b.id = 'baloncuk'; document.body.appendChild(b); }
    b.className = 'baloncuk' + (h.bilgi ? ' bilgi' : h.duzeltme !== undefined ? ' uyari' : '');
    b.innerHTML = h.bilgi ? `<div class="mono">${kac(h.mesaj)}</div><div class="ipucu">${kac(h.ipucu)}</div>`
      : `<div>${kac(h.mesaj)}</div>${h.ipucu ? `<div class="ipucu">ipucu: ${kac(h.ipucu)}</div>` : ''}`;
    b.style.left = Math.min(x + 12, innerWidth - 480) + 'px';
    b.style.top = (y + 18) + 'px';
  }

  function cizAltPanel() {
    const p = $('#altPanel');
    p.classList.toggle('gizli', !D.altPanel);
    const sorunSayisi = D.sorunlar.length + D.uyarilar.length;
    const sekme = (id, ad) => `<span class="${D.altSekme === id ? 'etkin' : ''}" data-e="altSekme" data-a="${id}">${ad}</span>`;
    let icerik;
    if (D.altSekme === 'sorunlar') {
      const hatalar = D.sorunlar.map((h, i) => `<div class="sorun" data-e="sorunaGit" data-a="${i}">${S('error')}<div><span>${kac(h.mesaj)}</span><span class="yer">${kac(h.dosya ? goreliYol(h.dosya) : '')}${h.satir ? `:${h.satir}:${h.sutun}` : ''}</span>${h.ipucu ? `<div class="ipucu">ipucu: ${kac(h.ipucu)}</div>` : ''}</div>${h.duzeltme ? `<span class="duzelt" data-e="hataDuzelt" data-a="${i}" title="${kac(h.duzeltme.baslik)}">Düzelt</span>` : ''}</div>`).join('');
      const uyarilar = D.uyarilar.map((h, i) => `<div class="sorun uyari" data-e="uyariyaGit" data-a="${i}">${S('warning')}<div><span>${kac(h.mesaj)}</span><span class="yer">${kac(goreliYol(h.dosya))}:${h.satir}:${h.sutun}</span></div><span class="duzelt" data-e="uyariDuzelt" data-a="${i}">Düzelt</span></div>`).join('');
      icerik = hatalar + uyarilar || '<div class="tl dim">Sorun yok.</div>';
    } else {
      const liste = D.altSekme === 'cikti' ? D.cikti : D.terminal;
      icerik = liste.map(l => `<div class="tl ${l.c || ''}">${kac(l.t.replace(/\n$/, ''))}</div>`).join('')
        + (D.altSekme === 'terminal' && D.calisma ? `<div class="terminal-girdi"><span>›</span><input id="terminalGirdi" placeholder="Programa girdi yazıp Enter'a basın" autocomplete="off" spellcheck="false"></div>` : '')
        + (!liste.length && !D.calisma ? `<div class="tl dim">${D.altSekme === 'cikti' ? 'Derleme çıktısı burada görünür.' : 'Çalıştırmak için F5’e basın.'}</div>` : '');
    }
    const odak = odakKaydet();
    p.innerHTML = `<div class="alt-sekmeler">${sekme('terminal', 'TERMİNAL')}${sekme('sorunlar', `SORUNLAR${sorunSayisi ? ' (' + sorunSayisi + ')' : ''}`)}${sekme('cikti', 'ÇIKTI')}
      <div style="flex:1"></div><span class="simge" title="${D.calisma ? 'Durdur' : 'Çalıştır'}" data-e="${D.calisma ? 'durdur' : 'calistir'}">${D.calisma ? 'stop' : 'add'}</span><span class="simge" title="Temizle" data-e="terminalTemizle">delete</span></div>
      <div class="terminal" id="terminal">${icerik}</div>`;
    const t = $('#terminal');
    t.scrollTop = t.scrollHeight;
    if (odak && odak.id === 'terminalGirdi') odakGeriYukle(odak);
  }

  function cizDurum() {
    const n = D.sorunlar.length, u = D.uyarilar.length;
    $('#durumCubugu').innerHTML = `<div class="durum-rozet"><span class="gokturk">${GOKTURK}</span>Orhunca</div>
      ${D.proje.dal ? `<span class="ogeler">${S('fork_right')}${kac(D.proje.dal)}</span>` : ''}
      ${D.denetimHatasi ? `<span class="tiklanir uyarili" data-e="denetleKomut" title="${kac(D.denetimHatasi)} — yeniden denemek için tıklayın">${S('warning')} Denetlenemedi</span>` : `<span class="tiklanir ${n ? 'hatali' : u ? 'uyarili' : ''}" data-e="altSorunlar">${n} hata · ${u} uyarı</span>`}
      <div style="flex:1"></div>
      <span id="durumImlec">Satır ${D.imlec.satir}, Sütun ${D.imlec.sutun}</span><span>UTF-8</span><span>${kac(surumAdi())}</span>
      ${D.proje.guvenilir === false ? `<span class="tiklanir uyarili" data-e="kisitliModBilgi" title="Projeye güvenmek için tıklayın">${S('lock')} Kısıtlı mod</span>` : ''}
      <span>${kac(calismaEtiketi())}</span>`;
  }

  function calismaEtiketi() {
    const o = D.onizleme;
    if (D.proje?.web && o?.durum === 'acik') return o.arayuz ? 'Arayüz: önizlemede' : D.onizlemeAcik ? 'Önizleme: açık' : `Sunucu: ${o.kapi}`;
    if (D.calisma) return 'Çalışıyor…';
    return D.proje?.web ? 'Web' : 'Konsol';
  }

