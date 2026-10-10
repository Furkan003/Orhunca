/* Orhunca Stüdyo — Durum (D) ve sunucuyla konuşma.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Durum
  // =====================================================================
  const D = {
    ekran: 'baslangic', // baslangic | yeni | yapilandir | ogren | duzenleyici
    bilgi: { kullanici: '', surum: '0.0.0', isletim: 'linux', ayrac: '/', varsayilan_konum: '', ev: '' },
    yuklendi: false,
    projeler: [], sonSablonlar: [], sablonlar: [], yerlesikler: [],
    q: '', siralama: 'tarih',
    tq: '', kategori: 'Tümü', secili: 'konsol',
    projeAdi: 'yeni_konsol', adDokunuldu: false, konum: '', mevcutAdlar: new Set(),
    secenekler: { git: true, ornek: true, calistir: false, canli: false },
    olusturuluyor: false,
    proje: null, agac: [], kapaliKlasorler: new Set(),
    sekmeler: [], etkin: null, imlec: { satir: 1, sutun: 1 },
    terminal: [], cikti: [], sorunlar: [], uyarilar: [], altSekme: 'terminal', altPanel: true,
    calisma: null, argumanlar: '',
    // Kesme noktaları: { tam dosya yolu: [satır, ...] }
    kesmeler: ayarOku('kesmeler', {}),
    kesmeAyar: ayarOku('kesmeAyar', {}),
    izlenenler: ayarOku('izlenenler', []),
    // Web projelerinde canlı önizleme: { kapi, adres, yol, durum: bekliyor|acik|hata|durdu, surum }
    onizleme: null, onizlemeAcik: true,
    yanPanel: 'gezgin', vt: null, sinamalar: null, sinamaSonuc: {}, sinamaMesgul: null, araMetin: '', araSonuc: [], degistirMetin: '', tamKelime: false,
    paketler: [], paketKaynagi: '', paketMesgul: false, paketDizini: null, paketDizinHatasi: '',
    menu: null, modal: null, bildirim: null,
    yaziBoyutu: ayarOku('yaziBoyutu', 13),
    yazarkenDenetle: ayarOku('yazarkenDenetle', true),
    yavasHiz: ayarOku('yavasHiz', 700),
    acilis: ayarOku('acilis', true),
    otomatikKaydet: ayarOku('otomatikKaydet', true),
    guncellemeDenetle: ayarOku('guncellemeDenetle', true), guncelleme: null, guncellemeDurumu: '',
    // Yapay zekâ asistanı (kullanıcının kendi API anahtarıyla)
    asistan: { durum: null, mesajlar: [], bekliyor: false, modeller: null, hata: '', girdi: '', anahtar: '', adres: null, secilen: null, ayarAcik: false, baglaniyor: false, istek: 0 },
    asistanGoster: ayarOku('asistanGoster', true), asistanPaneliAcik: ayarOku('asistanPaneliAcik', false),
    // Özel tema: seçilen temanın kopyası (arka plan resmi olmadan; resim sunucudan yüklenir)
    ozelTema: ayarOku('temaOnbellek', null), temaKimlik: ayarOku('temaKimlik', null),
    temaTaslak: null, temalarim: [], galeri: null, galeriHatasi: '',
    // 'koyu', 'acik' ya da 'sistem' (işletim sisteminin ayarı)
    tema: ayarOku('tema', 'koyu'),
  };

  function temaUygula() {
    const T = window.OrhuncaTema;
    if (D.temaTaslak) return T.uygula(D.temaTaslak);
    if (D.ozelTema) return T.uygula(D.ozelTema);
    const acik = D.tema === 'acik' || (D.tema === 'sistem' && matchMedia('(prefers-color-scheme: light)').matches);
    T.uygula({ taban: acik ? 'acik' : 'koyu', renkler: {}, yazi: {} });
  }

  /** Seçilen temayı kalıcı yapar; resimli temanın resmi önbelleğe yazılmaz. */
  function temayiSec(tema, kimlik) {
    D.ozelTema = tema;
    D.temaKimlik = kimlik || null;
    const onbellek = tema && tema.arka_plan ? { ...tema, arka_plan: { ...tema.arka_plan, kaynak: undefined }, resimli: true } : tema;
    ayarYaz('temaOnbellek', onbellek);
    ayarYaz('temaKimlik', D.temaKimlik);
    if (tema?.yazi?.kod_boyut) { D.yaziBoyutu = tema.yazi.kod_boyut; ayarYaz('yaziBoyutu', D.yaziBoyutu); }
    temaUygula();
  }

  /** Açılışta: önbellekteki tema resimliyse resmi sunucudan alınır. */
  async function temaResminiYukle() {
    if (!D.ozelTema?.resimli || !D.temaKimlik) return;
    const r = await api('/api/tema?' + sorgu({ kimlik: D.temaKimlik })).catch(() => null);
    if (r?.tema) {
      try { D.ozelTema = window.OrhuncaTema.dogrula(r.tema); D.ozelTema.resimli = true; temaUygula(); } catch { /* bozuk */ }
    }
  }
  temaUygula();
  matchMedia('(prefers-color-scheme: light)').addEventListener?.('change', temaUygula);

  const ayrac = () => D.bilgi.ayrac || '/';
  const tamYol = goreli => D.proje.yol.replace(/[\\/]+$/, '') + ayrac() + goreli.split('/').join(ayrac());
  /** `a/b/../c` → `a/c` (her iki ayraç da) */
  const normal = yol => {
    const parcalar = [];
    for (const p of yol.split(/[\\/]/)) {
      if (p === '..') parcalar.pop(); else if (p !== '.') parcalar.push(p);
    }
    return parcalar.join('/');
  };
  const goreliYol = tam => {
    const kok = normal(D.proje.yol), t = normal(tam);
    return t.startsWith(kok + '/') ? t.slice(kok.length + 1) : t;
  };
  const kisaYol = yol => D.bilgi.ev && yol.startsWith(D.bilgi.ev) && D.bilgi.isletim !== 'windows' ? '~' + yol.slice(D.bilgi.ev.length) : yol;
  const surumAdi = () => 'Orhunca ' + D.bilgi.surum.split('.').slice(0, 2).join('.');
  const kullaniciAdi = () => ilkBuyuk(D.bilgi.kullanici || 'geliştirici');
  const sablon = kimlik => D.sablonlar.find(s => s.kimlik === kimlik) || D.sablonlar[0];

