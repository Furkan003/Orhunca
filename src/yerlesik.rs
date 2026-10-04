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
    // Web
    y!(
        "json",
        "json(değer) → metin",
        "Değeri (liste, sözlük, model...) JSON metnine çevirir."
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
];

pub fn bul(ad: &str) -> Option<&'static Yerlesik> {
    YERLESIKLER.iter().find(|y| y.ad == ad)
}
