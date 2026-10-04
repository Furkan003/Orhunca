// Arayüz örneklerini (örnekler/arayüz/) Node.js'te DOM olmadan çalıştırır: olayları
// tetikler ve programın çizdiği öğe ağacını denetler. tests/wasm.rs çağırır:
//   node tests/arayuz_senaryolari.js <klasör>
// Klasörde orhunca.js, orhunca_rt.wasm ve her örneğin <ad>.wasm dosyası bulunur.
'use strict';
const fs = require('fs');
const path = require('path');
const klasor = process.argv[2];
const Orhunca = require(path.resolve(klasor, 'orhunca.js'));

function esit(gercek, beklenen, ne) {
  if (JSON.stringify(gercek) !== JSON.stringify(beklenen)) {
    throw new Error(`${ne}: beklenen ${JSON.stringify(beklenen)}, bulunan ${JSON.stringify(gercek)}`);
  }
}

async function uygulama(ad) {
  const cikti = [];
  const { kod, uygulama } = await Orhunca.baslat({
    calismaZamani: fs.readFileSync(path.join(klasor, 'orhunca_rt.wasm')),
    program: fs.readFileSync(path.join(klasor, ad + '.wasm')),
    cikti: (akis, b) => cikti.push(Buffer.from(b).toString()),
    // Toplayıcı neredeyse her güvenli noktada çalışır.
    ortam: { ORHUNCA_GC_ESIK: '64' },
  });
  esit(kod, 0, ad + ' başlangıç');
  if (!uygulama) throw new Error(ad + ': arayüz programı değil');
  const ogeler = (tur) => {
    const s = [];
    const gez = (v) => {
      if (v.tur === tur) s.push(v);
      v.cocuk.forEach(gez);
    };
    gez(uygulama.agac());
    return s;
  };
  const tikla = (tur, metin) => {
    const v = ogeler(tur).find((o) => o.oz.metin === metin);
    if (!v) throw new Error(`${ad}: '${metin}' ${tur} öğesi yok`);
    esit(uygulama.tetikle(v.olay['tıklanınca'], null), 0, `${ad}: '${metin}' tıklaması`);
  };
  const bagla = (tur, sira, deger) => {
    esit(uygulama.tetikle(ogeler(tur)[sira].olay['bağ'], deger), 0, `${ad}: ${tur} değeri`);
  };
  const yazilar = () => ogeler('yazı').map((v) => v.oz.metin);
  return { uygulama, ogeler, tikla, bagla, yazilar, cikti };
}

const senaryolar = {
  async 'sayaç'() {
    const u = await uygulama('sayaç');
    for (let i = 0; i < 3; i++) u.tikla('düğme', '+ 1');
    esit(u.yazilar()[0], 'Şu anki değer: 3', 'üç tıklama');
    u.bagla('kaydırıcı', 0, '5');
    u.tikla('düğme', '+ 5');
    u.tikla('düğme', '+ 5');
    esit(u.yazilar(), ['Şu anki değer: 13', 'Adım: 5', "On'u geçtin!"], 'adım 5');
    u.bagla('kaydırıcı', 0, 'sayı değil');
    esit(u.yazilar()[1], 'Adım: 5', 'geçersiz girdi yok sayılır');
    u.tikla('düğme', 'Sıfırla');
    esit(u.yazilar(), ['Şu anki değer: 0', 'Adım: 5'], 'sıfırla');
  },

  async 'yapılacaklar'() {
    const u = await uygulama('yapılacaklar');
    esit(u.yazilar(), ["Henüz iş yok. Yukarıya yazıp Enter'a basın."], 'boş liste');
    esit(u.ogeler('düğme')[0].oz.etkin, 'yanlış', 'boşken Ekle kapalı');
    for (const is of ['Ekmek al', '  Ödevi bitir ', 'Koşuya çık']) {
      u.bagla('giriş', 0, is);
      const g = u.ogeler('giriş')[0];
      esit(u.uygulama.tetikle(g.olay['gönderilince'], is), 0, 'Enter');
    }
    esit(u.ogeler('giriş')[0].oz['değer'], '', 'giriş temizlenir');
    esit(u.ogeler('onay_kutusu').map((v) => v.oz.metin), ['Ekmek al', 'Ödevi bitir', 'Koşuya çık'], 'işler');
    u.bagla('onay_kutusu', 1, 'doğru');
    esit(u.yazilar(), ['2 iş kaldı'], 'biri bitti');
    u.bagla('seçim', 0, 'Bitenler');
    esit(u.ogeler('onay_kutusu').map((v) => [v.oz.metin, v.oz['değer']]), [['Ödevi bitir', 'doğru']], 'bitenler');
    u.bagla('seçim', 0, 'Kalanlar');
    esit(u.ogeler('onay_kutusu').map((v) => v.oz.metin), ['Ekmek al', 'Koşuya çık'], 'kalanlar');
    u.bagla('seçim', 0, 'Hepsi');
    // İlk "Sil": döngüde yakalanan sıra 0
    const sil = u.ogeler('düğme').filter((v) => v.oz.metin === 'Sil');
    u.uygulama.tetikle(sil[0].olay['tıklanınca'], null);
    esit(u.ogeler('onay_kutusu').map((v) => [v.oz.metin, v.oz['değer']]), [['Ödevi bitir', 'doğru'], ['Koşuya çık', 'yanlış']], 'silindi');
    // Çok iş: toplayıcı altında ad bozulmamalı
    for (let i = 0; i < 60; i++) {
      u.bagla('giriş', 0, 'iş ' + i);
      u.tikla('düğme', 'Ekle');
    }
    const adlar = u.ogeler('onay_kutusu').map((v) => v.oz.metin);
    esit(adlar.length, 62, 'iş sayısı');
    esit(adlar.slice(2), Array.from({ length: 60 }, (_, i) => 'iş ' + i), 'adlar');
  },

  async 'hesap_makinesi'() {
    const u = await uygulama('hesap_makinesi');
    const bas = (tuslar) => {
      for (const t of tuslar) u.tikla('düğme', t);
    };
    const gosterge = () => u.yazilar()[1];
    bas(['1', '2', '+', '3', '=']);
    esit(gosterge(), '15', '12 + 3');
    bas(['×', '4', '=']);
    esit(gosterge(), '60', '15 × 4');
    u.tikla('düğme', 'Temizle');
    bas(['7', '÷', '2', '=']);
    esit(gosterge(), '3.5', '7 ÷ 2');
    u.tikla('düğme', 'Temizle');
    bas(['0', '.', '5', '+', '.', '2', '5', '=']);
    esit(gosterge(), '0.75', '0.5 + .25');
    bas(['−', '1', '=']);
    esit(gosterge(), '-0.25', 'eksi');
    esit(u.ogeler('düğme').length, 17, 'düğme sayısı');
  },
};

(async () => {
  for (const [ad, senaryo] of Object.entries(senaryolar)) {
    await senaryo();
    console.log('TAMAM ' + ad);
  }
})().catch((h) => {
  console.error(h.stack || String(h));
  process.exit(1);
});
