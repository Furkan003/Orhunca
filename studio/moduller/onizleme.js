/* Orhunca Stüdyo — Canlı önizleme (web projeleri).
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
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
    // Çalıştırılmadan önce adres bilinmez (önceki projenin adresi gösterilmez).
    const kok = o.arayuz || (!o.adres && D.proje?.arayuz) ? `arayüz · ${D.proje?.ad || ''}` : o.adres ? o.adres.replace(/^https?:\/\//, '') : o.kapi ? `localhost:${o.kapi}` : 'F5 ile başlatın';
    $('#onizlemeAdres').innerHTML = `<span class="nokta ${durum}"></span><span class="adres">${kac(kok + (o.yol && o.yol !== '/' ? o.yol : ''))}</span>`;
    const g = $('#onizlemeGovde');
    if (o.adres) {
      const cerceve = $('#onizlemeCerceve');
      if (!cerceve || cerceve.dataset.surum !== String(o.surum)) {
        // Arayüz programları Stüdyo'nun kendi sunucusundan gelir: Stüdyo'ya erişemesinler diye yalıtılır.
        const yalit = o.arayuz ? ' sandbox="allow-scripts allow-forms allow-modals allow-popups"' : '';
        g.innerHTML = `<iframe id="onizlemeCerceve" data-surum="${o.surum}" src="${kac(onizlemeAdresi())}" title="Canlı önizleme" allow="camera; geolocation"${yalit}></iframe>`;
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

  // Asistan panelinin sol kenarı sürüklenerek genişletilir.
  document.addEventListener('mousedown', e => {
    if (!e.target.classList?.contains('asistan-tutamac')) return;
    e.preventDefault();
    const panel = $('#asistanKap'), c = $('#onizlemeCerceve');
    if (c) c.style.pointerEvents = 'none';
    const tasi = h => {
      const sag = window.innerWidth - panel.getBoundingClientRect().right;
      panel.style.width = Math.round(Math.max(300, Math.min(window.innerWidth - h.clientX - sag, window.innerWidth * 0.6))) + 'px';
    };
    const birak = () => {
      document.removeEventListener('mousemove', tasi);
      document.removeEventListener('mouseup', birak);
      if (c) c.style.pointerEvents = '';
      ayarYaz('asistanGenisligi', panel.getBoundingClientRect().width);
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

