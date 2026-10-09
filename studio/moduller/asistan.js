/* Orhunca Stüdyo — Yapay zekâ asistanı.
 * Dosyalar index.html'deki sırayla yüklenir; en üst düzeydeki tanımlar ortaktır. */
'use strict';
  // =====================================================================
  // Yapay zekâ asistanı
  // =====================================================================
  const asistanAcik = () => D.asistanGoster && D.asistan.durum && !D.asistan.durum.kapali;

  async function asistanDurumuYukle() {
    D.asistan.durum = await api('/api/asistan').catch(() => null);
    if (D.ekran === 'duzenleyici') guncelle('baslik', 'etkinlik', 'asistan');
  }

  /** Asistan yanıtı: basit Markdown ve kod blokları. */
  function mdAsistan(md, mesaj) {
    let blok = 0;
    return md.split(/```/).map((parca, i) => {
      if (i % 2 === 0) return mdBasit(parca);
      const dil = (parca.match(/^[^\n]*/)[0] || '').trim().toLowerCase();
      const kod = parca.replace(/^[^\n]*\n/, '').replace(/\n$/, '');
      const orhunca = !dil || dil === 'orhunca' || dil === 'ohc';
      const no = blok++;
      // Orhunca kod bloğu dosyaya yazılabilir (araç kullanamayan modeller için de).
      const dugmeler = mesaj === undefined ? '' : `<div class="asistan-blok-dugmeleri">${orhunca ? `<span data-e="asistanBlokUygula" data-a="${mesaj}:${no}" title="Bu kodu açık dosyaya yaz">${S('done')}Uygula</span>` : ''}<span data-e="asistanBlokKopyala" data-a="${mesaj}:${no}" title="Kopyala">${S('content_copy')}Kopyala</span></div>`;
      return `<div class="asistan-blok"><pre class="asistan-kod" ${orhunca ? 'data-orhunca' : ''}>${kac(kod)}</pre>${dugmeler}</div>`;
    }).join('');
  }
  /** Asistan iletisindeki n. kod bloğu. */
  const asistanBlogu = (md, n) => (md.split(/```/).filter((_, i) => i % 2)[n] || '').replace(/^[^\n]*\n/, '').replace(/\n$/, '') + '\n';

  /** Sağdaki asistan paneli (Cursor'daki gibi). */
  function cizAsistan() {
    const kap = $('#asistanKap');
    if (!kap) return;
    const acik = asistanAcik() && D.asistanPaneliAcik;
    kap.classList.toggle('gizli', !acik);
    if (!acik) { kap.innerHTML = ''; return; }
    const odak = document.activeElement?.id === 'asistanGirdi';
    const g = ayarOku('asistanGenisligi', 0);
    if (g) kap.style.width = g + 'px';
    kap.innerHTML = `<div class="asistan-tutamac" title="Genişliği değiştirmek için sürükleyin"></div>
      <div class="panel-baslik"><span style="flex:1">${S('auto_awesome', '', 'font-size:16px;color:var(--vurgu);margin-right:6px;vertical-align:-3px')}ASİSTAN</span>
        ${D.asistan.mesajlar.length ? `<span class="simge" title="Yeni konuşma" data-e="asistanYeni" style="margin-right:6px">add_comment</span>` : ''}
        <span class="simge" title="Kapat (Ctrl+I)" data-e="asistanAcKapa">close</span></div>` + asistanPaneli();
    const l = $('#asistanMesajlar'); if (l) l.scrollTop = l.scrollHeight;
    kap.querySelectorAll('pre[data-orhunca]').forEach(p => { p.innerHTML = p.textContent.split('\n').map(x => vurgula(x, 'ohc')).join('\n'); });
    if (odak) $('#asistanGirdi')?.focus();
  }

  /** Seçili (ya da kurulumda düzenlenen) sağlayıcı. */
  const asistanSaglayici = () => {
    const d = D.asistan.durum || {}, l = d.saglayicilar || [];
    return l.find(p => p.kimlik === (D.asistan.secilen || d.saglayici)) || l[0] || {};
  };

  /** Sağlayıcı seçimi, anahtar, adres. */
  function asistanKurulum() {
    const A = D.asistan, d = A.durum || {}, p = asistanSaglayici();
    const cip = x => `<span class="asistan-saglayici ${x.kimlik === p.kimlik ? 'secili' : ''}" data-e="asistanSaglayiciSec" data-a="${x.kimlik}">${kac(x.ad.replace(' (yerel)', ''))}${x.hazir && (x.anahtar_var || x.model) ? S('check') : ''}</span>`;
    const l = d.saglayicilar || [];
    const anahtarAlani = p.anahtar_gerekli || p.anahtar_var || p.kimlik === 'ozel'
      ? `<div class="alan" style="gap:6px"><label>API anahtarı${p.anahtar_gerekli ? '' : ' (isteğe bağlı)'}</label>
          <div class="yan-yana" style="gap:6px"><input id="asistanAnahtar" data-g="asistanAnahtar" type="password" class="metin-girdi" style="flex:1" placeholder="${p.anahtar_var ? 'Kayıtlı: …' + kac(p.anahtar_sonu || '') + ' (değiştirmek için yazın)' : 'Anahtarı yapıştırın'}" value="${kac(A.anahtar)}" autocomplete="off" spellcheck="false">
          ${p.anahtar_var ? `<span class="kare-dugme simge" title="Anahtarı bu bilgisayardan sil" data-e="asistanAnahtarSil" style="width:32px;height:32px;font-size:18px">key_off</span>` : ''}</div></div>` : '';
    const adresAlani = p.yerel
      ? `<div class="alan" style="gap:6px"><label>Sunucu adresi</label><input data-g="asistanAdres" class="metin-girdi" placeholder="${kac(p.varsayilan_adres)}" value="${kac(A.adres ?? p.adres ?? '')}" spellcheck="false"></div>` : '';
    return `<div class="panel-ic asistan-kurulum">
      ${d.hazir ? '' : `<div class="asistan-tanitim">${S('auto_awesome')}<div><b>Kodlama asistanı</b><br>Sorularınızı yanıtlar, kod yazar; yazdığı kodu kendisi denetleyip çalıştırır. Dosyanızdaki değişiklikleri siz onaylarsınız.</div></div>`}
      <div class="asistan-baslik2">Bulut</div>
      <div class="asistan-saglayicilar">${l.filter(x => !x.yerel).map(cip).join('')}</div>
      <div class="asistan-baslik2">Bu bilgisayarda <span>ücretsiz, internetsiz</span></div>
      <div class="asistan-saglayicilar">${l.filter(x => x.yerel).map(cip).join('')}</div>
      <div class="panel-not asistan-ipucu">${kac(p.ipucu || '')}${p.sayfa ? ` <a data-e="disAdresAc" data-a="${kac(p.sayfa)}">${p.yerel ? 'İndir' : 'Anahtar al'} ${S('open_in_new')}</a>` : ''}</div>
      ${adresAlani}${anahtarAlani}
      <div class="yan-yana" style="gap:6px">
        <div class="panel-dugme birincil" style="flex:1" data-e="asistanBaglan">${A.baglaniyor ? '<div class="donen kucuk"></div>' : S(p.yerel ? 'lan' : 'key')}${p.yerel ? 'Bağlan' : 'Kaydet ve bağlan'}</div>
        ${d.hazir && A.ayarAcik ? `<div class="panel-dugme" data-e="asistanAyarKapat">Vazgeç</div>` : ''}</div>
      ${A.hata ? `<div class="panel-not asistan-hata">${kac(A.hata)}</div>` : ''}
      <div class="panel-not" style="font-size:12px">Anahtarlar yalnızca bu bilgisayarda saklanır, arayüze bile geri gönderilmez. Bulut sağlayıcılarında kullanım ücreti kendi hesabınıza yansır; yerel modeller ücretsizdir ve kodunuz bilgisayarınızdan çıkmaz.</div>
      <div class="asistan-ajan" data-e="ajanBagla">${S('hub')}<div><b>Kendi ajanınızı bağlayın</b><br>Claude Code, Cursor, VS Code, Codex, Gemini CLI, Cline, Windsurf, Zed…</div></div>
    </div>`;
  }

  const ADIMLAR = { kodu_calistir: ['play_arrow', 'Kodu çalıştırdı'], kodu_denetle: ['task_alt', 'Kodu denetledi'], arayuzu_dene: ['select_window', 'Arayüzü denedi'], rehber_oku: ['menu_book', 'Rehbere baktı'], dosyayi_degistir: ['edit_document', 'Değişiklik önerdi'] };

  function asistanPaneli() {
    const A = D.asistan, d = A.durum || {};
    if (!d.hazir || A.ayarAcik) return asistanKurulum();
    const modeller = A.modeller || (d.model ? [{ kimlik: d.model, ad: d.model }] : []);
    const listede = modeller.some(m => m.kimlik === d.model);
    const secim = `<div class="yan-yana" style="gap:6px"><span class="asistan-rozet" data-e="asistanAyarAc" title="Sağlayıcıyı değiştir">${S(d.yerel ? 'computer' : 'cloud')}${kac((d.saglayici_adi || '').replace(/ \(.*\)$/, ''))}</span><select class="metin-girdi" data-g="asistanModel" style="flex:1;min-width:0" title="Model">
        ${d.model ? '' : '<option value="">Model seçin…</option>'}
        ${d.model && !listede ? `<option value="${kac(d.model)}" selected>${kac(d.model)}</option>` : ''}
        ${modeller.map(m => `<option value="${kac(m.kimlik)}" ${m.kimlik === d.model ? 'selected' : ''}>${kac(m.ad)}</option>`).join('')}
        <option value="__elle__">Başka bir model adı yaz…</option>
      </select><span class="kare-dugme simge" title="Model listesini yenile" data-e="asistanModelleriYukle" style="width:32px;height:32px;font-size:18px">refresh</span><span class="kare-dugme simge" title="Sağlayıcı ve anahtar ayarları" data-e="asistanAyarAc" style="width:32px;height:32px;font-size:18px">settings</span></div>`;
    const mesajlar = A.mesajlar.map((m, i) => m.rol === 'kullanici'
      ? `<div class="asistan-mesaj kullanici">${kac(m.metin)}</div>`
      : `<div class="asistan-mesaj">
          ${(m.adimlar || []).map(a => { const [simge, ad] = ADIMLAR[a.ad] || ADIMLAR.dosyayi_degistir; return `<div class="asistan-adim">${S(simge)}${ad}</div>`; }).join('')}
          <div class="asistan-metin">${mdAsistan(m.metin || '', i)}</div>
          ${m.aracsiz ? `<div class="asistan-adim" title="Araç kullanabilen bir model seçerseniz asistan kodu kendisi denetler ve çalıştırır.">${S('info')}Bu model araç kullanamıyor; kodu kendisi denetleyemedi.</div>` : ''}
          ${m.oneri ? `<div class="asistan-oneri"><div class="asistan-oneri-baslik">${S('edit_document')}<span class="esnek">${kac(m.oneri.aciklama || 'Dosya için öneri')}</span></div>
            <pre class="asistan-kod" data-orhunca>${kac(m.oneri.icerik)}</pre>
            <div class="yan-yana" style="gap:6px">${m.uygulandi ? `<span class="panel-not">${S('check')} Uygulandı</span>` : `<div class="dugme birincil kucuk" data-e="asistanUygula" data-a="${i}">${S('done')}Uygula</div>`}<div class="dugme kucuk" data-e="asistanKopyala" data-a="${i}">${S('content_copy')}Kopyala</div></div></div>` : ''}
        </div>`).join('');
    return `<div class="panel-ic asistan-panel">${secim}
      <div class="asistan-mesajlar" id="asistanMesajlar">${mesajlar || `<div class="panel-not">Açık dosyanız soruyla birlikte gönderilir.</div>
        <div class="asistan-oneriler">${(D.sorunlar.length ? ['Bu hatayı açıkla ve düzelt'] : []).concat(['Bu kodu adım adım açıkla', 'Koda yorum satırları ekle', 'Bana bu konuda bir alıştırma ver']).map(o => `<div class="asistan-hazir" data-e="asistanHazir" data-a="${kac(o)}">${S('auto_awesome')}${kac(o)}</div>`).join('')}</div>`}
        ${A.bekliyor ? `<div class="asistan-mesaj"><div class="donen kucuk"></div> ${d.yerel ? 'Model çalışıyor… (yerel modeller yavaş olabilir)' : 'Düşünüyor…'}</div>` : ''}
        ${A.hata ? `<div class="panel-not asistan-hata">${kac(A.hata)}</div>` : ''}</div>
      <div class="asistan-girdi"><textarea id="asistanGirdi" data-g="asistanGirdi" class="metin-girdi" rows="3" placeholder="Bir şey sorun ya da isteyin… (Enter: gönder, Shift+Enter: yeni satır)" ${A.bekliyor ? 'disabled' : ''}>${kac(A.girdi)}</textarea>
        ${A.bekliyor ? `<span class="kare-dugme simge" title="Durdur" data-e="asistanDurdur">stop_circle</span>` : `<span class="kare-dugme simge birincil" title="Gönder" data-e="asistanGonder">send</span>`}</div>
    </div>`;
  }

  /** Asistanın önerdiği içeriği, iletinin ait olduğu dosyaya yazar (geri alınabilir). */
  async function asistanYaz(m, icerik) {
    if (m.dosya && m.dosya !== D.etkin) await dosyaAc(m.dosya);
    const ta = $('#kodAlani');
    if (!ta) { bildir('Önce bir dosya açın.', true); return false; }
    ta.focus(); ta.select();
    metinEkle(ta, icerik);
    imleciGuncelle();
    bildir('Değişiklik uygulandı (Ctrl+Z ile geri alabilirsiniz).');
    return true;
  }
