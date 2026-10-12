/* Orhunca Stüdyo — İsteğe bağlı sistem sesi ve yerel EMA LiteRT desteği. */
'use strict';
let sesliAcik = ayarOku('sesliKod', false);
let sesliSonSatir = '';
let sesliSaglayici = ayarOku('sesliSaglayici', 'sistem') === 'ema' ? 'ema' : 'sistem';
let sesliHiz = Number(ayarOku('sesliHiz', 1)) || 1;
sesliHiz = Math.max(0.75, Math.min(1.5, sesliHiz));
let sesliSozluk = ayarOku('sesliSozluk', {});
let sesliEmaDurum = { hazir: false, kuruluyor: false, ileti: 'EMA durumu yükleniyor' };
let sesliIstek = 0, sesliAbort = null, sesliOynatici = null, sesliAdres = null;
let sesliAnlikIstek = null, sesliBekleyen = null;
const sesliOnbellek = new Map();
let sesliOnbellekBoyut = 0, sesliDurumZamani;
const sesliMotor = () => window.speechSynthesis;
const sesliTurkceSes = () => sesliMotor()?.getVoices().find(s => /^tr(?:-|$)/i.test(s.lang.replace('_', '-')));
const SESLI_ADLAR = { API: 'a pe i', JSON: 'cey son', HTTP: 'eyç ti ti pi', HTTPS: 'eyç ti ti pi es', SQL: 'es kü el', SQLite: 'es kü el layt', Rust: 'rast', JavaScript: 'cava skript', Python: 'payton', CPU: 'si pi yu' };
const SESLI_ISARETLER = { '+=': 'artır eşittir', '-=': 'azalt eşittir', '*=': 'çarp eşittir', '/=': 'böl eşittir', '==': 'eşit mi', '!=': 'eşit değil mi', '<=': 'küçük veya eşit', '>=': 'büyük veya eşit', '**': 'üs', '->': 'ok', '=>': 'ok', '=': 'eşittir', '+': 'artı', '-': 'eksi', '*': 'çarpı', '/': 'bölü', '%': 'kalan', '<': 'küçüktür', '>': 'büyüktür', '[': 'köşeli parantez aç', ']': 'köşeli parantez kapat', '(': 'parantez aç', ')': 'parantez kapat', '{': 'süslü parantez aç', '}': 'süslü parantez kapat', ':': 'iki nokta', ',': 'virgül', '.': 'nokta', '#': 'yorum', '"': 'çift tırnak', "'": 'kesme işareti', '\\': 'ters eğik çizgi', ';': 'noktalı virgül' };
function sesliAdTelaffuzu(ad) {
  if (sesliSozluk && Object.hasOwn(sesliSozluk, ad) && typeof sesliSozluk[ad] === 'string') return sesliSozluk[ad];
  if (Object.hasOwn(SESLI_ADLAR, ad)) return SESLI_ADLAR[ad];
  return ad.replace(/_/g, ' alt çizgi ').replace(/([a-zçğıöşü])([A-ZÇĞİÖŞÜ])/g, '$1 $2');
}
function sesliKodTelaffuzu(kod) {
  // Uzun işaretler önce eşleşir. Noktalı sayılar tanıma göre okunur, kod değişmez.
  return String(kod).replace(/\d+\.\d+|[\p{L}_][\p{L}\p{N}_]*|\+=|-=|\*=|\/=|==|!=|<=|>=|\*\*|->|=>|[^\s]/gu, parca => {
    if (/^\d+\.\d+$/.test(parca)) return ' ' + parca.replace('.', ' virgül ') + ' ';
    if (Object.hasOwn(SESLI_ISARETLER, parca)) return ' ' + SESLI_ISARETLER[parca] + ' ';
    return sesliAdTelaffuzu(parca);
  }).replace(/\s+/g, ' ').trim();
}
function sesliSistemDurumu() {
  if (!sesliMotor() || !window.SpeechSynthesisUtterance) return 'Bu ortam sistem sesini desteklemiyor.';
  const ses = sesliTurkceSes();
  return ses ? `Türkçe ses: ${ses.name}. Ses sağlayıcısına göre internet gerekebilir.` : 'Türkçe sistem sesi bulunamadı. İşletim sistemine Türkçe ses paketi kurun veya EMA paketini yükleyin.';
}
function sesliDurum() {
  return sesliSaglayici === 'ema' ? (sesliEmaDurum.hazir ? 'EMA Türkçe ses hazır. Yerelde, çevrimdışı çalışır. Ses yapay zekâ ile üretilir.' : sesliEmaDurum.ileti || 'EMA paketi kurulu değil; sistem sesi kullanılır.') : sesliSistemDurumu();
}
function sesliDurdur(istegiKes = true) {
  sesliIstek++; sesliBekleyen = null;
  sesliMotor()?.cancel();
  if (istegiKes) { sesliAbort?.abort(); sesliAbort = null; }
  sesliOynatici?.pause(); sesliOynatici = null;
  if (sesliAdres) URL.revokeObjectURL(sesliAdres);
  sesliAdres = null;
}
function sesliSistemOku(metin) {
  const motor = sesliMotor(), ses = sesliTurkceSes();
  if (!motor || !ses || !window.SpeechSynthesisUtterance) return false;
  motor.cancel();
  const soz = new window.SpeechSynthesisUtterance(metin);
  soz.lang = 'tr-TR'; soz.voice = ses; soz.rate = sesliHiz;
  soz.onerror = () => { motor.cancel(); };
  motor.speak(soz);
  return true;
}
function sesliOnbellegeAl(anahtar, veri) {
  const eski = sesliOnbellek.get(anahtar);
  if (eski) { sesliOnbellek.delete(anahtar); sesliOnbellekBoyut -= eski.size; }
  sesliOnbellek.set(anahtar, veri); sesliOnbellekBoyut += veri.size;
  while (sesliOnbellekBoyut > 16 * 1024 * 1024 || sesliOnbellek.size > 32) {
    const ilk = sesliOnbellek.keys().next().value;
    sesliOnbellekBoyut -= sesliOnbellek.get(ilk).size; sesliOnbellek.delete(ilk);
  }
}
async function sesliEmaOku(istek) {
  // Hızlı gezinmede kuyruk büyümez: yalnızca son satır bekler.
  if (sesliAnlikIstek) { sesliBekleyen = istek; return; }
  sesliAnlikIstek = istek;
  const { metin, sira, hiz } = istek, anahtar = JSON.stringify([metin, hiz]);
  try {
    let veri = sesliOnbellek.get(anahtar);
    if (veri) sesliOnbellegeAl(anahtar, veri);
    else {
      sesliAbort = new AbortController();
      const zaman = setTimeout(() => sesliAbort?.abort(), 32000);
      try {
        let r;
        for (let deneme = 0; deneme < 5; deneme++) {
          r = await fetch('/api/ses/oku', { method: 'POST', headers: { 'X-Orhunca-Anahtar': ANAHTAR || '', 'Content-Type': 'application/json' }, body: JSON.stringify({ metin, hiz }), signal: sesliAbort.signal });
          if (r.status !== 429 || sira !== sesliIstek) break;
          await new Promise(coz => setTimeout(coz, 150));
          if (sira !== sesliIstek) return;
        }
        if (!r.ok || !r.headers.get('Content-Type')?.startsWith('audio/wav')) throw new Error('EMA ses üretemedi.');
        veri = await r.blob();
        sesliOnbellegeAl(anahtar, veri);
      } finally { clearTimeout(zaman); }
    }
    if (sira !== sesliIstek) return;
    sesliAdres = URL.createObjectURL(veri);
    const oynatici = new Audio(sesliAdres); sesliOynatici = oynatici;
    const adres = sesliAdres;
    const temizle = () => { URL.revokeObjectURL(adres); if (sesliAdres === adres) { sesliAdres = null; sesliOynatici = null; } };
    oynatici.onended = temizle; oynatici.onerror = temizle;
    await oynatici.play();
  } catch (e) {
    if (sira === sesliIstek) {
      sesliEmaDurum.ileti = 'EMA sesi çalınamadı; sistem sesi deneniyor. Paketi yeniden kurabilirsiniz.';
      if (sesliAdres) URL.revokeObjectURL(sesliAdres);
      sesliAdres = null; sesliOynatici = null;
      sesliSistemOku(metin);
      if (D.modal?.tur === 'ayarlar') katmanlariCiz();
    }
  } finally {
    sesliAbort = null; sesliAnlikIstek = null;
    const sonraki = sesliBekleyen; sesliBekleyen = null;
    if (sonraki?.sira === sesliIstek) sesliEmaOku(sonraki);
  }
}
function sesliOku(metin, telaffuzHazir = false) {
  if (!sesliAcik) return false;
  sesliDurdur(false);
  metin = String(metin);
  if (!telaffuzHazir) metin = metin.replace(/[\p{L}_][\p{L}\p{N}_]*/gu, ad => {
    if (sesliSozluk && Object.hasOwn(sesliSozluk, ad) && typeof sesliSozluk[ad] === 'string') return sesliSozluk[ad];
    return Object.hasOwn(SESLI_ADLAR, ad) ? SESLI_ADLAR[ad] : ad;
  });
  metin = metin.slice(0, 1200);
  if (!metin.trim()) return false;
  if (sesliSaglayici === 'ema' && sesliEmaDurum.hazir) {
    sesliEmaOku({ metin, sira: sesliIstek, hiz: sesliHiz }); return true;
  }
  return sesliSistemOku(metin);
}
async function sesliDurumYenile() {
  clearTimeout(sesliDurumZamani);
  try { const d = await api('/api/ses/durum'); if (!d.hata) sesliEmaDurum = d; }
  catch { sesliEmaDurum.ileti = 'Ses sunucusuna erişilemedi.'; }
  if (D.modal?.tur === 'ayarlar' && !['sesliSozluk', 'sesliHiz'].includes(document.activeElement?.id)) katmanlariCiz();
  if (sesliEmaDurum.kuruluyor) sesliDurumZamani = setTimeout(sesliDurumYenile, 1000);
}
function sesliAyarlari() {
  return `<div class="ayar-bolum">Sesli kod desteği</div>
    <button class="dugme" data-e="sesliDegistir" aria-pressed="${sesliAcik}">Sesli modu ${sesliAcik ? 'kapat' : 'aç'}</button>
    <button class="dugme" data-e="sesliDene">Türkçe sesi dene</button>
    <button class="dugme" data-e="sesliSaglayiciSec" data-a="sistem" aria-pressed="${sesliSaglayici === 'sistem'}">Sistem sesi</button>
    <button class="dugme" data-e="sesliSaglayiciSec" data-a="ema" aria-pressed="${sesliSaglayici === 'ema'}">EMA Türkçe ses</button>
    <p role="status">${kac(sesliDurum())}</p>
    <button class="dugme" data-e="sesliPaketiKur" ${sesliEmaDurum.kuruluyor ? 'disabled' : ''}>${sesliEmaDurum.hazir ? 'EMA paketini doğrula / onar' : 'EMA Türkçe ses paketini indir'}</button>
    ${sesliEmaDurum.kuruluyor ? `<progress aria-label="Ses paketi kurulumu" max="100" value="${sesliEmaDurum.yuzde || 0}"></progress>` : ''}
    <p>${kac(sesliEmaDurum.ileti || '')}</p>
    <p>Model 27 MB; çalışma motoruyla toplam kurulum daha büyüktür. Python 3.11–3.13 gerekir. İlk kurulum internet kullanır; kodunuz seslendirme için internete gönderilmez.</p>
    <label for="sesliHiz">Okuma hızı</label><select id="sesliHiz" class="metin-girdi">${[0.75, 0.85, 1, 1.15, 1.3, 1.5].map(h => `<option value="${h}" ${h === sesliHiz ? 'selected' : ''}>${h.toString().replace('.', ',')} kat</option>`).join('')}</select>
    <p>Ctrl+Alt+Shift+S: aç/kapat · L: satır ve girinti · H: satırdaki hata · K: konum · O: dosya özeti · X: konuşmayı durdur. Hepsinde Ctrl+Alt+Shift tutulur.</p>
    <details><summary>Telaffuz sözlüğü</summary><label for="sesliSozluk">Adların okunuşu (JSON)</label><textarea id="sesliSozluk" class="metin-girdi" rows="4" spellcheck="false">${kac(JSON.stringify(sesliSozluk, null, 2))}</textarea><button class="dugme" data-e="sesliSozlukKaydet">Telaffuzları kaydet</button><p>Örnek: {"Orhunca": "Orhunca", "API": "a pe i"}. Kaynak kod değişmez.</p></details>
    <p>Mevcut ekran okuyucunuzla çift ses duyarsanız sesli modu kapatın. Bu özellik bir ekran okuyucunun yerini tutmaz.</p>`;
}
EYLEM.sesliSaglayiciSec = ad => {
  sesliDurdur(); sesliSaglayici = ad === 'ema' ? 'ema' : 'sistem';
  ayarYaz('sesliSaglayici', sesliSaglayici); sesliDurumYenile();
};
EYLEM.sesliPaketiKur = async () => {
  const r = await api('/api/ses/kur', {}).catch(() => ({ hata: 'Ses kurulumu başlatılamadı.' }));
  if (r.hata) return bildir(r.hata, true);
  sesliDurumYenile();
};
EYLEM.sesliSozlukKaydet = () => {
  try {
    const yeni = JSON.parse($('#sesliSozluk').value);
    if (!yeni || Array.isArray(yeni) || typeof yeni !== 'object' || Object.keys(yeni).length > 64 || Object.entries(yeni).some(([k,v]) => !k || k.length > 64 || typeof v !== 'string' || !v.trim() || v.length > 160)) throw new Error();
    sesliSozluk = yeni; ayarYaz('sesliSozluk', yeni); bildir('Telaffuz sözlüğü kaydedildi.');
  } catch { bildir('En fazla 64 ad içeren geçerli bir JSON sözlüğü girin. Adlar 64, okunuşlar 160 karakteri aşmamalı.', true); }
};
EYLEM.sesliDegistir = () => {
  sesliAcik = !sesliAcik; ayarYaz('sesliKod', sesliAcik);
  sesliDurdur(); sesliSonSatir = '';
  if (sesliAcik) bildir('Sesli kod desteği açıldı. ' + sesliDurum());
  if (D.modal?.tur === 'ayarlar') katmanlariCiz();
};
EYLEM.sesliDene = () => {
  const once = sesliAcik; sesliAcik = true;
  const sonuc = sesliOku('Orhunca Stüdyo Türkçe ses denemesi.'); sesliAcik = once;
  if (!sonuc) bildir(sesliDurum(), true);
};
document.addEventListener('change', e => {
  if (e.target.id === 'sesliHiz') {
    sesliHiz = Math.max(0.75, Math.min(1.5, Number(e.target.value) || 1));
    ayarYaz('sesliHiz', sesliHiz); sesliDurdur();
  }
});
const sesliEskiAyarlar = EYLEM.ayarlarModal;
EYLEM.ayarlarModal = (...a) => { sesliEskiAyarlar?.(...a); sesliDurumYenile(); };
if (typeof api === 'function') sesliDurumYenile();
function sesliSatirBilgisi(ta) {
  const once = ta.value.slice(0, ta.selectionStart).split('\n');
  const satir = once.length, metin = ta.value.split('\n')[satir - 1] || '';
  const girinti = metin.match(/^[\t ]*/)[0];
  return { satir, sutun: once.at(-1).length + 1, metin, girinti: `${(girinti.match(/ /g) || []).length} boşluk, ${(girinti.match(/\t/g) || []).length} sekme` };
}
function sesliSatiriOku(ta) {
  const b = sesliSatirBilgisi(ta);
  sesliOku(`Satır ${b.satir}. Girinti ${b.girinti}. ${sesliKodTelaffuzu(b.metin.trim()) || 'Boş satır'}`, true);
}
document.addEventListener('keydown', e => {
  if (!e.ctrlKey || !e.altKey || !e.shiftKey || e.metaKey || e.getModifierState?.('AltGraph')) return;
  const tus = e.key.toLowerCase();
  if (!['s', 'l', 'h', 'k', 'o', 'x'].includes(tus)) return;
  if (tus === 's') { e.preventDefault(); e.stopImmediatePropagation(); EYLEM.sesliDegistir(); return; }
  if (tus === 'x') { e.preventDefault(); e.stopImmediatePropagation(); sesliDurdur(); return; }
  const ta = document.activeElement;
  if (!sesliAcik || ta?.id !== 'kodAlani') return;
  e.preventDefault(); e.stopImmediatePropagation();
  const b = sesliSatirBilgisi(ta), s = etkinSekme();
  if (tus === 'l') sesliSatiriOku(ta);
  if (tus === 'k') sesliOku(`Satır ${b.satir}, sütun ${b.sutun}.`);
  const hatalar = D.sorunlar.filter(h => h.dosya && normal(h.dosya) === normal(tamYol(s.yol)));
  if (tus === 'h') sesliOku(hatalar.filter(h => h.satir === b.satir).map(h => h.mesaj || h.metin || 'Hata').join('. ') || 'Bu satırda bildirilen hata yok.');
  if (tus === 'o') sesliOku(`${s.yol}. ${ta.value.split('\n').length} satır, ${hatalar.length} hata.`);
}, true);
document.addEventListener('keyup', e => {
  if (!sesliAcik || e.target.id !== 'kodAlani' || !['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown'].includes(e.key) || e.ctrlKey || e.altKey || e.metaKey) return;
  const b = sesliSatirBilgisi(e.target), kimlik = `${D.etkin}:${b.satir}`;
  if (kimlik !== sesliSonSatir) { sesliSonSatir = kimlik; sesliSatiriOku(e.target); }
});
sesliMotor()?.addEventListener('voiceschanged', () => {
  if (D.modal?.tur === 'ayarlar') katmanlariCiz();
});
window.addEventListener('pagehide', () => sesliDurdur());
const sesliYardim = document.createElement('p');
sesliYardim.id = 'sesliKodYardim';
sesliYardim.textContent = 'Sesli kod desteği: Ctrl Alt Shift S ile açılır. Ctrl Alt Shift L satırı, H hatayı, K konumu, O dosya özetini okur. Ctrl Alt Shift X konuşmayı durdurur.';
sesliYardim.style.cssText = 'position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%)';
document.body.appendChild(sesliYardim);
