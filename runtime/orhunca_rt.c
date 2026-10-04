/*
 * Orhunca çalışma zamanı ve standart kütüphanesi.
 *
 * Derleyicinin ürettiği makine kodu yazdırma, metin, liste, sözlük, dosya,
 * matematik ve zaman işlemleri için bu kütüphaneyi çağırır. Tüm değerler 64
 * bitlik tamsayı olarak taşınır: metinler NUL ile biten UTF-8 dizilerine,
 * listeler `Liste`, sözlükler `Sozluk` yapısına işaretçidir, ondalıklar
 * `double` bit desenidir.
 *
 * Bellek: metinler, listeler ve sözlükler çöp toplayıcının yönettiği yığından
 * ayrılır. Toplayıcı "tutucu" (conservative) bir işaretle-süpür toplayıcıdır:
 * yığıttaki ve yazmaçlardaki her 64 bitlik sözcüğü olası bir işaretçi sayar,
 * ulaşılamayan nesneleri geri verir. Derleyicinin ayrıca bir şey yapması gerekmez.
 *
 * Tip kodları: 0 sayı, 1 metin, 2 mantık, 3 ondalık, 4 + 8*öğe = liste,
 * 5 + 8*(anahtar + 2*değer) = sözlük (anahtar: 0 sayı, 1 metin), 6 = model.
 *
 * Model nesneleri birer listedir: [tanım, bağlama hataları, kimlik, alanlar...].
 * Tanım, derleyicinin ürettiği sabit bir metindir (modelin adı ve alanları).
 *
 * Web: `ohc_sun` tek iş parçacıklı bir HTTP/1.1 sunucusu başlatır; derleyici
 * her `al "/yol":` tanımını `ohc_web_yol` ile kaydeder.
 */
#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <setjmp.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <shellapi.h>
#include <direct.h>
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <signal.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <unistd.h>
#endif

#define TUR_METIN 0
#define TUR_LISTE 1
#define TUR_SOZLUK 2

#define KOD_SAYI 0
#define KOD_METIN 1
#define KOD_MANTIK 2
#define KOD_ONDALIK 3

#define KOD_MODEL 6

#define M(x) ((const char *)(intptr_t)(x))
#define D(p) ((int64_t)(intptr_t)(p))

/* Web isteği işlenirken oluşan çalışma hataları sunucuyu durdurmaz: hata
 * isteğin başına geri sarılır ve tarayıcıya 500 sayfası gönderilir. */
#if defined(__GNUC__)
typedef void *Tuzak[5];
#define TUZAK_KUR(t) __builtin_setjmp(t)
#define TUZAGA_DON(t) __builtin_longjmp(t, 1)
#else
typedef jmp_buf Tuzak;
#define TUZAK_KUR(t) setjmp(t)
#define TUZAGA_DON(t) longjmp(t, 1)
#endif
static Tuzak istek_tuzagi;
static volatile int tuzak_kurulu;
static char son_hata[1024];

static void hata(int64_t satir, const char *mesaj) {
    fflush(stdout);
    if (satir > 0)
        snprintf(son_hata, sizeof son_hata, "Çalışma hatası (satır %" PRId64 "): %s", satir, mesaj);
    else
        snprintf(son_hata, sizeof son_hata, "Çalışma hatası: %s", mesaj);
    fprintf(stderr, "%s\n", son_hata);
    if (tuzak_kurulu) {
        tuzak_kurulu = 0;
        TUZAGA_DON(istek_tuzagi);
    }
    exit(1);
}

static void *ham_ayir(size_t n) {
    void *p = malloc(n ? n : 1);
    if (!p) hata(0, "bellek yetersiz");
    return p;
}

static void *ham_buyut(void *p, size_t n) {
    p = realloc(p, n ? n : 1);
    if (!p) hata(0, "bellek yetersiz");
    return p;
}

/* ====================================================================== */
/* Çöp toplayıcı                                                          */
/* ====================================================================== */

typedef struct Nesne {
    struct Nesne *sonraki;
    size_t boyut; /* yük + dışarıda tutulan dizilerin bayt sayısı */
    uint8_t tur;
    uint8_t isaretli;
} Nesne;

typedef struct {
    int64_t uzunluk;
    int64_t kapasite;
    int64_t *ogeler; /* ayrıca malloc ile ayrılır, liste ile birlikte geri verilir */
} Liste;

typedef struct {
    int64_t uzunluk;
    int64_t kapasite;
    int64_t anahtar_kodu; /* -1: ilk eklemeye kadar bilinmiyor */
    int64_t *anahtarlar;  /* ekleme sırasıyla */
    int64_t *degerler;
    int64_t *dizin;       /* karma tablo: sıra + 1, 0 boş */
    int64_t dizin_kap;
} Sozluk;

#define BASLIK (sizeof(Nesne))
#define YUK(n) ((void *)((char *)(n) + BASLIK))
#define NESNE(p) ((Nesne *)((char *)(p) - BASLIK))

static Nesne *nesneler;          /* tüm canlı nesnelerin bağlı listesi */
static uintptr_t *tablo;         /* yük adreslerinin karma kümesi */
static size_t tablo_kap, tablo_dolu;
static size_t ayrilan_bayt;      /* son toplamadan beri */
static size_t canli_bayt;
static size_t taban_esik = 8u << 20;
static int sabit_esik;           /* test modu: eşik büyümez, toplayıcı sık çalışır */
static size_t esik = 8u << 20;   /* bu kadar ayrılınca topla */
static uintptr_t *yigin_dibi;
static size_t toplama_sayisi, en_yuksek_canli;

static size_t karma(uintptr_t p) {
    p >>= 3;
    p ^= p >> 17;
    p *= (uintptr_t)0x9E3779B97F4A7C15ull;
    return (size_t)(p ^ (p >> 29));
}

static void tabloya_ekle(uintptr_t p) {
    size_t i = karma(p) & (tablo_kap - 1);
    while (tablo[i]) i = (i + 1) & (tablo_kap - 1);
    tablo[i] = p;
    tablo_dolu++;
}

static void tabloyu_kur(size_t gereken) {
    size_t kap = 1024;
    while (kap < gereken * 2) kap <<= 1;
    free(tablo);
    tablo = calloc(kap, sizeof(uintptr_t));
    if (!tablo) hata(0, "bellek yetersiz");
    tablo_kap = kap;
    tablo_dolu = 0;
    for (Nesne *n = nesneler; n; n = n->sonraki) tabloya_ekle((uintptr_t)YUK(n));
}

static int yonetilen_mi(uintptr_t p) {
    if (!p || (p & 7) || !tablo_kap) return 0;
    size_t i = karma(p) & (tablo_kap - 1);
    while (tablo[i]) {
        if (tablo[i] == p) return 1;
        i = (i + 1) & (tablo_kap - 1);
    }
    return 0;
}

/* İşaretleme yığını (özyineleme yerine) */
static uintptr_t *bekleyen;
static size_t bekleyen_say, bekleyen_kap;

static void aday(uintptr_t p) {
    if (!yonetilen_mi(p)) return;
    Nesne *n = NESNE(p);
    if (n->isaretli) return;
    n->isaretli = 1;
    if (n->tur == TUR_METIN) return;
    if (bekleyen_say == bekleyen_kap) {
        bekleyen_kap = bekleyen_kap ? bekleyen_kap * 2 : 256;
        bekleyen = ham_buyut(bekleyen, bekleyen_kap * sizeof(uintptr_t));
    }
    bekleyen[bekleyen_say++] = p;
}

static void bekleyenleri_isle(void) {
    while (bekleyen_say) {
        uintptr_t p = bekleyen[--bekleyen_say];
        if (NESNE(p)->tur == TUR_LISTE) {
            Liste *l = (Liste *)p;
            for (int64_t i = 0; i < l->uzunluk; i++) aday((uintptr_t)l->ogeler[i]);
        } else {
            Sozluk *s = (Sozluk *)p;
            for (int64_t i = 0; i < s->uzunluk; i++) {
                aday((uintptr_t)s->anahtarlar[i]);
                aday((uintptr_t)s->degerler[i]);
            }
        }
    }
}

#if defined(__GNUC__)
#define SATIR_ICI_DEGIL __attribute__((noinline))
#else
#define SATIR_ICI_DEGIL
#endif

/* Bu işlevin çerçevesi, yazmaçların kaydedildiği `topla` çerçevesinin altındadır;
 * buradan yığıt dibine kadar her sözcük taranır. */
static SATIR_ICI_DEGIL void yigini_tara(void) {
    volatile uintptr_t isaret = 0;
    for (uintptr_t *p = (uintptr_t *)&isaret; p < yigin_dibi; p++) aday(*p);
    bekleyenleri_isle();
}

static void nesneyi_birak(Nesne *n) {
    if (n->tur == TUR_LISTE) {
        free(((Liste *)YUK(n))->ogeler);
    } else if (n->tur == TUR_SOZLUK) {
        Sozluk *s = (Sozluk *)YUK(n);
        free(s->anahtarlar);
        free(s->degerler);
        free(s->dizin);
    }
    free(n);
}

static void supur(void) {
    Nesne **onceki = &nesneler;
    size_t canli = 0, sayi = 0;
    while (*onceki) {
        Nesne *n = *onceki;
        if (n->isaretli) {
            n->isaretli = 0;
            canli += n->boyut;
            sayi++;
            onceki = &n->sonraki;
        } else {
            *onceki = n->sonraki;
            nesneyi_birak(n);
        }
    }
    canli_bayt = canli;
    tabloyu_kur(sayi);
}

static SATIR_ICI_DEGIL void topla(void) {
    jmp_buf yazmaclar; /* çağıranın yazmaçlarındaki işaretçiler buraya düşer */
    setjmp(yazmaclar);
    yigini_tara();
    supur();
    ayrilan_bayt = 0;
    esik = (!sabit_esik && canli_bayt * 2 > taban_esik) ? canli_bayt * 2 : taban_esik;
    toplama_sayisi++;
    (void)yazmaclar;
}

static void hesapla(size_t n) {
    ayrilan_bayt += n;
    canli_bayt += n;
    if (canli_bayt > en_yuksek_canli) en_yuksek_canli = canli_bayt;
}

/* Nesnenin dışarıda tuttuğu dizilerin büyümesini hesaba katar. */
static void buyume(void *yuk, size_t n) {
    NESNE(yuk)->boyut += n;
    hesapla(n);
}

static void *gc_ayir(size_t boyut, uint8_t tur) {
    if (ayrilan_bayt > esik) topla();
    Nesne *n = ham_ayir(BASLIK + boyut);
    n->tur = tur;
    n->isaretli = 0;
    n->boyut = boyut;
    n->sonraki = nesneler;
    nesneler = n;
    if (tablo_dolu * 2 >= tablo_kap) tabloyu_kur(tablo_dolu + 1);
    else tabloya_ekle((uintptr_t)YUK(n));
    hesapla(boyut);
    return YUK(n);
}

static char *metin_ayir(size_t n) { return gc_ayir(n, TUR_METIN); }

/* n baytlık bir parçadan yeni metin oluşturur. */
static int64_t metin_yap(const char *s, size_t n) {
    char *m = metin_ayir(n + 1);
    memcpy(m, s, n);
    m[n] = 0;
    return D(m);
}

/* ====================================================================== */
/* Giriş noktası                                                           */
/* ====================================================================== */

int ohc_ana(void);

static int arguman_sayisi;
static char **argumanlar;

static void argumanlari_kaydet(int argc, char **argv) {
#ifdef _WIN32
    /* Windows'ta argv sistem kod sayfasındadır; UTF-8 için geniş karakterli
     * komut satırından yeniden oluşturulur. */
    int n = 0;
    LPWSTR *genis = CommandLineToArgvW(GetCommandLineW(), &n);
    if (genis) {
        argumanlar = ham_ayir(sizeof(char *) * (size_t)(n + 1));
        for (int i = 0; i < n; i++) {
            int boy = WideCharToMultiByte(CP_UTF8, 0, genis[i], -1, NULL, 0, NULL, NULL);
            argumanlar[i] = ham_ayir((size_t)boy);
            WideCharToMultiByte(CP_UTF8, 0, genis[i], -1, argumanlar[i], boy, NULL, NULL);
        }
        arguman_sayisi = n;
        LocalFree(genis);
        return;
    }
#endif
    arguman_sayisi = argc;
    argumanlar = argv;
}

int main(int argc, char **argv) {
    volatile uintptr_t dip = 0;
    yigin_dibi = (uintptr_t *)&dip + 1;
#ifdef _WIN32
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
#endif
    argumanlari_kaydet(argc, argv);
    /* Test için: ORHUNCA_GC_ESIK=4096 toplayıcıyı çok sık çalıştırır. */
    const char *e = getenv("ORHUNCA_GC_ESIK");
    if (e && atol(e) > 0) {
        taban_esik = esik = (size_t)atol(e);
        sabit_esik = 1;
    }
    tabloyu_kur(0);
    int kod = ohc_ana();
    fflush(stdout);
    if (getenv("ORHUNCA_BELLEK_RAPORU"))
        fprintf(stderr, "bellek: %" PRIu64 " toplama, en yüksek canlı bellek %" PRIu64 " bayt\n",
                (uint64_t)toplama_sayisi, (uint64_t)en_yuksek_canli);
    return kod;
}

void ohc_tasma(int64_t satir) {
    hata(satir, "tamsayı taşması: sonuç 64 bitlik sayı sınırını aştı (çok büyük değerler için ondalık kullanın)");
}

/* ====================================================================== */
/* Biçimlendirme ve yazdırma                                               */
/* ====================================================================== */

static double ondalik(int64_t bitler) {
    double d;
    memcpy(&d, &bitler, sizeof d);
    return d;
}

static int64_t bitlere(double d) {
    int64_t b;
    memcpy(&b, &d, sizeof b);
    return b;
}

/* Büyüyebilen bayt tamponu (çöp toplayıcı dışında). */
typedef struct {
    char *v;
    size_t n, kap;
} Tampon;

static void t_ekle(Tampon *t, const char *s, size_t n) {
    if (t->n + n + 1 > t->kap) {
        size_t kap = t->kap ? t->kap : 64;
        while (kap < t->n + n + 1) kap *= 2;
        t->v = ham_buyut(t->v, kap);
        t->kap = kap;
    }
    memcpy(t->v + t->n, s, n);
    t->n += n;
    t->v[t->n] = 0;
}

static void t_yaz(Tampon *t, const char *s) { t_ekle(t, s, strlen(s)); }

/* Tamponun içeriğinden yeni bir metin yapar ve tamponu bırakır. */
static int64_t t_metin(Tampon *t) {
    int64_t m = metin_yap(t->v ? t->v : "", t->n);
    free(t->v);
    t->v = NULL;
    t->n = t->kap = 0;
    return m;
}

/* 3.0 → "3.0", 0.1 + 0.2 → "0.3" */
static void ondalik_bicimle(double d, char *s, size_t n) {
    snprintf(s, n, "%.15g", d);
    if (!strpbrk(s, ".eni")) strncat(s, ".0", n - strlen(s) - 1);
}

static void model_bicimle(Tampon *t, int64_t d);

static void bicimle(Tampon *t, int64_t d, int64_t kod, int ic) {
    char k[64];
    switch (kod % 8) {
    case KOD_SAYI:
        snprintf(k, sizeof k, "%" PRId64, d);
        t_yaz(t, k);
        break;
    case KOD_METIN:
        if (ic) t_yaz(t, "\"");
        t_yaz(t, M(d));
        if (ic) t_yaz(t, "\"");
        break;
    case KOD_MANTIK: t_yaz(t, d ? "doğru" : "yanlış"); break;
    case KOD_ONDALIK:
        ondalik_bicimle(ondalik(d), k, sizeof k);
        t_yaz(t, k);
        break;
    case 4: {
        Liste *l = (Liste *)(intptr_t)d;
        t_yaz(t, "[");
        for (int64_t i = 0; i < l->uzunluk; i++) {
            if (i) t_yaz(t, ", ");
            bicimle(t, l->ogeler[i], kod / 8, 1);
        }
        t_yaz(t, "]");
        break;
    }
    case 5: {
        Sozluk *s = (Sozluk *)(intptr_t)d;
        int64_t ak = (kod / 8) % 2, dk = (kod / 8) / 2;
        t_yaz(t, "{");
        for (int64_t i = 0; i < s->uzunluk; i++) {
            if (i) t_yaz(t, ", ");
            bicimle(t, s->anahtarlar[i], ak, 1);
            t_yaz(t, ": ");
            bicimle(t, s->degerler[i], dk, 1);
        }
        t_yaz(t, "}");
        break;
    }
    case KOD_MODEL: model_bicimle(t, d); break;
    }
}

void ohc_yaz(int64_t d, int64_t kod) {
    Tampon t = {0};
    bicimle(&t, d, kod, 0);
    t_ekle(&t, "\n", 1);
    fwrite(t.v, 1, t.n, stdout);
    free(t.v);
}

int64_t ohc_metne_cevir(int64_t d, int64_t kod) {
    if (kod == KOD_METIN) return d;
    if (kod == KOD_MANTIK) return D(d ? "doğru" : "yanlış");
    Tampon t = {0};
    bicimle(&t, d, kod, 0);
    return t_metin(&t);
}

/* ====================================================================== */
/* Aritmetik ve matematik                                                  */
/* ====================================================================== */

int64_t ohc_bol(int64_t a, int64_t b, int64_t satir) {
    if (b == 0) hata(satir, "sıfıra bölme");
    if (a == INT64_MIN && b == -1) hata(satir, "bölmede taşma");
    /* Tam bölme aşağı yuvarlar: -7 // 2 = -4 */
    int64_t q = a / b;
    if ((a % b != 0) && ((a < 0) != (b < 0))) q--;
    return q;
}

int64_t ohc_mod(int64_t a, int64_t b, int64_t satir) {
    if (b == 0) hata(satir, "sıfıra göre kalan alınamaz");
    if (b == -1) return 0;
    int64_t r = a % b;
    if (r != 0 && ((r < 0) != (b < 0))) r += b;
    return r;
}

int64_t ohc_ondalik_bol(int64_t a, int64_t b, int64_t satir) {
    double y = ondalik(b);
    if (y == 0.0) hata(satir, "sıfıra bölme");
    return bitlere(ondalik(a) / y);
}

/* Yarımlar sıfırdan uzağa yuvarlanır: 2.5 → 3, -2.5 → -3. */
int64_t ohc_yuvarla(int64_t a) {
    double x = ondalik(a);
    if (x != x) return 0;
    if (x >= 9.2e18) return INT64_MAX;
    if (x <= -9.2e18) return INT64_MIN;
    return x >= 0 ? (int64_t)(x + 0.5) : (int64_t)(x - 0.5);
}

int64_t ohc_yuvarla_basamak(int64_t a, int64_t basamak) {
    double x = ondalik(a), carpan = 1.0;
    if (basamak < 0) basamak = 0;
    if (basamak > 15) basamak = 15;
    for (int64_t i = 0; i < basamak; i++) carpan *= 10.0;
    double r = x * carpan;
    if (r > 9.0e18 || r < -9.0e18) return a; /* zaten tamsayı kadar büyük */
    r = r >= 0 ? (double)(int64_t)(r + 0.5) : (double)(int64_t)(r - 0.5);
    return bitlere(r / carpan);
}

/* islem: 0 karekök, 1 sinüs, 2 kosinüs, 3 tanjant, 4 doğal logaritma */
int64_t ohc_matematik(int64_t islem, int64_t a, int64_t satir) {
    double x = ondalik(a);
    switch (islem) {
    case 0:
        if (x < 0) hata(satir, "negatif sayının karekökü alınamaz");
        return bitlere(sqrt(x));
    case 1: return bitlere(sin(x));
    case 2: return bitlere(cos(x));
    case 3: return bitlere(tan(x));
    default:
        if (x <= 0) hata(satir, "logaritma yalnızca pozitif sayılar için tanımlıdır");
        return bitlere(log(x));
    }
}

int64_t ohc_logaritma_taban(int64_t a, int64_t b, int64_t satir) {
    double x = ondalik(a), t = ondalik(b);
    if (x <= 0) hata(satir, "logaritma yalnızca pozitif sayılar için tanımlıdır");
    if (t <= 0 || t == 1.0) hata(satir, "logaritma tabanı pozitif ve 1'den farklı olmalı");
    return bitlere(log(x) / log(t));
}

int64_t ohc_us_tam(int64_t a, int64_t b, int64_t satir) {
    if (b < 0) hata(satir, "tamsayılarda üs negatif olamaz; ondalık kullanın: üs(2.0, -1)");
    int64_t sonuc = 1, taban = a;
    while (b > 0) {
        if (b & 1) {
            if (__builtin_mul_overflow(sonuc, taban, &sonuc)) ohc_tasma(satir);
        }
        b >>= 1;
        if (b && __builtin_mul_overflow(taban, taban, &taban)) ohc_tasma(satir);
    }
    return sonuc;
}

int64_t ohc_us(int64_t a, int64_t b) { return bitlere(pow(ondalik(a), ondalik(b))); }

int64_t ohc_mutlak(int64_t a, int64_t satir) {
    if (a == INT64_MIN) ohc_tasma(satir);
    return a < 0 ? -a : a;
}

static int64_t en_kars(int64_t a, int64_t b, int64_t kod);

int64_t ohc_en_iki(int64_t a, int64_t b, int64_t kod, int64_t yon) {
    int k = (int)en_kars(a, b, kod);
    return (yon > 0 ? k >= 0 : k <= 0) ? a : b;
}

/* Rastgele sayılar: xorshift64*. ORHUNCA_TOHUM ile tekrarlanabilir. */
static uint64_t rng_durum;

static uint64_t rng(void) {
    if (!rng_durum) {
        const char *t = getenv("ORHUNCA_TOHUM");
        uint64_t tohum = t ? (uint64_t)strtoull(t, NULL, 10) : 0;
        if (!tohum) {
            volatile int yerel = 0;
            tohum = (uint64_t)time(NULL) ^ ((uint64_t)(uintptr_t)&yerel << 16) ^ (uint64_t)clock();
        }
        rng_durum = tohum * 0x9E3779B97F4A7C15ull + 1;
        if (!rng_durum) rng_durum = 1;
    }
    rng_durum ^= rng_durum >> 12;
    rng_durum ^= rng_durum << 25;
    rng_durum ^= rng_durum >> 27;
    return rng_durum * 0x2545F4914F6CDD1Dull;
}

int64_t ohc_rastgele(void) { return bitlere((double)(rng() >> 11) * (1.0 / 9007199254740992.0)); }

int64_t ohc_rastgele_aralik(int64_t a, int64_t b, int64_t satir) {
    if (a > b) hata(satir, "rastgele(a, b) için a, b'den büyük olamaz");
    uint64_t aralik = (uint64_t)b - (uint64_t)a + 1;
    if (aralik == 0) return (int64_t)rng();
    return (int64_t)((uint64_t)a + (uint64_t)(((__uint128_t)rng() * aralik) >> 64));
}

/* ====================================================================== */
/* Metinler                                                                */
/* ====================================================================== */

/* UTF-8 öncü baytına göre karakter uzunluğu. */
static int u8_boy(unsigned char c) { return c < 0x80 ? 1 : c < 0xE0 ? 2 : c < 0xF0 ? 3 : 4; }

/* n karakter ilerler (metnin sonunda durur). */
static const char *u8_ilerle(const char *s, int64_t n) {
    while (n > 0 && *s) {
        int b = u8_boy((unsigned char)*s);
        for (int i = 0; i < b && *s; i++) s++;
        n--;
    }
    return s;
}

/* UTF-8 karakter sayısı (bayt değil). */
int64_t ohc_metin_uzunluk(int64_t a) {
    int64_t n = 0;
    for (const unsigned char *p = (const unsigned char *)M(a); *p; p++)
        if ((*p & 0xC0) != 0x80) n++;
    return n;
}

int64_t ohc_metin_birlestir(int64_t a, int64_t b) {
    size_t n = strlen(M(a)), m = strlen(M(b));
    char *s = metin_ayir(n + m + 1);
    /* a ve b hâlâ bu çerçevede canlıdır, toplayıcı onları görür */
    memcpy(s, M(a), n);
    memcpy(s + n, M(b), m + 1);
    return D(s);
}

int64_t ohc_metin_esit(int64_t a, int64_t b) { return strcmp(M(a), M(b)) == 0; }

int64_t ohc_metin_harf(int64_t m, int64_t i, int64_t satir) {
    int64_t n = ohc_metin_uzunluk(m);
    if (i < 0 || i >= n) {
        char mesaj[128];
        snprintf(mesaj, sizeof mesaj, "metnin sınırı aşıldı: sıra %" PRId64 ", uzunluk %" PRId64, i, n);
        hata(satir, mesaj);
    }
    const char *p = u8_ilerle(M(m), i);
    return metin_yap(p, (size_t)(u8_ilerle(p, 1) - p));
}

int64_t ohc_metin_parca(int64_t m, int64_t bas, int64_t uz) {
    if (bas < 0) bas = 0;
    if (uz < 0) uz = 0;
    const char *p = u8_ilerle(M(m), bas);
    const char *q = u8_ilerle(p, uz);
    return metin_yap(p, (size_t)(q - p));
}

/* Kod noktası okur / yazar. */
static uint32_t u8_oku(const char **s) {
    const unsigned char *p = (const unsigned char *)*s;
    uint32_t c = p[0];
    int b = u8_boy(p[0]);
    if (b == 1) {
        *s += 1;
        return c;
    }
    c &= (b == 2) ? 0x1F : (b == 3) ? 0x0F : 0x07;
    for (int i = 1; i < b; i++) {
        if ((p[i] & 0xC0) != 0x80) { /* bozuk dizi: tek bayt olarak geç */
            *s += 1;
            return p[0];
        }
        c = (c << 6) | (p[i] & 0x3F);
    }
    *s += b;
    return c;
}

static void u8_yaz(Tampon *t, uint32_t c) {
    char b[4];
    size_t n;
    if (c < 0x80) { b[0] = (char)c; n = 1; }
    else if (c < 0x800) { b[0] = (char)(0xC0 | (c >> 6)); b[1] = (char)(0x80 | (c & 0x3F)); n = 2; }
    else if (c < 0x10000) {
        b[0] = (char)(0xE0 | (c >> 12)); b[1] = (char)(0x80 | ((c >> 6) & 0x3F)); b[2] = (char)(0x80 | (c & 0x3F)); n = 3;
    } else {
        b[0] = (char)(0xF0 | (c >> 18)); b[1] = (char)(0x80 | ((c >> 12) & 0x3F));
        b[2] = (char)(0x80 | ((c >> 6) & 0x3F)); b[3] = (char)(0x80 | (c & 0x3F)); n = 4;
    }
    t_ekle(t, b, n);
}

/* Türkçe büyük/küçük harf: i ↔ İ, ı ↔ I; diğer Latin harfleri olağan biçimde. */
static uint32_t buyuk(uint32_t c) {
    if (c == 'i') return 0x130;
    if (c == 0x131) return 'I';
    if (c >= 'a' && c <= 'z') return c - 32;
    if (c >= 0xE0 && c <= 0xFE && c != 0xF7) return c - 32;
    if (c >= 0x100 && c <= 0x17F && c != 0x131 && (c & 1)) return c - 1; /* ğ ş ... */
    return c;
}

static uint32_t kucuk(uint32_t c) {
    if (c == 'I') return 0x131;
    if (c == 0x130) return 'i';
    if (c >= 'A' && c <= 'Z') return c + 32;
    if (c >= 0xC0 && c <= 0xDE && c != 0xD7) return c + 32;
    if (c >= 0x100 && c <= 0x17F && c != 0x130 && !(c & 1)) return c + 1;
    return c;
}

static int64_t harf_donustur(int64_t m, uint32_t (*f)(uint32_t)) {
    Tampon t = {0};
    const char *p = M(m);
    while (*p) u8_yaz(&t, f(u8_oku(&p)));
    return t_metin(&t);
}

int64_t ohc_buyuk_harf(int64_t m) { return harf_donustur(m, buyuk); }
int64_t ohc_kucuk_harf(int64_t m) { return harf_donustur(m, kucuk); }

static int bosluk(char c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\v' || c == '\f'; }

int64_t ohc_kirp(int64_t m) {
    const char *s = M(m);
    while (*s && bosluk(*s)) s++;
    size_t n = strlen(s);
    while (n && bosluk(s[n - 1])) n--;
    return metin_yap(s, n);
}

int64_t ohc_liste_yeni(void);
void ohc_liste_ekle(int64_t lp, int64_t d);

int64_t ohc_metin_bol(int64_t m, int64_t ayrac) {
    int64_t l = ohc_liste_yeni();
    const char *s = M(m), *a = M(ayrac);
    size_t an = strlen(a);
    if (an == 0) { /* boşluklardan böl, boş parçaları atla */
        while (*s) {
            while (*s && bosluk(*s)) s++;
            if (!*s) break;
            const char *bas = s;
            while (*s && !bosluk(*s)) s++;
            ohc_liste_ekle(l, metin_yap(bas, (size_t)(s - bas)));
        }
        return l;
    }
    for (;;) {
        const char *q = strstr(s, a);
        if (!q) {
            ohc_liste_ekle(l, metin_yap(s, strlen(s)));
            break;
        }
        ohc_liste_ekle(l, metin_yap(s, (size_t)(q - s)));
        s = q + an;
    }
    return l;
}

int64_t ohc_satirlar(int64_t m) {
    int64_t l = ohc_liste_yeni();
    const char *s = M(m);
    while (*s) {
        const char *q = strchr(s, '\n');
        size_t n = q ? (size_t)(q - s) : strlen(s);
        size_t k = (n && s[n - 1] == '\r') ? n - 1 : n;
        ohc_liste_ekle(l, metin_yap(s, k));
        if (!q) break;
        s = q + 1;
    }
    return l;
}

int64_t ohc_harfler(int64_t m) {
    int64_t l = ohc_liste_yeni();
    const char *s = M(m);
    while (*s) {
        const char *q = u8_ilerle(s, 1);
        ohc_liste_ekle(l, metin_yap(s, (size_t)(q - s)));
        s = q;
    }
    return l;
}

int64_t ohc_birlestir(int64_t lp, int64_t ayrac) {
    Liste *l = (Liste *)(intptr_t)lp;
    Tampon t = {0};
    for (int64_t i = 0; i < l->uzunluk; i++) {
        if (i) t_yaz(&t, M(ayrac));
        t_yaz(&t, M(l->ogeler[i]));
    }
    return t_metin(&t);
}

int64_t ohc_metin_icerir(int64_t m, int64_t a) { return strstr(M(m), M(a)) != NULL; }

int64_t ohc_metin_bul(int64_t m, int64_t a) {
    const char *q = strstr(M(m), M(a));
    if (!q) return -1;
    int64_t n = 0;
    for (const char *p = M(m); p < q; p++)
        if ((*p & 0xC0) != 0x80) n++;
    return n;
}

int64_t ohc_degistir(int64_t m, int64_t eski, int64_t yeni) {
    const char *s = M(m), *e = M(eski), *y = M(yeni);
    size_t en = strlen(e);
    if (!en) return m;
    Tampon t = {0};
    for (;;) {
        const char *q = strstr(s, e);
        if (!q) {
            t_yaz(&t, s);
            break;
        }
        t_ekle(&t, s, (size_t)(q - s));
        t_yaz(&t, y);
        s = q + en;
    }
    return t_metin(&t);
}

int64_t ohc_baslar(int64_t m, int64_t on) { return strncmp(M(m), M(on), strlen(M(on))) == 0; }

int64_t ohc_biter(int64_t m, int64_t son) {
    size_t n = strlen(M(m)), k = strlen(M(son));
    return k <= n && strcmp(M(m) + n - k, M(son)) == 0;
}

int64_t ohc_tekrarla(int64_t m, int64_t kac, int64_t satir) {
    if (kac < 0) hata(satir, "tekrar sayısı negatif olamaz");
    size_t n = strlen(M(m));
    if (n && (uint64_t)kac > (uint64_t)(1u << 30) / n) hata(satir, "tekrarlanan metin çok büyük");
    char *s = metin_ayir(n * (size_t)kac + 1);
    for (int64_t i = 0; i < kac; i++) memcpy(s + n * (size_t)i, M(m), n);
    s[n * (size_t)kac] = 0;
    return D(s);
}

int64_t ohc_metin_ters(int64_t m) {
    const char *s = M(m);
    size_t n = strlen(s);
    char *r = metin_ayir(n + 1);
    size_t yaz = n;
    while (*s) {
        int b = u8_boy((unsigned char)*s);
        size_t k = 0;
        while (k < (size_t)b && s[k]) k++;
        yaz -= k;
        memcpy(r + yaz, s, k);
        s += k;
    }
    r[n] = 0;
    return D(r);
}

static void cevirme_hatasi(int64_t a, int64_t satir, const char *ne) {
    char mesaj[256];
    snprintf(mesaj, sizeof mesaj, "'%.200s' bir %s değil", M(a), ne);
    hata(satir, mesaj);
}

static int sayi_oku(const char *s, int64_t *n) {
    char *son;
    while (bosluk(*s)) s++;
    errno = 0;
    *n = strtoll(s, &son, 10);
    if (son == s || errno == ERANGE) return 0;
    while (bosluk(*son)) son++;
    return *son == 0;
}

/* Türkçe yazımdaki virgül de kabul edilir: "3,5" */
static int ondalik_oku(const char *s, double *d) {
    char tampon[128];
    size_t n = strlen(s);
    if (n >= sizeof tampon) return 0;
    for (size_t i = 0; i <= n; i++) tampon[i] = s[i] == ',' ? '.' : s[i];
    char *p = tampon, *son;
    while (bosluk(*p)) p++;
    *d = strtod(p, &son);
    if (son == p) return 0;
    while (bosluk(*son)) son++;
    return *son == 0;
}

int64_t ohc_metinden_sayi(int64_t a, int64_t satir) {
    int64_t n;
    if (!sayi_oku(M(a), &n)) cevirme_hatasi(a, satir, "sayı");
    return n;
}

int64_t ohc_metinden_ondalik(int64_t a, int64_t satir) {
    double d;
    if (!ondalik_oku(M(a), &d)) cevirme_hatasi(a, satir, "ondalık sayı");
    return bitlere(d);
}

int64_t ohc_sayi_mi(int64_t a) {
    int64_t n;
    return sayi_oku(M(a), &n);
}

int64_t ohc_ondalik_mi(int64_t a) {
    double d;
    return ondalik_oku(M(a), &d);
}

int64_t ohc_oku(void) {
    Tampon t = {0};
    int c;
    fflush(stdout);
    while ((c = getchar()) != EOF && c != '\n') {
        char b = (char)c;
        t_ekle(&t, &b, 1);
    }
    if (t.n && t.v[t.n - 1] == '\r') t.v[--t.n] = 0;
    return t_metin(&t);
}

/* Türk alfabesine göre karakter sırası: a b c ç d e f g ğ h ı i j ... */
static const char *ALFABE[] = {"a", "b", "c", "ç", "d", "e", "f", "g", "ğ", "h", "ı", "i", "j", "k", "l", "m",
                               "n", "o", "ö", "p", "q", "r", "s", "ş", "t", "u", "ü", "v", "w", "x", "y", "z"};
static const char *BUYUK_ALFABE[] = {"A", "B", "C", "Ç", "D", "E", "F", "G", "Ğ", "H", "I",
                                     "İ", "J", "K", "L", "M", "N", "O", "Ö", "P", "Q", "R",
                                     "S", "Ş", "T", "U", "Ü", "V", "W", "X", "Y", "Z"};

/* Bir karakteri okur, sıra değerini döndürür ve imleci ilerletir. */
static int64_t harf_sirasi(const unsigned char **p) {
    /* büyük ve küçük harf aynı sıradadır (I → ı, İ → i) */
    for (int i = 0; i < 32; i++) {
        const char *harfler[2] = {ALFABE[i], BUYUK_ALFABE[i]};
        for (int j = 0; j < 2; j++) {
            size_t n = strlen(harfler[j]);
            if (!strncmp((const char *)*p, harfler[j], n)) {
                *p += n;
                return 1000 + i;
            }
        }
    }
    /* harf dışı karakter: kod noktası (rakamlar ve noktalama harflerden önce) */
    const char *s = (const char *)*p;
    uint32_t c = u8_oku(&s);
    *p = (const unsigned char *)s;
    return c < 0x80 ? (int64_t)c : 100000 + (int64_t)c;
}

static int metin_karsilastir(const char *a, const char *b) {
    const unsigned char *x = (const unsigned char *)a, *y = (const unsigned char *)b;
    while (*x && *y) {
        int64_t p = harf_sirasi(&x), q = harf_sirasi(&y);
        if (p != q) return p < q ? -1 : 1;
    }
    if (*x) return 1;
    if (*y) return -1;
    int k = strcmp(a, b);
    return (k > 0) - (k < 0);
}

int64_t ohc_metin_kars(int64_t a, int64_t b) { return metin_karsilastir(M(a), M(b)); }

/* Tip koduna göre iki değeri karşılaştırır: -1, 0, 1 */
static int64_t en_kars(int64_t a, int64_t b, int64_t kod) {
    if (kod == KOD_METIN) return metin_karsilastir(M(a), M(b));
    if (kod == KOD_ONDALIK) {
        double x = ondalik(a), y = ondalik(b);
        return (x > y) - (x < y);
    }
    return (a > b) - (a < b);
}

static int degerler_esit(int64_t a, int64_t b, int64_t kod) {
    if (kod == KOD_METIN) return strcmp(M(a), M(b)) == 0;
    if (kod == KOD_ONDALIK) return ondalik(a) == ondalik(b);
    return a == b;
}

/* ====================================================================== */
/* Listeler                                                                */
/* ====================================================================== */

int64_t ohc_liste_yeni(void) {
    Liste *l = gc_ayir(sizeof(Liste), TUR_LISTE);
    l->uzunluk = 0;
    l->kapasite = 0;
    l->ogeler = NULL;
    l->ogeler = ham_ayir(sizeof(int64_t) * 4);
    l->kapasite = 4;
    buyume(l, sizeof(int64_t) * 4);
    return D(l);
}

void ohc_liste_ekle(int64_t lp, int64_t d) {
    Liste *l = (Liste *)(intptr_t)lp;
    if (l->uzunluk == l->kapasite) {
        size_t eklenen = sizeof(int64_t) * (size_t)l->kapasite;
        l->ogeler = ham_buyut(l->ogeler, sizeof(int64_t) * (size_t)l->kapasite * 2);
        l->kapasite *= 2;
        buyume(l, eklenen);
    }
    l->ogeler[l->uzunluk++] = d;
}

int64_t ohc_liste_uzunluk(int64_t lp) { return ((Liste *)(intptr_t)lp)->uzunluk; }

static void sinir(Liste *l, int64_t i, int64_t satir) {
    if (i < 0 || i >= l->uzunluk) {
        char mesaj[128];
        snprintf(mesaj, sizeof mesaj, "liste sınırı aşıldı: sıra %" PRId64 ", uzunluk %" PRId64, i,
                 l->uzunluk);
        hata(satir, mesaj);
    }
}

int64_t ohc_liste_al(int64_t lp, int64_t i, int64_t satir) {
    Liste *l = (Liste *)(intptr_t)lp;
    sinir(l, i, satir);
    return l->ogeler[i];
}

void ohc_liste_koy(int64_t lp, int64_t i, int64_t d, int64_t satir) {
    Liste *l = (Liste *)(intptr_t)lp;
    sinir(l, i, satir);
    l->ogeler[i] = d;
}

int64_t ohc_liste_sil(int64_t lp, int64_t i, int64_t satir) {
    Liste *l = (Liste *)(intptr_t)lp;
    sinir(l, i, satir);
    int64_t d = l->ogeler[i];
    memmove(l->ogeler + i, l->ogeler + i + 1, sizeof(int64_t) * (size_t)(l->uzunluk - i - 1));
    l->uzunluk--;
    return d;
}

int64_t ohc_liste_bul(int64_t lp, int64_t x, int64_t kod) {
    Liste *l = (Liste *)(intptr_t)lp;
    for (int64_t i = 0; i < l->uzunluk; i++)
        if (degerler_esit(l->ogeler[i], x, kod)) return i;
    return -1;
}

int64_t ohc_liste_cikar(int64_t lp, int64_t x, int64_t kod) {
    int64_t i = ohc_liste_bul(lp, x, kod);
    if (i < 0) return 0;
    ohc_liste_sil(lp, i, 0);
    return 1;
}

int64_t ohc_liste_parca(int64_t lp, int64_t bas, int64_t uz) {
    int64_t yeni = ohc_liste_yeni();
    Liste *l = (Liste *)(intptr_t)lp;
    if (bas < 0) bas = 0;
    for (int64_t i = bas; i < l->uzunluk && i - bas < uz; i++) ohc_liste_ekle(yeni, l->ogeler[i]);
    return yeni;
}

int64_t ohc_liste_kopya(int64_t lp) {
    int64_t yeni = ohc_liste_yeni();
    Liste *l = (Liste *)(intptr_t)lp;
    for (int64_t i = 0; i < l->uzunluk; i++) ohc_liste_ekle(yeni, l->ogeler[i]);
    return yeni;
}

int64_t ohc_liste_ters(int64_t lp) {
    int64_t yeni = ohc_liste_yeni();
    Liste *l = (Liste *)(intptr_t)lp;
    for (int64_t i = l->uzunluk - 1; i >= 0; i--) ohc_liste_ekle(yeni, l->ogeler[i]);
    return yeni;
}

void ohc_karistir(int64_t lp) {
    Liste *l = (Liste *)(intptr_t)lp;
    for (int64_t i = l->uzunluk - 1; i > 0; i--) {
        int64_t j = (int64_t)(rng() % (uint64_t)(i + 1));
        int64_t g = l->ogeler[i];
        l->ogeler[i] = l->ogeler[j];
        l->ogeler[j] = g;
    }
}

int64_t ohc_liste_en(int64_t lp, int64_t kod, int64_t yon, int64_t satir) {
    Liste *l = (Liste *)(intptr_t)lp;
    if (l->uzunluk == 0) hata(satir, yon > 0 ? "boş listenin en büyük öğesi yok" : "boş listenin en küçük öğesi yok");
    int64_t en = l->ogeler[0];
    for (int64_t i = 1; i < l->uzunluk; i++) {
        int64_t k = en_kars(l->ogeler[i], en, kod);
        if (yon > 0 ? k > 0 : k < 0) en = l->ogeler[i];
    }
    return en;
}

int64_t ohc_liste_toplam(int64_t lp, int64_t kod, int64_t satir) {
    Liste *l = (Liste *)(intptr_t)lp;
    if (kod == KOD_ONDALIK) {
        double t = 0;
        for (int64_t i = 0; i < l->uzunluk; i++) t += ondalik(l->ogeler[i]);
        return bitlere(t);
    }
    int64_t t = 0;
    for (int64_t i = 0; i < l->uzunluk; i++)
        if (__builtin_add_overflow(t, l->ogeler[i], &t)) ohc_tasma(satir);
    return t;
}

static int sayi_kars(const void *a, const void *b) {
    int64_t x = *(const int64_t *)a, y = *(const int64_t *)b;
    return (x > y) - (x < y);
}

static int ondalik_kars(const void *a, const void *b) {
    double x = ondalik(*(const int64_t *)a), y = ondalik(*(const int64_t *)b);
    return (x > y) - (x < y);
}

static int metin_kars(const void *a, const void *b) {
    return metin_karsilastir(M(*(const int64_t *)a), M(*(const int64_t *)b));
}

void ohc_liste_sirala(int64_t lp, int64_t oge_kodu) {
    Liste *l = (Liste *)(intptr_t)lp;
    int (*kars)(const void *, const void *) =
        oge_kodu == KOD_METIN ? metin_kars : oge_kodu == KOD_ONDALIK ? ondalik_kars : sayi_kars;
    if (l->uzunluk > 1) qsort(l->ogeler, (size_t)l->uzunluk, sizeof(int64_t), kars);
}

/* ====================================================================== */
/* Sözlükler                                                               */
/* ====================================================================== */

static uint64_t anahtar_karmasi(int64_t k, int64_t kod) {
    uint64_t h;
    if (kod == KOD_METIN) {
        h = 1469598103934665603ull; /* FNV-1a */
        for (const unsigned char *p = (const unsigned char *)M(k); *p; p++) h = (h ^ *p) * 1099511628211ull;
    } else {
        h = (uint64_t)k * 0x9E3779B97F4A7C15ull;
        h ^= h >> 31;
    }
    return h;
}

static int64_t s_kod(Sozluk *s, int64_t kod) { return s->anahtar_kodu >= 0 ? s->anahtar_kodu : kod; }

static int64_t sozlukte_bul(Sozluk *s, int64_t k, int64_t kod) {
    if (!s->dizin_kap) return -1;
    kod = s_kod(s, kod);
    uint64_t i = anahtar_karmasi(k, kod) & (uint64_t)(s->dizin_kap - 1);
    while (s->dizin[i]) {
        int64_t sira = s->dizin[i] - 1;
        if (degerler_esit(s->anahtarlar[sira], k, kod)) return sira;
        i = (i + 1) & (uint64_t)(s->dizin_kap - 1);
    }
    return -1;
}

static void dizini_kur(Sozluk *s) {
    int64_t kap = 16;
    while (kap < s->uzunluk * 2 + 2) kap *= 2;
    if (kap > s->dizin_kap) buyume(s, sizeof(int64_t) * (size_t)(kap - s->dizin_kap));
    free(s->dizin);
    s->dizin = calloc((size_t)kap, sizeof(int64_t));
    if (!s->dizin) hata(0, "bellek yetersiz");
    s->dizin_kap = kap;
    for (int64_t j = 0; j < s->uzunluk; j++) {
        uint64_t i = anahtar_karmasi(s->anahtarlar[j], s->anahtar_kodu) & (uint64_t)(kap - 1);
        while (s->dizin[i]) i = (i + 1) & (uint64_t)(kap - 1);
        s->dizin[i] = j + 1;
    }
}

int64_t ohc_sozluk_yeni(void) {
    Sozluk *s = gc_ayir(sizeof(Sozluk), TUR_SOZLUK);
    memset(s, 0, sizeof *s);
    s->anahtar_kodu = -1;
    return D(s);
}

void ohc_sozluk_koy(int64_t sp, int64_t k, int64_t d, int64_t kod) {
    Sozluk *s = (Sozluk *)(intptr_t)sp;
    if (s->anahtar_kodu < 0) s->anahtar_kodu = kod;
    int64_t sira = sozlukte_bul(s, k, kod);
    if (sira >= 0) {
        s->degerler[sira] = d;
        return;
    }
    if (s->uzunluk == s->kapasite) {
        int64_t kap = s->kapasite ? s->kapasite * 2 : 8;
        s->anahtarlar = ham_buyut(s->anahtarlar, sizeof(int64_t) * (size_t)kap);
        s->degerler = ham_buyut(s->degerler, sizeof(int64_t) * (size_t)kap);
        buyume(s, 2 * sizeof(int64_t) * (size_t)(kap - s->kapasite));
        s->kapasite = kap;
    }
    s->anahtarlar[s->uzunluk] = k;
    s->degerler[s->uzunluk] = d;
    s->uzunluk++;
    if (s->uzunluk * 2 > s->dizin_kap) {
        dizini_kur(s);
    } else {
        uint64_t i = anahtar_karmasi(k, s->anahtar_kodu) & (uint64_t)(s->dizin_kap - 1);
        while (s->dizin[i]) i = (i + 1) & (uint64_t)(s->dizin_kap - 1);
        s->dizin[i] = s->uzunluk;
    }
}

int64_t ohc_sozluk_al(int64_t sp, int64_t k, int64_t kod, int64_t satir) {
    Sozluk *s = (Sozluk *)(intptr_t)sp;
    int64_t sira = sozlukte_bul(s, k, kod);
    if (sira < 0) {
        Tampon t = {0};
        t_yaz(&t, "sözlükte ");
        bicimle(&t, k, s_kod(s, kod), 1);
        t_yaz(&t, " anahtarı yok (önce içerir(sözlük, anahtar) ile denetleyin)");
        hata(satir, t.v);
    }
    return s->degerler[sira];
}

int64_t ohc_sozluk_icerir(int64_t sp, int64_t k, int64_t kod) {
    return sozlukte_bul((Sozluk *)(intptr_t)sp, k, kod) >= 0;
}

void ohc_sozluk_sil(int64_t sp, int64_t k, int64_t kod) {
    Sozluk *s = (Sozluk *)(intptr_t)sp;
    int64_t sira = sozlukte_bul(s, k, kod);
    if (sira < 0) return;
    size_t kalan = sizeof(int64_t) * (size_t)(s->uzunluk - sira - 1);
    memmove(s->anahtarlar + sira, s->anahtarlar + sira + 1, kalan);
    memmove(s->degerler + sira, s->degerler + sira + 1, kalan);
    s->uzunluk--;
    dizini_kur(s);
}

int64_t ohc_sozluk_uzunluk(int64_t sp) { return ((Sozluk *)(intptr_t)sp)->uzunluk; }

int64_t ohc_sozluk_anahtarlar(int64_t sp) {
    int64_t l = ohc_liste_yeni();
    Sozluk *s = (Sozluk *)(intptr_t)sp;
    for (int64_t i = 0; i < s->uzunluk; i++) ohc_liste_ekle(l, s->anahtarlar[i]);
    return l;
}

int64_t ohc_sozluk_degerler(int64_t sp) {
    int64_t l = ohc_liste_yeni();
    Sozluk *s = (Sozluk *)(intptr_t)sp;
    for (int64_t i = 0; i < s->uzunluk; i++) ohc_liste_ekle(l, s->degerler[i]);
    return l;
}

/* ====================================================================== */
/* Dosyalar                                                                */
/* ====================================================================== */

static FILE *dosya_ac(const char *yol, const char *kip) {
#ifdef _WIN32
    /* Türkçe karakterli yollar için geniş karakterli API */
    wchar_t wyol[1024], wkip[8];
    if (!MultiByteToWideChar(CP_UTF8, 0, yol, -1, wyol, 1024)) return NULL;
    MultiByteToWideChar(CP_UTF8, 0, kip, -1, wkip, 8);
    return _wfopen(wyol, wkip);
#else
    return fopen(yol, kip);
#endif
}

static void dosya_hatasi(const char *yol, int64_t satir, const char *ne) {
    const char *neden = errno == ENOENT ? "dosya ya da klasör bulunamadı"
                        : errno == EACCES ? "izin yok"
                        : errno == EISDIR ? "bu bir klasör"
                                          : strerror(errno);
    char mesaj[600];
    snprintf(mesaj, sizeof mesaj, "'%.400s' %s: %s", yol, ne, neden);
    hata(satir, mesaj);
}

int64_t ohc_dosya_oku(int64_t yol, int64_t satir) {
    FILE *f = dosya_ac(M(yol), "rb");
    if (!f) dosya_hatasi(M(yol), satir, "okunamadı");
    Tampon t = {0};
    char b[8192];
    size_t n;
    while ((n = fread(b, 1, sizeof b, f)) > 0) t_ekle(&t, b, n);
    if (ferror(f)) {
        fclose(f);
        dosya_hatasi(M(yol), satir, "okunamadı");
    }
    fclose(f);
    return t_metin(&t);
}

void ohc_dosyaya_yaz(int64_t yol, int64_t m, int64_t ekle, int64_t satir) {
    FILE *f = dosya_ac(M(yol), ekle ? "ab" : "wb");
    if (!f) dosya_hatasi(M(yol), satir, "yazılamadı");
    size_t n = strlen(M(m));
    if (fwrite(M(m), 1, n, f) != n) {
        fclose(f);
        dosya_hatasi(M(yol), satir, "yazılamadı");
    }
    if (fclose(f) != 0) dosya_hatasi(M(yol), satir, "yazılamadı");
}

int64_t ohc_dosya_var(int64_t yol) {
    FILE *f = dosya_ac(M(yol), "rb");
    if (!f) return 0;
    fclose(f);
    return 1;
}

int64_t ohc_dosya_sil(int64_t yol) {
#ifdef _WIN32
    wchar_t wyol[1024];
    if (!MultiByteToWideChar(CP_UTF8, 0, M(yol), -1, wyol, 1024)) return 0;
    return _wremove(wyol) == 0;
#else
    return remove(M(yol)) == 0;
#endif
}

/* ====================================================================== */
/* Zaman ve sistem                                                         */
/* ====================================================================== */

int64_t ohc_zaman(void) {
#ifdef _WIN32
    FILETIME ft;
    GetSystemTimeAsFileTime(&ft);
    uint64_t t = ((uint64_t)ft.dwHighDateTime << 32) | ft.dwLowDateTime; /* 1601'den beri 100 ns */
    return bitlere((double)(t - 116444736000000000ull) / 1e7);
#else
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return bitlere((double)ts.tv_sec + (double)ts.tv_nsec / 1e9);
#endif
}

int64_t ohc_tarih(void) {
    time_t simdi = time(NULL);
    struct tm yerel;
#ifdef _WIN32
    yerel = *localtime(&simdi);
#else
    localtime_r(&simdi, &yerel);
#endif
    char b[32];
    strftime(b, sizeof b, "%Y-%m-%d %H:%M:%S", &yerel);
    return metin_yap(b, strlen(b));
}

void ohc_bekle(int64_t saniye) {
    double s = ondalik(saniye);
    if (!(s > 0)) return;
    fflush(stdout);
#ifdef _WIN32
    Sleep((DWORD)(s * 1000.0));
#else
    struct timespec ts;
    ts.tv_sec = (time_t)s;
    ts.tv_nsec = (long)((s - (double)ts.tv_sec) * 1e9);
    while (nanosleep(&ts, &ts) == -1 && errno == EINTR) {
    }
#endif
}

int64_t ohc_argumanlar(void) {
    int64_t l = ohc_liste_yeni();
    for (int i = 1; i < arguman_sayisi; i++) ohc_liste_ekle(l, metin_yap(argumanlar[i], strlen(argumanlar[i])));
    return l;
}

int64_t ohc_ortam(int64_t ad) {
    const char *d = getenv(M(ad));
    return metin_yap(d ? d : "", d ? strlen(d) : 0);
}

void ohc_cik(int64_t kod) {
    fflush(stdout);
    exit((int)kod);
}

/* ====================================================================== */
/* Modeller                                                                */
/* ====================================================================== */

typedef struct {
    char *ad;
    int64_t kod;
    int zorunlu;
    int en_az_var, en_fazla_var;
    double en_az, en_fazla;
} AlanBilgisi;

typedef struct ModelBilgisi {
    const char *tanim;
    char *ad;
    int64_t alan_sayisi;
    AlanBilgisi *alanlar;
    struct ModelBilgisi *sonraki;
} ModelBilgisi;

static ModelBilgisi *model_bilgileri;

static char *kopya_n(const char *s, size_t n) {
    char *k = ham_ayir(n + 1);
    memcpy(k, s, n);
    k[n] = 0;
    return k;
}

/* Derleyicinin ürettiği tanım metni: ilk satır modelin adı, sonra her alan için
 * "ad<TAB>tip kodu<TAB>zorunlu<TAB>en az<TAB>en fazla". Bir kez çözülür. */
static ModelBilgisi *model_bilgisi(const char *tanim) {
    for (ModelBilgisi *m = model_bilgileri; m; m = m->sonraki)
        if (m->tanim == tanim) return m;
    ModelBilgisi *m = ham_ayir(sizeof *m);
    memset(m, 0, sizeof *m);
    m->tanim = tanim;
    const char *p = tanim;
    const char *son = strchr(p, '\n');
    m->ad = kopya_n(p, son ? (size_t)(son - p) : strlen(p));
    p = son ? son + 1 : p + strlen(p);
    int64_t n = 0;
    for (const char *q = p; *q; q++)
        if (*q == '\n') n++;
    m->alanlar = ham_ayir(sizeof(AlanBilgisi) * (size_t)(n ? n : 1));
    while (*p) {
        son = strchr(p, '\n');
        if (!son) son = p + strlen(p);
        const char *parca[5] = {"", "0", "0", "", ""};
        size_t uz[5] = {0, 1, 1, 0, 0};
        int k = 0;
        const char *b = p;
        for (const char *q = p; q <= son && k < 5; q++) {
            if (q == son || *q == '\t') {
                parca[k] = b;
                uz[k] = (size_t)(q - b);
                k++;
                b = q + 1;
            }
        }
        AlanBilgisi *a = &m->alanlar[m->alan_sayisi++];
        memset(a, 0, sizeof *a);
        a->ad = kopya_n(parca[0], uz[0]);
        a->kod = strtoll(parca[1], NULL, 10);
        a->zorunlu = parca[2][0] == '1';
        if (uz[3]) {
            a->en_az_var = 1;
            a->en_az = strtod(parca[3], NULL);
        }
        if (uz[4]) {
            a->en_fazla_var = 1;
            a->en_fazla = strtod(parca[4], NULL);
        }
        p = *son ? son + 1 : son;
    }
    m->sonraki = model_bilgileri;
    model_bilgileri = m;
    return m;
}

#define ORNEK(n) ((Liste *)(intptr_t)(n))
/* i. alan (0: kimlik) */
#define ALAN(n, i) (ORNEK(n)->ogeler[2 + (i)])

static ModelBilgisi *nesne_bilgisi(int64_t n) { return model_bilgisi(M(ORNEK(n)->ogeler[0])); }

static int64_t alan_sirasi(ModelBilgisi *m, const char *ad) {
    for (int64_t i = 0; i < m->alan_sayisi; i++)
        if (!strcmp(m->alanlar[i].ad, ad)) return i;
    return -1;
}

static void nesne_denetle(int64_t n, int64_t yuva, int64_t satir) {
    if (!n) hata(satir, "boş bir model değerinin alanı kullanılamaz (değişkene henüz değer atanmamış olabilir)");
    if (yuva < 0 || yuva >= ORNEK(n)->uzunluk) hata(satir, "model alanı bulunamadı");
}

int64_t ohc_alan_al(int64_t n, int64_t yuva, int64_t satir) {
    nesne_denetle(n, yuva, satir);
    return ORNEK(n)->ogeler[yuva];
}

void ohc_alan_koy(int64_t n, int64_t yuva, int64_t d, int64_t satir) {
    nesne_denetle(n, yuva, satir);
    ORNEK(n)->ogeler[yuva] = d;
}

/* Bir tipin sıfır değeri (yeni listeler ve sözlükler her seferinde ayrı). */
static int64_t sifir_deger(int64_t kod) {
    switch (kod % 8) {
    case KOD_METIN: return D("");
    case KOD_ONDALIK: return bitlere(0.0);
    case 4: return ohc_liste_yeni();
    case 5: {
        int64_t s = ohc_sozluk_yeni();
        ((Sozluk *)(intptr_t)s)->anahtar_kodu = (kod / 8) % 2;
        return s;
    }
    default: return 0;
    }
}

/* Tüm alanları sıfır değerinde yeni bir nesne. */
static int64_t nesne_yeni(const char *tanim) {
    ModelBilgisi *m = model_bilgisi(tanim);
    int64_t n = ohc_liste_yeni();
    ohc_liste_ekle(n, D(tanim));
    ohc_liste_ekle(n, 0);
    for (int64_t i = 0; i < m->alan_sayisi; i++) ohc_liste_ekle(n, sifir_deger(m->alanlar[i].kod));
    return n;
}

/* Ürün(kimlik: 1, ad: "Kalem", fiyat: 12.5) */
static void model_bicimle(Tampon *t, int64_t d) {
    if (!d) {
        t_yaz(t, "boş");
        return;
    }
    ModelBilgisi *m = nesne_bilgisi(d);
    t_yaz(t, m->ad);
    t_yaz(t, "(");
    for (int64_t i = 0; i < m->alan_sayisi; i++) {
        if (i) t_yaz(t, ", ");
        t_yaz(t, m->alanlar[i].ad);
        t_yaz(t, ": ");
        bicimle(t, ALAN(d, i), m->alanlar[i].kod, 1);
    }
    t_yaz(t, ")");
}

/* ---------------------------------------------------------------------- */
/* JSON                                                                    */
/* ---------------------------------------------------------------------- */

static void json_metin(Tampon *t, const char *s) {
    t_yaz(t, "\"");
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        switch (*p) {
        case '"': t_yaz(t, "\\\""); break;
        case '\\': t_yaz(t, "\\\\"); break;
        case '\n': t_yaz(t, "\\n"); break;
        case '\r': t_yaz(t, "\\r"); break;
        case '\t': t_yaz(t, "\\t"); break;
        default:
            if (*p < 0x20) {
                char b[8];
                snprintf(b, sizeof b, "\\u%04x", *p);
                t_yaz(t, b);
            } else {
                t_ekle(t, (const char *)p, 1);
            }
        }
    }
    t_yaz(t, "\"");
}

/* `bosluklu`: ", " ve ": " ayraçları (veri dosyaları okunaklı olsun diye). */
static void json_yaz(Tampon *t, int64_t d, int64_t kod, int bosluklu) {
    const char *virgul = bosluklu ? ", " : ",", *iki_nokta = bosluklu ? ": " : ":";
    char k[64];
    switch (kod % 8) {
    case KOD_SAYI:
        snprintf(k, sizeof k, "%" PRId64, d);
        t_yaz(t, k);
        break;
    case KOD_METIN: json_metin(t, M(d)); break;
    case KOD_MANTIK: t_yaz(t, d ? "true" : "false"); break;
    case KOD_ONDALIK: {
        double x = ondalik(d);
        if (x != x || x - x != 0) {
            t_yaz(t, "null");
        } else {
            ondalik_bicimle(x, k, sizeof k);
            t_yaz(t, k);
        }
        break;
    }
    case 4: {
        Liste *l = (Liste *)(intptr_t)d;
        t_yaz(t, "[");
        for (int64_t i = 0; l && i < l->uzunluk; i++) {
            if (i) t_yaz(t, virgul);
            json_yaz(t, l->ogeler[i], kod / 8, bosluklu);
        }
        t_yaz(t, "]");
        break;
    }
    case 5: {
        Sozluk *s = (Sozluk *)(intptr_t)d;
        int64_t ak = (kod / 8) % 2, dk = (kod / 8) / 2;
        t_yaz(t, "{");
        for (int64_t i = 0; s && i < s->uzunluk; i++) {
            if (i) t_yaz(t, virgul);
            if (ak == KOD_METIN) {
                json_metin(t, M(s->anahtarlar[i]));
            } else {
                snprintf(k, sizeof k, "\"%" PRId64 "\"", s->anahtarlar[i]);
                t_yaz(t, k);
            }
            t_yaz(t, iki_nokta);
            json_yaz(t, s->degerler[i], dk, bosluklu);
        }
        t_yaz(t, "}");
        break;
    }
    case KOD_MODEL: {
        if (!d) {
            t_yaz(t, "null");
            break;
        }
        ModelBilgisi *m = nesne_bilgisi(d);
        t_yaz(t, "{");
        for (int64_t i = 0; i < m->alan_sayisi; i++) {
            if (i) t_yaz(t, virgul);
            json_metin(t, m->alanlar[i].ad);
            t_yaz(t, iki_nokta);
            json_yaz(t, ALAN(d, i), m->alanlar[i].kod, bosluklu);
        }
        t_yaz(t, "}");
        break;
    }
    }
}

int64_t ohc_json(int64_t d, int64_t kod) {
    Tampon t = {0};
    json_yaz(&t, d, kod, 0);
    return t_metin(&t);
}

typedef struct {
    const char *p;
    const char *hata;
    int derinlik;
} Json;

static void j_bosluk(Json *j) {
    while (*j->p == ' ' || *j->p == '\t' || *j->p == '\n' || *j->p == '\r') j->p++;
}

static int j_hata(Json *j, const char *m) {
    if (!j->hata) j->hata = m;
    return 0;
}

static int hex_deger(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

static uint32_t j_hex4(Json *j) {
    uint32_t u = 0;
    for (int i = 0; i < 4; i++) {
        int h = hex_deger(j->p[i]);
        if (h < 0) {
            j_hata(j, "geçersiz \\u kaçışı");
            return 0;
        }
        u = u * 16 + (uint32_t)h;
    }
    j->p += 4;
    return u;
}

/* Tırnaklı bir JSON metnini çözüp tampona yazar. */
static int j_metin(Json *j, Tampon *t) {
    if (*j->p != '"') return j_hata(j, "metin bekleniyordu");
    j->p++;
    t_ekle(t, "", 0);
    for (;;) {
        unsigned char c = (unsigned char)*j->p;
        if (!c) return j_hata(j, "metin kapatılmamış");
        j->p++;
        if (c == '"') break;
        if (c != '\\') {
            t_ekle(t, (const char *)&c, 1);
            continue;
        }
        char e = *j->p++;
        switch (e) {
        case '"': t_ekle(t, "\"", 1); break;
        case '\\': t_ekle(t, "\\", 1); break;
        case '/': t_ekle(t, "/", 1); break;
        case 'b': t_ekle(t, "\b", 1); break;
        case 'f': t_ekle(t, "\f", 1); break;
        case 'n': t_ekle(t, "\n", 1); break;
        case 'r': t_ekle(t, "\r", 1); break;
        case 't': t_ekle(t, "\t", 1); break;
        case 'u': {
            uint32_t u = j_hex4(j);
            if (j->hata) return 0;
            if (u >= 0xD800 && u < 0xDC00 && j->p[0] == '\\' && j->p[1] == 'u') {
                j->p += 2;
                uint32_t v = j_hex4(j);
                if (j->hata) return 0;
                if (v >= 0xDC00 && v < 0xE000) u = 0x10000 + ((u - 0xD800) << 10) + (v - 0xDC00);
            }
            u8_yaz(t, u);
            break;
        }
        default: return j_hata(j, "geçersiz kaçış dizisi");
        }
    }
    return 1;
}

/* Herhangi bir değeri atlar. */
static int j_atla(Json *j) {
    j_bosluk(j);
    if (++j->derinlik > 200) return j_hata(j, "çok derin iç içe değer");
    int ok = 1;
    char c = *j->p;
    if (c == '"') {
        Tampon t = {0};
        ok = j_metin(j, &t);
        free(t.v);
    } else if (c == '[' || c == '{') {
        char kapa = c == '[' ? ']' : '}';
        j->p++;
        j_bosluk(j);
        if (*j->p == kapa) {
            j->p++;
        } else {
            for (;;) {
                if (c == '{') {
                    Tampon t = {0};
                    j_bosluk(j);
                    ok = j_metin(j, &t);
                    free(t.v);
                    if (!ok) break;
                    j_bosluk(j);
                    if (*j->p != ':') {
                        ok = j_hata(j, "':' bekleniyordu");
                        break;
                    }
                    j->p++;
                }
                if (!(ok = j_atla(j))) break;
                j_bosluk(j);
                if (*j->p == ',') {
                    j->p++;
                    continue;
                }
                if (*j->p == kapa) {
                    j->p++;
                    break;
                }
                ok = j_hata(j, "',' bekleniyordu");
                break;
            }
        }
    } else if (!strncmp(j->p, "true", 4) || !strncmp(j->p, "null", 4)) {
        j->p += 4;
    } else if (!strncmp(j->p, "false", 5)) {
        j->p += 5;
    } else if (c == '-' || (c >= '0' && c <= '9')) {
        char *son;
        strtod(j->p, &son);
        j->p = son;
    } else {
        ok = j_hata(j, "beklenmeyen karakter");
    }
    j->derinlik--;
    return ok;
}

static int64_t j_deger(Json *j, int64_t kod, ModelBilgisi *mb);

/* Beklenen tipte bir değer okur; tip uyuşmazsa değeri atlar ve sıfır değer verir. */
static int64_t j_deger(Json *j, int64_t kod, ModelBilgisi *mb) {
    j_bosluk(j);
    char c = *j->p;
    switch (kod % 8) {
    case KOD_SAYI:
    case KOD_ONDALIK:
        if (c == '-' || (c >= '0' && c <= '9')) {
            char *son;
            double x = strtod(j->p, &son);
            int64_t n = 0;
            if (kod % 8 == KOD_SAYI) {
                n = strtoll(j->p, NULL, 10);
                if (strpbrk(j->p, ".eE") && strpbrk(j->p, ".eE") < son) n = (int64_t)x;
            }
            j->p = son;
            return kod % 8 == KOD_SAYI ? n : bitlere(x);
        }
        break;
    case KOD_METIN:
        if (c == '"') {
            Tampon t = {0};
            if (!j_metin(j, &t)) {
                free(t.v);
                return D("");
            }
            return t_metin(&t);
        }
        break;
    case KOD_MANTIK:
        if (!strncmp(j->p, "true", 4)) {
            j->p += 4;
            return 1;
        }
        if (!strncmp(j->p, "false", 5)) {
            j->p += 5;
            return 0;
        }
        break;
    case 4:
        if (c == '[') {
            int64_t l = ohc_liste_yeni();
            j->p++;
            j_bosluk(j);
            if (*j->p == ']') {
                j->p++;
                return l;
            }
            for (;;) {
                int64_t d = j_deger(j, kod / 8, mb);
                if (j->hata) return l;
                ohc_liste_ekle(l, d);
                j_bosluk(j);
                if (*j->p == ',') {
                    j->p++;
                    continue;
                }
                if (*j->p == ']') {
                    j->p++;
                    return l;
                }
                j_hata(j, "',' ya da ']' bekleniyordu");
                return l;
            }
        }
        break;
    case 5:
    case KOD_MODEL:
        if (c == '{' && (kod % 8 == 5 || mb)) {
            int model = kod % 8 == KOD_MODEL;
            int64_t sonuc = model ? nesne_yeni(mb->tanim) : sifir_deger(kod);
            int64_t ak = (kod / 8) % 2, dk = (kod / 8) / 2;
            j->p++;
            j_bosluk(j);
            if (*j->p == '}') {
                j->p++;
                return sonuc;
            }
            for (;;) {
                Tampon a = {0};
                j_bosluk(j);
                if (!j_metin(j, &a)) {
                    free(a.v);
                    return sonuc;
                }
                j_bosluk(j);
                if (*j->p != ':') {
                    free(a.v);
                    j_hata(j, "':' bekleniyordu");
                    return sonuc;
                }
                j->p++;
                if (model) {
                    int64_t i = alan_sirasi(mb, a.v);
                    free(a.v);
                    if (i < 0) {
                        if (!j_atla(j)) return sonuc;
                    } else {
                        int64_t d = j_deger(j, mb->alanlar[i].kod, NULL);
                        ALAN(sonuc, i) = d;
                    }
                } else {
                    int64_t anahtar = ak == KOD_METIN ? t_metin(&a) : (int64_t)strtoll(a.v ? a.v : "0", NULL, 10);
                    if (ak != KOD_METIN) free(a.v);
                    int64_t d = j_deger(j, dk, NULL);
                    ohc_sozluk_koy(sonuc, anahtar, d, ak);
                }
                if (j->hata) return sonuc;
                j_bosluk(j);
                if (*j->p == ',') {
                    j->p++;
                    continue;
                }
                if (*j->p == '}') {
                    j->p++;
                    return sonuc;
                }
                j_hata(j, "',' ya da '}' bekleniyordu");
                return sonuc;
            }
        }
        break;
    }
    /* Beklenmeyen tip ya da null: atla, sıfır değer ver. */
    j_atla(j);
    return sifir_deger(kod);
}

/* ---------------------------------------------------------------------- */
/* Veri deposu: her model için veri/<Model>.json                            */
/* ---------------------------------------------------------------------- */

static const char *veri_klasoru(void) {
    const char *v = getenv("ORHUNCA_VERI");
    return (v && *v) ? v : "veri";
}

static void klasor_olustur(const char *yol) {
#ifdef _WIN32
    wchar_t w[1024];
    if (MultiByteToWideChar(CP_UTF8, 0, yol, -1, w, 1024)) _wmkdir(w);
#else
    mkdir(yol, 0777);
#endif
}

static int dosya_tasi(const char *eski, const char *yeni) {
#ifdef _WIN32
    wchar_t a[1024], b[1024];
    if (!MultiByteToWideChar(CP_UTF8, 0, eski, -1, a, 1024)) return 0;
    if (!MultiByteToWideChar(CP_UTF8, 0, yeni, -1, b, 1024)) return 0;
    return MoveFileExW(a, b, MOVEFILE_REPLACE_EXISTING) != 0;
#else
    return rename(eski, yeni) == 0;
#endif
}

static char *veri_dosyasi(ModelBilgisi *m) {
    Tampon t = {0};
    t_yaz(&t, veri_klasoru());
    t_yaz(&t, "/");
    t_yaz(&t, m->ad);
    t_yaz(&t, ".json");
    return t.v;
}

/* Modelin tüm kayıtlarını okur (dosya yoksa boş liste). */
static int64_t kayitlari_oku(ModelBilgisi *m, int64_t satir) {
    char *yol = veri_dosyasi(m);
    int64_t liste = ohc_liste_yeni();
    FILE *f = dosya_ac(yol, "rb");
    if (!f) {
        free(yol);
        return liste;
    }
    Tampon t = {0};
    char b[8192];
    size_t n;
    while ((n = fread(b, 1, sizeof b, f)) > 0) t_ekle(&t, b, n);
    fclose(f);
    Json j = {t.v ? t.v : "", NULL, 0};
    j_bosluk(&j);
    if (*j.p) {
        if (*j.p != '[') {
            j_hata(&j, "kayıtlar bir JSON dizisi ([...]) olmalı");
        } else {
            j.p++;
            j_bosluk(&j);
            if (*j.p == ']') {
                j.p++;
            } else {
                for (;;) {
                    int64_t k = j_deger(&j, KOD_MODEL, m);
                    if (j.hata) break;
                    ohc_liste_ekle(liste, k);
                    j_bosluk(&j);
                    if (*j.p == ',') {
                        j.p++;
                        continue;
                    }
                    if (*j.p == ']') {
                        j.p++;
                        break;
                    }
                    j_hata(&j, "',' ya da ']' bekleniyordu");
                    break;
                }
            }
        }
    }
    if (j.hata) {
        char mesaj[600];
        snprintf(mesaj, sizeof mesaj, "'%.400s' dosyası bozuk: %s", yol, j.hata);
        free(t.v);
        free(yol);
        hata(satir, mesaj);
    }
    free(t.v);
    free(yol);
    return liste;
}

static void kayitlari_yaz(ModelBilgisi *m, int64_t liste, int64_t satir) {
    klasor_olustur(veri_klasoru());
    char *yol = veri_dosyasi(m);
    Tampon t = {0};
    t_yaz(&t, "[");
    Liste *l = ORNEK(liste);
    for (int64_t i = 0; i < l->uzunluk; i++) {
        t_yaz(&t, i ? ",\n  " : "\n  ");
        json_yaz(&t, l->ogeler[i], KOD_MODEL, 1);
    }
    t_yaz(&t, l->uzunluk ? "\n]\n" : "]\n");
    Tampon g = {0};
    t_yaz(&g, yol);
    t_yaz(&g, ".yeni");
    FILE *f = dosya_ac(g.v, "wb");
    int ok = f && fwrite(t.v, 1, t.n, f) == t.n;
    if (f && fclose(f) != 0) ok = 0;
    if (ok) ok = dosya_tasi(g.v, yol);
    if (!ok) {
        char mesaj[600];
        snprintf(mesaj, sizeof mesaj, "'%.400s' yazılamadı: %s", yol, strerror(errno));
        free(t.v);
        free(g.v);
        free(yol);
        hata(satir, mesaj);
    }
    free(t.v);
    free(g.v);
    free(yol);
}

static int64_t kayit_sirasi(int64_t liste, int64_t kimlik) {
    Liste *l = ORNEK(liste);
    for (int64_t i = 0; i < l->uzunluk; i++)
        if (ALAN(l->ogeler[i], 0) == kimlik) return i;
    return -1;
}

int64_t ohc_model_hepsi(int64_t tanim, int64_t satir) { return kayitlari_oku(model_bilgisi(M(tanim)), satir); }

/* Kimliği verilen kaydı döndürür; yoksa `varsayilan` nesnesini (kimlik 0). */
int64_t ohc_model_yukle(int64_t varsayilan, int64_t kimlik, int64_t satir) {
    int64_t liste = kayitlari_oku(nesne_bilgisi(varsayilan), satir);
    int64_t i = kayit_sirasi(liste, kimlik);
    return i < 0 ? varsayilan : ORNEK(liste)->ogeler[i];
}

int64_t ohc_model_var(int64_t tanim, int64_t kimlik, int64_t satir) {
    return kayit_sirasi(kayitlari_oku(model_bilgisi(M(tanim)), satir), kimlik) >= 0;
}

int64_t ohc_model_sil(int64_t tanim, int64_t kimlik, int64_t satir) {
    ModelBilgisi *m = model_bilgisi(M(tanim));
    int64_t liste = kayitlari_oku(m, satir);
    int64_t i = kayit_sirasi(liste, kimlik);
    if (i < 0) return 0;
    ohc_liste_sil(liste, i, satir);
    kayitlari_yaz(m, liste, satir);
    return 1;
}

/* Yeni nesneye (kimlik 0) sıradaki kimliği verir; var olanı günceller. */
int64_t ohc_model_kaydet(int64_t n, int64_t satir) {
    if (!n) hata(satir, "boş bir model değeri kaydedilemez");
    ModelBilgisi *m = nesne_bilgisi(n);
    int64_t liste = kayitlari_oku(m, satir);
    int64_t kimlik = ALAN(n, 0);
    int64_t i = kimlik > 0 ? kayit_sirasi(liste, kimlik) : -1;
    if (kimlik <= 0) {
        kimlik = 0;
        Liste *l = ORNEK(liste);
        for (int64_t j = 0; j < l->uzunluk; j++)
            if (ALAN(l->ogeler[j], 0) > kimlik) kimlik = ALAN(l->ogeler[j], 0);
        kimlik++;
        ALAN(n, 0) = kimlik;
    }
    if (i >= 0)
        ORNEK(liste)->ogeler[i] = n;
    else
        ohc_liste_ekle(liste, n);
    kayitlari_yaz(m, liste, satir);
    return kimlik;
}

/* ---------------------------------------------------------------------- */
/* Doğrulama ve form bağlama                                               */
/* ---------------------------------------------------------------------- */

/* "doğum_tarihi" → "Doğum tarihi" */
static void gorunen_ad(Tampon *t, const char *ad) {
    const char *p = ad;
    int ilk = 1;
    while (*p) {
        uint32_t c = u8_oku(&p);
        if (c == '_') c = ' ';
        if (ilk) c = buyuk(c);
        ilk = 0;
        u8_yaz(t, c);
    }
}

/* 100 → "100", 0.5 → "0,5" */
static void sayi_yaz(Tampon *t, double x) {
    char k[64];
    if (x == (double)(int64_t)x && fabs(x) < 1e15) {
        snprintf(k, sizeof k, "%" PRId64, (int64_t)x);
    } else {
        ondalik_bicimle(x, k, sizeof k);
        for (char *c = k; *c; c++)
            if (*c == '.') *c = ',';
    }
    t_yaz(t, k);
}

static void kural_mesaji(Tampon *t, AlanBilgisi *a, const char *on, double sinir, const char *son) {
    gorunen_ad(t, a->ad);
    t_yaz(t, on);
    sayi_yaz(t, sinir);
    t_yaz(t, son);
}

int64_t ohc_model_hatalar(int64_t n) {
    int64_t sonuc = ohc_liste_yeni();
    if (!n) return sonuc;
    ModelBilgisi *m = nesne_bilgisi(n);
    int64_t baglama = ORNEK(n)->ogeler[1];
    for (int64_t i = 1; i < m->alan_sayisi; i++) {
        AlanBilgisi *a = &m->alanlar[i];
        if (baglama && ohc_sozluk_icerir(baglama, i, KOD_SAYI)) {
            ohc_liste_ekle(sonuc, ohc_sozluk_al(baglama, i, KOD_SAYI, 0));
            continue;
        }
        int64_t d = ALAN(n, i);
        Tampon t = {0};
        switch (a->kod % 8) {
        case KOD_METIN: {
            const char *s = M(d);
            while (*s && bosluk(*s)) s++;
            double uz = (double)ohc_metin_uzunluk(d);
            if (!*s) {
                if (a->zorunlu) {
                    gorunen_ad(&t, a->ad);
                    t_yaz(&t, " boş bırakılamaz");
                }
            } else if (a->en_az_var && uz < a->en_az) {
                kural_mesaji(&t, a, " en az ", a->en_az, " karakter olmalı");
            } else if (a->en_fazla_var && uz > a->en_fazla) {
                kural_mesaji(&t, a, " en fazla ", a->en_fazla, " karakter olabilir");
            }
            break;
        }
        case KOD_SAYI:
        case KOD_ONDALIK: {
            double x = a->kod % 8 == KOD_SAYI ? (double)d : ondalik(d);
            if (a->en_az_var && x < a->en_az)
                kural_mesaji(&t, a, " en az ", a->en_az, " olmalı");
            else if (a->en_fazla_var && x > a->en_fazla)
                kural_mesaji(&t, a, " en fazla ", a->en_fazla, " olabilir");
            break;
        }
        case KOD_MANTIK:
            if (a->zorunlu && !d) {
                gorunen_ad(&t, a->ad);
                t_yaz(&t, " işaretlenmeli");
            }
            break;
        case 4:
        case 5: {
            double uz = (double)(a->kod % 8 == 4 ? ohc_liste_uzunluk(d) : ohc_sozluk_uzunluk(d));
            if (a->zorunlu && uz == 0) {
                gorunen_ad(&t, a->ad);
                t_yaz(&t, " boş bırakılamaz");
            } else if (a->en_az_var && uz < a->en_az) {
                kural_mesaji(&t, a, " en az ", a->en_az, " öğe içermeli");
            } else if (a->en_fazla_var && uz > a->en_fazla) {
                kural_mesaji(&t, a, " en fazla ", a->en_fazla, " öğe içerebilir");
            }
            break;
        }
        }
        if (t.n)
            ohc_liste_ekle(sonuc, t_metin(&t));
        else
            free(t.v);
    }
    return sonuc;
}

int64_t ohc_model_gecerli(int64_t n) { return ohc_liste_uzunluk(ohc_model_hatalar(n)) == 0; }

static int dogru_mu(const char *v) {
    const char *evet[] = {"on", "true", "doğru", "1", "evet", "yes", "Doğru", "Evet", "True", "On"};
    for (size_t i = 0; i < sizeof evet / sizeof *evet; i++)
        if (!strcmp(v, evet[i])) return 1;
    return 0;
}

/* İsteğin bir alanı (İstek modelinin tanımında adıyla aranır). */
static int64_t istek_alani(int64_t istek, const char *ad) {
    if (!istek) return 0;
    int64_t i = alan_sirasi(nesne_bilgisi(istek), ad);
    return i < 0 ? 0 : ALAN(istek, i);
}

/* Formdaki değerleri nesnenin alanlarına yazar; çevrilemeyenler bağlama hatası olur. */
void ohc_model_doldur(int64_t n, int64_t istek) {
    ModelBilgisi *m = nesne_bilgisi(n);
    int64_t form = istek_alani(istek, "form");
    if (!form) return;
    for (int64_t i = 1; i < m->alan_sayisi; i++) {
        AlanBilgisi *a = &m->alanlar[i];
        int64_t anahtar = D(a->ad);
        if (!ohc_sozluk_icerir(form, anahtar, KOD_METIN)) continue;
        int64_t deger = ohc_sozluk_al(form, anahtar, KOD_METIN, 0);
        const char *v = M(deger);
        const char *bas = v;
        while (*bas && bosluk(*bas)) bas++;
        const char *mesaj = NULL;
        switch (a->kod % 8) {
        case KOD_METIN: ALAN(n, i) = deger; break;
        case KOD_SAYI: {
            int64_t x;
            if (!*bas) {
                if (a->zorunlu) mesaj = " boş bırakılamaz";
            } else if (sayi_oku(v, &x)) {
                ALAN(n, i) = x;
            } else {
                mesaj = " bir tam sayı olmalı";
            }
            break;
        }
        case KOD_ONDALIK: {
            double x;
            if (!*bas) {
                if (a->zorunlu) mesaj = " boş bırakılamaz";
            } else if (ondalik_oku(v, &x)) {
                ALAN(n, i) = bitlere(x);
            } else {
                mesaj = " bir sayı olmalı";
            }
            break;
        }
        case KOD_MANTIK: ALAN(n, i) = dogru_mu(bas); break;
        default: break;
        }
        if (mesaj) {
            int64_t baglama = ORNEK(n)->ogeler[1];
            if (!baglama) {
                baglama = ohc_sozluk_yeni();
                ORNEK(n)->ogeler[1] = baglama;
            }
            Tampon t = {0};
            gorunen_ad(&t, a->ad);
            t_yaz(&t, mesaj);
            ohc_sozluk_koy(baglama, i, t_metin(&t), KOD_SAYI);
        }
    }
}

/* ---------------------------------------------------------------------- */
/* Metin yardımcıları: HTML kaçırma, para biçimi, URL kodlama              */
/* ---------------------------------------------------------------------- */

static void html_yaz(Tampon *t, const char *s) {
    for (; *s; s++) {
        switch (*s) {
        case '&': t_yaz(t, "&amp;"); break;
        case '<': t_yaz(t, "&lt;"); break;
        case '>': t_yaz(t, "&gt;"); break;
        case '"': t_yaz(t, "&quot;"); break;
        case '\'': t_yaz(t, "&#39;"); break;
        default: t_ekle(t, s, 1);
        }
    }
}

int64_t ohc_kacir(int64_t m) {
    if (!strpbrk(M(m), "&<>\"'")) return m;
    Tampon t = {0};
    html_yaz(&t, M(m));
    return t_metin(&t);
}

/* 1234.5 → "1.234,50" */
int64_t ohc_para(int64_t d) {
    double x = ondalik(d);
    char k[64];
    if (x != x || fabs(x) > 9e15) {
        ondalik_bicimle(x, k, sizeof k);
        return metin_yap(k, strlen(k));
    }
    int eksi = x < 0;
    long long kurus = llround(fabs(x) * 100.0);
    char rakam[32];
    snprintf(rakam, sizeof rakam, "%lld", kurus / 100);
    Tampon t = {0};
    if (eksi && kurus) t_yaz(&t, "-");
    size_t n = strlen(rakam);
    for (size_t i = 0; i < n; i++) {
        if (i && (n - i) % 3 == 0) t_yaz(&t, ".");
        t_ekle(&t, &rakam[i], 1);
    }
    snprintf(k, sizeof k, ",%02lld", kurus % 100);
    t_yaz(&t, k);
    return t_metin(&t);
}

static void url_kodla(Tampon *t, const char *s, const char *serbest) {
    static const char *hex = "0123456789ABCDEF";
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        if ((*p >= 'a' && *p <= 'z') || (*p >= 'A' && *p <= 'Z') || (*p >= '0' && *p <= '9') ||
            strchr(serbest, *p)) {
            t_ekle(t, (const char *)p, 1);
        } else {
            char b[3] = {'%', hex[*p >> 4], hex[*p & 15]};
            t_ekle(t, b, 3);
        }
    }
}

int64_t ohc_url_kodla(int64_t m) {
    Tampon t = {0};
    url_kodla(&t, M(m), "-_.~");
    return t_metin(&t);
}

/* ====================================================================== */
/* Web sunucusu                                                            */
/* ====================================================================== */

#ifdef _WIN32
typedef SOCKET Soket;
#define GECERSIZ_SOKET INVALID_SOCKET
#define soket_kapat closesocket
#else
typedef int Soket;
#define GECERSIZ_SOKET (-1)
#define soket_kapat close
#endif

typedef struct {
    const char *yontem;
    const char *kalip;
    int64_t (*islev)(int64_t);
} WebYolu;

static WebYolu *web_yollari;
static int64_t web_yolu_sayisi, web_yolu_kap;

void ohc_web_yol(int64_t yontem, int64_t kalip, int64_t islev) {
    if (web_yolu_sayisi == web_yolu_kap) {
        web_yolu_kap = web_yolu_kap ? web_yolu_kap * 2 : 16;
        web_yollari = ham_buyut(web_yollari, sizeof(WebYolu) * (size_t)web_yolu_kap);
    }
    WebYolu *y = &web_yollari[web_yolu_sayisi++];
    y->yontem = M(yontem);
    y->kalip = M(kalip);
    y->islev = (int64_t(*)(int64_t))(intptr_t)islev;
}

static void yuzde_coz(Tampon *t, const char *s, size_t n, int arti_bosluk) {
    t_ekle(t, "", 0);
    for (size_t i = 0; i < n; i++) {
        if (s[i] == '%' && i + 2 < n && hex_deger(s[i + 1]) >= 0 && hex_deger(s[i + 2]) >= 0) {
            char c = (char)(hex_deger(s[i + 1]) * 16 + hex_deger(s[i + 2]));
            t_ekle(t, &c, 1);
            i += 2;
        } else if (arti_bosluk && s[i] == '+') {
            t_ekle(t, " ", 1);
        } else {
            t_ekle(t, &s[i], 1);
        }
    }
}

/* a=1&b=iki+kelime → sözlük<metin, metin> */
static void form_coz(int64_t sozluk, const char *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        size_t j = i;
        while (j < n && s[j] != '&') j++;
        size_t e = i;
        while (e < j && s[e] != '=') e++;
        if (j > i) {
            Tampon a = {0}, d = {0};
            yuzde_coz(&a, s + i, e - i, 1);
            if (e < j) yuzde_coz(&d, s + e + 1, j - e - 1, 1);
            int64_t am = t_metin(&a);
            int64_t dm = t_metin(&d);
            ohc_sozluk_koy(sozluk, am, dm, KOD_METIN);
        }
        i = j + 1;
    }
}

/* JSON nesnesi gövdesinin üst düzey değerleri forma metin olarak girer. */
static void json_formu(int64_t sozluk, const char *govde) {
    Json j = {govde, NULL, 0};
    j_bosluk(&j);
    if (*j.p != '{') return;
    j.p++;
    for (;;) {
        j_bosluk(&j);
        if (*j.p == '}') return;
        Tampon a = {0};
        if (!j_metin(&j, &a)) {
            free(a.v);
            return;
        }
        j_bosluk(&j);
        if (*j.p != ':') {
            free(a.v);
            return;
        }
        j.p++;
        j_bosluk(&j);
        if (*j.p == '"') {
            Tampon d = {0};
            if (!j_metin(&j, &d)) {
                free(a.v);
                free(d.v);
                return;
            }
            int64_t am = t_metin(&a);
            int64_t dm = t_metin(&d);
            ohc_sozluk_koy(sozluk, am, dm, KOD_METIN);
        } else {
            const char *b = j.p;
            int duz = *j.p == '-' || (*j.p >= '0' && *j.p <= '9') || *j.p == 't' || *j.p == 'f';
            if (!j_atla(&j)) {
                free(a.v);
                return;
            }
            if (duz) {
                int64_t am = t_metin(&a);
                int64_t dm = metin_yap(b, (size_t)(j.p - b));
                ohc_sozluk_koy(sozluk, am, dm, KOD_METIN);
            } else {
                free(a.v);
            }
        }
        j_bosluk(&j);
        if (*j.p != ',') return;
        j.p++;
    }
}

static int tam_sayi_mi(const char *s, size_t n) {
    size_t i = (n && s[0] == '-') ? 1 : 0;
    if (i == n || n - i > 18) return 0;
    for (; i < n; i++)
        if (s[i] < '0' || s[i] > '9') return 0;
    return 1;
}

/* Kalıp ile yol eşleşirse sabit parça sayısını (+1) döndürür; parametreleri
 * `parametreler` sözlüğüne yazar (0 ise yazmaz). */
static int yol_eslesir(const char *kalip, const char *yol, int64_t parametreler) {
    const char *k = kalip + 1, *y = yol + 1;
    int sabit = 1;
    for (;;) {
        const char *ks = strchr(k, '/'), *ys = strchr(y, '/');
        size_t kn = ks ? (size_t)(ks - k) : strlen(k), yn = ys ? (size_t)(ys - y) : strlen(y);
        if (kn > 1 && k[0] == '{' && k[kn - 1] == '}') {
            const char *ic = k + 1;
            size_t icn = kn - 2;
            const char *iki = memchr(ic, ':', icn);
            size_t adn = iki ? (size_t)(iki - ic) : icn;
            if (yn == 0) return 0;
            if (iki && !tam_sayi_mi(y, yn)) return 0;
            if (parametreler) {
                int64_t am = metin_yap(ic, adn);
                int64_t dm = metin_yap(y, yn);
                ohc_sozluk_koy(parametreler, am, dm, KOD_METIN);
            }
        } else {
            if (kn != yn || memcmp(k, y, kn)) return 0;
            sabit++;
        }
        if (!ks && !ys) return sabit;
        if (!ks || !ys) return 0;
        k = ks + 1;
        y = ys + 1;
    }
}

static const char *durum_metni(int64_t d) {
    switch (d) {
    case 200: return "OK";
    case 201: return "Created";
    case 204: return "No Content";
    case 301: return "Moved Permanently";
    case 302: return "Found";
    case 303: return "See Other";
    case 304: return "Not Modified";
    case 307: return "Temporary Redirect";
    case 400: return "Bad Request";
    case 401: return "Unauthorized";
    case 403: return "Forbidden";
    case 404: return "Not Found";
    case 405: return "Method Not Allowed";
    case 409: return "Conflict";
    case 413: return "Payload Too Large";
    case 422: return "Unprocessable Entity";
    case 431: return "Request Header Fields Too Large";
    case 500: return "Internal Server Error";
    case 501: return "Not Implemented";
    default: return d < 400 ? "OK" : "Error";
    }
}

static int tumunu_gonder(Soket s, const char *v, size_t n) {
    while (n) {
        int k = send(s, v, n > 1048576 ? 1048576 : (int)n, 0);
        if (k <= 0) return 0;
        v += k;
        n -= (size_t)k;
    }
    return 1;
}

/* Başlık değerlerinde satır sonu olamaz (başlık enjeksiyonu). */
static void baslik_degeri(Tampon *t, const char *v) {
    for (; *v; v++)
        if (*v != '\r' && *v != '\n') t_ekle(t, v, 1);
}

static void yanit_gonder(Soket s, int64_t durum, const char *tur, const char *govde, size_t govde_n, const char *konum,
                         int64_t basliklar, int bas_istegi) {
    Tampon t = {0};
    char k[256];
    snprintf(k, sizeof k, "HTTP/1.1 %" PRId64 " %s\r\n", durum, durum_metni(durum));
    t_yaz(&t, k);
    if (tur && *tur && durum != 204) {
        t_yaz(&t, "Content-Type: ");
        baslik_degeri(&t, tur);
        t_yaz(&t, "\r\n");
    }
    snprintf(k, sizeof k, "Content-Length: %zu\r\n", durum == 204 ? (size_t)0 : govde_n);
    t_yaz(&t, k);
    if (konum && *konum) {
        /* Türkçe karakterli adresler yüzde kodlanır. */
        Tampon u = {0};
        url_kodla(&u, konum, "-_.~/?#[]@!$&'()*+,;=:%");
        t_yaz(&t, "Location: ");
        baslik_degeri(&t, u.v ? u.v : "");
        t_yaz(&t, "\r\n");
        free(u.v);
    }
    Sozluk *b = (Sozluk *)(intptr_t)basliklar;
    for (int64_t i = 0; b && i < b->uzunluk; i++) {
        baslik_degeri(&t, M(b->anahtarlar[i]));
        t_yaz(&t, ": ");
        baslik_degeri(&t, M(b->degerler[i]));
        t_yaz(&t, "\r\n");
    }
    t_yaz(&t, "Connection: close\r\n\r\n");
    tumunu_gonder(s, t.v, t.n);
    if (!bas_istegi && govde_n && durum != 204) tumunu_gonder(s, govde, govde_n);
    free(t.v);
}

static void hata_sayfasi(Soket s, int64_t durum, const char *baslik, const char *ayrinti, int bas) {
    Tampon t = {0};
    t_yaz(&t, "<!DOCTYPE html><html lang=\"tr\"><head><meta charset=\"utf-8\">"
              "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>");
    html_yaz(&t, baslik);
    t_yaz(&t, "</title><style>body{font-family:system-ui,sans-serif;margin:0;padding:48px 24px;"
              "background:#f5f3ee;color:#1b1d21}main{max-width:720px;margin:auto}h1{font-size:22px;margin:0 0 12px}"
              "pre{white-space:pre-wrap;background:#1b1d21;color:#f5f3ee;padding:16px;border-radius:8px;"
              "font-size:13px}small{color:#6b7078}</style></head><body><main><h1>");
    html_yaz(&t, baslik);
    t_yaz(&t, "</h1>");
    if (ayrinti && *ayrinti) {
        t_yaz(&t, "<pre>");
        html_yaz(&t, ayrinti);
        t_yaz(&t, "</pre>");
    }
    t_yaz(&t, "<small>Orhunca web sunucusu</small></main></body></html>");
    yanit_gonder(s, durum, "text/html; charset=utf-8", t.v, t.n, NULL, 0, bas);
    free(t.v);
}

static const char *icerik_turu(const char *yol) {
    const char *nokta = strrchr(yol, '.');
    if (!nokta || strchr(nokta, '/')) return "application/octet-stream";
    char u[16];
    size_t i = 0;
    for (const char *p = nokta + 1; *p && i < sizeof u - 1; p++) u[i++] = (char)(*p >= 'A' && *p <= 'Z' ? *p + 32 : *p);
    u[i] = 0;
    static const char *turler[][2] = {
        {"html", "text/html; charset=utf-8"}, {"htm", "text/html; charset=utf-8"},
        {"css", "text/css; charset=utf-8"}, {"js", "text/javascript; charset=utf-8"},
        {"mjs", "text/javascript; charset=utf-8"}, {"json", "application/json; charset=utf-8"},
        {"svg", "image/svg+xml"}, {"png", "image/png"}, {"jpg", "image/jpeg"}, {"jpeg", "image/jpeg"},
        {"gif", "image/gif"}, {"webp", "image/webp"}, {"ico", "image/x-icon"}, {"avif", "image/avif"},
        {"txt", "text/plain; charset=utf-8"}, {"md", "text/plain; charset=utf-8"},
        {"woff2", "font/woff2"}, {"woff", "font/woff"}, {"ttf", "font/ttf"}, {"otf", "font/otf"},
        {"pdf", "application/pdf"}, {"mp4", "video/mp4"}, {"webm", "video/webm"}, {"mp3", "audio/mpeg"},
        {"wav", "audio/wav"}, {"wasm", "application/wasm"}, {"xml", "application/xml"}, {"csv", "text/csv; charset=utf-8"},
    };
    for (size_t k = 0; k < sizeof turler / sizeof *turler; k++)
        if (!strcmp(u, turler[k][0])) return turler[k][1];
    return "application/octet-stream";
}

/* 0: yok, 1: dosya, 2: klasör */
static int dosya_turu(const char *yol) {
#ifdef _WIN32
    wchar_t w[1024];
    if (!MultiByteToWideChar(CP_UTF8, 0, yol, -1, w, 1024)) return 0;
    DWORD a = GetFileAttributesW(w);
    if (a == INVALID_FILE_ATTRIBUTES) return 0;
    return (a & FILE_ATTRIBUTE_DIRECTORY) ? 2 : 1;
#else
    struct stat st;
    if (stat(yol, &st) != 0) return 0;
    return S_ISDIR(st.st_mode) ? 2 : S_ISREG(st.st_mode) ? 1 : 0;
#endif
}

/* statik/ klasöründen dosya gönderir; dosya yoksa 0 döndürür. */
static int statik_gonder(Soket s, const char *yol, int bas) {
    if (strstr(yol, "/..") || strchr(yol, '\\') || strstr(yol, "//")) return 0;
    Tampon d = {0};
    t_yaz(&d, "statik");
    t_yaz(&d, yol);
    int tur = dosya_turu(d.v);
    if (tur == 2) {
        if (d.v[d.n - 1] != '/') t_yaz(&d, "/");
        t_yaz(&d, "index.html");
        tur = dosya_turu(d.v);
    }
    if (tur != 1) {
        free(d.v);
        return 0;
    }
    FILE *f = dosya_ac(d.v, "rb");
    if (!f) {
        free(d.v);
        return 0;
    }
    Tampon icerik = {0};
    char b[16384];
    size_t n;
    while ((n = fread(b, 1, sizeof b, f)) > 0) t_ekle(&icerik, b, n);
    fclose(f);
    yanit_gonder(s, 200, icerik_turu(d.v), icerik.v ? icerik.v : "", icerik.n, NULL, 0, bas);
    free(icerik.v);
    free(d.v);
    return 1;
}

static int64_t son_istek_basarili;

/* Yolun işlevini çalıştırır. Çalışma hatası olursa hata() buraya geri döner. */
static SATIR_ICI_DEGIL int64_t yolu_calistir(int64_t (*islev)(int64_t), int64_t istek) {
    son_istek_basarili = 0;
    if (TUZAK_KUR(istek_tuzagi)) return 0;
    tuzak_kurulu = 1;
    int64_t y = islev(istek);
    tuzak_kurulu = 0;
    son_istek_basarili = 1;
    return y;
}

static const char *bul_n(const char *s, size_t n, const char *aranan) {
    size_t m = strlen(aranan);
    for (size_t i = 0; i + m <= n; i++)
        if (!memcmp(s + i, aranan, m)) return s + i;
    return NULL;
}

static void istegi_isle(Soket s, const char *istek_tanimi) {
    double bas_zaman = ondalik(ohc_zaman());
    Tampon b = {0};
    char parca[16384];
    const char *baslik_sonu = NULL;
    while (!baslik_sonu) {
        int k = recv(s, parca, sizeof parca, 0);
        if (k <= 0) {
            free(b.v);
            return;
        }
        t_ekle(&b, parca, (size_t)k);
        baslik_sonu = bul_n(b.v, b.n, "\r\n\r\n");
        if (!baslik_sonu && b.n > 65536) {
            hata_sayfasi(s, 431, "İstek başlıkları çok büyük", NULL, 0);
            free(b.v);
            return;
        }
    }
    size_t govde_basi = (size_t)(baslik_sonu - b.v) + 4;
    /* İstek satırı: YÖNTEM HEDEF SÜRÜM */
    char *satir_sonu = strstr(b.v, "\r\n");
    *satir_sonu = 0;
    char yontem[16] = {0};
    char *bosluk1 = strchr(b.v, ' ');
    char *bosluk2 = bosluk1 ? strchr(bosluk1 + 1, ' ') : NULL;
    if (!bosluk1 || !bosluk2 || bosluk1 - b.v >= (long)sizeof yontem || bosluk1[1] != '/') {
        hata_sayfasi(s, 400, "Geçersiz istek", NULL, 0);
        free(b.v);
        return;
    }
    memcpy(yontem, b.v, (size_t)(bosluk1 - b.v));
    *bosluk2 = 0;
    const char *hedef = bosluk1 + 1;
    int bas_istegi = !strcmp(yontem, "HEAD");

    /* Başlıklar */
    int64_t basliklar = ohc_sozluk_yeni();
    int64_t uzunluk = 0;
    int parcali = 0;
    char *p = satir_sonu + 2;
    while (p < baslik_sonu) {
        char *son = strstr(p, "\r\n");
        if (!son) break;
        *son = 0;
        char *iki = strchr(p, ':');
        if (iki) {
            for (char *c = p; c < iki; c++)
                if (*c >= 'A' && *c <= 'Z') *c = (char)(*c + 32);
            char *v = iki + 1;
            while (*v == ' ' || *v == '\t') v++;
            size_t vn = strlen(v);
            while (vn && (v[vn - 1] == ' ' || v[vn - 1] == '\t')) vn--;
            int64_t am = metin_yap(p, (size_t)(iki - p));
            int64_t dm = metin_yap(v, vn);
            ohc_sozluk_koy(basliklar, am, dm, KOD_METIN);
            if (!strcmp(M(am), "content-length")) uzunluk = strtoll(v, NULL, 10);
            if (!strcmp(M(am), "transfer-encoding") && strstr(v, "chunked")) parcali = 1;
        }
        p = son + 2;
    }
    if (parcali) {
        hata_sayfasi(s, 501, "Parçalı (chunked) istek gövdesi desteklenmiyor", NULL, bas_istegi);
        free(b.v);
        return;
    }
    if (uzunluk < 0 || uzunluk > 16 * 1024 * 1024) {
        hata_sayfasi(s, 413, "İstek gövdesi çok büyük", NULL, bas_istegi);
        free(b.v);
        return;
    }
    while (b.n < govde_basi + (size_t)uzunluk) {
        int k = recv(s, parca, sizeof parca, 0);
        if (k <= 0) {
            free(b.v);
            return;
        }
        t_ekle(&b, parca, (size_t)k);
    }
    int64_t govde = metin_yap(b.v + govde_basi, (size_t)uzunluk);
    /* Gövde okunurken tampon yer değiştirmiş olabilir; hedefi yeniden bul. */
    hedef = strchr(b.v, ' ') + 1;

    /* Yol ve sorgu */
    const char *soru = strchr(hedef, '?');
    size_t yol_n = soru ? (size_t)(soru - hedef) : strlen(hedef);
    Tampon yt = {0};
    yuzde_coz(&yt, hedef, yol_n, 0);
    while (yt.n > 1 && yt.v[yt.n - 1] == '/') yt.v[--yt.n] = 0;
    int64_t yol = t_metin(&yt);
    int64_t sorgu = ohc_sozluk_yeni();
    ((Sozluk *)(intptr_t)sorgu)->anahtar_kodu = KOD_METIN;
    if (soru) form_coz(sorgu, soru + 1, strlen(soru + 1));
    Tampon gt = {0};
    yuzde_coz(&gt, hedef, strlen(hedef), 0);
    int64_t gorunen = t_metin(&gt);

    int64_t durum = 404;
    if ((!strcmp(yontem, "GET") || bas_istegi) && statik_gonder(s, M(yol), bas_istegi)) {
        durum = 200;
    } else {
        /* En çok sabit parçası olan eşleşen yol seçilir. */
        WebYolu *secilen = NULL;
        int en_iyi = 0, yontem_farkli = 0;
        for (int64_t i = 0; i < web_yolu_sayisi; i++) {
            int e = yol_eslesir(web_yollari[i].kalip, M(yol), 0);
            if (!e) continue;
            const char *y = web_yollari[i].yontem;
            if (strcmp(y, yontem) && !(bas_istegi && !strcmp(y, "GET"))) {
                yontem_farkli = 1;
                continue;
            }
            if (e > en_iyi) {
                en_iyi = e;
                secilen = &web_yollari[i];
            }
        }
        if (!secilen) {
            durum = yontem_farkli ? 405 : 404;
            Tampon a = {0};
            t_yaz(&a, yontem);
            t_yaz(&a, " ");
            t_yaz(&a, M(yol));
            hata_sayfasi(s, durum, yontem_farkli ? "Bu adres bu yöntemle kullanılamaz" : "Sayfa bulunamadı", a.v,
                         bas_istegi);
            free(a.v);
        } else {
            int64_t form = ohc_sozluk_yeni();
            ((Sozluk *)(intptr_t)form)->anahtar_kodu = KOD_METIN;
            int64_t tur = ohc_sozluk_icerir(basliklar, D("content-type"), KOD_METIN)
                              ? ohc_sozluk_al(basliklar, D("content-type"), KOD_METIN, 0)
                              : D("");
            if (strstr(M(tur), "application/x-www-form-urlencoded"))
                form_coz(form, M(govde), strlen(M(govde)));
            else if (strstr(M(tur), "json"))
                json_formu(form, M(govde));
            int64_t parametreler = ohc_sozluk_yeni();
            ((Sozluk *)(intptr_t)parametreler)->anahtar_kodu = KOD_METIN;
            yol_eslesir(secilen->kalip, M(yol), parametreler);

            int64_t istek = nesne_yeni(istek_tanimi);
            ModelBilgisi *im = model_bilgisi(istek_tanimi);
#define ISTEK_KOY(ad, deger)                                                                                           \
    do {                                                                                                               \
        int64_t i_ = alan_sirasi(im, ad);                                                                              \
        if (i_ >= 0) ALAN(istek, i_) = (deger);                                                                        \
    } while (0)
            ISTEK_KOY("yöntem", metin_yap(yontem, strlen(yontem)));
            ISTEK_KOY("yol", yol);
            ISTEK_KOY("sorgu", sorgu);
            ISTEK_KOY("form", form);
            ISTEK_KOY("parametreler", parametreler);
            ISTEK_KOY("gövde", govde);
            ISTEK_KOY("başlıklar", basliklar);
#undef ISTEK_KOY
            int64_t y = yolu_calistir(secilen->islev, istek);
            if (!son_istek_basarili) {
                durum = 500;
                hata_sayfasi(s, 500, "Sunucu hatası", son_hata, bas_istegi);
            } else if (!y) {
                durum = 204;
                yanit_gonder(s, 204, NULL, "", 0, NULL, 0, bas_istegi);
            } else {
                ModelBilgisi *ym = nesne_bilgisi(y);
                int64_t i;
                durum = (i = alan_sirasi(ym, "durum")) >= 0 ? ALAN(y, i) : 200;
                const char *tur_y = (i = alan_sirasi(ym, "tür")) >= 0 ? M(ALAN(y, i)) : "text/html; charset=utf-8";
                const char *g = (i = alan_sirasi(ym, "gövde")) >= 0 ? M(ALAN(y, i)) : "";
                const char *konum = (i = alan_sirasi(ym, "konum")) >= 0 ? M(ALAN(y, i)) : "";
                int64_t ek = (i = alan_sirasi(ym, "başlıklar")) >= 0 ? ALAN(y, i) : 0;
                yanit_gonder(s, durum, tur_y, g, strlen(g), konum, ek, bas_istegi);
            }
        }
    }
    double ms = (ondalik(ohc_zaman()) - bas_zaman) * 1000.0;
    printf("%s %s → %" PRId64 " · %.0f ms\n", yontem, M(gorunen), durum, ms);
    fflush(stdout);
    free(b.v);
}

static void zaman_asimi(Soket s, int saniye) {
#ifdef _WIN32
    DWORD ms = (DWORD)saniye * 1000;
    setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, (const char *)&ms, sizeof ms);
    setsockopt(s, SOL_SOCKET, SO_SNDTIMEO, (const char *)&ms, sizeof ms);
#else
    struct timeval tv;
    tv.tv_sec = saniye;
    tv.tv_usec = 0;
    setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof tv);
    setsockopt(s, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof tv);
#endif
}

/* Sunucuyu başlatır ve istekleri sırayla işler (dönmez). */
void ohc_sun(int64_t kapi, int64_t istek_tanimi) {
    /* ORHUNCA_KAPI programdaki kapıyı geçersiz kılar; 0 ise boş bir kapı seçilir. */
    const char *e = getenv("ORHUNCA_KAPI");
    int otomatik = e && !strcmp(e, "0");
    if (e && atol(e) > 0) kapi = atol(e);
    if (kapi <= 0 || kapi > 65535) kapi = 3000;
    if (otomatik) kapi = 0;
    const char *adres = getenv("ORHUNCA_ADRES");
    if (!adres || !*adres) adres = "127.0.0.1";
#ifdef _WIN32
    WSADATA w;
    if (WSAStartup(MAKEWORD(2, 2), &w)) hata(0, "Windows soket kütüphanesi başlatılamadı");
#else
    signal(SIGPIPE, SIG_IGN);
#endif
    Soket dinleyici = socket(AF_INET, SOCK_STREAM, 0);
    if (dinleyici == GECERSIZ_SOKET) hata(0, "sunucu soketi açılamadı");
#ifndef _WIN32
    int bir = 1;
    setsockopt(dinleyici, SOL_SOCKET, SO_REUSEADDR, &bir, sizeof bir);
#endif
    struct sockaddr_in a;
    memset(&a, 0, sizeof a);
    a.sin_family = AF_INET;
    a.sin_port = htons((unsigned short)kapi);
    a.sin_addr.s_addr = inet_addr(adres);
    if (bind(dinleyici, (struct sockaddr *)&a, sizeof a) != 0 || listen(dinleyici, 64) != 0) {
        char m[200];
        snprintf(m, sizeof m,
                 "%" PRId64 " numaralı kapı açılamadı; başka bir sunucu kullanıyor olabilir (ORHUNCA_KAPI ile başka "
                 "bir kapı seçin)",
                 kapi);
        hata(0, m);
    }
    if (otomatik) {
        socklen_t uz = sizeof a;
        getsockname(dinleyici, (struct sockaddr *)&a, &uz);
        kapi = ntohs(a.sin_port);
    }
    printf("Sunucu dinleniyor: http://localhost:%" PRId64 "\n", kapi);
    fflush(stdout);
    for (;;) {
        Soket s = accept(dinleyici, NULL, NULL);
        if (s == GECERSIZ_SOKET) continue;
        zaman_asimi(s, 10);
        istegi_isle(s, M(istek_tanimi));
        soket_kapat(s);
    }
}
