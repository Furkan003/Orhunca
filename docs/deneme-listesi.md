# Gerçek cihazlarda deneme listesi

Otomatik sınamaların yapamadığı denemeler: gerçek Windows, telefon, yapay zekâ hesapları. Her maddede
ne yapacağınız ve ne görmeniz gerektiği yazıyor. Beklenmeyen bir şey görürseniz Stüdyo'da
**Yardım → Hata bildir** ile bildirin.

[← README](../README.md)

## 1. Windows (Stüdyo masaüstü uygulaması)

- [ ] **Kurulum.** `Orhunca-Studyo-Windows-Kurulum.exe` kurulur, Başlat menüsünde "Orhunca Stüdyo" çıkar.
- [ ] **Siyah pencere yok.** Yeni bir konsol projesi oluşturun ve **F5**'e basın. Ayrı bir siyah (cmd)
      penceresi **açılmamalı**; çıktı Stüdyo'nun altındaki terminalde görünmeli.
- [ ] **Kod kendiliğinden başlamıyor.** Yeni proje oluşturunca program siz F5'e basana kadar çalışmamalı.
- [ ] **Girdi.** `ad = oku()` ve `ad'ı yaz.` yazın; F5 ile çalıştırıp terminale bir ad yazın; ad yazılmalı.
- [ ] **Otomatik kaydetme.** Bir şey yazıp 2 saniye bekleyin; sekmedeki "kaydedilmedi" noktası kaybolmalı.
- [ ] **Yerel geçmiş.** **Dosya → Yerel geçmiş…** açın; bir önceki hâli seçip **Bu hâle dön**'e basın.
- [ ] **Kod kaybolmuyor.** Bir şey yazın, başlangıç ekranına dönüp aynı projeyi açın; yazdığınız durmalı.
- [ ] **Türkçe klavye yardımı.** `Alt+S` → `ş`, `Alt+I` → `ı` yazmalı. `eger ` yazınca `eğer `
      olmalı; Ctrl+Z ile `eger`'e dönmeli.
- [ ] **Web projesi.** Yeni "Web Sitesi" projesi, F5 → sağda önizleme açılmalı.
- [ ] **Arayüz uygulaması.** Yeni "Arayüz Uygulaması", F5 → önizlemede düğmeler çalışmalı.
- [ ] **Masaüstü paketi.** **Çalıştır → Masaüstü uygulaması (Windows)** → oluşan `.exe` açılmalı.
- [ ] **Otomatik güncelleme.** Eski bir sürüm kuruluyken Stüdyo yeni sürümü bildirmeli; "Şimdi güncelle"
      sonrası Stüdyo yeni sürümle açılmalı (Yardım → Hakkında).

### 0.11 ile gelenler

- [ ] **Editör.** `(` yazınca `)` kendiliğinden gelmeli; imleç bir parantezin yanındayken eşi
      çerçeveyle işaretlenmeli. Alt+↑/↓ satırı taşımalı, Ctrl+G satıra gitmeli.
- [ ] **Yeniden adlandır.** Bir değişkenin üzerinde F2 → yeni ad; projedeki bütün kullanımları
      değişmeli. ⇧+F12 kullanıldığı yerleri listelemeli.
- [ ] **Sınamalar.** `sınama_hesap.ohc` adlı bir dosyaya `eşit_olmalı(2 + 2, 4)` yazın; Sınamalar
      panelinde çalıştırınca yeşil olmalı, `5` yapınca kırmızı ve anlaşılır bir mesaj vermeli.
- [ ] **Git paneli.** Proje Git ile oluşturulmuşsa bir değişiklik yapın; Git panelinde görünmeli,
      tıklayınca fark açılmalı, mesaj yazıp **İşle** ile kaydedilmeli.
- [ ] **Koşullu kesme.** Bir döngü satırının numarasına sağ tıklayın, koşul `i == 3` yazın; F6 ile
      program yalnızca `i` 3 olunca durmalı.
- [ ] **Yeni dil özellikleri.** `örnekler/model_işlevleri.ohc` ve `örnekler/adsız_işlevler.ohc`
      çalışmalı. Arayüz uygulamasında `arka planda:` örneği (`örnekler/arayüz/arka_plan.ohc`): düğmeye
      basınca önce "Hesaplanıyor…" görünmeli, sonra sonuç.

## 2. Telefon

- [ ] **Android.** Bir arayüz projesinde **Çalıştır → Telefon uygulaması (Android)** → `.apk`'yı telefona
      atıp kurun ("bilinmeyen kaynaklar" izni gerekir). Uygulama açılmalı, düğmeler çalışmalı.
- [ ] **Telefon komutları.** `titret(200)`, `paylaş("deneme")`, `bildirim_gönder("Başlık", "Metin")`
      düğmelere bağlanıp telefonda denenmeli.
- [ ] **Oyun.** 2B Oyun şablonu telefonda açılmalı, dokunma ile oynanabilmeli.
- [ ] **Telefonda kod yazma.** Telefonda `https://furkan003.github.io/Orhunca/dene.html` açın,
      ana ekrana ekleyin. Uçak modunda uygulamayı açın; kod yazıp ▶ ile çalıştırın. Sembol çubuğu
      klavyenin hemen üstünde durmalı; **Programlarım**'da yazdığınız program görünmeli.
- [ ] **iPhone.** `--hedef ios` ile üretilen proje GitHub'a yüklenince imzasız `.ipa` üretilmeli;
      AltStore/Sideloadly ile kurulabilmeli. (Mac varsa Xcode'da ▶ ile.)

## 3. Yapay zekâ

- [ ] **Stüdyo asistanı, bulut.** **Asistan → OpenAI** (ya da Claude/Gemini), anahtarı girin. "Not ortalaması
      programı yaz" deyin. Asistan kodu denetleyip çalıştırmalı ("Kodu çalıştırdı" adımı), **Uygula** ile
      dosyaya yazılmalı.
- [ ] **Kısa rehber kalitesi.** Birkaç farklı istek deneyin (döngü, model, web, arayüz). Asistan gerektiğinde
      "Rehbere baktı" adımı göstermeli; yazdığı kod ilk ya da ikinci denemede derlenmeli. Sık yaptığı
      hataları not edin.
- [ ] **Yerel model.** Ollama kurup `ollama pull qwen2.5-coder:7b`; **Asistan → Ollama → Bağlan**.
- [ ] **Ajan bağlama.** **Asistan → Kendi ajanınızı bağlayın** → Codex CLI ya da Claude Code komutunu
      çalıştırın; ajana "Orhunca ile hesap makinesi yaz ve çalıştır" deyin.
- [ ] **ChatGPT web.** Sohbete `https://furkan003.github.io/Orhunca/llms-full.txt` bağlantısını verip kod
      yazdırın; kodu Stüdyo'da çalıştırın.

## 4. Sunucuya yayınlama (VPS ya da hosting varsa)

- [ ] **orhunca yayınla.** Bir "Tam Yığın Uygulama" projesinde `orhunca yayınla kok@<sunucu-ip>`
      çalıştırın; sonunda `✓ Yayında: http://<ip>:3000` yazmalı ve site tarayıcıda açılmalı.
- [ ] **Alan adı ve HTTPS.** Alan adının A kaydını sunucuya yönlendirip `--alan ornek.com` ile yeniden
      yayınlayın; `https://ornek.com` kilit simgesiyle açılmalı.
- [ ] **Güncelleme verileri silmiyor.** Sitede birkaç kayıt ekleyin, kodda küçük bir değişiklik yapıp
      yeniden yayınlayın; kayıtlar durmalı.

- [ ] **Paylaşımlı hosting (Natro vb.).** `orhunca yayınla --cgi` ile oluşan `cikti/cgi/` klasörünün
      içindekileri `public_html`e yükleyin (`uygulama.cgi` izni 755). Site açılmalı, kayıt eklenebilmeli.
- [ ] **Yalnızca PHP/MySQL olan hosting.** `orhunca yayınla --php` ile oluşan `cikti/php/` içindekileri
      `public_html`e yükleyin. cPanel'de MySQL veritabanı açıp bilgileri `orhunca/ayarlar.php`'ye
      yazın; site açılmalı, kayıt eklenebilmeli, phpMyAdmin'de tabloda görünmeli.
- [ ] **PHP gibi.** `public_html`e `orhunca yayınla --cgi --kaynakla` ile yükleyin; dosya yöneticisinde
      `index.ohc`'yi değiştirip sayfayı yenileyin, değişiklik görünmeli.

## 5. Diğer sistemler (varsa)

- [ ] **Pardus/Ubuntu.** `orhunca-studyo_amd64.deb` kurulur, Stüdyo açılır, F5 çalışır.
- [ ] **macOS.** `.dmg` → ilk açılışta sağ tık → Aç; F5 çalışır.
- [ ] **Raspberry Pi.** `kur.sh` ile komut satırı kurulur, `orhunca çalıştır` çalışır.

## 6. Gerçek kullanıcı denemesi

Listedeki en değerli madde: 3-5 kişiye (bir öğrenci, bir yetişkin, bir öğretmen) Stüdyo'yu verin, hiç
yardım etmeden ilk programlarını yazmalarını isteyin ve nerede takıldıklarını not edin.
