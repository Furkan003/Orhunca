// README'deki tanıtım GIF'i (docs/ekran/tanitim.gif): Stüdyo'da kod yazılır, F5 ile
// çalıştırılır, çıktı görünür.
//   cargo build --release && node araclar/tanitim_gif.mjs     (Playwright ve ffmpeg gerekir)
import { spawn, execFileSync } from 'child_process';
import { mkdtempSync, readdirSync, rmSync, writeFileSync } from 'fs';
import { createRequire } from 'module';
import os from 'os';
import path from 'path';

const require = createRequire(import.meta.url);
let pw;
try {
  pw = require('playwright');
} catch {
  pw = require(path.join(process.execPath, '../../lib/node_modules/playwright'));
}
const kok = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const ev = mkdtempSync(path.join(os.tmpdir(), 'orhunca-gif-'));
const video = path.join(ev, 'video');

const studyo = spawn(path.join(kok, 'target/release/orhunca'), ['stüdyo', '--tarayıcı-açma', '--kapı', '0'], {
  env: { ...process.env, HOME: ev, USERPROFILE: ev, XDG_CONFIG_HOME: path.join(ev, '.config'), USER: 'Ayşe' },
  stdio: ['ignore', 'pipe', 'inherit'],
});
const adres = await new Promise((t) => {
  let b = '';
  studyo.stdout.on('data', (d) => {
    b += d;
    const m = b.match(/http:\/\/\S+/);
    if (m) t(m[0]);
  });
});
const taban = adres.split('/?')[0];
const anahtar = new URL(adres).searchParams.get('anahtar');
const r = await fetch(taban + '/api/proje/olustur', {
  method: 'POST',
  headers: { 'X-Orhunca-Anahtar': anahtar, 'Content-Type': 'application/json' },
  body: JSON.stringify({ sablon: 'konsol', ad: 'merhaba', konum: path.join(ev, 'Projeler'), git: false, ornek: false }),
}).then((x) => x.json());
const dosya = path.join(ev, 'Projeler', 'merhaba', 'ana.ohc');
writeFileSync(dosya, '');

const tarayici = await pw.chromium.launch();
const sayfa = await tarayici.newPage({ viewport: { width: 1100, height: 620 } });
await sayfa.goto(taban + '/?acilis=0');
await sayfa.evaluate(() => localStorage.setItem('orhunca.acilis', 'false'));
await sayfa.close();
const baglam = await tarayici.newContext({ viewport: { width: 1100, height: 620 }, recordVideo: { dir: video, size: { width: 1100, height: 620 } } });
const s = await baglam.newPage();
await s.goto(taban + '/?acilis=0');
await s.evaluate(() => localStorage.setItem('orhunca.acilis', 'false'));
await s.goto(adres + '&acilis=0&ac=' + encodeURIComponent(dosya));
await s.waitForTimeout(1200);
await s.click('#kodAlani');
const kod = [
  'fiil (kişi: metin)\'yi selamla:',
  '"Merhaba, " + kişi + "!"\'yı yaz.',
  '',
  '"Ayşe"\'yi selamla.',
  '',
  'notlar = [85, 92, 78, 99]',
  'her n için notlar\'dan:',
  'eğer n 90\'dan büyükse:',
  '("Tebrikler: " + n)\'yi yaz.',
];
for (const [i, satir] of kod.entries()) {
  // Girintiyi düzenleyici kendiliğinden ekler; blok bitince geri alınır.
  if (i === 2 || i === 4) await s.keyboard.press('Shift+Tab');
  await s.keyboard.type(satir, { delay: 38 });
  if (i < kod.length - 1) await s.keyboard.press('Enter');
}
await s.waitForTimeout(500);
await s.keyboard.press('F5');
await s.waitForFunction(() => document.body.innerText.includes('Tebrikler: 99'), null, { timeout: 30000 });
await s.waitForTimeout(2500);
await baglam.close();
await tarayici.close();
studyo.kill();

const webm = path.join(video, readdirSync(video)[0]);
const palet = path.join(ev, 'palet.png');
const cikti = path.join(kok, 'docs/ekran/tanitim.gif');
const filtre = 'fps=12,scale=900:-1:flags=lanczos';
execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-ss', '1.0', '-i', webm, '-vf', `${filtre},palettegen=max_colors=96`, palet]);
execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-ss', '1.0', '-i', webm, '-i', palet, '-lavfi', `${filtre}[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=4`, cikti]);
console.log('✓', cikti, r.hata || '');
rmSync(ev, { recursive: true, force: true });
