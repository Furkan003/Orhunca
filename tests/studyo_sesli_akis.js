// EMA yanıtları geç gelince eski satır çalınmaz; son satır ve önbellek kullanılır.
'use strict';
const fs = require('fs'), vm = require('vm'), assert = require('assert');
const istekler = [], calinanlar = [], sistem = [], adresler = new Map(), olaylar = {};
let adresNo = 0, durdurulan = 0;
const baglam = {
  setTimeout, clearTimeout, AbortController,
  window: { speechSynthesis: { getVoices: () => [{ lang: 'tr-TR' }], cancel() {}, speak: s => sistem.push(s.text), addEventListener() {} }, SpeechSynthesisUtterance: function(t) { this.text = t; }, addEventListener() {} },
  document: { activeElement: {}, addEventListener: (ad, f) => olaylar[ad] = f, createElement: () => ({ style: {} }), body: { appendChild() {} } },
  ayarOku: (ad, varsayilan) => ad === 'sesliKod' ? true : ad === 'sesliSaglayici' ? 'ema' : varsayilan,
  ayarYaz() {}, EYLEM: {}, D: {}, ANAHTAR: 'deneme-anahtari', kac: x => x, katmanlariCiz() {}, bildir() {},
  api: async () => ({ hazir: true, kuruluyor: false }),
  URL: { createObjectURL: b => { const a = 'ses:' + ++adresNo; adresler.set(a, b); return a; }, revokeObjectURL: a => adresler.delete(a) },
  Audio: class { constructor(a) { this.a = a; } pause() { durdurulan++; } async play() { calinanlar.push(adresler.get(this.a).metin); } },
  fetch: (yol, oz) => new Promise((coz, reddet) => { istekler.push({ yol, oz, coz, reddet, veri: JSON.parse(oz.body) }); }),
};
vm.createContext(baglam);
vm.runInContext(fs.readFileSync('studio/moduller/sesli.js', 'utf8'), baglam);
const tur = () => new Promise(coz => setImmediate(coz));
function yanit(i) { istekler[i].coz({ ok: true, status: 200, headers: { get: () => 'audio/wav' }, blob: async () => ({ size: 120, metin: istekler[i].veri.metin }) }); }
(async () => {
  await tur();
  baglam.sesliOku('eski satır');
  assert.equal(istekler.length, 1);
  assert.equal(istekler[0].oz.headers['X-Orhunca-Anahtar'], 'deneme-anahtari');
  baglam.sesliOku('ara satır'); baglam.sesliOku('son satır');
  assert.equal(istekler.length, 1);
  yanit(0); await tur();
  assert.equal(calinanlar.length, 0);
  assert.equal(istekler.length, 2);
  assert.equal(istekler[1].veri.metin, 'son satır');
  yanit(1); await tur();
  assert.deepEqual(calinanlar, ['son satır']);
  baglam.sesliOku('son satır'); await tur();
  assert.equal(istekler.length, 2); // Aynı satır yeniden üretilmedi.
  assert.deepEqual(calinanlar, ['son satır', 'son satır']);
  baglam.sesliDurdur(); assert.ok(durdurulan > 0); assert.equal(adresler.size, 0);
  baglam.sesliOku('durdurulan satır'); baglam.sesliDurdur();
  yanit(2); await tur(); assert.equal(calinanlar.length, 2);
  baglam.sesliOku('hata sonrası'); istekler[3].reddet(new Error('motor yok')); await tur();
  assert.equal(sistem.at(-1), 'hata sonrası');
  for (let i = 0; i < 40; i++) baglam.sesliOnbellegeAl(String(i), { size: 1000000 });
  assert.ok(vm.runInContext('sesliOnbellek.size <= 32 && sesliOnbellekBoyut <= 16*1024*1024', baglam));
  console.log('EMA son satır, iptal, önbellek ve sistem sesine dönüş senaryoları geçti.');
})().catch(e => { console.error(e); process.exitCode = 1; });
