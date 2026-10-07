// Deneme sayfasını çevrimdışı çalıştırır (kurulabilir web uygulaması). Derleyici ve
// çalışma zamanı ilk ziyarette önbelleğe alınır; sonra internet olmadan da kod yazılıp
// çalıştırılabilir. Önbellekteki dosya hemen verilir, arka planda yenisi indirilir.
// SURUM, site her derlendiğinde araclar/site.sh tarafından değiştirilir.
'use strict';
const SURUM = '__SURUM__';
const ONBELLEK = 'orhunca-' + SURUM;
const DOSYALAR = [
  'dene.html', 'dene.js', 'isci.js', 'vurgula.js', 'stil.css', 'ornekler.json',
  'manifest.webmanifest', 'marka/logo.svg', 'marka/simge.svg',
  'yazitipleri/yazitipleri.css',
  'calisma/oyun.wasm', 'calisma/orhunca_rt.wasm', 'calisma/orhunca.js', 'calisma/arayuz.html',
];

self.addEventListener('install', (e) => {
  e.waitUntil(caches.open(ONBELLEK).then((c) => c.addAll(DOSYALAR)).then(() => self.skipWaiting()));
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    caches.keys()
      .then((l) => Promise.all(l.filter((a) => a.startsWith('orhunca-') && a !== ONBELLEK).map((a) => caches.delete(a))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (e) => {
  const istek = e.request;
  if (istek.method !== 'GET' || new URL(istek.url).origin !== location.origin) return;
  e.respondWith(caches.open(ONBELLEK).then(async (c) => {
    const eski = await c.match(istek, { ignoreSearch: true });
    const yeni = fetch(istek).then((y) => {
      if (y.ok && y.type === 'basic') c.put(istek, y.clone());
      return y;
    });
    if (eski) {
      e.waitUntil(yeni.catch(() => {}));
      return eski;
    }
    return yeni;
  }));
});
