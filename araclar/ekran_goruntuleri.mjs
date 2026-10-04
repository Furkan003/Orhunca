// README ve site için Stüdyo ekran görüntülerini (docs/ekran/*.png) üretir.
//   cargo build --release && node araclar/ekran_goruntuleri.mjs
// Gerekenler: Playwright (npm i -g playwright) ve Chromium. Geçici bir ev klasörüyle
// Stüdyo başlatılır, örnek projeler API ile oluşturulur, ekranlar sırayla çekilir.
import { spawn } from 'child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'fs';
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
const orhunca = path.join(kok, 'target/release/orhunca');
const cikti = path.join(kok, 'docs/ekran');
const ev = mkdtempSync(path.join(os.tmpdir(), 'orhunca-ekran-'));

const studyo = spawn(orhunca, ['stüdyo', '--tarayıcı-açma', '--kapı', '0'], {
  env: { ...process.env, HOME: ev, USERPROFILE: ev, APPDATA: ev, XDG_CONFIG_HOME: path.join(ev, '.config'), USER: 'Ayşe' },
  stdio: ['ignore', 'pipe', 'inherit'],
});
const adres = await new Promise((tamam) => {
  let b = '';
  studyo.stdout.on('data', (d) => {
    b += d;
    const m = b.match(/http:\/\/\S+/);
    if (m) tamam(m[0]);
  });
});
const taban = adres.split('/?')[0];
const anahtar = new URL(adres).searchParams.get('anahtar');
const api = async (yol, govde) => {
  const r = await fetch(taban + yol, {
    method: 'POST',
    headers: { 'X-Orhunca-Anahtar': anahtar, 'Content-Type': 'application/json' },
    body: JSON.stringify(govde),
  });
  return r.json();
};

const projeler = path.join(ev, 'Orhunca', 'Projeler');
const olustur = async (sablon, ad, ornek = true) => {
  const r = await api('/api/proje/olustur', { sablon, ad, konum: projeler, git: false, ornek });
  if (r.hata) throw new Error(r.hata);
  return path.join(projeler, ad);
};
const konsol = await olustur('konsol', 'not_hesaplama', false);
writeFileSync(
  path.join(konsol, 'ana.ohc'),
  `# Sınıfın not ortalaması ve harf notları
model Öğrenci:
    ad: metin
    not: sayı

işlev harf_notu(n: sayı) -> metin:
    eğer n >= 85 ise:
        döndür "A"
    değilse eğer n >= 70 ise:
        döndür "B"
    değilse eğer n >= 50 ise:
        döndür "C"
    döndür "F"

sınıf = [
    Öğrenci(ad: "Ayşe", not: 92),
    Öğrenci(ad: "Mehmet", not: 74),
    Öğrenci(ad: "Elif", not: 88),
    Öğrenci(ad: "Can", not: 45),
]
toplam_not = 0
her ö için sınıf'tan:
    toplam_not += ö.not
    (ö.ad + ": " + ö.not + " (" + harf_notu(ö.not) + ")")'yi yaz.

("Ortalama: " + toplam_not / uzunluk(sınıf))'yı yaz.
`,
);
const arayuz = await olustur('arayuz', 'sinif_defteri', false);
writeFileSync(path.join(arayuz, 'uygulama.ohc'), readFileSync(path.join(kok, 'örnekler/arayüz/sınıf_defteri.ohc')));
const web = await olustur('web_sitesi', 'okul_sitesi', true);
await olustur('sayi_tahmin', 'tahmin_oyunu', true);

const tarayici = await pw.chromium.launch();
const sayfa = await tarayici.newPage({ viewport: { width: 1360, height: 860 }, deviceScaleFactor: 1 });
const ac = async (yol, tema = 'koyu') => {
  // Ayarlar önce yazılır (uygulama açılınca adres çubuğundaki parametreleri temizler).
  await sayfa.goto(taban + '/?acilis=0');
  await sayfa.evaluate((t) => {
    localStorage.setItem('orhunca.tema', JSON.stringify(t));
    localStorage.setItem('orhunca.acilis', 'false');
  }, tema);
  await sayfa.goto(adres + '&acilis=0' + (yol ? '&ac=' + encodeURIComponent(yol) : ''));
  await sayfa.waitForTimeout(900);
};
const cek = async (ad) => {
  console.log('…', ad);
  await sayfa.screenshot({ path: path.join(cikti, ad + '.png') });
  console.log('✓', ad);
};

// Başlangıç ve yeni proje
await ac(null);
await cek('baslangic');
await sayfa.keyboard.press('Control+Shift+N');
await sayfa.waitForTimeout(500);
await cek('yeni-proje');

// Düzenleyici: programı çalıştır
await ac(path.join(konsol, 'ana.ohc'));
await sayfa.keyboard.press('F5');
await sayfa.waitForFunction(() => document.body.innerText.includes('Ortalama'), null, { timeout: 30000 });
await sayfa.waitForTimeout(400);
await cek('duzenleyici');

// Açık tema
await ac(path.join(konsol, 'ana.ohc'), 'acik');
await sayfa.keyboard.press('F5');
await sayfa.waitForFunction(() => document.body.innerText.includes('Ortalama'), null, { timeout: 30000 });
await sayfa.waitForTimeout(400);
await cek('acik-tema');

// Hata ayıklama: döngüdeki satırda kesme noktası
await ac(path.join(konsol, 'ana.ohc'));
await sayfa.click('#satirNolari >> text="23"');
await sayfa.keyboard.press('F6');
await sayfa.waitForFunction(() => document.body.innerText.includes('DEĞİŞKENLER') || document.body.innerText.includes('Değişkenler'), null, { timeout: 30000 });
await sayfa.waitForTimeout(800);
await cek('hata-ayiklama');
await sayfa.keyboard.press('Shift+F5');

// Yazarken hata gösterimi
await ac(path.join(konsol, 'ana.ohc'));
await sayfa.click('#kodAlani');
await sayfa.keyboard.press('Control+End');
await sayfa.keyboard.type('\nortlama\'yı yaz.');
await sayfa.waitForTimeout(1500);
await cek('canli-hata');

// Arayüz uygulaması canlı önizlemede
await ac(path.join(arayuz, 'uygulama.ohc'));
await sayfa.keyboard.press('F5');
await sayfa.waitForSelector('#onizlemeCerceve', { timeout: 30000 });
await sayfa.waitForTimeout(2500);
const cerceve = sayfa.frameLocator('#onizlemeCerceve');
await cerceve.locator('text=Grafik').first().click().catch(() => {});
await sayfa.waitForTimeout(800);
await cek('arayuz');

// Web sitesi canlı önizlemede
await ac(path.join(web, 'sunucu.ohc'));
await sayfa.keyboard.press('F5');
await sayfa.waitForSelector('#onizlemeCerceve', { timeout: 60000 });
await sayfa.waitForTimeout(3000);
await cek('canli-onizleme');
await sayfa.keyboard.press('Shift+F5');

// Dersler paneli
await ac(path.join(konsol, 'ana.ohc'));
await sayfa.click('[title="Dersler"]');
await sayfa.waitForTimeout(600);
await sayfa.locator('text=Değişkenler').first().click().catch(() => {});
await sayfa.waitForTimeout(600);
await cek('dersler');

await tarayici.close();
await api('/api/kapat', {}).catch(() => {});
studyo.kill();
rmSync(ev, { recursive: true, force: true });
