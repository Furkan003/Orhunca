/* Orhunca Stüdyo — arayüz. Bağımlılık yok; `orhunca stüdyo` sunucusuyla konuşur. */
'use strict';
(() => {
  // =====================================================================
  // Yardımcılar
  // =====================================================================
  const $ = (s, k = document) => k.querySelector(s);
  const kac = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const S = (ad, sinif = '', stil = '') => `<span class="simge ${sinif}"${stil ? ` style="${stil}"` : ''}>${ad}</span>`;
  const GOKTURK = '𐰆';
  const TAURI = !!window.__TAURI__;
  const ilkBuyuk = s => s ? s.charAt(0).toLocaleUpperCase('tr') + s.slice(1) : s;
  const buyuk = s => String(s).toLocaleUpperCase('tr');
  const kucuk = s => String(s).toLocaleLowerCase('tr');
  const iki = n => String(n).padStart(2, '0');
  const tarihBicim = sn => { const d = new Date(sn * 1000); return `${iki(d.getDate())}.${iki(d.getMonth() + 1)}.${d.getFullYear()} ${iki(d.getHours())}:${iki(d.getMinutes())}`; };
  const sureBicim = ms => { const s = ms / 1000; return (s < 0.1 ? s.toFixed(2) : s.toFixed(1)).replace('.', ',') + ' sn'; };
  const boyutBicim = b => b > 1048576 ? (b / 1048576).toFixed(1).replace('.', ',') + ' MB' : Math.max(1, Math.round(b / 1024)) + ' KB';
  const uzanti = ad => { const i = ad.lastIndexOf('.'); return i > 0 ? ad.slice(i + 1).toLowerCase() : ''; };
  const sonParca = yol => yol.split(/[\\/]/).pop();
  const bekle = ms => new Promise(r => setTimeout(r, ms));
  const ayarOku = (a, v) => { try { const x = localStorage.getItem('orhunca.' + a); return x === null ? v : JSON.parse(x); } catch { return v; } };
  const ayarYaz = (a, v) => { try { localStorage.setItem('orhunca.' + a, JSON.stringify(v)); } catch { /* yok */ } };

  // Oturum anahtarı: sunucu tarayıcıyı ?anahtar=... ile açar.
  // Açılışta açılacak proje ya da dosya (masaüstünde çift tıklanan .ohc): ?ac=<tam yol>
  const ACILACAK = new URL(location.href).searchParams.get('ac');
  const ANAHTAR = (() => {
    const u = new URL(location.href);
    let a = u.searchParams.get('anahtar');
    if (a) {
      try { sessionStorage.setItem('orhunca-anahtar', a); } catch { /* yok */ }
      history.replaceState(null, '', location.pathname);
    } else {
      try { a = sessionStorage.getItem('orhunca-anahtar'); } catch { /* yok */ }
    }
    return a;
  })();

  async function api(yol, govde) {
    const r = await fetch(yol, {
      method: govde === undefined ? 'GET' : 'POST',
      headers: { 'X-Orhunca-Anahtar': ANAHTAR || '', 'Content-Type': 'application/json' },
      body: govde === undefined ? undefined : JSON.stringify(govde),
    });
    const j = await r.json().catch(() => ({ hata: 'Sunucudan geçersiz yanıt geldi.' }));
    if (r.status === 403) throw new Error(j.hata || 'Erişim reddedildi.');
    return j;
  }
  const sorgu = o => Object.entries(o).map(([a, d]) => `${a}=${encodeURIComponent(d)}`).join('&');

  // =====================================================================
  // Durum
  // =====================================================================
  const D = {
    ekran: 'baslangic', // baslangic | yeni | yapilandir | ogren | duzenleyici
    bilgi: { kullanici: '', surum: '0.0.0', isletim: 'linux', ayrac: '/', varsayilan_konum: '', ev: '' },
    yuklendi: false,
    projeler: [], sonSablonlar: [], sablonlar: [], yerlesikler: [],
    q: '', siralama: 'tarih',
    tq: '', kategori: 'Tümü', secili: 'konsol',
    projeAdi: 'yeni_konsol', adDokunuldu: false, konum: '', mevcutAdlar: new Set(),
    secenekler: { git: true, ornek: true, calistir: true, canli: true },
    olusturuluyor: false,
    proje: null, agac: [], kapaliKlasorler: new Set(),
    sekmeler: [], etkin: null, imlec: { satir: 1, sutun: 1 },
    terminal: [], cikti: [], sorunlar: [], uyarilar: [], altSekme: 'terminal', altPanel: true,
    calisma: null, argumanlar: '',
    // Kesme noktaları: { tam dosya yolu: [satır, ...] }
    kesmeler: ayarOku('kesmeler', {}),
    // Web projelerinde canlı önizleme: { kapi, adres, yol, durum: bekliyor|acik|hata|durdu, surum }
    onizleme: null, onizlemeAcik: true,
    yanPanel: 'gezgin', araMetin: '', araSonuc: [],
    paketler: [], paketKaynagi: '', paketMesgul: false,
    menu: null, modal: null, bildirim: null,
    yaziBoyutu: ayarOku('yaziBoyutu', 13),
    yazarkenDenetle: ayarOku('yazarkenDenetle', true),
    acilis: ayarOku('acilis', true),
  };

  const ayrac = () => D.bilgi.ayrac || '/';
  const tamYol = goreli => D.proje.yol.replace(/[\\/]+$/, '') + ayrac() + goreli.split('/').join(ayrac());
  /** `a/b/../c` → `a/c` (her iki ayraç da) */
  const normal = yol => {
    const parcalar = [];
    for (const p of yol.split(/[\\/]/)) {
      if (p === '..') parcalar.pop(); else if (p !== '.') parcalar.push(p);
    }
    return parcalar.join('/');
  };
  const goreliYol = tam => {
    const kok = normal(D.proje.yol), t = normal(tam);
    return t.startsWith(kok + '/') ? t.slice(kok.length + 1) : t;
  };
  const kisaYol = yol => D.bilgi.ev && yol.startsWith(D.bilgi.ev) && D.bilgi.isletim !== 'windows' ? '~' + yol.slice(D.bilgi.ev.length) : yol;
  const surumAdi = () => 'Orhunca ' + D.bilgi.surum.split('.').slice(0, 2).join('.');
  const kullaniciAdi = () => ilkBuyuk(D.bilgi.kullanici || 'geliştirici');
  const sablon = kimlik => D.sablonlar.find(s => s.kimlik === kimlik) || D.sablonlar[0];

  // =====================================================================
  // Orhunca sözdizimi renklendirme
  // =====================================================================
  const ANAHTAR_KELIMELER = new Set(['eğer', 'değilse', 'ise', 'her', 'için', 'kadar', 'işlev', 'fiil', 'döndür', 'dur', 'sürdür', 'dene', 'yakala', 'seçenek', 've', 'veya', 'değil', 'olduğu', 'sürece', 'iken', 'kullan', 'sabit', 'doğru', 'yanlış', 'ekrana', 'tıklanınca', 'değişince', 'gönderilince', 'çalınca']);
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

  // =====================================================================
  // Menüler
  // =====================================================================
  const MENULER = [
    { ad: 'Dosya', ogeler: [['Yeni dosya…', '', 'yeniDosyaModal'], ['Yeni klasör…', '', 'yeniKlasorModal'], '-', ['Kaydet', 'Ctrl+S', 'kaydet'], ['Tümünü kaydet', 'Ctrl+Alt+S', 'tumunuKaydet'], '-', ['Başlangıç ekranı', '', 'baslangicaDon'], ['Projeyi kapat', '', 'projeyiKapat'], '-', ["Stüdyo'yu kapat", '', 'studyoyuKapat']] },
    { ad: 'Düzen', ogeler: [['Geri al', 'Ctrl+Z', 'geriAl'], ['Yinele', 'Ctrl+Y', 'yinele'], '-', ['Kes', 'Ctrl+X', 'kes'], ['Kopyala', 'Ctrl+C', 'kopyala'], ['Yapıştır', 'Ctrl+V', 'yapistir'], '-', ['Satırı yorum yap', 'Ctrl+/', 'yorumYap'], ['Biçimlendir', 'Ctrl+⇧+F', 'bicimlendir']] },
    { ad: 'Seçim', ogeler: [['Tümünü seç', 'Ctrl+A', 'tumunuSec'], ['Satırı seç', 'Ctrl+L', 'satiriSec'], ['Satırı çoğalt', 'Ctrl+⇧+D', 'satiriCogalt']] },
    { ad: 'Görünüm', ogeler: [['Gezgin', '', 'panelGezgin'], ['Ara', '', 'panelAra'], ['Yapı', '', 'panelYapi'], ['Çalıştır', '', 'panelCalistir'], '-', ['Alt paneli göster/gizle', 'Ctrl+J', 'altPanelAcKapa'], '-', ['Yazıyı büyüt', 'Ctrl+=', 'yaziBuyut'], ['Yazıyı küçült', 'Ctrl+-', 'yaziKucult']] },
    { ad: 'Çalıştır', ogeler: [['Çalıştır', 'F5', 'calistir'], ['Hata ayıkla', 'F6', 'ayikla'], ['Durdur', '⇧+F5', 'durdur'], ['Denetle', 'F7', 'denetleKomut'], '-', ['Kesme noktası ekle/kaldır', 'F9', 'kesmeImlec'], ['Devam', 'F5', 'ayDevam'], ['Üstünden adım', 'F10', 'ayUstunden'], ['İçine adım', 'F11', 'ayAdim'], ['Dışına adım', '⇧+F11', 'ayCik'], '-', ['Canlı önizlemeyi göster/gizle', '', 'onizlemeAcKapa'], ['Önizlemeyi tarayıcıda aç', '', 'onizlemeTarayici'], '-', ['Linux için derle', '', 'derleLinux'], ['Windows için derle', '', 'derleWindows'], ['Web için derle (WebAssembly)', '', 'derleWeb'], '-', ['Masaüstü uygulaması (Linux)', '', 'paketleLinux'], ['Masaüstü uygulaması (Windows)', '', 'paketleWindows']] },
    { ad: 'Terminal', ogeler: [['Terminali temizle', '', 'terminalTemizle'], ['Sorunları göster', '', 'altSorunlar'], ['Çıktıyı göster', '', 'altCikti']] },
    { ad: 'Yardım', ogeler: [["Orhunca'yı öğren", '', 'ogrenAc'], ['Klavye kısayolları', '', 'kisayollarModal'], ['Sürüm notları', '', 'guncellemeModal'], '-', ['Hakkında', '', 'hakkindaModal']] },
  ];

  function cizAcilir(m) {
    return `<div class="acilir-menu">${m.ogeler.map(o => o === '-' ? '<div class="acilir-ayrac"></div>'
      : `<div class="acilir-oge" data-e="${o[2]}"><span>${kac(o[0])}</span>${o[1] ? `<span class="kisayol">${kac(o[1])}</span>` : ''}</div>`).join('')}</div>`;
  }

  // =====================================================================
  // Ortak parçalar
  // =====================================================================
  function cizBaslik() {
    const duz = D.ekran === 'duzenleyici' && D.proje;
    // Masaüstü uygulamasında (Tauri) başlık çubuğunun boş yerleri pencereyi taşır.
    const tasi = TAURI ? ' data-tauri-drag-region' : '';
    return `<div class="baslik-cubugu"${tasi}>
      <div class="logo"${tasi}><div class="logo-kutu"${tasi}><img src="simge.svg" alt=""${tasi}></div><span class="logo-ad"${tasi}>Orhunca</span></div>
      ${duz ? `<div class="menuler">${MENULER.map(m => `<span class="menu-baslik ${D.menu === m.ad ? 'acik' : ''}" data-e="menuAc" data-a="${m.ad}">${m.ad}${D.menu === m.ad ? cizAcilir(m) : ''}</span>`).join('')}</div>
      <div class="pencere-adi"${tasi}>${kac(D.proje.ad)} — Orhunca</div>` : ''}
      <div style="flex:1;align-self:stretch"${tasi}></div>
      ${TAURI ? `<div class="pencere-dugmeleri"><span class="simge" data-e="pencereKucult">remove</span><span class="simge" style="font-size:16px" data-e="pencereBuyut">crop_square</span><span class="simge kapat" data-e="pencereKapat">close</span></div>` : ''}
    </div>`;
  }

  function cizYanMenu() {
    const etkin = D.ekran === 'yapilandir' ? 'yeni' : D.ekran;
    const ogeler = [['baslangic', 'Başlangıç', 'home'], ['yeni', 'Şablonlar', 'grid_view'], ['ogren', 'Öğren', 'menu_book']];
    return `<div class="yan-menu">
      ${ogeler.map(([id, ad, simge]) => `<div class="yan-oge ${etkin === id ? 'etkin' : ''}" data-e="git" data-a="${id}"><div class="cubuk"></div>${S(simge)}<span>${ad}</span></div>`).join('')}
      <div style="flex:1"></div>
      <div class="kullanici">
        <div class="avatar">${kac(kullaniciAdi().charAt(0))}</div>
        <div class="esnek"><div class="kullanici-ad">${kac(kullaniciAdi())}</div><div class="kullanici-alt">${kac(surumAdi())} · Topluluk</div></div>
        ${S('settings', 'ayar-simge').replace('<span ', '<span data-e="ayarlarModal" title="Ayarlar" ')}
      </div>
    </div>`;
  }

  // =====================================================================
  // 01 Başlangıç
  // =====================================================================
  function cizBaslangic() {
    const ql = kucuk(D.q.trim());
    const liste = D.projeler.filter(p => !ql || kucuk(p.ad).includes(ql))
      .sort((a, b) => D.siralama === 'tarih' ? b.tarih - a.tarih : a.ad.localeCompare(b.ad, 'tr'));
    const satirlar = liste.map(p => `
      <div class="proje ${p.var ? '' : 'kayip'}" data-e="projeAc" data-a="${kac(p.yol)}" title="${p.var ? '' : 'Bu klasör artık yok'}">
        <div class="proje-simge">${S(p.simge || 'folder')}</div>
        <div style="min-width:0"><div class="proje-ad">${kac(p.ad)}<span>.ohcproj</span></div><div class="proje-yol">${kac(p.yol)}</div></div>
        <div class="proje-sag"><span class="etiket">${kac(p.var ? (p.sablon_adi || 'Orhunca projesi') : 'Bulunamadı')}</span><span class="proje-tarih">${tarihBicim(p.tarih)}</span></div>
      </div>`).join('');
    const bos = D.projeler.length === 0
      ? `<div class="bos-durum">Henüz bir proje yok.<br>Sağdaki <b>Yeni proje oluştur</b> ile başlayın ya da var olan bir projeyi açın.</div>`
      : liste.length === 0 ? `<div class="bos-durum">“${kac(D.q)}” ile eşleşen proje bulunamadı.</div>` : '';
    return `<div class="baslangic" data-screen-label="01 Başlangıç">
      <div class="baslangic-sol">
        <div><div class="selam">Tekrar hoş geldin, ${kac(kullaniciAdi())}</div><h1>Başlayın</h1></div>
        <div class="arama">${S('search')}<input id="q" data-g="q" value="${kac(D.q)}" placeholder="Son projelerde ara" autocomplete="off"><span class="tus">Alt+S</span></div>
        <div class="bolum-baslik"><h2>Son projeler</h2><span class="sayi-rozet">${liste.length}</span><div style="flex:1"></div>
          <div class="siralama" data-e="siralamaDegistir"><span>${D.siralama === 'tarih' ? 'Son açılma tarihi' : 'Ada göre (A–Z)'}</span>${S('swap_vert')}</div></div>
        <div class="proje-listesi">${satirlar}${bos}</div>
      </div>
      <div class="hizli">
        <h2>Hızlı işlemler</h2>
        <div class="islem vurgulu" data-e="git" data-a="yeni">${S('add')}<div class="esnek"><div class="islem-ust"><span class="islem-ad">Yeni proje oluştur</span><span class="islem-tus">Ctrl+⇧+N</span></div><div class="islem-alt">Konsol, web sitesi, API ya da kütüphane şablonu seçin</div></div></div>
        <div class="islem" data-e="klasorModal" data-a="ac">${S('folder_open')}<div class="esnek"><div class="islem-ad">Var olan projeyi aç</div><div class="islem-alt">Bir klasör veya .ohcproj dosyası seçin</div></div></div>
        <div class="islem" data-e="klonlaModal">${S('cloud_download')}<div class="esnek"><div class="islem-ad">Depodan klonla</div><div class="islem-alt">Git deposundan proje indirin</div></div></div>
        <div class="islem" data-e="git" data-a="ogren">${S('school')}<div class="esnek"><div class="islem-ad">Orhunca'yı öğren</div><div class="islem-alt">Türkçe anahtar kelimeler rehberi</div></div></div>
        <div style="flex:1"></div>
        <div class="guncelleme" data-e="guncellemeModal">${S('new_releases')}<div class="esnek"><div class="guncelleme-ad">Ekim 2026 güncellemesi</div><div class="guncelleme-alt">Modeller, web sunucusu, .ohchtml görünümleri ve canlı önizleme.</div></div>${S('chevron_right')}</div>
      </div>
    </div>`;
  }

  // =====================================================================
  // 02 Yeni proje
  // =====================================================================
  function dosyaSimgesi(ad, klasor) {
    if (klasor) return { gokturk: false, simge: 'folder_open' };
    const u = uzanti(ad);
    if (u === 'ohc' || u === 'ohchtml') return { gokturk: true };
    const tablo = { ohcproj: 'tune', md: 'info', json: 'data_object', svg: 'image', png: 'image', jpg: 'image', jpeg: 'image', gif: 'image', webp: 'image', ico: 'image', css: 'css', js: 'javascript', html: 'html', htm: 'html', txt: 'description' };
    return { gokturk: false, simge: tablo[u] || 'description' };
  }

  /** ['a/b.ohc', 'c.ohc'] → girintili ağaç satırları */
  function agacSatirlari(dosyalar) {
    const satirlar = [], gorulen = new Set();
    for (const f of dosyalar) {
      const p = f.split('/');
      for (let i = 0; i < p.length - 1; i++) {
        const a = p.slice(0, i + 1).join('/');
        if (!gorulen.has(a)) { gorulen.add(a); satirlar.push({ ad: p[i], derinlik: i, klasor: true, yol: a }); }
      }
      satirlar.push({ ad: p[p.length - 1], derinlik: p.length - 1, klasor: false, yol: f });
    }
    return satirlar;
  }

  function cizYeni() {
    const tql = kucuk(D.tq.trim());
    const liste = D.sablonlar.filter(t => (D.kategori === 'Tümü' || t.kategoriler.includes(D.kategori))
      && (!tql || kucuk(t.ad + ' ' + t.aciklama + ' ' + t.etiketler.join(' ')).includes(tql)));
    const sec = sablon(D.secili);
    const son = (D.sonSablonlar.length ? D.sonSablonlar : ['konsol', 'kutuphane']).map(sablon).filter(Boolean);
    const onizleme = agacSatirlari(sec.dosyalar.map(f => f.replace('{ad}', D.projeAdi.trim() || sec.kimlik))).map(r => {
      const s = dosyaSimgesi(r.ad, r.klasor);
      return `<div class="agac-satir" style="padding-left:${12 + r.derinlik * 14}px;color:${r.klasor ? 'var(--yazi2)' : 'var(--ikincil)'}">${s.gokturk ? `<span class="gokturk">${GOKTURK}</span>` : S(s.simge, '', `color:${r.klasor ? 'var(--sari)' : 'var(--soluk2)'}`)}<span>${kac(r.ad)}</span></div>`;
    }).join('');
    const kartlar = liste.map(t => {
      const secili = t.kimlik === D.secili;
      return `<div class="sablon ${secili ? 'secili' : ''} ${t.yakinda ? 'yakinda' : ''}" data-e="sablonSec" data-ee="sablonCift" data-a="${t.kimlik}">
        <div class="sablon-simge">${S(t.simge)}</div>
        <div style="min-width:0"><div class="sablon-ad">${kac(t.ad)}</div><div class="sablon-aciklama">${kac(t.aciklama)}</div>
          <div class="etiketler">${t.etiketler.map(g => `<span>${kac(g)}</span>`).join('')}</div></div>
        ${secili ? S('check_circle', 'dolu tik') : '<span></span>'}
        ${t.yakinda ? `<div class="yakinda-not" style="grid-column:2;margin-top:-4px"><div class="etiketler" style="margin-top:0"><span class="kilit">${S('lock', '', 'font-size:13px')}${kac(t.yakinda)}'da geliyor</span></div></div>` : ''}
      </div>`;
    }).join('');
    const kategoriler = ['Tümü', 'Web', 'Sunucu', 'Masaüstü', 'Konsol', 'Kütüphane'];
    return `<div class="sihirbaz" data-screen-label="02 Yeni proje">
      <div class="ust-satir"><div class="geri simge" data-e="git" data-a="baslangic">arrow_back</div><h1>Yeni proje oluşturun</h1><div style="flex:1"></div><span class="adim">Adım 1 / 2 · Şablon</span></div>
      <div class="yeni-govde">
        <div class="yeni-sol">
          <h2>Son kullanılan şablonlar</h2>
          ${son.map(t => `<div class="son-sablon" data-e="sablonSec" data-a="${t.kimlik}">${S(t.simge)}<span class="son-sablon-ad">${kac(t.ad)}</span><span class="son-sablon-alt">Orhunca</span></div>`).join('')}
          <div class="ince-ayrac"></div>
          <div><div class="kucuk-baslik">Oluşturulacak dosyalar</div><div class="secili-ad">${kac(sec.ad)}</div></div>
          <div class="dosya-onizleme">${onizleme}</div>
        </div>
        <div class="yeni-sag">
          <div class="arama">${S('search')}<input id="tq" data-g="tq" value="${kac(D.tq)}" placeholder="Şablon ara (ör. konsol, oyun, kütüphane)" autocomplete="off"></div>
          <div class="kategoriler">${kategoriler.map(k => `<div class="kategori ${D.kategori === k ? 'etkin' : ''}" data-e="kategoriSec" data-a="${k}">${k}</div>`).join('')}</div>
          <div class="sablon-listesi">${kartlar}${liste.length ? '' : '<div class="bos-durum">Bu filtreyle eşleşen şablon yok.</div>'}</div>
        </div>
      </div>
      <div class="alt-cubuk"><span class="ipucu-metni">İpucu: şablona çift tıklayarak doğrudan devam edebilirsiniz.</span>
        <div class="dugme" data-e="git" data-a="baslangic">Geri</div>
        <div class="dugme birincil ${sec.yakinda ? 'pasif' : ''}" data-e="git" data-a="yapilandir">Sonraki</div></div>
    </div>`;
  }

  // =====================================================================
  // 03 Yapılandır
  // =====================================================================
  function adHatasi() {
    const n = D.projeAdi.trim();
    if (!n) return 'Proje adı gerekli.';
    if (!/^[\p{L}\p{N}_-]+$/u.test(n)) return 'Yalnızca harf, rakam, alt çizgi (_) ve tire (-) kullanılabilir.';
    if (D.mevcutAdlar.has(n)) return 'Bu konumda aynı adlı bir proje zaten var.';
    return null;
  }

  function cizYapilandir() {
    const sec = sablon(D.secili), hata = adHatasi();
    const konum = D.konum || D.bilgi.varsayilan_konum;
    const secenekler = [['git', 'Git deposu başlat', 'Proje klasöründe yeni bir depo ve .gitignore oluşturur.'],
      ['ornek', 'Örnek içerik ekle', sec.web ? 'Şablonu çalışan bir örnek sayfayla doldurur.' : 'Şablonu çalışan bir örnek programla doldurur.'],
      sec.web ? ['canli', 'Canlı önizlemeyi aç', 'Kaydettiğiniz anda tarayıcı önizlemesi yenilenir.']
        : ['calistir', 'Açılınca çalıştır', 'Proje açıldığında ilk çalıştırma terminalde gösterilir.']];
    return `<div class="sihirbaz" data-screen-label="03 Yapılandır">
      <div class="ust-satir"><div class="geri simge" data-e="git" data-a="yeni">arrow_back</div><h1>Projenizi yapılandırın</h1><div style="flex:1"></div><span class="adim">Adım 2 / 2 · Ayarlar</span></div>
      <div class="yapilandir-govde"><div class="form">
        <div class="ozet-kart"><div class="ozet-simge">${S(sec.simge)}</div><div class="esnek"><div class="ozet-ad">${kac(sec.ad)}</div><div class="ozet-alt">${kac(sec.etiketler.join(' · '))}</div></div><div class="baglanti" data-e="git" data-a="yeni">Değiştir</div></div>
        <div class="alan"><label for="projeAdi">Proje adı</label>
          <input id="projeAdi" data-g="projeAdi" class="metin-girdi ${hata ? 'hatali' : ''}" value="${kac(D.projeAdi)}" spellcheck="false" autocomplete="off">
          ${hata ? `<div class="alan-hata">${S('error')}${kac(hata)}</div>` : ''}</div>
        <div class="alan"><label for="konum">Konum</label>
          <div class="yan-yana"><input id="konum" data-g="konum" class="metin-girdi" style="flex:1;font-size:13px" value="${kac(konum)}" spellcheck="false" autocomplete="off"><div class="kare-dugme simge" data-e="klasorModal" data-a="konum" title="Klasör seç">folder_open</div></div></div>
        <div class="yol-kutusu"><div>Proje şu klasörde oluşturulacak</div><div>${kac(konum.replace(/[\\/]+$/, '') + ayrac() + (D.projeAdi.trim() || '…'))}</div></div>
        <div class="alan" style="gap:2px"><div style="font-size:13px;font-weight:500;color:var(--yazi2);margin-bottom:6px">Seçenekler</div>
          ${secenekler.map(([a, ad, alt]) => `<div class="secenek" data-e="secenekDegistir" data-a="${a}"><div class="esnek"><div class="secenek-ad">${ad}</div><div class="secenek-alt">${alt}</div></div><div class="anahtar ${D.secenekler[a] ? 'acik' : ''}"><div></div></div></div>`).join('')}
        </div>
      </div></div>
      <div class="alt-cubuk"><div class="dugme" data-e="git" data-a="yeni">Geri</div>
        <div class="dugme birincil ${hata ? 'pasif' : ''}" data-e="olustur"><span>Oluştur</span>${S('arrow_forward', '', 'font-size:18px')}</div></div>
    </div>`;
  }

  // =====================================================================
  // 05 Öğren
  // =====================================================================
  const ANAHTAR_TABLOSU = [
    ['x = değer', 'let / var', 'yaş = 21'],
    ['sabit', 'const', 'sabit PI = 3.14159'],
    ['işlev', 'function', 'işlev topla(a, b):'],
    ['fiil', '(Orhunca’ya özgü)', "fiil sayı'yı karele:"],
    ['eğer … ise / değilse', 'if / else', "eğer yaş 18'den büyükse:"],
    ['her … için', 'for each', 'her öğe için listeden:'],
    ['… olduğu sürece', 'while', "x 10'dan küçük olduğu sürece:"],
    ['dur / sürdür', 'break / continue', 'dur'],
    ['döndür', 'return', 'döndür sonuç'],
    ['dene / yakala', 'try / catch', 'dene:  …  yakala hata:'],
    ['hata_ver(mesaj)', 'throw', 'hata_ver("geçersiz değer")'],
    ["… 'i yaz", 'print', '"Merhaba"\'yı yaz.'],
    ['ve / veya / değil', 'and / or / not', 'eğer a ve değil b ise:'],
    ['kullan', 'import', 'kullan "araçlar.ohc"'],
    ['model', 'class / struct', 'model Ürün:'],
    ['seçenek', 'enum', 'seçenek Renk: kırmızı, yeşil'],
    ["… 'i kaydet", 'save', "ürün'ü kaydet."],
    ['al / gönder "/yol":', 'GET / POST route', 'al "/ürünler":'],
    ['görünüm("ad", x)', 'render view', 'döndür görünüm("ürünler", liste)'],
    ['durum', 'state (useState)', 'durum sayaç = 0'],
    ['arayüz:', 'render / build()', 'arayüz:'],
    ['… tıklanınca:', 'onClick', 'düğme("Artır") tıklanınca:'],
    ['giriş(durum)', 'bound input (v-model)', 'giriş(ad, "Adınız")'],
    ['bileşen', 'component', 'bileşen Kart(başlık: metin):'],
  ];
  const HAL_TABLOSU = [
    ['-(y)ı / -(y)i', 'belirtme · nesne', "5'i sayılara ekle."],
    ['-(y)a / -(y)e', 'yönelme · hedef', "metni \"not.txt\"'ye yaz."],
    ['-dan / -den', 'ayrılma · kaynak', "her sayı için sayılardan:"],
    ['-(n)ın / -(n)in', 'ilgi · sahiplik', 'sayıların uzunluğunu yaz.'],
    ['-(y)la / -(y)le', 'vasıta · araç', "ad'ı (x > 0)'la doğrula."],
  ];

  function cizOgren() {
    const tablo = (b, satirlar, sinif = '') => `<div class="tablo ${sinif}"><div class="tablo-baslik">${b.map(x => `<span>${x}</span>`).join('')}</div>
      ${satirlar.map(([a, b2, c]) => `<div class="tablo-satir"><span>${kac(a)}</span><span>${kac(b2)}</span><span>${sinif ? kac(c) : vurgulaSatir(c)}</span></div>`).join('')}</div>`;
    return `<div class="ogren" data-screen-label="05 Öğren"><div class="ogren-ic">
      <div><h1>Öğren</h1><p class="giris">Orhunca'da anahtar kelimeler Türkçedir ve cümleler Türkçe gibi kurulur: değerler hâl ekleriyle işaretlenir, fiil sona gelir. Başka bir dilden geliyorsanız karşılıkları aşağıda.</p></div>
      ${tablo(['Orhunca', 'Karşılığı', 'Örnek'], ANAHTAR_TABLOSU)}
      <h2>Hâl ekleri</h2>
      ${tablo(['Ek', 'Rolü', 'Örnek'], HAL_TABLOSU)}
      <div class="cagri-kart"><div class="esnek"><div class="cagri-ad">İlk programını yaz</div><div class="cagri-alt">Konsol Uygulaması şablonuyla başla, F5 ile çalıştır, sonucu terminalde anında gör.</div></div><div class="dugme birincil" data-e="ilkProgram">Başla</div></div>
      <h2>Standart kütüphane</h2>
      ${tablo(['İşlev', 'Kullanım', 'Açıklama'], D.yerlesikler.map(y => [y.ad, y.kullanim, y.aciklama]), 'genis')}
    </div></div>`;
  }

  // =====================================================================
  // Modallar
  // =====================================================================
  function cizModal() {
    const m = D.modal;
    if (!m) return '';
    const kabuk = (baslik, govde, alt) => `<div class="ortu" data-e="modalDis"><div class="modal" data-e="hic">
      <div class="modal-baslik"><h2>${baslik}</h2>${S('close').replace('<span ', '<span data-e="modalKapat" ')}</div>
      <div class="modal-govde">${govde}</div>${alt ? `<div class="modal-alt">${alt}</div>` : ''}</div></div>`;
    if (m.tur === 'klasor') {
      const liste = m.yukleniyor ? '<div class="bos-durum"><div class="donen kucuk" style="margin:auto"></div></div>'
        : (m.klasorler || []).map((k, i) => `<div class="klasor-oge ${m.secili === i ? 'secili' : ''}" data-e="klasorSec" data-ee="klasorGir" data-a="${i}">
            ${k.proje ? `<span class="gokturk">${GOKTURK}</span>` : S('folder')}<span>${kac(k.ad)}</span>${k.proje ? '<span class="etiket">Orhunca projesi</span>' : ''}</div>`).join('')
          || '<div class="bos-durum">Bu klasörde alt klasör yok.</div>';
      const ac = m.mod === 'ac';
      return kabuk(ac ? 'Proje klasörünü seçin' : 'Konum seçin', `
        <div class="klasor-yolu"><div class="kare-dugme simge" data-e="klasorUst" title="Üst klasör">arrow_upward</div><input id="klasorYolu" data-g="klasorYolu" class="metin-girdi" value="${kac(m.yol || '')}" spellcheck="false"></div>
        <div class="klasor-listesi">${liste}</div>
        ${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}
        ${ac && m.proje ? `<div class="panel-not">${S('check_circle', 'dolu', 'font-size:16px;color:var(--vurgu);vertical-align:-3px')} Bu klasör bir Orhunca projesi.</div>` : ''}`,
        `<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="klasorOnayla">${ac ? 'Aç' : 'Bu konumu seç'}</div>`);
    }
    if (m.tur === 'klonla') {
      return kabuk('Depodan klonla', `
        <div class="alan"><label>Depo adresi</label><input id="klonUrl" data-g="klonUrl" class="metin-girdi" value="${kac(m.url)}" placeholder="https://github.com/kullanici/proje.git" spellcheck="false"></div>
        <div class="alan"><label>Konum</label><input id="klonKonum" data-g="klonKonum" class="metin-girdi" value="${kac(m.konum)}" spellcheck="false"></div>
        ${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}`,
        `${m.calisiyor ? '<div class="donen kucuk"></div><span class="ipucu-metni">Klonlanıyor…</span>' : ''}<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil ${m.calisiyor ? 'pasif' : ''}" data-e="klonla">Klonla</div>`);
    }
    if (m.tur === 'yeniDosya') {
      return kabuk(m.klasor ? 'Yeni klasör' : 'Yeni dosya', `
        <div class="alan"><label>Proje içindeki yol</label><input id="yeniDosyaAdi" data-g="yeniDosyaAdi" class="metin-girdi" value="${kac(m.ad)}" placeholder="${m.klasor ? 'araclar' : 'araclar.ohc'}" spellcheck="false"></div>
        ${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}`,
        `<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="yeniDosyaOlustur">Oluştur</div>`);
    }
    if (m.tur === 'ayarlar') {
      return kabuk('Ayarlar', `
        <div class="secenek" style="cursor:default"><div class="esnek"><div class="secenek-ad">Düzenleyici yazı boyutu</div><div class="secenek-alt">Kod ve satır numaraları</div></div>
          <div class="kare-dugme simge" style="width:32px;height:32px;font-size:18px" data-e="yaziKucult">remove</div><span class="mono" style="width:32px;text-align:center">${D.yaziBoyutu}</span><div class="kare-dugme simge" style="width:32px;height:32px;font-size:18px" data-e="yaziBuyut">add</div></div>
        <div class="secenek" data-e="yazarkenDenetleDegistir"><div class="esnek"><div class="secenek-ad">Yazarken denetle</div><div class="secenek-alt">Hatalar siz yazarken altı çizili gösterilir.</div></div><div class="anahtar ${D.yazarkenDenetle ? 'acik' : ''}"><div></div></div></div>
        <div class="secenek" data-e="acilisDegistir"><div class="esnek"><div class="secenek-ad">Açılış animasyonu</div><div class="secenek-alt">Stüdyo açılırken Orhunca logosu canlandırılır.</div></div><div class="anahtar ${D.acilis ? 'acik' : ''}"><div></div></div></div>`,
        `<div class="dugme birincil" data-e="modalKapat">Tamam</div>`);
    }
    if (m.tur === 'guncelleme') {
      return kabuk('Sürüm notları', `
        <div class="surum-notu"><h3>0.5 · Ekim 2026</h3><ul><li>Türkçe arayüz dili: <code>durum</code>, <code>arayüz:</code>, <code>düğme("Ekle") tıklanınca:</code>, <code>giriş(ad)</code>, <code>bileşen</code></li><li>WebAssembly: <code>--hedef web</code> ile tarayıcıda çalışan tek dosyalık sayfa</li><li>Stüdyo: Arayüz Uygulaması şablonu ve canlı önizlemede çalışan uygulamalar</li><li>Öz-barındırmanın ilk adımı: Orhunca ile yazılmış sözcük çözümleyici</li><li>Tipi yazılmış değişkenler: <code>işler: liste&lt;metin&gt; = []</code>; <code>kod()</code> ve <code>karakter()</code></li></ul></div>
        <div class="surum-notu"><h3>0.4</h3><ul><li>Modeller: <code>model Ürün:</code>, alan kuralları ve Türkçe doğrulama mesajları</li><li>Kalıcı kayıtlar: <code>ürün'ü kaydet.</code>, <code>Ürün.hepsi()</code>, <code>Ürün.bul(3)</code></li><li>Web sunucusu: <code>al "/ürünler":</code>, formlar, JSON API, statik dosyalar</li><li><code>.ohchtml</code> görünümleri: <code>@model</code>, <code>@düzen</code>, <code>@eğer</code>, <code>@her</code></li><li>Stüdyo: web şablonları ve kaydedince yenilenen canlı önizleme</li></ul></div>
        <div class="surum-notu"><h3>0.3</h3><ul><li>Standart kütüphane: metin, liste, dosya, matematik ve zaman işlevleri</li><li><code>sözlük</code> tipi: <code>{"elma": 5}</code></li><li><code>kullan "dosya.ohc"</code> ile birden fazla dosya, <code>sabit</code> tanımları</li><li>Tamsayı taşması denetimi, Türkçe hata açıklamaları</li><li>Orhunca Stüdyo</li></ul></div>
        <div class="surum-notu"><h3>0.2</h3><ul><li>Ondalık sayılar</li><li>Kendi fiillerinizi tanımlama: <code>fiil sayı'yı karele:</code></li><li>Otomatik bellek yönetimi (çöp toplayıcı)</li></ul></div>
        <div class="surum-notu"><h3>0.1</h3><ul><li>İlk derleyici: hâl ekleri, Türkçe koşullar ve döngüler</li><li>Cranelift ile Linux ve Windows programları</li></ul></div>`,
        `<div class="dugme birincil" data-e="modalKapat">Kapat</div>`);
    }
    if (m.tur === 'kisayollar') {
      const k = [['Çalıştır', 'F5'], ['Hata ayıkla', 'F6'], ['Durdur', '⇧+F5'], ['Denetle', 'F7'], ['Kesme noktası', 'F9'], ['Üstünden / içine adım', 'F10 / F11'], ['Kaydet', 'Ctrl+S'], ['Tümünü kaydet', 'Ctrl+Alt+S'], ['Satırı yorum yap', 'Ctrl+/'], ['Biçimlendir', 'Ctrl+⇧+F'], ['Satırı çoğalt', 'Ctrl+⇧+D'], ['Alt paneli göster/gizle', 'Ctrl+J'], ['Girinti / geri girinti', 'Tab / ⇧+Tab'], ['Projelerde ara', 'Alt+S'], ['Yeni proje', 'Ctrl+⇧+N']];
      return kabuk('Klavye kısayolları', `<div class="kisayol-listesi">${k.map(([a, b]) => `<span>${a}</span><span>${b}</span>`).join('')}</div>`, `<div class="dugme birincil" data-e="modalKapat">Kapat</div>`);
    }
    if (m.tur === 'hakkinda') {
      return kabuk('Hakkında', `<div style="display:flex;gap:16px;align-items:center"><div class="logo-kutu" style="width:48px;height:48px"><img src="simge.svg" alt=""></div>
        <div><div style="font-size:16px;font-weight:600">Orhunca Stüdyo</div><div class="panel-not">${kac(surumAdi())} (${kac(D.bilgi.surum)}) · ${kac(D.bilgi.isletim)}</div></div></div>
        <div class="panel-not">Türkçe tabanlı programlama dili Orhunca için geliştirme ortamı. Derleyici Rust ile yazılmıştır ve Cranelift ile doğrudan makine kodu üretir.</div>
        <div class="panel-not"><a href="https://github.com/furkan003/orhunca" target="_blank" rel="noopener">github.com/furkan003/orhunca</a></div>`, `<div class="dugme birincil" data-e="modalKapat">Kapat</div>`);
    }
    return '';
  }

  function cizOlusturuluyor() {
    if (!D.olusturuluyor) return '';
    return `<div class="ortu"><div class="olusturuluyor"><div class="donen"></div><div>
      <div class="olusturuluyor-ad">“${kac(D.projeAdi.trim())}” oluşturuluyor</div>
      <div class="olusturuluyor-alt">Şablon dosyaları kopyalanıyor…</div></div></div></div>`;
  }

  function cizBildirim() {
    if (!D.bildirim) return '';
    return `<div class="bildirim ${D.bildirim.hata ? 'hata' : ''}">${S(D.bildirim.hata ? 'error' : 'check_circle')}<span>${kac(D.bildirim.metin)}</span></div>`;
  }

  let bildirimZamani;
  function bildir(metin, hata = false) {
    D.bildirim = { metin, hata };
    clearTimeout(bildirimZamani);
    bildirimZamani = setTimeout(() => { D.bildirim = null; katmanlariCiz(); }, hata ? 6000 : 3500);
    katmanlariCiz();
  }

  // =====================================================================
  // Ana çizim
  // =====================================================================
  function odakKaydet() {
    const a = document.activeElement;
    if (!a || !a.id || a.tagName !== 'INPUT') return null;
    return { id: a.id, bas: a.selectionStart, son: a.selectionEnd };
  }
  function odakGeriYukle(o) {
    if (!o) return;
    const el = document.getElementById(o.id);
    if (el) { el.focus(); try { el.setSelectionRange(o.bas, o.son); } catch { /* yok */ } }
  }

  function ciz() {
    const kok = $('#uygulama');
    if (!ANAHTAR) {
      kok.innerHTML = `<div class="pencere">${cizBaslik()}<div class="tam-ekran-mesaj"><div class="gokturk">${GOKTURK}</div>
        <div style="font-size:18px;color:var(--yazi);font-weight:600">Orhunca Stüdyo</div>
        <div>Stüdyo'yu açmak için terminalde <code>orhunca stüdyo</code> komutunu çalıştırın.</div></div></div>`;
      return;
    }
    if (D.ekran === 'duzenleyici') { duzenleyiciIskelet(); return; }
    const odak = odakKaydet();
    const ekran = { baslangic: cizBaslangic, yeni: cizYeni, yapilandir: cizYapilandir, ogren: cizOgren }[D.ekran] || cizBaslangic;
    const kaydirma = $('.proje-listesi, .sablon-listesi, .ogren, .yapilandir-govde')?.scrollTop;
    kok.innerHTML = `<div class="pencere">${cizBaslik()}<div class="govde">${cizYanMenu()}${D.yuklendi ? ekran() : '<div class="tam-ekran-mesaj"><div class="donen"></div></div>'}</div><div id="katman"></div></div>`;
    const yeniKap = $('.proje-listesi, .sablon-listesi, .ogren, .yapilandir-govde');
    if (yeniKap && kaydirma) yeniKap.scrollTop = kaydirma;
    katmanlariCiz();
    odakGeriYukle(odak);
  }

  /** Modal, "oluşturuluyor" örtüsü ve bildirimler: ekranı yeniden çizmeden güncellenir. */
  function katmanlariCiz() {
    const k = $('#katman');
    if (!k) return;
    const odak = odakKaydet();
    k.innerHTML = cizModal() + cizOlusturuluyor() + cizBildirim();
    odakGeriYukle(odak);
  }

  // =====================================================================
  // 04 Düzenleyici
  // =====================================================================
  let iskeletVar = false;

  function duzenleyiciIskelet() {
    const kok = $('#uygulama');
    if (!iskeletVar || !$('#kodBolge')) {
      kok.innerHTML = `<div class="pencere"><div id="baslikKap">${cizBaslik()}</div>
        <div class="govde"><div class="duzenleyici" data-screen-label="04 Düzenleyici">
          <div class="duz-govde">
            <div class="etkinlik-cubugu" id="etkinlik"></div>
            <div class="yan-panel" id="yanPanel"></div>
            <div class="duz-orta">
              <div class="sekmeler" id="sekmeler"></div>
              <div class="kirinti" id="kirinti"></div>
              <div id="kodBolge" style="flex:1;min-height:0;display:flex;flex-direction:column"></div>
              <div class="alt-panel" id="altPanel"></div>
            </div>
            <div class="onizleme gizli" id="onizleme"></div>
          </div>
          <div class="durum-cubugu" id="durumCubugu"></div>
        </div></div><div id="katman"></div></div>`;
      iskeletVar = true;
      guncelle('hepsi');
    } else {
      guncelle('baslik');
    }
  }

  function guncelle(...parcalar) {
    if (D.ekran !== 'duzenleyici') { ciz(); return; }
    const hepsi = parcalar.includes('hepsi');
    const p = a => hepsi || parcalar.includes(a);
    if (p('baslik')) { const b = $('#baslikKap'); if (b) b.innerHTML = cizBaslik(); }
    if (p('etkinlik')) cizEtkinlik();
    if (p('yan')) cizYanPanel();
    if (p('sekmeler')) cizSekmeler();
    if (p('kod')) cizKod();
    if (p('isaretler')) isaretleriCiz();
    if (p('alt')) cizAltPanel();
    if (p('durum')) cizDurum();
    if (p('onizleme')) cizOnizleme();
    if (p('katman') || hepsi) katmanlariCiz();
  }

  function cizEtkinlik() {
    const ogeler = [['gezgin', 'description', 'Gezgin'], ['ara', 'search', 'Ara'], ['yapi', 'account_tree', 'Yapı'], ['calistir', 'play_circle', 'Çalıştır'], ['eklentiler', 'extension', 'Paketler']];
    $('#etkinlik').innerHTML = ogeler.map(([id, simge, ad]) => `<span class="simge ${D.yanPanel === id ? 'etkin' : ''}" title="${ad}" data-e="yanPanelSec" data-a="${id}">${simge}</span>`).join('')
      + `<div style="flex:1"></div><span class="simge" title="Başlangıç ekranı" data-e="baslangicaDon">home</span><span class="simge" title="Ayarlar" data-e="ayarlarModal">settings</span>`;
  }

  function gorunurAgac() {
    return D.agac.filter(g => {
      const p = g.yol.split('/');
      for (let i = 1; i < p.length; i++) if (D.kapaliKlasorler.has(p.slice(0, i).join('/'))) return false;
      return true;
    });
  }

  function cizYanPanel() {
    const kap = $('#yanPanel');
    const baslik = (ad, ek = '') => `<div class="panel-baslik"><span style="flex:1">${ad}</span>${ek}</div>`;
    if (D.yanPanel === 'gezgin') {
      const satirlar = gorunurAgac().map(g => {
        const ad = sonParca(g.yol), derinlik = g.yol.split('/').length - 1;
        const s = dosyaSimgesi(ad, g.klasor);
        const kapali = g.klasor && D.kapaliKlasorler.has(g.yol);
        const simge = g.klasor ? S(kapali ? 'folder' : 'folder_open') : s.gokturk ? `<span class="gokturk">${GOKTURK}</span>` : S(s.simge);
        return `<div class="agac-oge ${g.klasor ? 'klasor' : ''} ${D.etkin === g.yol ? 'etkin' : ''}" style="padding-left:${14 + derinlik * 14}px" data-e="${g.klasor ? 'klasorAcKapa' : 'dosyaAc'}" data-a="${kac(g.yol)}" title="${kac(g.yol)}">${simge}<span class="ad">${kac(ad)}</span></div>`;
      }).join('');
      kap.innerHTML = baslik('GEZGİN', `<span class="simge" title="Yeni dosya" data-e="yeniDosyaModal" style="margin-right:6px">note_add</span><span class="simge" title="Yenile" data-e="agaciYenile">refresh</span>`)
        + `<div class="proje-baslik">${S('expand_more')}<span>${kac(buyuk(D.proje.ad))}</span></div><div class="agac">${satirlar}</div>`;
    } else if (D.yanPanel === 'ara') {
      const gruplar = {};
      for (const r of D.araSonuc) (gruplar[r.dosya] ||= []).push(r);
      const vurgu = t => { const i = kucuk(t).indexOf(kucuk(D.araMetin)); return i < 0 ? kac(t) : kac(t.slice(0, i)) + '<b>' + kac(t.slice(i, i + D.araMetin.length)) + '</b>' + kac(t.slice(i + D.araMetin.length)); };
      kap.innerHTML = baslik('ARA') + `<div class="panel-ic"><input id="araMetin" data-g="araMetin" class="metin-girdi" placeholder="Projede ara" value="${kac(D.araMetin)}" spellcheck="false" autocomplete="off">
        ${Object.entries(gruplar).map(([d, l]) => `<div><div class="ara-dosya">${dosyaSimgesi(d).gokturk ? `<span class="gokturk" style="color:var(--vurgu)">${GOKTURK}</span>` : S('description', '', 'font-size:15px')}${kac(d)}</div>${l.map(r => `<div class="ara-sonuc" data-e="konumaGit" data-a="${kac(r.dosya)}|${r.satir}">${vurgu(r.metin)}</div>`).join('')}</div>`).join('')}
        ${D.araMetin && !D.araSonuc.length ? '<div class="panel-not">Sonuç yok.</div>' : ''}</div>`;
    } else if (D.yanPanel === 'yapi') {
      const s = etkinSekme(), ogeler = [];
      if (s && !s.ikili) s.icerik.split('\n').forEach((l, i) => {
        let m;
        if ((m = l.match(/^\s*işlev\s+([^\s(]+)/u))) ogeler.push(['işlev', m[1], i + 1]);
        else if ((m = l.match(/^\s*fiil\b(.*?)([^\s:'’()]+)\s*(->[^:]*)?:\s*$/u))) ogeler.push(['fiil', m[2], i + 1]);
        else if ((m = l.match(/^\s*sabit\s+([^\s=]+)/u))) ogeler.push(['sabit', m[1], i + 1]);
        else if ((m = l.match(/^\s*kullan\s+"([^"]+)"/u))) ogeler.push(['kullan', m[1], i + 1]);
      });
      kap.innerHTML = baslik('YAPI') + `<div class="panel-ic">${ogeler.map(([t, ad, n]) => `<div class="yapi-oge" data-e="satiraGit" data-a="${n}"><span class="tur">${t}</span><span class="${t === 'fiil' || t === 'işlev' ? 'f' : t === 'sabit' ? 't' : 's'}">${kac(ad)}</span><span class="satir-no">${n}</span></div>`).join('')
        || '<div class="panel-not">Bu dosyada işlev, fiil ya da sabit tanımı yok.</div>'}</div>`;
    } else if (D.yanPanel === 'calistir') {
      const giris = girisDosyasi();
      kap.innerHTML = baslik('ÇALIŞTIR') + `<div class="panel-ic">
        <div class="panel-not">Giriş dosyası<br><span class="mono" style="color:var(--yazi2)">${kac(giris || '—')}</span></div>
        ${D.calisma ? `<div class="panel-dugme" data-e="durdur">${S('stop')}Durdur</div>` : `<div class="panel-dugme birincil" data-e="calistir">${S('play_arrow')}Çalıştır (F5)</div><div class="panel-dugme" data-e="ayikla">${S('bug_report')}Hata ayıkla (F6)</div>`}
        ${D.calisma?.ayikla ? ayiklamaPaneli() : ''}
        <div class="alan" style="gap:6px"><label style="font-size:12px">Program argümanları</label><input id="argumanlar" data-g="argumanlar" class="metin-girdi" placeholder="ör. bir iki" value="${kac(D.argumanlar)}" spellcheck="false"></div>
        <div class="panel-dugme" data-e="denetleKomut">${S('task_alt')}Denetle (F7)</div>
        <div class="ince-ayrac"></div>
        <div class="panel-not">Dağıtım için derle (proje/cikti/)</div>
        <div class="panel-dugme" data-e="derleLinux">${S('terminal')}Linux için derle</div>
        <div class="panel-dugme" data-e="derleWindows">${S('desktop_windows')}Windows için derle</div>
        <div class="panel-dugme" data-e="derleWeb">${S('language')}Web için derle</div>
        <div class="panel-not">Arayüz programını kendi penceresinde açılan uygulamaya paketle</div>
        <div class="panel-dugme" data-e="paketleLinux">${S('select_window')}Masaüstü (Linux)</div>
        <div class="panel-dugme" data-e="paketleWindows">${S('select_window')}Masaüstü (Windows)</div>
      </div>`;
    } else {
      const liste = D.paketler.map(p => `<div class="paket-oge" title="${kac(p.kaynak)}">${S('deployed_code')}<div class="esnek"><div class="paket-ad">${kac(p.ad)}</div><div class="paket-kaynak">${kac(p.kaynak)}</div><div class="paket-kaynak">${p.kurulu ? (p.isleme || '').slice(0, 10) : '<span style="color:var(--sari)">kurulu değil</span>'}</div></div><span class="simge sil" title="Kaldır" data-e="paketKaldir" data-a="${kac(p.ad)}">delete</span></div>`).join('');
      kap.innerHTML = baslik('PAKETLER', D.paketMesgul ? '<div class="donen kucuk"></div>' : `<span class="simge" title="Yenile" data-e="paketleriYenile">refresh</span>`) + `<div class="panel-ic">
        ${liste || '<div class="panel-not">Bu projenin paketi yok. Bir Git deposundan Orhunca kütüphanesi ekleyin; kodda <span class="mono">kullan "paket_adı"</span> ile kullanılır.</div>'}
        <div class="alan" style="gap:6px"><label style="font-size:12px">Git adresi</label><input id="paketKaynagi" data-g="paketKaynagi" class="metin-girdi" placeholder="github:kişi/depo#v1.0" value="${kac(D.paketKaynagi)}" spellcheck="false"></div>
        <div class="panel-dugme birincil ${D.paketMesgul ? 'pasif' : ''}" data-e="paketEkle">${S('add')}Paket ekle</div>
        <div class="panel-dugme ${D.paketMesgul ? 'pasif' : ''}" data-e="paketYukle">${S('cloud_download')}Tümünü yükle</div>
        <div class="panel-dugme ${D.paketMesgul ? 'pasif' : ''}" data-e="paketGuncelle">${S('refresh')}Güncelle</div>
        <div class="panel-not" style="font-size:12px">Paketler projenin <span class="mono">paketler/</span> klasörüne kurulur; sürümler <span class="mono">orhunca.kilit</span> dosyasında tutulur.</div></div>`;
    }
  }

  function cizSekmeler() {
    $('#sekmeler').innerHTML = D.sekmeler.map(s => {
      const kirli = !s.ikili && s.icerik !== s.kayitli;
      return `<div class="sekme ${s.yol === D.etkin ? 'etkin' : ''}" data-e="sekmeSec" data-a="${kac(s.yol)}" title="${kac(s.yol)}"><span>${kac(sonParca(s.yol))}</span>${kirli ? `<span class="kirli" data-e="sekmeKapat" data-a="${kac(s.yol)}"></span>` : `<span class="simge kapat" data-e="sekmeKapat" data-a="${kac(s.yol)}">close</span>`}</div>`;
    }).join('');
    $('#kirinti').textContent = D.etkin ? [D.proje.ad, ...D.etkin.split('/')].join('  ›  ') : D.proje.ad;
  }

  /** Hata ayıklama: araç çubuğu, değişkenler, çağrı yığını, kesme noktaları */
  function ayiklamaPaneli() {
    const ay = D.calisma.ay, durdu = !!ay?.durdu;
    const dugme = (e, simge, ad, kisayol) => `<span class="simge ${durdu ? '' : 'pasif'}" title="${ad} (${kisayol})" data-e="${e}">${simge}</span>`;
    const arac = `<div class="ay-arac">${durdu ? dugme('ayDevam', 'play_arrow', 'Devam', 'F5') : `<span class="simge" title="Duraklat" data-e="ayDuraklat">pause</span>`}
      ${dugme('ayUstunden', 'redo', 'Üstünden adım', 'F10')}${dugme('ayAdim', 'arrow_downward', 'İçine adım', 'F11')}${dugme('ayCik', 'arrow_upward', 'Dışına adım', '⇧+F11')}
      <span class="simge" title="Durdur (⇧+F5)" data-e="durdur">stop</span>
      <span class="ay-durum">${!ay?.bagli && !durdu ? 'başlatılıyor…' : durdu ? ({ kesme: 'kesme noktası', hata: 'çalışma hatası', duraklat: 'duraklatıldı' }[ay.neden] || 'durdu') : 'çalışıyor'}</span></div>`;
    if (!durdu) return arac + kesmelerBolumu();
    const ileti = ay.ileti ? `<div class="ay-ileti">${kac(ay.ileti)}</div>` : '';
    const degiskenler = ay.degiskenler.length
      ? ay.degiskenler.map(d => `<div class="ay-deg" title="${kac(d.ad + ': ' + d.tip + ' = ' + d.deger)}"><span class="ad">${kac(d.ad)}</span><span class="tip">${kac(d.tip)}</span><span class="deger">${kac(d.deger)}</span></div>`).join('')
      : '<div class="panel-not">Bu çerçevede değişken yok.</div>';
    const yigin = ay.yigin.map((c, i) => `<div class="ay-cerceve ${i === ay.cerceve ? 'secili' : ''}" data-e="cerceveSec" data-a="${i}"><span>${kac(c.islev)}</span><span class="yer">${kac(goreliYol(c.dosya))}:${c.satir}</span></div>`).join('');
    return arac + ileti + `<div class="ay-baslik">DEĞİŞKENLER</div>${degiskenler}<div class="ay-baslik">ÇAĞRI YIĞINI</div>${yigin}` + kesmelerBolumu();
  }

  function kesmelerBolumu() {
    const l = kesmeListesi();
    return `<div class="ay-baslik">KESME NOKTALARI</div>` + (l.length
      ? l.map(k => `<div class="ay-cerceve" data-e="konumaGit" data-a="${kac(goreliYol(k.dosya))}|${k.satir}"><span>${kac(goreliYol(k.dosya).split('/').pop())}</span><span class="yer">satır ${k.satir}</span></div>`).join('')
      : '<div class="panel-not">Satır numarasına tıklayarak ekleyin (F9).</div>');
  }

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
      <div class="etkin-satir" id="etkinSatir"></div><div class="ayiklama-satir" id="ayiklamaSatir" hidden></div><div id="hataKatmani"></div>
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
      d.classList.toggle('hatali', benim.some(h => h.satir === i + 1));
      d.classList.toggle('kesme', kesmeler.includes(i + 1));
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
    return Object.entries(D.kesmeler).flatMap(([dosya, l]) => l.map(satir => ({ dosya, satir })));
  }

  function kesmeDegistir(satir) {
    const s = etkinSekme();
    if (!s || !satir) return;
    const tam = normal(tamYol(s.yol));
    const l = new Set(D.kesmeler[tam] || []);
    if (l.has(satir)) l.delete(satir); else l.add(satir);
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
    ta.addEventListener('input', () => {
      const s = etkinSekme();
      if (!s) return;
      const onceKirli = s.icerik !== s.kayitli;
      s.icerik = ta.value;
      if (/^\s*fiil\b/m.test(s.icerik) || kullaniciFiilleri.size) fiilleriTopla();
      vurguyuGuncelle();
      imleciGuncelle();
      imleciGoster();
      if (onceKirli !== (s.icerik !== s.kayitli)) cizSekmeler();
      if (D.yanPanel === 'yapi') cizYanPanel();
      denetlemeyiPlanla();
    });
    ['keyup', 'click', 'select', 'focus'].forEach(o => ta.addEventListener(o, imleciGuncelle));
    $('#kodKap').addEventListener('scroll', () => { const s = etkinSekme(), k = $('#kodKap'); if (s && k) s.kaydirma = { x: k.scrollLeft, y: k.scrollTop }; });
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
      baloncukGoster(y ? { bilgi: true, mesaj: y.kullanim, ipucu: y.aciklama } : null, e.clientX, e.clientY);
    });
    ta.addEventListener('mouseleave', () => baloncukGoster(null));
    ta.addEventListener('keydown', e => {
      const v = ta.value, bas = ta.selectionStart, son = ta.selectionEnd;
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
      } else if (e.key === 'Backspace' && bas === son && bas > 0) {
        const satirBas = v.lastIndexOf('\n', bas - 1) + 1;
        const once = v.slice(satirBas, bas);
        if (once.length > 0 && /^ +$/.test(once) && once.length % 4 === 0) {
          e.preventDefault();
          ta.setSelectionRange(bas - 4, bas);
          metinEkle(ta, '');
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
      const hatalar = D.sorunlar.map((h, i) => `<div class="sorun" data-e="sorunaGit" data-a="${i}">${S('error')}<div><span>${kac(h.mesaj)}</span><span class="yer">${kac(h.dosya ? goreliYol(h.dosya) : '')}${h.satir ? `:${h.satir}:${h.sutun}` : ''}</span>${h.ipucu ? `<div class="ipucu">ipucu: ${kac(h.ipucu)}</div>` : ''}</div></div>`).join('');
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
      <span class="tiklanir ${n ? 'hatali' : u ? 'uyarili' : ''}" data-e="altSorunlar">${n} hata · ${u} uyarı</span>
      <div style="flex:1"></div>
      <span id="durumImlec">Satır ${D.imlec.satir}, Sütun ${D.imlec.sutun}</span><span>UTF-8</span><span>${kac(surumAdi())}</span>
      <span>${kac(calismaEtiketi())}</span>`;
  }

  function calismaEtiketi() {
    const o = D.onizleme;
    if (D.proje?.web && o?.durum === 'acik') return o.arayuz ? 'Arayüz: önizlemede' : D.onizlemeAcik ? 'Önizleme: açık' : `Sunucu: ${o.kapi}`;
    if (D.calisma) return 'Çalışıyor…';
    return D.proje?.web ? 'Web' : 'Konsol';
  }

  // =====================================================================
  // Canlı önizleme (web projeleri)
  // =====================================================================
  function onizlemeAdresi() {
    const o = D.onizleme;
    return o?.adres ? o.adres + encodeURI(o.yol || '/') : null;
  }

  function cizOnizleme() {
    const k = $('#onizleme');
    if (!k) return;
    const goster = !!(D.proje?.web && D.onizlemeAcik);
    k.classList.toggle('gizli', !goster);
    if (!goster) { k.innerHTML = ''; return; }
    const genislik = ayarOku('onizlemeGenisligi', 0);
    k.style.width = genislik ? `${Math.min(genislik, window.innerWidth * 0.7)}px` : '';
    if (!$('#onizlemeGovde')) {
      k.innerHTML = `<div class="onizleme-tutamac" title="Genişliği değiştirmek için sürükleyin"></div><div class="onizleme-baslik"><span>CANLI ÖNİZLEME</span>
          <span class="simge" title="Yenile" data-e="onizlemeYenile">refresh</span>
          <span class="simge" title="Tarayıcıda aç" data-e="onizlemeTarayici">open_in_new</span>
          <span class="simge" title="Gizle" data-e="onizlemeAcKapa">close</span></div>
        <div class="onizleme-adres" id="onizlemeAdres"></div>
        <div class="onizleme-govde" id="onizlemeGovde"></div>`;
    }
    const o = D.onizleme || {}, durum = o.durum || 'yok';
    const kok = o.arayuz ? `arayüz · ${D.proje?.ad || ''}` : o.adres ? o.adres.replace(/^https?:\/\//, '') : `localhost:${o.kapi || 3000}`;
    $('#onizlemeAdres').innerHTML = `<span class="nokta ${durum}"></span><span class="adres">${kac(kok + (o.yol && o.yol !== '/' ? o.yol : ''))}</span>`;
    const g = $('#onizlemeGovde');
    if (o.adres) {
      const cerceve = $('#onizlemeCerceve');
      if (!cerceve || cerceve.dataset.surum !== String(o.surum)) {
        // Arayüz programları Stüdyo'nun kendi sunucusundan gelir: Stüdyo'ya erişemesinler diye yalıtılır.
        const yalit = o.arayuz ? ' sandbox="allow-scripts allow-forms allow-modals allow-popups"' : '';
        g.innerHTML = `<iframe id="onizlemeCerceve" data-surum="${o.surum}" src="${kac(onizlemeAdresi())}" title="Canlı önizleme"${yalit}></iframe>`;
      }
      g.querySelector('.onizleme-ortu')?.remove();
      const ortu = { bekliyor: 'Yeniden derleniyor…', hata: 'Derleme hatası — ayrıntılar terminalde', durdu: 'Sunucu durdu · F5 ile başlatın' }[durum];
      if (ortu) g.insertAdjacentHTML('beforeend', `<div class="onizleme-ortu ${durum}">${kac(ortu)}</div>`);
    } else {
      const metin = durum === 'bekliyor' ? 'Sunucu başlatılıyor…' : durum === 'hata' ? 'Derleme hatası — ayrıntılar terminalde' : 'Sayfayı görmek için projeyi çalıştırın.';
      g.innerHTML = `<div class="onizleme-bos"><div>${kac(metin)}</div>${durum === 'bekliyor' ? '' : `<div class="dugme birincil" data-e="calistir">${S('play_arrow')}Çalıştır (F5)</div>`}</div>`;
    }
  }

  /** Önizlemedeki sayfayı bulunduğu adreste yeniler; sayfa Orhunca betiğini taşıyorsa ileti, yoksa adres yeniden yüklenir. */
  let yenilemeSirasi = 0;
  function onizlemeyiYenile() {
    const c = $('#onizlemeCerceve'), o = D.onizleme;
    if (!o?.adres) return;
    const sert = () => { o.surum = (o.surum || 0) + 1; cizOnizleme(); };
    if (!c || !o.betik) { sert(); return; }
    // Sayfa (Orhunca betiğini taşıyorsa) kendini yeniler ve adresini bildirir; bildirmezse
    // (ör. JSON yanıtı açıksa) çerçeve son bilinen adresle yeniden yüklenir.
    const sira = ++yenilemeSirasi;
    o.yanitBekleniyor = sira;
    c.contentWindow.postMessage('orhunca:yenile', '*');
    setTimeout(() => { if (D.onizleme === o && o.yanitBekleniyor === sira) sert(); }, 1500);
  }

  function sunucuHazir(adres) {
    const o = D.onizleme || (D.onizleme = { surum: 0 });
    const yeni = o.adres !== adres;
    if (D.proje?.web && D.onizlemeAcik && !o.bilgilendirildi) terminaleEkle('  Kaydettiğinizde sayfa otomatik yenilenir.', 'dim');
    o.adres = adres; o.durum = 'acik'; o.bilgilendirildi = true;
    if (yeni) { o.surum = (o.surum || 0) + 1; o.betik = false; }
    guncelle('onizleme', 'durum');
    if (!yeni) onizlemeyiYenile();
  }

  // Önizleme panelinin sol kenarı sürüklenerek genişletilir.
  document.addEventListener('mousedown', e => {
    if (!e.target.classList?.contains('onizleme-tutamac')) return;
    e.preventDefault();
    const panel = $('#onizleme'), c = $('#onizlemeCerceve');
    if (c) c.style.pointerEvents = 'none'; // sürüklerken iframe fareyi yutmasın
    const tasi = h => {
      const g = Math.round(Math.max(240, Math.min(window.innerWidth - h.clientX, window.innerWidth * 0.7)));
      panel.style.width = g + 'px';
    };
    const birak = () => {
      document.removeEventListener('mousemove', tasi);
      document.removeEventListener('mouseup', birak);
      if (c) c.style.pointerEvents = '';
      ayarYaz('onizlemeGenisligi', panel.getBoundingClientRect().width);
      vurguyuGuncelle();
    };
    document.addEventListener('mousemove', tasi);
    document.addEventListener('mouseup', birak);
  });

  window.addEventListener('message', e => {
    const c = $('#onizlemeCerceve');
    if (!c || e.source !== c.contentWindow || e.data?.orhunca !== 'adres' || !D.onizleme) return;
    D.onizleme.yol = String(e.data.adres || '/');
    D.onizleme.betik = true;
    D.onizleme.yanitBekleniyor = null;
    const a = $('#onizlemeAdres .adres');
    if (a) a.textContent = D.onizleme.adres.replace(/^https?:\/\//, '') + (D.onizleme.yol !== '/' ? D.onizleme.yol : '');
  });

  /** Web projesinde kaydedilen dosyaya göre: kod değiştiyse sunucu yeniden başlar, statik dosyada sayfa yenilenir. */
  let yenidenBaslatma;
  function kayittanSonra(yol) {
    if (!D.proje?.web || !D.onizleme || D.onizleme.durum === 'durdu' && !D.calisma) return;
    clearTimeout(yenidenBaslatma);
    yenidenBaslatma = setTimeout(() => {
      if (/^statik\//.test(yol) && D.calisma) onizlemeyiYenile();
      else if (D.calisma || D.onizleme.durum === 'hata' || D.onizleme.arayuz) calistir();
    }, 120);
  }

  // =====================================================================
  // Proje işlemleri
  // =====================================================================
  function girisDosyasi() {
    if (!D.proje) return null;
    if (D.proje.giris && D.agac.some(g => g.yol === D.proje.giris)) return D.proje.giris;
    const s = etkinSekme();
    if (s && uzanti(s.yol) === 'ohc') return s.yol;
    return D.agac.find(g => !g.klasor && uzanti(g.yol) === 'ohc')?.yol || null;
  }

  async function agaciYukle() {
    const r = await api('/api/agac?' + sorgu({ kok: D.proje.yol }));
    D.agac = r.girdiler || [];
  }

  async function projeyiAc(bilgi, { ilkCalistirma = false } = {}) {
    if (D.calisma) await durdur();
    D.proje = bilgi;
    D.onizleme = null;
    D.sekmeler = []; D.etkin = null; D.sorunlar = []; D.uyarilar = []; D.terminal = []; D.cikti = [];
    D.kapaliKlasorler = new Set(); D.calisma = null; D.menu = null; D.modal = null;
    D.yanPanel = 'gezgin'; D.imlec = { satir: 1, sutun: 1 };
    await agaciYukle();
    if (D.agac.some(g => g.yol === 'paketler')) D.kapaliKlasorler.add('paketler');
    D.paketler = [];
    const giris = girisDosyasi();
    if (giris) await dosyaAc(giris, false);
    const projeDosyasi = D.agac.find(g => uzanti(g.yol) === 'ohcproj');
    if (projeDosyasi) await dosyaAc(projeDosyasi.yol, false);
    if (giris) D.etkin = giris;
    D.ekran = 'duzenleyici';
    iskeletVar = false;
    ciz();
    if (bilgi.uyari) bildir(bilgi.uyari, true);
    if (ilkCalistirma && giris) calistir();
    else {
      D.terminal = [{ t: istem(), c: 'mut' }, { t: bilgi.web ? 'Sunucuyu başlatıp sayfayı önizlemek için F5’e basın.' : 'Çalıştırmak için F5’e basın.', c: 'dim' }];
      guncelle('alt');
      denetle();
    }
  }

  async function dosyaAc(yol, ciz_ = true) {
    if (!D.sekmeler.some(s => s.yol === yol)) {
      const r = await api('/api/dosya?' + sorgu({ yol: tamYol(yol) }));
      if (r.hata) { bildir(r.hata, true); return; }
      D.sekmeler.push({ yol, icerik: r.icerik ?? '', kayitli: r.icerik ?? '', ikili: !!r.ikili });
      fiilleriTopla();
    }
    D.etkin = yol;
    if (ciz_) guncelle('sekmeler', 'kod', 'yan', 'durum');
  }

  async function kaydet(s = etkinSekme(), sessiz = false) {
    if (!s || s.ikili || s.icerik === s.kayitli) return true;
    const r = await api('/api/dosya', { yol: tamYol(s.yol), icerik: s.icerik });
    if (r.hata) { bildir(r.hata, true); return false; }
    s.kayitli = s.icerik;
    if (!sessiz) { cizSekmeler(); kayittanSonra(s.yol); }
    return true;
  }

  async function tumunuKaydet({ yenile = false } = {}) {
    let degisen = null;
    for (const s of D.sekmeler) {
      if (!s.ikili && s.icerik !== s.kayitli) degisen = s.yol;
      if (!(await kaydet(s, true))) return false;
    }
    if (D.ekran === 'duzenleyici') cizSekmeler();
    if (yenile && degisen) kayittanSonra(degisen);
    return true;
  }

  let denetimZamani;
  function denetlemeyiPlanla() {
    if (!D.yazarkenDenetle) return;
    clearTimeout(denetimZamani);
    denetimZamani = setTimeout(denetle, 450);
  }

  let denetimSirasi = 0;
  async function denetle() {
    if (!D.proje) return;
    const sira = ++denetimSirasi;
    const acik = {};
    for (const s of D.sekmeler) if (!s.ikili && (uzanti(s.yol) === 'ohc' || uzanti(s.yol) === 'ohchtml')) acik[tamYol(s.yol)] = s.icerik;
    const hedefler = [...new Set([girisDosyasi(), etkinSekme() && uzanti(D.etkin) === 'ohc' ? D.etkin : null].filter(Boolean))];
    const hatalar = [], uyarilar = [];
    const ayni = (a, b) => a.mesaj === b.mesaj && a.satir === b.satir && a.sutun === b.sutun && normal(a.dosya) === normal(b.dosya);
    for (const h of hedefler) {
      const r = await api('/api/denetle', { dosya: tamYol(h), acik }).catch(() => ({ hatalar: [] }));
      for (const x of r.hatalar || []) if (!hatalar.some(y => ayni(x, y))) hatalar.push(x);
      for (const x of r.uyarilar || []) if (!uyarilar.some(y => ayni(x, y))) uyarilar.push(x);
    }
    if (sira !== denetimSirasi || D.ekran !== 'duzenleyici') return;
    D.sorunlar = hatalar;
    D.uyarilar = uyarilar;
    guncelle('isaretler', 'durum');
    const toplam = hatalar.length + uyarilar.length;
    if (D.altSekme === 'sorunlar') guncelle('alt');
    else { const sekme = $('.alt-sekmeler span[data-a="sorunlar"]'); if (sekme) sekme.textContent = `SORUNLAR${toplam ? ' (' + toplam + ')' : ''}`; }
  }

  function istem() {
    const yol = D.proje.yol;
    return D.bilgi.isletim === 'windows' ? `PS ${yol}> orhunca çalıştır` : `${kisaYol(yol)} $ orhunca çalıştır`;
  }

  function terminaleEkle(t, c = '', birlestir = false) {
    // Programın art arda gelen çıktı parçaları tek blokta birleştirilir.
    const son = D.terminal[D.terminal.length - 1];
    if (birlestir && son && son.c === c && son.birlesik) son.t += t;
    else D.terminal.push({ t, c, birlesik: birlestir });
    if (D.terminal.length > 3000) D.terminal.splice(0, D.terminal.length - 3000);
  }

  async function calistir(ayikla = false) {
    if (!D.proje) return;
    if (D.calisma) await durdur();
    await tumunuKaydet();
    const giris = girisDosyasi();
    if (!giris) { bildir('Çalıştırılacak .ohc dosyası yok.', true); return; }
    D.altPanel = true; D.altSekme = 'terminal';
    if (D.terminal.length) terminaleEkle('');
    terminaleEkle(istem(), 'mut');
    guncelle('alt');
    const argumanlar = D.argumanlar.match(/"[^"]*"|\S+/g)?.map(a => a.replace(/^"|"$/g, '')) || [];
    const r = await api('/api/calistir', { dosya: tamYol(giris), klasor: D.proje.yol, argumanlar, ayikla, kesmeler: ayikla ? kesmeListesi() : [] }).catch(e => ({ hata: e.message }));
    if (r.derleme_hatasi) {
      terminaleEkle('✗ Derleme başarısız', 'err');
      terminaleEkle(r.derleme_hatasi, 'err');
      D.sorunlar = r.hatalar || [];
      if (D.proje.web) D.onizleme = { ...(D.onizleme || { surum: 0 }), durum: 'hata' };
      guncelle('alt', 'isaretler', 'durum', 'onizleme');
      return;
    }
    if (r.hata) { terminaleEkle(r.hata, 'err'); guncelle('alt'); return; }
    D.sorunlar = [];
    if (r.arayuz) {
      // Arayüz programı: WebAssembly'ye derlendi; sayfa önizlemede çalışır, süreç yok.
      terminaleEkle(`✓ Derleme tamamlandı (WebAssembly) · ${sureBicim(r.derleme_ms)}`, 'ok');
      if (!D.onizleme?.arayuz) terminaleEkle('  Arayüz önizlemede çalışıyor; kaydettiğinizde yenilenir.', 'dim');
      D.proje.web = true;
      const o = D.onizleme || { surum: 0 };
      D.onizleme = { ...o, adres: location.origin + r.arayuz, yol: '/', durum: 'acik', arayuz: true, betik: false, surum: (o.surum || 0) + 1 };
      guncelle('alt', 'isaretler', 'durum', 'yan', 'onizleme');
      return;
    }
    terminaleEkle(`✓ Derleme tamamlandı · ${sureBicim(r.derleme_ms)}`, 'ok');
    if (ayikla) {
      const n = kesmeListesi().length;
      terminaleEkle(`● Hata ayıklama · ${n ? n + ' kesme noktası' : 'kesme noktası yok (satır numarasına tıklayarak ekleyin)'}`, 'bilgi');
      D.yanPanel = 'calistir';
    }
    D.calisma = { kimlik: r.kimlik, konum: 0, ayikla, ay: null };
    if (D.proje.web) D.onizleme = { ...(D.onizleme || { surum: 0 }), kapi: r.kapi, durum: 'bekliyor' };
    guncelle('alt', 'isaretler', 'durum', 'yan', 'onizleme');
    $('#terminalGirdi')?.focus();
    ciktiyiIzle(r.kimlik);
  }

  async function ciktiyiIzle(kimlik) {
    while (D.calisma && D.calisma.kimlik === kimlik) {
      const r = await api('/api/cikti?' + sorgu({ kimlik, konum: D.calisma.konum })).catch(() => null);
      if (!r || r.hata) { D.calisma = null; break; }
      for (const p of r.parcalar) programCiktisi(p.t, p.tur === 'hata' ? 'err' : '');
      D.calisma.konum = r.konum;
      if (r.ayiklama) {
        const once = D.calisma.ay;
        const yeniDurak = r.ayiklama.durdu && (!once?.durdu || once.surum !== r.ayiklama.surum);
        // Çerçeve seçimi yerelde de tutulur (yanıt gelene kadar).
        if (once?.durdu && r.ayiklama.durdu && once.surum === r.ayiklama.surum && once.cerceve !== r.ayiklama.cerceve && !r.ayiklama.degiskenler.length) r.ayiklama.cerceve = once.cerceve;
        const degisti = JSON.stringify(once) !== JSON.stringify(r.ayiklama);
        D.calisma.ay = r.ayiklama;
        if (yeniDurak) {
          const yer = ayiklamaYeri();
          const neden = { kesme: 'Kesme noktasında durdu', adim: 'Durdu', duraklat: 'Duraklatıldı', hata: 'Çalışma hatasında durdu' }[r.ayiklama.neden] || 'Durdu';
          if (r.ayiklama.neden !== 'adim') terminaleEkle(`● ${neden}: ${goreliYol(yer?.dosya || '')}:${yer?.satir ?? '?'}`, r.ayiklama.neden === 'hata' ? 'err' : 'bilgi');
          if (D.ekran === 'duzenleyici') { if (D.yanPanel !== 'calistir') D.yanPanel = 'calistir'; await durulanYereGit(); guncelle('etkinlik', 'alt'); }
        } else if (degisti && D.ekran === 'duzenleyici') guncelle('yan', 'isaretler');
      }
      if (r.bitti) {
        const kod = r.kod;
        if (D.calisma.durduruldu) terminaleEkle(`— Durduruldu · ${sureBicim(r.sure_ms)}`, 'dim');
        else terminaleEkle(`— Program bitti · çıkış kodu ${kod ?? '?'} · ${sureBicim(r.sure_ms)}`, kod === 0 ? 'dim' : 'err');
        D.calisma = null;
        if (D.onizleme && D.onizleme.durum !== 'hata') D.onizleme.durum = 'durdu';
        if (D.ekran === 'duzenleyici') guncelle('alt', 'durum', 'yan', 'onizleme', 'isaretler');
        return;
      }
      if (r.parcalar.length && D.ekran === 'duzenleyici' && D.altSekme === 'terminal') cizAltPanel();
      await bekle(r.parcalar.length ? 30 : 90);
    }
  }

  /** Program çıktısını terminale ekler; web sunucusunun "dinleniyor" satırını yakalar. */
  function programCiktisi(t, c) {
    const m = c ? null : /^(.*?)(● Sunucu dinleniyor: (http:\/\/\S+))\n?/s.exec(t);
    if (!m) { terminaleEkle(t, c, true); return; }
    if (m[1]) terminaleEkle(m[1], c, true);
    terminaleEkle(m[2], 'bilgi');
    sunucuHazir(m[3]);
    const kalan = t.slice(m[0].length);
    if (kalan) terminaleEkle(kalan, c, true);
  }

  async function durdur() {
    if (!D.calisma) return;
    const c = D.calisma;
    c.durduruldu = true;
    await api('/api/durdur', { kimlik: c.kimlik }).catch(() => null);
    // Çıktı izleyicisi programın bittiğini görene kadar beklenir (en çok 2 sn).
    for (let i = 0; i < 40 && D.calisma === c; i++) await bekle(50);
  }

  async function derle(hedef) {
    if (!D.proje) return;
    await tumunuKaydet();
    const giris = girisDosyasi();
    if (!giris) { bildir('Derlenecek .ohc dosyası yok.', true); return; }
    const ad = { windows: 'Windows', web: 'Web (WebAssembly)', 'masaustu-linux': 'Linux masaüstü', 'masaustu-windows': 'Windows masaüstü' }[hedef] || 'Linux';
    D.altPanel = true; D.altSekme = 'cikti';
    D.cikti.push({ t: `${ad} için derleniyor: ${giris}`, c: 'mut' });
    guncelle('alt');
    const r = await api('/api/derle', { dosya: tamYol(giris), hedef }).catch(e => ({ hata: e.message }));
    if (r.derleme_hatasi) {
      D.cikti.push({ t: r.derleme_hatasi, c: 'err' });
      if (r.hatalar?.[0]?.satir) { D.sorunlar = r.hatalar; guncelle('isaretler', 'durum'); }
    } else if (r.hata) D.cikti.push({ t: r.hata, c: 'err' });
    else {
      D.cikti.push({ t: `✓ ${goreliYol(r.cikti)} oluşturuldu · ${boyutBicim(r.boyut)} · ${sureBicim(r.sure_ms)}`, c: 'ok' });
      bildir(`${ad} programı hazır: ${goreliYol(r.cikti)}`);
      await agaciYukle();
      guncelle('yan');
    }
    guncelle('alt');
  }

  async function verileriYukle() {
    try {
      const [bilgi, projeler, sablonlar, yerlesikler] = await Promise.all([api('/api/durum'), api('/api/projeler'), api('/api/sablonlar'), api('/api/yerlesikler')]);
      D.bilgi = bilgi;
      D.projeler = projeler.projeler || [];
      D.sonSablonlar = projeler.son_sablonlar || [];
      D.sablonlar = sablonlar.sablonlar || [];
      D.yerlesikler = yerlesikler.yerlesikler || [];
      D.konum = bilgi.varsayilan_konum;
      D.yuklendi = true;
    } catch (e) {
      $('#uygulama').innerHTML = `<div class="pencere"><div class="tam-ekran-mesaj"><div class="gokturk">${GOKTURK}</div><div>${kac(e.message)}</div><div>Terminalde <code>orhunca stüdyo</code> ile yeniden açın.</div></div></div>`;
      throw e;
    }
  }

  async function projeleriYenile() {
    const p = await api('/api/projeler');
    D.projeler = p.projeler || [];
    D.sonSablonlar = p.son_sablonlar || [];
  }

  async function mevcutAdlariYukle() {
    const r = await api('/api/klasor?' + sorgu({ yol: D.konum || D.bilgi.varsayilan_konum })).catch(() => ({}));
    D.mevcutAdlar = new Set((r.klasorler || []).map(k => k.ad));
    if (D.ekran === 'yapilandir') ciz();
  }

  async function klasorYukle(yol) {
    const m = D.modal;
    m.yukleniyor = true; m.hata = null; katmanlariCiz();
    const r = await api('/api/klasor?' + sorgu({ yol })).catch(e => ({ hata: e.message }));
    if (D.modal !== m) return;
    m.yukleniyor = false;
    if (r.hata) { m.hata = r.hata; katmanlariCiz(); return; }
    Object.assign(m, { yol: r.yol, ust: r.ust, klasorler: r.klasorler, proje: r.proje, secili: null });
    katmanlariCiz();
  }

  // =====================================================================
  // Eylemler
  // =====================================================================
  const EYLEM = {
    hic() { /* modalın içine tıklama örtüyü kapatmasın */ },
    git(ekran) {
      if (ekran === 'yapilandir') {
        if (sablon(D.secili).yakinda) return;
        mevcutAdlariYukle();
      }
      if (ekran === 'baslangic') projeleriYenile().then(() => D.ekran === 'baslangic' && ciz());
      D.ekran = ekran; D.menu = null; ciz();
    },
    siralamaDegistir() { D.siralama = D.siralama === 'tarih' ? 'ad' : 'tarih'; ciz(); },
    async projeAc(yol) {
      const p = D.projeler.find(x => x.yol === yol);
      if (p && !p.var) {
        await api('/api/proje/unut', { yol });
        await projeleriYenile();
        ciz();
        bildir(`“${p.ad}” bulunamadı; listeden kaldırıldı.`, true);
        return;
      }
      const r = await api('/api/proje/ac', { yol });
      if (r.hata) { bildir(r.hata, true); return; }
      projeyiAc(r);
    },
    sablonSec(kimlik) {
      const t = sablon(kimlik);
      if (t.yakinda) { bildir(`${t.ad} şablonu ${t.yakinda}'da gelecek.`); return; }
      if (!D.adDokunuldu) D.projeAdi = 'yeni_' + kimlik;
      D.secili = kimlik;
      if (D.ekran !== 'yeni') D.ekran = 'yeni';
      ciz();
    },
    sablonCift(kimlik) { if (!sablon(kimlik).yakinda) { EYLEM.sablonSec(kimlik); EYLEM.git('yapilandir'); } },
    kategoriSec(k) { D.kategori = k; ciz(); },
    secenekDegistir(a) { D.secenekler[a] = !D.secenekler[a]; ciz(); },
    async olustur() {
      if (adHatasi() || D.olusturuluyor) return;
      D.olusturuluyor = true; katmanlariCiz();
      const t0 = Date.now();
      const r = await api('/api/proje/olustur', { sablon: D.secili, ad: D.projeAdi.trim(), konum: D.konum || D.bilgi.varsayilan_konum, git: D.secenekler.git, ornek: D.secenekler.ornek }).catch(e => ({ hata: e.message }));
      await bekle(Math.max(0, 900 - (Date.now() - t0)));
      D.olusturuluyor = false;
      if (r.hata) { katmanlariCiz(); bildir(r.hata, true); return; }
      D.adDokunuldu = false;
      const web = sablon(D.secili).web;
      D.onizlemeAcik = !web || D.secenekler.canli;
      projeyiAc(r, { ilkCalistirma: web ? D.secenekler.canli : D.secenekler.calistir });
    },
    ilkProgram() { D.secili = 'konsol'; if (!D.adDokunuldu) D.projeAdi = 'ilk_programim'; EYLEM.git('yapilandir'); },

    // modallar
    modalKapat() { D.modal = null; katmanlariCiz(); },
    modalDis(_, el, e) { if (e.target === el) EYLEM.modalKapat(); },
    klasorModal(mod) {
      const yol = mod === 'konum' ? (D.konum || D.bilgi.varsayilan_konum) : (D.bilgi.varsayilan_konum || D.bilgi.ev);
      D.modal = { tur: 'klasor', mod, yol, klasorler: [] };
      klasorYukle(yol).then(() => { if (D.modal?.hata && D.modal.yol !== D.bilgi.ev) klasorYukle(D.bilgi.ev); });
    },
    klasorSec(i) { D.modal.secili = +i; katmanlariCiz(); },
    klasorGir(i) { klasorYukle(D.modal.klasorler[+i].yol); },
    klasorUst() { if (D.modal.ust) klasorYukle(D.modal.ust); },
    async klasorOnayla() {
      const m = D.modal;
      const yol = m.secili != null ? m.klasorler[m.secili].yol : m.yol;
      if (m.mod === 'konum') { D.konum = yol; D.modal = null; ciz(); mevcutAdlariYukle(); return; }
      const r = await api('/api/proje/ac', { yol });
      if (r.hata) { m.hata = r.hata; katmanlariCiz(); return; }
      D.modal = null;
      projeyiAc(r);
    },
    klonlaModal() { D.modal = { tur: 'klonla', url: '', konum: D.bilgi.varsayilan_konum }; katmanlariCiz(); $('#klonUrl')?.focus(); },
    async klonla() {
      const m = D.modal;
      if (m.calisiyor) return;
      m.calisiyor = true; m.hata = null; katmanlariCiz();
      const r = await api('/api/proje/klonla', { url: m.url, konum: m.konum }).catch(e => ({ hata: e.message }));
      m.calisiyor = false;
      if (r.hata) { m.hata = r.hata; katmanlariCiz(); return; }
      D.modal = null;
      projeyiAc(r);
    },
    ayarlarModal() { D.modal = { tur: 'ayarlar' }; katmanlariCiz(); },
    guncellemeModal() { D.menu = null; D.modal = { tur: 'guncelleme' }; guncelleVeyaCiz(); },
    kisayollarModal() { D.modal = { tur: 'kisayollar' }; guncelleVeyaCiz(); },
    hakkindaModal() { D.modal = { tur: 'hakkinda' }; guncelleVeyaCiz(); },
    yeniDosyaModal() { if (!D.proje) return; D.modal = { tur: 'yeniDosya', ad: '', klasor: false }; guncelleVeyaCiz(); $('#yeniDosyaAdi')?.focus(); },
    yeniKlasorModal() { if (!D.proje) return; D.modal = { tur: 'yeniDosya', ad: '', klasor: true }; guncelleVeyaCiz(); $('#yeniDosyaAdi')?.focus(); },
    async yeniDosyaOlustur() {
      const m = D.modal, ad = m.ad.trim().replace(/\\/g, '/').replace(/^\/+/, '');
      if (!ad || ad.split('/').includes('..')) { m.hata = 'Geçerli bir ad girin.'; katmanlariCiz(); return; }
      const r = await api('/api/dosya/yeni', { yol: tamYol(ad), klasor: m.klasor });
      if (r.hata) { m.hata = r.hata; katmanlariCiz(); return; }
      D.modal = null;
      await agaciYukle();
      if (!m.klasor) await dosyaAc(ad, false);
      guncelle('yan', 'sekmeler', 'kod', 'katman');
    },
    yaziBuyut() { D.yaziBoyutu = Math.min(20, D.yaziBoyutu + 1); ayarYaz('yaziBoyutu', D.yaziBoyutu); yaziDegisti(); },
    yaziKucult() { D.yaziBoyutu = Math.max(11, D.yaziBoyutu - 1); ayarYaz('yaziBoyutu', D.yaziBoyutu); yaziDegisti(); },
    acilisDegistir() { D.acilis = !D.acilis; ayarYaz('acilis', D.acilis); katmanlariCiz(); },
    yazarkenDenetleDegistir() { D.yazarkenDenetle = !D.yazarkenDenetle; ayarYaz('yazarkenDenetle', D.yazarkenDenetle); katmanlariCiz(); },

    // düzenleyici
    menuAc(ad, el, e) {
      if (e.target.closest('.acilir-menu')) return;
      D.menu = D.menu === ad ? null : ad; guncelle('baslik');
    },
    yanPanelSec(p) { D.yanPanel = p; guncelle('etkinlik', 'yan'); if (p === 'ara') $('#araMetin')?.focus(); if (p === 'eklentiler') paketleriYukle(); },
    paketleriYenile() { paketleriYukle(); },
    paketEkle() { if (D.paketKaynagi.trim()) paketIslemi('/api/paket/ekle', { kaynak: D.paketKaynagi.trim() }, `Paket ekleniyor: ${D.paketKaynagi.trim()}`); },
    paketYukle() { paketIslemi('/api/paket/yukle', { guncelle: false }, 'Paketler yükleniyor…'); },
    paketGuncelle() { paketIslemi('/api/paket/yukle', { guncelle: true }, 'Paketler güncelleniyor…'); },
    paketKaldir(ad) { if (confirm(`“${ad}” paketi kaldırılsın mı?`)) paketIslemi('/api/paket/kaldir', { ad }, `Paket kaldırılıyor: ${ad}`); },
    panelGezgin() { EYLEM.yanPanelSec('gezgin'); }, panelAra() { EYLEM.yanPanelSec('ara'); },
    panelYapi() { EYLEM.yanPanelSec('yapi'); }, panelCalistir() { EYLEM.yanPanelSec('calistir'); },
    klasorAcKapa(yol) { D.kapaliKlasorler.has(yol) ? D.kapaliKlasorler.delete(yol) : D.kapaliKlasorler.add(yol); cizYanPanel(); },
    dosyaAc(yol) { dosyaAc(yol); },
    async agaciYenile() { await agaciYukle(); cizYanPanel(); },
    sekmeSec(yol) { D.etkin = yol; guncelle('sekmeler', 'kod', 'yan', 'durum'); },
    sekmeKapat(yol, el, e) {
      e.stopPropagation();
      const s = D.sekmeler.find(x => x.yol === yol);
      if (s && !s.ikili && s.icerik !== s.kayitli && !confirm(`“${sonParca(yol)}” dosyasında kaydedilmemiş değişiklikler var. Yine de kapatılsın mı?`)) return;
      const i = D.sekmeler.indexOf(s);
      D.sekmeler.splice(i, 1);
      if (D.etkin === yol) D.etkin = (D.sekmeler[i] || D.sekmeler[i - 1])?.yol || null;
      guncelle('sekmeler', 'kod', 'yan');
    },
    satiraGit(n) { satiraGit(+n); },
    async konumaGit(a) { const [d, n] = a.split('|'); await dosyaAc(d); satiraGit(+n); },
    async sorunaGit(i) {
      const h = D.sorunlar[+i];
      if (!h?.dosya) return;
      await dosyaAc(goreliYol(h.dosya));
      satiraGit(h.satir, h.sutun);
    },
    altSekme(a) { D.altSekme = a; cizAltPanel(); },
    altSorunlar() { D.altPanel = true; D.altSekme = 'sorunlar'; guncelle('alt'); },
    altCikti() { D.altPanel = true; D.altSekme = 'cikti'; guncelle('alt'); },
    altPanelAcKapa() { D.altPanel = !D.altPanel; guncelle('alt'); },
    terminalTemizle() { if (D.altSekme === 'cikti') D.cikti = []; else D.terminal = []; guncelle('alt'); },
    calistir() { calistir(); },
    durdur() { durdur(); },
    onizlemeYenile() { onizlemeyiYenile(); },
    onizlemeTarayici() {
      const a = onizlemeAdresi();
      if (!a) { bildir('Önce projeyi çalıştırın (F5).'); return; }
      // Masaüstü uygulamasında sistem tarayıcısı Stüdyo sunucusu üzerinden açılır.
      if (TAURI) api('/api/tarayicida_ac', { adres: a }).catch(e => bildir(e.message, true));
      else window.open(a, '_blank', 'noopener');
    },
    onizlemeAcKapa() {
      if (!D.proje?.web) { bildir('Canlı önizleme web projelerinde kullanılır.'); return; }
      D.onizlemeAcik = !D.onizlemeAcik;
      D.menu = null;
      guncelle('onizleme', 'durum', 'baslik');
    },
    denetleKomut() { tumunuKaydet().then(denetle).then(() => { if (!D.sorunlar.length) bildir('Hata yok.'); else EYLEM.altSorunlar(); }); },
    derleLinux() { derle('linux'); },
    derleWindows() { derle('windows'); },
    derleWeb() { derle('web'); },
    ayikla() { calistir(true); },
    kesmeImlec() { kesmeDegistir(D.imlec.satir); },
    ayDevam() { ayiklamaKomutu('devam'); },
    ayAdim() { ayiklamaKomutu('adim'); },
    ayUstunden() { ayiklamaKomutu('ustunden'); },
    ayCik() { ayiklamaKomutu('cik'); },
    ayDuraklat() { ayiklamaKomutu('duraklat'); },
    cerceveSec(i) { cerceveSec(+i); },
    kesmeDegistir(n) { kesmeDegistir(+n); },
    paketleLinux() { derle('masaustu-linux'); },
    paketleWindows() { derle('masaustu-windows'); },
    kaydet() { kaydet(); },
    tumunuKaydet() { tumunuKaydet({ yenile: true }); },
    async baslangicaDon() {
      await projeleriYenile();
      D.ekran = 'baslangic'; D.menu = null; iskeletVar = false; ciz();
    },
    async projeyiKapat() {
      if (D.sekmeler.some(s => !s.ikili && s.icerik !== s.kayitli) && !confirm('Kaydedilmemiş değişiklikler var. Proje kapatılsın mı?')) return;
      if (D.calisma) await durdur();
      D.proje = null; D.calisma = null;
      EYLEM.baslangicaDon();
    },
    async studyoyuKapat() {
      if (D.sekmeler.some(s => !s.ikili && s.icerik !== s.kayitli) && !confirm('Kaydedilmemiş değişiklikler var. Stüdyo kapatılsın mı?')) return;
      await api('/api/kapat', {}).catch(() => null);
      $('#uygulama').innerHTML = `<div class="pencere"><div class="tam-ekran-mesaj"><div class="gokturk">${GOKTURK}</div><div>Orhunca Stüdyo kapatıldı. Bu sekmeyi kapatabilirsiniz.</div></div></div>`;
    },
    ogrenAc() { iskeletVar = false; D.menu = null; D.ekran = 'ogren'; ciz(); },
    geriAl() { $('#kodAlani')?.focus(); document.execCommand('undo'); },
    yinele() { $('#kodAlani')?.focus(); document.execCommand('redo'); },
    kes() { $('#kodAlani')?.focus(); document.execCommand('cut'); },
    kopyala() { $('#kodAlani')?.focus(); document.execCommand('copy'); },
    async yapistir() { const ta = $('#kodAlani'); if (!ta) return; ta.focus(); try { metinEkle(ta, await navigator.clipboard.readText()); } catch { bildir('Pano okunamadı; Ctrl+V kullanın.', true); } },
    yorumYap() { yorumYap(); },
    async bicimlendir() {
      const ta = $('#kodAlani'), s = etkinSekme();
      if (!ta || !s || uzanti(s.yol) !== 'ohc') return;
      const r = await api('/api/bicimlendir', { icerik: ta.value });
      if (r.icerik === undefined || r.icerik === ta.value) { bildir('Dosya zaten düzgün.'); return; }
      const konum = ta.selectionStart;
      ta.focus();
      ta.select();
      metinEkle(ta, r.icerik);
      const yeni = Math.min(konum, r.icerik.length);
      ta.setSelectionRange(yeni, yeni);
      imleciGuncelle();
      bildir('Dosya biçimlendirildi.');
    },
    async uyariyaGit(i) {
      const h = D.uyarilar[+i];
      if (!h) return;
      await dosyaAc(goreliYol(h.dosya));
      satiraGit(h.satir, h.sutun);
    },
    async uyariDuzelt(i, el, e) {
      e.stopPropagation();
      const h = D.uyarilar[+i];
      if (!h) return;
      await dosyaAc(goreliYol(h.dosya));
      const ta = $('#kodAlani');
      if (!ta) return;
      const satirlar = ta.value.split('\n');
      let konum = 0;
      for (let k = 0; k < h.satir - 1; k++) konum += satirlar[k].length + 1;
      konum += h.sutun - 1;
      ta.focus();
      ta.setSelectionRange(konum, konum + h.uzunluk);
      metinEkle(ta, h.duzeltme);
    },
    tumunuSec() { const ta = $('#kodAlani'); if (ta) { ta.focus(); ta.select(); } },
    satiriSec() { const ta = $('#kodAlani'); if (!ta) return; ta.focus(); ta.dispatchEvent(new KeyboardEvent('keydown', { key: 'l', ctrlKey: true })); },
    satiriCogalt() { satiriCogalt(); },
    pencereKucult() { try { window.__TAURI__.window.getCurrentWindow().minimize(); } catch { /* yok */ } },
    pencereBuyut() { try { window.__TAURI__.window.getCurrentWindow().toggleMaximize(); } catch { /* yok */ } },
    pencereKapat() { try { window.__TAURI__.window.getCurrentWindow().close(); } catch { /* yok */ } },
  };

  async function paketleriYukle() {
    if (!D.proje) return;
    const r = await api('/api/paket/liste?' + sorgu({ kok: D.proje.yol })).catch(e => ({ hata: e.message }));
    D.paketler = r.paketler || [];
    if (D.yanPanel === 'eklentiler') cizYanPanel();
  }

  async function paketIslemi(yol, govde, baslik) {
    if (D.paketMesgul) return;
    D.paketMesgul = true; cizYanPanel();
    D.altPanel = true; D.altSekme = 'cikti';
    D.cikti.push({ t: baslik, c: 'mut' });
    guncelle('alt');
    const r = await api(yol, { kok: D.proje.yol, ...govde }).catch(e => ({ hata: e.message }));
    D.paketMesgul = false;
    if (r.hata) { D.cikti.push({ t: r.hata, c: 'err' }); bildir('Paket işlemi başarısız.', true); }
    else {
      for (const s of r.gunluk || []) D.cikti.push({ t: s, c: s.startsWith('✓') ? 'ok' : s.startsWith('uyarı') ? 'err' : '' });
      if (yol.endsWith('ekle')) D.paketKaynagi = '';
      bildir('Paketler güncellendi.');
    }
    await agaciYukle();
    if (D.agac.some(g => g.yol === 'paketler')) D.kapaliKlasorler.add('paketler');
    await paketleriYukle();
    guncelle('alt', 'yan');
    denetle();
  }

  function guncelleVeyaCiz() { D.menu = null; if (D.ekran === 'duzenleyici') guncelle('baslik', 'katman'); else ciz(); }

  function yaziDegisti() {
    document.documentElement.style.setProperty('--kod-boyut', D.yaziBoyutu + 'px');
    katmanlariCiz();
    if (D.ekran === 'duzenleyici') cizKod();
  }

  function satiraGit(satir, sutun = 1) {
    const ta = $('#kodAlani');
    if (!ta) return;
    const satirlar = ta.value.split('\n');
    let konum = 0;
    for (let i = 0; i < Math.min(satir - 1, satirlar.length); i++) konum += satirlar[i].length + 1;
    konum += Math.max(0, sutun - 1);
    ta.focus();
    ta.setSelectionRange(konum, konum);
    imleciGuncelle();
    const kap = $('#kodKap');
    kap.scrollTop = Math.max(0, 4 + (satir - 1) * 21 - kap.clientHeight / 3);
  }

  const GIRDI = {
    q(v) { D.q = v; ciz(); },
    tq(v) { D.tq = v; ciz(); },
    projeAdi(v) { D.projeAdi = v; D.adDokunuldu = true; ciz(); },
    konum(v) { D.konum = v; ciz(); clearTimeout(GIRDI._k); GIRDI._k = setTimeout(mevcutAdlariYukle, 300); },
    klasorYolu(v) { D.modal.yol = v; },
    klonUrl(v) { D.modal.url = v; },
    klonKonum(v) { D.modal.konum = v; },
    yeniDosyaAdi(v) { D.modal.ad = v; },
    argumanlar(v) { D.argumanlar = v; },
    paketKaynagi(v) { D.paketKaynagi = v; },
    araMetin(v) {
      D.araMetin = v;
      clearTimeout(GIRDI._a);
      GIRDI._a = setTimeout(async () => {
        if (!v.trim()) { D.araSonuc = []; cizYanPanel(); return; }
        const r = await api('/api/ara?' + sorgu({ kok: D.proje.yol, metin: v }));
        if (D.araMetin === v) { D.araSonuc = r.sonuclar || []; cizYanPanel(); }
      }, 200);
    },
  };

  // =====================================================================
  // Olay bağlama
  // =====================================================================
  document.addEventListener('click', e => {
    if (D.menu && !e.target.closest('.menu-baslik')) { D.menu = null; guncelle('baslik'); }
    const el = e.target.closest('[data-e]');
    if (!el) return;
    const ad = el.dataset.e;
    if (D.menu && el.closest('.acilir-menu')) { D.menu = null; guncelle('baslik'); }
    EYLEM[ad]?.(el.dataset.a, el, e);
  });
  document.addEventListener('dblclick', e => {
    const el = e.target.closest('[data-ee]');
    if (el) EYLEM[el.dataset.ee]?.(el.dataset.a, el, e);
  });
  document.addEventListener('mouseover', e => {
    // Bir menü açıkken diğer menü başlığına gelince o menü açılır.
    if (!D.menu) return;
    const el = e.target.closest('.menu-baslik');
    if (el && el.dataset.a !== D.menu) { D.menu = el.dataset.a; guncelle('baslik'); }
  });
  document.addEventListener('input', e => {
    const g = e.target.dataset?.g;
    if (g) GIRDI[g]?.(e.target.value, e.target);
  });
  document.addEventListener('keydown', async e => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (e.target.id === 'terminalGirdi' && e.key === 'Enter' && D.calisma) {
      const metin = e.target.value;
      e.target.value = '';
      terminaleEkle(metin + '\n', 'girdi-yanki');
      cizAltPanel();
      $('#terminalGirdi')?.focus();
      const r = await api('/api/girdi', { kimlik: D.calisma.kimlik, metin: metin + '\n' }).catch(x => ({ hata: x.message }));
      if (r.hata) bildir(r.hata, true);
      return;
    }
    if (e.target.id === 'klasorYolu' && e.key === 'Enter') { klasorYukle(e.target.value); return; }
    if (e.target.id === 'klonUrl' && e.key === 'Enter') { EYLEM.klonla(); return; }
    if (e.target.id === 'paketKaynagi' && e.key === 'Enter') { EYLEM.paketEkle(); return; }
    if (e.target.id === 'yeniDosyaAdi' && e.key === 'Enter') { EYLEM.yeniDosyaOlustur(); return; }
    if (e.target.id === 'projeAdi' && e.key === 'Enter') { EYLEM.olustur(); return; }
    if (e.key === 'Escape') {
      if (D.modal) { EYLEM.modalKapat(); return; }
      if (D.menu) { D.menu = null; guncelle('baslik'); return; }
    }
    if (D.ekran === 'duzenleyici') {
      const durdu = D.calisma?.ay?.durdu;
      if (e.key === 'F5') { e.preventDefault(); e.shiftKey ? durdur() : durdu ? ayiklamaKomutu('devam') : calistir(); return; }
      if (e.key === 'F6') { e.preventDefault(); calistir(true); return; }
      if (e.key === 'F9') { e.preventDefault(); kesmeDegistir(D.imlec.satir); return; }
      if (e.key === 'F10' && durdu) { e.preventDefault(); ayiklamaKomutu('ustunden'); return; }
      if (e.key === 'F11' && durdu) { e.preventDefault(); ayiklamaKomutu(e.shiftKey ? 'cik' : 'adim'); return; }
      if (e.key === 'F7') { e.preventDefault(); EYLEM.denetleKomut(); return; }
      if (ctrl && e.altKey && (e.key === 's' || e.key === 'S')) { e.preventDefault(); tumunuKaydet({ yenile: true }); return; }
      if (ctrl && (e.key === 's' || e.key === 'S')) { e.preventDefault(); kaydet(); return; }
      if (ctrl && (e.key === 'j' || e.key === 'J')) { e.preventDefault(); EYLEM.altPanelAcKapa(); return; }
      if (ctrl && e.shiftKey && (e.key === 'F' || e.key === 'f')) { e.preventDefault(); EYLEM.bicimlendir(); return; }
      if (ctrl && (e.key === '=' || e.key === '+')) { e.preventDefault(); EYLEM.yaziBuyut(); return; }
      if (ctrl && e.key === '-') { e.preventDefault(); EYLEM.yaziKucult(); return; }
    } else {
      if (e.altKey && (e.key === 's' || e.key === 'S' || e.code === 'KeyS')) { e.preventDefault(); if (D.ekran !== 'baslangic') EYLEM.git('baslangic'); setTimeout(() => $('#q')?.focus()); return; }
      if (ctrl && e.shiftKey && (e.key === 'N' || e.key === 'n')) { e.preventDefault(); EYLEM.git('yeni'); }
    }
  });
  window.addEventListener('resize', () => { if (D.ekran === 'duzenleyici') vurguyuGuncelle(); });
  window.addEventListener('beforeunload', e => {
    if (D.sekmeler.some(s => !s.ikili && s.icerik !== s.kayitli)) { e.preventDefault(); e.returnValue = ''; }
  });

  // =====================================================================
  // Başlat
  // =====================================================================
  ciz();
  if (ANAHTAR) verileriYukle().then(async () => {
    D.projeAdi = 'yeni_' + D.secili;
    ciz();
    if (ACILACAK) {
      const r = await api('/api/proje/ac', { yol: ACILACAK }).catch(e => ({ hata: e.message }));
      if (r.hata) { bildir(r.hata, true); return; }
      await projeyiAc(r);
      if (/\.(ohc|ohchtml)$/i.test(ACILACAK)) { await dosyaAc(goreliYol(ACILACAK)); }
    }
  });

  // Testler ve Tauri için küçük bir kapı
  window.orhuncaStudyo = { durum: D, eylem: EYLEM };
})();
