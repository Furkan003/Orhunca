// Arayüz, Orhunca sunucusundaki /api/görevler uç noktalarıyla konuşur.
const API = '/api/görevler';
const $ = s => document.querySelector(s);

async function istek(yontem, adres, govde) {
  const y = await fetch(adres, {
    method: yontem,
    headers: govde ? { 'Content-Type': 'application/json' } : {},
    body: govde ? JSON.stringify(govde) : undefined,
  });
  const veri = await y.json();
  if (!y.ok) throw new Error(Array.isArray(veri) ? veri.join(', ') : veri);
  return veri;
}

function hata(m) {
  $('#hata').textContent = m || '';
  $('#hata').hidden = !m;
}

async function yenile() {
  const gorevler = await istek('GET', API);
  $('#liste').replaceChildren(...gorevler.map(g => {
    const li = document.createElement('li');
    li.className = g.tamamlandı ? 'bitti' : '';
    const kutu = document.createElement('input');
    kutu.type = 'checkbox';
    kutu.checked = g.tamamlandı;
    kutu.onchange = () => istek('PUT', `${API}/${g.kimlik}`).then(yenile, e => hata(e.message));
    const ad = document.createElement('span');
    ad.textContent = g.başlık;
    const sil = document.createElement('button');
    sil.textContent = 'Sil';
    sil.className = 'sil';
    sil.onclick = () => istek('DELETE', `${API}/${g.kimlik}`).then(yenile, e => hata(e.message));
    li.append(kutu, ad, sil);
    return li;
  }));
  const kalan = gorevler.filter(g => !g.tamamlandı).length;
  $('#ozet').textContent = gorevler.length ? `${kalan} görev kaldı` : 'Henüz görev yok.';
}

$('#yeni').onsubmit = async e => {
  e.preventDefault();
  try {
    await istek('POST', API, { başlık: $('#baslik').value });
    $('#baslik').value = '';
    hata('');
    await yenile();
  } catch (h) {
    hata(h.message);
  }
};

yenile().catch(e => hata(e.message));
