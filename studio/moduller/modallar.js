/* Orhunca Stüdyo — İletişim kutuları (modallar).
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Modallar
  // =====================================================================
  /**
   * Onay ve metin soruları için Stüdyo'nun kendi penceresi. Tarayıcının confirm()/prompt()
   * pencereleri görünümde tutarsız ve bazı WebView'larda (macOS WKWebView) hiç çalışmaz.
   * Açık bir pencerenin üstünde sorulursa yanıttan sonra o pencereye dönülür.
   */
  let soruCozucu = null;
  function soruSor({ baslik = 'Onay', mesaj = '', girdi = null, dugme = 'Tamam', tehlikeli = false }) {
    if (soruCozucu) soruBitir(null);
    return new Promise(coz => {
      soruCozucu = coz;
      D.menu = null;
      D.modal = { tur: 'soru', baslik, mesaj, girdi, dugme, tehlikeli, onceki: D.modal };
      katmanlariCiz();
      setTimeout(() => { const g = $('#soruGirdi'); if (g) { g.focus(); g.select(); } else $('#soruOnay')?.focus(); }, 30);
    });
  }
  /** Evet/hayır sorusu: onaylanırsa true. */
  const onayla = (mesaj, ayar = {}) => soruSor({ mesaj, ...ayar }).then(r => r !== null);
  /** Metin sorusu: vazgeçilirse null. */
  const metinSor = (mesaj, varsayilan = '', ayar = {}) => soruSor({ baslik: 'Girdi', mesaj, girdi: varsayilan, ...ayar });
  function soruBitir(sonuc) {
    const coz = soruCozucu, m = D.modal;
    soruCozucu = null;
    D.modal = m?.tur === 'soru' ? m.onceki || null : D.modal;
    katmanlariCiz();
    coz?.(sonuc);
  }

  function cizModal() {
    const m = D.modal;
    if (!m) return '';
    const kabuk = (baslik, govde, alt, sinif = '') => `<div class="ortu" data-e="modalDis"><div class="modal ${sinif}" data-e="hic">
      <div class="modal-baslik"><h2>${baslik}</h2>${S('close').replace('<span ', '<span data-e="modalKapat" ')}</div>
      <div class="modal-govde">${govde}</div>${alt ? `<div class="modal-alt">${alt}</div>` : ''}</div></div>`;
    if (m.tur === 'klasor') {
      const liste = m.yukleniyor ? '<div class="bos-durum"><div class="donen kucuk" style="margin:auto"></div></div>'
        : (m.klasorler || []).map((k, i) => `<div class="klasor-oge ${m.secili === i ? 'secili' : ''}" data-e="klasorSec" data-ee="klasorGir" data-a="${i}">
            ${k.proje ? `<span class="gokturk">${GOKTURK}</span>` : S('folder')}<span>${kac(k.ad)}</span>${k.proje ? '<span class="etiket">Orhunca projesi</span>' : ''}</div>`).join('')
          || '<div class="bos-durum">Bu klasörde alt klasör yok.</div>';
      const ac = m.mod === 'ac';
      return kabuk(ac ? 'Proje klasörünü seçin' : 'Konum seçin', `
        <div class="klasor-yolu"><div class="kare-dugme simge" data-e="klasorUst" title="Üst klasör">arrow_upward</div><input id="klasorYolu" data-g="klasorYolu" class="metin-girdi" value="${kac(m.yol || '')}" spellcheck="false"></div>
        <div class="klasor-listesi">${liste}</div>
        ${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}
        ${ac && m.proje ? `<div class="panel-not">${S('check_circle', 'dolu', 'font-size:16px;color:var(--vurgu);vertical-align:-3px')} Bu klasör bir Orhunca projesi.</div>` : ''}`,
        `<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="klasorOnayla">${ac ? 'Aç' : 'Bu konumu seç'}</div>`);
    }
    if (m.tur === 'klonla') {
      return kabuk('Depodan klonla', `
        <div class="alan"><label>Depo adresi</label><input id="klonUrl" data-g="klonUrl" class="metin-girdi" value="${kac(m.url)}" placeholder="https://github.com/kullanici/proje.git" spellcheck="false"></div>
        <div class="alan"><label>Konum</label><input id="klonKonum" data-g="klonKonum" class="metin-girdi" value="${kac(m.konum)}" spellcheck="false"></div>
        ${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}`,
        `${m.calisiyor ? '<div class="donen kucuk"></div><span class="ipucu-metni">Klonlanıyor…</span>' : ''}<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil ${m.calisiyor ? 'pasif' : ''}" data-e="klonla">Klonla</div>`);
    }
    if (m.tur === 'hizli') return cizHizli(m);
    if (m.tur === 'projeOzellikleri') return cizProjeOzellikleri(m, kabuk);
    if (m.tur === 'yeniOge') {
      return kabuk('Yeni öğe ekle', cizYeniOge(m),
        `<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="yeniOgeOlustur">Ekle</div>`, 'genis-orta');
    }
    if (m.tur === 'yeniDosya') {
      return kabuk(m.klasor ? 'Yeni klasör' : 'Yeni dosya', `
        <div class="alan"><label>Proje içindeki yol</label><input id="yeniDosyaAdi" data-g="yeniDosyaAdi" class="metin-girdi" value="${kac(m.ad)}" placeholder="${m.klasor ? 'araclar' : 'araclar.ohc'}" spellcheck="false"></div>
        ${m.hata ? `<div class="modal-hata">${kac(m.hata)}</div>` : ''}`,
        `<div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="yeniDosyaOlustur">Oluştur</div>`);
    }
    if (m.tur === 'ayarlar') {
      return kabuk('Ayarlar', `
        <div class="secenek" style="cursor:default"><div class="esnek"><div class="secenek-ad">Düzenleyici yazı boyutu</div><div class="secenek-alt">Kod ve satır numaraları</div></div>
          <div class="kare-dugme simge" style="width:32px;height:32px;font-size:18px" data-e="yaziKucult">remove</div><span class="mono" style="width:32px;text-align:center">${D.yaziBoyutu}</span><div class="kare-dugme simge" style="width:32px;height:32px;font-size:18px" data-e="yaziBuyut">add</div></div>
        ${duzenleyiciAyarlari()}<div class="ayar-bolum">Genel</div>
        <div class="secenek" data-e="yazarkenDenetleDegistir"><div class="esnek"><div class="secenek-ad">Yazarken denetle</div><div class="secenek-alt">Hatalar siz yazarken altı çizili gösterilir.</div></div><div class="anahtar ${D.yazarkenDenetle ? 'acik' : ''}"><div></div></div></div>
        <div class="secenek" style="cursor:default"><div class="esnek"><div class="secenek-ad">Tema</div><div class="secenek-alt">Sınıfta projektör için açık tema önerilir.</div></div>
          <div class="tema-secim">${[['koyu', 'Koyu'], ['acik', 'Açık'], ['sistem', 'Sistem']].map(([t, ad]) => `<span class="${!D.ozelTema && D.tema === t ? 'secili' : ''}" data-e="temaSec" data-a="${t}">${ad}</span>`).join('')}</div></div>
        <div class="secenek" data-e="gorunumModal"><div class="esnek"><div class="secenek-ad">Görünüm ve temalar</div><div class="secenek-alt">${D.ozelTema ? 'Etkin tema: ' + kac(D.ozelTema.ad) + ' · ' : ''}Renkler, yazı tipleri, arka plan resmi ya da GIF; temaları paylaşın.</div></div>${S('palette')}</div>
        <div class="secenek" data-e="otomatikKaydetDegistir"><div class="esnek"><div class="secenek-ad">Otomatik kaydet</div><div class="secenek-alt">Yazmayı bıraktıktan kısa süre sonra dosya kendiliğinden kaydedilir. Önceki hâller Dosya → Yerel geçmiş'te durur.</div></div><div class="anahtar ${D.otomatikKaydet ? 'acik' : ''}"><div></div></div></div>
        <div class="secenek" data-e="guncellemeDenetleDegistir"><div class="esnek"><div class="secenek-ad">Güncellemeleri denetle</div><div class="secenek-alt">Açılışta yeni sürüm olup olmadığına bakılır (GitHub'a tek bir istek; başka veri gönderilmez).</div></div><div class="anahtar ${D.guncellemeDenetle ? 'acik' : ''}"><div></div></div></div>
        ${D.asistan.durum?.kapali ? '' : `<div class="secenek" data-e="asistanGosterDegistir"><div class="esnek"><div class="secenek-ad">Yapay zekâ asistanı</div><div class="secenek-alt">Kenar çubuğunda asistan simgesi gösterilir. Claude, GPT, Gemini ya da bilgisayarınızdaki yerel modellerle (Ollama, LM Studio) çalışır.</div></div><div class="anahtar ${D.asistanGoster ? 'acik' : ''}"><div></div></div></div>`}
        <div class="secenek" data-e="acilisDegistir"><div class="esnek"><div class="secenek-ad">Açılış animasyonu</div><div class="secenek-alt">Stüdyo açılırken Orhunca logosu canlandırılır.</div></div><div class="anahtar ${D.acilis ? 'acik' : ''}"><div></div></div></div>`,
        `<div class="dugme birincil" data-e="modalKapat">Tamam</div>`);
    }
    if (m.tur === 'soru') return kabuk(kac(m.baslik), `<div class="soru-mesaj">${kac(m.mesaj)}</div>${m.girdi !== null ? `<input id="soruGirdi" class="metin-girdi" value="${kac(m.girdi)}" spellcheck="false" autocomplete="off">` : ''}`,
      `<div style="flex:1"></div><div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil ${m.tehlikeli ? 'tehlikeli' : ''}" id="soruOnay" tabindex="0" data-e="soruOnayla">${kac(m.dugme)}</div>`);
    if (m.tur === 'vt') return cizVt(kabuk);
    if (m.tur === 'gorunum') return cizGorunum(kabuk);
    if (m.tur === 'ceviri') return cizCeviri(kabuk);
    if (m.tur === 'gecmis') return cizGecmis(kabuk);
    if (m.tur === 'satiraGit') {
      const n = etkinSekme()?.icerik.split('\n').length || 1;
      return kabuk('Satıra git', `<div class="alan"><label>Satır numarası (1–${n})</label><input id="satirGirdi" class="metin-girdi mono" inputmode="numeric" autocomplete="off" placeholder="${D.imlec?.satir || 1}"></div>`,
        `<div style="flex:1"></div><div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="satiraGitOnayla">Git</div>`);
    }
    if (m.tur === 'kesmeAyar') return kabuk(`Kesme noktası · ${sonParca(m.dosya)}:${m.satir}`, `
      <div class="alan"><label>Koşul (boş bırakılırsa her geçişte durur)</label><input id="kesmeKosul" data-g="kesmeKosul" class="metin-girdi mono" value="${kac(m.kosul)}" placeholder="ör. i == 5 ya da toplam > 100" spellcheck="false" autocomplete="off"></div>
      <div class="alan"><label>Günlük mesajı (yazılırsa durmaz, terminale yazar; {ifade} değeri ekler)</label><input id="kesmeGunluk" data-g="kesmeGunluk" class="metin-girdi mono" value="${kac(m.gunluk)}" placeholder="ör. i = {i}, toplam {toplam}" spellcheck="false" autocomplete="off"></div>
      ${D.calisma?.ayikla ? '<div class="secenek-alt">Koşul ve günlük değişiklikleri hata ayıklama yeniden başlatılınca geçerli olur.</div>' : ''}`,
      `<div class="dugme" data-e="kesmeAyarKaldir">${S('delete')}Kesme noktasını kaldır</div><div style="flex:1"></div><div class="dugme" data-e="modalKapat">İptal</div><div class="dugme birincil" data-e="kesmeAyarKaydet">Kaydet</div>`);
    if (m.tur === 'fark') return kabuk(`${m.yol} ${m.hazir ? '(hazırlanan)' : ''}`, `<div class="fark">${m.metin == null ? '<div class="donen kucuk"></div>' : farkHtml(m.metin)}</div>`,
      `<div class="dugme" data-e="modalKapat">Kapat</div>${m.isleme ? '' : `<div class="dugme" data-e="farkDosyaAc">${S('open_in_new')}Dosyayı aç</div>`}`).replace('class="modal"', 'class="modal genis"');
    if (m.tur === 'ajan') return cizAjan(kabuk);
    if (m.tur === 'yeniSurum') {
      const g = D.guncelleme || {};
      const durum = D.guncellemeDurumu;
      return kabuk(`Orhunca ${kac(g.surum || '')}`, `
        <div class="secenek-alt" style="margin-bottom:12px">Kurulu sürüm: ${kac(g.simdiki || '')}. Dosya indirildikten sonra SHA-256 ile doğrulanır.</div>
        <div class="surum-notu md">${mdBasit(g.notlar || '')}</div>
        ${durum ? `<div class="guncelleme-durum">${durum === 'indiriliyor' ? '<div class="donen kucuk"></div> İndiriliyor ve doğrulanıyor…' : kac(durum)}</div>` : ''}`,
        `<div class="dugme" data-e="modalKapat">Daha sonra</div><div class="dugme birincil ${durum === 'indiriliyor' ? 'pasif' : ''}" data-e="guncellemeyiKur">${S('cloud_download')}Şimdi güncelle</div>`);
    }
    if (m.tur === 'guncelleme') {
      return kabuk('Sürüm notları', `
        <div class="surum-notu"><h3>1.2.0 · Ekim 2026</h3><ul><li>Gezginde sağ tık menüsü: sil, yeniden adlandır, çoğalt, kes/kopyala/yapıştır, sürükle-bırak, klasörde göster</li><li>Yeni öğe penceresi (Ctrl+Shift+A): model, sınama, web yolu, görünüm, bileşen…</li><li>Dosyada bul/değiştir (Ctrl+F / Ctrl+H), tanıma git (F12), hızlı aç (Ctrl+P), komut paleti (Ctrl+Shift+P)</li><li>Üzerine gelince bilgi, parametre yardımı, ekranı bölme, sekme menüsü</li><li>Proje özellikleri, ortam değişkenleri, düzenleyici ayarları, kaydederken biçimlendirme</li><li>Git dalları, işleme ayrıntısı, satırı kimin değiştirdiği</li><li>Resim/Markdown/CSV/JSON önizleme, kod parçacıkları, yer imleri, YAPILACAKLAR, çoklu kabuk, dosya karşılaştırma, sınama kapsamı</li></ul></div>
        <div class="surum-notu"><h3>1.1.1 · Ekim 2026</h3><ul><li>Windows'ta Güncelle düğmesi yönetici izniyle çalışıyor</li></ul></div>
        <div class="surum-notu"><h3>1.1 · Ekim 2026</h3><ul><li>SQLite, PostgreSQL, MySQL/MariaDB ve SQL Server; Stüdyo'da veritabanı görüntüleyici</li><li>15 resmi paket, 6 yeni şablon; KABUK sekmesi, profil, eklentiler, uzaktan geliştirme</li><li>Telefon: kamera, karekod, konum, dosya seçme</li></ul></div>
        <div class="surum-notu"><h3>1.0 · Ekim 2026</h3><ul><li>İlk kararlı sürüm: 1.0'da çalışan programlar 1.x boyunca bozulmaz</li><li>Dil, Stüdyo, web, arayüz, telefon, PHP ve VPS yayını tek pakette</li></ul></div>
        <div class="surum-notu"><h3>0.11 · Ekim 2026</h3><ul><li>Model işlevleri (<code>bu.ad</code>), adsız işlevler (<code>süz(l, işlev(x) -> x > 10)</code>), <code>arka planda:</code></li><li>Test çerçevesi (<code>orhunca sına</code>), Git paneli, gelişmiş hata ayıklayıcı (koşullu kesme, günlük noktası)</li><li>Yeniden adlandırma ve bütün başvurular (F2, ⇧+F12)</li><li>Web programlarını PHP + MySQL'e çevirme: <code>orhunca yayınla --php</code></li><li>Editör: parantez eşleştirme, otomatik kapatma, satır taşıma, satıra git</li></ul></div>
        <div class="surum-notu"><h3>0.10 · Ekim 2026</h3><ul><li>C kütüphanelerini çağırma: <code>kütüphane "m":</code></li><li>Paket mağazası: paketlerin izinleri gösterilir, onayınız alınır, içerik özeti denetlenir</li><li>Bootstrap: <code>@bootstrap</code>, Türkçe sınıf adları, <code>tema("bootstrap")</code></li><li>Hızlı düzelt, projede bul ve değiştir; kısıtlı mod</li><li>Yerleşik işlevler başvurusu (Öğren sayfası, <code>orhunca başvuru</code>)</li></ul></div>
        <div class="surum-notu"><h3>0.9 · Ekim 2026</h3><ul><li><code>orhunca yayınla</code>: VPS'e tek komutla kurulum, alan adı ve HTTPS</li><li>Paylaşımlı hosting (cPanel): <code>--cgi</code>; <code>.ohc</code> dosyaları PHP gibi çalışır</li><li>Telefonda ve tablette internetsiz kod yazma (tarayıcıda dene → ana ekrana ekle)</li><li>Türkçe klavyesi olmayanlar için Alt+C/G/I/O/S/U ve harfsiz yazımın düzeltilmesi</li><li>Yapay zekâ ajanları arayüz programlarını tıklayarak deneyebilir</li></ul></div>
        <div class="surum-notu"><h3>0.8 · Ekim 2026</h3><ul><li>Otomatik kaydetme ve yerel geçmiş (Dosya → Yerel geçmiş)</li><li><code>.env</code> desteği; Yardım → Hata bildir</li><li>Kaydedilmemiş kodun kaybolması ve kaydedilemeyen kodun çalışması düzeltildi</li><li>Python/JavaScript karşılığı Orhunca ile birebir aynı sonucu verir</li><li>Geriye uyumluluk: <code>dil = "1"</code>, <code>orhunca düzelt</code></li></ul></div>
        <div class="surum-notu"><h3>0.7.2 · Ekim 2026</h3><ul><li>Windows: Çalıştır'da boş konsol penceresi açılmıyor</li><li>Yapay zekâ: her istekte tam rehber yerine kısa rehber; ayrıntılar bölüm bölüm okunur (yaklaşık beşte bir belirteç)</li><li>Uzun program çıktıları modele kırpılarak gönderilir</li></ul></div>
        <div class="surum-notu"><h3>0.7 · Ekim 2026</h3><ul><li>Telefon uygulamaları: Android (<code>.apk</code>) ve iPhone/iPad (Xcode projesi); <code>titret</code>, <code>paylaş</code>, <code>bildirim_gönder</code></li><li>2B oyunlar: <code>oyun_alanı</code>, çizim, klavye, fare, dokunma ve ses</li><li>Python ve JavaScript karşılığını yan yana gösterme</li><li>Asistan: Claude, GPT, Gemini, OpenRouter, Groq, Mistral, DeepSeek, xAI ve yerel modeller (Ollama, LM Studio)</li><li>Claude Code, Codex, Cursor, VS Code gibi ajanlar için hazır bağlantı</li></ul></div>
        <div class="surum-notu"><h3>0.6 · Ekim 2026</h3><ul><li>Dersler paneli: 13 ders ve otomatik denetlenen alıştırmalar</li><li>Arayüz öğeleri: <code>tablo</code>, <code>grafik</code>, <code>sekmeler</code>, <code>iletişim_kutusu</code></li><li>Kütüphane: tarih, desenler, CSV, <code>json_al</code>, <code>http_al</code></li><li>Paket dizini: <code>orhunca paket ara</code>; istatistik ve geometri paketleri</li><li>Açık tema, tamamlama, adım adım gösterim, otomatik güncelleme</li></ul></div>
        <div class="surum-notu"><h3>0.5 · Ekim 2026</h3><ul><li>Türkçe arayüz dili: <code>durum</code>, <code>arayüz:</code>, <code>düğme("Ekle") tıklanınca:</code>, <code>giriş(ad)</code>, <code>bileşen</code></li><li>WebAssembly: <code>--hedef web</code> ile tarayıcıda çalışan tek dosyalık sayfa</li><li>Stüdyo: Arayüz Uygulaması şablonu ve canlı önizlemede çalışan uygulamalar</li><li>Öz-barındırmanın ilk adımı: Orhunca ile yazılmış sözcük çözümleyici</li><li>Tipi yazılmış değişkenler: <code>işler: liste&lt;metin&gt; = []</code>; <code>kod()</code> ve <code>karakter()</code></li></ul></div>
        <div class="surum-notu"><h3>0.4</h3><ul><li>Modeller: <code>model Ürün:</code>, alan kuralları ve Türkçe doğrulama mesajları</li><li>Kalıcı kayıtlar: <code>ürün'ü kaydet.</code>, <code>Ürün.hepsi()</code>, <code>Ürün.bul(3)</code></li><li>Web sunucusu: <code>al "/ürünler":</code>, formlar, JSON API, statik dosyalar</li><li><code>.ohchtml</code> görünümleri: <code>@model</code>, <code>@düzen</code>, <code>@eğer</code>, <code>@her</code></li><li>Stüdyo: web şablonları ve kaydedince yenilenen canlı önizleme</li></ul></div>
        <div class="surum-notu"><h3>0.3</h3><ul><li>Standart kütüphane: metin, liste, dosya, matematik ve zaman işlevleri</li><li><code>sözlük</code> tipi: <code>{"elma": 5}</code></li><li><code>kullan "dosya.ohc"</code> ile birden fazla dosya, <code>sabit</code> tanımları</li><li>Tamsayı taşması denetimi, Türkçe hata açıklamaları</li><li>Orhunca Stüdyo</li></ul></div>
        <div class="surum-notu"><h3>0.2</h3><ul><li>Ondalık sayılar</li><li>Kendi fiillerinizi tanımlama: <code>fiil sayı'yı karele:</code></li><li>Otomatik bellek yönetimi (çöp toplayıcı)</li></ul></div>
        <div class="surum-notu"><h3>0.1</h3><ul><li>İlk derleyici: hâl ekleri, Türkçe koşullar ve döngüler</li><li>Cranelift ile Linux ve Windows programları</li></ul></div>`,
        `<div class="dugme birincil" data-e="modalKapat">Kapat</div>`);
    }
    if (m.tur === 'kisayollar') {
      const k = [['Çalıştır', 'F5'], ['Hata ayıkla', 'F6'], ['Durdur', '⇧+F5'], ['Denetle', 'F7'], ['Kesme noktası', 'F9'], ['Üstünden / içine adım', 'F10 / F11'], ['Kaydet', 'Ctrl+S'], ['Tümünü kaydet', 'Ctrl+Alt+S'], ['Satırı yorum yap', 'Ctrl+/'], ['Biçimlendir', 'Ctrl+⇧+F'], ['Satırı çoğalt', 'Ctrl+⇧+D'], ['Satırı yukarı / aşağı taşı', 'Alt+↑ / Alt+↓'], ['Satıra git', 'Ctrl+G'], ['Dosyada bul / değiştir', 'Ctrl+F / Ctrl+H'], ['Hızlı aç / komut paleti', 'Ctrl+P / Ctrl+⇧+P'], ['Tanıma git / geri', 'F12 (Ctrl+tıklama) / Alt+←'], ['Kapanan sekmeyi aç / sekmeyi kapat', 'Ctrl+⇧+T / Ctrl+W'], ['Ekranı böl', 'Ctrl+\\'], ['Yer imi / sonraki / önceki', 'Ctrl+Alt+K / L / J'], ['Kod parçacığı (ör. işlev, eğer, her, model)', 'Tab'], ['Yeni öğe ekle', 'Ctrl+⇧+A'], ['Gezginde sil / adlandır', 'Delete / F2'], ['Yeniden adlandır / başvurular', 'F2 / ⇧+F12'], ['Alt paneli göster/gizle', 'Ctrl+J'], ['Girinti / geri girinti', 'Tab / ⇧+Tab'], ['Projelerde ara', 'Alt+S'], ['Yeni proje', 'Ctrl+⇧+N'], ['ç ğ ı ö ş ü (Türkçe klavyesi olmayanlar için)', 'Alt+C/G/I/O/S/U'], ['Ç Ğ İ Ö Ş Ü', 'Alt+⇧+C/G/I/O/S/U']];
      return kabuk('Klavye kısayolları', `<div class="kisayol-listesi">${k.map(([a, b]) => `<span>${a}</span><span>${b}</span>`).join('')}</div>`, `<div class="dugme birincil" data-e="modalKapat">Kapat</div>`);
    }
    if (m.tur === 'hataBildir') {
      const kutu = (a, ad, alt, pasif) => `<div class="secenek ${pasif ? 'pasif' : ''}" data-e="hataBildirSec" data-a="${a}"><div class="esnek"><div class="secenek-ad">${ad}</div><div class="secenek-alt">${alt}</div></div><div class="anahtar ${m[a] && !pasif ? 'acik' : ''}"><div></div></div></div>`;
      const sonHata = hataBildirSonHata();
      return kabuk(`${S('bug_report')} Hata bildir`, `
        <div class="alan" style="gap:6px"><label style="font-size:12.5px">Ne oldu? Ne yapmaya çalıştınız, ne bekliyordunuz?</label>
          <textarea class="metin-girdi" data-g="hataBildirNe" rows="5" style="height:auto;resize:vertical;font-family:inherit;line-height:1.45" placeholder="Örnek: Çalıştır'a basınca program açılıyor ama çıktı görünmüyor.">${kac(m.ne || '')}</textarea></div>
        ${kutu('kod', 'Açık dosyanın kodunu ekle', etkinSekme() && !etkinSekme().ikili ? kac(etkinSekme().yol) + ' (ilk 3000 karakter)' : 'Açık dosya yok', !(etkinSekme() && !etkinSekme().ikili))}
        ${kutu('cikti', 'Son hata mesajını ekle', sonHata ? kac(sonHata.split('\n')[0].slice(0, 90)) : 'Hata mesajı yok', !sonHata)}
        <div class="panel-not" style="font-size:12px;margin-top:8px">Eklenecek: Orhunca ${kac(D.bilgi.surum)} · ${kac(D.bilgi.isletim)}. Hiçbir şey kendiliğinden gönderilmez: GitHub'da hazır doldurulmuş bir sayfa açılır, göz atıp siz gönderirsiniz (GitHub hesabı gerekir). Kodunuzda kişisel bilgi varsa eklemeyin.</div>`,
        `<div class="dugme" data-e="modalKapat">Vazgeç</div><div class="dugme birincil" data-e="hataBildirGonder">${S('open_in_new')}GitHub'da aç</div>`);
    }
    if (m.tur === 'hakkinda') {
      return kabuk('Hakkında', `<div style="display:flex;gap:16px;align-items:center"><div class="logo-kutu" style="width:48px;height:48px"><img src="simge.svg" alt=""></div>
        <div><div style="font-size:16px;font-weight:600">Orhunca Stüdyo</div><div class="panel-not">${kac(surumAdi())} (${kac(D.bilgi.surum)}) · ${kac(D.bilgi.isletim)}</div></div></div>
        <div class="panel-not">Türkçe tabanlı programlama dili Orhunca için geliştirme ortamı. Derleyici Rust ile yazılmıştır ve Cranelift ile doğrudan makine kodu üretir.</div>
        <div class="panel-not"><a href="https://github.com/furkan003/orhunca" target="_blank" rel="noopener">github.com/furkan003/orhunca</a></div>`, `<div class="dugme birincil" data-e="modalKapat">Kapat</div>`);
    }
    return '';
  }

  function cizOlusturuluyor() {
    if (!D.olusturuluyor) return '';
    return `<div class="ortu"><div class="olusturuluyor"><div class="donen"></div><div>
      <div class="olusturuluyor-ad">“${kac(D.projeAdi.trim())}” oluşturuluyor</div>
      <div class="olusturuluyor-alt">Şablon dosyaları kopyalanıyor…</div></div></div></div>`;
  }

  function cizBildirim() {
    if (!D.bildirim) return '';
    return `<div class="bildirim ${D.bildirim.hata ? 'hata' : ''}">${S(D.bildirim.hata ? 'error' : 'check_circle')}<span>${kac(D.bildirim.metin)}</span></div>`;
  }

  let bildirimZamani;
  function bildir(metin, hata = false) {
    D.bildirim = { metin, hata };
    clearTimeout(bildirimZamani);
    bildirimZamani = setTimeout(() => { D.bildirim = null; katmanlariCiz(); }, hata ? 6000 : 3500);
    katmanlariCiz();
  }

