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

  /**
   * Programı çalıştırır ve çıkış kodunu döndürür.
   *
   * s.calismaZamani, s.program: wasm baytları ya da WebAssembly.Module
   * s.cikti(akis, baytlar): 1 çıktı, 2 hata akışı (Uint8Array, UTF-8)
   * s.satirOku(): klavyeden bir satır (metin), girdi bittiyse null
   * s.dosyalar: { oku(yol) → Uint8Array|null, yaz(yol, baytlar, ekle) → bool,
   *               sil(yol) → bool, tasi(eski, yeni) → bool, klasor(yol) → bool }
   * s.bekle(saniye), s.ortam (nesne), s.argumanlar (dizi)
   */
  async function calistir(s) {
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

    const rtOrnek = await ornekle(s.calismaZamani, { js });
    rt = rtOrnek.exports;
    bellek = rt.memory;
    const prog = await ornekle(s.program, { rt });
    try {
      rt.ohc_wasm_baslat();
      prog.exports.ohc_ana();
      rt.ohc_wasm_bitir();
      return 0;
    } catch (h) {
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
    }
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
    process.exitCode = await calistir(s);
  }

  return { calistir, komutSatiri, tarayiciDosyalari, bellekDosyalari, bicimle, sayiOku };
});
