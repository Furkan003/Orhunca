/* Orhunca Stüdyo — Eklentiler.
 * Kullanıcının kurduğu eklentiler <ayar klasörü>/eklentiler/<ad>/eklenti.js dosyalarıdır;
 * açılışta yüklenir ve window.OrhuncaStudyo.eklenti({...}) ile komut ekler. Komutlar
 * "Eklentiler" menüsünde görünür. Belge: docs/studyo-eklentileri.md
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  const EKLENTI_KOMUTLARI = [];

  /** Eklentilerin kullandığı arayüz: düzenleyici, bildirim, kabuk ve Stüdyo API'si. */
  const EKLENTI_API = Object.freeze({
    /** Açık dosya: { yol, metin } ya da null */
    etkinDosya() {
      const s = etkinSekme();
      return s ? { yol: s.yol, metin: s.icerik } : null;
    },
    /** Seçili metin (yoksa boş) */
    secim() {
      const ta = $('#kodAlani');
      return ta ? ta.value.slice(ta.selectionStart, ta.selectionEnd) : '';
    },
    /** Seçimi (seçim yoksa imlecin yerine) verilen metinle değiştirir; geri alınabilir. */
    seciliyiDegistir(metin) {
      const ta = $('#kodAlani');
      if (!ta) return false;
      ta.focus();
      metinEkle(ta, String(metin));
      return true;
    },
    /** Açık dosyanın tüm metnini değiştirir; geri alınabilir. */
    metniDegistir(metin) {
      const ta = $('#kodAlani');
      if (!ta) return false;
      ta.focus();
      ta.select();
      metinEkle(ta, String(metin));
      return true;
    },
    bildir(mesaj, hata = false) { bildir(String(mesaj), hata); },
    /** KABUK sekmesinde bir komut çalıştırır (ör. "orhunca sına"). */
    kabuk(komut) {
      D.altPanel = true; D.altSekme = 'kabuk'; guncelle('alt');
      return kabukKomutu(String(komut));
    },
    proje() { return D.proje ? { ad: D.proje.ad, yol: D.proje.yol } : null; },
    /** Stüdyo'nun kendi API'si: api('/api/denetle', { dosya }) */
    api(yol, govde) { return api(yol, govde); },
  });

  window.OrhuncaStudyo = Object.freeze({
    surum: 1,
    /** eklenti({ ad, komutlar: [{ ad, calistir(api) }] }) */
    eklenti(tanim) {
      if (!tanim || !Array.isArray(tanim.komutlar)) return;
      for (const k of tanim.komutlar) {
        if (!k || typeof k.calistir !== 'function') continue;
        const no = EKLENTI_KOMUTLARI.length;
        EKLENTI_KOMUTLARI.push({ eklenti: String(tanim.ad || ''), ad: String(k.ad || 'Komut'), calistir: k.calistir });
        EYLEM['eklentiKomutu' + no] = async () => {
          try { await k.calistir(EKLENTI_API); } catch (h) { bildir(`${tanim.ad || 'Eklenti'}: ${h && h.message || h}`, true); }
        };
      }
      eklentiMenusunuKur();
    },
  });

  function eklentiMenusunuKur() {
    let m = MENULER.find(x => x.ad === 'Eklentiler');
    if (!m) {
      m = { ad: 'Eklentiler', ogeler: [] };
      MENULER.splice(MENULER.length - 1, 0, m);
    }
    m.ogeler = EKLENTI_KOMUTLARI.map((k, i) => [k.eklenti ? `${k.eklenti}: ${k.ad}` : k.ad, '', 'eklentiKomutu' + i]);
    if (typeof guncelle === 'function' && D.ekran === 'duzenleyici') guncelle('baslik');
  }

  async function eklentileriYukle() {
    const r = await api('/api/eklentiler').catch(() => ({ eklentiler: [] }));
    for (const e of r.eklentiler || []) {
      const s = document.createElement('script');
      s.src = '/eklenti/' + encodeURIComponent(e.ad) + '.js';
      s.onerror = () => bildir(`Eklenti yüklenemedi: ${e.ad}`, true);
      document.head.appendChild(s);
    }
  }
