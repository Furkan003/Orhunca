/*
 * Orhunca çalışma zamanı.
 *
 * Derleyicinin ürettiği makine kodu yazdırma, metin ve liste işlemleri için bu
 * küçük kütüphaneyi çağırır. Tüm değerler 64 bitlik tamsayı olarak taşınır:
 * metinler NUL ile biten UTF-8 dizilerine, listeler `Liste` yapısına işaretçidir,
 * ondalıklar `double` bit desenidir.
 *
 * Bellek: metinler ve listeler çöp toplayıcının yönettiği yığından ayrılır.
 * Toplayıcı "tutucu" (conservative) bir işaretle-süpür toplayıcıdır: yığıttaki
 * ve yazmaçlardaki her 64 bitlik sözcüğü olası bir işaretçi sayar, ulaşılamayan
 * nesneleri geri verir. Derleyicinin ayrıca bir şey yapması gerekmez.
 *
 * Tip kodları: 0 sayı, 1 metin, 2 mantık, 3 ondalık, 4 + 8*öğe = liste.
 */
#include <inttypes.h>
#include <setjmp.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#endif

#define TUR_METIN 0
#define TUR_LISTE 1

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

/* ====================================================================== */
/* Çöp toplayıcı                                                          */
/* ====================================================================== */

typedef struct Nesne {
    struct Nesne *sonraki;
    size_t boyut; /* yük + (listelerde) öğe dizisi bayt sayısı */
    uint8_t tur;
    uint8_t isaretli;
} Nesne;

typedef struct {
    int64_t uzunluk;
    int64_t kapasite;
    int64_t *ogeler; /* ayrıca malloc ile ayrılır, liste ile birlikte geri verilir */
} Liste;

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
    if (n->tur != TUR_LISTE) return;
    if (bekleyen_say == bekleyen_kap) {
        bekleyen_kap = bekleyen_kap ? bekleyen_kap * 2 : 256;
        bekleyen = realloc(bekleyen, bekleyen_kap * sizeof(uintptr_t));
        if (!bekleyen) hata(0, "bellek yetersiz");
    }
    bekleyen[bekleyen_say++] = p;
}

static void bekleyenleri_isle(void) {
    while (bekleyen_say) {
        Liste *l = (Liste *)bekleyen[--bekleyen_say];
        for (int64_t i = 0; i < l->uzunluk; i++) aday((uintptr_t)l->ogeler[i]);
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
            if (n->tur == TUR_LISTE) free(((Liste *)YUK(n))->ogeler);
            free(n);
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

/* ====================================================================== */
/* Giriş noktası                                                           */
/* ====================================================================== */

int ohc_ana(void);

int main(void) {
    volatile uintptr_t dip = 0;
    yigin_dibi = (uintptr_t *)&dip + 1;
#ifdef _WIN32
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
#endif
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
        fprintf(stderr, "bellek: %" PRIu64 " toplama, en yüksek canlı bellek %" PRIu64 " bayt\n", (uint64_t)toplama_sayisi,
                (uint64_t)en_yuksek_canli);
    return kod;
}

/* ====================================================================== */
/* Yazdırma                                                                */
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

/* 3.0 → "3.0", 0.1 + 0.2 → "0.3" */
static void ondalik_bicimle(double d, char *s, size_t n) {
    snprintf(s, n, "%.15g", d);
    if (!strpbrk(s, ".eni")) strncat(s, ".0", n - strlen(s) - 1);
}

static void deger_yaz(int64_t d, int64_t kod, int ic);

static void liste_yaz(Liste *l, int64_t oge_kodu) {
    putchar('[');
    for (int64_t i = 0; i < l->uzunluk; i++) {
        if (i) fputs(", ", stdout);
        deger_yaz(l->ogeler[i], oge_kodu, 1);
    }
    putchar(']');
}

static void deger_yaz(int64_t d, int64_t kod, int ic) {
    char tampon[64];
    switch (kod % 8) {
    case 0: printf("%" PRId64, d); break;
    case 1:
        if (ic) printf("\"%s\"", (const char *)(intptr_t)d);
        else fputs((const char *)(intptr_t)d, stdout);
        break;
    case 2: fputs(d ? "doğru" : "yanlış", stdout); break;
    case 3:
        ondalik_bicimle(ondalik(d), tampon, sizeof tampon);
        fputs(tampon, stdout);
        break;
    default: liste_yaz((Liste *)(intptr_t)d, kod / 8); break;
    }
}

void ohc_yaz(int64_t d, int64_t kod) {
    deger_yaz(d, kod, 0);
    putchar('\n');
}

/* ====================================================================== */
/* Aritmetik                                                               */
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

/* Yarımlar sıfırdan uzağa yuvarlanır: 2.5 → 3, -2.5 → -3 (libm gerektirmez). */
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

/* ====================================================================== */
/* Metinler                                                                */
/* ====================================================================== */

int64_t ohc_metin_birlestir(int64_t a, int64_t b) {
    const char *x = (const char *)(intptr_t)a, *y = (const char *)(intptr_t)b;
    size_t n = strlen(x), m = strlen(y);
    char *s = metin_ayir(n + m + 1);
    /* x ve y hâlâ bu çerçevede canlıdır, toplayıcı onları görür */
    memcpy(s, (const char *)(intptr_t)a, n);
    memcpy(s + n, (const char *)(intptr_t)b, m + 1);
    return (int64_t)(intptr_t)s;
}

int64_t ohc_metne_cevir(int64_t d, int64_t kod) {
    if (kod == 1) return d;
    if (kod == 2) return (int64_t)(intptr_t)(d ? "doğru" : "yanlış");
    char tampon[64];
    if (kod == 3) ondalik_bicimle(ondalik(d), tampon, sizeof tampon);
    else snprintf(tampon, sizeof tampon, "%" PRId64, d);
    size_t n = strlen(tampon);
    char *s = metin_ayir(n + 1);
    memcpy(s, tampon, n + 1);
    return (int64_t)(intptr_t)s;
}

int64_t ohc_metin_esit(int64_t a, int64_t b) {
    return strcmp((const char *)(intptr_t)a, (const char *)(intptr_t)b) == 0;
}

/* UTF-8 karakter sayısı (bayt değil). */
int64_t ohc_metin_uzunluk(int64_t a) {
    int64_t n = 0;
    for (const unsigned char *p = (const unsigned char *)(intptr_t)a; *p; p++)
        if ((*p & 0xC0) != 0x80) n++;
    return n;
}

static void cevirme_hatasi(int64_t a, int64_t satir, const char *ne) {
    char mesaj[256];
    snprintf(mesaj, sizeof mesaj, "'%.200s' bir %s değil", (const char *)(intptr_t)a, ne);
    hata(satir, mesaj);
}

static int bosluk(char c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n'; }

int64_t ohc_metinden_sayi(int64_t a, int64_t satir) {
    const char *s = (const char *)(intptr_t)a;
    char *son;
    while (bosluk(*s)) s++;
    int64_t n = strtoll(s, &son, 10);
    while (bosluk(*son)) son++;
    if (son == s || *son) cevirme_hatasi(a, satir, "sayı");
    return n;
}

int64_t ohc_metinden_ondalik(int64_t a, int64_t satir) {
    /* Türkçe yazımdaki virgül de kabul edilir: "3,5" */
    char tampon[128];
    const char *s = (const char *)(intptr_t)a;
    size_t n = strlen(s);
    if (n >= sizeof tampon) cevirme_hatasi(a, satir, "ondalık sayı");
    for (size_t i = 0; i <= n; i++) tampon[i] = s[i] == ',' ? '.' : s[i];
    char *p = tampon, *son;
    while (bosluk(*p)) p++;
    double d = strtod(p, &son);
    while (bosluk(*son)) son++;
    if (son == p || *son) cevirme_hatasi(a, satir, "ondalık sayı");
    return bitlere(d);
}

int64_t ohc_oku(void) {
    size_t kap = 64, n = 0;
    char *t = ham_ayir(kap);
    int c;
    fflush(stdout);
    while ((c = getchar()) != EOF && c != '\n') {
        if (n + 1 >= kap) {
            kap *= 2;
            t = realloc(t, kap);
            if (!t) hata(0, "bellek yetersiz");
        }
        t[n++] = (char)c;
    }
    if (n && t[n - 1] == '\r') n--;
    char *s = metin_ayir(n + 1);
    memcpy(s, t, n);
    s[n] = 0;
    free(t);
    return (int64_t)(intptr_t)s;
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
    NESNE(l)->boyut += sizeof(int64_t) * 4;
    hesapla(sizeof(int64_t) * 4);
    return (int64_t)(intptr_t)l;
}

void ohc_liste_ekle(int64_t lp, int64_t d) {
    Liste *l = (Liste *)(intptr_t)lp;
    if (l->uzunluk == l->kapasite) {
        size_t eklenen = sizeof(int64_t) * (size_t)l->kapasite;
        l->kapasite *= 2;
        int64_t *yeni = realloc(l->ogeler, sizeof(int64_t) * (size_t)l->kapasite);
        if (!yeni) hata(0, "bellek yetersiz");
        l->ogeler = yeni;
        NESNE(l)->boyut += eklenen;
        hesapla(eklenen);
    }
    l->ogeler[l->uzunluk++] = d;
}

int64_t ohc_liste_uzunluk(int64_t lp) { return ((Liste *)(intptr_t)lp)->uzunluk; }

static void sinir(Liste *l, int64_t i, int64_t satir) {
    if (i < 0 || i >= l->uzunluk) {
        char mesaj[128];
        snprintf(mesaj, sizeof mesaj, "liste sınırı aşıldı: indeks %" PRId64 ", uzunluk %" PRId64, i,
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
    const unsigned char *s = *p;
    int64_t c = *s;
    int n = c < 0x80 ? 1 : c < 0xE0 ? 2 : c < 0xF0 ? 3 : 4;
    for (int i = 1; i < n && s[i]; i++) c = (c << 8) | s[i];
    *p += n;
    return c < 0x80 ? c : 100000 + c;
}

static int metin_karsilastir(const char *a, const char *b) {
    const unsigned char *x = (const unsigned char *)a, *y = (const unsigned char *)b;
    while (*x && *y) {
        int64_t p = harf_sirasi(&x), q = harf_sirasi(&y);
        if (p != q) return p < q ? -1 : 1;
    }
    if (*x) return 1;
    if (*y) return -1;
    return strcmp(a, b);
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
    return metin_karsilastir((const char *)(intptr_t) * (const int64_t *)a,
                             (const char *)(intptr_t) * (const int64_t *)b);
}

void ohc_liste_sirala(int64_t lp, int64_t oge_kodu) {
    Liste *l = (Liste *)(intptr_t)lp;
    int (*kars)(const void *, const void *) =
        oge_kodu == 1 ? metin_kars : oge_kodu == 3 ? ondalik_kars : sayi_kars;
    if (l->uzunluk > 1) qsort(l->ogeler, (size_t)l->uzunluk, sizeof(int64_t), kars);
}
