// Sitedeki derleyicinin (calisma/oyun.wasm) bütün deneme örneklerini derlediğini ve
// konsol örneklerinin örnekler/*.beklenen çıktısını verdiğini sınar (Node.js).
//   node araclar/site_sinamasi.mjs _site
import { readFileSync, existsSync } from 'fs';
import { createRequire } from 'module';
import path from 'path';

const site = process.argv[2] || '_site';
const require = createRequire(import.meta.url);
const Orhunca = require(path.resolve(site, 'calisma/orhunca.js'));
const { instance } = await WebAssembly.instantiate(readFileSync(path.join(site, 'calisma/oyun.wasm')), {});
const d = instance.exports;
const rt = readFileSync(path.join(site, 'calisma/orhunca_rt.wasm'));
const ornekler = JSON.parse(readFileSync(path.join(site, 'ornekler.json'), 'utf8'));

function derle(kaynak) {
  const b = new TextEncoder().encode(kaynak);
  const p = d.ayir(b.length);
  new Uint8Array(d.memory.buffer, p, b.length).set(b);
  const r = d.derle(p, b.length);
  d.birak(p, b.length);
  const n = new DataView(d.memory.buffer).getUint32(r, true);
  const tur = new Uint8Array(d.memory.buffer, r + 4, 1)[0];
  const veri = new Uint8Array(d.memory.buffer, r + 5, n - 1).slice();
  d.birak(r, n);
  return { tur, veri };
}

let hata = 0;
for (const o of ornekler) {
  const { tur, veri } = derle(o.kod);
  if (tur === 1) { console.log(`✗ ${o.kimlik}: ${new TextDecoder().decode(veri)}`); hata++; continue; }
  if (tur === 2) { console.log(`✓ ${o.kimlik} (arayüz)`); continue; }
  let cikti = '';
  const kod = await Orhunca.calistir({
    calismaZamani: rt, program: veri,
    cikti: (_, b) => { cikti += new TextDecoder().decode(b); },
    satirOku: () => null, dosyalar: Orhunca.bellekDosyalari(), ortam: {}, argumanlar: [],
  });
  const beklenenYol = ['örnekler', o.kaynak || ''].join('/');
  const beklenen = o.dosya && existsSync(o.dosya.replace(/\.ohc$/, '.beklenen'))
    ? readFileSync(o.dosya.replace(/\.ohc$/, '.beklenen'), 'utf8') : null;
  if (kod !== 0 && o.kimlik !== 'girdi') { console.log(`✗ ${o.kimlik}: çıkış kodu ${kod}\n${cikti}`); hata++; continue; }
  if (beklenen !== null && beklenen !== cikti) { console.log(`✗ ${o.kimlik}: çıktı farklı (${beklenenYol})\n${cikti}`); hata++; continue; }
  console.log(`✓ ${o.kimlik}`);
}
// Kurulabilir web uygulaması: manifest simgeleri ve çevrimdışı önbelleğe alınan dosyalar var mı?
const manifest = JSON.parse(readFileSync(path.join(site, 'manifest.webmanifest'), 'utf8'));
const sw = readFileSync(path.join(site, 'sw.js'), 'utf8');
if (sw.includes('__SURUM__')) { console.log('✗ sw.js: önbellek sürümü yazılmamış'); hata++; }
const onbellek = JSON.parse(/const DOSYALAR = (\[[^\]]*\])/.exec(sw)[1].replace(/'/g, '"').replace(/,\s*\]/, ']'));
for (const d of [manifest.start_url, ...manifest.icons.map((i) => i.src), ...onbellek]) {
  if (!existsSync(path.join(site, d))) { console.log(`✗ çevrimdışı dosya eksik: ${d}`); hata++; }
}
console.log(`✓ kurulabilir web uygulaması (${onbellek.length} dosya önbellekte)`);
process.exit(hata ? 1 : 0);
