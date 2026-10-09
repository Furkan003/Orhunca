/* Orhunca Stüdyo — Ana çizim.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
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

