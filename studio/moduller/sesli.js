/* Orhunca Stüdyo — İsteğe bağlı sesli kod desteği. */
'use strict';
let sesliAcik = ayarOku('sesliKod', false);
let sesliSonSatir = '';
const sesliMotor = () => window.speechSynthesis;
const sesliTurkceSes = () => sesliMotor()?.getVoices().find(s => /^tr(?:-|$)/i.test(s.lang.replace('_', '-')));
function sesliDurum() {
  if (!sesliMotor() || !window.SpeechSynthesisUtterance) return 'Bu ortam seslendirmeyi desteklemiyor.';
  const ses = sesliTurkceSes();
  return ses ? `Türkçe ses: ${ses.name}. Ses sağlayıcısına göre internet gerekebilir.` : 'Türkçe ses henüz bulunamadı. İşletim sistemine Türkçe ses paketi kurun; ses listesi yüklenince yeniden deneyin.';
}
function sesliOku(metin) {
  const motor = sesliMotor(), ses = sesliTurkceSes();
  if (!sesliAcik || !motor || !ses || !window.SpeechSynthesisUtterance) return false;
  motor.cancel(); // Eski bildirimler ve hızlı gezinme konuşma kuyruğu biriktirmez.
  const soz = new window.SpeechSynthesisUtterance(String(metin).slice(0, 4000));
  soz.lang = 'tr-TR'; soz.voice = ses;
  soz.onerror = () => { motor.cancel(); };
  motor.speak(soz);
  return true;
}
function sesliAyarlari() {
  return `<div class="ayar-bolum">Sesli kod desteği</div>
    <button class="dugme" data-e="sesliDegistir" aria-pressed="${sesliAcik}">Sesli modu ${sesliAcik ? 'kapat' : 'aç'}</button>
    <button class="dugme" data-e="sesliDene">Türkçe sesi dene</button>
    <p role="status">${kac(sesliDurum())}</p>
    <p>Ctrl+Alt+S: aç/kapat · Ctrl+Alt+L: satır ve girinti · Ctrl+Alt+H: satırdaki hata · Ctrl+Alt+K: konum · Ctrl+Alt+O: dosya özeti · Ctrl+Alt+X: konuşmayı durdur.</p>
    <p>Mevcut ekran okuyucunuzla çift ses duyarsanız sesli modu kapatın. Bu özellik bir ekran okuyucunun yerini tutmaz.</p>`;
}
EYLEM.sesliDegistir = () => {
  sesliAcik = !sesliAcik; ayarYaz('sesliKod', sesliAcik);
  sesliMotor()?.cancel(); sesliSonSatir = '';
  if (sesliAcik) bildir('Sesli kod desteği açıldı. ' + sesliDurum());
  if (D.modal?.tur === 'ayarlar') katmanlariCiz();
};
EYLEM.sesliDene = () => {
  const once = sesliAcik; sesliAcik = true;
  const sonuc = sesliOku('Orhunca Stüdyo Türkçe ses denemesi.'); sesliAcik = once;
  if (!sonuc) bildir(sesliDurum(), true);
};
function sesliSatirBilgisi(ta) {
  const once = ta.value.slice(0, ta.selectionStart).split('\n');
  const satir = once.length, metin = ta.value.split('\n')[satir - 1] || '';
  const girinti = metin.match(/^[\t ]*/)[0];
  return { satir, sutun: once.at(-1).length + 1, metin, girinti: `${(girinti.match(/ /g) || []).length} boşluk, ${(girinti.match(/\t/g) || []).length} sekme` };
}
function sesliSatiriOku(ta) {
  const b = sesliSatirBilgisi(ta);
  sesliOku(`Satır ${b.satir}. Girinti ${b.girinti}. ${b.metin.trim() || 'Boş satır'}`);
}
document.addEventListener('keydown', e => {
  if (!e.ctrlKey || !e.altKey || e.shiftKey || e.metaKey || e.getModifierState?.('AltGraph')) return;
  const tus = e.key.toLowerCase();
  if (!['s', 'l', 'h', 'k', 'o', 'x'].includes(tus)) return;
  if (tus === 's') { e.preventDefault(); EYLEM.sesliDegistir(); return; }
  if (tus === 'x') { e.preventDefault(); sesliMotor()?.cancel(); return; }
  const ta = document.activeElement;
  if (!sesliAcik || ta?.id !== 'kodAlani') return;
  e.preventDefault();
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
window.addEventListener('pagehide', () => sesliMotor()?.cancel());
const sesliYardim = document.createElement('p');
sesliYardim.id = 'sesliKodYardim';
sesliYardim.textContent = 'Sesli kod desteği: Ctrl Alt S ile açılır. Ctrl Alt L satırı, H hatayı, K konumu, O dosya özetini okur. Ctrl Alt X konuşmayı durdurur.';
sesliYardim.style.cssText = 'position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%)';
document.body.appendChild(sesliYardim);
