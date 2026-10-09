/* Orhunca Stüdyo — Menüler ve ortak parçalar.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Menüler
  // =====================================================================
  const MENULER = [
    { ad: 'Dosya', ogeler: [['Yeni dosya…', '', 'yeniDosyaModal'], ['Yeni klasör…', '', 'yeniKlasorModal'], '-', ['Kaydet', 'Ctrl+S', 'kaydet'], ['Tümünü kaydet', 'Ctrl+Alt+S', 'tumunuKaydet'], ['Yerel geçmiş…', '', 'gecmisModal'], ['Projeye güveni kaldır (kısıtlı mod)', '', 'guveniKaldir'], '-', ['Başlangıç ekranı', '', 'baslangicaDon'], ['Projeyi kapat', '', 'projeyiKapat'], '-', ["Stüdyo'yu kapat", '', 'studyoyuKapat']] },
    { ad: 'Düzen', ogeler: [['Geri al', 'Ctrl+Z', 'geriAl'], ['Yinele', 'Ctrl+Y', 'yinele'], '-', ['Kes', 'Ctrl+X', 'kes'], ['Kopyala', 'Ctrl+C', 'kopyala'], ['Yapıştır', 'Ctrl+V', 'yapistir'], '-', ['Satırı yorum yap', 'Ctrl+/', 'yorumYap'], ['Biçimlendir', 'Ctrl+⇧+F', 'bicimlendir'], '-', ['Yeniden adlandır…', 'F2', 'yenidenAdlandir'], ['Bütün başvurular', '⇧+F12', 'basvurulariBul']] },
    { ad: 'Seçim', ogeler: [['Tümünü seç', 'Ctrl+A', 'tumunuSec'], ['Satırı seç', 'Ctrl+L', 'satiriSec'], ['Satırı çoğalt', 'Ctrl+⇧+D', 'satiriCogalt'], ['Satırı yukarı taşı', 'Alt+↑', 'satiriYukari'], ['Satırı aşağı taşı', 'Alt+↓', 'satiriAsagi'], '-', ['Satıra git…', 'Ctrl+G', 'satiraGitModal']] },
    { ad: 'Görünüm', ogeler: [['Gezgin', '', 'panelGezgin'], ['Ara', '', 'panelAra'], ['Yapı', '', 'panelYapi'], ['Çalıştır', '', 'panelCalistir'], '-', ['Alt paneli göster/gizle', 'Ctrl+J', 'altPanelAcKapa'], ['Yapay zekâ asistanı', 'Ctrl+I', 'asistanAcKapa'], ['Canlı önizleme (web)', '', 'onizlemeAcKapa'], '-', ['Python karşılığını göster', '', 'ceviriPython'], ['JavaScript karşılığını göster', '', 'ceviriJs'], '-', ['Yazıyı büyüt', 'Ctrl+=', 'yaziBuyut'], ['Yazıyı küçült', 'Ctrl+-', 'yaziKucult']] },
    { ad: 'Çalıştır', ogeler: [['Çalıştır', 'F5', 'calistir'], ['Hata ayıkla', 'F6', 'ayikla'], ['Adım adım göster', '', 'yavasCalistir'], ['Durdur', '⇧+F5', 'durdur'], ['Denetle', 'F7', 'denetleKomut'], '-', ['Kesme noktası ekle/kaldır', 'F9', 'kesmeImlec'], ['Devam', 'F5', 'ayDevam'], ['Üstünden adım', 'F10', 'ayUstunden'], ['İçine adım', 'F11', 'ayAdim'], ['Dışına adım', '⇧+F11', 'ayCik'], '-', ['Canlı önizlemeyi göster/gizle', '', 'onizlemeAcKapa'], ['Önizlemeyi tarayıcıda aç', '', 'onizlemeTarayici'], '-', ['Linux için derle', '', 'derleLinux'], ['Windows için derle', '', 'derleWindows'], ['Web için derle (WebAssembly)', '', 'derleWeb'], '-', ['Masaüstü uygulaması (Linux)', '', 'paketleLinux'], ['Masaüstü uygulaması (Windows)', '', 'paketleWindows'], ['Telefon uygulaması (Android)', '', 'paketleAndroid'], ['Telefon uygulaması (iPhone)', '', 'paketleIos']] },
    { ad: 'Terminal', ogeler: [['Terminali temizle', '', 'terminalTemizle'], ['Sorunları göster', '', 'altSorunlar'], ['Çıktıyı göster', '', 'altCikti']] },
    { ad: 'Yardım', ogeler: [["Orhunca'yı öğren", '', 'ogrenAc'], ['Klavye kısayolları', '', 'kisayollarModal'], ['Sürüm notları', '', 'guncellemeModal'], '-', ['Hata bildir…', '', 'hataBildirModal'], ['Hakkında', '', 'hakkindaModal']] },
  ];

  function cizAcilir(m) {
    return `<div class="acilir-menu">${m.ogeler.map(o => o === '-' ? '<div class="acilir-ayrac"></div>'
      : `<div class="acilir-oge" data-e="${o[2]}"><span>${kac(o[0])}</span>${o[1] ? `<span class="kisayol">${kac(o[1])}</span>` : ''}</div>`).join('')}</div>`;
  }

  // =====================================================================
  // Ortak parçalar
  // =====================================================================
  function cizBaslik() {
    const duz = D.ekran === 'duzenleyici' && D.proje;
    // Masaüstü uygulamasında (Tauri) başlık çubuğunun boş yerleri pencereyi taşır.
    const tasi = TAURI ? ' data-tauri-drag-region' : '';
    return `<div class="baslik-cubugu"${tasi}>
      <div class="logo"${tasi}><div class="logo-kutu"${tasi}><img src="simge.svg" alt=""${tasi}></div><span class="logo-ad"${tasi}>Orhunca</span></div>
      ${duz ? `<div class="menuler">${MENULER.map(m => `<span class="menu-baslik ${D.menu === m.ad ? 'acik' : ''}" data-e="menuAc" data-a="${m.ad}">${m.ad}${D.menu === m.ad ? cizAcilir(m) : ''}</span>`).join('')}</div>
      <div class="pencere-adi"${tasi}>${kac(D.proje.ad)} — Orhunca</div>` : ''}
      <div style="flex:1;align-self:stretch"${tasi}></div>
      ${duz && asistanAcik() ? `<div class="baslik-asistan ${D.asistanPaneliAcik ? 'acik' : ''}" data-e="asistanAcKapa" title="Yapay zekâ asistanı (Ctrl+I)">${S('auto_awesome')}<span>Asistan</span></div>` : ''}
      ${TAURI ? `<div class="pencere-dugmeleri"><span class="simge" data-e="pencereKucult">remove</span><span class="simge" style="font-size:16px" data-e="pencereBuyut">crop_square</span><span class="simge kapat" data-e="pencereKapat">close</span></div>` : ''}
    </div>`;
  }

  function cizYanMenu() {
    const etkin = D.ekran === 'yapilandir' ? 'yeni' : D.ekran;
    const ogeler = [['baslangic', 'Başlangıç', 'home'], ['yeni', 'Şablonlar', 'grid_view'], ['ogren', 'Öğren', 'menu_book']];
    return `<div class="yan-menu">
      ${ogeler.map(([id, ad, simge]) => `<div class="yan-oge ${etkin === id ? 'etkin' : ''}" data-e="git" data-a="${id}"><div class="cubuk"></div>${S(simge)}<span>${ad}</span></div>`).join('')}
      <div style="flex:1"></div>
      <div class="kullanici">
        <div class="avatar">${kac(kullaniciAdi().charAt(0))}</div>
        <div class="esnek"><div class="kullanici-ad">${kac(kullaniciAdi())}</div><div class="kullanici-alt">${kac(surumAdi())} · Topluluk</div></div>
        ${S('settings', 'ayar-simge').replace('<span ', '<span data-e="ayarlarModal" title="Ayarlar" ')}
      </div>
    </div>`;
  }

