/* Orhunca Stüdyo — Yan paneller: Git, sınamalar, dersler, ayıklama.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  const GIT_DURUM = { M: ['D', 'değişti'], A: ['E', 'eklendi'], D: ['S', 'silindi'], R: ['A', 'yeniden adlandırıldı'], '?': ['Y', 'yeni (takip edilmiyor)'], U: ['Ç', 'çakışma'], C: ['K', 'kopyalandı'] };

  /** Git paneli: değişiklikler, hazırlama, işleme (commit), gönderme/çekme, son işlemeler. */
  function gitPaneli() {
    const G = D.git;
    if (!G) { gitYukle(); return '<div class="panel-not">Yükleniyor…</div>'; }
    if (G.guvensiz) return `<div class="panel-not">Kısıtlı modda Git paneli kapalı: Git, deponun kendi ayarlarındaki komutları çalıştırabilir.</div><div class="dugme birincil" style="margin-top:10px" data-e="projeyeGuven">Projeye güven</div>`;
    if (G.hata) return `<div class="panel-not">${kac(G.hata)}</div>`;
    if (!G.depo) return `<div class="panel-not">Bu proje bir Git deposu değil. Git, kodunuzun her hâlini saklar ve GitHub'a göndermenizi sağlar.</div><div class="dugme birincil" style="margin-top:10px" data-e="gitBaslat">${S('add')}Depo başlat</div>`;
    const hazir = G.degisiklikler.filter(d => d.hazir), bekleyen = G.degisiklikler.filter(d => !d.hazir);
    const oge = (d) => {
      const [harf, ad] = GIT_DURUM[d.durum] || [d.durum, d.durum];
      const eylem = d.hazir
        ? `<span class="simge" title="Hazırlıktan çıkar" data-e="gitHazirla" data-a="-${kac(d.yol)}">remove</span>`
        : `<span class="simge" title="Değişikliği at" data-e="gitAt" data-a="${d.durum === '?' ? '?' : ''}${kac(d.yol)}">restore</span><span class="simge" title="Hazırla" data-e="gitHazirla" data-a="+${kac(d.yol)}">add</span>`;
      return `<div class="git-oge" data-e="gitFark" data-a="${d.hazir ? 1 : 0}${kac(d.yol)}" title="${kac(d.yol)} · ${ad}"><span class="esnek">${kac(sonParca(d.yol))}<span class="git-klasor">${kac(d.yol.includes('/') ? d.yol.slice(0, d.yol.lastIndexOf('/')) : '')}</span></span><span class="git-eylem">${eylem}</span><span class="git-harf g-${d.durum === '?' ? 'Y' : d.durum}">${harf}</span></div>`;
    };
    const bolum = (ad, l, hepsi) => l.length ? `<div class="git-bolum"><span class="esnek">${ad} (${l.length})</span>${hepsi}</div>${l.map(oge).join('')}` : '';
    const yon = (G.onde ? ` ↑${G.onde}` : '') + (G.geride ? ` ↓${G.geride}` : '');
    return `<div class="git-dal" title="Dallar: geçiş, yeni dal, birleştir">${S('fork_right')}<b>${kac(G.dal || '?')}</b><span>${yon}</span>${S('expand_more', '', 'margin-left:auto;font-size:16px')}</div>
      <textarea id="gitMesaj" data-g="gitMesaj" class="metin-girdi git-mesaj" rows="2" placeholder="Ne değişti? (Ctrl+Enter: işle)">${kac(D.gitMesaj || '')}</textarea>
      <div class="dugme birincil git-isle ${G.degisiklikler.length ? '' : 'pasif'}" data-e="gitIsle">${S('check')}İşle (commit)</div>
      ${bolum('Hazırlanan', hazir, `<span class="simge" title="Hepsini hazırlıktan çıkar" data-e="gitHazirla" data-a="-*">remove</span>`)}
      ${bolum('Değişiklikler', bekleyen, `<span class="simge" title="Hepsini hazırla" data-e="gitHazirla" data-a="+*">add</span>`)}
      ${G.degisiklikler.length ? '' : '<div class="panel-not">Değişiklik yok.</div>'}
      ${G.gecmis?.length ? `<div class="git-bolum" style="margin-top:14px"><span class="esnek">Son işlemeler</span></div>${G.gecmis.map(c => `<div class="git-islem" title="${kac(c.yazar)} · ${kac(c.zaman)}"><span class="mono">${kac(c.kisa)}</span> ${kac(c.mesaj)}<span class="git-zaman">${kac(c.zaman)}</span></div>`).join('')}` : ''}`;
  }

  async function gitYukle() {
    if (!D.proje) return;
    const r = await api('/api/git/durum?' + sorgu({ kok: D.proje.yol })).catch(e => ({ hata: e.message }));
    D.git = r;
    if (r.depo && r.dal) D.proje.dal = r.dal;
    if (D.yanPanel === 'git') cizYanPanel();
    guncelle('durum');
  }

  /** Kayıttan ya da dosya değişikliğinden sonra Git durumu kısa bir beklemeyle yenilenir. */
  let gitYenilemeZamani;
  function gitYenilePlanla() {
    if (!D.proje || (D.yanPanel !== 'git' && !D.git)) return;
    clearTimeout(gitYenilemeZamani);
    gitYenilemeZamani = setTimeout(gitYukle, 500);
  }

  async function gitIslem(yol, govde, basari) {
    const r = await api(yol, { kok: D.proje.yol, ...govde }).catch(e => ({ hata: e.message }));
    if (r.hata) { bildir(r.hata, true); D.altPanel = true; D.altSekme = 'cikti'; D.cikti.push({ t: r.hata, c: 'err' }); guncelle('alt'); }
    else if (basari) bildir(typeof basari === 'function' ? basari(r) : basari);
    await gitYukle();
    return !r.hata;
  }

  /** Birleşik farkı satır satır renklendirir. */
  function farkHtml(f) {
    if (!f.trim()) return '<div class="panel-not">Fark yok (yalnızca dosya kipi değişmiş olabilir).</div>';
    return f.split('\n').filter(s => !/^(diff --git|index |new file mode|deleted file mode|similarity|rename )/.test(s)).map(s => {
      const c = s.startsWith('@@') ? 'f-baslik' : s.startsWith('+++') || s.startsWith('---') ? 'f-dosya' : s[0] === '+' ? 'f-ekle' : s[0] === '-' ? 'f-sil' : '';
      return `<div class="${c}">${kac(s) || ' '}</div>`;
    }).join('');
  }

  /** Sınamalar paneli (Test Gezgini): *_sına.ohc dosyalarındaki sına_ işlevleri. */
  function sinamaPaneli() {
    const L = D.sinamalar;
    if (!L) { sinamalariYukle(); return '<div class="panel-not">Yükleniyor…</div>'; }
    if (!L.dosyalar.length) {
      return `<div class="panel-not">Bu projede sınama yok.<br><br>Adı <b>_sına.ohc</b> ile biten bir dosya oluşturun (ör. <span class="mono">hesap_sına.ohc</span>); adı <b>sına_</b> ile başlayan her işlev bir sınamadır:</div>
        <pre class="sinama-ornek">kullan "ana.ohc"\n\nişlev sına_toplama():\n    eşit_olmalı(2 + 3, 5)\n    doğrula(uzunluk("abc") == 3)</pre>`;
    }
    const S_ = D.sinamaSonuc;
    let gecen = 0, kalan = 0;
    const satirlar = L.dosyalar.map(d => {
      const ust = `<div class="sinama-dosya">${S('description', '', 'font-size:15px')}<span class="esnek" data-e="konumaGit" data-a="${kac(d.dosya)}|1">${kac(d.dosya)}</span><span class="simge sinama-calistir" title="Bu dosyadaki sınamaları çalıştır" data-e="sinamaDosyaCalistir" data-a="${kac(d.dosya)}">play_arrow</span></div>`;
      if (d.hata || S_[d.dosya + '|']) return ust + `<div class="sinama-mesaj">${kac((S_[d.dosya + '|'] || d.hata).split('\n')[0])}</div>`;
      return ust + d.sinamalar.map(t => {
        const s = S_[d.dosya + '|' + t.ad];
        if (s) s.gecti ? gecen++ : kalan++;
        const durum = D.sinamaMesgul && (!D.sinamaMesgul.dosya || D.sinamaMesgul.dosya === d.dosya) && (!D.sinamaMesgul.ad || D.sinamaMesgul.ad === t.ad)
          ? '<span class="simge sinama-durum bekliyor">more_horiz</span>'
          : s ? `<span class="simge sinama-durum ${s.gecti ? 'gecti' : 'kaldi'}">${s.gecti ? 'check_circle' : 'error'}</span>` : '<span class="simge sinama-durum">task_alt</span>';
        return `<div class="sinama-oge">${durum}<span class="esnek" data-e="konumaGit" data-a="${kac(d.dosya)}|${t.satir}" title="Tanıma git">${kac(t.ad.replace(/^s[ıi]na_/, '').replace(/_/g, ' '))}</span><span class="simge sinama-calistir" title="Bu sınamayı çalıştır" data-e="sinamaCalistir" data-a="${kac(d.dosya)}|${kac(t.ad)}">play_arrow</span></div>`
          + (s && !s.gecti ? `<div class="sinama-mesaj">${kac(s.mesaj)}${s.cikti ? `<pre>${kac(s.cikti.trimEnd())}</pre>` : ''}</div>` : '');
      }).join('');
    }).join('');
    const ozet = gecen + kalan ? `<div class="sinama-ozet ${kalan ? 'kaldi' : 'gecti'}">${gecen} geçti · ${kalan} kaldı</div>` : '';
    return ozet + satirlar;
  }

  /** Veritabanı paneli: JSON kayıt dosyaları ve SQL tabloları. */
  function vtPaneli() {
    const V = D.vt;
    if (!D.proje) return '<div class="panel-not">Önce bir proje açın.</div>';
    if (!V) { vtYukle(); return '<div class="panel-not">Yükleniyor…</div>'; }
    if (V.hata) return `<div class="panel-not">${kac(V.hata)}</div>`;
    const ad = { json: 'JSON dosyaları', sqlite: 'SQLite', postgresql: 'PostgreSQL', mysql: 'MySQL / MariaDB', sqlserver: 'SQL Server' }[V.tur] || V.tur;
    const oge = (e, a, s) => `<div class="yapi-oge" data-e="${e}" data-a="${kac(a)}">${S(s, '', 'font-size:15px')}<span class="esnek">${kac(a)}</span></div>`;
    let h = `<div class="panel-not">Depolama: <b>${ad}</b><br><span style="color:var(--yazi2)">.env içindeki ORHUNCA_VERITABANI ile değişir.</span></div>`;
    if (V.json.length) h += '<div class="panel-not"><b>Kayıt dosyaları (veri/)</b></div>' + V.json.map(m => oge('vtJsonAc', m, 'data_object')).join('');
    if (V.sql.length) h += '<div class="panel-not"><b>SQL tabloları</b></div>' + V.sql.map(t => oge('vtTabloAc', t, 'table')).join('');
    if (V.sql_hatasi) h += `<div class="sinama-mesaj">${kac(V.sql_hatasi)}</div>`;
    if (!V.json.length && !V.sql.length) h += '<div class="panel-not">Henüz kayıt yok. Programı çalıştırıp bir model kaydettiğinizde burada görünür.</div>';
    return h;
  }

  async function vtYukle() {
    if (!D.proje) return;
    const r = await api('/api/veritabani?' + sorgu({ kok: D.proje.yol })).catch(e => ({ hata: e.message }));
    D.vt = r.hata ? { hata: r.hata } : r.sonuc;
    if (D.yanPanel === 'veritabani') cizYanPanel();
  }

  function vtAd(t) {
    return D.vt?.tur === 'mysql' ? '`' + t + '`' : D.vt?.tur === 'sqlserver' ? '[' + t + ']' : '"' + t + '"';
  }

  async function vtAc(tur, ad) {
    if (tur === 'sql' && !(await guvenSor('Veritabanı sorgusu çalıştırmak'))) return;
    const sql = tur === 'sql' && ad ? (D.vt?.tur === 'sqlserver' ? `SELECT TOP 200 * FROM ${vtAd(ad)}` : `SELECT * FROM ${vtAd(ad)} LIMIT 200`) : '';
    D.modal = { tur: 'vt', kip: tur, ad, sql, satirlar: null, hata: '' };
    katmanlariCiz();
    if (tur === 'json') {
      const r = await api('/api/veritabani/json?' + sorgu({ kok: D.proje.yol, model: ad })).catch(e => ({ hata: e.message }));
      Object.assign(D.modal, r.hata ? { hata: r.hata, satirlar: [] } : { satirlar: r.sonuc });
      katmanlariCiz();
    } else if (sql) vtSorgula();
  }

  async function vtSorgula() {
    const m = D.modal;
    if (m?.tur !== 'vt') return;
    m.sql = $('#vtSql')?.value ?? m.sql;
    m.calisiyor = true; m.hata = ''; katmanlariCiz();
    const r = await api('/api/veritabani/sorgu', { kok: D.proje.yol, sorgu: m.sql }).catch(e => ({ hata: e.message }));
    m.calisiyor = false;
    Object.assign(m, r.hata ? { hata: r.hata, satirlar: [] } : { satirlar: Array.isArray(r.sonuc) ? r.sonuc : [r.sonuc] });
    katmanlariCiz();
    if (!/^\s*(select|with|show|pragma)/i.test(m.sql)) vtYukle();
  }

  function vtTablosu(satirlar) {
    if (!satirlar) return '<div class="donen kucuk"></div>';
    if (!satirlar.length) return '<div class="panel-not">Kayıt yok.</div>';
    const sutunlar = [...new Set(satirlar.flatMap(s => s && typeof s === 'object' ? Object.keys(s) : ['değer']))];
    const hucre = d => kac(d == null ? '' : typeof d === 'object' ? JSON.stringify(d) : String(d));
    return `<div class="vt-tablo"><table><thead><tr>${sutunlar.map(s => `<th>${kac(s)}</th>`).join('')}</tr></thead><tbody>${
      satirlar.map(s => `<tr>${sutunlar.map(k => `<td>${hucre(s && typeof s === 'object' ? s[k] : s)}</td>`).join('')}</tr>`).join('')}</tbody></table></div>
      <div class="panel-not">${satirlar.length} satır</div>`;
  }

  function cizVt(kabuk) {
    const m = D.modal;
    const ust = m.kip === 'sql'
      ? `<textarea id="vtSql" class="metin-girdi mono" rows="4" spellcheck="false" placeholder="SELECT * FROM ...">${kac(m.sql)}</textarea>`
      : '';
    return kabuk(m.kip === 'json' ? `veri/${kac(m.ad)}.json` : 'SQL sorgusu', ust + (m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : '') + vtTablosu(m.kip === 'sql' && !m.sql ? [] : m.satirlar),
      m.kip === 'sql' ? `${m.calisiyor ? '<div class="donen kucuk"></div>' : ''}<div class="dugme" data-e="modalKapat">Kapat</div><div class="dugme birincil" data-e="vtSorgula">Çalıştır</div>` : '<div class="dugme" data-e="modalKapat">Kapat</div>');
  }

  async function sinamalariYukle() {
    if (!D.proje) return;
    const r = await api('/api/sinamalar?' + sorgu({ kok: D.proje.yol })).catch(e => ({ hata: e.message }));
    D.sinamalar = r.hata ? { dosyalar: [] } : r;
    if (r.hata) bildir(r.hata, true);
    if (D.yanPanel === 'sinamalar') cizYanPanel();
  }

  async function sinamalariCalistir(dosya = '', ad = '') {
    if (!D.proje || D.sinamaMesgul) return;
    if (!(await guvenSor('Sınamaları çalıştırmak'))) return;
    if (!(await tumunuKaydet())) { bildir('Dosya kaydedilemediği için sınamalar çalıştırılmadı.', true); return; }
    D.sinamaMesgul = { dosya, ad };
    await sinamalariYukle();
    const r = await api('/api/sina', { kok: D.proje.yol, dosya, ad }).catch(e => ({ hata: e.message }));
    D.sinamaMesgul = null;
    if (r.hata) { bildir(r.hata, true); cizYanPanel(); return; }
    for (const d of r.dosyalar) {
      delete D.sinamaSonuc[d.dosya + '|'];
      if (d.hata) D.sinamaSonuc[d.dosya + '|'] = d.hata;
      for (const t of d.sinamalar) D.sinamaSonuc[d.dosya + '|' + t.ad] = t;
    }
    bildir(r.kaldi ? `${r.kaldi} sınama kaldı, ${r.gecti} geçti.` : `${r.gecti} sınamanın hepsi geçti.`, r.kaldi > 0);
    cizYanPanel();
  }

  function dersPaneli() {
    if (!DERSLER) { dersleriYukle().then(() => cizYanPanel()); return '<div class="panel-not">Yükleniyor…</div>'; }
    const tamam = dersTamam();
    const d = DERSLER.find(x => x.kimlik === D.ders);
    if (!d) {
      return DERSLER.map(x => {
        const bitti = x.gorevler.filter((_, i) => tamam[x.kimlik + '-' + (i + 1)]).length;
        return `<div class="ders-oge" data-e="dersSec" data-a="${x.kimlik}"><span class="ders-no ${bitti === x.gorevler.length && bitti ? 'bitti' : ''}">${bitti === x.gorevler.length && bitti ? '✓' : x.sira}</span><span class="esnek"><b>${kac(x.baslik)}</b><span>${kac(x.ozet)}</span></span></div>`;
      }).join('');
    }
    const gorevler = d.gorevler.map((g, i) => {
      const k = d.kimlik + '-' + (i + 1), sonuc = D.dersSonuc?.[k];
      return `<div class="ders-gorev ${tamam[k] ? 'tamam' : ''}"><div class="ders-gorev-baslik">${tamam[k] ? S('check_circle') : S('task_alt')}<b>${kac(g.baslik)}</b></div>${g.aciklama}
        ${g.girdi ? `<div class="ders-kutu"><span>Girdi</span><pre>${kac(g.girdi)}</pre></div>` : ''}
        ${g.cikti != null ? `<div class="ders-kutu"><span>Beklenen çıktı</span><pre>${kac(g.cikti)}</pre></div>` : ''}
        <div class="ders-dugmeler"><div class="panel-dugme birincil" data-e="gorevBasla" data-a="${i}">${S('edit')}Başla</div><div class="panel-dugme" data-e="gorevDenetle" data-a="${i}">${S('check')}Kontrol et</div></div>
        ${sonuc ? `<div class="ders-sonuc ${sonuc.basarili ? 'iyi' : 'kotu'}">${kac(sonuc.mesaj)}${sonuc.cikti != null ? `<pre>${kac(sonuc.cikti)}</pre>` : ''}</div>` : ''}
        <details><summary>Çözümü göster</summary><pre data-orhunca>${kac(g.cozum)}</pre></details></div>`;
    }).join('');
    const sira = DERSLER.indexOf(d);
    return `<div class="ders-ust"><span data-e="dersSec" data-a="">${S('arrow_back')} Dersler</span>${DERSLER[sira + 1] ? `<span data-e="dersSec" data-a="${DERSLER[sira + 1].kimlik}">Sonraki ${S('arrow_forward')}</span>` : ''}</div>
      <div class="ders-baslik"><small>Ders ${d.sira}</small>${kac(d.baslik)}</div><div class="ders-icerik">${d.html}</div>
      <div class="ders-alt-baslik">ALIŞTIRMALAR</div>${gorevler}`;
  }
  async function gorevBasla(i) {
    const d = DERSLER.find(x => x.kimlik === D.ders), g = d.gorevler[i];
    const r = await api('/api/ders/hazirla', { dosya: `${d.kimlik}-${i + 1}`, baslangic: g.baslangic }).catch(e => ({ hata: e.message }));
    if (r.hata) { bildir(r.hata, true); return; }
    if (D.proje?.yol !== r.proje.yol) await projeyiAc(r.proje);
    D.yanPanel = 'dersler';
    await dosyaAc(r.dosya);
    if (g.girdi) bildir('Bu alıştırma girdi okur: çalıştırınca terminale yazın ya da "Kontrol et" ile hazır girdiyle denetleyin.');
    guncelle('yan', 'etkinlik');
  }
  async function gorevDenetle(i) {
    const d = DERSLER.find(x => x.kimlik === D.ders), g = d.gorevler[i], k = `${d.kimlik}-${i + 1}`;
    if (!D.proje || !D.sekmeler.some(s => s.yol === k + '.ohc')) { await gorevBasla(i); return; }
    if (!(await tumunuKaydet())) { bildir('Dosya kaydedilemediği için denetlenmedi.', true); return; }
    const r = await api('/api/ders/denetle', { dosya: tamYol(k + '.ohc'), girdi: g.girdi || '', beklenen: g.cikti }).catch(e => ({ hata: e.message }));
    D.dersSonuc = D.dersSonuc || {};
    if (r.basarili) {
      const t = dersTamam(); t[k] = true; ayarYaz('derslerTamam', t);
      D.dersSonuc[k] = { basarili: true, mesaj: 'Tebrikler! Alıştırma doğru. 🎉' };
    } else if (r.derleme_hatasi) {
      D.dersSonuc[k] = { basarili: false, mesaj: 'Program derlenmedi:', cikti: r.derleme_hatasi };
    } else if (r.hata && r.cikti == null) {
      D.dersSonuc[k] = { basarili: false, mesaj: r.hata };
    } else {
      D.dersSonuc[k] = { basarili: false, mesaj: 'Çıktı beklenenden farklı. Programın çıktısı:', cikti: (r.cikti || '') + (r.hata || '') };
    }
    cizYanPanel();
  }

  /** Hata ayıklama: araç çubuğu, değişkenler, çağrı yığını, kesme noktaları */
  function ayiklamaPaneli() {
    const ay = D.calisma.ay, durdu = !!ay?.durdu;
    const dugme = (e, simge, ad, kisayol) => `<span class="simge ${durdu ? '' : 'pasif'}" title="${ad} (${kisayol})" data-e="${e}">${simge}</span>`;
    const c = D.calisma;
    const yavasArac = c.yavas ? `<div class="ay-yavas"><span class="simge" title="${c.yavasAcik ? 'Duraklat' : 'Sürdür'}" data-e="yavasDegistir">${c.yavasAcik ? 'pause' : 'play_arrow'}</span>
      <span>Adım adım</span><select data-g="yavasHiz" title="Hız">${[[1500, 'Çok yavaş'], [700, 'Yavaş'], [300, 'Orta'], [100, 'Hızlı']].map(([v, a]) => `<option value="${v}" ${+D.yavasHiz === v ? 'selected' : ''}>${a}</option>`).join('')}</select></div>` : '';
    const arac = yavasArac + `<div class="ay-arac">${durdu ? dugme('ayDevam', 'play_arrow', 'Devam', 'F5') : `<span class="simge" title="Duraklat" data-e="ayDuraklat">pause</span>`}
      ${dugme('ayUstunden', 'redo', 'Üstünden adım', 'F10')}${dugme('ayAdim', 'arrow_downward', 'İçine adım', 'F11')}${dugme('ayCik', 'arrow_upward', 'Dışına adım', '⇧+F11')}
      <span class="simge" title="Durdur (⇧+F5)" data-e="durdur">stop</span>
      <span class="ay-durum">${!ay?.bagli && !durdu ? 'başlatılıyor…' : durdu ? ({ kesme: 'kesme noktası', hata: 'çalışma hatası', duraklat: 'duraklatıldı' }[ay.neden] || 'durdu') : 'çalışıyor'}</span></div>`;
    if (!durdu) return arac + kesmelerBolumu();
    const ileti = ay.ileti ? `<div class="ay-ileti">${kac(ay.ileti)}</div>` : '';
    const degiskenler = ay.degiskenler.length
      ? ay.degiskenler.map(d => `<div class="ay-deg" title="${kac(d.ad + ': ' + d.tip + ' = ' + d.deger)}"><span class="ad">${kac(d.ad)}</span><span class="tip">${kac(d.tip)}</span><span class="deger">${kac(d.deger)}</span></div>`).join('')
      : '<div class="panel-not">Bu çerçevede değişken yok.</div>';
    const yigin = ay.yigin.map((c, i) => `<div class="ay-cerceve ${i === ay.cerceve ? 'secili' : ''}" data-e="cerceveSec" data-a="${i}"><span>${kac(c.islev)}</span><span class="yer">${kac(goreliYol(c.dosya))}:${c.satir}</span></div>`).join('');
    const izlenen = `<div class="ay-baslik">İZLENENLER</div>` + D.izlenenler.map((ad, i) => {
      const d = ay.degiskenler.find(x => x.ad === ad);
      return `<div class="ay-deg"><span class="ad">${kac(ad)}</span><span class="tip">${d ? kac(d.tip) : ''}</span><span class="deger">${d ? kac(d.deger) : '<i>bu çerçevede yok</i>'}</span><span class="simge ay-sil" title="Kaldır" data-e="izlemeKaldir" data-a="${i}">close</span></div>`;
    }).join('') + `<input id="izlemeEkle" class="metin-girdi ay-izle" placeholder="Değişken adı + Enter" spellcheck="false" autocomplete="off">`;
    return arac + ileti + izlenen + `<div class="ay-baslik">DEĞİŞKENLER</div>${degiskenler}<div class="ay-baslik">ÇAĞRI YIĞINI</div>${yigin}` + kesmelerBolumu();
  }

  function kesmelerBolumu() {
    const l = kesmeListesi();
    return `<div class="ay-baslik">KESME NOKTALARI</div>` + (l.length
      ? l.map(k => `<div class="ay-cerceve" data-e="konumaGit" data-a="${kac(goreliYol(k.dosya))}|${k.satir}" title="Sağ tık satır numarasında: koşul ve günlük"><span>${kac(goreliYol(k.dosya).split('/').pop())}${k.gunluk ? ` <span class="ay-kosul">◆ ${kac(k.gunluk)}</span>` : k.kosul ? ` <span class="ay-kosul">eğer ${kac(k.kosul)}</span>` : ''}</span><span class="yer">satır ${k.satir}</span></div>`).join('')
      : '<div class="panel-not">Satır numarasına tıklayarak ekleyin (F9). Sağ tık: koşullu kesme ya da günlük noktası.</div>');
  }
