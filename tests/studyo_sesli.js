// Ses motorunun bulunmaması, geç yüklenmesi ve kod okuma davranışları.
'use strict';
const fs = require('fs'), vm = require('vm'), assert = require('assert');
const olaylar = {}, soylenenler = [];
let sesler = [], iptal = 0;
const ta = { id: 'kodAlani', value: 'işlev başla():\n    yaz("merhaba")\n', selectionStart: 20 };
const motor = { getVoices: () => sesler, cancel: () => iptal++, speak: s => soylenenler.push(s.text), addEventListener: () => {} };
const baglam = { window: { speechSynthesis: motor, SpeechSynthesisUtterance: function(t) { this.text = t; }, addEventListener: () => {} },
  document: { activeElement: ta, addEventListener: (ad, f) => olaylar[ad] = f, createElement: () => ({ style: {} }), body: { appendChild: () => {} } },
  ayarOku: () => false, ayarYaz: () => {}, EYLEM: {}, D: { etkin: 'ana.ohc', sorunlar: [{ dosya: 'ana.ohc', satir: 2, mesaj: 'Tanımsız isim' }] },
  etkinSekme: () => ({ yol: 'ana.ohc' }), normal: x => x, tamYol: x => x, kac: x => x, katmanlariCiz: () => {}, bildir: () => {} };
vm.createContext(baglam);
vm.runInContext(fs.readFileSync('studio/moduller/sesli.js', 'utf8'), baglam);
assert.equal(baglam.sesliOku('kapalı'), false);
baglam.EYLEM.sesliDegistir();
assert.equal(baglam.sesliOku('ses yok'), false);
sesler = [{ lang: 'tr_TR', name: 'Türkçe' }];
assert.equal(baglam.sesliOku('deneme'), true);
assert.equal(soylenenler.at(-1), 'deneme');
function tus(key, ek = {}) { olaylar.keydown({ ctrlKey: true, altKey: true, key, preventDefault() {}, ...ek }); }
tus('l'); assert.match(soylenenler.at(-1), /Satır 2.*4 boşluk.*merhaba/);
tus('h'); assert.equal(soylenenler.at(-1), 'Tanımsız isim');
tus('k'); assert.match(soylenenler.at(-1), /Satır 2, sütun/);
tus('o'); assert.match(soylenenler.at(-1), /3 satır, 1 hata/);
const sayi = soylenenler.length;
tus('l', { getModifierState: () => true }); assert.equal(soylenenler.length, sayi);
tus('x'); assert.ok(iptal > 0);
baglam.EYLEM.sesliDegistir(); assert.equal(baglam.sesliOku('kapandı'), false);
baglam.window.speechSynthesis = undefined; baglam.EYLEM.sesliDene();
assert.match(baglam.sesliDurum(), /desteklemiyor/);
console.log('Sesli kod senaryoları geçti.');
