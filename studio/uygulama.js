/* Orhunca Stüdyo — arayüz. Bağımlılık yok; `orhunca stüdyo` sunucusuyla konuşur.
 * Bölümler studio/moduller/ altındadır ve index.html'deki sırayla yüklenir; bu dosya
 * en son yüklenir ve uygulamayı başlatır. */
'use strict';
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
