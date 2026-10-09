/* Orhunca Stüdyo — Sayfalar: başlangıç, yeni proje, yapılandır, öğren.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // 01 Başlangıç
  // =====================================================================
  function cizBaslangic() {
    const ql = kucuk(D.q.trim());
    const liste = D.projeler.filter(p => !ql || kucuk(p.ad).includes(ql))
      .sort((a, b) => D.siralama === 'tarih' ? b.tarih - a.tarih : a.ad.localeCompare(b.ad, 'tr'));
    const satirlar = liste.map(p => `
      <div class="proje ${p.var ? '' : 'kayip'}" data-e="projeAc" data-a="${kac(p.yol)}" title="${p.var ? '' : 'Bu klasör artık yok'}">
        <div class="proje-simge">${S(p.simge || 'folder')}</div>
        <div style="min-width:0"><div class="proje-ad">${kac(p.ad)}<span>.ohcproj</span></div><div class="proje-yol">${kac(p.yol)}</div></div>
        <div class="proje-sag"><span class="etiket">${kac(p.var ? (p.sablon_adi || 'Orhunca projesi') : 'Bulunamadı')}</span><span class="proje-tarih">${tarihBicim(p.tarih)}</span></div>
      </div>`).join('');
    const bos = D.projeler.length === 0
      ? `<div class="bos-durum">Henüz bir proje yok.<br>Sağdaki <b>Yeni proje oluştur</b> ile başlayın ya da var olan bir projeyi açın.</div>`
      : liste.length === 0 ? `<div class="bos-durum">“${kac(D.q)}” ile eşleşen proje bulunamadı.</div>` : '';
    return `<div class="baslangic" data-screen-label="01 Başlangıç">
      <div class="baslangic-sol">
        <div><div class="selam">Tekrar hoş geldin, ${kac(kullaniciAdi())}</div><h1>Başlayın</h1></div>
        <div class="arama">${S('search')}<input id="q" data-g="q" value="${kac(D.q)}" placeholder="Son projelerde ara" autocomplete="off"><span class="tus">Alt+S</span></div>
        <div class="bolum-baslik"><h2>Son projeler</h2><span class="sayi-rozet">${liste.length}</span><div style="flex:1"></div>
          <div class="siralama" data-e="siralamaDegistir"><span>${D.siralama === 'tarih' ? 'Son açılma tarihi' : 'Ada göre (A–Z)'}</span>${S('swap_vert')}</div></div>
        <div class="proje-listesi">${satirlar}${bos}</div>
      </div>
      <div class="hizli">
        <h2>Hızlı işlemler</h2>
        <div class="islem vurgulu" data-e="git" data-a="yeni">${S('add')}<div class="esnek"><div class="islem-ust"><span class="islem-ad">Yeni proje oluştur</span><span class="islem-tus">Ctrl+⇧+N</span></div><div class="islem-alt">Konsol, web sitesi, API ya da kütüphane şablonu seçin</div></div></div>
        <div class="islem" data-e="klasorModal" data-a="ac">${S('folder_open')}<div class="esnek"><div class="islem-ad">Var olan projeyi aç</div><div class="islem-alt">Bir klasör veya .ohcproj dosyası seçin</div></div></div>
        <div class="islem" data-e="klonlaModal">${S('cloud_download')}<div class="esnek"><div class="islem-ad">Depodan klonla</div><div class="islem-alt">Git deposundan proje indirin</div></div></div>
        <div class="islem" data-e="dersleriAc">${S('school')}<div class="esnek"><div class="islem-ad">Derslerle öğren</div><div class="islem-alt">13 ders, otomatik denetlenen alıştırmalar</div></div></div>
        <div class="islem" data-e="git" data-a="ogren">${S('menu_book')}<div class="esnek"><div class="islem-ad">Başvuru rehberi</div><div class="islem-alt">Anahtar kelimeler, hâl ekleri, kütüphane</div></div></div>
        <div style="flex:1"></div>
        ${D.guncelleme?.yeni
          ? `<div class="guncelleme yeni" data-e="yeniSurumModal">${S('cloud_download')}<div class="esnek"><div class="guncelleme-ad">Orhunca ${kac(D.guncelleme.surum)} hazır</div><div class="guncelleme-alt">Yeni sürümü görmek ve tek tıkla güncellemek için tıklayın.</div></div>${S('chevron_right')}</div>`
          : `<div class="guncelleme" data-e="guncellemeModal">${S('new_releases')}<div class="esnek"><div class="guncelleme-ad">Orhunca ${kac(D.bilgi?.surum || '')}</div><div class="guncelleme-alt">Dersler, tablo ve grafikler, tarih ve desen işlevleri, paket dizini.</div></div>${S('chevron_right')}</div>`}
      </div>
    </div>`;
  }

  // =====================================================================
  // 02 Yeni proje
  // =====================================================================
  function dosyaSimgesi(ad, klasor) {
    if (klasor) return { gokturk: false, simge: 'folder_open' };
    const u = uzanti(ad);
    if (u === 'ohc' || u === 'ohchtml') return { gokturk: true };
    const tablo = { ohcproj: 'tune', md: 'info', json: 'data_object', svg: 'image', png: 'image', jpg: 'image', jpeg: 'image', gif: 'image', webp: 'image', ico: 'image', css: 'css', js: 'javascript', html: 'html', htm: 'html', txt: 'description' };
    return { gokturk: false, simge: tablo[u] || 'description' };
  }

  /** ['a/b.ohc', 'c.ohc'] → girintili ağaç satırları */
  function agacSatirlari(dosyalar) {
    const satirlar = [], gorulen = new Set();
    for (const f of dosyalar) {
      const p = f.split('/');
      for (let i = 0; i < p.length - 1; i++) {
        const a = p.slice(0, i + 1).join('/');
        if (!gorulen.has(a)) { gorulen.add(a); satirlar.push({ ad: p[i], derinlik: i, klasor: true, yol: a }); }
      }
      satirlar.push({ ad: p[p.length - 1], derinlik: p.length - 1, klasor: false, yol: f });
    }
    return satirlar;
  }

  function cizYeni() {
    const tql = kucuk(D.tq.trim());
    const liste = D.sablonlar.filter(t => (D.kategori === 'Tümü' || t.kategoriler.includes(D.kategori))
      && (!tql || kucuk(t.ad + ' ' + t.aciklama + ' ' + t.etiketler.join(' ')).includes(tql)));
    const sec = sablon(D.secili) || {};
    // Arama seçili şablonu gizlediyse soldaki dosya listesi başka bir şablonu göstermesin.
    const gorunur = !!sec.kimlik && liste.some(t => t.kimlik === sec.kimlik);
    const son = (D.sonSablonlar.length ? D.sonSablonlar : ['konsol', 'kutuphane']).map(sablon).filter(Boolean);
    const onizleme = agacSatirlari((sec.dosyalar || []).map(f => f.replace('{ad}', D.projeAdi.trim() || sec.kimlik))).map(r => {
      const s = dosyaSimgesi(r.ad, r.klasor);
      return `<div class="agac-satir" style="padding-left:${12 + r.derinlik * 14}px;color:${r.klasor ? 'var(--yazi2)' : 'var(--ikincil)'}">${s.gokturk ? `<span class="gokturk">${GOKTURK}</span>` : S(s.simge, '', `color:${r.klasor ? 'var(--sari)' : 'var(--soluk2)'}`)}<span>${kac(r.ad)}</span></div>`;
    }).join('');
    const kartlar = liste.map(t => {
      const secili = t.kimlik === D.secili;
      return `<div class="sablon ${secili ? 'secili' : ''} ${t.yakinda ? 'yakinda' : ''}" data-e="sablonSec" data-ee="sablonCift" data-a="${t.kimlik}">
        <div class="sablon-simge">${S(t.simge)}</div>
        <div style="min-width:0"><div class="sablon-ad">${kac(t.ad)}</div><div class="sablon-aciklama">${kac(t.aciklama)}</div>
          <div class="etiketler">${t.etiketler.map(g => `<span>${kac(g)}</span>`).join('')}</div></div>
        ${secili ? S('check_circle', 'dolu tik') : '<span></span>'}
        ${t.yakinda ? `<div class="yakinda-not" style="grid-column:2;margin-top:-4px"><div class="etiketler" style="margin-top:0"><span class="kilit">${S('lock', '', 'font-size:13px')}${kac(t.yakinda)}'da geliyor</span></div></div>` : ''}
      </div>`;
    }).join('');
    const kategoriler = ['Tümü', 'Web', 'Sunucu', 'Masaüstü', 'Konsol', 'Kütüphane'];
    return `<div class="sihirbaz" data-screen-label="02 Yeni proje">
      <div class="ust-satir"><div class="geri simge" data-e="git" data-a="baslangic">arrow_back</div><h1>Yeni proje oluşturun</h1><div style="flex:1"></div><span class="adim">Adım 1 / 2 · Şablon</span></div>
      <div class="yeni-govde">
        <div class="yeni-sol">
          <h2>Son kullanılan şablonlar</h2>
          ${son.map(t => `<div class="son-sablon" data-e="sablonSec" data-a="${t.kimlik}">${S(t.simge)}<span class="son-sablon-ad">${kac(t.ad)}</span><span class="son-sablon-alt">Orhunca</span></div>`).join('')}
          <div class="ince-ayrac"></div>
          ${gorunur ? `<div><div class="kucuk-baslik">Oluşturulacak dosyalar</div><div class="secili-ad">${kac(sec.ad)}</div></div>
          <div class="dosya-onizleme">${onizleme}</div>` : `<div class="panel-not">${D.sablonlar.length ? 'Sağdaki listeden bir şablon seçin.' : 'Şablonlar yükleniyor…'}</div>`}
        </div>
        <div class="yeni-sag">
          <div class="arama">${S('search')}<input id="tq" data-g="tq" value="${kac(D.tq)}" placeholder="Şablon ara (ör. konsol, oyun, kütüphane)" autocomplete="off"></div>
          <div class="kategoriler">${kategoriler.map(k => `<div class="kategori ${D.kategori === k ? 'etkin' : ''}" data-e="kategoriSec" data-a="${k}">${k}</div>`).join('')}</div>
          <div class="sablon-listesi">${kartlar}${liste.length ? '' : '<div class="bos-durum">Bu filtreyle eşleşen şablon yok.</div>'}</div>
        </div>
      </div>
      <div class="alt-cubuk"><span class="ipucu-metni">İpucu: şablona çift tıklayarak doğrudan devam edebilirsiniz.</span>
        <div class="dugme" data-e="git" data-a="baslangic">Geri</div>
        <div class="dugme birincil ${sec.yakinda || !gorunur ? 'pasif' : ''}" data-e="git" data-a="yapilandir">Sonraki</div></div>
    </div>`;
  }

  // =====================================================================
  // 03 Yapılandır
  // =====================================================================
  function adHatasi() {
    const n = D.projeAdi.trim();
    if (!n) return 'Proje adı gerekli.';
    if (!/^[\p{L}\p{N}_-]+$/u.test(n)) return 'Yalnızca harf, rakam, alt çizgi (_) ve tire (-) kullanılabilir.';
    if (D.mevcutAdlar.has(n)) return 'Bu konumda aynı adlı bir proje zaten var.';
    return null;
  }

  function cizYapilandir() {
    const sec = sablon(D.secili), hata = adHatasi();
    const konum = D.konum || D.bilgi.varsayilan_konum;
    const secenekler = [['git', 'Git deposu başlat', 'Proje klasöründe yeni bir depo ve .gitignore oluşturur.'],
      ['ornek', 'Örnek içerik ekle', sec.web ? 'Şablonu çalışan bir örnek sayfayla doldurur.' : 'Şablonu çalışan bir örnek programla doldurur.'],
      sec.web ? ['canli', 'Açılınca çalıştır', 'Sunucu hemen başlar ve sayfa sağdaki önizlemede açılır.']
        : ['calistir', 'Açılınca çalıştır', 'Proje açıldığında ilk çalıştırma terminalde gösterilir.']];
    return `<div class="sihirbaz" data-screen-label="03 Yapılandır">
      <div class="ust-satir"><div class="geri simge" data-e="git" data-a="yeni">arrow_back</div><h1>Projenizi yapılandırın</h1><div style="flex:1"></div><span class="adim">Adım 2 / 2 · Ayarlar</span></div>
      <div class="yapilandir-govde"><div class="form">
        <div class="ozet-kart"><div class="ozet-simge">${S(sec.simge)}</div><div class="esnek"><div class="ozet-ad">${kac(sec.ad)}</div><div class="ozet-alt">${kac(sec.etiketler.join(' · '))}</div></div><div class="baglanti" data-e="git" data-a="yeni">Değiştir</div></div>
        <div class="alan"><label for="projeAdi">Proje adı</label>
          <input id="projeAdi" data-g="projeAdi" class="metin-girdi ${hata ? 'hatali' : ''}" value="${kac(D.projeAdi)}" spellcheck="false" autocomplete="off">
          ${hata ? `<div class="alan-hata">${S('error')}${kac(hata)}</div>` : ''}</div>
        <div class="alan"><label for="konum">Konum</label>
          <div class="yan-yana"><input id="konum" data-g="konum" class="metin-girdi" style="flex:1;font-size:13px" value="${kac(konum)}" spellcheck="false" autocomplete="off"><div class="kare-dugme simge" data-e="klasorModal" data-a="konum" title="Klasör seç">folder_open</div></div></div>
        <div class="yol-kutusu"><div>Proje şu klasörde oluşturulacak</div><div>${kac(konum.replace(/[\\/]+$/, '') + ayrac() + (D.projeAdi.trim() || '…'))}</div></div>
        <div class="alan" style="gap:2px"><div style="font-size:13px;font-weight:500;color:var(--yazi2);margin-bottom:6px">Seçenekler</div>
          ${secenekler.map(([a, ad, alt]) => `<div class="secenek" data-e="secenekDegistir" data-a="${a}"><div class="esnek"><div class="secenek-ad">${ad}</div><div class="secenek-alt">${alt}</div></div><div class="anahtar ${D.secenekler[a] ? 'acik' : ''}"><div></div></div></div>`).join('')}
        </div>
      </div></div>
      <div class="alt-cubuk"><div class="dugme" data-e="git" data-a="yeni">Geri</div>
        <div class="dugme birincil ${hata ? 'pasif' : ''}" data-e="olustur"><span>Oluştur</span>${S('arrow_forward', '', 'font-size:18px')}</div></div>
    </div>`;
  }

  // =====================================================================
  // 05 Öğren
  // =====================================================================
  const ANAHTAR_TABLOSU = [
    ['x = değer', 'let / var', 'yaş = 21'],
    ['sabit', 'const', 'sabit PI = 3.14159'],
    ['işlev', 'function', 'işlev topla(a, b):'],
    ['kütüphane "m":', 'extern "C" / ctypes', 'kütüphane "m": işlev sqrt(x: ondalık) -> ondalık'],
    ['fiil', '(Orhunca’ya özgü)', "fiil sayı'yı karele:"],
    ['eğer … ise / değilse', 'if / else', "eğer yaş 18'den büyükse:"],
    ['her … için', 'for each', 'her öğe için listeden:'],
    ['… olduğu sürece', 'while', "x 10'dan küçük olduğu sürece:"],
    ['dur / sürdür', 'break / continue', 'dur'],
    ['döndür', 'return', 'döndür sonuç'],
    ['dene / yakala', 'try / catch', 'dene:  …  yakala hata:'],
    ['hata_ver(mesaj)', 'throw', 'hata_ver("geçersiz değer")'],
    ["… 'i yaz", 'print', '"Merhaba"\'yı yaz.'],
    ['ve / veya / değil', 'and / or / not', 'eğer a ve değil b ise:'],
    ['kullan', 'import', 'kullan "araçlar.ohc"'],
    ['model', 'class / struct', 'model Ürün:'],
    ['seçenek', 'enum', 'seçenek Renk: kırmızı, yeşil'],
    ["… 'i kaydet", 'save', "ürün'ü kaydet."],
    ['al / gönder "/yol":', 'GET / POST route', 'al "/ürünler":'],
    ['görünüm("ad", x)', 'render view', 'döndür görünüm("ürünler", liste)'],
    ['durum', 'state (useState)', 'durum sayaç = 0'],
    ['arayüz:', 'render / build()', 'arayüz:'],
    ['… tıklanınca:', 'onClick', 'düğme("Artır") tıklanınca:'],
    ['giriş(durum)', 'bound input (v-model)', 'giriş(ad, "Adınız")'],
    ['bileşen', 'component', 'bileşen Kart(başlık: metin):'],
  ];
  const HAL_TABLOSU = [
    ['-(y)ı / -(y)i', 'belirtme · nesne', "5'i sayılara ekle."],
    ['-(y)a / -(y)e', 'yönelme · hedef', "metni \"not.txt\"'ye yaz."],
    ['-dan / -den', 'ayrılma · kaynak', "her sayı için sayılardan:"],
    ['-(n)ın / -(n)in', 'ilgi · sahiplik', 'sayıların uzunluğunu yaz.'],
    ['-(y)la / -(y)le', 'vasıta · araç', "ad'ı (x > 0)'la doğrula."],
  ];

  function cizOgren() {
    const tablo = (b, satirlar, sinif = '') => `<div class="tablo ${sinif}"><div class="tablo-baslik">${b.map(x => `<span>${x}</span>`).join('')}</div>
      ${satirlar.map(([a, b2, c]) => `<div class="tablo-satir"><span>${kac(a)}</span><span>${kac(b2)}</span><span>${sinif ? kac(c) : vurgulaSatir(c)}</span></div>`).join('')}</div>`;
    return `<div class="ogren" data-screen-label="05 Öğren"><div class="ogren-ic">
      <div><h1>Öğren</h1><p class="giris">Orhunca'da anahtar kelimeler Türkçedir ve cümleler Türkçe gibi kurulur: değerler hâl ekleriyle işaretlenir, fiil sona gelir. Başka bir dilden geliyorsanız karşılıkları aşağıda.</p></div>
      ${tablo(['Orhunca', 'Karşılığı', 'Örnek'], ANAHTAR_TABLOSU)}
      <h2>Hâl ekleri</h2>
      ${tablo(['Ek', 'Rolü', 'Örnek'], HAL_TABLOSU)}
      <div class="cagri-kart"><div class="esnek"><div class="cagri-ad">İlk programını yaz</div><div class="cagri-alt">Konsol Uygulaması şablonuyla başla, F5 ile çalıştır, sonucu terminalde anında gör.</div></div><div class="dugme birincil" data-e="ilkProgram">Başla</div></div>
      <div class="cagri-kart"><div class="esnek"><div class="cagri-ad">Derslerle öğren</div><div class="cagri-alt">13 ders, otomatik denetlenen alıştırmalar: ilk programdan arayüz ve web uygulamalarına.</div></div><div class="dugme" data-e="dersleriAc">Derslere başla</div></div>
      <h2>Yerleşik işlevler</h2>
      <input id="basvuruAra" data-g="basvuruAra" class="metin-girdi basvuru-ara" placeholder="İşlev ara (ör. tarih, büyük harf, dosya)" spellcheck="false" autocomplete="off">
      <div id="basvuru">${[...new Set(D.yerlesikler.map(y => y.bolum))].map(b => `<div class="basvuru-bolum"><h3>${kac(b)}</h3>${tablo(['İşlev', 'Kullanım', 'Açıklama'], D.yerlesikler.filter(y => y.bolum === b).map(y => [y.ad, y.kullanim, y.aciklama]), 'genis')}</div>`).join('')}</div>
      <div class="panel-not gizli" id="basvuruYok">Bu aramaya uyan işlev yok.</div>
    </div></div>`;
  }

