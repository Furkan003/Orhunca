// Yapay zekâ ajanları için: arayüz programını Node.js'te tarayıcısız çalıştırır, verilen
// eylemleri (tıklama, yazma, gönderme, oyun karesi) uygular ve ekranı metin olarak yazar.
// Rust tarafı (src/ajan.rs, arayuz_calistir) çağırır:
//   node ajan_arayuz.js <klasör>
// Klasörde orhunca.js, orhunca_rt.wasm, program.wasm ve eylemler.json bulunur.
'use strict';
const fs = require('fs');
const path = require('path');
const klasor = process.argv[2];
const Orhunca = require(path.join(klasor, 'orhunca.js'));
const eylemler = JSON.parse(fs.readFileSync(path.join(klasor, 'eylemler.json'), 'utf8'));

// Görünüşü belirleyen (renk, boyut, boşluk…) özellikler ekran metnine yazılmaz.
const ANLAMLI = ['değer', 'seçenekler', 'ipucu', 'etkin', 'gizli', 'tür', 'en_az', 'en_çok'];

function tanim(v) {
  const oz = v.oz || {};
  let s = v.tur;
  if (oz.metin !== undefined && oz.metin !== '') s += ` "${oz.metin}"`;
  const ek = [];
  for (const a of ANLAMLI) {
    if (oz[a] === undefined || oz[a] === '') continue;
    if (a === 'etkin' && oz[a] !== 'yanlış') continue;
    if (a === 'gizli' && oz[a] !== 'doğru') continue;
    ek.push(`${a}: ${typeof oz[a] === 'string' ? oz[a] : JSON.stringify(oz[a])}`);
  }
  // 'bağ': öğe bir duruma bağlı, 'yaz' eylemiyle değiştirilebilir.
  const olaylar = Object.keys(v.olay || {}).map((o) => (o === 'bağ' ? 'değiştirilebilir' : o));
  if (olaylar.length) ek.push('olay: ' + olaylar.join(', '));
  return ek.length ? `${s} (${ek.join('; ')})` : s;
}

function ekran(uygulama) {
  const satirlar = [];
  const gez = (v, d) => {
    satirlar.push('  '.repeat(d) + tanim(v));
    (v.cocuk || []).forEach((c) => gez(c, d + 1));
  };
  const kok = uygulama.agac();
  (kok ? kok.cocuk || [] : []).forEach((c) => gez(c, 0));
  return satirlar.length ? satirlar.join('\n') : '(ekran boş)';
}

function hepsi(uygulama) {
  const s = [];
  const gez = (v) => {
    s.push(v);
    (v.cocuk || []).forEach(gez);
  };
  const kok = uygulama.agac();
  if (kok) gez(kok);
  return s;
}

(async () => {
  const cikti = [];
  const yaz = (m) => process.stdout.write(m + '\n');
  let sonuc;
  try {
    sonuc = await Orhunca.baslat({
      calismaZamani: fs.readFileSync(path.join(klasor, 'orhunca_rt.wasm')),
      program: fs.readFileSync(path.join(klasor, 'program.wasm')),
      cikti: (_akis, b) => cikti.push(Buffer.from(b).toString()),
    });
  } catch (e) {
    yaz('Program başlatılamadı: ' + e.message);
    process.exit(1);
  }
  const { kod, uygulama } = sonuc;
  const programCiktisi = () => {
    const m = cikti.splice(0).join('');
    if (m.trim()) yaz('Program çıktısı:\n' + m.trimEnd());
  };
  if (kod !== 0 || !uygulama) {
    programCiktisi();
    yaz(`Program açılışta durdu (çıkış kodu ${kod}).`);
    process.exit(1);
  }
  yaz('Ekran:\n' + ekran(uygulama));
  programCiktisi();
  for (const e of eylemler) {
    const ad = Object.keys(e)[0];
    const d = e[ad];
    const ogeler = hepsi(uygulama);
    const bul = (tur, sira) => ogeler.filter((v) => v.tur === tur)[sira || 0];
    let olay = null, deger = null, aciklama = '';
    if (ad === 'tıkla' || ad === 'tikla') {
      const v = ogeler.find((o) => o.olay && o.olay['tıklanınca'] != null && o.oz && o.oz.metin === String(d));
      if (!v) { yaz(`\n> tıkla "${d}": bu metinde tıklanabilir bir öğe yok`); continue; }
      olay = v.olay['tıklanınca'];
      aciklama = `tıkla "${d}"`;
    } else if (ad === 'yaz' || ad === 'değiştir' || ad === 'degistir') {
      const v = bul(d.tür || d.tur || 'giriş', d.sıra ?? d.sira ?? 0);
      if (!v) { yaz(`\n> ${ad}: '${d.tür || d.tur || 'giriş'}' öğesi yok`); continue; }
      deger = String(d.değer ?? d.deger ?? '');
      aciklama = `${v.tur} ← "${deger}"`;
      if (v.olay['bağ'] != null) uygulama.tetikle(v.olay['bağ'], deger);
      olay = v.olay['değişince'] ?? null;
    } else if (ad === 'gönder' || ad === 'gonder') {
      const v = bul((d && (d.tür || d.tur)) || 'giriş', (d && (d.sıra ?? d.sira)) || 0);
      if (!v || v.olay['gönderilince'] == null) { yaz(`\n> gönder: Enter ile gönderilen bir giriş yok`); continue; }
      olay = v.olay['gönderilince'];
      aciklama = 'gönder (Enter)';
    } else if (ad === 'kare') {
      const v = ogeler.find((o) => o.olay && o.olay['her_karede'] != null);
      if (!v) { yaz('\n> kare: oyun_alanı yok'); continue; }
      const n = Math.min(Number(d) || 1, 600);
      for (let i = 0; i < n; i++) uygulama.tetikle(v.olay['her_karede'], null);
      aciklama = `${n} kare`;
    } else {
      yaz(`\n> bilinmeyen eylem '${ad}' (tıkla, yaz, gönder, kare)`);
      continue;
    }
    const k = olay != null ? uygulama.tetikle(olay, deger) : 0;
    yaz(`\n> ${aciklama}` + (k ? ` (çalışma hatası, kod ${k})` : ''));
    programCiktisi();
    yaz('Ekran:\n' + ekran(uygulama));
  }
  process.exit(0);
})();
