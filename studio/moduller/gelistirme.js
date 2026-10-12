/* Orhunca Stüdyo — Kurtarma ve proje geliştirme araçları. */
'use strict';
const KURTARMA_OTURUMU = Array.from(crypto.getRandomValues(new Uint8Array(16)), b => b.toString(16).padStart(2, '0')).join('');
let taslakSirasi = 0, taslakZamani, taslakKuyrugu = Promise.resolve(), yedekZamani = 0;
function taslakVerisi() {
  if (!D.proje) return null;
  return { kok: D.proje.yol, islem: 'taslak', oturum: KURTARMA_OTURUMU, sira: ++taslakSirasi,
    sekmeler: D.sekmeler.filter(s => !s.ikili && s.icerik !== s.kayitli).map(s => ({ yol: s.yol, icerik: s.icerik, kayitli: s.kayitli, surum: s.surum })) };
}
function taslagiSakla() {
  const veri = taslakVerisi();
  if (!veri) return Promise.resolve();
  taslakKuyrugu = taslakKuyrugu.catch(() => {}).then(async () => {
    const r = await api('/api/gelistirme', veri);
    if (r.hata) throw Error(r.hata);
  }).catch(e => { if (D.proje?.yol === veri.kok) bildir('Kurtarma taslağı saklanamadı: ' + e.message, true); });
  return taslakKuyrugu;
}
function taslagiPlanla() { clearTimeout(taslakZamani); taslakZamani = setTimeout(taslagiSakla, 300); }
document.addEventListener('input', e => { if (e.target.closest('#kodBolge')) taslagiPlanla(); });
setInterval(otomatikYedek, 600000);
document.addEventListener('visibilitychange', () => { if (document.hidden) taslagiSakla(); });
window.addEventListener('pagehide', () => {
  const v = taslakVerisi();
  if (v) fetch('/api/gelistirme', { method: 'POST', keepalive: true, headers: { 'Content-Type': 'application/json', 'X-Orhunca-Anahtar': ANAHTAR || '' }, body: JSON.stringify(v) }).catch(() => {});
});
async function otomatikYedek() {
  if (!D.proje || Date.now() - yedekZamani < 600000) return;
  const proje = D.proje; yedekZamani = Date.now();
  const r = await api('/api/gelistirme', { kok: proje.yol, islem: 'yedek', otomatik: true }).catch(e => ({ hata: e.message }));
  if (r.hata && D.proje === proje) { yedekZamani = 0; bildir('Otomatik yedek alınamadı: ' + r.hata, true); }
}
async function kurtarmayiDenetle(proje) {
  const r = await api('/api/gelistirme', { kok: proje.yol, islem: 'kurtarma' }).catch(() => null);
  if (D.proje !== proje || !r) return;
  if (r.taslaklar?.some(t => t.oturum !== KURTARMA_OTURUMU && t.sekmeler.length)) bildir('Kurtarılabilecek sekmeler var. Proje → Kurtarma ve yedekler menüsünü açın.');
  otomatikYedek();
}
const GELISTIRME_SEKMELERI = [['kurtarma', 'Kurtarma'], ['saglik', 'Proje sağlığı'], ['profiller', 'Hedef profilleri'], ['api', 'API senaryoları'], ['cihaz', 'Cihaz ve günlük'], ['gecis', 'Veritabanı'], ['demo', 'Demo ve teslim']];
async function gelistirmeAc(sekme = 'kurtarma') {
  if (!D.proje) return;
  const proje = D.proje;
  if (['saglik', 'gecis', 'cihaz'].includes(sekme) && !(await guvenSor('Proje araçlarını çalıştırmak'))) return;
  if (D.proje !== proje) return;
  D.menu = null;
  const m = { tur: 'gelistirme', proje: D.proje, sekme, veri: null, sonuc: null, mesgul: true };
  D.modal = m; katmanlariCiz();
  const islem = ({ profiller: 'ayarlar', api: 'ayarlar', cihaz: 'cihazlar', gecis: 'gecis-onizle', demo: 'ayarlar' })[sekme] || sekme;
  try { m.veri = await api('/api/gelistirme', { kok: m.proje.yol, islem, hedef: sekme === 'cihaz' ? 'android' : 'web' }); }
  catch (e) { m.veri = { hata: e.message }; }
  m.mesgul = false;
  if (D.modal === m) katmanlariCiz();
}
function gelDugme(ad, eylem, arg = '', tehlikeli = false) {
  return `<button type="button" class="dugme ${tehlikeli ? 'tehlikeli' : ''}" data-e="${eylem}" data-a="${kac(arg)}" ${D.modal?.mesgul ? 'disabled' : ''}>${kac(ad)}</button>`;
}
function gelAlan(ad, id, deger = '', tur = 'text') {
  return `<label for="${id}">${kac(ad)}</label><input class="metin-girdi" id="${id}" type="${tur}" value="${kac(deger)}">`;
}
function cizGelistirme(kabuk) {
  const m = D.modal, v = m.veri || {}, sec = m.sekme;
  let govde = `<nav class="gel-sekmeler" aria-label="Proje araçları">${GELISTIRME_SEKMELERI.map(([id, ad]) => gelDugme(ad, 'gelistirmeAc', id).replace('<button ', `<button aria-current="${id === m.sekme ? 'page' : 'false'}" `)).join('')}</nav>`;
  if (m.mesgul) govde += '<p role="status">İşlem sürüyor…</p>';
  if (v.hata) govde += `<p role="alert">${kac(v.hata)}</p>`;
  if (sec === 'kurtarma') {
    govde += '<h3>Kaydedilmemiş sekmeler</h3><p>Kurtarma editörde açılır; diske yazmadan önce farkları inceleyebilirsiniz.</p>';
    govde += (v.taslaklar || []).map((t, i) => `<p>${t.sekmeler.length} sekme · ${tarihBicim(t.zaman)} ${gelDugme('Sekmeleri kurtar', 'taslakKurtar', String(i))} ${gelDugme('Taslağı sil', 'taslakSil', t.oturum, true)}</p>`).join('') || '<p>Taslak yok.</p>';
    govde += '<h3>Proje yedekleri</h3><p>Yedekler özel depoda saklanır. Kurtarma özgün projeyi değiştirmeden yeni klasör oluşturur. Otomatik yedek aralığı 10 dakikadır.</p>' + gelDugme('Şimdi yedek al', 'gelIslem', 'yedek');
    govde += (v.yedekler || []).map(t => `<p>${tarihBicim(t.zaman)} · ${Math.ceil(t.boyut / 1024)} KiB ${gelDugme('Kopyada dene', 'yedekDene', t.kimlik)} ${gelDugme('Yedeği sil', 'yedekSil', t.kimlik, true)}</p>`).join('');
  } else if (sec === 'saglik') {
    govde += '<label for="gelHedef">Hedef</label><select id="gelHedef" class="metin-girdi">' + ['web', 'android', 'ios', 'linux', 'windows', 'macos'].map(x => `<option ${v.hedef === x ? 'selected' : ''}>${x}</option>`).join('') + '</select>' + gelDugme('Yenile', 'saglikYenile');
    govde += '<ul>' + (v.araclar || []).map(a => `<li>${kac(a.ad)}: ${kac(a.durum)} ${kac(a.surum || '')}</li>`).join('') + '</ul>';
    govde += `<h3>Son derleme</h3><pre>${kac(v.son_derleme ? JSON.stringify(v.son_derleme, null, 2) : 'Henüz çalıştırılmadı.')}</pre><h3>Son başarılı derleme</h3><pre>${kac(v.son_basarili_derleme ? JSON.stringify(v.son_basarili_derleme, null, 2) : 'Henüz çalıştırılmadı.')}</pre><h3>Son sınama</h3><pre>${kac(v.son_sinama ? JSON.stringify(v.son_sinama, null, 2) : 'Henüz çalıştırılmadı.')}</pre>`;
    govde += (v.paketler || []).map(p => `<p>${kac(p.ad)}: ${p.kurulu ? 'kurulu' : 'eksik'}</p>`).join('');
  } else if (sec === 'profiller' || sec === 'api' || sec === 'demo') {
    if (sec === 'profiller' || sec === 'demo') {
      govde += '<p>Profiller geliştirme, sınama ve teslim hedeflerini saklar. Ortam değerlerinde ${DEGISKEN} kullanın; gerçek anahtarları .env içinde tutun. Web/mobil istemcisinin API adresini acik_ortam alanına koyun; bu değerler teslim dosyasına gömülür.</p>';
      govde += '<label for="gelProfil">Hedef profili</label><select id="gelProfil" class="metin-girdi">' + (v.ayarlar?.profiller || []).map(p => `<option value="${kac(p.ad)}">${kac(p.ad)} · ${kac(p.hedef)}</option>`).join('') + '</select>' + gelDugme('Derle ve teslim klasörü hazırla', 'profilDerle') + gelDugme('Profil ortamıyla yerelde çalıştır', 'profilCalistir');
      if (sec === 'demo') govde += '<p>Demo kopyası kişisel veri ve .env olmadan oluşturulur; demo/veri içindeki örnek kayıtlar yüklenir. Her oluşturma temiz bir demo verir.</p>' + gelDugme('Temiz demo oluştur', 'gelIslem', 'demo');
    }
    if (sec === 'api') govde += '<p>İstek koleksiyonunda beklenen HTTP durumunu ve yanıt metnini tanımlayın. Yetki reddi için 401/403 bekleyen ayrı senaryolar ekleyin. Çevrimdışı (0) ve hatalı yanıt (503) kipleri ağ isteği yapmadan sonuç benzetir.</p>' + gelDugme('Senaryoları çalıştır', 'gelIslem', 'api-sina');
    govde += '<details><summary>Profilleri ve API koleksiyonunu düzenle</summary><p>Değişiklikler studyo.ayar.json dosyasında sürüm kontrolüyle kaydedilir.</p><label for="gelAyar">Proje araçları ayarları</label><textarea id="gelAyar" class="metin-girdi" rows="15" spellcheck="false">' + kac(JSON.stringify(v.ayarlar || {}, null, 2)) + '</textarea>' + gelDugme('Ayarları kaydet', 'gelAyarKaydet') + gelDugme('API örneği ekle', 'gelApiOrnegi') + '</details>';
  } else if (sec === 'cihaz') {
    const hedef = m.cihazHedef || 'android';
    govde += '<label for="gelCihazHedef">Çalıştırma ortamı</label><select id="gelCihazHedef" class="metin-girdi">' + [['android','Android cihaz / emülatör'],['ios','iOS simülatörü (macOS / Xcode)'],['masaustu','Bu bilgisayar']].map(([id,ad]) => `<option value="${id}" ${hedef === id ? 'selected' : ''}>${ad}</option>`).join('') + '</select>' + gelDugme('Cihazları yenile', 'cihazYenile');
    govde += `<pre>${kac(v.cikti || '')}</pre>`;
    if (hedef === 'masaustu') {
      govde += gelDugme('Çalıştır', 'gelMasaustu', 'calistir') + gelDugme('Hata ayıkla', 'gelMasaustu', 'ayikla') + gelDugme('Durdur', 'gelMasaustu', 'durdur') + gelDugme('Günlükleri göster', 'gelMasaustu', 'gunluk');
    } else {
      govde += '<p>Android için USB hata ayıklamayı açın. iOS simülatörü macOS ve Xcode gerektirir; fiziksel iPhone için Xcode cihaz araçlarını kullanın.</p>';
      govde += gelAlan('Cihaz seri numarası / simülatör kimliği', 'gelSeri', m.cihazForm?.seri || '') + gelAlan(hedef === 'ios' ? 'Derlenmiş .app yolu (projeye göre)' : 'APK yolu (projeye göre)', 'gelApk', m.cihazForm?.apk || (hedef === 'ios' ? 'cikti/Uygulama.app' : 'cikti/uygulama.apk')) + gelAlan('Uygulama kimliği', 'gelUygulama', m.cihazForm?.uygulama || 'com.orhunca.uygulama');
      govde += ['kur', 'baslat', 'durdur', 'gunluk'].map((a, i) => gelDugme(['Uygulamayı kur', 'Başlat', 'Durdur', 'Son günlükler'][i], 'cihazIslem', a)).join('');
      if (hedef === 'ios') govde += gelDugme('Simülatörü aç', 'cihazIslem', 'ac');
    }
    govde += gelDugme(m.canli ? 'Canlı günlükleri kapat' : 'Canlı günlükleri aç', 'cihazCanli');
  } else if (sec === 'gecis') {
    govde += '<p>SQLite geçişlerini gecisler/001_ad.sql, tersini 001_ad.geri.sql olarak ekleyin. Uygulama öncesinde tutarlı veritabanı yedeği alınır; geçişler ve kayıt defteri tek işlemde yazılır.</p>';
    govde += `<pre>${kac(JSON.stringify(v, null, 2))}</pre>` + gelDugme('Bekleyen geçişleri uygula', 'gecisIslem', 'gecis-uygula') + gelDugme('Son geçişi geri al', 'gecisIslem', 'gecis-geri', true);
  }
  if (m.sonuc) govde += `<h3>Sonuç</h3><pre id="gelSonuc" role="status">${kac(JSON.stringify(m.sonuc, null, 2))}</pre>` + gelDugme('Raporu indir', 'gelRaporIndir');
  return kabuk('Proje araçları', govde, gelDugme('Kapat', 'modalKapat'), 'genis');
}
async function gelCalistir(islem, alanlar = {}) {
  const m = D.modal;
  if (m?.tur !== 'gelistirme' || m.mesgul) return;
  if (!['yedek', 'yedek-dene', 'yedek-sil', 'taslak-sil'].includes(islem) && !(await guvenSor('Proje araçlarını çalıştırmak'))) return;
  if (D.modal !== m || D.proje !== m.proje) return;
  m.mesgul = true; katmanlariCiz();
  try { m.sonuc = await api('/api/gelistirme', { kok: m.proje.yol, islem, ...alanlar }); }
  catch (e) { m.sonuc = { hata: e.message }; }
  m.mesgul = false;
  if (D.modal === m) katmanlariCiz();
  return m.sonuc;
}
Object.assign(EYLEM, {
  gelistirmeAc,
  async silineniTemizle(kimlik) {
    const m = D.modal;
    if (m?.tur !== 'silinenler' || !(await onayla('Bu kurtarma kaydı kalıcı olarak silinsin mi?', {tehlikeli:true,dugme:'Kalıcı sil'}))) return;
    const r=await api('/api/cop/sil',{kok:m.proje.yol,kimlik});
    if (r.hata) return bildir(r.hata,true);
    if (D.modal===m) EYLEM.silinenlerModal();
  },
  kurtarmaAc() { return gelistirmeAc('kurtarma'); },
  saglikAc() { return gelistirmeAc('saglik'); },
  profillerAc() { return gelistirmeAc('profiller'); },
  apiSenaryoAc() { return gelistirmeAc('api'); },
  cihazAc() { return gelistirmeAc('cihaz'); },
  gecisAc() { return gelistirmeAc('gecis'); },
  demoAc() { return gelistirmeAc('demo'); },
  gelIslem(islem) { return gelCalistir(islem); },
  async yedekDene(kimlik) { const r = await gelCalistir('yedek-dene', { kimlik }); if (r?.yol) await agaciYukle(); },
  async yedekSil(kimlik) { if (await onayla('Bu yedek kalıcı olarak silinsin mi?', { tehlikeli: true, dugme: 'Sil' })) await gelCalistir('yedek-sil', { kimlik }); },
  async taslakSil(oturum) { if (await onayla('Kurtarma taslağı kalıcı olarak silinsin mi?', { tehlikeli: true, dugme: 'Sil' })) await gelCalistir('taslak-sil', { oturum }); },
  async taslakKurtar(i) {
    const m = D.modal, t = m?.veri?.taslaklar?.[+i];
    if (!t || m.proje !== D.proje) return;
    if (D.sekmeler.some(s => !s.ikili && s.icerik !== s.kayitli)) return bildir('Önce açık değişiklikleri kaydedin veya ayrı bir Stüdyo penceresinde kurtarın.', true);
    const yeni = [];
    for (const s of t.sekmeler) {
      const r = await api('/api/dosya?' + sorgu({ yol: tamYol(s.yol) }));
      if (r.ikili) continue;
      yeni.push({ ...s, zaman: r.zaman || 0, cakisma: r.surum !== s.surum ? { ...r, surum: r.surum || 'yok', icerik: r.icerik ?? null } : null });
    }
    if (D.proje !== m.proje || D.modal !== m) return;
    for (const s of yeni) { const i = D.sekmeler.findIndex(x => x.yol === s.yol); if (i < 0) D.sekmeler.push(s); else D.sekmeler[i] = s; }
    D.etkin = yeni[0]?.yol || D.etkin; D.modal = null; guncelle('kod', 'sekmeler', 'katman');
    await taslagiSakla(); bildir('Sekmeler kurtarıldı. Kaydetmeden önce içerikleri inceleyin.');
  },
  async saglikYenile() { const hedef = $('#gelHedef').value; const r = await gelCalistir('saglik', { hedef }); if (r && D.modal?.tur === 'gelistirme') { D.modal.veri = r; D.modal.sonuc = null; katmanlariCiz(); } },
  async profilCalistir() { const profil = $('#gelProfil').value; D.modal = null; katmanlariCiz(); await calistir(false, { profil }); },
  async profilDerle() { const profil = $('#gelProfil').value; if (await tumunuKaydet()) await gelCalistir('profil-derle', { profil }); },
  async gelAyarKaydet() {
    const m = D.modal; let ayarlar;
    try { ayarlar = JSON.parse($('#gelAyar').value); } catch { return bildir('Ayarlar geçerli JSON olmalı.', true); }
    const r = await api('/api/gelistirme', { kok: m.proje.yol, islem: 'ayar-kaydet', ayarlar, surum: m.veri.surum });
    if (D.modal !== m) return;
    if (r.hata) return bildir(r.hata, true);
    m.veri = { ayarlar, surum: r.surum }; katmanlariCiz(); bildir('Proje araçları ayarları kaydedildi.');
  },
  gelApiOrnegi() {
    try {
      const a = JSON.parse($('#gelAyar').value); a.senaryolar ||= [];
      a.senaryolar.push({ ad: 'Yetki reddi', url: 'http://127.0.0.1:8080/api/ozel', yontem: 'GET', basliklar: {}, beklenen: 401, icerir: '' }, { ad: 'Yetkili istek', url: 'http://127.0.0.1:8080/api/ozel', yontem: 'GET', basliklar: { Authorization: 'Bearer ${API_TOKEN}' }, beklenen: 200 }, { ad: 'Çevrimdışı', kip: 'cevrimdisi', beklenen: 0 });
      $('#gelAyar').value = JSON.stringify(a, null, 2);
    } catch { bildir('Önce JSON hatasını düzeltin.', true); }
  },
  async cihazYenile() {
    const m = D.modal; if (m.canli) { clearInterval(m.canli); m.canli = null; } m.cihazHedef = $('#gelCihazHedef').value;
    const r = await gelCalistir('cihazlar', { hedef: m.cihazHedef });
    if (D.modal === m) { m.veri = r || {}; m.sonuc = null; katmanlariCiz(); }
  },
  cihazIslem(eylem) {
    const m = D.modal;
    m.cihazForm = { seri: $('#gelSeri').value, apk: $('#gelApk').value, uygulama: $('#gelUygulama').value };
    return gelCalistir('cihaz', { eylem, hedef: m.cihazHedef || 'android', ...m.cihazForm });
  },
  async gelMasaustu(eylem) {
    const m = D.modal;
    if (eylem === 'calistir' || eylem === 'ayikla') await calistir(eylem === 'ayikla');
    if (eylem === 'durdur') await durdur();
    if (D.modal === m) { m.sonuc = { cikti: D.terminal.map(s => s.t).join('\n'), suruyor: !!D.calisma }; katmanlariCiz(); }
  },
  cihazCanli() {
    const m = D.modal; if (m.canli) { clearInterval(m.canli); m.canli = null; katmanlariCiz(); return; }
    const hedef = m.cihazHedef || 'android';
    if (hedef !== 'masaustu') m.cihazForm = { seri: $('#gelSeri').value, apk: $('#gelApk').value, uygulama: $('#gelUygulama').value };
    m.canli = setInterval(async () => {
      if (D.modal !== m) { clearInterval(m.canli); return; }
      if (hedef === 'masaustu') EYLEM.gelMasaustu('gunluk');
      else if (!m.mesgul) await gelCalistir('cihaz', { eylem: 'gunluk', hedef, ...m.cihazForm });
    }, 2000);
    katmanlariCiz();
  },
  async gecisIslem(islem) {
    const m = D.modal;
    if (await onayla('Önizlenen veritabanı geçişi uygulansın mı? Önce otomatik yedek alınacak.', { dugme: 'Uygula', tehlikeli: true })) await gelCalistir(islem, { surum: m.veri.surum });
  },
  gelRaporIndir() {
    const url = URL.createObjectURL(new Blob([JSON.stringify(D.modal.sonuc, null, 2)], { type: 'application/json' }));
    const a = document.createElement('a'); a.href = url; a.download = 'orhunca-rapor.json'; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
  },
  onizlemeTelefon() { ayarYaz('onizlemeGenisligi', 390); cizOnizleme(); },
  onizlemeTablet() { ayarYaz('onizlemeGenisligi', 768); cizOnizleme(); },
  onizlemeMasaustu() { ayarYaz('onizlemeGenisligi', 0); cizOnizleme(); },
});
