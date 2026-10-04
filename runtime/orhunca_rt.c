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
 * 5 + 8*(anahtar + 2*değer) = sözlük (anahtar: 0 sayı, 1 metin).
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
#include <windows.h>
#include <shellapi.h>
#else
#include <unistd.h>
#endif

#define TUR_METIN 0
#define TUR_LISTE 1
#define TUR_SOZLUK 2

#define KOD_SAYI 0
#define KOD_METIN 1
#define KOD_MANTIK 2
#define KOD_ONDALIK 3

#define M(x) ((const char *)(intptr_t)(x))
#define D(p) ((int64_t)(intptr_t)(p))

static void hata(int64_t satir, const char *mesaj) {
    fflush(stdout);
    if (satir > 0)
        fprintf(stderr, "Çalışma hatası (satır %" PRId64 "): %s\n", satir, mesaj);
    else
        fprintf(stderr, "Çalışma hatası: %s\n", mesaj);
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
