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
 * WebAssembly: aynı kaynak wasm32'ye derlenir (araclar/wasm_calisma_zamani.sh);
 * sistem işleri runtime/wasm/libc.c üzerinden JavaScript'e devredilir. Orada
 * yığıt taranamadığından toplayıcı, derleyicinin tuttuğu "gölge yığıtı" tarar.
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
#ifdef __wasm__
/* WebAssembly: sistem kütüphanesi yerine runtime/wasm/libc.h (JavaScript'e devreder) */
#include "libc.h"
#else
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
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0600 /* WSAPoll */
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <shellapi.h>
#include <direct.h>
#else
#include <arpa/inet.h>
#include <dlfcn.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <poll.h>
#include <sys/mman.h>
#include <signal.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <unistd.h>
#endif
#endif /* __wasm__ */

#define TUR_METIN 0
#define TUR_LISTE 1
#define TUR_SOZLUK 2

#define KOD_SAYI 0
#define KOD_METIN 1
#define KOD_MANTIK 2
#define KOD_ONDALIK 3

#define KOD_MODEL 6

#define M(x) ((const char *)(intptr_t)(x))
#define D(p) ((int64_t)(uintptr_t)(p))

/* Hata yakalama: `dene:` bloğu (ve web isteği) bir "yakalayıcı" kurar. Çalışma
 * hatası olursa hata() en içteki yakalayıcıya geri sarar. Web isteği işlenirken
 * oluşan ve yakalanmayan hatalar sunucuyu durdurmaz: tarayıcıya 500 sayfası gider.
 * WebAssembly'de geri sarmayı JavaScript yapar (js_hata_yakala bir istisna fırlatır). */
#if defined(__wasm__)
#elif defined(__GNUC__)
typedef void *Tuzak[5];
#define TUZAK_KUR(t) __builtin_setjmp(t)
#define TUZAGA_DON(t) __builtin_longjmp(t, 1)
#else
typedef jmp_buf Tuzak;
#define TUZAK_KUR(t) setjmp(t)
#define TUZAGA_DON(t) longjmp(t, 1)
#endif
#ifndef __wasm__
typedef struct Yakalayici {
    Tuzak tuzak;
    struct Yakalayici *onceki;
    int istek; /* web isteği: hata yine de yazılır */
} Yakalayici;
static Yakalayici *yakalayici;
/* Hata ayıklamada çağrı yığınının derinliği (yakalanan hatada geri alınır) */
static int ay_derinlik;
static void ay_hatada_dur(const char *mesaj);
#endif
static char son_hata[1024];
/* Son çalışma hatasının yalnızca mesajı (yakala bloğundaki değişkene gelir) */
static char son_mesaj[1024];

static void hata(int64_t satir, const char *mesaj) {
    fflush(stdout);
    snprintf(son_mesaj, sizeof son_mesaj, "%s", mesaj);
    if (satir > 0)
        snprintf(son_hata, sizeof son_hata, "Çalışma hatası (satır %" PRId64 "): %s", satir, mesaj);
    else
        snprintf(son_hata, sizeof son_hata, "Çalışma hatası: %s", mesaj);
#ifdef __wasm__
    js_hata_yakala();
#else
    if (yakalayici) {
        Yakalayici *y = yakalayici;
        yakalayici = y->onceki;
        if (y->istek) fprintf(stderr, "%s\n", son_hata);
        TUZAGA_DON(y->tuzak);
    }
    ay_hatada_dur(son_hata);
#endif
    fprintf(stderr, "%s\n", son_hata);
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

/* Yükler 8 bayta hizalı kalsın (wasm32'de sizeof(Nesne) 12'dir). */
#define BASLIK ((sizeof(Nesne) + 7) & ~(size_t)7)
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
#ifndef __wasm__
static uintptr_t *yigin_dibi;
#endif
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

#ifdef __wasm__
/* WebAssembly'de yığıt ve yerel değişkenler taranamaz. Derleyici, çöp
 * toplayıcının yönettiği her değeri (metin, liste, sözlük, model değişkenleri ve
 * bir Orhunca çağrısı boyunca yaşayan ara değerler) bellekteki "gölge yığıtta"
 * tutar. Toplama yalnızca güvenli noktalarda (işlev girişleri ve döngü başları)
 * yapılır: o anda canlı her değer gölge yığıttadır. Ayırma yalnızca bayrağı
 * kaldırır. */
static int64_t *golge_taban, *golge_tepe;
static volatile uint8_t toplama_gerekli;

static void yigini_tara(void) {
    for (int64_t *p = golge_taban; p < golge_tepe; p++)
        if (!((uint64_t)*p >> 32)) aday((uintptr_t)*p);
    bekleyenleri_isle();
}
#else
/* Bu işlevin çerçevesi, yazmaçların kaydedildiği `topla` çerçevesinin altındadır;
 * buradan yığıt dibine kadar her sözcük taranır. */
static SATIR_ICI_DEGIL void yigini_tara(void) {
    volatile uintptr_t isaret = 0;
    for (uintptr_t *p = (uintptr_t *)&isaret; p < yigin_dibi; p++) aday(*p);
    bekleyenleri_isle();
}
#endif

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
#ifdef __wasm__
    toplama_gerekli = 0;
#else
    jmp_buf yazmaclar; /* çağıranın yazmaçlarındaki işaretçiler buraya düşer */
    setjmp(yazmaclar);
    (void)yazmaclar;
#endif
    yigini_tara();
    supur();
    ayrilan_bayt = 0;
    esik = (!sabit_esik && canli_bayt * 2 > taban_esik) ? canli_bayt * 2 : taban_esik;
    toplama_sayisi++;
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
#ifdef __wasm__
    if (ayrilan_bayt > esik) toplama_gerekli = 1;
#else
    if (ayrilan_bayt > esik) topla();
#endif
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

#ifndef __wasm__
int ohc_ana(void);
#endif

static int arguman_sayisi;
static char **argumanlar;

#ifndef __wasm__
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
#endif

/* Çalıştırmaya hazırlık ve bitiş: yerel programda main, WebAssembly'de
 * JavaScript yükleyicisi (orhunca.js) çağırır. */
static void baslat(void) {
    /* Test için: ORHUNCA_GC_ESIK=4096 toplayıcıyı çok sık çalıştırır. */
    const char *e = getenv("ORHUNCA_GC_ESIK");
    if (e && atol(e) > 0) {
        taban_esik = esik = (size_t)atol(e);
        sabit_esik = 1;
    }
    tabloyu_kur(0);
}

static void bitir(void) {
    fflush(stdout);
    if (getenv("ORHUNCA_BELLEK_RAPORU"))
        fprintf(stderr, "bellek: %" PRIu64 " toplama, en yüksek canlı bellek %" PRIu64 " bayt\n",
                (uint64_t)toplama_sayisi, (uint64_t)en_yuksek_canli);
}

#ifdef __wasm__
#define DISA(ad) __attribute__((export_name(#ad)))

DISA(ohc_wasm_baslat) void ohc_wasm_baslat(void) {
    int n = js_arguman_sayisi();
    argumanlar = ham_ayir(sizeof(char *) * (size_t)(n + 1));
    for (int i = 0; i < n; i++) argumanlar[i] = js_arguman(i);
    arguman_sayisi = n;
    baslat();
}

DISA(ohc_wasm_bitir) void ohc_wasm_bitir(void) { bitir(); }

/* Derleyicinin ürettiği programın kullandığı yardımcılar */
DISA(ohc_wasm_ayir) int64_t ohc_wasm_ayir(int64_t bayt) { return D(ham_ayir((size_t)bayt)); }

DISA(ohc_wasm_golge) int64_t ohc_wasm_golge(int64_t bayt) {
    golge_taban = golge_tepe = ham_ayir((size_t)bayt);
    memset(golge_taban, 0, (size_t)bayt);
    return D(golge_taban);
}

DISA(ohc_wasm_bayrak) int64_t ohc_wasm_bayrak(void) { return D(&toplama_gerekli); }

DISA(ohc_guvenli_nokta) void ohc_guvenli_nokta(int64_t tepe) {
    golge_tepe = (int64_t *)(uintptr_t)tepe;
    if (toplama_gerekli) topla();
}

/* JavaScript'ten gelen metin (arayüz olaylarında kullanıcının girdiği değer):
 * n baytlık yönetilen bir metin ayrılır, JavaScript içini doldurur. */
DISA(ohc_wasm_metin) int64_t ohc_wasm_metin(int64_t n) {
    char *m = metin_ayir((size_t)n + 1);
    m[n] = 0;
    return D(m);
}

DISA(ohc_yigin_tasti) void ohc_yigin_tasti(void) {
    hata(0, "çok derin özyineleme: işlevler birbirini bitmeyecek kadar çok çağırıyor");
}
#else
#ifdef ORHUNCA_CALISTIRICI
static int (*program_yukle(void))(void);
#endif

int main(int argc, char **argv) {
    volatile uintptr_t dip = 0;
    yigin_dibi = (uintptr_t *)&dip + 1;
#ifdef _WIN32
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
#endif
    argumanlari_kaydet(argc, argv);
    baslat();
#ifdef ORHUNCA_CALISTIRICI
    int kod = program_yukle()();
#else
    int kod = ohc_ana();
#endif
    bitir();
    return kod;
}
#endif

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

/* Taşma denetimli çarpma. 128 bitlik ara sonuç kullanmaz (wasm32'de bunun için
 * derleyici kütüphanesi gerekirdi): mutlak değerler sınırla karşılaştırılır. */
static int carp_tasar(int64_t a, int64_t b, int64_t *sonuc) {
    uint64_t ma = a < 0 ? 0 - (uint64_t)a : (uint64_t)a;
    uint64_t mb = b < 0 ? 0 - (uint64_t)b : (uint64_t)b;
    uint64_t sinir = (a < 0) != (b < 0) ? (uint64_t)INT64_MAX + 1 : (uint64_t)INT64_MAX;
    *sonuc = (int64_t)((uint64_t)a * (uint64_t)b);
    return ma != 0 && mb > sinir / ma;
}

/* WebAssembly'de çarpma: derleyici küçük sayılarda doğrudan çarpar, büyüklerde bunu çağırır. */
int64_t ohc_carp(int64_t a, int64_t b, int64_t satir) {
    int64_t r;
    if (carp_tasar(a, b, &r)) ohc_tasma(satir);
    return r;
}

int64_t ohc_us_tam(int64_t a, int64_t b, int64_t satir) {
    if (b < 0) hata(satir, "tamsayılarda üs negatif olamaz; ondalık kullanın: üs(2.0, -1)");
    int64_t sonuc = 1, taban = a;
    while (b > 0) {
        if (b & 1) {
            if (carp_tasar(sonuc, taban, &sonuc)) ohc_tasma(satir);
        }
        b >>= 1;
        if (b && carp_tasar(taban, taban, &taban)) ohc_tasma(satir);
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
#ifdef __wasm__
            tohum = (uint64_t)(js_zaman() * 1e6) ^ ((uint64_t)js_rastgele_tohum() << 20);
#else
            volatile int yerel = 0;
            tohum = (uint64_t)time(NULL) ^ ((uint64_t)(uintptr_t)&yerel << 16) ^ (uint64_t)clock();
#endif
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

/* 64 × 64 bitlik çarpımın üst 64 biti */
static uint64_t ust_carpim(uint64_t a, uint64_t b) {
    uint64_t a0 = (uint32_t)a, a1 = a >> 32, b0 = (uint32_t)b, b1 = b >> 32;
    uint64_t p00 = a0 * b0, p01 = a0 * b1, p10 = a1 * b0, p11 = a1 * b1;
    uint64_t orta = (p00 >> 32) + (uint32_t)p01 + (uint32_t)p10;
    return p11 + (p01 >> 32) + (p10 >> 32) + (orta >> 32);
}

int64_t ohc_rastgele_aralik(int64_t a, int64_t b, int64_t satir) {
    if (a > b) hata(satir, "rastgele(a, b) için a, b'den büyük olamaz");
    uint64_t aralik = (uint64_t)b - (uint64_t)a + 1;
    if (aralik == 0) return (int64_t)rng();
    return (int64_t)((uint64_t)a + ust_carpim(rng(), aralik));
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

/* Büyük harf (alan adlarının ilk harfi için): i → İ. Programların büyük_harf /
 * küçük_harf işlevleri runtime/ön_kütüphane.ohc'dedir. */
static uint32_t buyuk(uint32_t c) {
    if (c == 'i') return 0x130;
    if (c == 0x131) return 'I';
    if (c >= 'a' && c <= 'z') return c - 32;
    if (c >= 0xE0 && c <= 0xFE && c != 0xF7) return c - 32;
    if (c >= 0x100 && c <= 0x17F && c != 0x131 && (c & 1)) return c - 1; /* ğ ş ... */
    return c;
}


static int bosluk(char c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\v' || c == '\f'; }

int64_t ohc_liste_yeni(void);
void ohc_liste_ekle(int64_t lp, int64_t d);

/* İlk karakterin Unicode kodu: kod("A") = 65, kod("ç") = 231; boş metinde 0. */
int64_t ohc_kod(int64_t m) {
    const char *s = M(m);
    return *s ? (int64_t)u8_oku(&s) : 0;
}

/* Unicode kodundan tek karakterlik metin: karakter(231) = "ç". */
int64_t ohc_karakter(int64_t n, int64_t satir) {
    if (n <= 0 || n > 0x10FFFF || (n >= 0xD800 && n <= 0xDFFF)) {
        char m[96];
        snprintf(m, sizeof m, "%" PRId64 " geçerli bir karakter kodu değil", n);
        hata(satir, m);
    }
    Tampon t = {0};
    u8_yaz(&t, (uint32_t)n);
    return t_metin(&t);
}

/* Metnin karakter kodları: kodlar("aç") = [97, 231]. */
int64_t ohc_kodlar(int64_t m) {
    int64_t l = ohc_liste_yeni();
    const char *s = M(m);
    while (*s) ohc_liste_ekle(l, (int64_t)u8_oku(&s));
    return l;
}

/* Karakter kodlarından metin: kodlardan([97, 231]) = "aç". */
int64_t ohc_kodlardan(int64_t lp, int64_t satir) {
    Liste *l = (Liste *)(intptr_t)lp;
    Tampon t = {0};
    for (int64_t i = 0; i < l->uzunluk; i++) {
        int64_t n = l->ogeler[i];
        if (n <= 0 || n > 0x10FFFF || (n >= 0xD800 && n <= 0xDFFF)) {
            free(t.v);
            char m[96];
            snprintf(m, sizeof m, "%" PRId64 " geçerli bir karakter kodu değil", n);
            hata(satir, m);
        }
        u8_yaz(&t, (uint32_t)n);
    }
    return t_metin(&t);
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
#if defined(__wasm__)
    return bitlere(js_zaman());
#elif defined(_WIN32)
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
#ifdef __wasm__
    char t[32];
    js_tarih(t);
    return metin_yap(t, strlen(t));
#else
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
#endif
}

void ohc_bekle(int64_t saniye) {
    double s = ondalik(saniye);
    if (!(s > 0)) return;
    fflush(stdout);
#if defined(__wasm__)
    js_bekle(s);
#elif defined(_WIN32)
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
/* Hata yakalama (dene / yakala)                                          */
/* ====================================================================== */

/* `dene:` bloğu: derleyici bloğu ayrı bir işleve çevirir; işlev, çevreleyen
 * işlevin değişkenlerini `cerceve` adresindeki yuvalardan okur ve yazar.
 * Sonuç: bloğun dönüş kodu (0 sona ulaştı, 1 döndür, 2 dur, 3 sürdür) ya da
 * çalışma hatasında -1. WebAssembly'de bu işi JavaScript yükleyicisi yapar. */
#ifndef __wasm__
int64_t ohc_dene(int64_t govde, int64_t cerceve) {
    Yakalayici y;
    y.onceki = yakalayici;
    y.istek = 0;
    int derinlik = ay_derinlik;
    if (TUZAK_KUR(y.tuzak)) {
        ay_derinlik = derinlik;
        return -1;
    }
    yakalayici = &y;
    int64_t sonuc = ((int64_t(*)(int64_t))(intptr_t)govde)(cerceve);
    yakalayici = y.onceki;
    return sonuc;
}
#endif

/* Son çalışma hatasının mesajı (`yakala hata:`) */
int64_t ohc_hata_mesaji(void) { return metin_yap(son_mesaj, strlen(son_mesaj)); }

/* `hata_ver("mesaj")`: programın kendi çalışma hatası */
void ohc_hata_ver(int64_t mesaj, int64_t satir) { hata(satir, M(mesaj)); }

/* ====================================================================== */
/* Modeller                                                                */
/* ====================================================================== */

typedef struct {
    char *ad;
    char *etiket; /* hata mesajlarında gösterilen ad (NULL: addan türetilir) */
    int64_t kod;
    int zorunlu, e_posta;
    int en_az_var, en_fazla_var;
    double en_az, en_fazla;
    char *secenekler; /* seçenek türündeki alanın geçerli değerleri: "a|b|c" (yoksa NULL) */
    char *ic_model;   /* alan (ya da liste/sözlük öğesi) bir modelse onun adı */
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
 * "ad<TAB>tip kodu<TAB>kurallar<TAB>en az<TAB>en fazla<TAB>etiket[<TAB>ek]"
 * (kurallar: 1 zorunlu, 2 e-posta biçimi; ek: seçenek alanında "a|b|c", model içeren
 * alanda "@Model"). Her tanım bir kez çözülür. */
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
        const char *parca[7] = {"", "0", "0", "", "", "", ""};
        size_t uz[7] = {0, 1, 1, 0, 0, 0, 0};
        int k = 0;
        const char *b = p;
        for (const char *q = p; q <= son && k < 7; q++) {
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
        int kurallar = atoi(parca[2]);
        a->zorunlu = kurallar & 1;
        a->e_posta = (kurallar & 2) != 0;
        a->etiket = uz[5] ? kopya_n(parca[5], uz[5]) : NULL;
        if (uz[6] && parca[6][0] == '@')
            a->ic_model = kopya_n(parca[6] + 1, uz[6] - 1);
        else if (uz[6])
            a->secenekler = kopya_n(parca[6], uz[6]);
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

/* Programın modellerinin tanımları (ana programın başında kaydedilir): iç içe
 * modellerin tanımı adla bulunur. */
static const char **kayitli_tanimlar;
static int64_t kayitli_tanim_sayisi;

void ohc_model_tanimla(int64_t tanim) {
    kayitli_tanimlar = ham_buyut(kayitli_tanimlar, sizeof(char *) * (size_t)(kayitli_tanim_sayisi + 1));
    kayitli_tanimlar[kayitli_tanim_sayisi++] = M(tanim);
}

static ModelBilgisi *model_bilgisi_adla(const char *ad) {
    if (!ad) return NULL;
    size_t n = strlen(ad);
    for (int64_t i = 0; i < kayitli_tanim_sayisi; i++) {
        const char *t = kayitli_tanimlar[i];
        if (!strncmp(t, ad, n) && (t[n] == '\n' || !t[n])) return model_bilgisi(t);
    }
    return NULL;
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
        /* Kaydedilmemiş nesnenin kimliği (0) yazılmaz. */
        int ilk = 1;
        for (int64_t i = 0; i < m->alan_sayisi; i++) {
            if (i == 0 && ALAN(d, 0) == 0) continue;
            if (!ilk) t_yaz(t, virgul);
            ilk = 0;
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
                        int64_t d = j_deger(j, mb->alanlar[i].kod, model_bilgisi_adla(mb->alanlar[i].ic_model));
                        ALAN(sonuc, i) = d;
                    }
                } else {
                    int64_t anahtar = ak == KOD_METIN ? t_metin(&a) : (int64_t)strtoll(a.v ? a.v : "0", NULL, 10);
                    if (ak != KOD_METIN) free(a.v);
                    int64_t d = j_deger(j, dk, mb);
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

/* Dosyayı taşır ya da adını değiştirir (hedef varsa üzerine yazılır). */
int64_t ohc_dosya_tasi(int64_t eski, int64_t yeni) { return dosya_tasi(M(eski), M(yeni)); }

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

/* Alanın hata mesajlarındaki adı: etiketi ya da "doğum_tarihi" → "Doğum tarihi" */
static void gorunen_ad(Tampon *t, AlanBilgisi *a) {
    if (a->etiket) {
        t_yaz(t, a->etiket);
        return;
    }
    const char *p = a->ad;
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
    gorunen_ad(t, a);
    t_yaz(t, on);
    sayi_yaz(t, sinir);
    t_yaz(t, son);
}

/* ad@alan.uzantı: boşluksuz, tek @, alanda nokta */
static int e_posta_mi(const char *s) {
    while (*s && bosluk(*s)) s++;
    const char *at = strchr(s, '@');
    if (!at || at == s || strchr(at + 1, '@')) return 0;
    const char *nokta = strrchr(at, '.');
    if (!nokta || nokta == at + 1 || !nokta[1]) return 0;
    for (const char *p = s; *p; p++)
        if (*p == ' ' || *p == '\t' || *p == '<' || *p == '>' || *p == ',') {
            const char *q = p;
            while (*q && bosluk(*q)) q++;
            if (*q) return 0;
            break;
        }
    return 1;
}

/* `deger`, "a|b|c" biçimindeki seçeneklerden biri mi? */
static int secenek_mi(const char *secenekler, const char *deger) {
    size_t n = strlen(deger);
    const char *p = secenekler;
    while (1) {
        const char *son = strchr(p, '|');
        size_t uz = son ? (size_t)(son - p) : strlen(p);
        if (uz == n && !strncmp(p, deger, n)) return 1;
        if (!son) return 0;
        p = son + 1;
    }
}

/* `Renk("mavi")`: metni seçenek türüne çevirir; geçerli değilse çalışma hatası. */
int64_t ohc_secenek_cevir(int64_t m, int64_t secenekler, int64_t tur, int64_t satir) {
    if (secenek_mi(M(secenekler), M(m))) return m;
    Tampon t = {0};
    t_yaz(&t, "'");
    t_yaz(&t, M(m));
    t_yaz(&t, "' bir ");
    t_yaz(&t, M(tur));
    t_yaz(&t, " değeri değil (değerler: ");
    for (const char *c = M(secenekler); *c; c++) {
        if (*c == '|')
            t_yaz(&t, ", ");
        else
            t_ekle(&t, c, 1);
    }
    t_yaz(&t, ")");
    static char mesaj[1024];
    size_t n = t.n < sizeof mesaj - 1 ? t.n : sizeof mesaj - 1;
    memcpy(mesaj, t.v, n);
    mesaj[n] = 0;
    free(t.v);
    hata(satir, mesaj);
    return 0;
}

int64_t ohc_model_hatalar(int64_t n);

/* İç modelin hataları, alanın adıyla: "Müşteri: Ad boş bırakılamaz", "Kalemler 2: ..." */
static void ic_hatalar(int64_t sonuc, AlanBilgisi *a, int64_t nesne, int64_t sira) {
    if (!nesne) return;
    int64_t h = ohc_model_hatalar(nesne);
    for (int64_t i = 0; i < ohc_liste_uzunluk(h); i++) {
        Tampon t = {0};
        gorunen_ad(&t, a);
        if (sira >= 0) {
            char b[32];
            snprintf(b, sizeof b, " %" PRId64, sira + 1);
            t_yaz(&t, b);
        }
        t_yaz(&t, ": ");
        t_yaz(&t, M(ohc_liste_al(h, i, 0)));
        ohc_liste_ekle(sonuc, t_metin(&t));
    }
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
            if (a->secenekler && !secenek_mi(a->secenekler, M(d))) {
                gorunen_ad(&t, a);
                t_yaz(&t, " şunlardan biri olmalı: ");
                for (const char *c = a->secenekler; *c; c++) {
                    if (*c == '|')
                        t_yaz(&t, ", ");
                    else
                        t_ekle(&t, c, 1);
                }
            } else if (!*s) {
                if (a->zorunlu) {
                    gorunen_ad(&t, a);
                    t_yaz(&t, " boş bırakılamaz");
                }
            } else if (a->e_posta && !e_posta_mi(M(d))) {
                gorunen_ad(&t, a);
                t_yaz(&t, " geçerli bir e-posta adresi olmalı");
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
                gorunen_ad(&t, a);
                t_yaz(&t, " işaretlenmeli");
            }
            break;
        case KOD_MODEL:
            if (!d) {
                if (a->zorunlu) {
                    gorunen_ad(&t, a);
                    t_yaz(&t, " boş bırakılamaz");
                }
            } else {
                ic_hatalar(sonuc, a, d, -1);
            }
            break;
        case 4:
        case 5: {
            double uz = (double)(a->kod % 8 == 4 ? ohc_liste_uzunluk(d) : ohc_sozluk_uzunluk(d));
            if (a->kod == 4 + 8 * KOD_MODEL)
                for (int64_t k = 0; k < (int64_t)uz; k++) ic_hatalar(sonuc, a, ohc_liste_al(d, k, 0), k);
            if (a->zorunlu && uz == 0) {
                gorunen_ad(&t, a);
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
            gorunen_ad(&t, a);
            t_yaz(&t, mesaj);
            ohc_sozluk_koy(baglama, i, t_metin(&t), KOD_SAYI);
        }
    }
}

/* ---------------------------------------------------------------------- */
/* Metin yardımcıları: HTML kaçırma, para biçimi, URL kodlama              */
/* ---------------------------------------------------------------------- */

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

#ifndef __wasm__
/* HTML ve URL kaçırma (sunucunun hata sayfaları ve yönlendirmeleri için; programların
 * kaçır / url_kodla işlevleri runtime/ön_kütüphane.ohc'dedir). */
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

static const char *bul_n(const char *s, size_t n, const char *aranan) {
    size_t m = strlen(aranan);
    for (size_t i = 0; i + m <= n; i++)
        if (!memcmp(s + i, aranan, m)) return s + i;
    return NULL;
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

/* Stüdyo'nun canlı önizlemesi için (ORHUNCA_ONIZLEME): sayfa, Stüdyo'dan gelen
 * iletiyle bulunduğu adreste yenilenir ve adresini Stüdyo'ya bildirir. */
static const char ONIZLEME_BETIGI[] =
    "<script>(function(){var p=window.parent;if(p===window)return;"
    "addEventListener('message',function(e){if(e.source===p&&e.data==='orhunca:yenile')location.reload()});"
    "p.postMessage({orhunca:'adres',adres:decodeURI(location.pathname)+decodeURIComponent(location.search)},'*')"
    "})()</script>";
static int onizleme_acik = -1;

/* ASCII büyük/küçük harf ayrımı yapmadan ilk n bayt eşit mi (0: eşit) */
static int harf_duyarsiz_esit(const char *a, const char *b, size_t n) {
    for (size_t i = 0; i < n; i++) {
        char x = a[i], y = b[i];
        if (x >= 'A' && x <= 'Z') x = (char)(x + 32);
        if (y >= 'A' && y <= 'Z') y = (char)(y + 32);
        if (x != y) return 1;
        if (!x) return 0;
    }
    return 0;
}

/* ---------------------------------------------------------------------- */
/* Yanıtlar: her bağlantının gönderilecek baytları bir tampondadır.        */
/* ---------------------------------------------------------------------- */

/* Yanıtın ek başlıkları: Set-Cookie satırları ve bağlantının durumu. */
typedef struct {
    Tampon *cikti;
    int acik_kalsin; /* keep-alive */
    int bas_istegi;  /* HEAD: gövde gönderilmez */
    Tampon cerezler; /* "Set-Cookie: ...\r\n" satırları */
} Yanit;

static void yanit_yaz(Yanit *y, int64_t durum, const char *tur, const char *govde, size_t govde_n, const char *konum,
                      int64_t basliklar) {
    if (onizleme_acik < 0) onizleme_acik = getenv("ORHUNCA_ONIZLEME") != NULL;
    Tampon ek = {0};
    if (onizleme_acik && tur && !strncmp(tur, "text/html", 9)) {
        /* Betik </body>'den önce eklenir (yoksa sona). */
        const char *son = NULL;
        for (const char *p = govde; (p = bul_n(p, govde_n - (size_t)(p - govde), "</body>")); p++) son = p;
        size_t once = son ? (size_t)(son - govde) : govde_n;
        t_ekle(&ek, govde, once);
        t_yaz(&ek, ONIZLEME_BETIGI);
        t_ekle(&ek, govde + once, govde_n - once);
        govde = ek.v;
        govde_n = ek.n;
    }
    Tampon *t = y->cikti;
    char k[256];
    snprintf(k, sizeof k, "HTTP/1.1 %" PRId64 " %s\r\n", durum, durum_metni(durum));
    t_yaz(t, k);
    if (tur && *tur && durum != 204) {
        t_yaz(t, "Content-Type: ");
        baslik_degeri(t, tur);
        t_yaz(t, "\r\n");
    }
    snprintf(k, sizeof k, "Content-Length: %zu\r\n", durum == 204 ? (size_t)0 : govde_n);
    t_yaz(t, k);
    if (konum && *konum) {
        /* Türkçe karakterli adresler yüzde kodlanır. */
        Tampon u = {0};
        url_kodla(&u, konum, "-_.~/?#[]@!$&'()*+,;=:%");
        t_yaz(t, "Location: ");
        baslik_degeri(t, u.v ? u.v : "");
        t_yaz(t, "\r\n");
        free(u.v);
    }
    Sozluk *b = (Sozluk *)(intptr_t)basliklar;
    for (int64_t i = 0; b && i < b->uzunluk; i++) {
        baslik_degeri(t, M(b->anahtarlar[i]));
        t_yaz(t, ": ");
        baslik_degeri(t, M(b->degerler[i]));
        t_yaz(t, "\r\n");
    }
    if (y->cerezler.n) t_ekle(t, y->cerezler.v, y->cerezler.n);
    t_yaz(t, y->acik_kalsin ? "Connection: keep-alive\r\n\r\n" : "Connection: close\r\n\r\n");
    if (!y->bas_istegi && govde_n && durum != 204) t_ekle(t, govde, govde_n);
    free(ek.v);
}

static void hata_sayfasi(Yanit *y, int64_t durum, const char *baslik, const char *ayrinti) {
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
    yanit_yaz(y, durum, "text/html; charset=utf-8", t.v, t.n, NULL, 0);
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
static int statik_gonder(Yanit *y, const char *yol) {
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
    yanit_yaz(y, 200, icerik_turu(d.v), icerik.v ? icerik.v : "", icerik.n, NULL, 0);
    free(icerik.v);
    free(d.v);
    return 1;
}

static int64_t son_istek_basarili;

/* Yolun işlevini çalıştırır. Çalışma hatası olursa hata() buraya geri döner. */
static SATIR_ICI_DEGIL int64_t yolu_calistir(int64_t (*islev)(int64_t), int64_t istek) {
    Yakalayici y;
    son_istek_basarili = 0;
    y.onceki = yakalayici;
    y.istek = 1;
    if (TUZAK_KUR(y.tuzak)) return 0;
    yakalayici = &y;
    int64_t sonuc = islev(istek);
    yakalayici = y.onceki;
    son_istek_basarili = 1;
    return sonuc;
}

/* ---------------------------------------------------------------------- */
/* Rastgele baytlar (oturum kimlikleri, yükleme adları)                    */
/* ---------------------------------------------------------------------- */

static void rastgele_baytlar(unsigned char *b, size_t n) {
    int tamam = 0;
#ifdef _WIN32
    typedef BOOLEAN(WINAPI * RtlGenRandomT)(PVOID, ULONG);
    HMODULE m = LoadLibraryA("advapi32.dll");
    RtlGenRandomT f = m ? (RtlGenRandomT)(void *)GetProcAddress(m, "SystemFunction036") : NULL;
    if (f && f(b, (ULONG)n)) tamam = 1;
#else
    FILE *u = fopen("/dev/urandom", "rb");
    if (u) {
        tamam = fread(b, 1, n, u) == n;
        fclose(u);
    }
#endif
    if (!tamam)
        for (size_t i = 0; i < n; i++) b[i] = (unsigned char)(rng() >> 56);
}

static void rastgele_onaltilik(char *s, size_t bayt) {
    unsigned char b[64];
    rastgele_baytlar(b, bayt);
    for (size_t i = 0; i < bayt; i++) snprintf(s + 2 * i, 3, "%02x", b[i]);
}

/* ---------------------------------------------------------------------- */
/* Çerezler ve oturumlar                                                   */
/* ---------------------------------------------------------------------- */

#define OTURUM_CEREZI "orhunca_oturum"
#define OTURUM_OMRU (7 * 24 * 3600.0) /* son kullanımdan sonra */
/* sözlük<metin, metin> tip kodu */
#define KOD_METIN_SOZLUGU (5 + 8 * (1 + 2 * KOD_METIN))

/* Oturumlar sunucunun belleğinde JSON metni olarak durur (çöp toplayıcının
 * dışında); her istekte `istek.oturum` sözlüğüne açılır, sonra geri yazılır. */
typedef struct {
    char kimlik[33];
    char *json;
    double son;
} Oturum;

static Oturum *oturumlar;
static int64_t oturum_sayisi, oturum_kap;

static Oturum *oturum_bul(const char *kimlik) {
    if (strlen(kimlik) != 32) return NULL;
    for (int64_t i = 0; i < oturum_sayisi; i++)
        if (!strcmp(oturumlar[i].kimlik, kimlik)) return &oturumlar[i];
    return NULL;
}

static void oturum_sil(Oturum *o) {
    free(o->json);
    *o = oturumlar[--oturum_sayisi];
}

static void eski_oturumlari_sil(double simdi) {
    for (int64_t i = 0; i < oturum_sayisi;)
        if (simdi - oturumlar[i].son > OTURUM_OMRU)
            oturum_sil(&oturumlar[i]);
        else
            i++;
}

/* "a=1; b=iki" → sözlük (değerler yüzde kodlamasından çözülür) */
static void cerezleri_coz(int64_t sozluk, const char *s) {
    while (*s) {
        while (*s == ' ' || *s == ';') s++;
        const char *bas = s;
        while (*s && *s != ';') s++;
        const char *esit = memchr(bas, '=', (size_t)(s - bas));
        if (esit && esit > bas) {
            const char *ad_son = esit;
            while (ad_son > bas && ad_son[-1] == ' ') ad_son--;
            Tampon d = {0};
            yuzde_coz(&d, esit + 1, (size_t)(s - esit - 1), 0);
            int64_t am = metin_yap(bas, (size_t)(ad_son - bas));
            int64_t dm = t_metin(&d);
            ohc_sozluk_koy(sozluk, am, dm, KOD_METIN);
        }
    }
}

static void cerez_ekle(Tampon *t, const char *ad, const char *deger, int tls, int http_only) {
    t_yaz(t, "Set-Cookie: ");
    url_kodla(t, ad, "-_.~");
    t_yaz(t, "=");
    url_kodla(t, deger, "-_.~");
    t_yaz(t, "; Path=/; SameSite=Lax");
    if (!*deger) t_yaz(t, "; Max-Age=0");
    if (http_only) t_yaz(t, "; HttpOnly");
    if (tls) t_yaz(t, "; Secure");
    t_yaz(t, "\r\n");
}

/* ---------------------------------------------------------------------- */
/* Dosya yükleme (multipart/form-data)                                    */
/* ---------------------------------------------------------------------- */

static const char *bul_bellek(const char *s, size_t n, const char *aranan, size_t m) {
    for (size_t i = 0; i + m <= n; i++)
        if (s[i] == aranan[0] && !memcmp(s + i, aranan, m)) return s + i;
    return NULL;
}

/* Başlık değerinden `ad="değer"` parametresi */
static int baslik_parametresi(const char *b, size_t n, const char *ad, Tampon *deger) {
    size_t an = strlen(ad);
    for (size_t i = 0; i + an + 1 < n; i++) {
        if ((i == 0 || b[i - 1] == ' ' || b[i - 1] == ';') && !memcmp(b + i, ad, an) && b[i + an] == '=') {
            size_t j = i + an + 1;
            if (j < n && b[j] == '"') {
                j++;
                size_t k = j;
                while (k < n && b[k] != '"') k++;
                t_ekle(deger, b + j, k - j);
            } else {
                size_t k = j;
                while (k < n && b[k] != ';' && b[k] != ' ') k++;
                t_ekle(deger, b + j, k - j);
            }
            t_ekle(deger, "", 0);
            return 1;
        }
    }
    return 0;
}

/* Yüklenen dosyanın adından güvenli bir dosya adı: yalnızca son parça, denetim
 * karakterleri ve yol ayraçları olmadan. */
static void guvenli_ad(Tampon *t, const char *ad) {
    const char *p = ad;
    for (const char *q = ad; *q; q++)
        if (*q == '/' || *q == '\\') p = q + 1;
    size_t n = 0;
    for (; *p && n < 100; p++) {
        unsigned char c = (unsigned char)*p;
        if (c < 32 || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|') continue;
        if (n == 0 && c == '.') continue;
        t_ekle(t, p, 1);
        n++;
    }
    if (!n) t_yaz(t, "dosya");
}

/* Gövdeyi parçalara ayırır: metin alanları `form`a, dosyalar yükleme klasörüne
 * kaydedilip `dosyalar` sözlüğüne (YüklenenDosya) girer. */
static void parcali_formu_coz(const char *govde, size_t n, const char *tur, int64_t form, int64_t dosyalar) {
    Tampon sinir = {0};
    if (!baslik_parametresi(tur, strlen(tur), "boundary", &sinir)) return;
    Tampon ayrac = {0};
    t_yaz(&ayrac, "--");
    t_yaz(&ayrac, sinir.v);
    free(sinir.v);
    ModelBilgisi *dm = model_bilgisi_adla("YüklenenDosya");
    const char *p = bul_bellek(govde, n, ayrac.v, ayrac.n);
    while (p) {
        p += ayrac.n;
        if ((size_t)(govde + n - p) >= 2 && p[0] == '-' && p[1] == '-') break; /* son */
        if ((size_t)(govde + n - p) >= 2 && p[0] == '\r' && p[1] == '\n') p += 2;
        const char *baslik_sonu = bul_bellek(p, (size_t)(govde + n - p), "\r\n\r\n", 4);
        if (!baslik_sonu) break;
        const char *icerik = baslik_sonu + 4;
        const char *sonraki = bul_bellek(icerik, (size_t)(govde + n - icerik), ayrac.v, ayrac.n);
        if (!sonraki) break;
        size_t icerik_n = (size_t)(sonraki - icerik);
        if (icerik_n >= 2 && sonraki[-2] == '\r' && sonraki[-1] == '\n') icerik_n -= 2;
        /* Parçanın başlıkları */
        Tampon ad = {0}, dosya_adi = {0}, parca_turu = {0};
        int dosya_mi = 0;
        for (const char *s = p; s < baslik_sonu;) {
            const char *e = bul_bellek(s, (size_t)(baslik_sonu - s), "\r\n", 2);
            if (!e) e = baslik_sonu;
            size_t sn = (size_t)(e - s);
            if (sn > 20 && !harf_duyarsiz_esit(s, "content-disposition:", 20)) {
                baslik_parametresi(s + 20, sn - 20, "name", &ad);
                dosya_mi = baslik_parametresi(s + 20, sn - 20, "filename", &dosya_adi);
            } else if (sn > 13 && !harf_duyarsiz_esit(s, "content-type:", 13)) {
                const char *v = s + 13;
                while (*v == ' ') v++;
                t_ekle(&parca_turu, v, (size_t)(e - v));
                t_ekle(&parca_turu, "", 0);
            }
            s = e + 2;
        }
        if (ad.v && !dosya_mi) {
            int64_t am = metin_yap(ad.v, strlen(ad.v));
            int64_t dm2 = metin_yap(icerik, icerik_n);
            ohc_sozluk_koy(form, am, dm2, KOD_METIN);
        } else if (ad.v && dosya_mi && dm && (icerik_n > 0 || (dosya_adi.v && *dosya_adi.v))) {
            /* <veri>/yüklemeler/<rastgele>-<ad> */
            Tampon yol = {0};
            t_yaz(&yol, veri_klasoru());
            klasor_olustur(yol.v);
            t_yaz(&yol, "/yüklemeler");
            klasor_olustur(yol.v);
            char r[17];
            rastgele_onaltilik(r, 8);
            t_yaz(&yol, "/");
            t_yaz(&yol, r);
            t_yaz(&yol, "-");
            guvenli_ad(&yol, dosya_adi.v ? dosya_adi.v : "");
            FILE *f = dosya_ac(yol.v, "wb");
            if (f) {
                fwrite(icerik, 1, icerik_n, f);
                fclose(f);
                int64_t d = nesne_yeni(dm->tanim);
                int64_t i;
                if ((i = alan_sirasi(dm, "ad")) >= 0) ALAN(d, i) = metin_yap(dosya_adi.v ? dosya_adi.v : "", dosya_adi.v ? strlen(dosya_adi.v) : 0);
                if ((i = alan_sirasi(dm, "tür")) >= 0)
                    ALAN(d, i) = parca_turu.v ? metin_yap(parca_turu.v, strlen(parca_turu.v)) : D("application/octet-stream");
                if ((i = alan_sirasi(dm, "yol")) >= 0) ALAN(d, i) = metin_yap(yol.v, yol.n);
                if ((i = alan_sirasi(dm, "boyut")) >= 0) ALAN(d, i) = (int64_t)icerik_n;
                ohc_sozluk_koy(dosyalar, metin_yap(ad.v, strlen(ad.v)), d, KOD_METIN);
            }
            free(yol.v);
        }
        free(ad.v);
        free(dosya_adi.v);
        free(parca_turu.v);
        p = sonraki;
    }
    free(ayrac.v);
}

/* ---------------------------------------------------------------------- */
/* İstek                                                                   */
/* ---------------------------------------------------------------------- */

static int64_t en_buyuk_govde = -1;

/* Tamponda tam bir istek var mı? Varsa toplam bayt sayısı (başlıklar + gövde),
 * eksikse 0, hatalıysa -1 (431: başlıklar, -2: 413 gövde çok büyük). Parçalı
 * (chunked) gövde `cozulen`e açılır. */
static int64_t istek_tamam_mi(const char *v, size_t n, Tampon *cozulen, size_t *govde_basi) {
    const char *son = bul_n(v, n, "\r\n\r\n");
    if (!son) return n > 65536 ? -1 : 0;
    if (en_buyuk_govde < 0) {
        const char *e = getenv("ORHUNCA_EN_BUYUK_GOVDE");
        en_buyuk_govde = e && atoll(e) > 0 ? atoll(e) : 32 * 1024 * 1024;
    }
    size_t bas = (size_t)(son - v) + 4;
    *govde_basi = bas;
    int64_t uzunluk = 0;
    int parcali = 0;
    for (const char *p = v; p < son;) {
        const char *e = bul_n(p, (size_t)(son - p) + 2, "\r\n");
        if (!e) break;
        if ((size_t)(e - p) > 15 && !harf_duyarsiz_esit(p, "content-length:", 15)) uzunluk = strtoll(p + 15, NULL, 10);
        if ((size_t)(e - p) > 18 && !harf_duyarsiz_esit(p, "transfer-encoding:", 18) && bul_n(p, (size_t)(e - p), "chunked"))
            parcali = 1;
        p = e + 2;
    }
    if (uzunluk < 0 || uzunluk > en_buyuk_govde) return -2;
    if (!parcali) return n >= bas + (size_t)uzunluk ? (int64_t)(bas + (size_t)uzunluk) : 0;
    /* Parçalı gövde: <onaltılık boy>\r\n<veri>\r\n ... 0\r\n\r\n */
    cozulen->n = 0;
    size_t i = bas;
    for (;;) {
        const char *e = bul_n(v + i, n - i, "\r\n");
        if (!e) return 0;
        long long boy = strtoll(v + i, NULL, 16);
        if (boy < 0 || (int64_t)cozulen->n + boy > en_buyuk_govde) return -2;
        i = (size_t)(e - v) + 2;
        if (boy == 0) {
            const char *bitis = bul_n(v + i, n - i, "\r\n");
            /* Sondaki başlıklar (trailer) yok sayılır. */
            while (bitis && bitis != v + i) {
                i = (size_t)(bitis - v) + 2;
                bitis = bul_n(v + i, n - i, "\r\n");
            }
            if (!bitis) return 0;
            return (int64_t)(i + 2);
        }
        if (n < i + (size_t)boy + 2) return 0;
        t_ekle(cozulen, v + i, (size_t)boy);
        i += (size_t)boy + 2;
    }
}

/* Tam bir isteği işler, yanıtı `y->cikti`ya yazar. `v`/`n`: isteğin baytları. */
static void istegi_isle(Yanit *y, char *v, size_t n, size_t govde_basi, const char *govde_v, size_t govde_n,
                        const char *istek_tanimi, int tls) {
    double bas_zaman = ondalik(ohc_zaman());
    (void)n;
    const char *baslik_sonu = v + govde_basi - 4;
    /* İstek satırı: YÖNTEM HEDEF SÜRÜM */
    char *satir_sonu = strstr(v, "\r\n");
    *satir_sonu = 0;
    char yontem[16] = {0};
    char *bosluk1 = strchr(v, ' ');
    char *bosluk2 = bosluk1 ? strchr(bosluk1 + 1, ' ') : NULL;
    if (!bosluk1 || !bosluk2 || bosluk1 - v >= (long)sizeof yontem || bosluk1[1] != '/') {
        y->acik_kalsin = 0;
        hata_sayfasi(y, 400, "Geçersiz istek", NULL);
        return;
    }
    memcpy(yontem, v, (size_t)(bosluk1 - v));
    *bosluk2 = 0;
    const char *surum = bosluk2 + 1;
    const char *hedef = bosluk1 + 1;
    y->bas_istegi = !strcmp(yontem, "HEAD");
    /* HTTP/1.1'de bağlantı açık kalır (Connection: close değilse); 1.0'da kapanır. */
    y->acik_kalsin = !strcmp(surum, "HTTP/1.1");

    /* Başlıklar */
    int64_t basliklar = ohc_sozluk_yeni();
    ((Sozluk *)(intptr_t)basliklar)->anahtar_kodu = KOD_METIN;
    char *p = satir_sonu + 2;
    while (p < baslik_sonu) {
        char *son = strstr(p, "\r\n");
        if (!son) break;
        *son = 0;
        char *iki = strchr(p, ':');
        if (iki) {
            for (char *c = p; c < iki; c++)
                if (*c >= 'A' && *c <= 'Z') *c = (char)(*c + 32);
            char *d = iki + 1;
            while (*d == ' ' || *d == '\t') d++;
            size_t dn = strlen(d);
            while (dn && (d[dn - 1] == ' ' || d[dn - 1] == '\t')) dn--;
            int64_t am = metin_yap(p, (size_t)(iki - p));
            int64_t dm = metin_yap(d, dn);
            ohc_sozluk_koy(basliklar, am, dm, KOD_METIN);
            if (!strcmp(M(am), "connection")) {
                if (strstr(M(dm), "close") || strstr(M(dm), "Close")) y->acik_kalsin = 0;
                if (strstr(M(dm), "keep-alive") || strstr(M(dm), "Keep-Alive")) y->acik_kalsin = 1;
            }
        }
        p = son + 2;
    }
    int64_t govde = metin_yap(govde_v, govde_n);

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
    if ((!strcmp(yontem, "GET") || y->bas_istegi) && statik_gonder(y, M(yol))) {
        durum = 200;
    } else {
        /* En çok sabit parçası olan eşleşen yol seçilir. */
        WebYolu *secilen = NULL;
        int en_iyi = 0, yontem_farkli = 0;
        for (int64_t i = 0; i < web_yolu_sayisi; i++) {
            int e = yol_eslesir(web_yollari[i].kalip, M(yol), 0);
            if (!e) continue;
            const char *wy = web_yollari[i].yontem;
            if (strcmp(wy, yontem) && !(y->bas_istegi && !strcmp(wy, "GET"))) {
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
            hata_sayfasi(y, durum, yontem_farkli ? "Bu adres bu yöntemle kullanılamaz" : "Sayfa bulunamadı", a.v);
            free(a.v);
        } else {
            int64_t form = ohc_sozluk_yeni();
            ((Sozluk *)(intptr_t)form)->anahtar_kodu = KOD_METIN;
            int64_t dosyalar = ohc_sozluk_yeni();
            ((Sozluk *)(intptr_t)dosyalar)->anahtar_kodu = KOD_METIN;
            int64_t tur = ohc_sozluk_icerir(basliklar, D("content-type"), KOD_METIN)
                              ? ohc_sozluk_al(basliklar, D("content-type"), KOD_METIN, 0)
                              : D("");
            if (strstr(M(tur), "application/x-www-form-urlencoded"))
                form_coz(form, M(govde), strlen(M(govde)));
            else if (strstr(M(tur), "multipart/form-data")) {
                parcali_formu_coz(govde_v, govde_n, M(tur), form, dosyalar);
                govde = D("");
            } else if (strstr(M(tur), "json"))
                json_formu(form, M(govde));
            int64_t parametreler = ohc_sozluk_yeni();
            ((Sozluk *)(intptr_t)parametreler)->anahtar_kodu = KOD_METIN;
            yol_eslesir(secilen->kalip, M(yol), parametreler);

            /* Çerezler ve oturum */
            int64_t cerezler = ohc_sozluk_yeni();
            ((Sozluk *)(intptr_t)cerezler)->anahtar_kodu = KOD_METIN;
            if (ohc_sozluk_icerir(basliklar, D("cookie"), KOD_METIN))
                cerezleri_coz(cerezler, M(ohc_sozluk_al(basliklar, D("cookie"), KOD_METIN, 0)));
            double simdi = ondalik(ohc_zaman());
            eski_oturumlari_sil(simdi);
            Oturum *o = ohc_sozluk_icerir(cerezler, D(OTURUM_CEREZI), KOD_METIN)
                            ? oturum_bul(M(ohc_sozluk_al(cerezler, D(OTURUM_CEREZI), KOD_METIN, 0)))
                            : NULL;
            char kimlik[33] = {0};
            if (o) memcpy(kimlik, o->kimlik, 33);
            int64_t oturum;
            if (o && o->json) {
                Json j = {o->json, NULL, 0};
                oturum = j_deger(&j, KOD_METIN_SOZLUGU, NULL);
            } else {
                oturum = ohc_sozluk_yeni();
                ((Sozluk *)(intptr_t)oturum)->anahtar_kodu = KOD_METIN;
            }

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
            ISTEK_KOY("çerezler", cerezler);
            ISTEK_KOY("oturum", oturum);
            ISTEK_KOY("dosyalar", dosyalar);
#undef ISTEK_KOY
            int64_t sonuc = yolu_calistir(secilen->islev, istek);
            if (son_istek_basarili) {
                /* Oturum geri yazılır: boşaldıysa silinir; yeni ise çerezle verilir. */
                int64_t i_ = alan_sirasi(im, "oturum");
                int64_t son_oturum = i_ >= 0 ? ALAN(istek, i_) : 0;
                int bos = !son_oturum || ohc_sozluk_uzunluk(son_oturum) == 0;
                o = kimlik[0] ? oturum_bul(kimlik) : NULL;
                if (bos) {
                    if (o) {
                        oturum_sil(o);
                        cerez_ekle(&y->cerezler, OTURUM_CEREZI, "", tls, 1);
                    }
                } else {
                    if (!o) {
                        if (oturum_sayisi == oturum_kap) {
                            oturum_kap = oturum_kap ? oturum_kap * 2 : 16;
                            oturumlar = ham_buyut(oturumlar, sizeof(Oturum) * (size_t)oturum_kap);
                        }
                        o = &oturumlar[oturum_sayisi++];
                        rastgele_onaltilik(o->kimlik, 16);
                        o->json = NULL;
                        cerez_ekle(&y->cerezler, OTURUM_CEREZI, o->kimlik, tls, 1);
                    }
                    Tampon jt = {0};
                    json_yaz(&jt, son_oturum, KOD_METIN_SOZLUGU, 0);
                    t_ekle(&jt, "", 0);
                    free(o->json);
                    o->json = jt.v;
                    o->son = simdi;
                }
            }
            if (!son_istek_basarili) {
                durum = 500;
                hata_sayfasi(y, 500, "Sunucu hatası", son_hata);
            } else if (!sonuc) {
                durum = 204;
                yanit_yaz(y, 204, NULL, "", 0, NULL, 0);
            } else {
                ModelBilgisi *ym = nesne_bilgisi(sonuc);
                int64_t i;
                durum = (i = alan_sirasi(ym, "durum")) >= 0 ? ALAN(sonuc, i) : 200;
                const char *tur_y = (i = alan_sirasi(ym, "tür")) >= 0 ? M(ALAN(sonuc, i)) : "text/html; charset=utf-8";
                const char *g = (i = alan_sirasi(ym, "gövde")) >= 0 ? M(ALAN(sonuc, i)) : "";
                const char *konum = (i = alan_sirasi(ym, "konum")) >= 0 ? M(ALAN(sonuc, i)) : "";
                int64_t ek = (i = alan_sirasi(ym, "başlıklar")) >= 0 ? ALAN(sonuc, i) : 0;
                Sozluk *c = (i = alan_sirasi(ym, "çerezler")) >= 0 ? (Sozluk *)(intptr_t)ALAN(sonuc, i) : NULL;
                for (int64_t k = 0; c && k < c->uzunluk; k++)
                    cerez_ekle(&y->cerezler, M(c->anahtarlar[k]), M(c->degerler[k]), tls, 0);
                yanit_yaz(y, durum, tur_y, g, strlen(g), konum, ek);
            }
        }
    }
    double ms = (ondalik(ohc_zaman()) - bas_zaman) * 1000.0;
    printf("  %s %s → %" PRId64 " · %.0f ms\n", yontem, M(gorunen), durum, ms);
    fflush(stdout);
}

static int surec_yasiyor(long kimlik) {
#ifdef _WIN32
    HANDLE h = OpenProcess(SYNCHRONIZE, FALSE, (DWORD)kimlik);
    if (!h) return 0;
    DWORD d = WaitForSingleObject(h, 0);
    CloseHandle(h);
    return d == WAIT_TIMEOUT;
#else
    /* Ebeveyn ölünce süreç başka bir sürece bağlanır; ölen ebeveyn zombi
     * olarak kalsa bile getppid() değişir. */
    return getppid() == (pid_t)kimlik;
#endif
}

/* ---------------------------------------------------------------------- */
/* HTTPS: OpenSSL (libssl 3 / 1.1) çalışma anında yüklenir; derlemede      */
/* bağımlılık gerekmez. ORHUNCA_SERTIFIKA ve ORHUNCA_ANAHTAR verilirse     */
/* sunucu HTTPS konuşur.                                                   */
/* ---------------------------------------------------------------------- */

typedef struct {
    void *(*TLS_server_method)(void);
    void *(*SSL_CTX_new)(void *);
    int (*SSL_CTX_use_certificate_chain_file)(void *, const char *);
    int (*SSL_CTX_use_PrivateKey_file)(void *, const char *, int);
    long (*SSL_CTX_ctrl)(void *, int, long, void *);
    void *(*SSL_new)(void *);
    int (*SSL_set_fd)(void *, int);
    int (*SSL_accept)(void *);
    int (*SSL_read)(void *, void *, int);
    int (*SSL_write)(void *, const void *, int);
    int (*SSL_get_error)(const void *, int);
    int (*SSL_shutdown)(void *);
    void (*SSL_free)(void *);
} TlsIslevleri;

static TlsIslevleri tls;
static void *tls_baglami; /* SSL_CTX */

#define TLS_HATA_OKUMA_BEKLE 2
#define TLS_HATA_YAZMA_BEKLE 3
#define TLS_HATA_SIFIR 6

#ifdef _WIN32
#define KUTUPHANE_AC(ad) ((void *)LoadLibraryA(ad))
#define KUTUPHANE_ISLEVI(k, ad) ((void *)GetProcAddress((HMODULE)(k), ad))
#else
#define KUTUPHANE_AC(ad) dlopen(ad, RTLD_NOW | RTLD_GLOBAL)
#define KUTUPHANE_ISLEVI(k, ad) dlsym(k, ad)
#endif

static void tls_kur(const char *sertifika, const char *anahtar) {
    static const char *adlar[] = {
#ifdef _WIN32
        "libssl-3-x64.dll", "libssl-3.dll", "libssl-1_1-x64.dll",
#elif defined(__APPLE__)
        "libssl.3.dylib", "/opt/homebrew/opt/openssl@3/lib/libssl.3.dylib", "/usr/local/opt/openssl@3/lib/libssl.3.dylib",
        "libssl.dylib",
#else
        "libssl.so.3", "libssl.so.1.1", "libssl.so",
#endif
    };
    void *k = NULL;
    for (size_t i = 0; i < sizeof adlar / sizeof *adlar && !k; i++) k = KUTUPHANE_AC(adlar[i]);
    if (!k) hata(0, "HTTPS için OpenSSL kütüphanesi (libssl) bulunamadı; kurun ya da ORHUNCA_SERTIFIKA'yı kaldırın");
#define TLS_BAGLA(ad)                                                                                                  \
    do {                                                                                                               \
        *(void **)&tls.ad = KUTUPHANE_ISLEVI(k, #ad);                                                                  \
        if (!tls.ad) hata(0, "OpenSSL kütüphanesinde " #ad " bulunamadı (OpenSSL 1.1 ya da 3 gerekir)");                 \
    } while (0)
    TLS_BAGLA(TLS_server_method);
    TLS_BAGLA(SSL_CTX_new);
    TLS_BAGLA(SSL_CTX_use_certificate_chain_file);
    TLS_BAGLA(SSL_CTX_use_PrivateKey_file);
    TLS_BAGLA(SSL_CTX_ctrl);
    TLS_BAGLA(SSL_new);
    TLS_BAGLA(SSL_set_fd);
    TLS_BAGLA(SSL_accept);
    TLS_BAGLA(SSL_read);
    TLS_BAGLA(SSL_write);
    TLS_BAGLA(SSL_get_error);
    TLS_BAGLA(SSL_shutdown);
    TLS_BAGLA(SSL_free);
#undef TLS_BAGLA
    tls_baglami = tls.SSL_CTX_new(tls.TLS_server_method());
    if (!tls_baglami) hata(0, "TLS bağlamı oluşturulamadı");
    if (tls.SSL_CTX_use_certificate_chain_file(tls_baglami, sertifika) != 1) {
        char m[600];
        snprintf(m, sizeof m, "sertifika okunamadı: %.500s (PEM biçiminde olmalı)", sertifika);
        hata(0, m);
    }
    if (tls.SSL_CTX_use_PrivateKey_file(tls_baglami, anahtar, 1 /* PEM */) != 1) {
        char m[600];
        snprintf(m, sizeof m, "özel anahtar okunamadı ya da sertifikayla uyuşmuyor: %.500s", anahtar);
        hata(0, m);
    }
    /* SSL_CTX_set_mode: kısmi yazma ve yer değiştiren yazma tamponu */
    tls.SSL_CTX_ctrl(tls_baglami, 33, 1 | 2, NULL);
}

/* ---------------------------------------------------------------------- */
/* Bağlantılar ve olay döngüsü                                             */
/* ---------------------------------------------------------------------- */

/* Sunucu tek iş parçacığında çalışır ama bağlantıları beklemez: bütün
 * bağlantılar bir olay döngüsünde (poll) birlikte okunur ve yazılır; yavaş bir
 * istemci ötekileri bekletmez. Bir istek tamamen gelince programın yolu
 * çalıştırılır (yollar sırayla çalışır; çöp toplayıcı ve kayıt dosyaları
 * tek iş parçacığında güvendedir). HTTP/1.1 bağlantıları açık kalabilir. */
typedef struct {
    Soket s;
    void *ssl;
    int el_sikisma;   /* TLS el sıkışması sürüyor */
    int tls_bekle;    /* TLS yazma yerine okuma (ya da tersi) bekliyor: POLLIN/POLLOUT */
    Tampon gelen;
    Tampon giden;
    size_t gonderilen;
    int kapanacak;    /* yanıt gidince kapanır */
    double son_etkinlik;
} Baglanti;

#define BOSTA_KALMA_SURESI 30.0
#define YARIM_ISTEK_SURESI 15.0

static void engelsiz_yap(Soket s) {
#ifdef _WIN32
    u_long bir = 1;
    ioctlsocket(s, FIONBIO, &bir);
#else
    fcntl(s, F_SETFL, fcntl(s, F_GETFL, 0) | O_NONBLOCK);
#endif
}

static int beklemeli_hata(void) {
#ifdef _WIN32
    int e = WSAGetLastError();
    return e == WSAEWOULDBLOCK || e == WSAEINTR;
#else
    return errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR;
#endif
}

static void baglanti_kapat(Baglanti *b) {
    if (b->ssl) {
        tls.SSL_shutdown(b->ssl);
        tls.SSL_free(b->ssl);
    }
    soket_kapat(b->s);
    free(b->gelen.v);
    free(b->giden.v);
}

/* Okuyabildiği kadar okur. 0: bağlantı bitti ya da hata. */
static int baglanti_oku(Baglanti *b) {
    char parca[16384];
    for (;;) {
        int k;
        if (b->ssl) {
            k = tls.SSL_read(b->ssl, parca, sizeof parca);
            if (k <= 0) {
                int e = tls.SSL_get_error(b->ssl, k);
                if (e == TLS_HATA_OKUMA_BEKLE) return 1;
                if (e == TLS_HATA_YAZMA_BEKLE) {
                    b->tls_bekle = POLLOUT;
                    return 1;
                }
                return 0;
            }
        } else {
            k = (int)recv(b->s, parca, sizeof parca, 0);
            if (k < 0) return beklemeli_hata();
            if (k == 0) return 0;
        }
        t_ekle(&b->gelen, parca, (size_t)k);
        if (b->gelen.n > (size_t)en_buyuk_govde + 70000) return 1;
    }
}

/* Gönderebildiği kadar gönderir. 0: hata. */
static int baglanti_yaz(Baglanti *b) {
    while (b->gonderilen < b->giden.n) {
        size_t kalan = b->giden.n - b->gonderilen;
        int parca = kalan > 1048576 ? 1048576 : (int)kalan;
        int k;
        if (b->ssl) {
            k = tls.SSL_write(b->ssl, b->giden.v + b->gonderilen, parca);
            if (k <= 0) {
                int e = tls.SSL_get_error(b->ssl, k);
                if (e == TLS_HATA_YAZMA_BEKLE) return 1;
                if (e == TLS_HATA_OKUMA_BEKLE) {
                    b->tls_bekle = POLLIN;
                    return 1;
                }
                return 0;
            }
        } else {
            k = (int)send(b->s, b->giden.v + b->gonderilen, parca, 0);
            if (k < 0) return beklemeli_hata();
        }
        b->gonderilen += (size_t)k;
    }
    b->giden.n = 0;
    b->gonderilen = 0;
    return 1;
}

/* Tamponda tam bir istek varsa işler ve yanıtı hazırlar. */
static void istek_varsa_isle(Baglanti *b, const char *istek_tanimi) {
    if (b->giden.n || b->kapanacak || !b->gelen.n) return;
    Tampon cozulen = {0};
    size_t govde_basi = 0;
    int64_t boy = istek_tamam_mi(b->gelen.v, b->gelen.n, &cozulen, &govde_basi);
    Yanit y = {&b->giden, 0, 0, {0}};
    if (boy == 0) {
        free(cozulen.v);
        return;
    }
    if (boy < 0) {
        b->kapanacak = 1;
        if (boy == -1)
            hata_sayfasi(&y, 431, "İstek başlıkları çok büyük", NULL);
        else
            hata_sayfasi(&y, 413, "İstek gövdesi çok büyük", "ORHUNCA_EN_BUYUK_GOVDE ile sınır (bayt) değiştirilebilir");
        free(cozulen.v);
        return;
    }
    /* İstek, tamponun başında NUL ile biten ayrı bir kopyada işlenir. */
    char *v = ham_ayir((size_t)boy + 1);
    memcpy(v, b->gelen.v, (size_t)boy);
    v[boy] = 0;
    int parcali = cozulen.v != NULL || bul_n(v, govde_basi, "chunked") != NULL;
    const char *govde_v = parcali ? (cozulen.v ? cozulen.v : "") : v + govde_basi;
    size_t govde_n = parcali ? cozulen.n : (size_t)boy - govde_basi;
    istegi_isle(&y, v, (size_t)boy, govde_basi, govde_v, govde_n, istek_tanimi, b->ssl != NULL);
    if (!y.acik_kalsin) b->kapanacak = 1;
    free(y.cerezler.v);
    free(v);
    free(cozulen.v);
    memmove(b->gelen.v, b->gelen.v + boy, b->gelen.n - (size_t)boy);
    b->gelen.n -= (size_t)boy;
}

/* Sunucuyu başlatır ve istekleri karşılar (dönmez). */
void ohc_sun(int64_t kapi, int64_t istek_tanimi) {
    /* ORHUNCA_KAPI programdaki kapıyı geçersiz kılar; 0 ise boş bir kapı seçilir. */
    const char *e = getenv("ORHUNCA_KAPI");
    int otomatik = e && !strcmp(e, "0");
    if (e && atol(e) > 0) kapi = atol(e);
    if (kapi <= 0 || kapi > 65535) kapi = 3000;
    if (otomatik) kapi = 0;
    const char *adres = getenv("ORHUNCA_ADRES");
    if (!adres || !*adres) adres = "127.0.0.1";
    const char *sertifika = getenv("ORHUNCA_SERTIFIKA");
    const char *anahtar = getenv("ORHUNCA_ANAHTAR");
    int https = sertifika && *sertifika;
    if (https) tls_kur(sertifika, anahtar && *anahtar ? anahtar : sertifika);
    if (en_buyuk_govde < 0) {
        const char *g = getenv("ORHUNCA_EN_BUYUK_GOVDE");
        en_buyuk_govde = g && atoll(g) > 0 ? atoll(g) : 32 * 1024 * 1024;
    }
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
    if (bind(dinleyici, (struct sockaddr *)&a, sizeof a) != 0 || listen(dinleyici, 128) != 0) {
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
    engelsiz_yap(dinleyici);
    printf("● Sunucu dinleniyor: %s://localhost:%" PRId64 "\n", https ? "https" : "http", kapi);
    fflush(stdout);
    /* Stüdyo'dan başlatıldıysa Stüdyo kapanınca sunucu da kapanır. */
    const char *ebeveyn = getenv("ORHUNCA_EBEVEYN");
    long ebeveyn_kimligi = ebeveyn ? atol(ebeveyn) : 0;

    Baglanti *baglantilar = NULL;
    size_t sayi = 0, kap = 0;
    struct pollfd *pler = NULL;
    size_t pkap = 0;
    double son_denetim = 0;
    for (;;) {
        if (pkap < sayi + 1) {
            pkap = (sayi + 1) * 2;
            pler = ham_buyut(pler, sizeof *pler * pkap);
        }
        pler[0].fd = dinleyici;
        pler[0].events = POLLIN;
        pler[0].revents = 0;
        for (size_t i = 0; i < sayi; i++) {
            Baglanti *b = &baglantilar[i];
            short olay;
            if (b->tls_bekle)
                olay = (short)b->tls_bekle;
            else if (b->el_sikisma)
                olay = POLLIN;
            else
                olay = b->giden.n ? POLLOUT : POLLIN;
            pler[i + 1].fd = b->s;
            pler[i + 1].events = olay;
            pler[i + 1].revents = 0;
        }
#ifdef _WIN32
        int hazir = WSAPoll(pler, (ULONG)(sayi + 1), 1000);
#else
        int hazir = poll(pler, (nfds_t)(sayi + 1), 1000);
#endif
        double simdi = ondalik(ohc_zaman());
        if (simdi - son_denetim >= 1.0) {
            son_denetim = simdi;
            if (ebeveyn_kimligi > 0 && !surec_yasiyor(ebeveyn_kimligi)) exit(0);
        }
        if (hazir < 0) continue;
        /* Bekleme sırasındaki bağlantılar (yeni kabul edilenlerin olayı yoktur) */
        size_t onceki = sayi;
        if (pler[0].revents & POLLIN) {
            for (;;) {
                Soket s = accept(dinleyici, NULL, NULL);
                if (s == GECERSIZ_SOKET) break;
                engelsiz_yap(s);
                if (sayi == kap) {
                    kap = kap ? kap * 2 : 16;
                    baglantilar = ham_buyut(baglantilar, sizeof *baglantilar * kap);
                }
                Baglanti *b = &baglantilar[sayi++];
                memset(b, 0, sizeof *b);
                b->s = s;
                b->son_etkinlik = simdi;
                if (https) {
                    b->ssl = tls.SSL_new(tls_baglami);
                    tls.SSL_set_fd(b->ssl, (int)s);
                    b->el_sikisma = 1;
                }
            }
        }
        for (size_t i = 0; i < sayi; i++) {
            Baglanti *b = &baglantilar[i];
            short r = i < onceki ? pler[i + 1].revents : 0;
            int canli = 1;
            if (r) {
                b->son_etkinlik = simdi;
                b->tls_bekle = 0;
            }
            if (r && b->el_sikisma) {
                int k = tls.SSL_accept(b->ssl);
                if (k == 1) {
                    b->el_sikisma = 0;
                } else {
                    int e2 = tls.SSL_get_error(b->ssl, k);
                    if (e2 == TLS_HATA_YAZMA_BEKLE)
                        b->tls_bekle = POLLOUT;
                    else if (e2 != TLS_HATA_OKUMA_BEKLE)
                        canli = 0;
                }
                r = 0;
            }
            if (canli && (r & (POLLERR | POLLNVAL))) canli = 0;
            if (canli && (r & (POLLIN | POLLHUP)) && !b->giden.n) {
                /* Karşı taraf kapattıysa elde kalan tam istek yine yanıtlanır. */
                int acik = baglanti_oku(b);
                istek_varsa_isle(b, M(istek_tanimi));
                if (!acik) {
                    if (b->giden.n)
                        b->kapanacak = 1;
                    else
                        canli = 0;
                }
            }
            if (canli && b->giden.n) {
                if (!baglanti_yaz(b))
                    canli = 0;
                else if (!b->giden.n) {
                    if (b->kapanacak)
                        canli = 0;
                    else
                        istek_varsa_isle(b, M(istek_tanimi)); /* ardışık (pipelined) istek */
                }
            }
            double sinir = b->gelen.n ? YARIM_ISTEK_SURESI : BOSTA_KALMA_SURESI;
            if (canli && simdi - b->son_etkinlik > sinir) canli = 0;
            if (!canli) {
                baglanti_kapat(b);
                b->s = GECERSIZ_SOKET;
            }
        }
        size_t j = 0;
        for (size_t i = 0; i < sayi; i++)
            if (baglantilar[i].s != GECERSIZ_SOKET) baglantilar[j++] = baglantilar[i];
        sayi = j;
    }
}

/* ====================================================================== */
/* Hata ayıklama (Stüdyo)                                                  */
/* ====================================================================== */

/* `--ayıklama` ile derlenen programlarda derleyici her deyimden önce
 * ohc_ay_satir'ı, her işlevin başında ve sonunda ohc_ay_gir/ohc_ay_cik'ı
 * çağırır. ORHUNCA_AYIKLA ortam değişkenindeki kapıya (127.0.0.1) bağlanılır;
 * Stüdyo kesme noktalarını ve komutları satır satır gönderir:
 *   kesmeler 0:3 0:7 · devam · adim · ustunden · cik · cerceve <i> · duraklat
 * Program durduğunda gönderilen olay:
 *   dur <neden> / ileti <hata> / cerceve <i> <dosya> <satır> <işlev> ... /
 *   deg <ad>\t<tip>\t<değer> ... / son */

typedef struct {
    const char *ad;
    int64_t dosya, satir;
    int64_t *yuvalar;
    const char *tanim;
} AyCerceve;

static AyCerceve *ay_yigin;
static int ay_kap, ay_etkin = -1; /* -1: henüz bakılmadı */
static Soket ay_soket = GECERSIZ_SOKET;
enum { AY_DEVAM, AY_ADIM, AY_USTUNDEN, AY_CIK };
static int ay_kip = AY_ADIM, ay_hedef;
static int64_t *ay_kesmeler; /* (dosya, satır) çiftleri */
static int ay_kesme_sayisi;
static char ay_tampon[65536];
static size_t ay_dolu;
static unsigned ay_sayac;

static void ay_gonder(Tampon *t) {
    size_t g = 0;
    while (g < t->n) {
        int n = (int)send(ay_soket, t->v + g, (int)(t->n - g), 0);
        if (n <= 0) exit(0); /* Stüdyo bağlantıyı kapattı: program durdurulur */
        g += (size_t)n;
    }
    t->n = 0;
}

/* Bir sonraki komut satırı (bekler). */
static char *ay_satir_oku(void) {
    static char satir[65536];
    for (;;) {
        char *s = memchr(ay_tampon, '\n', ay_dolu);
        if (s) {
            size_t n = (size_t)(s - ay_tampon);
            memcpy(satir, ay_tampon, n);
            satir[n] = 0;
            memmove(ay_tampon, s + 1, ay_dolu - n - 1);
            ay_dolu -= n + 1;
            return satir;
        }
        if (ay_dolu == sizeof ay_tampon) ay_dolu = 0;
        int n = (int)recv(ay_soket, ay_tampon + ay_dolu, (int)(sizeof ay_tampon - ay_dolu), 0);
        if (n <= 0) exit(0);
        ay_dolu += (size_t)n;
    }
}

static void ay_baglan(void) {
    const char *k = getenv("ORHUNCA_AYIKLA");
    ay_etkin = 0;
    if (!k || !*k) return;
#ifdef _WIN32
    WSADATA w;
    WSAStartup(MAKEWORD(2, 2), &w);
#else
    signal(SIGPIPE, SIG_IGN);
#endif
    Soket s = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in a;
    memset(&a, 0, sizeof a);
    a.sin_family = AF_INET;
    a.sin_port = htons((unsigned short)atoi(k));
    a.sin_addr.s_addr = htonl(0x7f000001);
    if (s == GECERSIZ_SOKET || connect(s, (struct sockaddr *)&a, sizeof a) != 0) {
        fprintf(stderr, "hata ayıklayıcıya bağlanılamadı (kapı %s)\n", k);
        exit(1);
    }
    int bir = 1;
    setsockopt(s, IPPROTO_TCP, 1 /* TCP_NODELAY */, (const char *)&bir, sizeof bir);
    ay_soket = s;
    ay_etkin = 1;
}

/* "0:3 0:7": (dosya, satır) çiftleri */
static void ay_kesmeleri_oku(char *p) {
    ay_kesme_sayisi = 0;
    while (*p) {
        while (*p == ' ') p++;
        if (!*p) break;
        char *son;
        int64_t d = strtoll(p, &son, 10);
        if (*son != ':') break;
        int64_t st = strtoll(son + 1, &son, 10);
        ay_kesmeler = ham_buyut(ay_kesmeler, sizeof(int64_t) * 2 * (size_t)(ay_kesme_sayisi + 1));
        ay_kesmeler[2 * ay_kesme_sayisi] = d;
        ay_kesmeler[2 * ay_kesme_sayisi + 1] = st;
        ay_kesme_sayisi++;
        p = son;
    }
}

/* Değerin okunabilir biçimi (tek satır, en çok ~4000 bayt). */
static void ay_deger(Tampon *t, int64_t d, int64_t kod) {
    int k = (int)(kod % 8);
    if (d == 0 && (k == KOD_METIN || k == 4 || k == 5 || k == KOD_MODEL)) {
        t_yaz(t, "—");
        return;
    }
    Tampon b = {0};
    bicimle(&b, d, kod, 1);
    size_t n = b.n > 4000 ? 4000 : b.n;
    for (size_t i = 0; i < n; i++) {
        char c = b.v[i];
        if (c == '\n') t_yaz(t, "\\n");
        else if (c == '\t') t_yaz(t, "\\t");
        else if (c == '\r') t_yaz(t, "\\r");
        else t_ekle(t, &c, 1);
    }
    if (n < b.n) t_yaz(t, "…");
    free(b.v);
}

/* Çerçevenin değişkenleri: tanım satırları "ad\tkod\ttip" (yuva sırasıyla). */
static void ay_degiskenler(Tampon *t, AyCerceve *c) {
    if (!c->tanim) return;
    const char *p = c->tanim;
    for (int i = 0; *p; i++) {
        const char *son = strchr(p, '\n');
        if (!son) son = p + strlen(p);
        const char *s1 = memchr(p, '\t', (size_t)(son - p));
        const char *s2 = s1 ? memchr(s1 + 1, '\t', (size_t)(son - s1 - 1)) : NULL;
        if (s2) {
            t_yaz(t, "deg ");
            t_ekle(t, p, (size_t)(s1 - p));
            t_yaz(t, "\t");
            t_ekle(t, s2 + 1, (size_t)(son - s2 - 1));
            t_yaz(t, "\t");
            ay_deger(t, c->yuvalar[i], atoll(s1 + 1));
            t_yaz(t, "\n");
        }
        p = *son ? son + 1 : son;
    }
}

/* Program durur: olayı gönderir, yürütme komutu gelene kadar bekler. */
static void ay_dur(const char *neden, const char *ileti) {
    Tampon t = {0};
    char k[256];
    fflush(stdout);
    fflush(stderr);
    snprintf(k, sizeof k, "dur %s\n", neden);
    t_yaz(&t, k);
    if (ileti) {
        t_yaz(&t, "ileti ");
        t_yaz(&t, ileti);
        t_yaz(&t, "\n");
    }
    for (int i = ay_derinlik - 1; i >= 0; i--) {
        AyCerceve *c = &ay_yigin[i];
        snprintf(k, sizeof k, "cerceve %d %" PRId64 " %" PRId64 " ", ay_derinlik - 1 - i, c->dosya,
                 c->satir);
        t_yaz(&t, k);
        t_yaz(&t, c->ad);
        t_yaz(&t, "\n");
    }
    if (ay_derinlik) ay_degiskenler(&t, &ay_yigin[ay_derinlik - 1]);
    t_yaz(&t, "son\n");
    ay_gonder(&t);
    for (;;) {
        char *s = ay_satir_oku();
        if (!strncmp(s, "kesmeler", 8)) {
            ay_kesmeleri_oku(s + 8);
        } else if (!strncmp(s, "cerceve ", 8)) {
            int i = atoi(s + 8);
            snprintf(k, sizeof k, "cdeg %d\n", i);
            t_yaz(&t, k);
            if (i >= 0 && i < ay_derinlik) ay_degiskenler(&t, &ay_yigin[ay_derinlik - 1 - i]);
            t_yaz(&t, "son\n");
            ay_gonder(&t);
        } else if (!strcmp(s, "devam")) {
            ay_kip = AY_DEVAM;
            break;
        } else if (!strcmp(s, "adim")) {
            ay_kip = AY_ADIM;
            break;
        } else if (!strcmp(s, "ustunden")) {
            ay_kip = AY_USTUNDEN;
            ay_hedef = ay_derinlik;
            break;
        } else if (!strcmp(s, "cik")) {
            ay_kip = AY_CIK;
            ay_hedef = ay_derinlik;
            break;
        }
    }
    free(t.v);
}

static void ay_hatada_dur(const char *mesaj) {
    if (ay_etkin == 1) ay_dur("hata", mesaj);
}

void ohc_ay_gir(int64_t ad) {
    if (ay_etkin < 0) ay_baglan();
    if (!ay_etkin) return;
    if (ay_derinlik == ay_kap) {
        ay_kap = ay_kap ? 2 * ay_kap : 64;
        ay_yigin = ham_buyut(ay_yigin, sizeof(AyCerceve) * (size_t)ay_kap);
    }
    AyCerceve *c = &ay_yigin[ay_derinlik++];
    c->ad = M(ad);
    c->dosya = c->satir = 0;
    c->yuvalar = NULL;
    c->tanim = NULL;
}

void ohc_ay_cik(void) {
    if (ay_etkin == 1 && ay_derinlik > 0) ay_derinlik--;
}

void ohc_ay_satir(int64_t satir, int64_t dosya, int64_t yuvalar, int64_t tanim) {
    if (ay_etkin != 1 || !ay_derinlik) return;
    AyCerceve *c = &ay_yigin[ay_derinlik - 1];
    c->satir = satir;
    c->dosya = dosya;
    c->yuvalar = (int64_t *)(intptr_t)yuvalar;
    c->tanim = M(tanim);
    const char *neden = NULL;
    if (ay_kip == AY_ADIM) neden = "adim";
    else if (ay_kip == AY_USTUNDEN && ay_derinlik <= ay_hedef) neden = "adim";
    else if (ay_kip == AY_CIK && ay_derinlik < ay_hedef) neden = "adim";
    if (!neden)
        for (int i = 0; i < ay_kesme_sayisi; i++)
            if (ay_kesmeler[2 * i] == dosya && ay_kesmeler[2 * i + 1] == satir) {
                neden = "kesme";
                break;
            }
    /* Çalışırken gelen komutlar (arada bir, beklemeden bakılır): kesme
     * noktalarının değişmesi ve "duraklat". */
    if (!neden && (++ay_sayac & 255) == 0) {
        size_t bos = sizeof ay_tampon - ay_dolu;
#ifdef _WIN32
        u_long bekleme = 1;
        ioctlsocket(ay_soket, FIONBIO, &bekleme);
        int n = bos ? recv(ay_soket, ay_tampon + ay_dolu, (int)bos, 0) : -1;
        bekleme = 0;
        ioctlsocket(ay_soket, FIONBIO, &bekleme);
#else
        int n = bos ? (int)recv(ay_soket, ay_tampon + ay_dolu, bos, MSG_DONTWAIT) : -1;
#endif
        if (n == 0) exit(0);
        if (n > 0) ay_dolu += (size_t)n;
        char *s;
        while ((s = memchr(ay_tampon, '\n', ay_dolu))) {
            *s = 0;
            if (!strncmp(ay_tampon, "kesmeler", 8)) ay_kesmeleri_oku(ay_tampon + 8);
            else if (!strcmp(ay_tampon, "duraklat")) neden = "duraklat";
            size_t k = (size_t)(s - ay_tampon) + 1;
            memmove(ay_tampon, s + 1, ay_dolu - k);
            ay_dolu -= k;
        }
    }
    if (neden) ay_dur(neden, NULL);
}
#endif /* __wasm__ */

#ifdef ORHUNCA_CALISTIRICI
/* ====================================================================== */
/* Hazır çalıştırıcı                                                       */
/* ====================================================================== */

/* Derleyici C derleyicisi ve bağlayıcı olmadan da çalıştırılabilir dosya üretir:
 * bu çalışma zamanı önceden derlenmiş bir "çalıştırıcı" olarak derleyiciye
 * gömülüdür; derleyici programın makine kodunu (kendi içinde bağlayıp) dosyanın
 * sonuna ekler. Program açılınca kod belleğe yüklenir, çalışma zamanı işlevlerinin
 * adresleri yazılır ve çalıştırılır (src/baglayici.rs).
 *
 * Paket: "OHCGRT01" | u64 kod boyu | u64 giriş | u64 içe aktarım sayısı |
 *        (u32 ad boyu, ad)* | u64 düzeltme sayısı | (u64 yer, u32 tür, u32 içe aktarım)* | kod
 * Dosyanın sonu: paket | "ORHUNCA!" | u64 paket boyu
 * Düzeltme türleri: 1 içe aktarılan işlevin adresi, 2 yere kodun başlangıç adresi eklenir. */

typedef struct {
    const char *ad;
    void *adres;
} IslevAdresi;

#include "calistirici_islevleri.h"

static void yukleme_hatasi(const char *m) {
    fprintf(stderr, "Orhunca çalıştırıcısı: %s\n", m);
    exit(70);
}

static uint64_t oku_u64(const unsigned char *p) {
    uint64_t v = 0;
    for (int i = 7; i >= 0; i--) v = (v << 8) | p[i];
    return v;
}

static uint32_t oku_u32(const unsigned char *p) { return (uint32_t)p[0] | (uint32_t)p[1] << 8 | (uint32_t)p[2] << 16 | (uint32_t)p[3] << 24; }

static int (*program_yukle(void))(void) {
#ifdef _WIN32
    static wchar_t yol[32768];
    if (!GetModuleFileNameW(NULL, yol, 32768)) yukleme_hatasi("çalıştırılabilir dosyanın yolu bulunamadı");
    FILE *f = _wfopen(yol, L"rb");
#else
    FILE *f = fopen("/proc/self/exe", "rb");
#endif
    if (!f) yukleme_hatasi("çalıştırılabilir dosya okunamadı");
    unsigned char son[16];
    if (fseek(f, -16, SEEK_END) != 0 || fread(son, 1, 16, f) != 16 || memcmp(son, "ORHUNCA!", 8) != 0)
        yukleme_hatasi("programın kodu bulunamadı (dosya bozuk ya da yalnızca çalıştırıcı)");
    uint64_t boy = oku_u64(son + 8);
    unsigned char *p = ham_ayir((size_t)boy);
    if (fseek(f, -(long)(16 + boy), SEEK_END) != 0 || fread(p, 1, (size_t)boy, f) != boy) yukleme_hatasi("kod okunamadı");
    fclose(f);
    const unsigned char *q = p, *bitis = p + boy;
#define GEREK(n)                                                                                                       \
    if ((size_t)(bitis - q) < (size_t)(n)) yukleme_hatasi("kod paketi bozuk")
    GEREK(32);
    if (memcmp(q, "OHCGRT01", 8) != 0) yukleme_hatasi("kod paketinin sürümü bu çalıştırıcıyla uyuşmuyor");
    uint64_t kod_boyu = oku_u64(q + 8), giris = oku_u64(q + 16), ice_sayisi = oku_u64(q + 24);
    q += 32;
    void **adresler = ham_ayir(sizeof(void *) * (size_t)(ice_sayisi + 1));
    size_t tablo_n = sizeof CALISTIRICI_ISLEVLERI / sizeof *CALISTIRICI_ISLEVLERI;
    for (uint64_t i = 0; i < ice_sayisi; i++) {
        GEREK(4);
        uint32_t n = oku_u32(q);
        q += 4;
        GEREK(n);
        adresler[i] = NULL;
        for (size_t k = 0; k < tablo_n; k++)
            if (strlen(CALISTIRICI_ISLEVLERI[k].ad) == n && !memcmp(CALISTIRICI_ISLEVLERI[k].ad, q, n))
                adresler[i] = CALISTIRICI_ISLEVLERI[k].adres;
        if (!adresler[i]) {
            char m[300];
            snprintf(m, sizeof m, "çalışma zamanında '%.*s' bulunamadı (derleyici ile çalıştırıcının sürümleri farklı)",
                     (int)(n > 200 ? 200 : n), q);
            yukleme_hatasi(m);
        }
        q += n;
    }
    GEREK(8);
    uint64_t duzeltme_sayisi = oku_u64(q);
    q += 8;
    const unsigned char *duzeltmeler = q;
    GEREK(duzeltme_sayisi * 16);
    q += duzeltme_sayisi * 16;
    GEREK(kod_boyu);
    if (giris >= kod_boyu) yukleme_hatasi("kod paketi bozuk");
    size_t alan = (size_t)kod_boyu ? (size_t)kod_boyu : 1;
#ifdef _WIN32
    unsigned char *taban = VirtualAlloc(NULL, alan, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!taban) yukleme_hatasi("kod için bellek ayrılamadı");
#else
    unsigned char *taban = mmap(NULL, alan, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (taban == MAP_FAILED) yukleme_hatasi("kod için bellek ayrılamadı");
#endif
    memcpy(taban, q, (size_t)kod_boyu);
    for (uint64_t i = 0; i < duzeltme_sayisi; i++) {
        const unsigned char *d = duzeltmeler + 16 * i;
        uint64_t yer = oku_u64(d);
        uint32_t tur = oku_u32(d + 8), ice = oku_u32(d + 12);
        if (yer + 8 > kod_boyu) yukleme_hatasi("kod paketi bozuk");
        uint64_t deger;
        memcpy(&deger, taban + yer, 8);
        if (tur == 1 && ice < ice_sayisi)
            deger += (uint64_t)(uintptr_t)adresler[ice];
        else if (tur == 2)
            deger += (uint64_t)(uintptr_t)taban;
        else
            yukleme_hatasi("kod paketi bozuk");
        memcpy(taban + yer, &deger, 8);
    }
#undef GEREK
#ifdef _WIN32
    DWORD eski;
    if (!VirtualProtect(taban, alan, PAGE_EXECUTE_READ, &eski)) yukleme_hatasi("kod çalıştırılabilir yapılamadı");
    FlushInstructionCache(GetCurrentProcess(), taban, alan);
#else
    if (mprotect(taban, alan, PROT_READ | PROT_EXEC) != 0) yukleme_hatasi("kod çalıştırılabilir yapılamadı");
#endif
    free(p);
    free(adresler);
    int (*ana)(void);
    void *giris_adresi = taban + giris;
    memcpy(&ana, &giris_adresi, sizeof ana);
    return ana;
}
#endif /* ORHUNCA_CALISTIRICI */
