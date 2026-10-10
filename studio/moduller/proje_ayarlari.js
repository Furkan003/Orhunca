/* Orhunca Stüdyo — Proje Özellikleri, çalıştırma ayarları, düzenleyici ayarları,
 * kaydederken biçimlendirme ve Git dalları / geçmiş / satır geçmişi (blame).
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Düzenleyici ayarları
  // =====================================================================
  D.girinti = ayarOku('girinti', 4);
  D.kodYazi = ayarOku('kodYazi', '');
  D.kaydedinceBicimlendir = ayarOku('kaydedinceBicimlendir', false);
  D.satirGecmisiGoster = ayarOku('satirGecmisiGoster', true);
  const YAZI_TIPLERI = ['', 'Cascadia Code', 'Consolas', 'JetBrains Mono', 'Fira Code', 'Source Code Pro', 'Courier New'];
  /** Orhunca dosyalarında girinti her zaman 4 boşluktur (biçimlendirici de öyle yazar). */
  function girintiMetni() {
    const s = etkinSekme();
    return ' '.repeat(!s || s.yol.endsWith('.ohc') ? 4 : D.girinti);
  }
  function yaziTipiniUygula() {
    document.documentElement.style.setProperty('--kod-yazi', D.kodYazi ? `'${D.kodYazi}', var(--mono)` : 'var(--mono)');
  }
  yaziTipiniUygula();

  function duzenleyiciAyarlari() {
    return `<div class="ayar-bolum">Düzenleyici</div>
      <div class="secenek" style="cursor:default"><div class="esnek"><div class="secenek-ad">Girinti genişliği</div><div class="secenek-alt">CSS, JavaScript, HTML ve JSON dosyaları için. Orhunca dosyalarında her zaman 4 boşluk.</div></div>
        <div class="tema-secim">${[2, 4, 8].map(n => `<span class="${D.girinti === n ? 'secili' : ''}" data-e="girintiSec" data-a="${n}">${n}</span>`).join('')}</div></div>
      <div class="secenek" style="cursor:default"><div class="esnek"><div class="secenek-ad">Kod yazı tipi</div><div class="secenek-alt">Bilgisayarınızda kurulu değilse varsayılan kullanılır.</div></div>
        <select class="metin-girdi" data-g="kodYaziSec" style="width:180px">${YAZI_TIPLERI.map(y => `<option value="${kac(y)}" ${D.kodYazi === y ? 'selected' : ''}>${y ? kac(y) : 'Varsayılan'}</option>`).join('')}</select></div>
      <div class="secenek" data-e="kaydedinceBicimlendirDegistir"><div class="esnek"><div class="secenek-ad">Kaydederken biçimlendir</div><div class="secenek-alt">Ctrl+S ile kaydederken Orhunca dosyası düzenlenir (girinti, boşluklar).</div></div><div class="anahtar ${D.kaydedinceBicimlendir ? 'acik' : ''}"><div></div></div></div>
      <div class="secenek" data-e="satirGecmisiDegistir"><div class="esnek"><div class="secenek-ad">Satırı kimin değiştirdiğini göster</div><div class="secenek-alt">Git deposunda imlecin bulunduğu satırın sonunda son değiştiren kişi ve tarih soluk yazılır.</div></div><div class="anahtar ${D.satirGecmisiGoster ? 'acik' : ''}"><div></div></div></div>`;
  }

  // Kaydederken biçimlendirme: elle kaydetmede (otomatik kaydetmede değil)
  const bicimsizKaydet = kaydet;
  kaydet = async function (s = etkinSekme(), sessiz = false, otomatik = false) {
    if (D.kaydedinceBicimlendir && !otomatik && s && !s.ikili && s.yol.endsWith('.ohc')) {
      const r = await api('/api/bicimlendir', { icerik: s.icerik }).catch(() => ({}));
      if (typeof r.icerik === 'string' && r.icerik !== s.icerik) {
        const ta = $('#kodAlani');
        if (ta && D.etkin === s.yol) {
          const konum = Math.min(ta.selectionStart, r.icerik.length);
          ta.focus(); ta.select(); metinEkle(ta, r.icerik); ta.setSelectionRange(konum, konum); imleciGuncelle();
        } else s.icerik = r.icerik;
      }
    }
    return bicimsizKaydet(s, sessiz, otomatik);
  };

  // =====================================================================
  // Proje Özellikleri (Visual Studio: Project → Properties)
  // =====================================================================
  const PROJE_ALANLARI = [['ad', 'Ad'], ['sürüm', 'Sürüm'], ['açıklama', 'Açıklama'], ['giriş', 'Giriş dosyası'], ['paket_kimliği', 'Paket kimliği (Android/iOS)'], ['simge', 'Simge (PNG)']];
  const projeDosyasi = () => D.agac.find(g => !g.klasor && !g.yol.includes('/') && g.yol.endsWith('.ohcproj'))?.yol;
  /** `.ohcproj` dosyasının en üst bölümündeki `anahtar = "değer"` satırları */
  function projeAyarlariniOku(metin) {
    const a = {};
    for (const l of metin.split('\n')) {
      if (/^\s*\[/.test(l)) break;
      const m = l.match(/^\s*([^\s=#]+)\s*=\s*"((?:[^"\\]|\\.)*)"/u);
      if (m) a[m[1]] = m[2].replace(/\\"/g, '"');
    }
    return a;
  }
  function projeAyarlariniYaz(metin, ayarlar) {
    let satirlar = metin.split('\n');
    const bolum = satirlar.findIndex(l => /^\s*\[/.test(l));
    const ust = bolum < 0 ? satirlar.length : bolum;
    for (const [k, v] of Object.entries(ayarlar)) {
      const i = satirlar.slice(0, ust).findIndex(l => new RegExp(`^\\s*${k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*=`, 'u').test(l));
      const yeni = `${k} = "${v.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
      if (i >= 0) { if (v) satirlar[i] = yeni; else satirlar.splice(i, 1); }
      else if (v) { const yer = satirlar.slice(0, ust).reduce((s, l, j) => l.trim() ? j + 1 : s, 0); satirlar.splice(yer, 0, yeni); }
    }
    return satirlar.join('\n').replace(/\n*$/, '\n');
  }
  function envOku(metin, ad) {
    for (const l of metin.split('\n')) {
      const m = l.match(new RegExp(`^\\s*(?:export\\s+)?${ad}\\s*=\\s*(.*)$`));
      if (m) return m[1].split(' #')[0].trim().replace(/^["']|["']$/g, '');
    }
    return '';
  }
  function envYaz(metin, ad, deger) {
    const satirlar = metin ? metin.replace(/\n$/, '').split('\n') : [];
    const i = satirlar.findIndex(l => new RegExp(`^\\s*(?:export\\s+)?${ad}\\s*=`).test(l));
    if (i >= 0) { if (deger) satirlar[i] = `${ad}=${deger}`; else satirlar.splice(i, 1); }
    else if (deger) satirlar.push(`${ad}=${deger}`);
    return satirlar.join('\n') + '\n';
  }
  const calismaAnahtari = () => 'calistirma:' + D.proje.yol;
  function calismaAyarlari() { return ayarOku(calismaAnahtari(), { argumanlar: '', ortam: '' }); }
  /** "AD=değer" satırları → { AD: 'değer' } */
  function ortamNesnesi(metin) {
    const o = {};
    for (const l of (metin || '').split('\n')) { const m = l.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/); if (m) o[m[1]] = m[2].trim(); }
    return o;
  }

  async function projeOzellikleri(sekme = 'genel') {
    if (!D.proje) return;
    D.menu = null;
    const pd = projeDosyasi();
    const oku = async y => { const r = await api('/api/dosya?' + sorgu({ yol: tamYol(y) })).catch(() => ({})); return r.icerik ?? ''; };
    const metin = pd ? await oku(pd) : '';
    const env = await oku('.env');
    const c = calismaAyarlari();
    D.modal = {
      tur: 'projeOzellikleri', sekme, pd, metin, env,
      ayar: projeAyarlariniOku(metin),
      vt: envOku(env, 'ORHUNCA_VERITABANI'),
      argumanlar: D.argumanlar || c.argumanlar, ortam: c.ortam,
    };
    katmanlariCiz();
  }
  function cizProjeOzellikleri(m, kabuk) {
    const ohc = D.agac.filter(g => !g.klasor && g.yol.endsWith('.ohc')).map(g => g.yol);
    const sekme = (k, ad) => `<span class="${m.sekme === k ? 'secili' : ''}" data-e="ozellikSekme" data-a="${k}">${ad}</span>`;
    const alan = (k, ad) => k === 'giriş'
      ? `<div class="alan"><label>${ad}</label><select class="metin-girdi" data-g="ozellikAlan" data-k="${k}"><option value="">(otomatik: ana.ohc / sunucu.ohc)</option>${ohc.map(y => `<option ${m.ayar[k] === y ? 'selected' : ''}>${kac(y)}</option>`).join('')}</select></div>`
      : `<div class="alan"><label>${ad}</label><input class="metin-girdi" data-g="ozellikAlan" data-k="${k}" value="${kac(m.ayar[k] || '')}" spellcheck="false"></div>`;
    const govde = m.sekme === 'genel'
      ? (m.pd ? `<div class="panel-not">Proje dosyası: <span class="mono">${kac(m.pd)}</span></div>${PROJE_ALANLARI.map(([k, ad]) => alan(k, ad)).join('')}`
        : '<div class="panel-not">Bu klasörde .ohcproj proje dosyası yok. Kaydedince oluşturulur.</div>' + PROJE_ALANLARI.map(([k, ad]) => alan(k, ad)).join(''))
      : m.sekme === 'vt'
        ? `<div class="panel-not">Veritabanı adresi .env dosyasındaki ORHUNCA_VERITABANI satırına yazılır. Boş bırakılırsa veriler JSON dosyalarında (veri/) saklanır.</div>
          <div class="alan"><label>Tür</label><div class="tema-secim">${[['', 'JSON'], ['sqlite', 'SQLite'], ['postgresql://', 'PostgreSQL'], ['mysql://', 'MySQL'], ['sqlserver://', 'SQL Server']].map(([v, ad]) => `<span class="${(m.vt || '').startsWith(v) && (v || !m.vt) ? 'secili' : ''}" data-e="ozellikVtTur" data-a="${v}">${ad}</span>`).join('')}</div></div>
          <div class="alan"><label>Adres</label><input class="metin-girdi mono" data-g="ozellikVt" value="${kac(m.vt)}" placeholder="ör. postgresql://kullanici:sifre@localhost/veritabani" spellcheck="false"></div>`
        : `<div class="panel-not">Bu ayarlar yalnız Stüdyo'dan çalıştırırken kullanılır ve bu bilgisayarda saklanır.</div>
          <div class="alan"><label>Program argümanları</label><input class="metin-girdi mono" data-g="ozellikArg" value="${kac(m.argumanlar)}" placeholder='ör. bir "iki kelime"' spellcheck="false"></div>
          <div class="alan"><label>Ortam değişkenleri (her satıra AD=değer)</label><textarea class="metin-girdi mono" rows="5" data-g="ozellikOrtam" placeholder="API_ANAHTARI=deneme&#10;DIL=tr" spellcheck="false">${kac(m.ortam)}</textarea></div>`;
    return kabuk('Proje özellikleri', `<div class="tema-secim ozellik-sekmeler">${sekme('genel', 'Genel')}${sekme('calistir', 'Çalıştırma')}${sekme('vt', 'Veritabanı')}</div>${govde}${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}`,
      `<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="ozellikKaydet">Kaydet</div>`);
  }
  async function ozellikleriKaydet() {
    const m = D.modal;
    const yaz = async (y, icerik) => { const r = await api('/api/dosya', { yol: tamYol(y), icerik }).catch(e => ({ hata: e.message })); if (r.hata) throw new Error(r.hata); };
    try {
      const pd = m.pd || `${D.proje.ad}.ohcproj`;
      const yeniProje = projeAyarlariniYaz(m.metin, m.ayar);
      if (yeniProje !== m.metin && (m.pd || Object.values(m.ayar).some(Boolean))) await yaz(pd, yeniProje);
      const yeniEnv = envYaz(m.env, 'ORHUNCA_VERITABANI', m.vt.trim());
      if (yeniEnv.trim() !== m.env.trim()) await yaz('.env', yeniEnv);
      D.argumanlar = m.argumanlar;
      ayarYaz(calismaAnahtari(), { argumanlar: m.argumanlar, ortam: m.ortam });
      // Açık sekmeler diskteki yeni hâli gösterir.
      for (const s of D.sekmeler) if ((s.yol === pd || s.yol === '.env') && s.icerik === s.kayitli) { const d = await api('/api/dosya?' + sorgu({ yol: tamYol(s.yol) })); if (!d.hata) { s.icerik = s.kayitli = d.icerik; s.zaman = d.zaman; } }
      D.modal = null;
      await agaciYukle();
      if (m.pd || Object.values(m.ayar).some(Boolean)) D.proje.giris = m.ayar['giriş'] || null;
      guncelle('yan', 'kod', 'sekmeler', 'katman', 'durum');
      bildir('Proje özellikleri kaydedildi.');
    } catch (e) { m.hata = e.message; katmanlariCiz(); }
  }

  // =====================================================================
  // Git: dallar, geçmiş, satır geçmişi
  // =====================================================================
  async function dalMenusu(e) {
    const r = await api('/api/git/dallar?' + sorgu({ kok: D.proje.yol })).catch(x => ({ hata: x.message }));
    if (r.hata) return bildir(r.hata, true);
    const islem = async (islem, ad) => {
      const s = await api('/api/git/dal', { kok: D.proje.yol, islem, ad }).catch(x => ({ hata: x.message }));
      if (s.hata) return bildir(s.hata, true);
      bildir(s.mesaj);
      if (islem !== 'sil') await diskiYenile();
      gitYukle();
    };
    const sor = async (baslik, varsayilan = '') => (await metinSor(baslik, varsayilan, { baslik: 'Git dalı', dugme: 'Tamam' }))?.trim();
    sagMenuAc(e, [
      ['Yeni dal…', async () => { const ad = await sor('Yeni dalın adı (etkin daldan oluşturulup ona geçilir)'); if (ad) islem('olustur', ad); }],
      '-',
      ...r.dallar.map(d => [`${d === r.etkin ? '● ' : '   '}${d}`, () => d !== r.etkin && islem('gec', d)]),
      ...(r.uzak.length ? ['-', ...r.uzak.filter(u => !r.dallar.includes(u.slice(u.indexOf('/') + 1))).map(u => [`   ${u}`, () => islem('gec', u)])] : []),
      '-',
      ['Bir dalı bu dala birleştir…', async () => { const ad = await sor(`Hangi dal “${r.etkin}” dalına birleştirilsin?`); if (ad) islem('birlestir', ad); }],
      ['Dal sil…', async () => { const ad = await sor('Silinecek dal (birleştirilmemiş dallar silinmez)'); if (ad && ad !== r.etkin) islem('sil', ad); }],
    ]);
  }
  /** Dal değişince açık dosyalar diskten yeniden okunur (kaydedilmemiş olanlar korunur). */
  async function diskiYenile() {
    await agaciYukle();
    for (const s of D.sekmeler) {
      if (s.ikili || s.icerik !== s.kayitli) continue;
      const d = await api('/api/dosya?' + sorgu({ yol: tamYol(s.yol) })).catch(() => ({ hata: 1 }));
      if (!d.hata && !d.ikili) { s.icerik = s.kayitli = d.icerik; s.zaman = d.zaman; }
    }
    D.sekmeler = D.sekmeler.filter(s => s.icerik !== s.kayitli || D.agac.some(g => g.yol === s.yol));
    if (D.etkin && !D.sekmeler.some(s => s.yol === D.etkin)) D.etkin = D.sekmeler[0]?.yol || null;
    satirGecmisi.clear();
    guncelle('yan', 'sekmeler', 'kod', 'durum');
  }
  async function islemeGoster(kisa, baslik) {
    D.modal = { tur: 'fark', yol: baslik || kisa, isleme: true, metin: null };
    katmanlariCiz();
    const r = await api('/api/git/isleme?' + sorgu({ kok: D.proje.yol, kimlik: kisa })).catch(e => ({ hata: e.message }));
    if (D.modal?.tur !== 'fark') return;
    if (r.hata) { D.modal = null; katmanlariCiz(); return bildir(r.hata, true); }
    D.modal.metin = r.fark; katmanlariCiz();
  }
  document.addEventListener('click', e => {
    const dal = e.target.closest('.git-dal');
    if (dal) { dalMenusu(e); return; }
    const isl = e.target.closest('.git-islem');
    if (isl) { const k = isl.querySelector('.mono')?.textContent; if (k) islemeGoster(k, isl.textContent.trim()); }
  });

  // Satır geçmişi (blame): imlecin satırının sonunda soluk yazı
  const satirGecmisi = new Map();
  let blameZamanlayici = null;
  async function satirGecmisiniGoster() {
    $('#satirBilgisi')?.remove();
    const s = etkinSekme();
    if (!D.satirGecmisiGoster || !s || s.ikili || !D.git?.depo || s.icerik !== s.kayitli) return;
    if (!satirGecmisi.has(s.yol)) {
      satirGecmisi.set(s.yol, null);
      const r = await api('/api/git/satirlar?' + sorgu({ kok: D.proje.yol, yol: s.yol })).catch(() => ({}));
      satirGecmisi.set(s.yol, r.satirlar || []);
    }
    const l = satirGecmisi.get(s.yol), b = l?.[D.imlec.satir - 1];
    const ic = $('#kodIc');
    if (!b || !ic || etkinSekme() !== s) return;
    const metin = s.icerik.split('\n')[D.imlec.satir - 1] || '';
    const d = document.createElement('div');
    d.id = 'satirBilgisi';
    d.className = 'satir-bilgisi';
    const yeni = /^0+$/.test(b.kisa);
    d.textContent = yeni ? 'Henüz işlenmedi' : `${b.yazar}, ${new Date(b.zaman * 1000).toLocaleDateString('tr-TR')} · ${b.ozet}`;
    d.title = yeni ? '' : `${b.kisa} — tıklayınca işlemenin değişikliklerini gösterir`;
    d.style.top = (4 + (D.imlec.satir - 1) * 21) + 'px';
    d.style.left = (56 + (metin.length + 4) * karakterGenisligi) + 'px';
    if (!yeni) d.addEventListener('mousedown', ev => { ev.preventDefault(); islemeGoster(b.kisa, `${b.kisa} ${b.ozet}`); });
    ic.appendChild(d);
  }
  function satirGecmisiPlanla() { clearTimeout(blameZamanlayici); blameZamanlayici = setTimeout(satirGecmisiniGoster, 250); }
  ['keyup', 'click'].forEach(o => document.addEventListener(o, e => { if (e.target.id === 'kodAlani') satirGecmisiPlanla(); }));
  document.addEventListener('input', e => { if (e.target.id === 'kodAlani') $('#satirBilgisi')?.remove(); });
  const blamesizKaydet = kaydet;
  kaydet = async function (...a) { const r = await blamesizKaydet(...a); satirGecmisi.delete(a[0]?.yol || D.etkin); return r; };

  Object.assign(EYLEM, {
    projeOzellikleri() { projeOzellikleri(); },
    calistirmaAyarlari() { projeOzellikleri('calistir'); },
    ozellikSekme(k) { D.modal.sekme = k; D.modal.hata = null; katmanlariCiz(); },
    ozellikVtTur(v) { D.modal.vt = v === 'sqlite' ? 'sqlite' : v; katmanlariCiz(); },
    ozellikKaydet: ozellikleriKaydet,
    girintiSec(n) { D.girinti = +n; ayarYaz('girinti', D.girinti); katmanlariCiz(); },
    kaydedinceBicimlendirDegistir() { D.kaydedinceBicimlendir = !D.kaydedinceBicimlendir; ayarYaz('kaydedinceBicimlendir', D.kaydedinceBicimlendir); katmanlariCiz(); },
    satirGecmisiDegistir() { D.satirGecmisiGoster = !D.satirGecmisiGoster; ayarYaz('satirGecmisiGoster', D.satirGecmisiGoster); katmanlariCiz(); if (!D.satirGecmisiGoster) $('#satirBilgisi')?.remove(); },
    gitDallar(a, el, e) { dalMenusu(e); },
  });
  Object.assign(GIRDI, {
    ozellikAlan(v, el) { D.modal.ayar[el.dataset.k] = v; },
    ozellikVt(v) { D.modal.vt = v; },
    ozellikArg(v) { D.modal.argumanlar = v; },
    ozellikOrtam(v) { D.modal.ortam = v; },
    kodYaziSec(v) { D.kodYazi = v; ayarYaz('kodYazi', v); yaziTipiniUygula(); if (D.ekran === 'duzenleyici') guncelle('kod'); },
  });
  // <select> değişimi 'input' olayı da üretir; ek bir şey gerekmez.
