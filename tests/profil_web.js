/* Teslim HTML'sindeki gerçek WebAssembly programı açık profil ayarını okur. */
'use strict';
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
const html = fs.readFileSync(process.argv[2], 'utf8');
const betikler = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]);
assert.equal(betikler.length, 2, 'Açık ayar script etiketini kapatamaz.');
const oge = () => ({ textContent: '', disabled: false, addEventListener() {}, appendChild(n) { this.textContent += n.textContent; } });
const ogeler = { cikti: oge(), durum: oge(), yeniden: oge() };
const yerel = new Map();
const baglam = vm.createContext({
  console, WebAssembly, TextEncoder, TextDecoder, Uint8Array, ArrayBuffer, setTimeout, clearTimeout,
  crypto: require('node:crypto').webcrypto,
  atob: s => Buffer.from(s, 'base64').toString('binary'),
  document: { title: 'Profil sınaması', getElementById: id => ogeler[id], createElement: oge, createTextNode: s => ({ textContent: s }) },
  localStorage: { getItem: k => yerel.get(k) ?? null, setItem: (k, v) => yerel.set(k, v), removeItem: k => yerel.delete(k), key: i => [...yerel.keys()][i] ?? null, get length() { return yerel.size; } },
});
baglam.window = baglam;
for (const betik of betikler) vm.runInContext(betik, baglam);
(async () => {
  for (let i = 0; i < 100 && ogeler.durum.textContent !== 'bitti'; i++) await new Promise(r => setTimeout(r, 20));
  assert.equal(ogeler.durum.textContent, 'bitti', ogeler.cikti.textContent);
  assert.equal(ogeler.cikti.textContent, process.argv[3] + '\n');
})().catch(e => { console.error(e); process.exitCode = 1; });
