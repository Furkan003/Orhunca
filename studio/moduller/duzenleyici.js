/* Orhunca Stüdyo — Düzenleyici.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
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
              <div class="kisitli-serit gizli" id="kisitliSerit"></div>
              <div class="kirinti" id="kirinti"></div>
              <div id="kodBolge" style="flex:1;min-height:0;display:flex;flex-direction:column"></div>
              <div class="alt-panel" id="altPanel"></div>
            </div>
            <div class="onizleme gizli" id="onizleme"></div>
            <div class="asistan-kap gizli" id="asistanKap"></div>
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
    if (p('asistan')) cizAsistan();
    if (p('katman') || hepsi) katmanlariCiz();
  }

  function cizEtkinlik() {
    const ogeler = [['gezgin', 'description', 'Gezgin'], ['ara', 'search', 'Ara'], ['yapi', 'account_tree', 'Yapı'], ['git', 'fork_right', 'Git'], ['calistir', 'play_circle', 'Çalıştır'], ['sinamalar', 'task_alt', 'Sınamalar'], ['veritabani', 'database', 'Veritabanı'], ['dersler', 'school', 'Dersler'], ['eklentiler', 'extension', 'Paketler']];
    $('#etkinlik').innerHTML = ogeler.map(([id, simge, ad]) => `<span class="simge ${D.yanPanel === id ? 'etkin' : ''}" title="${ad}" data-e="yanPanelSec" data-a="${id}">${simge}</span>`).join('')
      + (asistanAcik() ? `<span class="simge ${D.asistanPaneliAcik ? 'etkin' : ''}" title="Yapay zekâ asistanı (Ctrl+I)" data-e="asistanAcKapa">smart_toy</span>` : '')
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
    kap.classList.toggle('genis', D.yanPanel === 'dersler');
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
        <div class="yan-yana ara-degistir"><input id="degistirMetin" data-g="degistirMetin" class="metin-girdi" placeholder="Şununla değiştir" value="${kac(D.degistirMetin)}" spellcheck="false" autocomplete="off"><div class="kare-dugme simge ${D.araMetin ? '' : 'pasif'}" data-e="tumunuDegistir" title="Projedeki bütün dosyalarda değiştir">find_replace</div></div>
        <label class="ara-secenek"><input type="checkbox" data-e="tamKelimeDegistir" ${D.tamKelime ? 'checked' : ''}>Yalnızca tam kelime</label>
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
    } else if (D.yanPanel === 'git') {
      const ek = D.git?.depo ? `<span class="simge" title="Çek (pull)" data-e="gitCek">arrow_downward</span><span class="simge" title="Gönder (push)" data-e="gitGonder">arrow_upward</span>` : '';
      kap.innerHTML = baslik('GIT', ek + `<span class="simge" title="Yenile" data-e="gitYukle">refresh</span>`) + `<div class="panel-ic">${gitPaneli()}</div>`;
    } else if (D.yanPanel === 'sinamalar') {
      kap.innerHTML = baslik('SINAMALAR', `<span class="simge" title="Hepsini çalıştır" data-e="sinamalariCalistir">play_arrow</span><span class="simge" title="Yenile" data-e="sinamalariYukle">refresh</span>`) + `<div class="panel-ic">${sinamaPaneli()}</div>`;
    } else if (D.yanPanel === 'veritabani') {
      kap.innerHTML = baslik('VERİTABANI', `<span class="simge" title="SQL sorgusu" data-e="vtSorguAc">terminal</span><span class="simge" title="Yenile" data-e="vtYukle">refresh</span>`) + `<div class="panel-ic">${vtPaneli()}</div>`;
    } else if (D.yanPanel === 'dersler') {
      kap.innerHTML = baslik('DERSLER') + `<div class="panel-ic ders-panel">${dersPaneli()}</div>`;
      kap.querySelectorAll('pre[data-orhunca]').forEach(p => { p.innerHTML = p.textContent.split('\n').map(x => vurgula(x, 'ohc')).join('\n'); });
    } else if (D.yanPanel === 'calistir') {
      const giris = girisDosyasi();
      const altlar = D.proje.alt_projeler || [];
      const secim = altlar.length ? `<div class="panel-not">Başlangıç projesi<br><select class="metin-girdi" data-g="baslangicSec" style="width:100%;margin-top:4px">
          <option value="">${kac(D.proje.ad)} (kök)</option>${altlar.map((a, i) => `<option value="${i}" ${D.baslangic === i ? 'selected' : ''}>${kac(a.ad)} — ${kac(a.yol)}</option>`).join('')}</select></div>` : '';
      kap.innerHTML = baslik('ÇALIŞTIR') + `<div class="panel-ic">${secim}
        <div class="panel-not">Giriş dosyası<br><span class="mono" style="color:var(--yazi2)">${kac(giris || '—')}</span></div>
        ${D.calisma ? `<div class="panel-dugme" data-e="durdur">${S('stop')}Durdur</div>` : `<div class="panel-dugme birincil" data-e="calistir">${S('play_arrow')}Çalıştır (F5)</div><div class="panel-dugme" data-e="ayikla">${S('bug_report')}Hata ayıkla (F6)</div><div class="panel-dugme" data-e="yavasCalistir">${S('slow_motion_video')}Adım adım göster</div><div class="panel-dugme" data-e="profilCikar" title="Hangi işlev ve satır ne kadar sürüyor (KABUK sekmesinde)">${S('speed')}Profil çıkar</div>`}
        ${D.calisma?.ayikla ? ayiklamaPaneli() : ''}
        <div class="alan" style="gap:6px"><label style="font-size:12px">Program argümanları</label><input id="argumanlar" data-g="argumanlar" class="metin-girdi" placeholder="ör. bir iki" value="${kac(D.argumanlar)}" spellcheck="false"></div>
        <div class="panel-dugme" data-e="denetleKomut">${S('task_alt')}Denetle (F7)</div>
        <div class="panel-dugme" data-e="ceviriPython">${S('translate')}Python / JavaScript karşılığı</div>
        <div class="ince-ayrac"></div>
        <div class="panel-not">Dağıtım için derle (proje/cikti/)</div>
        <div class="panel-dugme" data-e="derleLinux">${S('terminal')}Linux için derle</div>
        <div class="panel-dugme" data-e="derleWindows">${S('desktop_windows')}Windows için derle</div>
        <div class="panel-dugme" data-e="derleWeb">${S('language')}Web için derle</div>
        <div class="panel-not">Arayüz programını masaüstü ya da telefon uygulamasına paketle</div>
        <div class="panel-dugme" data-e="paketleLinux">${S('select_window')}Masaüstü (Linux)</div>
        <div class="panel-dugme" data-e="paketleWindows">${S('select_window')}Masaüstü (Windows)</div>
        <div class="panel-dugme" data-e="paketleAndroid">${S('smartphone')}Telefon (Android .apk)</div>
        <div class="panel-dugme" data-e="paketleIos">${S('smartphone')}Telefon (iPhone, Xcode projesi)</div>
      </div>`;
    } else {
      const izinRozeti = iz => (Array.isArray(iz) ? iz : String(iz || '').split(',').map(x => x.trim()).filter(Boolean)).map(x => `<span class="paket-izin" title="Bu paketin izni">${kac(x)}</span>`).join('');
      const liste = D.paketler.map(p => `<div class="paket-oge" title="${kac(p.kaynak)}">${S('deployed_code')}<div class="esnek"><div class="paket-ad">${kac(p.ad)}</div><div class="paket-kaynak">${kac(p.kaynak)}</div><div class="paket-kaynak">${p.kurulu ? (p.isleme || '').slice(0, 10) : '<span style="color:var(--sari)">kurulu değil</span>'}</div><div>${izinRozeti(p.izinler)}</div></div><span class="simge sil" title="Kaldır" data-e="paketKaldir" data-a="${kac(p.ad)}">delete</span></div>`).join('');
      const kurulu = new Set(D.paketler.map(p => p.ad));
      const dizin = D.paketDizini == null
        ? (D.paketDizinHatasi ? `<div class="panel-not" style="font-size:12px">Paket dizinine ulaşılamadı: ${kac(D.paketDizinHatasi)}</div>` : '<div class="panel-not">Paket dizini yükleniyor…</div>')
        : D.paketDizini.filter(p => !kurulu.has(p.ad)).map(p => `<div class="paket-oge" title="${kac(p.kaynak)}">${S('deployed_code')}<div class="esnek"><div class="paket-ad">${kac(p.ad)}${p.surum ? ` <span class="paket-kaynak">${kac(p.surum)}</span>` : ''}</div><div class="paket-kaynak" style="white-space:normal">${kac(p.aciklama)}</div><div>${izinRozeti(p.izinler)}${p.sahip ? `<span class="paket-kaynak"> · ${kac(p.sahip)}</span>` : ''}</div></div><span class="simge" title="Ekle" data-e="paketDizindenEkle" data-a="${kac(p.ad)}">add</span></div>`).join('') || '<div class="panel-not">Dizindeki bütün paketler ekli.</div>';
      kap.innerHTML = baslik('PAKETLER', D.paketMesgul ? '<div class="donen kucuk"></div>' : `<span class="simge" title="Yenile" data-e="paketleriYenile">refresh</span>`) + `<div class="panel-ic">
        ${liste || '<div class="panel-not">Bu projenin paketi yok. Bir Git deposundan Orhunca kütüphanesi ekleyin; kodda <span class="mono">kullan "paket_adı"</span> ile kullanılır.</div>'}
        <div class="panel-not" style="font-size:11px;letter-spacing:.06em;margin-top:8px">PAKET DİZİNİ</div>
        ${dizin}
        <div class="alan" style="gap:6px"><label style="font-size:12px">Paket adı ya da Git adresi</label><input id="paketKaynagi" data-g="paketKaynagi" class="metin-girdi" placeholder="istatistik ya da github:kişi/depo#v1.0" value="${kac(D.paketKaynagi)}" spellcheck="false"></div>
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
    cizKisitliSerit();
  }

  /** Güvenilmeyen proje (kısıtlı mod) şeridi: kod okunup düzenlenebilir, çalıştırılamaz. */
  function cizKisitliSerit() {
    const el = $('#kisitliSerit');
    if (!el) return;
    const kisitli = D.proje?.guvenilir === false && !D.kisitliSeritKapali;
    el.classList.toggle('gizli', !kisitli);
    el.innerHTML = kisitli ? `${S('lock')}<span><b>Kısıtlı mod.</b> Bu projeyi Stüdyo'da siz oluşturmadınız. Kodu okuyup düzenleyebilirsiniz; çalıştırma, hata ayıklama, derleme, paket kurma ve asistanın kod çalıştırması projeye güvenene kadar kapalı.</span>
      <div class="dugme birincil" data-e="projeyeGuven">Projeye güven</div><span class="simge kapat" data-e="kisitliSeritKapat" title="Gizle">close</span>` : '';
  }

  async function projeyeGuven(guven = true) {
    const r = await api('/api/proje/guven', { yol: D.proje.yol, guven }).catch(e => ({ hata: e.message }));
    if (r.hata) { bildir(r.hata, true); return false; }
    D.proje.guvenilir = r.guvenilir;
    cizKisitliSerit(); guncelle('durum');
    if (D.yanPanel === 'git') gitYukle();
    if (guven) bildir('Projeye güvenildi; kod çalıştırılabilir.');
    return r.guvenilir === guven;
  }

  /** Kısıtlı modda kod çalıştıran bir işlemden önce kullanıcıya sorulur. */
  async function guvenSor(islem) {
    if (!D.proje || D.proje.guvenilir !== false) return true;
    if (!(await onayla(`Bu proje kısıtlı modda: Stüdyo'da siz oluşturmadınız (indirildi, kopyalandı ya da başka bir yerden açıldı).\n\n${islem} için projeye güvenmeniz gerekir. Güvenilen bir projenin kodu bu bilgisayarda çalışır; dosyalarınıza ve internete erişebilir.\n\nKodu okuduysanız ve kaynağını tanıyorsanız güvenin. Projeye güvenilsin mi?`, { baslik: 'Kısıtlı mod', dugme: 'Projeye güven' }))) return false;
    return projeyeGuven(true);
  }

  // ---- Dersler: dersler.json (dersler/*.md'den üretilir), alıştırmalar ~/Orhunca/Dersler
  // projesinde yapılır ve "Kontrol et" ile denetlenir.
  let DERSLER = null;
  async function dersleriYukle() {
    if (!DERSLER) DERSLER = await fetch('dersler.json').then(r => r.json()).catch(() => []);
    return DERSLER;
  }
  const dersTamam = () => ayarOku('derslerTamam', {});
