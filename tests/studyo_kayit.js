// Gerçek Stüdyo modülleriyle gecikmeli ağ yanıtları ve sekme yarışları.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');

function bekleyen() {
  let bitir;
  const soz = new Promise(r => { bitir = r; });
  return { soz, bitir };
}
function ortam() {
  const zamanlar = new Map(); let sira = 0;
  const c = vm.createContext({
    D: { proje: { yol: '/p' }, ekran: 'duzenleyici', sekmeler: [], otomatikKaydet: true },
    GIRDI: {},
    document: { addEventListener() {}, hidden: false }, window: { addEventListener() {} },
    setInterval() {}, setTimeout(f) { zamanlar.set(++sira, f); return sira; }, clearTimeout(id) { zamanlar.delete(id); },
    tamYol: y => '/p/' + y, sorgu: o => new URLSearchParams(o).toString(), sonParca: y => y,
    etkinSekme() { return c.D.sekmeler.find(s => s.yol === c.D.etkin); },
    cizSekmeler() {}, gitYenilePlanla() {}, kayittanSonra() {}, bildir() {}, guncelle() {},
    katmanlariCiz() {}, onayla: async () => true,
  });
  for (const dosya of ['proje.js', 'gezgin.js']) {
    // index.html: proje.js, EYLEM tanımından önce yüklenir.
    if (dosya === 'gezgin.js') c.EYLEM = {};
    vm.runInContext(fs.readFileSync(path.join(__dirname, '../studio/moduller', dosya), 'utf8'), c);
  }
  c.sekme = (yol, icerik = 'yeni') => {
    const s = { yol, icerik, kayitli: 'ilk', zaman: 1, surum: 'v1' };
    c.D.sekmeler.push(s); c.D.etkin = yol; return s;
  };
  c.zamanlariCalistir = async () => { const isler = [...zamanlar.values()]; zamanlar.clear(); for (const f of isler) await f(); };
  return c;
}

(async () => {
  {
    const c = ortam(); const a = c.sekme('a'); c.kaydetmeyiPlanla();
    const b = c.sekme('b'); c.kaydetmeyiPlanla(); c.sekme('c', 'ilk');
    const yollar = [];
    c.api = async (_, g) => { yollar.push(g.yol); return { tamam: true, zaman: 2, surum: 'v2' }; };
    await c.zamanlariCalistir();
    assert.deepEqual(yollar, ['/p/a', '/p/b']);
    assert.equal(a.kayitli, 'yeni'); assert.equal(b.kayitli, 'yeni');
  }
  {
    const c = ortam(); const s = c.sekme('a'); const ilk = bekleyen(); const istekler = [];
    c.api = async (_, g) => { istekler.push(g); return istekler.length === 1 ? ilk.soz : { tamam: true, surum: 'v3', zaman: 3 }; };
    const bir = c.kaydet(s); s.icerik = 'daha yeni'; const iki = c.kaydet(s);
    assert.equal(istekler.length, 1);
    ilk.bitir({ tamam: true, surum: 'v2', zaman: 2 });
    await Promise.all([bir, iki]);
    assert.equal(istekler[1].surum, 'v2'); assert.equal(istekler[1].icerik, 'daha yeni');
    assert.equal(s.kayitli, 'daha yeni');
  }
  {
    const c = ortam(); const s = c.sekme('a', 'ilk'); const oku = bekleyen(); const basladi = bekleyen();
    c.api = async y => { if (y === '/api/dosya/zamanlar') return { zamanlar: { '/p/a': 2 } }; basladi.bitir(); return oku.soz; };
    const islem = c.disDegisiklikleriDenetle(); await basladi.soz;
    s.icerik = 'istek beklenirken yazılan'; oku.bitir({ icerik: 'disk', zaman: 2, surum: 'v2' }); await islem;
    assert.equal(s.icerik, 'istek beklenirken yazılan'); assert.equal(s.kayitli, 'ilk');
  }
  {
    const c = ortam(); const s = c.sekme('a'); let istek = 0;
    c.api = async () => { istek++; return { hata: 'Dosya değişti', cakisma: true, surum: 'v2', icerik: 'disk' }; };
    assert.equal(await c.kaydet(s, true, true), false);
    assert.equal(s.icerik, 'yeni'); assert.equal(s.kayitli, 'ilk'); assert.equal(s.zaman, 1);
    await c.kaydet(s, true, true); assert.equal(istek, 1);
  }
  {
    const c = ortam(); const s = c.sekme('a', 'ilk'); let giden;
    c.D.modal = { tur: 'kayitCakismasi', proje: c.D.proje, sekme: s, disk: { surum: 'v2', icerik: 'disk' } };
    c.api = async (_, g) => { giden = g; return { tamam: true, zaman: 3, surum: 'v3' }; };
    await c.kayitCakismasiCoz('kaydet');
    assert.equal(giden.icerik, 'ilk'); assert.equal(giden.surum, 'v2');
    assert.equal(s.kayitli, 'ilk');
  }
  console.log('5 Stüdyo kayıt senaryosu geçti.');
})().catch(e => { console.error(e); process.exitCode = 1; });
