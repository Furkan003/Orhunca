//! Yerleşik işlevlerin listesi: ad, kullanım biçimi ve açıklama.
//! Denetçinin hata mesajları ve dil sunucusunun tamamlama/açıklamaları buradan beslenir.

#[allow(dead_code)] // açıklama: dil sunucusu ve Stüdyo kullanır
pub struct Yerlesik {
    pub ad: &'static str,
    pub kullanim: &'static str,
    pub aciklama: &'static str,
}

macro_rules! y {
    ($ad:expr, $k:expr, $a:expr) => {
        Yerlesik {
            ad: $ad,
            kullanim: $k,
            aciklama: $a,
        }
    };
}

pub const YERLESIKLER: &[Yerlesik] = &[
    // Dönüşümler
    y!(
        "uzunluk",
        "uzunluk(liste | metin | sözlük) → sayı",
        "Öğe ya da karakter sayısı."
    ),
    y!(
        "metin",
        "metin(değer) → metin",
        "Herhangi bir değeri metne çevirir."
    ),
    y!(
        "sayı",
        "sayı(metin | ondalık) → sayı",
        "Metni sayıya çevirir; ondalığın kesirli kısmını atar."
    ),
    y!(
        "ondalık",
        "ondalık(metin | sayı) → ondalık",
        "\"3,5\" gibi virgüllü yazımı da okur."
    ),
    y!(
        "yuvarla",
        "yuvarla(x) → sayı · yuvarla(x, basamak) → ondalık",
        "Yarımlar sıfırdan uzağa yuvarlanır."
    ),
    y!(
        "sayı_mı",
        "sayı_mı(metin) → mantık",
        "Metin bir tamsayı mı?"
    ),
    y!(
        "ondalık_mı",
        "ondalık_mı(metin) → mantık",
        "Metin bir ondalık sayı mı?"
    ),
    // Metin
    y!(
        "büyük_harf",
        "büyük_harf(metin) → metin",
        "Türkçe kurallarla büyük harfe çevirir (i → İ)."
    ),
    y!(
        "küçük_harf",
        "küçük_harf(metin) → metin",
        "Türkçe kurallarla küçük harfe çevirir (I → ı)."
    ),
    y!(
        "kırp",
        "kırp(metin) → metin",
        "Baştaki ve sondaki boşlukları siler."
    ),
    y!(
        "parça",
        "parça(metin | liste, baş, uzunluk)",
        "Baştan itibaren verilen uzunlukta parça."
    ),
    y!(
        "böl",
        "böl(metin, ayraç) → liste<metin>",
        "Metni ayraçtan böler; ayraç \"\" ise boşluklardan."
    ),
    y!(
        "birleştir",
        "birleştir(liste<metin>, ayraç) → metin",
        "Liste öğelerini aralarına ayraç koyarak birleştirir."
    ),
    y!(
        "içerir",
        "içerir(metin | liste | sözlük, aranan) → mantık",
        "Aranan içinde geçiyor mu? Sözlükte anahtara bakar."
    ),
    y!(
        "bul",
        "bul(metin | liste, aranan) → sayı",
        "İlk geçtiği yerin sırası; yoksa -1."
    ),
    y!(
        "değiştir",
        "değiştir(metin, eski, yeni) → metin",
        "Tüm geçişleri değiştirir."
    ),
    y!(
        "başlar",
        "başlar(metin, ön) → mantık",
        "Metin bu önle mi başlıyor?"
    ),
    y!(
        "biter",
        "biter(metin, son) → mantık",
        "Metin bu sonla mı bitiyor?"
    ),
    y!(
        "tekrarla",
        "tekrarla(metin, kaç) → metin",
        "Metni art arda tekrarlar."
    ),
    y!(
        "harfler",
        "harfler(metin) → liste<metin>",
        "Metnin karakterleri."
    ),
    y!(
        "kodlar",
        "kodlar(metin) → liste<sayı>",
        "Karakterlerin Unicode kodları: kodlar(\"aç\") = [97, 231]."
    ),
    y!(
        "kodlardan",
        "kodlardan(liste<sayı>) → metin",
        "Unicode kodlarından metin: kodlardan([97, 231]) = \"aç\"."
    ),
    y!(
        "kod",
        "kod(metin) → sayı",
        "İlk karakterin Unicode kodu: kod(\"A\") = 65, kod(\"ç\") = 231 (boş metinde 0)."
    ),
    y!(
        "karakter",
        "karakter(sayı) → metin",
        "Unicode kodundan karakter: karakter(231) = \"ç\"."
    ),
    y!(
        "satırlar",
        "satırlar(metin) → liste<metin>",
        "Metni satırlarına ayırır."
    ),
    // Liste ve sözlük
    y!(
        "sil",
        "sil(liste, sıra) → öğe · sil(sözlük, anahtar)",
        "Öğeyi siler; listede silinen öğeyi döndürür."
    ),
    y!("ters", "ters(liste | metin)", "Ters çevrilmiş kopya."),
    y!(
        "karıştır",
        "karıştır(liste)",
        "Listeyi rastgele karıştırır."
    ),
    y!(
        "kopya",
        "kopya(liste) → liste",
        "Listenin bağımsız bir kopyası."
    ),
    y!(
        "en_büyük",
        "en_büyük(liste) · en_büyük(a, b)",
        "En büyük değer."
    ),
    y!(
        "en_küçük",
        "en_küçük(liste) · en_küçük(a, b)",
        "En küçük değer."
    ),
    y!(
        "toplam",
        "toplam(liste) → sayı | ondalık",
        "Sayıların toplamı."
    ),
    y!(
        "anahtarlar",
        "anahtarlar(sözlük) → liste",
        "Sözlüğün anahtarları (ekleme sırasıyla)."
    ),
    y!(
        "değerler",
        "değerler(sözlük) → liste",
        "Sözlüğün değerleri (ekleme sırasıyla)."
    ),
    // Dosya
    y!(
        "dosya_oku",
        "dosya_oku(yol) → metin",
        "Dosyanın tüm içeriği."
    ),
    y!(
        "dosyaya_yaz",
        "dosyaya_yaz(yol, metin)",
        "Dosyayı baştan yazar."
    ),
    y!(
        "dosyaya_ekle",
        "dosyaya_ekle(yol, metin)",
        "Dosyanın sonuna ekler."
    ),
    y!("dosya_var", "dosya_var(yol) → mantık", "Dosya var mı?"),
    y!(
        "dosya_sil",
        "dosya_sil(yol) → mantık",
        "Dosyayı siler; silindiyse doğru."
    ),
    y!(
        "dosya_taşı",
        "dosya_taşı(eski, yeni) → mantık",
        "Dosyayı taşır ya da adını değiştirir (ör. yüklenen dosyayı statik/ klasörüne)."
    ),
    // Matematik
    y!("karekök", "karekök(x) → ondalık", "Karekök."),
    y!(
        "üs",
        "üs(taban, üs)",
        "Üs alma; iki sayı için sayı, değilse ondalık."
    ),
    y!("mutlak", "mutlak(x)", "Mutlak değer."),
    y!("sinüs", "sinüs(radyan) → ondalık", "Sinüs."),
    y!("kosinüs", "kosinüs(radyan) → ondalık", "Kosinüs."),
    y!("tanjant", "tanjant(radyan) → ondalık", "Tanjant."),
    y!(
        "logaritma",
        "logaritma(x) · logaritma(x, taban) → ondalık",
        "Doğal ya da verilen tabanda logaritma."
    ),
    y!(
        "rastgele",
        "rastgele() → ondalık · rastgele(a, b) → sayı",
        "Rastgele sayı; a ve b dahil."
    ),
    // Zaman ve sistem
    y!(
        "zaman",
        "zaman() → ondalık",
        "1970'ten beri geçen saniye (ölçüm için)."
    ),
    y!(
        "tarih",
        "tarih() → metin",
        "Şu anki tarih ve saat: 2026-10-04 14:30:00"
    ),
    y!(
        "bugün",
        "bugün() → metin",
        "Bugünün tarihi: \"2026-10-04\""
    ),
    y!("saat", "saat() → metin", "Şu anki saat: \"14:30:00\""),
    y!(
        "gün_ekle",
        "gün_ekle(tarih, gün) → metin",
        "Tarihe gün ekler (eksi sayı geri gider): gün_ekle(\"2026-10-04\", 30)"
    ),
    y!(
        "gün_farkı",
        "gün_farkı(tarih1, tarih2) → sayı",
        "İki tarih arasındaki gün sayısı. Tarihler 2026-10-04 ya da 04.10.2026 biçiminde."
    ),
    y!(
        "haftanın_günü",
        "haftanın_günü(tarih) → metin",
        "\"Pazartesi\", \"Salı\" ..."
    ),
    y!(
        "tarih_yazısı",
        "tarih_yazısı(tarih) → metin",
        "Okunur biçim: \"4 Ekim 2026\""
    ),
    // Desenler (düzenli ifadeler)
    y!(
        "eşleşir",
        "eşleşir(metin, desen) → mantık",
        "Metnin tamamı desene uyuyor mu? eşleşir(tel, \"0\\d{10}\"). Desen: . \\d \\w \\s [a-z] [^0-9] * + ? {n,m} ( | ) ^ $"
    ),
    y!(
        "desen_bul",
        "desen_bul(metin, desen) → metin",
        "Desene uyan ilk parça; yoksa \"\"."
    ),
    y!(
        "eşleşmeler",
        "eşleşmeler(metin, desen) → liste<metin>",
        "Desene uyan bütün parçalar: eşleşmeler(yazı, \"\\d+\")"
    ),
    y!(
        "desen_değiştir",
        "desen_değiştir(metin, desen, yeni) → metin",
        "Desene uyan bütün parçaları değiştirir."
    ),
    y!(
        "desen_böl",
        "desen_böl(metin, desen) → liste<metin>",
        "Metni desene uyan yerlerden böler: desen_böl(m, \"[,;]\\s*\")"
    ),
    // CSV
    y!(
        "csv_oku",
        "csv_oku(metin) → liste<liste<metin>>",
        "CSV metnini satırlara ve alanlara ayırır; ayraç (, ; ya da sekme) kendiliğinden anlaşılır."
    ),
    y!(
        "csv_yaz",
        "csv_yaz(liste<liste<metin>>) → metin",
        "Tabloyu CSV metnine çevirir: dosyaya_yaz(\"x.csv\", csv_yaz(tablo))"
    ),
    y!(
        "bekle",
        "bekle(saniye)",
        "Programı verilen süre kadar bekletir."
    ),
    y!("oku", "oku() → metin", "Klavyeden bir satır okur."),
    y!(
        "argümanlar",
        "argümanlar() → liste<metin>",
        "Programa komut satırından verilen değerler."
    ),
    y!("ortam", "ortam(ad) → metin", "Ortam değişkeni; yoksa \"\"."),
    y!("çık", "çık(kod)", "Programı verilen çıkış koduyla bitirir."),
    y!(
        "boş_mu",
        "boş_mu(nesne) → mantık",
        "Model değeri boş mu? (kendi modeline dönen alanlar, ör. sonraki: Düğüm, başta boştur)"
    ),
    y!(
        "hata_ver",
        "hata_ver(mesaj)",
        "Bir çalışma hatası oluşturur; 'dene' bloğundaysa 'yakala' bloğu çalışır, değilse program biter."
    ),
    // Web
    y!(
        "http_al",
        "http_al(adres) → metin",
        "Bir web adresinin içeriğini indirir: http_al(\"https://...\"). JSON yanıtlarından değer almak için json_al."
    ),
    y!(
        "http_gönder",
        "http_gönder(adres, gövde) → metin",
        "POST isteği gönderir; gövde { ya da [ ile başlıyorsa JSON olarak. Yanıtın gövdesini döndürür."
    ),
    y!(
        "json",
        "json(değer) → metin",
        "Değeri (liste, sözlük, model...) JSON metnine çevirir."
    ),
    y!(
        "json_al",
        "json_al(json, yol) → metin",
        "JSON metninden değer okur: json_al(yanıt, \"hava.sıcaklık\"), json_al(m, \"liste.0.ad\"); yoksa \"\"."
    ),
    y!(
        "kaçır",
        "kaçır(değer) → metin",
        "HTML'de güvenle gösterilecek biçime çevirir: < → &lt;"
    ),
    y!(
        "para",
        "para(sayı) → metin",
        "Türkçe para biçimi: 1234.5 → \"1.234,50\""
    ),
    y!(
        "url_kodla",
        "url_kodla(metin) → metin",
        "Adreslerde kullanmak için yüzde kodlar: \"çay\" → \"%C3%A7ay\""
    ),
    y!(
        "görünüm",
        "görünüm(\"ad\") · görünüm(\"ad\", değer) → metin",
        "görünümler/ad.ohchtml dosyasını HTML olarak oluşturur."
    ),
    y!(
        "yanıt",
        "yanıt(durum, gövde) · yanıt(durum, gövde, tür) → Yanıt",
        "Durum kodlu web yanıtı: yanıt(404, \"Bulunamadı\")"
    ),
    y!(
        "yönlendir",
        "yönlendir(adres) → Yanıt",
        "Tarayıcıyı başka bir adrese gönderir (303)."
    ),
    y!(
        "json_yanıtı",
        "json_yanıtı(değer) · json_yanıtı(değer, durum) → Yanıt",
        "Değeri JSON olarak gönderen web yanıtı."
    ),
    y!(
        "sun",
        "sun() · sun(kapı)",
        "Web sunucusunu başlatır (varsayılan kapı 3000). Yol tanımlıysa kendiliğinden çağrılır."
    ),
    // Telefon (Android/iOS uygulamasında ve tarayıcıda; bilgisayar programında etkisizdir)
    y!(
        "titret",
        "titret(milisaniye)",
        "Telefonu verilen süre kadar titretir: titret(200)."
    ),
    y!(
        "paylaş",
        "paylaş(metin)",
        "Telefonun paylaşma penceresini açar (WhatsApp, e-posta...): paylaş(\"Puanım: \" + puan)."
    ),
    y!(
        "bildirim_gönder",
        "bildirim_gönder(başlık, metin)",
        "Telefonda bildirim gösterir; ilk seferde izin istenir."
    ),
    // Arayüz (tarayıcıda; bilgisayar programında etkisizdir)
    y!(
        "tema",
        "tema(ad)",
        "Arayüzün görünümünü seçer: tema(\"bootstrap\"). Bootstrap temasında düğmelere sınıf: \"başarı\", \"tehlike\", \"dikkat\", \"çerçeveli\"... verilebilir."
    ),
    // Oyun (oyun_alanı(...) her_karede: bloğunda çizer; bilgisayar programında etkisizdir)
    y!(
        "temizle",
        "temizle(renk)",
        "Oyun alanını verilen renge boyar: temizle(\"#101820\")."
    ),
    y!(
        "dikdörtgen",
        "dikdörtgen(x, y, genişlik, yükseklik, renk)",
        "Dolu dikdörtgen çizer. (0, 0) sol üst köşedir."
    ),
    y!(
        "daire",
        "daire(x, y, yarıçap, renk)",
        "Merkezi (x, y) olan dolu daire çizer."
    ),
    y!(
        "çizgi",
        "çizgi(x1, y1, x2, y2, renk)",
        "İki nokta arasına çizgi çizer."
    ),
    y!(
        "yazı_çiz",
        "yazı_çiz(metin, x, y, renk) · yazı_çiz(metin, x, y, renk, boyut)",
        "Oyun alanına yazı yazar (boyut piksel; varsayılan 16)."
    ),
    y!(
        "resim_çiz",
        "resim_çiz(adres, x, y, genişlik, yükseklik)",
        "Resim çizer: \"oyuncu.png\" (statik dosya) ya da bir internet adresi."
    ),
    y!(
        "ses",
        "ses(frekans, süre)",
        "Verilen frekansta (Hz) ve sürede (saniye) kısa bir ses çalar: ses(440, 0.1)."
    ),
    y!(
        "tuş_basılı",
        "tuş_basılı(tuş) → mantık",
        "Tuş şu an basılı mı: \"sol\", \"sağ\", \"yukarı\", \"aşağı\", \"boşluk\", \"enter\", \"a\" ... \"z\", \"0\" ... \"9\"."
    ),
    y!(
        "fare_x",
        "fare_x() → sayı",
        "Farenin (ya da parmağın) oyun alanındaki x konumu."
    ),
    y!(
        "fare_y",
        "fare_y() → sayı",
        "Farenin (ya da parmağın) oyun alanındaki y konumu."
    ),
    y!(
        "fare_basılı",
        "fare_basılı() → mantık",
        "Fare düğmesi basılı ya da ekrana dokunuluyor mu?"
    ),
];

/// Başvuru belgesinin bölümleri: (bölüm adı, bölümün ilk işlevi). YERLESIKLER bu sırayla
/// dizilidir; bir işlevin bölümü, kendisinden önce başlayan son bölümdür.
pub const BOLUMLER: &[(&str, &str)] = &[
    ("Dönüşümler", "uzunluk"),
    ("Metin", "büyük_harf"),
    ("Liste ve sözlük", "sil"),
    ("Dosya", "dosya_oku"),
    ("Matematik", "karekök"),
    ("Zaman ve sistem", "zaman"),
    ("Desenler (düzenli ifadeler)", "eşleşir"),
    ("CSV", "csv_oku"),
    ("Program", "bekle"),
    ("Web", "http_al"),
    ("Telefon", "titret"),
    ("Arayüz", "tema"),
    ("Oyun", "temizle"),
];

/// İşlevin başvuru belgesindeki bölümü.
pub fn bolum(y: &Yerlesik) -> &'static str {
    let sira = YERLESIKLER.iter().position(|x| x.ad == y.ad).unwrap_or(0);
    let mut ad = BOLUMLER[0].0;
    for (b, ilk) in BOLUMLER {
        match YERLESIKLER.iter().position(|x| x.ad == *ilk) {
            Some(i) if i <= sira => ad = b,
            _ => break,
        }
    }
    ad
}

pub fn bul(ad: &str) -> Option<&'static Yerlesik> {
    YERLESIKLER.iter().find(|y| y.ad == ad)
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn bolumler_sirali() {
        let mut onceki = 0;
        for (b, ilk) in BOLUMLER {
            let i = YERLESIKLER
                .iter()
                .position(|x| x.ad == *ilk)
                .unwrap_or_else(|| panic!("{b}: {ilk} yok"));
            assert!(i >= onceki, "{b}");
            onceki = i;
        }
        assert_eq!(bolum(bul("uzunluk").unwrap()), "Dönüşümler");
        assert_eq!(bolum(bul("temizle").unwrap()), "Oyun");
        assert_eq!(bolum(bul("fare_basılı").unwrap()), "Oyun");
    }
}
