/*
 * Orhunca WebAssembly yükleyicisi.
 *
 * Bir Orhunca programı WebAssembly'ye iki modül olarak derlenir: çalışma zamanı
 * (orhunca_rt.wasm, C'den derlenir; bellek, metinler, listeler, çöp toplayıcı)
 * ve program modülü (derleyicinin ürettiği kod). Bu dosya ikisini birleştirir
 * ve çalışma zamanının sistemden istediklerini (çıktı, girdi, dosyalar, zaman)
 * karşılar. Tarayıcıda ve Node.js'te çalışır:
 *
 *   node orhunca.js program.wasm [argümanlar...]
 *
 *   const kod = await Orhunca.calistir({ calismaZamani, program, cikti(akis, baytlar) {...} });
 */
(function (kok, fabrika) {
  'use strict';
  const Orhunca = fabrika();
  if (typeof module === 'object' && module.exports) {
    module.exports = Orhunca;
    if (typeof require === 'function' && require.main === module) Orhunca.komutSatiri();
  } else {
    kok.Orhunca = Orhunca;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
  'use strict';
  const kodlayici = new TextEncoder();

  /** Programın `çık(kod)` ile ya da bir çalışma hatasıyla bitmesi. */
  class Cikis {
    constructor(kod) {
      this.kod = kod;
    }
  }

  /* ---------------------------------------------------------------------- */
  /* printf'in %g, %f ve %e biçimleri (C ile aynı sonuç: tam ondalık açılım,  */
  /* yarımlarda çifte yuvarlama)                                             */
  /* ---------------------------------------------------------------------- */

  const bitGorunumu = new DataView(new ArrayBuffer(8));

  /** Sonlu, pozitif x'in tam değeri: x = d / 10^olcek (d BigInt). */
  function tamDeger(x) {
    bitGorunumu.setFloat64(0, x);
    const bitler = bitGorunumu.getBigUint64(0);
    const us = Number((bitler >> 52n) & 0x7ffn);
    let mantis = bitler & ((1n << 52n) - 1n);
    let ikiUs;
    if (us === 0) {
      ikiUs = -1074;
    } else {
      mantis |= 1n << 52n;
      ikiUs = us - 1075;
    }
    if (ikiUs >= 0) return { d: mantis << BigInt(ikiUs), olcek: 0 };
    // mantis / 2^k = mantis * 5^k / 10^k
    return { d: mantis * 5n ** BigInt(-ikiUs), olcek: -ikiUs };
  }

  /** d / 10^olcek değerini virgülden sonra `basamak` haneye yuvarlar (yarımda çifte). */
  function yuvarla(d, olcek, basamak) {
    if (basamak >= olcek) return d * 10n ** BigInt(basamak - olcek);
    const bolen = 10n ** BigInt(olcek - basamak);
    let q = d / bolen;
    const kalan = (d % bolen) * 2n;
    if (kalan > bolen || (kalan === bolen && q % 2n === 1n)) q += 1n;
    return q;
  }

  /** q / 10^basamak → "123.45" */
  function noktali(q, basamak) {
    let s = q.toString();
    if (basamak === 0) return s;
    s = s.padStart(basamak + 1, '0');
    return s.slice(0, s.length - basamak) + '.' + s.slice(s.length - basamak);
  }

  function sabitNokta(x, basamak) {
    const { d, olcek } = tamDeger(x);
    return noktali(yuvarla(d, olcek, basamak), basamak);
  }

  /** Bilimsel gösterim parçaları: x ≈ q / 10^basamak × 10^us, q'nun basamak+1 hanesi var. */
  function bilimsel(x, basamak) {
    if (x === 0) return { q: 0n, us: 0 };
    const { d, olcek } = tamDeger(x);
    let us = d.toString().length - 1 - olcek;
    let q = yuvarla(d, olcek, basamak - us);
    if (q.toString().length > basamak + 1) {
      // 9.99 → 10.0: bir hane taştı
      us += 1;
      q = yuvarla(d, olcek, basamak - us);
    }
    return { q, us };
  }

  function usluYaz(q, us, basamak) {
    const isaret = us < 0 ? '-' : '+';
    const u = String(Math.abs(us)).padStart(2, '0');
    return noktali(q, basamak) + 'e' + isaret + u;
  }

  function sondakiSifirlar(s) {
    return s.includes('.') ? s.replace(/0+$/, '').replace(/\.$/, '') : s;
  }

  function bicimle(x, hassasiyet, tur) {
    if (Number.isNaN(x)) return 'nan';
    if (!Number.isFinite(x)) return x < 0 ? '-inf' : 'inf';
    const eksi = x < 0 || Object.is(x, -0) ? '-' : '';
    const m = Math.abs(x);
    if (tur === 'f') return eksi + sabitNokta(m, hassasiyet);
    if (tur === 'e') {
      const { q, us } = bilimsel(m, hassasiyet);
      return eksi + usluYaz(q, us, hassasiyet);
    }
    // %g: üs, P anlamlı haneye yuvarlandıktan sonra belirlenir.
    const P = hassasiyet === 0 ? 1 : hassasiyet;
    const { q, us } = bilimsel(m, P - 1);
    if (us < P && us >= -4) return eksi + sondakiSifirlar(sabitNokta(m, P - 1 - us));
    const [govde, u] = usluYaz(q, us, P - 1).split('e');
    return eksi + sondakiSifirlar(govde) + 'e' + u;
  }

  /** strtod: metnin başındaki sayıyı okur; [değer, okunan bayt sayısı] */
  function sayiOku(s) {
    const m =
      /^[ \t\n\v\f\r]*([+-]?)(?:(0[xX](?:[0-9a-fA-F]+\.?[0-9a-fA-F]*|\.[0-9a-fA-F]+)(?:[pP][+-]?\d+)?)|(inf(?:inity)?|nan)|((?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?))/i.exec(
        s
      );
    if (!m) return [0, 0];
    const isaret = m[1] === '-' ? -1 : 1;
    let deger;
    if (m[2]) {
      const [, govde, us] = /^0[xX]([^pP]*)(?:[pP](.*))?$/.exec(m[2]);
      const [tam, kesir = ''] = govde.split('.');
      deger = parseInt(tam || '0', 16);
      for (let i = 0; i < kesir.length; i++) deger += parseInt(kesir[i], 16) / 16 ** (i + 1);
      if (us) deger *= 2 ** Number(us);
    } else if (m[3]) {
      deger = m[3][0] === 'n' || m[3][0] === 'N' ? NaN : Infinity;
    } else {
      deger = parseFloat(m[4]);
    }
    return [isaret * deger, m[0].length];
  }

  /* ---------------------------------------------------------------------- */
  /* Çalıştırma                                                              */
  /* ---------------------------------------------------------------------- */

  /** Programı çalıştırır ve çıkış kodunu döndürür (arayüz programlarında ilk çizimden sonra). */
  async function calistir(s) {
    return (await baslat(s)).kod;
  }

  /**
   * Programı başlatır: { kod, uygulama }. Arayüz programlarında `uygulama`
   * çizilen ağacı verir (`agac()`) ve olay tetikler (`tetikle(sıra, değer)`);
   * `s.arayuzKoku` bir DOM öğesiyse arayüz oraya çizilir.
   *
   * s.calismaZamani, s.program: wasm baytları ya da WebAssembly.Module
   * s.cikti(akis, baytlar): 1 çıktı, 2 hata akışı (Uint8Array, UTF-8)
   * s.satirOku(): klavyeden bir satır (metin), girdi bittiyse null
   * s.dosyalar: { oku(yol) → Uint8Array|null, yaz(yol, baytlar, ekle) → bool,
   *               sil(yol) → bool, tasi(eski, yeni) → bool, klasor(yol) → bool }
   * s.bekle(saniye), s.ortam (nesne), s.argumanlar (dizi)
   * s.arayuzKoku: arayüzün çizileceği DOM öğesi (tarayıcıda)
   */
  async function baslat(s) {
    let bellek = null;
    let rt = null;
    const u8 = () => new Uint8Array(bellek.buffer);
    const cozucu = new TextDecoder('utf-8');
    const metin = (p) => {
      const b = u8();
      let son = p;
      while (b[son]) son++;
      return cozucu.decode(b.subarray(p, son));
    };
    const ayir = (baytlar) => {
      const p = rt.ohc_js_ayir(baytlar.length + 1);
      const b = u8();
      b.set(baytlar, p);
      b[p + baytlar.length] = 0;
      return p;
    };
    const metinAyir = (m) => ayir(typeof m === 'string' ? kodlayici.encode(m) : m);
    const sayiYaz = (p, n) => new DataView(bellek.buffer).setInt32(p, n, true);
    const dosyalar = s.dosyalar || bellekDosyalari();
    const argumanlar = s.argumanlar || [];

    const js = {
      yaz(akis, p, n) {
        s.cikti(akis, u8().slice(p, p + n));
      },
      satir_oku() {
        const satir = s.satirOku ? s.satirOku() : null;
        return satir == null ? 0 : metinAyir(satir);
      },
      zaman: () => Date.now() / 1000,
      tarih(p) {
        const t = new Date();
        const iki = (n) => String(n).padStart(2, '0');
        const m =
          `${t.getFullYear()}-${iki(t.getMonth() + 1)}-${iki(t.getDate())} ` +
          `${iki(t.getHours())}:${iki(t.getMinutes())}:${iki(t.getSeconds())}`;
        const b = kodlayici.encode(m);
        u8().set(b, p);
        u8()[p + b.length] = 0;
      },
      cik(kod) {
        throw new Cikis(kod);
      },
      ondalik_yaz(x, hassasiyet, tur, p, kap) {
        const b = kodlayici.encode(bicimle(x, hassasiyet, String.fromCharCode(tur)));
        const n = Math.min(b.length, kap - 1);
        u8().set(b.subarray(0, n), p);
        u8()[p + n] = 0;
        return n;
      },
      ondalik_oku(p, okunan) {
        const b = u8();
        let son = p;
        const sinir = Math.min(b.length, p + 1024);
        while (son < sinir && b[son]) son++;
        let m = '';
        for (let i = p; i < son; i++) m += String.fromCharCode(b[i]);
        const [deger, n] = sayiOku(m);
        sayiYaz(okunan, n);
        return deger;
      },
      matematik(islem, a, b) {
        switch (islem) {
          case 1:
            return Math.sin(a);
          case 2:
            return Math.cos(a);
          case 3:
            return Math.tan(a);
          case 4:
            return Math.log(a);
          default:
            return Math.pow(a, b);
        }
      },
      ortam(p) {
        const d = (s.ortam || {})[metin(p)];
        return d == null ? 0 : metinAyir(String(d));
      },
      dosya_oku(p, uzunluk) {
        const b = dosyalar.oku(metin(p));
        if (!b) return 0;
        const q = ayir(b);
        sayiYaz(uzunluk, b.length);
        return q;
      },
      dosya_yaz: (p, q, n, ekle) => (dosyalar.yaz(metin(p), u8().slice(q, q + n), !!ekle) ? 1 : 0),
      dosya_sil: (p) => (dosyalar.sil(metin(p)) ? 1 : 0),
      dosya_tasi: (a, b) => (dosyalar.tasi(metin(a), metin(b)) ? 1 : 0),
      klasor_olustur: (p) => (dosyalar.klasor(metin(p)) ? 1 : 0),
      bekle(saniye) {
        (s.bekle || mesgulBekle)(saniye);
      },
      rastgele_tohum: () => (Math.random() * 4294967296) >>> 0,
      arguman_sayisi: () => 1 + argumanlar.length,
      arguman: (i) => metinAyir(i === 0 ? 'program' : String(argumanlar[i - 1])),
    };

    // Arayüz: çizim sırasında program öğe ağacını bu işlevlerle kurar.
    let yigin = [];
    const ui = {
      ac(p) {
        const d = { tur: metin(p), oz: {}, olay: {}, cocuk: [] };
        yigin[yigin.length - 1].cocuk.push(d);
        yigin.push(d);
      },
      ozellik(a, d) {
        yigin[yigin.length - 1].oz[metin(a)] = metin(d);
      },
      olay(a, no) {
        yigin[yigin.length - 1].olay[metin(a)] = no;
      },
      kapat() {
        yigin.pop();
      },
    };

    const rtOrnek = await ornekle(s.calismaZamani, { js });
    rt = rtOrnek.exports;
    bellek = rt.memory;
    const prog = await ornekle(s.program, { rt, ui });

    /** Çalışma hatasını yazar ve çıkış kodunu verir; başka hataları fırlatır. */
    const hataKodu = (h) => {
      if (h instanceof Cikis) return h.kod;
      let mesaj;
      if (h instanceof RangeError && /stack|yığ/i.test(h.message)) {
        mesaj = 'çok derin özyineleme: işlevler birbirini bitmeyecek kadar çok çağırıyor';
      } else if (typeof WebAssembly.RuntimeError === 'function' && h instanceof WebAssembly.RuntimeError) {
        mesaj = 'WebAssembly hatası: ' + h.message;
      } else {
        throw h;
      }
      s.cikti(2, kodlayici.encode('Çalışma hatası: ' + mesaj + '\n'));
      return 1;
    };

    try {
      rt.ohc_wasm_baslat();
      prog.exports.ohc_ana();
    } catch (h) {
      return { kod: hataKodu(h), uygulama: null };
    }
    if (!prog.exports.ohc_ciz) {
      rt.ohc_wasm_bitir();
      return { kod: 0, uygulama: null };
    }

    // Arayüz programı: çiz, olaylarda yeniden çiz.
    let agac = null;
    let durdu = false;
    let dom = null;
    const ciz = () => {
      const kok = { tur: 'kök', oz: {}, olay: {}, cocuk: [] };
      yigin = [kok];
      prog.exports.ohc_ciz();
      agac = kok;
      if (dom) dom(agac);
    };
    const tetikle = (no, deger) => {
      if (durdu) return 1;
      try {
        let p = 0;
        if (deger != null) {
          const b = kodlayici.encode(String(deger));
          p = Number(rt.ohc_wasm_metin(BigInt(b.length)));
          u8().set(b, p);
        }
        prog.exports.ohc_olay(no, p);
        ciz();
        return 0;
      } catch (h) {
        durdu = true;
        return hataKodu(h);
      }
    };
    if (s.arayuzKoku) dom = domCizici(s.arayuzKoku, tetikle);
    try {
      ciz();
    } catch (h) {
      durdu = true;
      return { kod: hataKodu(h), uygulama: null };
    }
    // bitir(): bellek raporu (ORHUNCA_BELLEK_RAPORU) için; uygulama çalışmayı sürdürebilir.
    const bitir = () => rt.ohc_wasm_bitir();
    return { kod: 0, uygulama: { agac: () => agac, tetikle, durdu: () => durdu, bitir } };
  }

  async function ornekle(kaynak, ice) {
    if (kaynak instanceof WebAssembly.Module) return WebAssembly.instantiate(kaynak, ice);
    return (await WebAssembly.instantiate(kaynak, ice)).instance;
  }

  function mesgulBekle(saniye) {
    const son = Date.now() + saniye * 1000;
    while (Date.now() < son) {
      /* tarayıcının ana iş parçacığında uyunamaz */
    }
  }

  /** Bellekte dosyalar (kalıcı değil). */
  function bellekDosyalari() {
    const d = new Map();
    return {
      oku: (y) => d.get(y) || null,
      yaz(y, b, ekle) {
        const eski = ekle ? d.get(y) : null;
        if (eski) {
          const yeni = new Uint8Array(eski.length + b.length);
          yeni.set(eski);
          yeni.set(b, eski.length);
          d.set(y, yeni);
        } else {
          d.set(y, b);
        }
        return true;
      },
      sil: (y) => d.delete(y),
      tasi(a, b) {
        if (!d.has(a)) return false;
        d.set(b, d.get(a));
        d.delete(a);
        return true;
      },
      klasor: () => true,
    };
  }

  /** Tarayıcıda dosyalar localStorage'da durur (yoksa bellekte). */
  function tarayiciDosyalari(onek) {
    let depo = null;
    try {
      depo = window.localStorage;
      depo.getItem('orhunca');
    } catch (_) {
      return bellekDosyalari();
    }
    const anahtar = (y) => (onek || 'orhunca:') + y;
    const ikiliMetin = (b) => {
      let s = '';
      for (let i = 0; i < b.length; i += 8192) s += String.fromCharCode.apply(null, b.subarray(i, i + 8192));
      return s;
    };
    const baytlar = (s) => Uint8Array.from(s, (c) => c.charCodeAt(0));
    return {
      oku(y) {
        const s = depo.getItem(anahtar(y));
        return s == null ? null : baytlar(s);
      },
      yaz(y, b, ekle) {
        try {
          const eski = ekle ? depo.getItem(anahtar(y)) || '' : '';
          depo.setItem(anahtar(y), eski + ikiliMetin(b));
          return true;
        } catch (_) {
          return false;
        }
      },
      sil(y) {
        if (depo.getItem(anahtar(y)) == null) return false;
        depo.removeItem(anahtar(y));
        return true;
      },
      tasi(a, b) {
        const s = depo.getItem(anahtar(a));
        if (s == null) return false;
        depo.setItem(anahtar(b), s);
        depo.removeItem(anahtar(a));
        return true;
      },
      klasor: () => true,
    };
  }

  /* ---------------------------------------------------------------------- */
  /* Arayüz: öğe ağacını DOM'a çizme                                          */
  /* ---------------------------------------------------------------------- */

  const ETIKETLER = {
    'başlık': ['h1', 'baslik'],
    'alt_başlık': ['h2', 'alt-baslik'],
    'yazı': ['p', 'yazi'],
    'düğme': ['button', 'dugme'],
    'bağlantı': ['a', 'baglanti'],
    'resim': ['img', 'resim'],
    'ayraç': ['hr', 'ayrac'],
    'boşluk': ['div', 'bosluk'],
    'giriş': ['input', 'giris'],
    'metin_alanı': ['textarea', 'metin-alani'],
    'onay_kutusu': ['label', 'onay'],
    'seçim': ['select', 'secim'],
    'kaydırıcı': ['input', 'kaydirici'],
    'ilerleme': ['progress', 'ilerleme'],
    'satır': ['div', 'satir'],
    'sütun': ['div', 'sutun'],
    'kart': ['div', 'kart'],
    'kutu': ['div', 'kutu'],
    'ızgara': ['div', 'izgara'],
    'zamanlayıcı': ['span', 'zamanlayici'],
  };
  const KAPSAYICI = { 'satır': 1, 'sütun': 1, 'kart': 1, 'kutu': 1, 'ızgara': 1 };
  const YAZILI = { 'başlık': 1, 'alt_başlık': 1, 'yazı': 1, 'düğme': 1, 'bağlantı': 1 };
  const RENKLER = {
    'kırmızı': '#e5484d', 'yeşil': '#30a46c', 'mavi': '#3e63dd', 'sarı': '#e2a336',
    'turuncu': '#f76b15', 'mor': '#8e4ec6', 'pembe': '#e93d82', 'gri': '#8b8d98',
    'siyah': '#111113', 'beyaz': '#ffffff', 'lacivert': '#1e3a8a', 'kahverengi': '#8d5a3b',
    'turkuaz': '#12a594', 'açık_gri': '#f0f0f3', 'koyu_gri': '#3a3a40', 'şeffaf': 'transparent',
  };
  const GIRIS_TURLERI = { 'şifre': 'password', 'e_posta': 'email', 'tarih': 'date', 'renk': 'color', 'sayı': 'number', 'saat': 'time' };
  const HIZA = { 'sol': 'flex-start', 'orta': 'center', 'sağ': 'flex-end', 'iki_yana': 'space-between' };
  const YAZI_HIZA = { 'sol': 'left', 'orta': 'center', 'sağ': 'right', 'iki_yana': 'justify' };

  const TEMA = `
.ohc-uygulama{--ohc-yazi:#16191d;--ohc-soluk:#5f6873;--ohc-pano:#fff;--ohc-arka:#f4f5f7;--ohc-kenar:#dde1e6;--ohc-vurgu:#0b8a83;--ohc-vurgu-yazi:#fff;
  font:16px/1.5 system-ui,-apple-system,'Segoe UI',sans-serif;color:var(--ohc-yazi);max-width:880px;margin:0 auto;padding:24px 16px 48px;display:flex;flex-direction:column;gap:12px;box-sizing:border-box}
@media (prefers-color-scheme:dark){.ohc-uygulama{--ohc-yazi:#e6e9ee;--ohc-soluk:#9aa3b2;--ohc-pano:#171b21;--ohc-arka:#0f1216;--ohc-kenar:#2a3039;--ohc-vurgu:#3fd0c6;--ohc-vurgu-yazi:#071a19}}
.ohc-uygulama *{box-sizing:border-box}
.ohc-uygulama [hidden]{display:none!important}
.ohc-baslik{font-size:30px;line-height:1.2;margin:0;font-weight:700;letter-spacing:-.01em}
.ohc-alt-baslik{font-size:20px;margin:6px 0 0;font-weight:600}
.ohc-yazi{margin:0}
.ohc-dugme{font:inherit;font-weight:600;color:var(--ohc-vurgu-yazi);background:var(--ohc-vurgu);border:1px solid transparent;border-radius:9px;padding:8px 16px;cursor:pointer;transition:filter .12s,transform .06s}
.ohc-dugme:hover{filter:brightness(1.08)}.ohc-dugme:active{transform:translateY(1px)}
.ohc-dugme:disabled{opacity:.45;cursor:default;filter:none;transform:none}
.ohc-giris,.ohc-metin-alani,.ohc-secim{font:inherit;color:inherit;background:var(--ohc-pano);border:1px solid var(--ohc-kenar);border-radius:9px;padding:8px 12px;min-width:0}
.ohc-giris:focus,.ohc-metin-alani:focus,.ohc-secim:focus{outline:2px solid var(--ohc-vurgu);outline-offset:-1px;border-color:transparent}
.ohc-metin-alani{min-height:96px;resize:vertical;width:100%}
.ohc-satir .ohc-giris{flex:1}
.ohc-onay{display:inline-flex;align-items:center;gap:8px;cursor:pointer;user-select:none}
.ohc-onay input{width:18px;height:18px;accent-color:var(--ohc-vurgu);margin:0;flex-shrink:0}
.ohc-kaydirici{accent-color:var(--ohc-vurgu);width:100%}
.ohc-ilerleme{width:100%;height:10px;accent-color:var(--ohc-vurgu)}
.ohc-baglanti{color:var(--ohc-vurgu)}
.ohc-resim{max-width:100%;border-radius:10px;display:block}
.ohc-ayrac{border:0;border-top:1px solid var(--ohc-kenar);margin:4px 0;width:100%}
.ohc-bosluk{flex:1}
.ohc-satir{display:flex;align-items:center;gap:10px}
.ohc-satir>*{min-width:0}
.ohc-satir>.ohc-dugme{flex-shrink:0;white-space:nowrap}
.ohc-sutun{display:flex;flex-direction:column;gap:10px}
.ohc-kart{display:flex;flex-direction:column;gap:10px;background:var(--ohc-pano);border:1px solid var(--ohc-kenar);border-radius:14px;padding:16px 18px;box-shadow:0 1px 2px rgba(0,0,0,.05),0 4px 16px rgba(0,0,0,.04)}
.ohc-kutu{display:flex;flex-direction:column;gap:10px}
.ohc-izgara{display:grid;gap:12px}
.ohc-tiklanir{cursor:pointer}
`;

  function temaEkle(belge) {
    if (belge.getElementById('ohc-tema')) return;
    const st = belge.createElement('style');
    st.id = 'ohc-tema';
    st.textContent = TEMA;
    belge.head.appendChild(st);
  }

  const renk = (r) => RENKLER[r] || r;

  /** Arka plan rengine göre okunaklı yazı rengi (#rgb ya da #rrggbb; bilinmiyorsa beyaz). */
  function zitRenk(arka) {
    let h = renk(arka).trim();
    if (/^#[0-9a-f]{3}$/i.test(h)) h = '#' + h.slice(1).replace(/./g, '$&$&');
    if (!/^#[0-9a-f]{6}$/i.test(h)) return '#fff';
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
    const isik = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    return isik > 0.6 ? '#16191d' : '#fff';
  }
  const uzunlukCss = (d) => (/^-?\d+(\.\d+)?$/.test(d) ? d + 'px' : d);

  /** Seçeneklerden satır içi stil */
  function stil(v) {
    const o = v.oz;
    const st = [];
    if (o.renk) st.push('color:' + renk(o.renk));
    if (o.arka) {
      st.push('background:' + renk(o.arka));
      if (!o.renk && v.tur === 'düğme') st.push('color:' + zitRenk(o.arka));
    }
    if (o.boyut) st.push('font-size:' + uzunlukCss(o.boyut));
    if (o['kalın'] === 'doğru') st.push('font-weight:700');
    if (o['kalın'] === 'yanlış') st.push('font-weight:400');
    if (o['eğik'] === 'doğru') st.push('font-style:italic');
    if (o['genişlik']) st.push('width:' + uzunlukCss(o['genişlik']));
    if (o['yükseklik']) st.push('height:' + uzunlukCss(o['yükseklik']));
    if (o['boşluk']) st.push('gap:' + uzunlukCss(o['boşluk']));
    if (o['iç_boşluk']) st.push('padding:' + uzunlukCss(o['iç_boşluk']));
    if (o['köşe']) st.push('border-radius:' + uzunlukCss(o['köşe']));
    if (o['kenarlık']) st.push('border:1px solid ' + renk(o['kenarlık']));
    if (o.hizala) {
      if (v.tur === 'satır') st.push('justify-content:' + (HIZA[o.hizala] || o.hizala));
      else if (KAPSAYICI[v.tur]) st.push('align-items:' + (HIZA[o.hizala] || o.hizala), 'text-align:' + (YAZI_HIZA[o.hizala] || o.hizala));
      else st.push('text-align:' + (YAZI_HIZA[o.hizala] || o.hizala));
    }
    if (v.tur === 'ızgara') st.push('grid-template-columns:repeat(' + (parseInt(o['sütun'], 10) || 1) + ',minmax(0,1fr))');
    return st.join(';');
  }

  /** Kullanıcının yazmakta olduğu değeri bozmadan değeri günceller: program
   * değeri değiştirdiyse (son çizimden farklıysa) kutuya yazılır. */
  function degerYaz(el, deger) {
    if (deger === el.__son) return;
    el.__son = deger;
    if (el.value !== deger) el.value = deger;
  }

  function domCizici(kok, tetikle) {
    const belge = kok.ownerDocument;
    temaEkle(belge);
    kok.classList.add('ohc-uygulama');

    const yarat = (v) => {
      const [etiket, sinif] = ETIKETLER[v.tur] || ['div', 'kutu'];
      const el = belge.createElement(etiket);
      el.__v = { tur: v.tur };
      el.__olay = {};
      el.__sinif = 'ohc-' + sinif;
      const olay = (ad, deger) => {
        const no = el.__olay[ad];
        if (no != null) tetikle(no, deger);
      };
      if (v.tur === 'onay_kutusu') {
        const kutu = belge.createElement('input');
        kutu.type = 'checkbox';
        el.append(kutu, belge.createElement('span'));
        kutu.addEventListener('change', () => olay('bağ', kutu.checked ? 'doğru' : 'yanlış'));
      } else if (v.tur === 'giriş' || v.tur === 'metin_alanı' || v.tur === 'kaydırıcı') {
        if (v.tur === 'kaydırıcı') el.type = 'range';
        el.addEventListener('input', () => olay('bağ', el.value));
        if (v.tur === 'giriş') {
          el.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.isComposing) olay('gönderilince', el.value);
          });
        }
      } else if (v.tur === 'seçim') {
        el.addEventListener('change', () => olay('bağ', el.value));
      } else if (v.tur === 'bağlantı') {
        el.target = '_blank';
        el.rel = 'noopener';
      } else if (v.tur === 'zamanlayıcı') {
        el.hidden = true;
      }
      el.addEventListener('click', (e) => {
        if (el.__olay['tıklanınca'] == null) return;
        e.stopPropagation();
        olay('tıklanınca', null);
      });
      guncelle(el, v);
      return el;
    };

    const guncelle = (el, v) => {
      const o = v.oz;
      el.__olay = v.olay;
      let sinif = el.__sinif + (v.olay['tıklanınca'] != null && !YAZILI[v.tur] ? ' ohc-tiklanir' : '') + (o['sınıf'] ? ' ' + o['sınıf'] : '');
      if (el.className !== sinif) el.className = sinif;
      const css = stil(v);
      if (el.getAttribute('style') !== css) {
        if (css) el.setAttribute('style', css);
        else el.removeAttribute('style');
      }
      el.hidden = o.gizli === 'doğru' || v.tur === 'zamanlayıcı';
      if (o.ipucu != null) el.title = o.ipucu;
      else if (el.title) el.removeAttribute('title');
      if (YAZILI[v.tur] && el.textContent !== (o.metin || '')) el.textContent = o.metin || '';
      switch (v.tur) {
        case 'düğme':
          el.disabled = o.etkin === 'yanlış';
          el.type = 'button';
          break;
        case 'bağlantı':
          if (el.getAttribute('href') !== o.adres) el.href = o.adres || '#';
          break;
        case 'resim':
          if (el.getAttribute('src') !== o.adres) el.src = o.adres || '';
          el.alt = o['açıklama'] || '';
          break;
        case 'giriş':
        case 'metin_alanı': {
          if (v.tur === 'giriş') {
            const tur = GIRIS_TURLERI[o['tür']] || 'text';
            if (el.type !== tur) el.type = tur;
          }
          el.placeholder = o.yer_tutucu || '';
          el.disabled = o.etkin === 'yanlış';
          degerYaz(el, o['değer'] || '');
          break;
        }
        case 'kaydırıcı':
          el.min = o.en_az || '0';
          el.max = o.en_fazla || '100';
          el.step = o['adım'] || 'any';
          el.disabled = o.etkin === 'yanlış';
          degerYaz(el, o['değer'] || '0');
          break;
        case 'ilerleme':
          el.max = parseFloat(o.en_fazla || '100') || 100;
          el.value = parseFloat(o['değer'] || '0') || 0;
          break;
        case 'onay_kutusu': {
          const [kutu, yazi] = el.children;
          kutu.checked = o['değer'] === 'doğru';
          kutu.disabled = o.etkin === 'yanlış';
          if (yazi.textContent !== (o.metin || '')) yazi.textContent = o.metin || '';
          break;
        }
        case 'seçim': {
          const secenekler = o['seçenekler'] || '[]';
          if (el.__secenekler !== secenekler) {
            el.__secenekler = secenekler;
            el.textContent = '';
            for (const s of JSON.parse(secenekler)) {
              const op = belge.createElement('option');
              op.value = op.textContent = s;
              el.appendChild(op);
            }
          }
          el.disabled = o.etkin === 'yanlış';
          if (el.value !== o['değer']) el.value = o['değer'] || '';
          break;
        }
        case 'zamanlayıcı': {
          const sure = Math.max(0.01, parseFloat(o['süre'] || '1')) * 1000;
          if (el.__sure !== sure) {
            clearInterval(el.__zaman);
            el.__sure = sure;
            el.__zaman = setInterval(() => {
              if (el.isConnected && el.__olay['çalınca'] != null) tetikle(el.__olay['çalınca'], null);
            }, sure);
          }
          break;
        }
      }
      if (KAPSAYICI[v.tur]) cocuklar(el, v.cocuk);
    };

    const temizle = (el) => {
      if (el.__zaman) clearInterval(el.__zaman);
      for (const c of el.children || []) temizle(c);
    };

    const cocuklar = (kap, yeniler) => {
      for (let i = 0; i < yeniler.length; i++) {
        const v = yeniler[i];
        const el = kap.children[i];
        if (el && el.__v && el.__v.tur === v.tur) {
          guncelle(el, v);
        } else {
          const yeni = yarat(v);
          if (el) {
            temizle(el);
            kap.replaceChild(yeni, el);
          } else {
            kap.appendChild(yeni);
          }
        }
      }
      while (kap.children.length > yeniler.length) {
        const el = kap.lastElementChild;
        temizle(el);
        kap.removeChild(el);
      }
    };

    return (agac) => cocuklar(kok, agac.cocuk);
  }

  /* ---------------------------------------------------------------------- */
  /* Node.js                                                                 */
  /* ---------------------------------------------------------------------- */

  function nodeSecenekleri(argumanlar) {
    const fs = require('fs');
    const yaz = (akis, b) => {
      let i = 0;
      while (i < b.length) {
        try {
          i += fs.writeSync(akis === 2 ? 2 : 1, b, i, b.length - i);
        } catch (h) {
          if (h.code !== 'EAGAIN') throw h;
        }
      }
    };
    let tampon = Buffer.alloc(0);
    let bitti = false;
    const satirOku = () => {
      for (;;) {
        const n = tampon.indexOf(10);
        if (n >= 0) {
          const satir = tampon.subarray(0, n);
          tampon = tampon.subarray(n + 1);
          return new Uint8Array(satir);
        }
        if (bitti) {
          if (!tampon.length) return null;
          const satir = tampon;
          tampon = Buffer.alloc(0);
          return new Uint8Array(satir);
        }
        const parca = Buffer.alloc(65536);
        let okunan = 0;
        try {
          okunan = fs.readSync(0, parca, 0, parca.length, null);
        } catch (h) {
          if (h.code === 'EAGAIN') {
            uyu(10);
            continue;
          }
          if (h.code !== 'EOF') throw h;
        }
        if (okunan === 0) bitti = true;
        else tampon = Buffer.concat([tampon, parca.subarray(0, okunan)]);
      }
    };
    const uyu = (ms) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
    const dene = (f) => {
      try {
        f();
        return true;
      } catch (_) {
        return false;
      }
    };
    return {
      argumanlar,
      ortam: process.env,
      cikti: yaz,
      satirOku,
      bekle: (saniye) => uyu(saniye * 1000),
      dosyalar: {
        oku(y) {
          try {
            return new Uint8Array(fs.readFileSync(y));
          } catch (_) {
            return null;
          }
        },
        yaz: (y, b, ekle) => dene(() => fs.writeFileSync(y, b, { flag: ekle ? 'a' : 'w' })),
        sil: (y) => dene(() => fs.unlinkSync(y)),
        tasi: (a, b) => dene(() => fs.renameSync(a, b)),
        klasor: (y) => dene(() => fs.mkdirSync(y)),
      },
    };
  }

  /** node orhunca.js program.wasm [argümanlar...] */
  async function komutSatiri() {
    const fs = require('fs');
    const path = require('path');
    const [programYolu, ...argumanlar] = process.argv.slice(2);
    if (!programYolu) {
      process.stderr.write('kullanım: node orhunca.js program.wasm [argümanlar...]\n');
      process.exitCode = 2;
      return;
    }
    const s = nodeSecenekleri(argumanlar);
    s.calismaZamani = fs.readFileSync(path.join(__dirname, 'orhunca_rt.wasm'));
    s.program = fs.readFileSync(programYolu);
    const { kod, uygulama } = await baslat(s);
    if (uygulama) {
      process.stderr.write('Bu bir arayüz programı: tarayıcıda açın (orhunca derle dosya.ohc --hedef web)\n');
    }
    process.exitCode = kod;
  }

  return { calistir, baslat, komutSatiri, tarayiciDosyalari, bellekDosyalari, bicimle, sayiOku };
});
