/* Orhunca Stüdyo — Eylemler.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
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
      // Web projelerinde önizleme her zaman açık başlar; F5 sayfayı sağda gösterir.
      D.onizlemeAcik = true;
      projeyiAc(r, { ilkCalistirma: web ? D.secenekler.canli : D.secenekler.calistir });
    },
    ilkProgram() { D.secili = 'konsol'; if (!D.adDokunuldu) D.projeAdi = 'ilk_programim'; EYLEM.git('yapilandir'); },

    // modallar
    modalKapat() { if (D.modal?.tur === 'soru') { soruBitir(null); return; } D.modal = null; katmanlariCiz(); },
    soruOnayla() { const m = D.modal; if (m?.tur === 'soru') soruBitir(m.girdi !== null ? ($('#soruGirdi')?.value ?? '') : ''); },
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
    temaSec(t) { D.tema = t; ayarYaz('tema', t); temayiSec(null, null); katmanlariCiz(); },
    async gorunumModal() {
      D.menu = null;
      const T = window.OrhuncaTema;
      const taban = D.ozelTema || T.HAZIR[document.documentElement.dataset.tema === 'acik' ? 1 : 0];
      D.temaTaslak = JSON.parse(JSON.stringify(taban));
      if (D.temaTaslak.resimli && D.ozelTema?.arka_plan?.kaynak) D.temaTaslak.arka_plan = { ...D.ozelTema.arka_plan };
      if (!D.temaTaslak.arka_plan?.kaynak) D.temaTaslak.arka_plan = null;
      D.temaTaslak.renkler = T.hesaplanan(D.temaTaslak);
      D.modal = { tur: 'gorunum', sekme: 'hazir' };
      katmanlariCiz();
      const r = await api('/api/temalar').catch(() => ({}));
      D.temalarim = r.temalar || [];
      if (D.modal?.tur === 'gorunum') katmanlariCiz();
    },
    gorunumSekme(s) {
      D.modal.sekme = s; katmanlariCiz();
      if (s === 'galeri' && !D.galeri) galeriyiYukle();
    },
    temaHazir(i) {
      const T = window.OrhuncaTema;
      D.temaTaslak = JSON.parse(JSON.stringify(T.HAZIR[+i]));
      D.temaKimlikTaslak = null;
      temaUygula();
      D.temaTaslak.renkler = T.hesaplanan(D.temaTaslak);
      katmanlariCiz();
    },
    async temaBenim(kimlik) {
      const r = await api('/api/tema?' + sorgu({ kimlik })).catch(e => ({ hata: e.message }));
      if (r.hata) return bildir(r.hata, true);
      try { D.temaTaslak = window.OrhuncaTema.dogrula(r.tema); } catch (e) { return bildir(e.message, true); }
      D.temaKimlikTaslak = kimlik;
      temaUygula(); D.temaTaslak.renkler = window.OrhuncaTema.hesaplanan(D.temaTaslak); katmanlariCiz();
    },
    async temaSil(kimlik) {
      if (!(await onayla('Bu tema silinsin mi?', { dugme: 'Sil', tehlikeli: true }))) return;
      await api('/api/tema/sil', { kimlik }).catch(() => null);
      if (D.temaKimlik === kimlik) temayiSec(null, null);
      const r = await api('/api/temalar').catch(() => ({}));
      D.temalarim = r.temalar || [];
      katmanlariCiz();
    },
    async temaGaleriden(dosya) {
      const r = await api('/api/tema/galeriden', { dosya }).catch(e => ({ hata: e.message }));
      if (r.hata) return bildir(r.hata, true);
      try { D.temaTaslak = window.OrhuncaTema.dogrula(r.tema); } catch (e) { return bildir(e.message, true); }
      D.temaKimlikTaslak = null;
      temaUygula(); D.temaTaslak.renkler = window.OrhuncaTema.hesaplanan(D.temaTaslak); katmanlariCiz();
      bildir(`“${D.temaTaslak.ad}” önizlemede; beğendiyseniz Kaydet ve uygula'ya basın.`);
    },
    temaArkaPlanKaldir() { D.temaTaslak.arka_plan = null; temaUygula(); katmanlariCiz(); },
    temaVarsayilan() { D.temaTaslak = null; D.modal = null; temayiSec(null, null); katmanlariCiz(); bildir('Varsayılan görünüme dönüldü.'); },
    gorunumKapat() { D.temaTaslak = null; D.modal = null; temaUygula(); katmanlariCiz(); },
    async temaKaydet() {
      const T = window.OrhuncaTema;
      let t;
      try { t = T.dogrula(D.temaTaslak); } catch (e) { return bildir(e.message, true); }
      const hazirMi = T.HAZIR.some(h => h.ad === t.ad);
      const r = hazirMi && !t.arka_plan && !Object.keys(D.temaTaslak._degisti || {}).length
        ? { kimlik: null }
        : await api('/api/tema/kaydet', { kimlik: D.temaKimlikTaslak || undefined, tema: t }).catch(e => ({ hata: e.message }));
      if (r.hata) return bildir(r.hata, true);
      D.temaTaslak = null; D.modal = null;
      temayiSec(t, r.kimlik);
      katmanlariCiz();
      if (D.ekran === 'duzenleyici') cizKod();
      bildir(`“${t.ad}” uygulandı.`);
    },
    async temaDisaAktar() {
      const T = window.OrhuncaTema;
      let t;
      try { t = T.dogrula(D.temaTaslak); } catch (e) { return bildir(e.message, true); }
      if (window.__TAURI__) {
        const r = await api('/api/tema/disa_aktar', { tema: t }).catch(e => ({ hata: e.message }));
        return r.hata ? bildir(r.hata, true) : bildir('Kaydedildi: ' + r.yol);
      }
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([JSON.stringify(t, null, 2)], { type: 'application/json' }));
      a.download = (t.ad || 'tema').replace(/[^\p{L}\p{N} _-]/gu, '') + '.ohctema';
      document.body.appendChild(a); a.click(); a.remove();
      setTimeout(() => URL.revokeObjectURL(a.href), 5000);
    },
    asistanAcKapa() {
      if (!asistanAcik()) return;
      D.asistanPaneliAcik = !D.asistanPaneliAcik; ayarYaz('asistanPaneliAcik', D.asistanPaneliAcik);
      guncelle('baslik', 'etkinlik', 'asistan');
      if (D.asistanPaneliAcik) {
        if (D.asistan.durum?.hazir && !D.asistan.modeller) EYLEM.asistanModelleriYukle();
        ($('#asistanGirdi') || $('#asistanAnahtar'))?.focus();
      } else $('#kodAlani')?.focus();
    },
    asistanSaglayiciSec(k) { const a = D.asistan; a.secilen = k; a.anahtar = ''; a.adres = null; a.hata = ''; cizAsistan(); },
    asistanAyarAc() { const a = D.asistan; a.ayarAcik = true; a.secilen = null; a.hata = ''; cizAsistan(); },
    asistanAyarKapat() { const a = D.asistan; a.ayarAcik = false; a.secilen = null; a.anahtar = ''; a.adres = null; a.hata = ''; cizAsistan(); },
    async asistanBaglan() {
      const a = D.asistan, p = asistanSaglayici();
      if (a.baglaniyor) return;
      const govde = { saglayici: p.kimlik };
      if (a.anahtar.trim()) govde.anahtar = a.anahtar.trim();
      if (a.adres !== null) govde.adres = a.adres.trim();
      if (p.anahtar_gerekli && !p.anahtar_var && !govde.anahtar) { a.hata = 'Önce API anahtarını yapıştırın.'; return cizAsistan(); }
      a.baglaniyor = true; a.hata = ''; cizAsistan();
      const r = await api('/api/asistan/ayar', govde).catch(e => ({ hata: e.message }));
      if (r.hata) { a.baglaniyor = false; a.hata = r.hata; return cizAsistan(); }
      a.durum = r; a.anahtar = ''; a.adres = null; a.modeller = null;
      const tamam = await EYLEM.asistanModelleriYukle(true);
      a.baglaniyor = false;
      if (tamam) { a.ayarAcik = false; a.secilen = null; }
      cizAsistan();
      if (tamam) $('#asistanGirdi')?.focus();
    },
    async asistanAnahtarSil() {
      if (!(await onayla('API anahtarı bu bilgisayardan silinsin mi?', { dugme: 'Sil', tehlikeli: true }))) return;
      const p = asistanSaglayici();
      D.asistan.durum = await api('/api/asistan/ayar', { saglayici: p.kimlik, anahtar: '' });
      D.asistan.modeller = null; D.asistan.secilen = p.kimlik; D.asistan.ayarAcik = true;
      cizAsistan();
    },
    /** Model listesini yükler; bağlantı denemesi olarak da kullanılır. */
    async asistanModelleriYukle(sessiz) {
      const a = D.asistan;
      const r = await api('/api/asistan/modeller').catch(e => ({ hata: e.message }));
      if (r.hata) { a.hata = r.hata; if (sessiz !== true) cizAsistan(); return false; }
      a.modeller = r.modeller; a.hata = '';
      if (!a.modeller.length && a.durum.saglayici === 'ollama') {
        a.hata = 'Ollama çalışıyor ama indirilmiş model yok. Terminalde bir model indirin, ör.: ollama pull qwen2.5-coder:7b';
        if (sessiz !== true) cizAsistan();
        return false;
      }
      // İlk kurulumda önerilen model (yerelde belleğe sığan), yoksa listenin başındaki (çoğunlukla en yeni) seçilir.
      if (!a.durum.model && a.modeller.length) a.durum = await api('/api/asistan/ayar', { model: (a.modeller.find(m => m.varsayilan) || a.modeller[0]).kimlik });
      if (sessiz !== true) cizAsistan();
      return true;
    },
    async ajanBagla() {
      const r = await api('/api/ajan').catch(() => ({ komut: 'orhunca' }));
      D.modal = { tur: 'ajan', komut: r.komut || 'orhunca', talimat: r.talimat || '', istemci: ayarOku('ajanIstemci', 'claude-code') };
      katmanlariCiz();
    },
    ajanIstemci(k) { D.modal.istemci = k; ayarYaz('ajanIstemci', k); katmanlariCiz(); },
    ajanKopyala() {
      const m = D.modal, i = ajanIstemcileri(m.komut).find(x => x.kimlik === m.istemci);
      navigator.clipboard?.writeText(i?.kod || '').then(() => bildir('Kopyalandı.'), () => bildir('Kopyalanamadı.', true));
    },
    async ajanTalimatEkle() {
      const m = D.modal;
      if (!D.proje) return bildir('Önce bir proje açın.', true);
      const yol = tamYol('AGENTS.md');
      const varmi = await api('/api/dosya?yol=' + encodeURIComponent(yol)).then(r => !r.hata).catch(() => false);
      if (varmi && !(await onayla('Projede zaten bir AGENTS.md var. Üzerine yazılsın mı?', { dugme: 'Üzerine yaz' }))) return;
      if (!varmi) { const r = await api('/api/dosya/yeni', { yol, klasor: false }); if (r.hata) return bildir(r.hata, true); }
      const r = await api('/api/dosya', { yol, icerik: m.talimat });
      if (r.hata) return bildir(r.hata, true);
      bildir('AGENTS.md projeye eklendi.');
      EYLEM.agaciYenile();
    },
    disAdresAc(a) {
      if (TAURI) api('/api/tarayicida_ac', { adres: a }).catch(e => bildir(e.message, true));
      else window.open(a, '_blank', 'noopener');
    },
    async asistanGonder() {
      const a = D.asistan, metin = a.girdi.trim();
      if (!metin || a.bekliyor) return;
      a.mesajlar.push({ rol: 'kullanici', metin });
      a.girdi = ''; a.hata = ''; a.bekliyor = true;
      const istek = ++a.istek;
      cizAsistan();
      const s = etkinSekme();
      const govde = { mesajlar: a.mesajlar.map(m => ({ rol: m.rol, metin: m.metin + (m.oneri ? '\n\n[Önerilen dosya içeriği:]\n' + m.oneri.icerik : '') })) };
      if (s && !s.ikili) { govde.dosya = s.yol; govde.icerik = s.icerik; a.mesajlar.at(-1).dosya = s.yol; }
      if (D.proje) govde.proje = D.proje.yol;
      const r = await api('/api/asistan/sor', govde).catch(e => ({ hata: e.message }));
      if (istek !== a.istek) return;
      a.bekliyor = false;
      if (r.hata) { a.hata = r.hata; a.mesajlar.pop(); a.girdi = metin; }
      else a.mesajlar.push({ rol: 'asistan', metin: r.yanit, adimlar: r.adimlar, oneri: r.oneri, oneri_denetim: r.oneri_denetim, bloklar: r.bloklar, aracsiz: r.aracsiz, dosya: s?.yol });
      cizAsistan(); $('#asistanGirdi')?.focus();
    },
    asistanDurdur() {
      const a = D.asistan; a.istek++; a.bekliyor = false; const son = a.mesajlar.pop(); a.girdi = son?.metin || ''; cizAsistan();
      // Sunucudaki model isteği de kesilir; yoksa yerel model yanıtı üretmeyi sürdürüp sonraki soruları bekletir.
      api('/api/asistan/durdur', {}).catch(() => {});
    },
    async ceviri(dil = 'python') {
      const s = etkinSekme();
      if (!s || s.ikili || uzanti(s.yol) !== 'ohc') return bildir('Önce bir Orhunca (.ohc) dosyası açın.', true);
      D.modal = { tur: 'ceviri', dil, yukleniyor: true, kaynak: s.icerik.split('\n') };
      katmanlariCiz();
      const r = await api('/api/cevir', { dosya: tamYol(s.yol), icerik: s.icerik, dil }).catch(e => ({ hata: e.message }));
      if (D.modal?.tur !== 'ceviri') return;
      Object.assign(D.modal, { yukleniyor: false, hata: r.hata || '', satirlar: r.satirlar || [] });
      katmanlariCiz();
    },
    ceviriDil(d) { EYLEM.ceviri(d); },
    ceviriPython() { EYLEM.ceviri('python'); },
    ceviriJs() { EYLEM.ceviri('javascript'); },
    ceviriKopyala() {
      const m = D.modal;
      navigator.clipboard?.writeText((m.satirlar || []).map(x => x.m).join('\n') + '\n').then(() => bildir('Kopyalandı.'), () => bildir('Kopyalanamadı.', true));
    },
    asistanHazir(metin) { D.asistan.girdi = metin; EYLEM.asistanGonder(); },
    asistanYeni() { const a = D.asistan; a.istek++; a.mesajlar = []; a.bekliyor = false; a.hata = ''; cizAsistan(); },
    async asistanUygula(i) {
      const m = D.asistan.mesajlar[+i];
      if (!m?.oneri) return;
      if (await asistanYaz(m, m.oneri.icerik)) { m.uygulandi = true; cizAsistan(); }
    },
    async asistanBlokUygula(a) {
      const [i, n] = a.split(':').map(Number), m = D.asistan.mesajlar[i];
      if (m) await asistanYaz(m, asistanBlogu(m.metin || '', n));
    },
    asistanBlokKopyala(a) {
      const [i, n] = a.split(':').map(Number), m = D.asistan.mesajlar[i];
      navigator.clipboard?.writeText(asistanBlogu(m?.metin || '', n)).then(() => bildir('Kopyalandı.'), () => bildir('Kopyalanamadı.', true));
    },
    asistanKopyala(i) {
      const m = D.asistan.mesajlar[+i];
      navigator.clipboard?.writeText(m?.oneri?.icerik || '').then(() => bildir('Kopyalandı.'), () => bildir('Kopyalanamadı.', true));
    },
    asistanGosterDegistir() { D.asistanGoster = !D.asistanGoster; ayarYaz('asistanGoster', D.asistanGoster); katmanlariCiz(); if (D.ekran === 'duzenleyici') guncelle('baslik', 'etkinlik', 'asistan'); },
    hataBildirModal() { D.menu = null; D.modal = { tur: 'hataBildir', ne: '', kod: true, cikti: true }; katmanlariCiz(); },
    hataBildirSec(a) { D.modal[a] = !D.modal[a]; katmanlariCiz(); },
    hataBildirGonder() {
      const m = D.modal;
      if (!(m.ne || '').trim()) return bildir('Önce ne olduğunu kısaca yazın.', true);
      EYLEM.disAdresAc(hataBildirAdresi(m));
      D.modal = null; katmanlariCiz();
      bildir('GitHub sayfası açıldı; göz atıp "Submit new issue" ile gönderin. Teşekkürler!');
    },
    otomatikKaydetDegistir() { D.otomatikKaydet = !D.otomatikKaydet; ayarYaz('otomatikKaydet', D.otomatikKaydet); katmanlariCiz(); },
    async gecmisModal() {
      const s = etkinSekme();
      if (!s || s.ikili) return bildir('Önce bir dosya açın.', true);
      D.menu = null;
      D.modal = { tur: 'gecmis', yol: s.yol, kayitlar: null, secili: null, icerik: null };
      katmanlariCiz();
      const r = await api('/api/gecmis?' + sorgu({ yol: tamYol(s.yol) })).catch(e => ({ hata: e.message }));
      if (D.modal?.tur !== 'gecmis') return;
      if (r.hata) { D.modal = null; katmanlariCiz(); return bildir(r.hata, true); }
      D.modal.kayitlar = r.kayitlar || [];
      katmanlariCiz();
    },
    async gecmisSec(zaman) {
      const m = D.modal;
      m.secili = zaman; m.icerik = null; katmanlariCiz();
      const r = await api('/api/gecmis/oku?' + sorgu({ yol: tamYol(m.yol), zaman })).catch(e => ({ hata: e.message }));
      if (D.modal !== m || m.secili !== zaman) return;
      if (r.hata) return bildir(r.hata, true);
      m.icerik = r.icerik; katmanlariCiz();
    },
    /** Shift+F12: imlecin üzerindeki ismin projedeki başvuruları Ara panelinde listelenir. */
    async basvurulariBul() {
      const s = etkinSekme();
      if (!s || s.ikili || !s.yol.endsWith('.ohc')) return;
      if (!(await tumunuKaydet())) return;
      const r = await api('/api/basvurular', { dosya: tamYol(s.yol), satir: D.imlec.satir - 1, sutun: D.imlec.sutun - 1 }).catch(e => ({ hata: e.message }));
      if (r.hata) return bildir(r.hata, true);
      D.araMetin = r.ad; D.araSonuc = r.sonuclar; D.yanPanel = 'ara';
      guncelle('etkinlik', 'yan');
      bildir(`“${r.ad}”: ${r.sonuclar.length} başvuru`);
    },
    /** F2: ismi projenin bütün dosyalarında yeniden adlandırır (ekler ünlü uyumuna göre). */
    async yenidenAdlandir() {
      const s = etkinSekme();
      if (!s || s.ikili || !s.yol.endsWith('.ohc')) return;
      if (!(await tumunuKaydet())) return;
      const konum = { dosya: tamYol(s.yol), satir: D.imlec.satir - 1, sutun: D.imlec.sutun - 1 };
      const b = await api('/api/basvurular', konum).catch(e => ({ hata: e.message }));
      if (b.hata) return bildir(b.hata, true);
      const dosyalar = new Set(b.sonuclar.map(x => x.dosya)).size;
      const yeni = ((await metinSor(`“${b.ad}” için yeni ad (${b.sonuclar.length} yer, ${dosyalar} dosya; ekler yeni ada göre düzelir):`, b.ad, { baslik: 'Yeniden adlandır', dugme: 'Adlandır' })) || '').trim();
      if (!yeni || yeni === b.ad) return;
      const r = await api('/api/adlandir', { ...konum, yeni }).catch(e => ({ hata: e.message }));
      if (r.hata) return bildir(r.hata, true);
      for (const yol of r.degisen) {
        const t = D.sekmeler.find(x => x.yol === yol);
        if (!t) continue;
        const d = await api('/api/dosya?' + sorgu({ yol: tamYol(yol) }));
        if (!d.hata && !d.ikili) t.icerik = t.kayitli = d.icerik;
      }
      fiilleriTopla();
      guncelle('sekmeler', 'kod', 'yan');
      denetle();
      // Ara panelindeki sonuçlar eski adla kalmasın.
      if (D.araMetin) GIRDI.araMetin(D.araMetin === r.ad ? r.yeni : D.araMetin);
      bildir(`“${r.ad}” → “${r.yeni}”: ${r.sayi} yer, ${r.degisen.length} dosya (önceki hâller yerel geçmişte).`);
    },
    tamKelimeDegistir(_, el) { D.tamKelime = el.checked; GIRDI.araMetin(D.araMetin); },
    async tumunuDegistir() {
      const aranan = D.araMetin, yeni = D.degistirMetin || '';
      if (!aranan || !D.proje) return;
      const dosyaSayisi = new Set(D.araSonuc.map(r => r.dosya)).size;
      if (!(await onayla(`Projedeki bütün dosyalarda “${aranan}” → “${yeni}” olarak değiştirilsin mi?${dosyaSayisi ? ` (${dosyaSayisi} dosya)` : ''}\n\nDosyaların önceki hâlleri yerel geçmişte saklanır.`, { baslik: 'Tümünü değiştir', dugme: 'Değiştir' }))) return;
      if (!(await tumunuKaydet())) return;
      const r = await api('/api/degistir', { kok: D.proje.yol, aranan, yeni, tamKelime: D.tamKelime }).catch(e => ({ hata: e.message }));
      if (r.hata) { bildir(r.hata, true); return; }
      // Açık sekmeler diskteki yeni hâliyle yenilenir.
      for (const yol of r.degisen) {
        const s = D.sekmeler.find(x => x.yol === yol);
        if (!s) continue;
        const d = await api('/api/dosya?' + sorgu({ yol: tamYol(yol) }));
        if (!d.hata && !d.ikili) s.icerik = s.kayitli = d.icerik;
      }
      bildir(r.sayi ? `${r.sayi} yer, ${r.degisen.length} dosyada değiştirildi.` : 'Değiştirilecek bir şey bulunamadı.');
      fiilleriTopla();
      guncelle('sekmeler', 'kod', 'yan');
      GIRDI.araMetin(D.araMetin);
      denetle();
    },
    projeyeGuven() { projeyeGuven(true); },
    kesmeAyarKaydet() {
      const m = D.modal;
      if (m?.tur !== 'kesmeAyar') return;
      const l = new Set(D.kesmeler[m.dosya] || []);
      l.add(m.satir);
      D.kesmeler[m.dosya] = [...l].sort((a, b) => a - b);
      const kosul = m.kosul.trim(), gunluk = m.gunluk.trim();
      if (kosul || gunluk) D.kesmeAyar[m.dosya + ':' + m.satir] = { ...(kosul && { kosul }), ...(gunluk && { gunluk }) };
      else delete D.kesmeAyar[m.dosya + ':' + m.satir];
      ayarYaz('kesmeler', D.kesmeler); ayarYaz('kesmeAyar', D.kesmeAyar);
      D.modal = null; katmanlariCiz(); isaretleriCiz();
      if (D.calisma?.ayikla) api('/api/ayikla', { kimlik: D.calisma.kimlik, komut: 'kesmeler', kesmeler: kesmeListesi() }).catch(() => null);
      if (D.yanPanel === 'calistir') cizYanPanel();
    },
    kesmeAyarKaldir() {
      const m = D.modal;
      if (m?.tur !== 'kesmeAyar') return;
      D.modal = null; katmanlariCiz();
      if ((D.kesmeler[m.dosya] || []).includes(m.satir)) kesmeDegistir(m.satir);
    },
    izlemeKaldir(i) { D.izlenenler.splice(+i, 1); ayarYaz('izlenenler', D.izlenenler); cizYanPanel(); },
    async guveniKaldir() {
      if (!D.proje) return;
      if (D.proje.guvenilir === false) { bildir('Bu proje zaten kısıtlı modda.'); return; }
      if (await projeyeGuven(false)) bildir('Proje kısıtlı moda alındı.');
    },
    kisitliSeritKapat() { D.kisitliSeritKapali = true; cizKisitliSerit(); },
    kisitliModBilgi() { D.kisitliSeritKapali = false; cizKisitliSerit(); },
    async gecmisGeriYukle() {
      const m = D.modal;
      if (m?.icerik == null) return;
      const icerik = m.icerik;
      D.modal = null; katmanlariCiz();
      await asistanYaz({ dosya: m.yol }, icerik);
    },
    guncellemeDenetleDegistir() { D.guncellemeDenetle = !D.guncellemeDenetle; ayarYaz('guncellemeDenetle', D.guncellemeDenetle); katmanlariCiz(); },
    yeniSurumModal() { D.menu = null; D.modal = { tur: 'yeniSurum' }; katmanlariCiz(); },
    async guncellemeyiKur() {
      if (D.guncellemeDurumu === 'indiriliyor') return;
      D.guncellemeDurumu = 'indiriliyor'; katmanlariCiz();
      const r = await api('/api/guncelleme/kur', { masaustu: !!window.__TAURI__ }).catch(e => ({ hata: e.message }));
      D.guncellemeDurumu = r.hata ? 'Güncellenemedi: ' + r.hata : r.mesaj;
      katmanlariCiz();
    },
    acilisDegistir() { D.acilis = !D.acilis; ayarYaz('acilis', D.acilis); katmanlariCiz(); },
    yazarkenDenetleDegistir() { D.yazarkenDenetle = !D.yazarkenDenetle; ayarYaz('yazarkenDenetle', D.yazarkenDenetle); katmanlariCiz(); },

    // düzenleyici
    menuAc(ad, el, e) {
      if (e.target.closest('.acilir-menu')) return;
      D.menu = D.menu === ad ? null : ad; guncelle('baslik');
    },
    yanPanelSec(p) { D.yanPanel = p; guncelle('etkinlik', 'yan'); if (p === 'ara') $('#araMetin')?.focus(); if (p === 'eklentiler') paketleriYukle(); if (p === 'sinamalar') sinamalariYukle(); if (p === 'git') gitYukle(); if (p === 'veritabani') vtYukle(); },
    vtYukle() { vtYukle(); },
    async profilCikar() {
      const giris = girisDosyasi();
      if (!giris) return bildir('Giriş dosyası yok.', true);
      if (!(await tumunuKaydet())) return;
      D.altPanel = true; D.altSekme = 'kabuk'; guncelle('alt');
      kabukKomutu(`orhunca profil "${goreliYol(giris)}"`);
    },
    vtSorguAc() { vtAc('sql', ''); },
    vtJsonAc(m) { vtAc('json', m); },
    vtTabloAc(t) { vtAc('sql', t); },
    vtSorgula() { vtSorgula(); },
    sinamalariYukle() { sinamalariYukle(); },
    gitYukle() { gitYukle(); },
    gitBaslat() { gitIslem('/api/git/baslat', {}, 'Git deposu başlatıldı.'); },
    gitCek() { gitIslem('/api/git/cek', {}, r => r.mesaj).then(ok => ok && Promise.all(D.sekmeler.map(async s => { if (s.icerik !== s.kayitli) return; const d = await api('/api/dosya?' + sorgu({ yol: tamYol(s.yol) })); if (!d.hata && !d.ikili) s.icerik = s.kayitli = d.icerik; })).then(() => guncelle('kod', 'sekmeler'))); },
    gitGonder() { gitIslem('/api/git/gonder', {}, r => r.mesaj); },
    gitHazirla(a) {
      const geri = a[0] === '-', yol = a.slice(1);
      const yollar = yol === '*' ? D.git.degisiklikler.filter(d => d.hazir === geri).map(d => d.yol) : [yol];
      if (yollar.length) gitIslem('/api/git/hazirla', { yollar, geri });
    },
    async gitAt(a) {
      const takipsiz = a[0] === '?', yol = takipsiz ? a.slice(1) : a;
      if (!(await onayla(takipsiz ? `“${yol}” yeni bir dosya: silinsin mi?\n\nSon hâli yerel geçmişte saklanır.` : `“${yol}” dosyasındaki değişiklikler atılsın mı (son işlenen hâline dönülür)?\n\nŞimdiki hâli yerel geçmişte saklanır.`, { dugme: takipsiz ? 'Sil' : 'Değişiklikleri at', tehlikeli: true }))) return;
      await tumunuKaydet();
      if (await gitIslem('/api/git/at', { yol, takipsiz })) {
        const s = D.sekmeler.find(x => x.yol === yol);
        if (s) {
          const d = await api('/api/dosya?' + sorgu({ yol: tamYol(yol) }));
          if (d.hata) D.sekmeler = D.sekmeler.filter(x => x !== s); else if (!d.ikili) s.icerik = s.kayitli = d.icerik;
          if (D.etkin === yol && d.hata) D.etkin = D.sekmeler[0]?.yol || null;
          guncelle('sekmeler', 'kod');
        }
        agaciYukle().then(() => guncelle('yan'));
      }
    },
    async gitIsle() {
      const mesaj = (D.gitMesaj || '').trim();
      if (!mesaj) { bildir('Ne değiştirdiğinizi anlatan bir mesaj yazın.', true); $('#gitMesaj')?.focus(); return; }
      if (!(await tumunuKaydet())) return;
      await gitYukle();
      if (!D.git?.degisiklikler?.some(d => d.hazir)) {
        if (!D.git?.degisiklikler?.length) return bildir('İşlenecek değişiklik yok.');
        if (!(await onayla('Hazırlanmış değişiklik yok. Bütün değişiklikler hazırlanıp işlensin mi?', { baslik: 'İşle (commit)', dugme: 'Hepsini işle' }))) return;
        if (!(await gitIslem('/api/git/hazirla', { yollar: D.git.degisiklikler.map(d => d.yol), geri: false }))) return;
      }
      if (await gitIslem('/api/git/isle', { mesaj }, r => `İşlendi: ${r.mesaj}`)) { D.gitMesaj = ''; cizYanPanel(); }
    },
    async gitFark(a) {
      const hazir = a[0] === '1', yol = a.slice(1);
      D.modal = { tur: 'fark', yol, hazir, metin: null }; katmanlariCiz();
      const r = await api('/api/git/fark?' + sorgu({ kok: D.proje.yol, yol, hazir: hazir ? '1' : '' })).catch(e => ({ hata: e.message }));
      if (D.modal?.tur !== 'fark' || D.modal.yol !== yol) return;
      if (r.hata) { D.modal = null; katmanlariCiz(); return bildir(r.hata, true); }
      D.modal.metin = r.fark; katmanlariCiz();
    },
    farkDosyaAc() { const y = D.modal?.yol; D.modal = null; katmanlariCiz(); if (y) dosyaAc(y); },
    sinamalariCalistir() { sinamalariCalistir(); },
    sinamaDosyaCalistir(d) { sinamalariCalistir(d); },
    sinamaCalistir(a) { const i = a.lastIndexOf('|'); sinamalariCalistir(a.slice(0, i), a.slice(i + 1)); },
    paketleriYenile() { paketleriYukle(); },
    paketEkle() { if (D.paketKaynagi.trim()) paketIslemi('/api/paket/ekle', { kaynak: D.paketKaynagi.trim() }, `Paket ekleniyor: ${D.paketKaynagi.trim()}`); },
    paketYukle() { paketIslemi('/api/paket/yukle', { guncelle: false }, 'Paketler yükleniyor…'); },
    paketGuncelle() { paketIslemi('/api/paket/yukle', { guncelle: true }, 'Paketler güncelleniyor…'); },
    paketDizindenEkle(ad) { paketIslemi('/api/paket/ekle', { kaynak: ad }, `Paket ekleniyor: ${ad}`); },
    async paketKaldir(ad) { if (await onayla(`“${ad}” paketi kaldırılsın mı?`, { dugme: 'Kaldır', tehlikeli: true })) paketIslemi('/api/paket/kaldir', { ad }, `Paket kaldırılıyor: ${ad}`); },
    panelGezgin() { EYLEM.yanPanelSec('gezgin'); }, panelAra() { EYLEM.yanPanelSec('ara'); },
    panelYapi() { EYLEM.yanPanelSec('yapi'); }, panelCalistir() { EYLEM.yanPanelSec('calistir'); },
    klasorAcKapa(yol) { D.kapaliKlasorler.has(yol) ? D.kapaliKlasorler.delete(yol) : D.kapaliKlasorler.add(yol); cizYanPanel(); },
    dosyaAc(yol) { dosyaAc(yol); },
    async agaciYenile() { await agaciYukle(); cizYanPanel(); },
    sekmeSec(yol) { D.etkin = yol; guncelle('sekmeler', 'kod', 'yan', 'durum'); },
    async sekmeKapat(yol, el, e) {
      e.stopPropagation();
      const s = D.sekmeler.find(x => x.yol === yol);
      if (s && !s.ikili && s.icerik !== s.kayitli && !(await onayla(`“${sonParca(yol)}” dosyasında kaydedilmemiş değişiklikler var. Yine de kapatılsın mı?`, { dugme: 'Kaydetmeden kapat', tehlikeli: true }))) return;
      const i = D.sekmeler.indexOf(s);
      D.sekmeler.splice(i, 1);
      if (D.etkin === yol) D.etkin = (D.sekmeler[i] || D.sekmeler[i - 1])?.yol || null;
      guncelle('sekmeler', 'kod', 'yan');
    },
    satiraGit(n) { satiraGit(+n); },
    satiraGitOnayla() {
      const n = parseInt($('#satirGirdi')?.value, 10);
      D.modal = null;
      katmanlariCiz();
      if (n > 0) satiraGit(n);
      else $('#kodAlani')?.focus();
    },
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
    terminalTemizle() { if (D.altSekme === 'cikti') D.cikti = []; else if (D.altSekme === 'kabuk') D.kabukSatir = []; else D.terminal = []; guncelle('alt'); },
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
    async denetleKomut() {
      if (!(await tumunuKaydet())) return bildir('Dosya kaydedilemediği için denetlenmedi.', true);
      if (!(await denetle())) return bildir(girisHatasi() || 'Denetlenecek .ohc dosyası yok.', true);
      if (D.denetimHatasi) bildir('Denetlenemedi: ' + D.denetimHatasi, true);
      else if (!D.sorunlar.length) bildir('Hata yok.');
      else EYLEM.altSorunlar();
    },
    derleLinux() { derle('linux'); },
    derleWindows() { derle('windows'); },
    derleWeb() { derle('web'); },
    ayikla() { calistir(true); },
    dersSec(k) { D.ders = k || null; cizYanPanel(); $('#yanPanel .panel-ic')?.scrollTo(0, 0); },
    gorevBasla(i) { gorevBasla(+i); },
    gorevDenetle(i) { gorevDenetle(+i); },
    async dersleriAc() {
      await dersleriYukle();
      const r = await api('/api/ders/hazirla', { dosya: '00-deneme', baslangic: '# Deneme sayfası: istediğinizi yazın, F5 ile çalıştırın.\n"Merhaba!"\'yı yaz.\n' }).catch(e => ({ hata: e.message }));
      if (r.hata) { bildir(r.hata, true); return; }
      await projeyiAc(r.proje);
      await dosyaAc(r.dosya);
      D.yanPanel = 'dersler'; D.ders = null;
      guncelle('yan', 'etkinlik');
    },
    yavasCalistir() { yavasCalistir(); },
    yavasDegistir() {
      const c = D.calisma;
      if (!c?.yavas) return;
      c.yavasAcik = !c.yavasAcik;
      if (c.yavasAcik && c.ay?.durdu) ayiklamaKomutu('adim');
      cizYanPanel();
    },
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
    paketleAndroid() { derle('masaustu-android'); },
    paketleIos() { derle('masaustu-ios'); },
    kaydet() { kaydet(); },
    tumunuKaydet() { tumunuKaydet({ yenile: true }); },
    async baslangicaDon() {
      await projeleriYenile();
      D.ekran = 'baslangic'; D.menu = null; iskeletVar = false; ciz();
    },
    async projeyiKapat() {
      if (!(await degisiklikleriKoru())) return;
      if (D.calisma) await durdur();
      D.proje = null; D.calisma = null;
      EYLEM.baslangicaDon();
    },
    async studyoyuKapat() {
      if (D.sekmeler.some(s => !s.ikili && s.icerik !== s.kayitli) && !(await onayla('Kaydedilmemiş değişiklikler var. Stüdyo kapatılsın mı?', { dugme: 'Kapat', tehlikeli: true }))) return;
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
    async hataDuzelt(i, el, e) {
      e.stopPropagation();
      const h = D.sorunlar[+i];
      const d = h && h.duzeltme;
      if (!d) return;
      await dosyaAc(goreliYol(h.dosya));
      const ta = $('#kodAlani');
      if (!ta) return;
      const satirlar = ta.value.split('\n');
      let konum = 0;
      for (let k = 0; k < d.satir - 1; k++) konum += satirlar[k].length + 1;
      // Sütun ve uzunluk karakter sayısıdır (Türkçe harfler tek karakter).
      const satir = [...(satirlar[d.satir - 1] || '')];
      konum += satir.slice(0, d.sutun - 1).join('').length;
      const uzunluk = satir.slice(d.sutun - 1, d.sutun - 1 + d.uzunluk).join('').length;
      ta.focus();
      ta.setSelectionRange(konum, konum + uzunluk);
      metinEkle(ta, d.yeni);
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
    satiriYukari() { const ta = $('#kodAlani'); if (ta) { ta.focus(); satirlariTasi(ta, -1); } },
    satiriAsagi() { const ta = $('#kodAlani'); if (ta) { ta.focus(); satirlariTasi(ta, 1); } },
    satiraGitModal() { D.menu = null; D.modal = { tur: 'satiraGit' }; katmanlariCiz(); setTimeout(() => $('#satirGirdi')?.focus(), 30); },
    pencereKucult() { try { window.__TAURI__.window.getCurrentWindow().minimize(); } catch { /* yok */ } },
    pencereBuyut() { try { window.__TAURI__.window.getCurrentWindow().toggleMaximize(); } catch { /* yok */ } },
    pencereKapat() { try { window.__TAURI__.window.getCurrentWindow().close(); } catch { /* yok */ } },
  };

  async function paketleriYukle() {
    if (D.paketDizini == null && !D.paketDizinYukleniyor) {
      D.paketDizinYukleniyor = true;
      api('/api/paket/dizin').then(r => {
        if (r.hata) D.paketDizinHatasi = r.hata; else D.paketDizini = r.paketler || [];
      }).catch(e => { D.paketDizinHatasi = e.message; }).finally(() => {
        D.paketDizinYukleniyor = false;
        if (D.yanPanel === 'eklentiler') cizYanPanel();
      });
    }
    if (!D.proje) return;
    const r = await api('/api/paket/liste?' + sorgu({ kok: D.proje.yol })).catch(e => ({ hata: e.message }));
    D.paketler = r.paketler || [];
    if (D.yanPanel === 'eklentiler') cizYanPanel();
  }

  async function paketIslemi(yol, govde, baslik) {
    if (D.paketMesgul) return;
    if (!(await guvenSor('Paket kurmak'))) return;
    D.paketMesgul = true; cizYanPanel();
    D.altPanel = true; D.altSekme = 'cikti';
    D.cikti.push({ t: baslik, c: 'mut' });
    guncelle('alt');
    let r = await api(yol, { kok: D.proje.yol, ...govde }).catch(e => ({ hata: e.message }));
    if (r.izin) {
      // Paket dosya, ağ, C kütüphanesi gibi izinler istiyor: kullanıcıya sorulur.
      D.cikti.push({ t: 'Paket şu izinleri istiyor:\n' + r.ayrinti, c: 'err' });
      guncelle('alt');
      if (await onayla('Bu paket(ler) şu izinleri istiyor:\n\n' + r.ayrinti + '\n\nYalnızca güvendiğiniz paketlere izin verin. Onaylıyor musunuz?', { baslik: 'Paket izinleri', dugme: 'İzin ver' })) {
        r = await api(yol, { kok: D.proje.yol, ...govde, izinVer: true }).catch(e => ({ hata: e.message }));
      } else {
        r = { hata: 'İzin verilmedi; paket eklenmedi.' };
      }
    }
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
    temaRenk(v, el) { D.temaTaslak.renkler[el.dataset.ad] = v; (D.temaTaslak._degisti ||= {})[el.dataset.ad] = 1; temaUygula(); },
    temaAd(v) { D.temaTaslak.ad = v; },
    temaYazar(v) { D.temaTaslak.yazar = v; },
    temaTaban(v) { D.temaTaslak.taban = v; temaUygula(); },
    temaYazi(v, el) { (D.temaTaslak.yazi ||= {})[el.dataset.ad] = el.type === 'range' ? +v : v; temaUygula(); const e = $('#tema-' + el.dataset.ad + '-deger'); if (e) e.textContent = v; },
    temaKose(v) { D.temaTaslak.kose = +v; temaUygula(); },
    temaArka(v, el) {
      const a = D.temaTaslak.arka_plan; if (!a) return;
      a[el.dataset.ad] = el.type === 'range' ? +v : v; temaUygula();
      const e = $('#tema-' + el.dataset.ad + '-deger'); if (e) e.textContent = v;
    },
    temaCss(v) { D.temaTaslak.ozel_css = v; clearTimeout(GIRDI._css); GIRDI._css = setTimeout(temaUygula, 300); },
    temaDosya(_, el) {
      const f = el.files?.[0]; if (!f) return;
      if (f.size > 22 * 1024 * 1024) return bildir('Dosya çok büyük (en çok 22 MB). Daha küçük bir resim ya da GIF seçin.', true);
      const r = new FileReader();
      r.onload = () => {
        D.temaTaslak.arka_plan = { kaynak: r.result, tur: f.type.startsWith('video') ? 'video' : 'resim', konum: 'kapla', saydamlik: 82, bulaniklik: 0, karartma: 25 };
        temaUygula(); katmanlariCiz();
      };
      r.readAsDataURL(f);
    },
    temaIceAktar(_, el) {
      const f = el.files?.[0]; if (!f) return;
      const r = new FileReader();
      r.onload = () => {
        try {
          D.temaTaslak = window.OrhuncaTema.dogrula(JSON.parse(r.result));
          D.temaTaslak.renkler = window.OrhuncaTema.hesaplanan(D.temaTaslak);
          D.temaKimlikTaslak = null;
          temaUygula(); katmanlariCiz();
          bildir(`“${D.temaTaslak.ad}” içe aktarıldı; kalıcı yapmak için Kaydet ve uygula.`);
        } catch (e) { bildir('Tema okunamadı: ' + e.message, true); }
      };
      r.readAsText(f);
    },
    yavasHiz(v) { D.yavasHiz = +v; ayarYaz('yavasHiz', D.yavasHiz); },
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
    asistanGirdi(v) { D.asistan.girdi = v; },
    asistanAnahtar(v) { D.asistan.anahtar = v; },
    asistanAdres(v) { D.asistan.adres = v; },
    hataBildirNe(v) { if (D.modal) D.modal.ne = v; },
    async asistanModel(v) {
      if (v === '__elle__') {
        v = ((await metinSor('Model adı (ör. qwen2.5-coder:7b, gpt-4.1, anthropic/claude-sonnet-4):', D.asistan.durum?.model || '', { baslik: 'Model', dugme: 'Seç' })) || '').trim();
        if (!v) return cizAsistan();
      }
      if (!v) return;
      const r = await api('/api/asistan/ayar', { model: v });
      if (r.hata) D.asistan.hata = r.hata; else D.asistan.durum = r;
      cizAsistan();
    },
    araMetin(v) {
      D.araMetin = v;
      clearTimeout(GIRDI._a);
      GIRDI._a = setTimeout(async () => {
        if (!v.trim()) { D.araSonuc = []; cizYanPanel(); return; }
        const r = await api('/api/ara?' + sorgu({ kok: D.proje.yol, metin: v, tam: D.tamKelime ? '1' : '' }));
        if (D.araMetin === v) { D.araSonuc = r.sonuclar || []; cizYanPanel(); }
      }, 200);
    },
    degistirMetin(v) { D.degistirMetin = v; },
    gitMesaj(v) { D.gitMesaj = v; },
    kesmeKosul(v) { if (D.modal) D.modal.kosul = v; },
    kesmeGunluk(v) { if (D.modal) D.modal.gunluk = v; },
    /** Öğren sayfasındaki yerleşik işlev listesini süzer (Türkçe harfsiz yazım da bulunur). */
    basvuruAra(v) {
      const sade = t => kucuk(t).replace(/[çğıöşü]/g, h => ({ ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u' })[h]).replace(/\u0307/g, '');
      const a = sade(v.trim());
      let toplam = 0;
      for (const b of document.querySelectorAll('#basvuru .basvuru-bolum')) {
        let gorunen = 0;
        for (const satir of b.querySelectorAll('.tablo-satir')) {
          const uyar = !a || sade(satir.textContent).includes(a);
          satir.classList.toggle('gizli', !uyar);
          if (uyar) gorunen++;
        }
        b.classList.toggle('gizli', !gorunen);
        toplam += gorunen;
      }
      $('#basvuruYok')?.classList.toggle('gizli', toplam > 0);
    },
  };

