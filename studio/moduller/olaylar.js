/* Orhunca Stüdyo — Olay bağlama.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
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
  document.addEventListener('contextmenu', e => {
    const satir = e.target.closest('#satirNolari > div');
    if (!satir) return;
    e.preventDefault();
    kesmeAyarAc(+satir.dataset.a);
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
  // Erişilebilirlik: tıklanabilir (data-e) öğeler klavyeyle odaklanır ve Enter/Boşluk
  // ile çalışır; yalnızca simgeden oluşanlar ekran okuyucuya başlığıyla tanıtılır.
  function erisilebilirYap(kok) {
    kok.querySelectorAll?.('[data-e]:not([tabindex])').forEach(el => {
      if (el.matches('button, a, input, select, textarea') || ['hic', 'modalDis'].includes(el.dataset.e)) return;
      el.setAttribute('tabindex', '0');
      el.setAttribute('role', 'button');
      if (!el.getAttribute('aria-label') && el.title && el.classList.contains('simge')) el.setAttribute('aria-label', el.title);
    });
  }
  new MutationObserver(l => l.forEach(m => m.addedNodes.forEach(n => n.nodeType === 1 && erisilebilirYap(n.parentElement || n))))
    .observe(document.body, { childList: true, subtree: true });
  document.addEventListener('keydown', e => {
    if ((e.key === 'Enter' || e.key === ' ') && e.target.getAttribute?.('role') === 'button' && e.target.dataset.e) {
      e.preventDefault();
      e.target.click();
    }
  });
  document.addEventListener('input', e => {
    const g = e.target.dataset?.g;
    if (g) GIRDI[g]?.(e.target.value, e.target);
  });
  document.addEventListener('keydown', async e => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (e.target.id === 'gitMesaj' && e.key === 'Enter' && ctrl) { e.preventDefault(); EYLEM.gitIsle(); return; }
    if ((e.target.id === 'kesmeKosul' || e.target.id === 'kesmeGunluk') && e.key === 'Enter') { e.preventDefault(); EYLEM.kesmeAyarKaydet(); return; }
    if (e.target.id === 'satirGirdi' && e.key === 'Enter') { e.preventDefault(); EYLEM.satiraGitOnayla(); return; }
    if (e.target.id === 'izlemeEkle' && e.key === 'Enter') {
      const ad = e.target.value.trim();
      if (ad && !D.izlenenler.includes(ad)) { D.izlenenler.push(ad); ayarYaz('izlenenler', D.izlenenler); }
      cizYanPanel(); setTimeout(() => $('#izlemeEkle')?.focus(), 0); return;
    }
    if (e.target.id === 'kabukGirdi' && e.key === 'Enter') { kabukKomutu(e.target.value); e.target.value = ''; return; }
    if (e.target.id === 'kabukGirdi' && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      const g = D.kabukGecmis; if (!g.length) return;
      D.kabukSira = Math.max(0, Math.min(g.length, (D.kabukSira ?? g.length) + (e.key === 'ArrowUp' ? -1 : 1)));
      e.target.value = g[D.kabukSira] || ''; e.preventDefault(); return;
    }
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
    if (e.target.id === 'asistanGirdi' && e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); EYLEM.asistanGonder(); return; }
    if (e.target.id === 'asistanAnahtar' && e.key === 'Enter') { EYLEM.asistanBaglan(); return; }
    if (e.target.id === 'yeniDosyaAdi' && e.key === 'Enter') { EYLEM.yeniDosyaOlustur(); return; }
    if ((e.target.id === 'soruGirdi' || e.target.id === 'soruOnay') && e.key === 'Enter') { e.preventDefault(); EYLEM.soruOnayla(); return; }
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
      if (e.key === 'F2' && e.target.id === 'kodAlani') { e.preventDefault(); EYLEM.yenidenAdlandir(); return; }
      if (e.key === 'F12' && e.shiftKey && e.target.id === 'kodAlani') { e.preventDefault(); EYLEM.basvurulariBul(); return; }
      if (ctrl && e.altKey && (e.key === 's' || e.key === 'S')) { e.preventDefault(); tumunuKaydet({ yenile: true }); return; }
      if (ctrl && (e.key === 's' || e.key === 'S')) { e.preventDefault(); kaydet(); return; }
      if (ctrl && (e.key === 'j' || e.key === 'J')) { e.preventDefault(); EYLEM.altPanelAcKapa(); return; }
      if (ctrl && !e.shiftKey && (e.key === 'i' || e.key === 'I')) { e.preventDefault(); EYLEM.asistanAcKapa(); return; }
      if (ctrl && e.shiftKey && (e.key === 'F' || e.key === 'f')) { e.preventDefault(); EYLEM.bicimlendir(); return; }
      if (ctrl && (e.key === '=' || e.key === '+')) { e.preventDefault(); EYLEM.yaziBuyut(); return; }
      if (ctrl && e.key === '-') { e.preventDefault(); EYLEM.yaziKucult(); return; }
    } else {
      if (e.altKey && (e.key === 's' || e.key === 'S' || e.code === 'KeyS')) { e.preventDefault(); if (D.ekran !== 'baslangic') EYLEM.git('baslangic'); setTimeout(() => $('#q')?.focus()); return; }
      if (ctrl && e.shiftKey && (e.key === 'N' || e.key === 'n')) { e.preventDefault(); EYLEM.git('yeni'); }
    }
  });
  window.addEventListener('resize', () => { if (D.ekran === 'duzenleyici') vurguyuGuncelle(); });
  // Dosyalar Stüdyo dışında (terminal, başka bir düzenleyici) değişmiş olabilir.
  window.addEventListener('focus', () => { if (D.ekran === 'duzenleyici') gitYenilePlanla(); });
  window.addEventListener('beforeunload', e => {
    if (D.sekmeler.some(s => !s.ikili && s.icerik !== s.kayitli)) { e.preventDefault(); e.returnValue = ''; }
  });
  // Sekme kapanırken yanıt bekleyen asistan isteği sunucuda da durdurulur.
  window.addEventListener('pagehide', () => {
    if (!D.asistan.bekliyor) return;
    fetch('/api/asistan/durdur', { method: 'POST', keepalive: true, headers: { 'X-Orhunca-Anahtar': ANAHTAR || '', 'Content-Type': 'application/json' }, body: '{}' }).catch(() => {});
  });


  // İletişim kutusunda odak görünür kalır, Tab dışarı kaçmaz, kapanınca geri döner.
  let oncekiModal = null, modalOncesiOdak = null;
  const modalGozlem = new MutationObserver(() => {
    const modal = document.querySelector('.modal');
    if (modal) {
      if (!oncekiModal) modalOncesiOdak = document.activeElement;
      modal.setAttribute('role', 'dialog'); modal.setAttribute('aria-modal', 'true');
      modal.setAttribute('aria-label', modal.querySelector('h2')?.textContent || 'İletişim kutusu');
      modal.removeAttribute('tabindex');
      modal.querySelectorAll('input,select,textarea').forEach(el => {
        if (!el.labels?.length && !el.getAttribute('aria-label')) el.setAttribute('aria-label', el.placeholder || el.id || 'Girdi');
      });
      if (!modal.contains(document.activeElement)) modal.querySelector('button,input,textarea,select,[tabindex="0"]')?.focus();
    } else if (oncekiModal && modalOncesiOdak?.isConnected) modalOncesiOdak.focus();
    oncekiModal = modal;
  });
  modalGozlem.observe(document.body, {childList:true,subtree:true});
  document.addEventListener('keydown', e => {
    const modal = document.querySelector('.modal');
    if (e.key !== 'Tab' || !modal) return;
    const liste = [...modal.querySelectorAll('button:not(:disabled),a[href],input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]')].filter(el => el.getClientRects().length);
    if (!liste.length) return;
    const ilk = liste[0], son = liste.at(-1);
    if (e.shiftKey && (document.activeElement === ilk || !modal.contains(document.activeElement))) { e.preventDefault(); son.focus(); }
    else if (!e.shiftKey && (document.activeElement === son || !modal.contains(document.activeElement))) { e.preventDefault(); ilk.focus(); }
  });
