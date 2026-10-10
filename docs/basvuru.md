# Yerleşik işlevler başvurusu

<!-- Bu dosya üretilir: cargo run -- başvuru --md > docs/basvuru.md -->

Orhunca'nın her programda hazır bulunan işlevleri. Terminalde aramak için
`orhunca başvuru <kelime>` (ör. `orhunca başvuru tarih`); Stüdyo'da **Öğren** sayfasında
ve kodda işlevin üzerine gelince de görünür. Dilin kendisi (değişkenler, koşullar,
döngüler, fiiller, modeller) için: [dil rehberi](dil-rehberi.md).

**Bölümler:** Dönüşümler · Metin · Liste ve sözlük · Dosya · Matematik · Zaman ve sistem · Desenler (düzenli ifadeler) · CSV · Program · Web · Telefon · Arayüz · Oyun · JavaScript · Sınama

## Dönüşümler

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `uzunluk` | `uzunluk(liste \| metin \| sözlük) → sayı` | Öğe ya da karakter sayısı. |
| `metin` | `metin(değer) → metin` | Herhangi bir değeri metne çevirir. |
| `sayı` | `sayı(metin \| ondalık) → sayı` | Metni sayıya çevirir; ondalığın kesirli kısmını atar. |
| `ondalık` | `ondalık(metin \| sayı) → ondalık` | "3,5" gibi virgüllü yazımı da okur. |
| `yuvarla` | `yuvarla(x) → sayı · yuvarla(x, basamak) → ondalık` | Yarımlar sıfırdan uzağa yuvarlanır. |
| `sayı_mı` | `sayı_mı(metin) → mantık` | Metin bir tamsayı mı? |
| `ondalık_mı` | `ondalık_mı(metin) → mantık` | Metin bir ondalık sayı mı? |

## Metin

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `büyük_harf` | `büyük_harf(metin) → metin` | Türkçe kurallarla büyük harfe çevirir (i → İ). |
| `küçük_harf` | `küçük_harf(metin) → metin` | Türkçe kurallarla küçük harfe çevirir (I → ı). |
| `kırp` | `kırp(metin) → metin` | Baştaki ve sondaki boşlukları siler. |
| `parça` | `parça(metin \| liste, baş, uzunluk)` | Baştan itibaren verilen uzunlukta parça. |
| `böl` | `böl(metin, ayraç) → liste<metin>` | Metni ayraçtan böler; ayraç "" ise boşluklardan. |
| `birleştir` | `birleştir(liste<metin>, ayraç) → metin` | Liste öğelerini aralarına ayraç koyarak birleştirir. |
| `içerir` | `içerir(metin \| liste \| sözlük, aranan) → mantık` | Aranan içinde geçiyor mu? Sözlükte anahtara bakar. |
| `bul` | `bul(metin \| liste, aranan) → sayı` | İlk geçtiği yerin sırası; yoksa -1. |
| `değiştir` | `değiştir(metin, eski, yeni) → metin` | Tüm geçişleri değiştirir. |
| `başlar` | `başlar(metin, ön) → mantık` | Metin bu önle mi başlıyor? |
| `biter` | `biter(metin, son) → mantık` | Metin bu sonla mı bitiyor? |
| `tekrarla` | `tekrarla(metin, kaç) → metin` | Metni art arda tekrarlar. |
| `harfler` | `harfler(metin) → liste<metin>` | Metnin karakterleri. |
| `kodlar` | `kodlar(metin) → liste<sayı>` | Karakterlerin Unicode kodları: kodlar("aç") = [97, 231]. |
| `kodlardan` | `kodlardan(liste<sayı>) → metin` | Unicode kodlarından metin: kodlardan([97, 231]) = "aç". |
| `kod` | `kod(metin) → sayı` | İlk karakterin Unicode kodu: kod("A") = 65, kod("ç") = 231 (boş metinde 0). |
| `karakter` | `karakter(sayı) → metin` | Unicode kodundan karakter: karakter(231) = "ç". |
| `satırlar` | `satırlar(metin) → liste<metin>` | Metni satırlarına ayırır. |

## Liste ve sözlük

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `sil` | `sil(liste, sıra) → öğe · sil(sözlük, anahtar)` | Öğeyi siler; listede silinen öğeyi döndürür. |
| `ters` | `ters(liste \| metin)` | Ters çevrilmiş kopya. |
| `karıştır` | `karıştır(liste)` | Listeyi rastgele karıştırır. |
| `kopya` | `kopya(liste) → liste` | Listenin bağımsız bir kopyası. |
| `en_büyük` | `en_büyük(liste) · en_büyük(a, b)` | En büyük değer. |
| `en_küçük` | `en_küçük(liste) · en_küçük(a, b)` | En küçük değer. |
| `toplam` | `toplam(liste) → sayı \| ondalık` | Sayıların toplamı. |
| `anahtarlar` | `anahtarlar(sözlük) → liste` | Sözlüğün anahtarları (ekleme sırasıyla). |
| `değerler` | `değerler(sözlük) → liste` | Sözlüğün değerleri (ekleme sırasıyla). |
| `değer` | `değer(sözlük, anahtar, varsayılan) → değer` | Anahtarın değeri; anahtar yoksa varsayılan: değer(istek.sorgu, "q", ""). |

## Dosya

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `dosya_oku` | `dosya_oku(yol) → metin` | Dosyanın tüm içeriği. |
| `dosyaya_yaz` | `dosyaya_yaz(yol, metin)` | Dosyayı baştan yazar. |
| `dosyaya_ekle` | `dosyaya_ekle(yol, metin)` | Dosyanın sonuna ekler. |
| `dosya_var` | `dosya_var(yol) → mantık` | Dosya var mı? |
| `dosya_sil` | `dosya_sil(yol) → mantık` | Dosyayı siler; silindiyse doğru. |
| `dosya_taşı` | `dosya_taşı(eski, yeni) → mantık` | Dosyayı taşır ya da adını değiştirir (ör. yüklenen dosyayı statik/ klasörüne). |
| `sql_sorgu` | `sql_sorgu(sorgu, [değerler]) → liste<sözlük<metin, metin>>` | Veritabanında (ORHUNCA_VERITABANI: SQLite, PostgreSQL, MySQL, SQL Server; ayar yoksa veri/orhunca.sqlite) SELECT çalıştırır; her satır sütun adı → değer sözlüğüdür. `?` yerlerine değerler listesi konur: sql_sorgu("SELECT ad FROM Ürün WHERE fiyat > ?", ["100"]) |
| `sql_çalıştır` | `sql_çalıştır(sorgu, [değerler]) → sayı` | INSERT, UPDATE, DELETE, CREATE gibi SQL deyimlerini çalıştırır; değişen satır sayısını verir. Değerler `?` ile verilir (SQL enjeksiyonuna karşı güvenli). |

## Matematik

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `karekök` | `karekök(x) → ondalık` | Karekök. |
| `üs` | `üs(taban, üs)` | Üs alma; iki sayı için sayı, değilse ondalık. |
| `mutlak` | `mutlak(x)` | Mutlak değer. |
| `bit_ve` | `bit_ve(a, b) → sayı` | Bit bit VE (AND): bit_ve(12, 10) → 8. Bayrak ve maske işlemleri için. |
| `bit_veya` | `bit_veya(a, b) → sayı` | Bit bit VEYA (OR): bit_veya(12, 10) → 14. |
| `bit_xor` | `bit_xor(a, b) → sayı` | Bit bit dışlayıcı VEYA (XOR): bit_xor(12, 10) → 6. Özet (hash) ve sağlama hesaplarında kullanılır. |
| `sola_kaydır` | `sola_kaydır(a, n) → sayı` | Bitleri n basamak sola kaydırır: sola_kaydır(1, 4) → 16. n 0–63 dışındaysa 0. |
| `sağa_kaydır` | `sağa_kaydır(a, n) → sayı` | Bitleri n basamak sağa kaydırır (soldan 0 girer): sağa_kaydır(256, 4) → 16. n 0–63 dışındaysa 0. |
| `sinüs` | `sinüs(radyan) → ondalık` | Sinüs. |
| `kosinüs` | `kosinüs(radyan) → ondalık` | Kosinüs. |
| `tanjant` | `tanjant(radyan) → ondalık` | Tanjant. |
| `logaritma` | `logaritma(x) · logaritma(x, taban) → ondalık` | Doğal ya da verilen tabanda logaritma. |
| `rastgele` | `rastgele() → ondalık · rastgele(a, b) → sayı` | Rastgele sayı; a ve b dahil. |
| `güvenli_anahtar` | `güvenli_anahtar(bayt_sayısı) → metin` | Tahmin edilemeyen rastgele anahtar (onaltılık; 16 bayt → 32 karakter). Oturum, şifre sıfırlama ve API anahtarları için; rastgele() bu işler için güvenli değildir. |

## Zaman ve sistem

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `zaman` | `zaman() → ondalık` | 1970'ten beri geçen saniye (ölçüm için). |
| `tarih` | `tarih() → metin` | Şu anki tarih ve saat: 2026-10-04 14:30:00 |
| `bugün` | `bugün() → metin` | Bugünün tarihi: "2026-10-04" |
| `saat` | `saat() → metin` | Şu anki saat: "14:30:00" |
| `gün_ekle` | `gün_ekle(tarih, gün) → metin` | Tarihe gün ekler (eksi sayı geri gider): gün_ekle("2026-10-04", 30) |
| `gün_farkı` | `gün_farkı(tarih1, tarih2) → sayı` | İki tarih arasındaki gün sayısı. Tarihler 2026-10-04 ya da 04.10.2026 biçiminde. |
| `haftanın_günü` | `haftanın_günü(tarih) → metin` | "Pazartesi", "Salı" ... |
| `tarih_yazısı` | `tarih_yazısı(tarih) → metin` | Okunur biçim: "4 Ekim 2026" |

## Desenler (düzenli ifadeler)

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `eşleşir` | `eşleşir(metin, desen) → mantık` | Metnin tamamı desene uyuyor mu? eşleşir(tel, "0\d{10}"). Desen: . \d \w \s [a-z] [^0-9] * + ? {n,m} ( \| ) ^ $ |
| `desen_bul` | `desen_bul(metin, desen) → metin` | Desene uyan ilk parça; yoksa "". |
| `eşleşmeler` | `eşleşmeler(metin, desen) → liste<metin>` | Desene uyan bütün parçalar: eşleşmeler(yazı, "\d+") |
| `desen_değiştir` | `desen_değiştir(metin, desen, yeni) → metin` | Desene uyan bütün parçaları değiştirir. |
| `desen_böl` | `desen_böl(metin, desen) → liste<metin>` | Metni desene uyan yerlerden böler: desen_böl(m, "[,;]\s*") |

## CSV

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `csv_oku` | `csv_oku(metin) → liste<liste<metin>>` | CSV metnini satırlara ve alanlara ayırır; ayraç (, ; ya da sekme) kendiliğinden anlaşılır. |
| `csv_yaz` | `csv_yaz(liste<liste<metin>>) → metin` | Tabloyu CSV metnine çevirir: dosyaya_yaz("x.csv", csv_yaz(tablo)) |

## Program

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `bekle` | `bekle(saniye)` | Programı verilen süre kadar bekletir. |
| `oku` | `oku() → metin` | Klavyeden bir satır okur. |
| `argümanlar` | `argümanlar() → liste<metin>` | Programa komut satırından verilen değerler. |
| `ortam` | `ortam(ad) → metin` | Ortam değişkeni; yoksa "". |
| `çık` | `çık(kod)` | Programı verilen çıkış koduyla bitirir. |
| `boş_mu` | `boş_mu(nesne) → mantık` | Model değeri boş mu? (kendi modeline dönen alanlar, ör. sonraki: Düğüm, başta boştur) |
| `hata_ver` | `hata_ver(mesaj)` | Bir çalışma hatası oluşturur; 'dene' bloğundaysa 'yakala' bloğu çalışır, değilse program biter. |

## Web

İnternetten veri alma ve web sunucusu yanıtları (`görünüm`, `yanıt`, `yönlendir`...).

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `http_al` | `http_al(adres) → metin` | Bir web adresinin içeriğini indirir: http_al("https://..."). JSON yanıtlarından değer almak için json_al. |
| `http_gönder` | `http_gönder(adres, gövde) → metin` | POST isteği gönderir; gövde { ya da [ ile başlıyorsa JSON olarak. Yanıtın gövdesini döndürür. |
| `http_iste` | `http_iste(yöntem, adres, gövde, başlıklar) → metin` | Her türlü HTTP isteği: http_iste("PUT", adres, gövde, {"Authorization": "Bearer …"}). Yöntem GET, POST, PUT, PATCH ya da DELETE; başlık yoksa {}. |
| `json` | `json(değer) → metin` | Değeri (liste, sözlük, model...) JSON metnine çevirir. |
| `json_al` | `json_al(json, yol) → metin` | JSON metninden değer okur: json_al(yanıt, "hava.sıcaklık"), json_al(m, "liste.0.ad"); yoksa "". |
| `json_uzunluk` | `json_uzunluk(json, yol) → sayı` | JSON dizisinin öğe (nesnenin anahtar) sayısı: json_uzunluk(yanıt, "öğrenciler"); yoksa 0. Öğeleri json_al(yanıt, "öğrenciler.0.ad") ile okuyun. |
| `kaçır` | `kaçır(değer) → metin` | HTML'de güvenle gösterilecek biçime çevirir: < → &lt; |
| `para` | `para(sayı) → metin` | Türkçe para biçimi: 1234.5 → "1.234,50" |
| `sayı_yazısı` | `sayı_yazısı(sayı, basamak) → metin` | Türkçe sayı biçimi: sayı_yazısı(86.333, 2) → "86,33", sayı_yazısı(1234.5, 1) → "1.234,5" |
| `url_kodla` | `url_kodla(metin) → metin` | Adreslerde kullanmak için yüzde kodlar: "çay" → "%C3%A7ay" |
| `görünüm` | `görünüm("ad") · görünüm("ad", değer) → metin` | görünümler/ad.ohchtml dosyasını HTML olarak oluşturur. |
| `yanıt` | `yanıt(durum, gövde) · yanıt(durum, gövde, tür) → Yanıt` | Durum kodlu web yanıtı: yanıt(404, "Bulunamadı") |
| `yönlendir` | `yönlendir(adres) → Yanıt` | Tarayıcıyı başka bir adrese gönderir (303). |
| `json_yanıtı` | `json_yanıtı(değer) · json_yanıtı(değer, durum) → Yanıt` | Değeri JSON olarak gönderen web yanıtı. |
| `sun` | `sun() · sun(kapı)` | Web sunucusunu başlatır (varsayılan kapı 3000). Yol tanımlıysa kendiliğinden çağrılır. |

## Telefon

Android/iOS uygulamasında ve tarayıcıda çalışır; bilgisayar programında etkisizdir.

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `titret` | `titret(milisaniye)` | Telefonu verilen süre kadar titretir: titret(200). |
| `paylaş` | `paylaş(metin)` | Telefonun paylaşma penceresini açar (WhatsApp, e-posta...): paylaş("Puanım: " + puan). |
| `bildirim_gönder` | `bildirim_gönder(başlık, metin)` | Telefonda bildirim gösterir; ilk seferde izin istenir. |

## Arayüz

Arayüz programlarında (tarayıcıda ve masaüstü paketinde) çalışır.

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `tema` | `tema(ad)` | Arayüzün görünümünü seçer: tema("bootstrap"). Bootstrap temasında düğmelere sınıf: "başarı", "tehlike", "dikkat", "çerçeveli"... verilebilir. |

## Oyun

`oyun_alanı(...)` içindeki `her_karede:` bloğunda çizer; bilgisayar programında etkisizdir.

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `temizle` | `temizle(renk)` | Oyun alanını verilen renge boyar: temizle("#101820"). |
| `dikdörtgen` | `dikdörtgen(x, y, genişlik, yükseklik, renk)` | Dolu dikdörtgen çizer. (0, 0) sol üst köşedir. |
| `daire` | `daire(x, y, yarıçap, renk)` | Merkezi (x, y) olan dolu daire çizer. |
| `çizgi` | `çizgi(x1, y1, x2, y2, renk)` | İki nokta arasına çizgi çizer. |
| `yazı_çiz` | `yazı_çiz(metin, x, y, renk) · yazı_çiz(metin, x, y, renk, boyut)` | Oyun alanına yazı yazar (boyut piksel; varsayılan 16). |
| `resim_çiz` | `resim_çiz(adres, x, y, genişlik, yükseklik)` | Resim çizer: "oyuncu.png" (statik dosya) ya da bir internet adresi. |
| `ses` | `ses(frekans, süre)` | Verilen frekansta (Hz) ve sürede (saniye) kısa bir ses çalar: ses(440, 0.1). |
| `tuş_basılı` | `tuş_basılı(tuş) → mantık` | Tuş şu an basılı mı: "sol", "sağ", "yukarı", "aşağı", "boşluk", "enter", "a" ... "z", "0" ... "9". |
| `fare_x` | `fare_x() → sayı` | Farenin (ya da parmağın) oyun alanındaki x konumu. |
| `fare_y` | `fare_y() → sayı` | Farenin (ya da parmağın) oyun alanındaki y konumu. |
| `fare_basılı` | `fare_basılı() → mantık` | Fare düğmesi basılı ya da ekrana dokunuluyor mu? |

## JavaScript

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `js_çalıştır` | `js_çalıştır(kod) → metin` | Web hedefinde JavaScript kodunu çalıştırır, sonucu metin olarak döndürür: js_çalıştır("navigator.language"). Bilgisayar programında boş metin döner. |
| `js_yükle` | `js_yükle(adres)` | Web hedefinde bir JavaScript kütüphanesini sayfaya ekler (yüklenince js_çalıştır ile kullanılır): js_yükle("https://cdn.jsdelivr.net/npm/chart.js") |

## Sınama

Sınama dosyalarında (`*_sına.ohc`) ve programın her yerinde kullanılabilir; ayrıntılar dil rehberinin Sınamalar bölümünde.

| İşlev | Kullanım | Açıklama |
|---|---|---|
| `doğrula` | `doğrula(koşul) · doğrula(koşul, açıklama)` | Koşul yanlışsa satırı ve açıklamayı gösteren bir çalışma hatası verir: doğrula(yaş >= 0, "yaş eksi olamaz"). |
| `eşit_olmalı` | `eşit_olmalı(gerçek, beklenen)` | İki değer farklıysa ikisini de gösteren bir çalışma hatası verir. Her tiple çalışır. Sınamaları çalıştırmak için: orhunca sına. |
