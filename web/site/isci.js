// Deneme sayfasının iş parçacığı: konsol programlarını sayfayı dondurmadan çalıştırır
// (sonsuz döngü "Durdur" ile sonlandırılır). Girdi önceden verilen satırlardan okunur.
/* global Orhunca */
importScripts('calisma/orhunca.js');

self.onmessage = async (e) => {
  const { calismaZamani, program, girdi } = e.data;
  const satirlar = girdi === '' ? [] : girdi.replace(/\r/g, '').split('\n');
  if (satirlar.length && satirlar[satirlar.length - 1] === '') satirlar.pop();
  let sira = 0;
  let kod;
  try {
    kod = await Orhunca.calistir({
      calismaZamani,
      program,
      cikti: (akis, b) => self.postMessage({ tur: 'cikti', akis, b }, [b.buffer]),
      satirOku: () => (sira < satirlar.length ? satirlar[sira++] : null),
      bekle: (saniye) => { const son = Date.now() + saniye * 1000; while (Date.now() < son) { /* bekle */ } },
      dosyalar: Orhunca.bellekDosyalari(),
      ortam: {},
      argumanlar: [],
    });
  } catch (h) {
    self.postMessage({ tur: 'cikti', akis: 2, b: new TextEncoder().encode('Çalışma hatası: ' + h + '\n') });
    kod = 1;
  }
  self.postMessage({ tur: 'bitti', kod, girdiBitti: sira >= satirlar.length });
};
