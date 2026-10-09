/* Orhunca Stüdyo — Proje işlemleri.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Proje işlemleri
  // =====================================================================
  /** Proje dosyasında yazan giriş dosyası diskte yoksa hata metni (yoksa null). */
  function girisHatasi() {
    const g = D.proje?.giris;
    if (!g || D.agac.some(x => x.yol === normal(g))) return null;
    return `Proje dosyasında giriş olarak yazan '${g}' bulunamadı. Dosyayı oluşturun ya da .ohcproj dosyasındaki giriş satırını düzeltin.`;
  }

  function girisDosyasi() {
    if (!D.proje) return null;
    // Yazılı giriş dosyası yoksa başka bir dosya sessizce seçilmez (CLI ile aynı davranış).
    if (D.proje.giris) return girisHatasi() ? null : normal(D.proje.giris);
    const s = etkinSekme();
    if (s && uzanti(s.yol) === 'ohc') return s.yol;
    return D.agac.find(g => !g.klasor && uzanti(g.yol) === 'ohc')?.yol || null;
  }

  async function agaciYukle() {
    const r = await api('/api/agac?' + sorgu({ kok: D.proje.yol }));
    D.agac = r.girdiler || [];
  }

  /** Açık projeden ayrılmadan önce kaydedilmemiş değişiklikleri kaydeder; kaydedilemezse
   *  kullanıcıya sorar. Ayrılmak güvenliyse doğru döner. */
  async function degisiklikleriKoru() {
    if (!D.proje || !D.sekmeler.some(s => !s.ikili && s.icerik !== s.kayitli)) return true;
    if (await tumunuKaydet()) return true;
    return onayla('Bazı dosyalar kaydedilemedi. Kaydedilmemiş değişiklikler kaybolacak; yine de devam edilsin mi?', { dugme: 'Devam et', tehlikeli: true });
  }

  async function projeyiAc(bilgi, { ilkCalistirma = false } = {}) {
    // Aynı proje yeniden açılıyorsa (ör. başlangıç ekranından) açık sekmeler korunur.
    if (D.proje && D.proje.yol === bilgi.yol && D.sekmeler.length) {
      D.ekran = 'duzenleyici'; D.menu = null; D.modal = null;
      iskeletVar = false; ciz();
      return;
    }
    if (!(await degisiklikleriKoru())) return;
    if (D.calisma) await durdur();
    D.proje = bilgi; D.kisitliSeritKapali = false; D.sinamalar = null; D.sinamaSonuc = {}; D.git = null; D.gitMesaj = '';
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
    else if (girisHatasi()) bildir(girisHatasi(), true);
    if (ilkCalistirma && giris) calistir();
    else {
      D.terminal = [{ t: istem(), c: 'mut' }, { t: bilgi.arayuz ? 'Uygulamayı önizlemede açmak için F5’e basın.' : bilgi.web ? 'Sunucuyu başlatıp sayfayı önizlemek için F5’e basın.' : 'Çalıştırmak için F5’e basın.', c: 'dim' }];
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

  async function kaydet(s = etkinSekme(), sessiz = false, otomatik = false) {
    if (!s || s.ikili || s.icerik === s.kayitli) return true;
    const icerik = s.icerik;
    const r = await api('/api/dosya', { yol: tamYol(s.yol), icerik }).catch(e => ({ hata: e.message }));
    if (r.hata) {
      // Otomatik kaydetmede aynı hata her duraklamada yeniden gösterilmez.
      if (!otomatik || s.otoHata !== r.hata) bildir(r.hata, true);
      if (otomatik) s.otoHata = r.hata;
      return false;
    }
    s.otoHata = null;
    s.kayitli = icerik;
    gitYenilePlanla();
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

  let kaydetmeZamani;
  /** Otomatik kaydetme: yazmaya ara verildikten 1,5 sn sonra açık dosya kaydedilir. */
  function kaydetmeyiPlanla() {
    if (!D.otomatikKaydet) return;
    clearTimeout(kaydetmeZamani);
    kaydetmeZamani = setTimeout(async () => {
      const s = etkinSekme();
      if (!s || s.ikili || s.icerik === s.kayitli) return;
      await kaydet(s, true, true);
      if (D.ekran === 'duzenleyici') cizSekmeler();
    }, 1500);
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
    let basarisiz = null;
    const ayni = (a, b) => a.mesaj === b.mesaj && a.satir === b.satir && a.sutun === b.sutun && normal(a.dosya) === normal(b.dosya);
    for (const h of hedefler) {
      const r = await api('/api/denetle', { dosya: tamYol(h), acik }).catch(e => ({ hata: e.message || 'bağlantı kurulamadı' }));
      // Denetim yapılamadıysa bu "hata yok" demek değildir.
      if (r.hata) basarisiz = r.hata;
      for (const x of r.hatalar || []) if (!hatalar.some(y => ayni(x, y))) hatalar.push(x);
      for (const x of r.uyarilar || []) if (!uyarilar.some(y => ayni(x, y))) uyarilar.push(x);
    }
    if (sira !== denetimSirasi || D.ekran !== 'duzenleyici') return hedefler.length;
    D.sorunlar = hatalar;
    D.uyarilar = uyarilar;
    D.denetimHatasi = basarisiz;
    guncelle('isaretler', 'durum');
    const toplam = hatalar.length + uyarilar.length;
    if (D.altSekme === 'sorunlar') guncelle('alt');
    else { const sekme = $('.alt-sekmeler span[data-a="sorunlar"]'); if (sekme) sekme.textContent = `SORUNLAR${toplam ? ' (' + toplam + ')' : ''}`; }
    return hedefler.length;
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

  /** Adım adım gösterim: program her satırda kısa bir süre durarak çalışır;
   *  satır vurgulanır, değişkenler güncellenir (öğretmek için). */
  function yavasCalistir() { calistir(true, { yavas: true }); }

  async function calistir(ayikla = false, { yavas = false } = {}) {
    if (!D.proje) return;
    if (D.calisma) await durdur();
    if (!(await guvenSor(ayikla ? 'Hata ayıklamak' : 'Programı çalıştırmak'))) return;
    if (!(await tumunuKaydet())) { bildir('Dosya kaydedilemediği için program çalıştırılmadı (ekrandaki kod diskteki koddan farklı).', true); return; }
    const giris = girisDosyasi();
    if (!giris) { bildir(girisHatasi() || 'Çalıştırılacak .ohc dosyası yok.', true); return; }
    D.altPanel = true; D.altSekme = 'terminal';
    if (D.terminal.length) terminaleEkle('');
    terminaleEkle(istem(), 'mut');
    guncelle('alt');
    const argumanlar = D.argumanlar.match(/"[^"]*"|\S+/g)?.map(a => a.replace(/^"|"$/g, '')) || [];
    const r = await api('/api/calistir', { dosya: tamYol(giris), klasor: D.proje.yol, argumanlar, ayikla, ilkte_dur: yavas, kesmeler: ayikla ? kesmeListesi() : [] }).catch(e => ({ hata: e.message }));
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
    if (yavas) {
      terminaleEkle('● Adım adım gösterim: her satır vurgulanır, değişkenler yan panelde', 'bilgi');
      D.yanPanel = 'calistir';
    } else if (ayikla) {
      const n = kesmeListesi().length;
      terminaleEkle(`● Hata ayıklama · ${n ? n + ' kesme noktası' : 'kesme noktası yok (satır numarasına tıklayarak ekleyin)'}`, 'bilgi');
      D.yanPanel = 'calistir';
    }
    D.calisma = { kimlik: r.kimlik, konum: 0, ayikla, ay: null, yavas, yavasAcik: yavas };
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
          // Adım adım gösterim: kesme noktası ve hata dışında kendiliğinden ilerler.
          const c = D.calisma;
          if (c?.yavas && c.yavasAcik && r.ayiklama.neden !== 'hata') {
            const surum = r.ayiklama.surum;
            setTimeout(() => {
              if (D.calisma === c && c.yavasAcik && c.ay?.durdu && c.ay.surum === surum) ayiklamaKomutu('adim');
            }, D.yavasHiz);
          }
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
    if (!(await guvenSor('Derlemek'))) return;
    if (!(await tumunuKaydet())) { bildir('Dosya kaydedilemediği için derlenmedi.', true); return; }
    const giris = girisDosyasi();
    if (!giris) { bildir(girisHatasi() || 'Derlenecek .ohc dosyası yok.', true); return; }
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
      setTimeout(guncellemeyiDenetle, 4000);
      temaResminiYukle();
      asistanDurumuYukle();
    } catch (e) {
      $('#uygulama').innerHTML = `<div class="pencere"><div class="tam-ekran-mesaj"><div class="gokturk">${GOKTURK}</div><div>${kac(e.message)}</div><div>Terminalde <code>orhunca stüdyo</code> ile yeniden açın.</div></div></div>`;
      throw e;
    }
  }

  async function galeriyiYukle() {
    D.galeriHatasi = '';
    const r = await api('/api/tema/galeri').catch(e => ({ hata: e.message }));
    if (r.hata) D.galeriHatasi = r.hata; else D.galeri = r.temalar || [];
    if (D.modal?.tur === 'gorunum') katmanlariCiz();
  }

  const PY_ANAHTAR = new Set(['def', 'return', 'if', 'elif', 'else', 'for', 'in', 'while', 'import', 'from', 'class', 'try', 'except', 'as', 'and', 'or', 'not', 'is', 'True', 'False', 'None', 'pass', 'break', 'continue', 'raise', 'lambda', 'with']);
  const PY_SOZCUK = /(#.*$)|("(?:[^"\\]|\\.)*"?|'(?:[^'\\]|\\.)*'?)|(\b\d+(?:\.\d+)?\b)|([\p{L}_][\p{L}\p{N}_]*)|(\s+)|(.)/gu;
  function vurgulaPy(satir) {
    let html = '', m;
    const R = new RegExp(PY_SOZCUK.source, 'gu');
    while ((m = R.exec(satir))) {
      const t = m[0];
      let c = m[1] ? 'c' : m[2] ? 's' : m[3] ? 'n' : '';
      if (m[4]) c = PY_ANAHTAR.has(t) ? 'k' : satir.slice(R.lastIndex).trimStart()[0] === '(' ? 'f' : '';
      html += sarmala(c, t);
    }
    return html;
  }

  /** Orhunca kodu ve Python/JavaScript karşılığı yan yana, satır satır eşleşmiş. */
  /** MCP destekleyen istemciler ve Orhunca sunucusunu eklemek için yapılandırmaları. */
  function ajanIstemcileri(komut) {
    const k = JSON.stringify(komut), kabuk = /[\s"'$]/.test(komut) ? k : komut;
    const mcpServers = `{\n  "mcpServers": {\n    "orhunca": { "command": ${k}, "args": ["mcp"] }\n  }\n}`;
    return [
      { kimlik: 'claude-code', ad: 'Claude Code', yer: 'Terminalde çalıştırın (bütün projelerde kullanmak için sona --scope user ekleyin):', kod: `claude mcp add orhunca -- ${kabuk} mcp` },
      { kimlik: 'codex', ad: 'Codex CLI', yer: 'Terminalde çalıştırın ya da ~/.codex/config.toml dosyasına ekleyin:', kod: `codex mcp add orhunca -- ${kabuk} mcp\n\n# ya da ~/.codex/config.toml\n[mcp_servers.orhunca]\ncommand = ${k}\nargs = ["mcp"]` },
      { kimlik: 'gemini', ad: 'Gemini CLI', yer: 'Terminalde çalıştırın ya da ~/.gemini/settings.json dosyasına ekleyin:', kod: `gemini mcp add orhunca ${kabuk} mcp\n\n// ya da ~/.gemini/settings.json\n${mcpServers}` },
      { kimlik: 'cursor', ad: 'Cursor', yer: 'Proje klasöründe .cursor/mcp.json (ya da bütün projeler için ~/.cursor/mcp.json):', kod: mcpServers },
      { kimlik: 'vscode', ad: 'VS Code (Copilot)', yer: 'Proje klasöründe .vscode/mcp.json:', kod: `{\n  "servers": {\n    "orhunca": { "type": "stdio", "command": ${k}, "args": ["mcp"] }\n  }\n}` },
      { kimlik: 'claude-desktop', ad: 'Claude Desktop', yer: 'Ayarlar → Geliştirici → Yapılandırmayı düzenle (claude_desktop_config.json):', kod: mcpServers },
      { kimlik: 'windsurf', ad: 'Windsurf', yer: '~/.codeium/windsurf/mcp_config.json:', kod: mcpServers },
      { kimlik: 'cline', ad: 'Cline / Roo Code', yer: 'MCP Servers → Configure → cline_mcp_settings.json (yerel modellerle de çalışır):', kod: mcpServers },
      { kimlik: 'continue', ad: 'Continue', yer: 'Proje klasöründe .continue/mcpServers/orhunca.yaml (Ollama gibi yerel modellerle de çalışır):', kod: `name: Orhunca\nversion: 0.0.1\nschema: v1\nmcpServers:\n  - name: orhunca\n    command: ${k}\n    args: ["mcp"]` },
      { kimlik: 'zed', ad: 'Zed', yer: 'settings.json dosyasına ekleyin:', kod: `{\n  "context_servers": {\n    "orhunca": { "source": "custom", "command": ${k}, "args": ["mcp"] }\n  }\n}` },
      { kimlik: 'opencode', ad: 'OpenCode', yer: 'Proje klasöründe opencode.json:', kod: `{\n  "$schema": "https://opencode.ai/config.json",\n  "mcp": {\n    "orhunca": { "type": "local", "command": [${k}, "mcp"], "enabled": true }\n  }\n}` },
      { kimlik: 'diger', ad: 'Diğer', yer: 'MCP destekleyen araçların çoğu (JetBrains AI, LM Studio, Goose, Jan…) bu biçimi kabul eder. Sunucu stdin/stdout üzerinden konuşur:', kod: mcpServers },
    ];
  }

  function cizAjan(kabuk) {
    const m = D.modal, l = ajanIstemcileri(m.komut), i = l.find(x => x.kimlik === m.istemci) || l[0];
    return kabuk(`${S('hub')} Kendi ajanınızı bağlayın`, `
      <div class="secenek-alt" style="margin-bottom:12px">Orhunca bir <b>MCP sunucusu</b> sunar. Ajanınız bu sunucuya bağlanınca Orhunca'nın dil rehberini okur, yazdığı kodu denetler ve çalıştırır. Araçlar: <span class="mono">orhunca_rehber</span>, <span class="mono">orhunca_denetle</span>, <span class="mono">orhunca_calistir</span>, <span class="mono">orhunca_bicimlendir</span>.</div>
      <div class="ajan-istemciler">${l.map(x => `<span class="asistan-saglayici ${x.kimlik === i.kimlik ? 'secili' : ''}" data-e="ajanIstemci" data-a="${x.kimlik}">${kac(x.ad)}</span>`).join('')}</div>
      <div class="secenek-alt" style="margin:12px 0 6px">${kac(i.yer)}</div>
      <pre class="asistan-kod ajan-kod">${kac(i.kod)}</pre>
      <div class="secenek" data-e="ajanTalimatEkle" style="margin-top:12px"><div class="esnek"><div class="secenek-ad">Projeye AGENTS.md ekle</div><div class="secenek-alt">MCP desteklemeyen ajanlar da (Codex, Copilot, Cursor, Jules, Aider…) projedeki bu dosyayı okuyup Orhunca kodunu nasıl yazacağını öğrenir.</div></div>${S('note_add')}</div>`,
      `<div class="dugme" data-e="ajanKopyala">${S('content_copy')} Kopyala</div><div class="dugme birincil" data-e="modalKapat">Tamam</div>`).replace('class="modal"', 'class="modal genis"');
  }

  /** "bugün 14:32", "dün 09:05", "03.10.2026 18:40" */
  function zamanYazisi(ms) {
    const t = new Date(ms), b = new Date(), iki = n => String(n).padStart(2, '0');
    const saat = `${iki(t.getHours())}:${iki(t.getMinutes())}`;
    const gun = d => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
    const fark = Math.round((gun(b) - gun(t)) / 86400000);
    if (fark === 0) return 'bugün ' + saat;
    if (fark === 1) return 'dün ' + saat;
    return `${iki(t.getDate())}.${iki(t.getMonth() + 1)}.${t.getFullYear()} ${saat}`;
  }

  function cizGecmis(kabuk) {
    const m = D.modal;
    const liste = m.kayitlar === null ? '<div class="bos-durum"><div class="donen kucuk" style="margin:auto"></div></div>'
      : !m.kayitlar.length ? '<div class="panel-not">Bu dosyanın henüz bir geçmişi yok. Dosya kaydedildikçe önceki hâlleri burada birikir.</div>'
      : m.kayitlar.map((k, i) => `<div class="gecmis-oge ${k.zaman === m.secili ? 'secili' : ''}" data-e="gecmisSec" data-a="${k.zaman}">${S(i === 0 ? 'save' : 'history')}<span class="esnek">${zamanYazisi(+k.zaman)}</span><span class="gecmis-boyut">${k.boyut} bayt</span></div>`).join('');
    const onizleme = m.icerik === null ? '<div class="panel-not">Soldan bir kayıt seçin.</div>'
      : `<pre class="asistan-kod gecmis-onizleme">${m.icerik.split('\n').map(x => vurgula(x, uzanti(m.yol) === 'ohc' ? 'ohc' : '')).join('\n')}</pre>`;
    return kabuk(`${S('history')} Yerel geçmiş: ${kac(m.yol)}`,
      `<div class="gecmis"><div class="gecmis-liste">${liste}</div><div class="gecmis-sag">${onizleme}</div></div>`,
      `<div class="panel-not" style="flex:1;font-size:12px;margin:0 12px 0 0">Geri getirdiğiniz hâl düzenleyiciye yazılır; Ctrl+Z ile geri alabilirsiniz.</div>
       <div class="dugme" data-e="modalKapat">Kapat</div><div class="dugme birincil ${m.icerik === null ? 'pasif' : ''}" data-e="gecmisGeriYukle">${S('restore')}Bu hâle dön</div>`).replace('class="modal"', 'class="modal genis"');
  }

  /** Terminaldeki son hata (derleme ya da çalışma hatası) ya da sorunlar panelindeki ilk hata. */
  function hataBildirSonHata() {
    const hatalar = D.terminal.filter(t => t.c === 'err').map(t => t.t);
    if (hatalar.length) return hatalar.slice(-6).join('\n');
    const h = D.sorunlar[0];
    return h ? `${h.dosya || ''}:${h.satir}:${h.sutun}: ${h.mesaj}` : '';
  }

  /** GitHub hata formunun alanlarını adresle doldurur (alan kimlikleri: .github/ISSUE_TEMPLATE/hata.yml). */
  function hataBildirAdresi(m) {
    const sistem = { windows: 'Windows 10/11', macos: 'macOS' }[D.bilgi.isletim] || 'Diğer Linux';
    const alan = { template: 'hata.yml', surum: D.bilgi.surum, sistem, ne: (m.ne || '').trim() || '(açıklama yazılmadı)' };
    const s = etkinSekme();
    if (m.kod && s && !s.ikili) alan.kod = s.icerik.slice(0, 3000);
    const h = hataBildirSonHata();
    if (m.cikti && h) alan.cikti = h.slice(0, 2000);
    alan.title = 'Hata: ' + alan.ne.split('\n')[0].slice(0, 70);
    const adres = () => 'https://github.com/Furkan003/Orhunca/issues/new?' + new URLSearchParams(alan).toString();
    // Adres çok uzunsa (tarayıcı sınırı) kod kısaltılır.
    while (adres().length > 7500 && alan.kod) alan.kod = alan.kod.slice(0, Math.floor(alan.kod.length * 0.7));
    return adres();
  }

  function cizCeviri(kabuk) {
    const m = D.modal;
    const vurgulaDil = m.dil === 'python' ? vurgulaPy : vurgulaJs;
    let govde;
    if (m.yukleniyor) govde = '<div class="bos-durum"><div class="donen kucuk" style="margin:auto"></div></div>';
    else if (m.hata) govde = `<div class="ceviri-hata">${kac(m.hata)}</div>`;
    else {
      const gruplar = [];
      for (const x of m.satirlar) {
        const son = gruplar.at(-1);
        if (son && son.k === x.k) son.m.push(x.m); else gruplar.push({ k: x.k, m: [x.m] });
      }
      const gosterilen = new Set();
      const satirlar = gruplar.map(g => {
        let sol = '';
        if (g.k && !gosterilen.has(g.k)) {
          gosterilen.add(g.k);
          sol = `<span class="no">${g.k}</span>${vurgula(m.kaynak[g.k - 1] || '', 'ohc')}`;
        }
        return `<div class="ceviri-satir ${sol ? '' : 'devam'}"><div class="ceviri-sol">${sol}</div><div class="ceviri-sag">${g.m.map(vurgulaDil).join('\n')}</div></div>`;
      }).join('');
      govde = `<div class="ceviri"><div class="ceviri-ust"><span>Orhunca</span><span>${m.dil === 'python' ? 'Python' : 'JavaScript'}</span></div><div class="ceviri-liste">${satirlar}</div></div>`;
    }
    const sekme = (d, ad) => `<span class="${m.dil === d ? 'secili' : ''}" data-e="ceviriDil" data-a="${d}">${ad}</span>`;
    const alt = `<div class="tema-secim">${sekme('python', 'Python')}${sekme('javascript', 'JavaScript')}</div>
      <div class="panel-not" style="flex:1;font-size:12px;margin:0 12px">Aynı program başka bir dilde. Kavramlar aynı, yalnızca yazılış değişir.</div>
      <div class="dugme" data-e="ceviriKopyala">${S('content_copy')} Kopyala</div><div class="dugme birincil" data-e="modalKapat">Tamam</div>`;
    return kabuk(`${S('translate')} Başka dillerde`, govde, alt).replace('class="modal"', 'class="modal genis"');
  }

  function cizGorunum(kabuk) {
    const T = window.OrhuncaTema, t = D.temaTaslak, m = D.modal;
    const renkKutulari = (r) => ['arka', 'pencere', 'vurgu', 'sz-anahtar', 'sz-metin', 'sz-islev'].map(a => `<i style="background:${kac(r?.[a] || 'transparent')}"></i>`).join('');
    const kart = (ad, alt, renkler, eylem, arg, ek = '') => `<div class="tema-kart" data-e="${eylem}" data-a="${kac(arg)}"><div class="tema-ornek">${renkKutulari(renkler)}</div><div class="esnek"><div class="tema-kart-ad">${kac(ad)}</div><div class="tema-kart-alt">${kac(alt)}</div></div>${ek}</div>`;
    let liste;
    if (m.sekme === 'hazir') liste = T.HAZIR.map((h, i) => kart(h.ad, h.taban === 'acik' ? 'Açık' : 'Koyu', { ...{ arka: h.taban === 'acik' ? '#e9e8e3' : '#0a0c0f', pencere: h.taban === 'acik' ? '#fff' : '#13161b', vurgu: h.taban === 'acik' ? '#0b8a83' : '#45d3c9' }, ...h.renkler }, 'temaHazir', i)).join('');
    else if (m.sekme === 'benim') liste = D.temalarim.map(x => kart(x.tema.ad, (x.resimli ? 'Arka plan resimli · ' : '') + (x.tema.yazar || ''), x.tema.renkler, 'temaBenim', x.kimlik, `<span class="simge sil" title="Sil" data-e="temaSil" data-a="${kac(x.kimlik)}">delete</span>`)).join('') || '<div class="panel-not">Henüz kaydettiğiniz tema yok. Bir temayı düzenleyip <b>Kaydet ve uygula</b>’ya basın.</div>';
    else liste = D.galeriHatasi ? `<div class="panel-not">Galeriye ulaşılamadı: ${kac(D.galeriHatasi)}</div>` : D.galeri ? (D.galeri.map(g => kart(g.ad, g.yazar ? 'Hazırlayan: ' + g.yazar : '', g.renkler, 'temaGaleriden', g.dosya)).join('') || '<div class="panel-not">Galeri boş.</div>') : '<div class="panel-not"><div class="donen kucuk"></div> Galeri yükleniyor…</div>';

    const gruplar = {};
    for (const [ad, etiket, grup] of T.DEGISKENLER) (gruplar[grup] ||= []).push([ad, etiket]);
    const renkler = Object.entries(gruplar).map(([g, l]) => `<div class="tema-grup"><h4>${g}</h4><div class="tema-renkler">${l.map(([ad, et]) => `<label class="tema-renk"><input type="color" data-g="temaRenk" data-ad="${ad}" value="${T.onaltilik(t.renkler[ad] || '#000000')}"><span>${et}</span></label>`).join('')}</div></div>`).join('');
    const y = t.yazi || {};
    const secim = (ad, liste, deger) => `<input class="metin-girdi" list="liste-${ad}" data-g="temaYazi" data-ad="${ad}" value="${kac(deger || '')}" placeholder="Varsayılan"><datalist id="liste-${ad}">${liste.map(x => `<option value="${kac(x)}">`).join('')}</datalist>`;
    const kaydirici = (g, ad, deger, min, max, birim) => `<label class="tema-kaydirici"><span>${{ olcek: 'Arayüz ölçeği', saydamlik: 'Panel saydamlığı', bulaniklik: 'Resim bulanıklığı', karartma: 'Resmi karart' }[ad] || ad}</span><input type="range" min="${min}" max="${max}" data-g="${g}" data-ad="${ad}" value="${deger}"><b id="tema-${ad}-deger">${deger}</b>${birim}</label>`;
    const a = t.arka_plan;
    const govde = `<div class="gorunum">
      <div class="gorunum-sol">
        <div class="tema-secim">${[['hazir', 'Hazır'], ['benim', 'Temalarım'], ['galeri', 'Topluluk']].map(([k, ad]) => `<span class="${m.sekme === k ? 'secili' : ''}" data-e="gorunumSekme" data-a="${k}">${ad}</span>`).join('')}</div>
        <div class="tema-liste">${liste}</div>
        <label class="dugme tema-ice">${S('upload_file')}İçe aktar (.ohctema)<input type="file" accept=".ohctema,application/json" data-g="temaIceAktar" hidden></label>
      </div>
      <div class="gorunum-sag">
        <div class="tema-satir"><label>Ad<input class="metin-girdi" data-g="temaAd" value="${kac(t.ad || '')}" maxlength="60"></label><label>Hazırlayan<input class="metin-girdi" data-g="temaYazar" value="${kac(t.yazar || '')}" maxlength="60"></label>
          <label>Taban<select class="metin-girdi" data-g="temaTaban"><option value="koyu" ${t.taban !== 'acik' ? 'selected' : ''}>Koyu</option><option value="acik" ${t.taban === 'acik' ? 'selected' : ''}>Açık</option></select></label></div>
        ${renkler}
        <div class="tema-grup"><h4>Yazı ve biçim</h4>
          <div class="tema-satir"><label>Arayüz yazı tipi${secim('arayuz', T.ARAYUZ_YAZI, y.arayuz)}</label><label>Kod yazı tipi${secim('kod', T.KOD_YAZI, y.kod)}</label></div>
          ${kaydirici('temaYazi', 'olcek', y.olcek || 100, 70, 150, '%')}
          <label class="tema-kaydirici"><span>Köşe yuvarlaklığı</span><input type="range" min="0" max="20" data-g="temaKose" value="${t.kose ?? 8}"><b></b></label>
        </div>
        <div class="tema-grup"><h4>Arka plan</h4>
          ${a ? `<div class="tema-arka-onizleme">${a.tur === 'video' ? `<video src="${a.kaynak}" muted autoplay loop></video>` : `<img src="${a.kaynak}" alt="">`}<div class="esnek">
              <label>Yerleşim<select class="metin-girdi" data-g="temaArka" data-ad="konum">${[['kapla', 'Ekranı kapla'], ['sigdir', 'Sığdır'], ['doseme', 'Döşe'], ['ortala', 'Ortala']].map(([k, ad]) => `<option value="${k}" ${a.konum === k ? 'selected' : ''}>${ad}</option>`).join('')}</select></label>
              <div class="dugme" data-e="temaArkaPlanKaldir">${S('delete')}Kaldır</div></div></div>
            ${kaydirici('temaArka', 'saydamlik', a.saydamlik, 20, 100, '%')}${kaydirici('temaArka', 'bulaniklik', a.bulaniklik, 0, 30, 'px')}${kaydirici('temaArka', 'karartma', a.karartma, 0, 90, '%')}`
          : '<div class="panel-not">Resim, hareketli GIF ya da kısa bir video (en çok 22 MB) arka plan olabilir; paneller saydamlaşır.</div>'}
          <label class="dugme">${S('image')}${a ? 'Başka dosya seç' : 'Resim, GIF ya da video seç'}<input type="file" accept="image/*,video/mp4,video/webm" data-g="temaDosya" hidden></label>
        </div>
        <details class="tema-grup"><summary>Gelişmiş: özel CSS</summary><textarea class="metin-girdi mono" rows="6" data-g="temaCss" spellcheck="false" placeholder=".kod-alani { letter-spacing: .02em; }">${kac(t.ozel_css || '')}</textarea>
          <div class="panel-not" style="font-size:12px">Paylaşılan temalarda dış adres yükleyen kurallar (@import, http) çalışmaz.</div></details>
      </div></div>`;
    const alt = `<div class="dugme" data-e="temaVarsayilan">Varsayılana dön</div><div class="dugme" data-e="temaDisaAktar">${S('download')}Dışa aktar</div><div style="flex:1"></div><div class="dugme" data-e="gorunumKapat">Vazgeç</div><div class="dugme birincil" data-e="temaKaydet">Kaydet ve uygula</div>`;
    return kabuk('Görünüm ve temalar', govde, alt).replace('class="modal"', 'class="modal genis"').replace('data-e="modalKapat"', 'data-e="gorunumKapat"').replace('class="ortu" data-e="modalDis"', 'class="ortu saydam" data-e="hic"');
  }

  /** Yeni sürüm denetimi: sessizdir, hata olursa bir şey göstermez. */
  async function guncellemeyiDenetle() {
    if (!D.guncellemeDenetle) return;
    const r = await api('/api/guncelleme').catch(() => null);
    if (!r || r.hata || r.kapali || !r.yeni) return;
    D.guncelleme = r;
    bildir(`Orhunca ${r.surum} çıktı — güncellemek için başlangıç ekranına bakın.`);
    if (D.ekran === 'baslangic') ciz();
  }

  /** Sürüm notları için küçük Markdown: başlık, liste, kalın, kod, tablo satırları düz yazı. */
  function mdBasit(md) {
    const satir = t => kac(t).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/`([^`]+)`/g, '<code>$1</code>');
    let html = '', liste = false;
    for (const l of md.split(/\r?\n/)) {
      const m = l.match(/^\s*[-*]\s+(.*)/);
      if (m) { if (!liste) { html += '<ul>'; liste = true; } html += `<li>${satir(m[1])}</li>`; continue; }
      if (liste) { html += '</ul>'; liste = false; }
      const b = l.match(/^#{1,4}\s+(.*)/);
      if (b) html += `<h3>${satir(b[1])}</h3>`;
      else if (/^\|?\s*-{3}/.test(l)) continue;
      else if (l.trim()) html += `<p>${satir(l.replace(/^\||\|$/g, '').replace(/\|/g, ' · '))}</p>`;
    }
    return html + (liste ? '</ul>' : '');
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

