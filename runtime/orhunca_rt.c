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
#include <sys/resource.h>
#include <signal.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <dirent.h>
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
#elif defined(__GNUC__) && (defined(__x86_64__) || defined(__i386__))
/* x86'da derleyicinin kendi setjmp'ı (Windows'ta SEH'e takılmadan geri sarar);
 * clang başka mimarilerde (ör. Apple işlemcileri) bunu desteklemez. */
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

/* CGI kipinde (paylaşımlı hosting) program bir hatayla bittiyse 500 sayfası gönderilir. */
static int cgi_hatali;

#define YIGIN_TASTI "çok derin özyineleme: işlevler birbirini bitmeyecek kadar çok çağırıyor (bitiş koşulunu denetleyin)"

static void veri_kilidi_birak(void);

static void hata(int64_t satir, const char *mesaj) {
    veri_kilidi_birak(); /* yakalanan bir hata kilidi açık bırakmasın */
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
    cgi_hatali = 1;
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
    hata(0, YIGIN_TASTI);
}
#else
/* Yığın sınırı: her işlevin girişinde yığının adresi bununla karşılaştırılır
 * (ohc_yigin_denetle). Hata mesajı yazılabilsin diye 256 KB pay bırakılır. */
static uintptr_t yigin_siniri;

static void yigin_sinirini_bul(void) {
    const uintptr_t pay = 256 * 1024;
#ifdef _WIN32
    MEMORY_BASIC_INFORMATION b;
    memset(&b, 0, sizeof b);
    if (VirtualQuery((void *)yigin_dibi, &b, sizeof b)) yigin_siniri = (uintptr_t)b.AllocationBase + 64 * 1024 + pay;
#else
    size_t boy = 8u << 20;
    struct rlimit r;
    if (getrlimit(RLIMIT_STACK, &r) == 0) boy = r.rlim_cur == RLIM_INFINITY ? (size_t)256 << 20 : (size_t)r.rlim_cur;
    if (boy < 2 * pay) boy = 2 * pay;
    yigin_siniri = (uintptr_t)yigin_dibi - boy + pay;
#endif
}

void ohc_yigin_denetle(int64_t adres) {
    if ((uintptr_t)adres < yigin_siniri) hata(0, YIGIN_TASTI);
}
#ifdef ORHUNCA_CALISTIRICI
static int (*program_yukle(void))(void);
#endif

/* Çalışma klasöründeki .env dosyası: AD=değer satırları ortam değişkeni olur. Zaten
 * tanımlı değişkenler değiştirilmez (sunucunun kendi ayarı önceliklidir). Desteklenen:
 * boş satır, # yorum, "export AD=…", tırnaklı değer ("…" ya da '…'), satır sonu yorumu. */
static void env_yukle(void) {
    FILE *f = fopen(".env", "rb");
    if (!f) return;
    char satir[8192];
    while (fgets(satir, sizeof satir, f)) {
        char *s = satir;
        size_t n = strlen(s);
        while (n && (s[n - 1] == '\n' || s[n - 1] == '\r')) s[--n] = 0;
        if (!strncmp(s, "\xEF\xBB\xBF", 3)) s += 3; /* UTF-8 BOM */
        while (*s == ' ' || *s == '\t') s++;
        if (!*s || *s == '#') continue;
        if (!strncmp(s, "export ", 7)) s += 7;
        char *esit = strchr(s, '=');
        if (!esit) continue;
        char *ad_son = esit;
        while (ad_son > s && (ad_son[-1] == ' ' || ad_son[-1] == '\t')) ad_son--;
        *ad_son = 0;
        char *d = esit + 1;
        while (*d == ' ' || *d == '\t') d++;
        size_t dn = strlen(d);
        if (dn >= 2 && (d[0] == '"' || d[0] == '\'') && d[dn - 1] == d[0]) {
            d[dn - 1] = 0;
            d++;
        } else {
            char *yorum = strstr(d, " #");
            if (yorum) *yorum = 0;
            dn = strlen(d);
            while (dn && (d[dn - 1] == ' ' || d[dn - 1] == '\t')) d[--dn] = 0;
        }
        if (!*s || getenv(s)) continue;
#ifdef _WIN32
        _putenv_s(s, d);
#else
        setenv(s, d, 0);
#endif
    }
    fclose(f);
}

#if !defined(_WIN32)
static void cgi_baslat(void);
#endif

int main(int argc, char **argv) {
    volatile uintptr_t dip = 0;
    yigin_dibi = (uintptr_t *)&dip + 1;
    yigin_sinirini_bul();
#ifdef _WIN32
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
#endif
    argumanlari_kaydet(argc, argv);
    env_yukle();
#if !defined(_WIN32)
    cgi_baslat();
#endif
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

int64_t ohc_us(int64_t a, int64_t b, int64_t satir) {
    double x = ondalik(a), y = ondalik(b), r = pow(x, y);
    if (isnan(r) && !isnan(x) && !isnan(y))
        hata(satir, "negatif bir sayının kesirli üssü alınamaz (ör. üs(-8.0, 0.5))");
    if (isinf(r) && isfinite(x) && isfinite(y)) {
        if (x == 0) hata(satir, "sıfırın negatif üssü alınamaz (sıfıra bölme)");
        hata(satir, "üs sonucu çok büyük (ondalık sayı sınırı aşıldı)");
    }
    return bitlere(r);
}

/* Bit işlemleri: 0 bit_ve, 1 bit_veya, 2 bit_xor, 3 sola_kaydır, 4 sağa_kaydır (mantıksal;
 * işaret biti de kayar). Kaydırma miktarı 0..63 dışındaysa sonuç 0'dır (her hedefte aynı). */
int64_t ohc_bit(int64_t islem, int64_t a, int64_t b) {
    uint64_t x = (uint64_t)a, y = (uint64_t)b;
    switch (islem) {
    case 0: return (int64_t)(x & y);
    case 1: return (int64_t)(x | y);
    case 2: return (int64_t)(x ^ y);
    case 3: return b < 0 || b > 63 ? 0 : (int64_t)(x << b);
    default: return b < 0 || b > 63 ? 0 : (int64_t)(x >> b);
    }
}

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

/* İşletim sisteminin şifrelemeye uygun rastgele kaynağı; başarısızsa 0. */
static int sistem_rastgele(unsigned char *b, size_t n) {
#ifdef __wasm__
    for (size_t i = 0; i < n; i++) {
        int32_t tamam = 0;
        b[i] = (unsigned char)js_guvenli_rastgele(&tamam);
        if (!tamam) return 0;
    }
    return 1;
#elif defined(_WIN32)
    typedef BOOLEAN(WINAPI * RtlGenRandomT)(PVOID, ULONG);
    HMODULE m = LoadLibraryA("advapi32.dll");
    RtlGenRandomT f = m ? (RtlGenRandomT)(void *)GetProcAddress(m, "SystemFunction036") : NULL;
    return f && f(b, (ULONG)n);
#else
    FILE *u = fopen("/dev/urandom", "rb");
    if (!u) return 0;
    int tamam = fread(b, 1, n, u) == n;
    fclose(u);
    return tamam;
#endif
}

/* `güvenli_anahtar(n)`: n rastgele bayt, onaltılık (oturum ve şifre sıfırlama anahtarları için).
 * rastgele()'nin aksine tahmin edilemez; güvenli kaynak yoksa hata verir. */
int64_t ohc_guvenli_anahtar(int64_t n, int64_t satir) {
    if (n < 1 || n > 512) hata(satir, "güvenli_anahtar: bayt sayısı 1 ile 512 arasında olmalı");
    unsigned char b[512];
    char s[1025];
    if (!sistem_rastgele(b, (size_t)n)) hata(satir, "işletim sisteminin güvenli rastgele kaynağı kullanılamadı");
    for (int64_t i = 0; i < n; i++) snprintf(s + 2 * i, 3, "%02x", b[i]);
    return metin_yap(s, (size_t)(2 * n));
}

/* JavaScript köprüsü: yalnızca web hedefinde (tarayıcıda) çalışır. */
int64_t ohc_js_calistir(int64_t kod, int64_t satir) {
#ifdef __wasm__
    int32_t n = 0, h = 0;
    char *c = js_js_calistir(M(kod), &n, &h);
    if (!c) return metin_yap("", 0);
    if (h) hata(satir, c);
    int64_t s = metin_yap(c, (size_t)n);
    free(c);
    return s;
#else
    (void)kod;
    (void)satir;
    return metin_yap("", 0);
#endif
}

void ohc_js_yukle(int64_t adres, int64_t satir) {
    (void)satir;
#ifdef __wasm__
    js_js_yukle(M(adres));
#else
    (void)adres;
#endif
}

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
    /* "nan", "inf" ve taşan değerler ("1e999") sayı sayılmaz: sessizce veriye karışırlar. */
    if (!isfinite(*d)) return 0;
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
/* HTTP istemcisi                                                          */
/* ====================================================================== */

/* http_al(adres) / http_gönder(adres, gövde): yanıtın gövdesi. İstek sistemdeki
 * curl ile yapılır (Windows 10+, macOS ve Linux dağıtımlarında hazır gelir; TLS
 * sertifikalarını işletim sistemi doğrular). Tarayıcıda (wasm) fetch yerine
 * eşzamanlı XHR kullanılır. Gövde curl'e standart girdiden verilir; adres
 * http:// ya da https:// ile başlamalıdır, böylece curl seçeneği sanılamaz. */

static const char *http_hatasi(int kod) {
    switch (kod) {
    case 6: return "sunucu bulunamadı (adresi ve internet bağlantısını denetleyin)";
    case 7: return "sunucuya bağlanılamadı";
    case 28: return "sunucu 30 saniye içinde yanıt vermedi";
    case 35: case 51: case 53: case 54: case 58: case 59: case 60: case 77: case 83:
        return "güvenli bağlantı (HTTPS) kurulamadı";
    case 47: return "çok fazla yönlendirme";
    case 52: return "sunucu boş yanıt döndürdü";
    case 55: case 56: return "bağlantı koptu";
    case 127: return "curl bulunamadı; HTTP istekleri için curl kurulu olmalı";
    default: return "istek başarısız oldu";
    }
}

#ifndef __wasm__
/* "Content-Type: x" gibi bir başlık satırı verilen adla mı başlıyor (büyük/küçük harf fark etmez) */
static int http_baslik_adi_mi(const char *b, const char *ad) {
    for (; *ad; b++, ad++) {
        char c = *b >= 'A' && *b <= 'Z' ? (char)(*b + 32) : *b;
        if (c != *ad) return 0;
    }
    return 1;
}
#endif

/* Bir HTTP isteği: `yontem` "GET", "POST"…; `govdeli` ise gövde gönderilir; `basliklar`
 * "Ad: değer" satırlarıdır (NULL olabilir). Yanıtın gövdesini döndürür. */
static int64_t http_istek(const char *yontem, const char *a, const char *g, int govdeli,
                          const char *basliklar, int64_t satir) {
    char mesaj[600];
    if (strncmp(a, "http://", 7) != 0 && strncmp(a, "https://", 8) != 0) {
        snprintf(mesaj, sizeof mesaj, "geçersiz adres '%.300s': http:// ya da https:// ile başlamalı", a);
        hata(satir, mesaj);
    }
    /* ASCII olmayan baytlar %XX olur; boşluk, tırnak ve denetim karakterleri reddedilir. */
    Tampon u = {0};
    for (const unsigned char *p = (const unsigned char *)a; *p; p++) {
        if (*p <= 32 || *p == '"' || *p == '\\' || *p == 127) {
            free(u.v);
            snprintf(mesaj, sizeof mesaj, "geçersiz adres '%.300s': boşluk ya da özel karakter içeremez (url_kodla kullanın)", a);
            hata(satir, mesaj);
        }
        if (*p >= 128) {
            char b[4];
            snprintf(b, sizeof b, "%%%02X", *p);
            t_ekle(&u, b, 3);
        } else {
            t_ekle(&u, (const char *)p, 1);
        }
    }
    if (!govdeli) g = "";
    size_t gn = strlen(g);
    Tampon t = {0};
    int kod = -1;
    if (!basliklar) basliklar = "";
#ifdef __wasm__
    int32_t durum = 0;
    int32_t n = 0;
    char *c = js_http(yontem, u.v, g, (int32_t)gn, basliklar, &n, &durum);
    free(u.v);
    if (!c) hata(satir, "HTTP isteği bu ortamda yapılamıyor");
    if (durum == 0) {
        snprintf(mesaj, sizeof mesaj, "HTTP isteği başarısız: %.400s", c);
        free(c);
        hata(satir, mesaj);
    }
    t_ekle(&t, c, (size_t)n);
    free(c);
    kod = 0;
    char durum_metni[16];
    snprintf(durum_metni, sizeof durum_metni, "\n%d", (int)durum);
    t_yaz(&t, durum_metni);
#else
    const char *tur = (gn > 0 && (g[0] == '{' || g[0] == '[')) ? "Content-Type: application/json"
                                                                : "Content-Type: application/x-www-form-urlencoded";
    /* Kullanıcının başlıkları: her satır bir "-H" (en çok 16; doğrulanmış, tırnak ve satır sonu yok) */
    char *baslik_kopya = strdup(basliklar);
    const char *arg[64];
    int k = 0;
    arg[k++] = "curl";
    arg[k++] = "-sS";
    arg[k++] = "-L";
    arg[k++] = "--max-redirs";
    arg[k++] = "5";
    arg[k++] = "--max-time";
    arg[k++] = "30";
    arg[k++] = "--proto";
    arg[k++] = "=http,https";
    arg[k++] = "--proto-redir";
    arg[k++] = "=http,https";
    arg[k++] = "-A";
    arg[k++] = "Orhunca";
    arg[k++] = "-w";
    arg[k++] = "\\n%{http_code}";
    if (strcmp(yontem, "GET") != 0 && strcmp(yontem, "POST") != 0) {
        arg[k++] = "-X";
        arg[k++] = yontem;
    }
    int tur_verildi = 0;
    for (char *b = baslik_kopya; b && *b && k < 56;) {
        char *son = strchr(b, '\n');
        if (son) *son = 0;
        if (*b) {
            if (http_baslik_adi_mi(b, "content-type:")) tur_verildi = 1;
            arg[k++] = "-H";
            arg[k++] = b;
        }
        b = son ? son + 1 : NULL;
    }
    if (govdeli) {
        if (!tur_verildi) {
            arg[k++] = "-H";
            arg[k++] = tur;
        }
        arg[k++] = "--data-binary";
        arg[k++] = "@-";
    }
    arg[k++] = u.v;
    arg[k] = NULL;
    fflush(stdout);
#ifdef _WIN32
    /* Komut satırı: adres doğrulandı (boşluk ve tırnak yok); "\n%{http_code}" tırnaklı. */
    Tampon s = {0};
    t_yaz(&s, "curl.exe");
    for (int i = 1; i < k; i++) {
        t_yaz(&s, " \"");
        t_yaz(&s, arg[i]);
        t_yaz(&s, "\"");
    }
    free(u.v);
    free(baslik_kopya);
    int wn = MultiByteToWideChar(CP_UTF8, 0, s.v, -1, NULL, 0);
    wchar_t *w = malloc(sizeof(wchar_t) * (size_t)wn);
    MultiByteToWideChar(CP_UTF8, 0, s.v, -1, w, wn);
    free(s.v);
    SECURITY_ATTRIBUTES sa = {sizeof sa, NULL, TRUE};
    HANDLE gir_o, gir_y, cik_o, cik_y;
    CreatePipe(&gir_o, &gir_y, &sa, 0);
    CreatePipe(&cik_o, &cik_y, &sa, 0);
    SetHandleInformation(gir_y, HANDLE_FLAG_INHERIT, 0);
    SetHandleInformation(cik_o, HANDLE_FLAG_INHERIT, 0);
    STARTUPINFOW si;
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = gir_o;
    si.hStdOutput = cik_y;
    si.hStdError = INVALID_HANDLE_VALUE;
    PROCESS_INFORMATION pi;
    BOOL basladi = CreateProcessW(NULL, w, NULL, NULL, TRUE, CREATE_NO_WINDOW, NULL, NULL, &si, &pi);
    free(w);
    CloseHandle(gir_o);
    CloseHandle(cik_y);
    if (!basladi) {
        CloseHandle(gir_y);
        CloseHandle(cik_o);
        hata(satir, http_hatasi(127));
    }
    DWORD yazilan;
    size_t gonderilen = 0;
    while (gonderilen < gn && WriteFile(gir_y, g + gonderilen, (DWORD)(gn - gonderilen), &yazilan, NULL) && yazilan > 0)
        gonderilen += yazilan;
    CloseHandle(gir_y);
    char b[8192];
    DWORD okunan;
    while (ReadFile(cik_o, b, sizeof b, &okunan, NULL) && okunan > 0) t_ekle(&t, b, okunan);
    CloseHandle(cik_o);
    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD cikis = 1;
    GetExitCodeProcess(pi.hProcess, &cikis);
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    kod = (int)cikis;
#else
    signal(SIGPIPE, SIG_IGN);
    int gir[2], cik[2];
    if (pipe(gir) != 0 || pipe(cik) != 0) {
        free(u.v);
        hata(satir, "HTTP isteği başlatılamadı");
    }
    pid_t p = fork();
    if (p == 0) {
        dup2(gir[0], 0);
        dup2(cik[1], 1);
        int bos = open("/dev/null", O_WRONLY);
        if (bos >= 0) dup2(bos, 2);
        close(gir[0]);
        close(gir[1]);
        close(cik[0]);
        close(cik[1]);
        execvp("curl", (char *const *)arg);
        _exit(127);
    }
    free(u.v);
    free(baslik_kopya);
    close(gir[0]);
    close(cik[1]);
    if (p < 0) {
        close(gir[1]);
        close(cik[0]);
        hata(satir, "HTTP isteği başlatılamadı");
    }
    size_t gonderilen = 0;
    while (gonderilen < gn) {
        ssize_t y = write(gir[1], g + gonderilen, gn - gonderilen);
        if (y < 0 && errno == EINTR) continue;
        if (y <= 0) break;
        gonderilen += (size_t)y;
    }
    close(gir[1]);
    char b[8192];
    for (;;) {
        ssize_t r = read(cik[0], b, sizeof b);
        if (r < 0 && errno == EINTR) continue;
        if (r <= 0) break;
        t_ekle(&t, b, (size_t)r);
    }
    close(cik[0]);
    int st = 0;
    while (waitpid(p, &st, 0) < 0 && errno == EINTR) {
    }
    kod = WIFEXITED(st) ? WEXITSTATUS(st) : -1;
#endif
#endif
    /* Sonda "\n<durum kodu>" var */
    int durum_kodu = 0;
    if (t.v) {
        char *son = strrchr(t.v, '\n');
        if (son) {
            durum_kodu = atoi(son + 1);
            t.n = (size_t)(son - t.v);
            t.v[t.n] = 0;
        }
    }
    if (kod != 0 || durum_kodu == 0) {
        free(t.v);
        snprintf(mesaj, sizeof mesaj, "HTTP isteği başarısız: %s", http_hatasi(kod));
        hata(satir, mesaj);
    }
    if (durum_kodu >= 400) {
        free(t.v);
        snprintf(mesaj, sizeof mesaj, "sunucu hata döndürdü: HTTP %d%s", durum_kodu,
                 durum_kodu == 404 ? " (sayfa bulunamadı)" : durum_kodu >= 500 ? " (sunucu hatası)" : "");
        hata(satir, mesaj);
    }
    return t_metin(&t);
}

int64_t ohc_http(int64_t yontem, int64_t adres, int64_t govde, int64_t satir) {
    return http_istek(yontem ? "POST" : "GET", M(adres), yontem ? M(govde) : "", (int)yontem, NULL, satir);
}

/* http_iste(yöntem, adres, gövde, başlıklar): yöntem GET, POST, PUT, PATCH ya da DELETE
 * (küçük harf de olur); başlıklar sözlük<metin, metin>. GET dışında gövde gönderilir. */
int64_t ohc_http_iste(int64_t yontem, int64_t adres, int64_t govde, int64_t basliklar, int64_t satir) {
    static const char *const YONTEMLER[] = {"GET", "POST", "PUT", "PATCH", "DELETE"};
    char y[16] = {0};
    const char *ym = M(yontem);
    for (size_t i = 0; ym[i] && i < sizeof y - 1; i++) y[i] = ym[i] >= 'a' && ym[i] <= 'z' ? (char)(ym[i] - 32) : ym[i];
    const char *secilen = NULL;
    for (size_t i = 0; i < sizeof YONTEMLER / sizeof *YONTEMLER; i++)
        if (strcmp(y, YONTEMLER[i]) == 0 && strlen(ym) == strlen(y)) secilen = YONTEMLER[i];
    char mesaj[400];
    if (!secilen) {
        snprintf(mesaj, sizeof mesaj, "geçersiz HTTP yöntemi '%.40s' (GET, POST, PUT, PATCH ya da DELETE olmalı)", ym);
        hata(satir, mesaj);
    }
    Tampon b = {0};
    Sozluk *s = (Sozluk *)(intptr_t)basliklar;
    if (s && s->uzunluk > 16) hata(satir, "en çok 16 başlık gönderilebilir");
    for (int64_t i = 0; s && i < s->uzunluk; i++) {
        const char *ad = M(s->anahtarlar[i]), *deger = M(s->degerler[i]);
        int gecerli = *ad != 0;
        for (const char *p = ad; *p; p++)
            if (!((*p >= 'a' && *p <= 'z') || (*p >= 'A' && *p <= 'Z') || (*p >= '0' && *p <= '9') || *p == '-' || *p == '_'))
                gecerli = 0;
        for (const char *p = deger; *p; p++)
            if (*p == '\r' || *p == '\n' || *p == '"' || *p == '\\') gecerli = 0;
        if (!gecerli) {
            free(b.v);
            snprintf(mesaj, sizeof mesaj,
                     "geçersiz HTTP başlığı '%.60s': ad harf, rakam ve - içerebilir; değerde tırnak, \\ ya da satır sonu olamaz", ad);
            hata(satir, mesaj);
        }
        t_yaz(&b, ad);
        t_yaz(&b, ": ");
        t_yaz(&b, deger);
        t_yaz(&b, "\n");
    }
    int64_t sonuc = http_istek(secilen, M(adres), M(govde), strcmp(secilen, "GET") != 0, b.v, satir);
    free(b.v);
    return sonuc;
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
    int tablo_hazir; /* SQLite tablosu oluşturuldu ve sütunları denetlendi */
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
    /* Hedefi o an başka bir program okuyorsa Windows taşımayı reddeder; kısa süre yeniden denenir. */
    for (int deneme = 0; deneme < 150; deneme++) {
        if (MoveFileExW(a, b, MOVEFILE_REPLACE_EXISTING)) return 1;
        DWORD e = GetLastError();
        if (e != ERROR_ACCESS_DENIED && e != ERROR_SHARING_VIOLATION && e != ERROR_LOCK_VIOLATION) return 0;
        Sleep(20);
    }
    return 0;
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

/*
 * Kaydetme ve silme "oku → değiştir → yaz" adımlarından oluşur. Aynı veri klasörünü
 * kullanan iki program bunu aynı anda yaparsa biri ötekinin kaydını silebilir. Bu yüzden
 * bu adımlar <Model>.json.kilit dosyası üzerinde süreçler arası bir kilitle yapılır.
 * Kilit alınamazsa (ör. salt okunur klasör) eskisi gibi kilitsiz devam edilir.
 */
#ifdef _WIN32
static HANDLE veri_kilidi = INVALID_HANDLE_VALUE;
#elif !defined(__wasm__)
static int veri_kilidi = -1;
#endif

static void veri_kilidi_birak(void) {
#ifdef _WIN32
    if (veri_kilidi != INVALID_HANDLE_VALUE) {
        OVERLAPPED o = {0};
        UnlockFileEx(veri_kilidi, 0, 1, 0, &o);
        CloseHandle(veri_kilidi);
        veri_kilidi = INVALID_HANDLE_VALUE;
    }
#elif !defined(__wasm__)
    if (veri_kilidi >= 0) {
        close(veri_kilidi); /* kilit de bırakılır */
        veri_kilidi = -1;
    }
#endif
}

static void veri_kilitle(ModelBilgisi *m) {
    veri_kilidi_birak();
    klasor_olustur(veri_klasoru());
    char *yol = veri_dosyasi(m);
    Tampon t = {0};
    t_yaz(&t, yol);
    t_yaz(&t, ".kilit");
    free(yol);
#ifdef _WIN32
    wchar_t w[1024];
    if (MultiByteToWideChar(CP_UTF8, 0, t.v, -1, w, 1024)) {
        HANDLE h = CreateFileW(w, GENERIC_READ | GENERIC_WRITE,
                               FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, NULL, OPEN_ALWAYS,
                               FILE_ATTRIBUTE_NORMAL, NULL);
        if (h != INVALID_HANDLE_VALUE) {
            OVERLAPPED o = {0};
            if (LockFileEx(h, LOCKFILE_EXCLUSIVE_LOCK, 0, 1, 0, &o))
                veri_kilidi = h;
            else
                CloseHandle(h);
        }
    }
#elif !defined(__wasm__)
    int fd = open(t.v, O_RDWR | O_CREAT | O_CLOEXEC, 0666);
    if (fd >= 0) {
        struct flock k = {0};
        k.l_type = F_WRLCK;
        k.l_whence = SEEK_SET;
        int r;
        while ((r = fcntl(fd, F_SETLKW, &k)) != 0 && errno == EINTR) {
        }
        if (r == 0)
            veri_kilidi = fd;
        else
            close(fd);
    }
#endif
    free(t.v);
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
    /* Geçici dosyanın adı programa özgüdür; iki program aynı geçici dosyaya yazmaz. */
    Tampon g = {0};
    t_yaz(&g, yol);
    char ek[48];
#ifdef _WIN32
    snprintf(ek, sizeof ek, ".%lu.yeni", (unsigned long)GetCurrentProcessId());
#elif defined(__wasm__)
    snprintf(ek, sizeof ek, ".yeni");
#else
    snprintf(ek, sizeof ek, ".%ld.yeni", (long)getpid());
#endif
    t_yaz(&g, ek);
    FILE *f = dosya_ac(g.v, "wb");
    int ok = f && fwrite(t.v, 1, t.n, f) == t.n;
    if (f && fclose(f) != 0) ok = 0;
    if (ok) ok = dosya_tasi(g.v, yol);
    if (!ok) {
        char mesaj[700];
        ohc_dosya_sil((int64_t)(intptr_t)g.v);
        snprintf(mesaj, sizeof mesaj,
                 "'%.400s' yazılamadı (klasöre yazma izni olmayabilir ya da dosya başka bir programda "
                 "açık olabilir)",
                 yol);
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

/* ---------------------------------------------------------------------- */
/* Veritabanları: ORHUNCA_VERITABANI ile modeller bir SQL veritabanına     */
/* yazılır (yoksa veri/<Model>.json):                                      */
/*   sqlite                                  <veri>/orhunca.sqlite         */
/*   postgresql://kullanıcı:şifre@sunucu:kapı/ad                           */
/*   mysql://kullanıcı:şifre@sunucu:kapı/ad          (MariaDB de)          */
/*   sqlserver://kullanıcı:şifre@sunucu\ÖRNEK/ad     (kullanıcısız: Windows */
/*                                                    kimlik doğrulaması)  */
/* Sürücü kütüphaneleri (libsqlite3/winsqlite3, libpq, libmysql/libmariadb, */
/* ODBC) çalışma anında yüklenir; programlar büyümez, derlemede bağımlılık  */
/* gerekmez. Her model bir tablodur: kimlik, sayı/mantık tam sayı, ondalık  */
/* kayan nokta, metin; liste, sözlük ve iç model alanları JSON metni.       */
/* Tablolar SQLite ve MySQL'de PHP çıktısınınkiyle aynıdır.                */
/* ---------------------------------------------------------------------- */

#ifndef __wasm__
#include <ctype.h>

enum { VT_JSON, VT_SQLITE, VT_POSTGRES, VT_MYSQL, VT_MSSQL };

/* Sorguya `?` yerine konan değer */
typedef struct {
    int tur; /* 0 NULL, 1 tam sayı, 2 ondalık, 3 metin */
    int64_t s;
    double o;
    const char *m;
} VtDeger;

/* Bir deyimin sonucu: bütün değerler metin olarak (NULL hücre: SQL NULL) */
typedef struct {
    int sutun;
    char **adlar;
    int64_t satir_sayisi, kap;
    char **hucreler; /* satir_sayisi × sutun */
    int64_t degisen;
    int64_t yeni_kimlik;
} VtSonuc;

static void vt_sonuc_birak(VtSonuc *s) {
    for (int i = 0; i < s->sutun; i++) free(s->adlar[i]);
    free(s->adlar);
    for (int64_t i = 0; i < s->satir_sayisi * s->sutun; i++) free(s->hucreler[i]);
    free(s->hucreler);
    memset(s, 0, sizeof *s);
}

static void vt_sonuc_sutunlar(VtSonuc *s, int n) {
    s->sutun = n;
    s->adlar = ham_ayir(sizeof(char *) * (size_t)(n ? n : 1));
    memset(s->adlar, 0, sizeof(char *) * (size_t)(n ? n : 1));
}

/* Yeni satırın hücreleri (hepsi NULL) */
static char **vt_sonuc_satir(VtSonuc *s) {
    if (s->satir_sayisi == s->kap) {
        s->kap = s->kap ? s->kap * 2 : 16;
        s->hucreler = ham_buyut(s->hucreler, sizeof(char *) * (size_t)(s->kap * (s->sutun ? s->sutun : 1)));
    }
    char **h = s->hucreler + s->satir_sayisi * s->sutun;
    memset(h, 0, sizeof(char *) * (size_t)s->sutun);
    s->satir_sayisi++;
    return h;
}

static int ascii_esit(const char *a, const char *b) {
    for (; *a && *b; a++, b++) {
        char x = *a >= 'A' && *a <= 'Z' ? *a + 32 : *a, y = *b >= 'A' && *b <= 'Z' ? *b + 32 : *b;
        if (x != y) return 0;
    }
    return !*a && !*b;
}

/* ---- Bağlantı adresi -------------------------------------------------- */

typedef struct {
    char *sunucu, *kullanici, *sifre, *ad, *secenekler;
    int kapi;
} VtAdres;

static VtAdres vt_adres;
static int vt_tur_durumu = -1;

/* Adresin bir parçası; %C3%BC → ü */
static char *adres_parcasi(const char *p, size_t n) {
    char *k = ham_ayir(n + 1);
    size_t j = 0;
    for (size_t i = 0; i < n; i++) {
        if (p[i] == '%' && i + 2 < n && isxdigit((unsigned char)p[i + 1]) && isxdigit((unsigned char)p[i + 2])) {
            char h[3] = {p[i + 1], p[i + 2], 0};
            k[j++] = (char)strtol(h, NULL, 16);
            i += 2;
        } else {
            k[j++] = p[i];
        }
    }
    k[j] = 0;
    return k;
}

static void adres_coz(const char *v) {
    const char *p = strstr(v, "://");
    p = p ? p + 3 : v;
    const char *yetki_son = p + strcspn(p, "/?");
    const char *at = NULL;
    for (const char *q = p; q < yetki_son; q++)
        if (*q == '@') at = q;
    const char *sunucu = p;
    if (at) {
        const char *iki = memchr(p, ':', (size_t)(at - p));
        if (iki) {
            vt_adres.kullanici = adres_parcasi(p, (size_t)(iki - p));
            vt_adres.sifre = adres_parcasi(iki + 1, (size_t)(at - iki - 1));
        } else {
            vt_adres.kullanici = adres_parcasi(p, (size_t)(at - p));
        }
        sunucu = at + 1;
    }
    const char *sun_son = yetki_son;
    for (const char *q = yetki_son; q > sunucu; q--) {
        if (q[-1] == ':') {
            vt_adres.kapi = atoi(q);
            sun_son = q - 1;
            break;
        }
        if (q[-1] < '0' || q[-1] > '9') break;
    }
    vt_adres.sunucu = adres_parcasi(sunucu, (size_t)(sun_son - sunucu));
    if (*yetki_son == '/') {
        const char *ad = yetki_son + 1;
        size_t n = strcspn(ad, "?");
        vt_adres.ad = adres_parcasi(ad, n);
        yetki_son = ad + n;
    }
    if (*yetki_son == '?') vt_adres.secenekler = adres_parcasi(yetki_son + 1, strlen(yetki_son + 1));
}

static int vt_turu(void) {
    if (vt_tur_durumu >= 0) return vt_tur_durumu;
    vt_tur_durumu = VT_JSON;
    const char *v = getenv("ORHUNCA_VERITABANI");
    if (!v || !*v || ascii_esit(v, "json")) return vt_tur_durumu;
    if (ascii_esit(v, "sqlite")) return vt_tur_durumu = VT_SQLITE;
    size_t n = strcspn(v, ":");
    char sema[16] = {0};
    if (n < sizeof sema) memcpy(sema, v, n);
    if (ascii_esit(sema, "postgresql") || ascii_esit(sema, "postgres"))
        vt_tur_durumu = VT_POSTGRES;
    else if (ascii_esit(sema, "mysql") || ascii_esit(sema, "mariadb"))
        vt_tur_durumu = VT_MYSQL;
    else if (ascii_esit(sema, "sqlserver") || ascii_esit(sema, "mssql"))
        vt_tur_durumu = VT_MSSQL;
    else {
        char m[600];
        snprintf(m, sizeof m,
                 "ORHUNCA_VERITABANI anlaşılamadı: '%.300s'\nipucu: sqlite, postgresql://kullanıcı:şifre@sunucu/ad, "
                 "mysql://... ya da sqlserver://... yazın",
                 v);
        hata(0, m);
    }
    /* `orhunca sına` gerçek veritabanına dokunmaz: sınamalar kendi geçici klasörlerindeki SQLite
     * dosyasıyla çalışır. */
    if (getenv("ORHUNCA_SINAMA")) return vt_tur_durumu = VT_SQLITE;
    adres_coz(v);
    return vt_tur_durumu;
}

/* Modeller SQL veritabanında mı? */
static int vt_sql_mi(void) { return vt_turu() != VT_JSON; }

/* Ham SQL'in veritabanı: JSON kipinde de SQLite dosyası kullanılır. */
static int vt_etkin(void) { return vt_turu() == VT_JSON ? VT_SQLITE : vt_turu(); }

/* ---- Kütüphane yükleme ------------------------------------------------ */

static void *dinamik_ac(const char *yol) {
#ifdef _WIN32
    /* Tam yolla yüklenen kütüphanenin bağımlılıkları da kendi klasöründen aranır. */
    if (strchr(yol, '\\')) return (void *)LoadLibraryExA(yol, NULL, LOAD_WITH_ALTERED_SEARCH_PATH);
    return (void *)LoadLibraryA(yol);
#else
    return dlopen(yol, RTLD_NOW | RTLD_LOCAL);
#endif
}

static void *dinamik_islev(void *k, const char *ad) {
#ifdef _WIN32
    return (void *)GetProcAddress((HMODULE)k, ad);
#else
    return dlsym(k, ad);
#endif
}

#ifdef _WIN32
/* "C:\Program Files\PostgreSQL\*" gibi bir desene uyan klasörlerden en yeni sürümün
 * altındaki kütüphane: ör. ...\PostgreSQL\17\bin\libpq.dll */
static void *windows_klasorde_ac(const char *desen, const char *alt) {
    WIN32_FIND_DATAA b;
    HANDLE h = FindFirstFileA(desen, &b);
    if (h == INVALID_HANDLE_VALUE) return NULL;
    char en_iyi[MAX_PATH] = {0};
    do {
        if ((b.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) && b.cFileName[0] != '.') {
            /* "9.6" < "16": önce uzunluk, sonra metin */
            size_t a = strlen(b.cFileName), e = strlen(en_iyi);
            if (!en_iyi[0] || a > e || (a == e && strcmp(b.cFileName, en_iyi) > 0))
                snprintf(en_iyi, sizeof en_iyi, "%s", b.cFileName);
        }
    } while (FindNextFileA(h, &b));
    FindClose(h);
    if (!en_iyi[0]) return NULL;
    char yol[MAX_PATH * 2];
    size_t kok = strrchr(desen, '\\') ? (size_t)(strrchr(desen, '\\') - desen) : 0;
    snprintf(yol, sizeof yol, "%.*s\\%s%s", (int)kok, desen, en_iyi, alt);
    return dinamik_ac(yol);
}
#endif

/* Önce ORHUNCA_VERITABANI_KUTUPHANESI, sonra bilinen adlar */
static void *vt_kutuphane(const char **adlar, size_t n) {
    const char *ozel = getenv("ORHUNCA_VERITABANI_KUTUPHANESI");
    if (ozel && *ozel) return dinamik_ac(ozel);
    void *k = NULL;
    for (size_t i = 0; i < n && !k; i++) {
#ifdef _WIN32
        const char *yildiz = strchr(adlar[i], '*');
        if (yildiz) {
            /* "desen*|alt yol" */
            const char *ayrac = strchr(adlar[i], '|');
            char desen[MAX_PATH];
            snprintf(desen, sizeof desen, "%.*s", (int)(ayrac - adlar[i]), adlar[i]);
            k = windows_klasorde_ac(desen, ayrac + 1);
            continue;
        }
#endif
        k = dinamik_ac(adlar[i]);
    }
    return k;
}

/* ---- SQL metnini tarama: `?` yerleri ve deyim sonu (;) ----------------- */

/* Tırnak, tanımlayıcı ve yorumların içindeki ? ve ; sayılmaz. Bulunan ? konumları
 * `yerler`e yazılır (en çok `en_cok`); dönüş deyimin uzunluğudur, *sonraki ;'den sonrası. */
static size_t sql_tara(const char *p, int tur, size_t *yerler, int en_cok, int *yer_sayisi, const char **sonraki) {
    size_t i = 0;
    *yer_sayisi = 0;
    while (p[i]) {
        char c = p[i];
        if (c == ';') {
            *sonraki = p + i + 1;
            return i;
        }
        if (c == '?') {
            if (*yer_sayisi < en_cok) yerler[*yer_sayisi] = i;
            (*yer_sayisi)++;
            i++;
        } else if (c == '\'' || c == '"' || c == '`' || (c == '[' && tur == VT_MSSQL)) {
            char kapa = c == '[' ? ']' : c;
            i++;
            while (p[i]) {
                if (tur == VT_MYSQL && p[i] == '\\' && kapa != '`' && p[i + 1]) {
                    i += 2;
                    continue;
                }
                if (p[i] == kapa) {
                    if (p[i + 1] == kapa) {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i++;
            }
            if (p[i]) i++;
        } else if (c == '-' && p[i + 1] == '-') {
            while (p[i] && p[i] != '\n') i++;
        } else if (c == '/' && p[i + 1] == '*') {
            i += 2;
            while (p[i] && !(p[i] == '*' && p[i + 1] == '/')) i++;
            if (p[i]) i += 2;
        } else if (c == '$' && tur == VT_POSTGRES) {
            /* $etiket$ ... $etiket$ */
            size_t j = i + 1;
            while (isalnum((unsigned char)p[j]) || p[j] == '_') j++;
            if (p[j] == '$') {
                size_t en = j - i + 1;
                const char *q = p + j + 1;
                while (*q && strncmp(q, p + i, en)) q++;
                i = *q ? (size_t)(q - p) + en : strlen(p);
            } else {
                i++;
            }
        } else {
            i++;
        }
    }
    *sonraki = p + i;
    return i;
}

/* ---- SQLite ----------------------------------------------------------- */

#define SQ_OK 0
#define SQ_SATIR 100
#define SQ_BITTI 101
#define SQ_KOPYALA ((void (*)(void *))(intptr_t)-1)

typedef struct {
    int (*open_v2)(const char *, void **, int, const char *);
    int (*prepare_v2)(void *, const char *, int, void **, const char **);
    int (*bind_int64)(void *, int, int64_t);
    int (*bind_double)(void *, int, double);
    int (*bind_text)(void *, int, const char *, int, void (*)(void *));
    int (*bind_null)(void *, int);
    int (*bind_parameter_count)(void *);
    int (*step)(void *);
    int (*column_count)(void *);
    const char *(*column_name)(void *, int);
    int (*column_type)(void *, int);
    int64_t (*column_int64)(void *, int);
    double (*column_double)(void *, int);
    const unsigned char *(*column_text)(void *, int);
    int (*column_bytes)(void *, int);
    int (*finalize)(void *);
    const char *(*errmsg)(void *);
    int (*busy_timeout)(void *, int);
    int64_t (*last_insert_rowid)(void *);
    int (*changes)(void *);
} SqliteIslevleri;

static SqliteIslevleri sq;
static void *sq_vt;

static void *dinamik_bagla(void *k, const char *on, const char *ad, int64_t satir, const char *kutuphane) {
    char tam[96];
    snprintf(tam, sizeof tam, "%s%s", on, ad);
    void *f = dinamik_islev(k, tam);
    if (!f) {
        char m[300];
        snprintf(m, sizeof m, "%s kütüphanesinde %s bulunamadı (sürümü çok eski olabilir)", kutuphane, tam);
        hata(satir, m);
    }
    return f;
}

static void sqlite_ac(int64_t satir) {
    if (sq_vt) return;
    static const char *adlar[] = {
#ifdef _WIN32
        "winsqlite3.dll", "sqlite3.dll",
#elif defined(__APPLE__)
        "libsqlite3.dylib", "/usr/lib/libsqlite3.dylib",
#elif defined(__ANDROID__)
        "libsqlite.so",
#else
        "libsqlite3.so.0", "libsqlite3.so",
#endif
    };
    void *k = vt_kutuphane(adlar, sizeof adlar / sizeof *adlar);
    if (!k)
        hata(satir, "SQLite kütüphanesi bulunamadı (Linux'ta libsqlite3 paketini kurun); "
                    "ORHUNCA_VERITABANI ayarını kaldırırsanız kayıtlar JSON dosyalarına yazılır");
#define SQ_BAGLA(ad) *(void **)&sq.ad = dinamik_bagla(k, "sqlite3_", #ad, satir, "SQLite")
    SQ_BAGLA(open_v2);
    SQ_BAGLA(prepare_v2);
    SQ_BAGLA(bind_int64);
    SQ_BAGLA(bind_double);
    SQ_BAGLA(bind_text);
    SQ_BAGLA(bind_null);
    SQ_BAGLA(bind_parameter_count);
    SQ_BAGLA(step);
    SQ_BAGLA(column_count);
    SQ_BAGLA(column_name);
    SQ_BAGLA(column_type);
    SQ_BAGLA(column_int64);
    SQ_BAGLA(column_double);
    SQ_BAGLA(column_text);
    SQ_BAGLA(column_bytes);
    SQ_BAGLA(finalize);
    SQ_BAGLA(errmsg);
    SQ_BAGLA(busy_timeout);
    SQ_BAGLA(last_insert_rowid);
    SQ_BAGLA(changes);
#undef SQ_BAGLA
    klasor_olustur(veri_klasoru());
    Tampon yol = {0};
    t_yaz(&yol, veri_klasoru());
    t_yaz(&yol, "/orhunca.sqlite");
    void *b = NULL;
    /* SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE */
    if (sq.open_v2(yol.v, &b, 2 | 4, NULL) != SQ_OK || !b) {
        char m[700];
        snprintf(m, sizeof m, "veritabanı açılamadı: %.500s", yol.v);
        free(yol.v);
        hata(satir, m);
    }
    free(yol.v);
    sq_vt = b;
    /* Aynı dosyayı başka bir program yazarken beklenir (ör. Stüdyo ile sunucu). */
    sq.busy_timeout(sq_vt, 5000);
}

static char *sqlite_hucre(void *st, int i) {
    char k[64];
    switch (sq.column_type(st, i)) {
    case 1: /* INTEGER */
        snprintf(k, sizeof k, "%" PRId64, sq.column_int64(st, i));
        return kopya_n(k, strlen(k));
    case 2: { /* FLOAT: en kısa tam gösterim */
        double x = sq.column_double(st, i);
        snprintf(k, sizeof k, "%.15g", x);
        if (strtod(k, NULL) != x) snprintf(k, sizeof k, "%.17g", x);
        return kopya_n(k, strlen(k));
    }
    case 5: return NULL;
    default: {
        const char *t = (const char *)sq.column_text(st, i);
        return kopya_n(t ? t : "", (size_t)sq.column_bytes(st, i));
    }
    }
}

/* SQLite'ta tek deyim: ? yerlerine değerler doğrudan bağlanır. */
static const char *sqlite_deyim(const char *sql, VtDeger *d, int n, int *bagli, VtSonuc *s, int64_t satir) {
    sqlite_ac(satir);
    void *st = NULL;
    const char *kalan = NULL;
    if (sq.prepare_v2(sq_vt, sql, -1, &st, &kalan) != SQ_OK) {
        char m[700];
        snprintf(m, sizeof m, "veritabanı hatası (SQL): %.600s", sq.errmsg(sq_vt));
        hata(satir, m);
    }
    if (!st) return NULL; /* yalnızca boşluk ya da yorum kaldı */
    int k = sq.bind_parameter_count(st);
    if (k > 0) {
        if ((bagli && *bagli) || k != n) {
            char m[200];
            snprintf(m, sizeof m, "SQL sorgusunda %d yer tutucu (?) var ama %d değer verildi", k, bagli && *bagli ? 0 : n);
            sq.finalize(st);
            hata(satir, m);
        }
        for (int i = 0; i < k; i++) {
            switch (d[i].tur) {
            case 1: sq.bind_int64(st, i + 1, d[i].s); break;
            case 2: sq.bind_double(st, i + 1, d[i].o); break;
            case 3: sq.bind_text(st, i + 1, d[i].m, -1, SQ_KOPYALA); break;
            default: sq.bind_null(st, i + 1);
            }
        }
        if (bagli) *bagli = 1;
    }
    int c = sq.column_count(st);
    vt_sonuc_sutunlar(s, c);
    for (int i = 0; i < c; i++) {
        const char *ad = sq.column_name(st, i);
        s->adlar[i] = kopya_n(ad ? ad : "", ad ? strlen(ad) : 0);
    }
    int r;
    while ((r = sq.step(st)) == SQ_SATIR) {
        char **h = vt_sonuc_satir(s);
        for (int i = 0; i < c; i++) h[i] = sqlite_hucre(st, i);
    }
    sq.finalize(st);
    if (r != SQ_BITTI) {
        char m[700];
        snprintf(m, sizeof m, "veritabanı hatası (SQL): %.600s", sq.errmsg(sq_vt));
        vt_sonuc_birak(s);
        hata(satir, m);
    }
    s->degisen = sq.changes(sq_vt);
    s->yeni_kimlik = sq.last_insert_rowid(sq_vt);
    return kalan;
}

/* ---- PostgreSQL (libpq) ----------------------------------------------- */

typedef struct {
    void *(*PQconnectdb)(const char *);
    int (*PQstatus)(const void *);
    char *(*PQerrorMessage)(const void *);
    void (*PQfinish)(void *);
    void *(*PQexecParams)(void *, const char *, int, const unsigned *, const char *const *, const int *, const int *, int);
    int (*PQresultStatus)(const void *);
    char *(*PQresultErrorMessage)(const void *);
    int (*PQntuples)(const void *);
    int (*PQnfields)(const void *);
    char *(*PQfname)(const void *, int);
    char *(*PQgetvalue)(const void *, int, int);
    int (*PQgetlength)(const void *, int, int);
    int (*PQgetisnull)(const void *, int, int);
    char *(*PQcmdTuples)(void *);
    void (*PQclear)(void *);
    void *(*PQsetNoticeProcessor)(void *, void (*)(void *, const char *), void *);
} PgIslevleri;

static PgIslevleri pg;
static void *pg_vt;

/* NOTICE iletileri (ör. "tablo zaten var") programın çıktısına karışmasın */
static void pg_bildirim(void *a, const char *m) {
    (void)a;
    (void)m;
}

static void conninfo_ekle(Tampon *t, const char *ad, const char *deger) {
    if (!deger || !*deger) return;
    t_yaz(t, ad);
    t_yaz(t, "='");
    for (const char *p = deger; *p; p++) {
        if (*p == '\'' || *p == '\\') t_yaz(t, "\\");
        t_ekle(t, p, 1);
    }
    t_yaz(t, "' ");
}

static void *pg_baglan(const char *ad, char **hata_metni) {
    Tampon t = {0};
    conninfo_ekle(&t, "host", vt_adres.sunucu && *vt_adres.sunucu ? vt_adres.sunucu : "localhost");
    if (vt_adres.kapi) {
        char k[16];
        snprintf(k, sizeof k, "%d", vt_adres.kapi);
        conninfo_ekle(&t, "port", k);
    }
    conninfo_ekle(&t, "user", vt_adres.kullanici);
    conninfo_ekle(&t, "password", vt_adres.sifre);
    conninfo_ekle(&t, "dbname", ad);
    conninfo_ekle(&t, "client_encoding", "UTF8");
    conninfo_ekle(&t, "connect_timeout", "10");
    /* ?sslmode=require gibi seçenekler olduğu gibi geçer */
    if (vt_adres.secenekler) {
        char *s = kopya_n(vt_adres.secenekler, strlen(vt_adres.secenekler));
        for (char *parca = strtok(s, "&"); parca; parca = strtok(NULL, "&")) {
            char *esit = strchr(parca, '=');
            if (!esit) continue;
            *esit = 0;
            conninfo_ekle(&t, parca, esit + 1);
        }
        free(s);
    }
    void *c = pg.PQconnectdb(t.v ? t.v : "");
    free(t.v);
    if (c && pg.PQstatus(c) == 0) {
        pg.PQsetNoticeProcessor(c, pg_bildirim, NULL);
        return c;
    }
    const char *e = c ? pg.PQerrorMessage(c) : "bellek yetmedi";
    *hata_metni = kopya_n(e, strlen(e));
    if (c) pg.PQfinish(c);
    return NULL;
}

static void pg_ac(int64_t satir) {
    if (pg_vt) return;
    static const char *adlar[] = {
#ifdef _WIN32
        "libpq.dll", "C:\\Program Files\\PostgreSQL\\*|\\bin\\libpq.dll",
#elif defined(__APPLE__)
        "libpq.5.dylib", "/opt/homebrew/opt/libpq/lib/libpq.5.dylib", "/usr/local/opt/libpq/lib/libpq.5.dylib",
        "/opt/homebrew/lib/libpq.5.dylib", "/Applications/Postgres.app/Contents/Versions/latest/lib/libpq.5.dylib",
#else
        "libpq.so.5", "libpq.so",
#endif
    };
    void *k = vt_kutuphane(adlar, sizeof adlar / sizeof *adlar);
    if (!k)
        hata(satir, "PostgreSQL istemci kütüphanesi (libpq) bulunamadı; PostgreSQL'i kurun (Linux'ta libpq5 "
                    "paketi) ya da yolunu ORHUNCA_VERITABANI_KUTUPHANESI ile verin");
#define PG_BAGLA(ad) *(void **)&pg.ad = dinamik_bagla(k, "", #ad, satir, "libpq")
    PG_BAGLA(PQconnectdb);
    PG_BAGLA(PQstatus);
    PG_BAGLA(PQerrorMessage);
    PG_BAGLA(PQfinish);
    PG_BAGLA(PQexecParams);
    PG_BAGLA(PQresultStatus);
    PG_BAGLA(PQresultErrorMessage);
    PG_BAGLA(PQntuples);
    PG_BAGLA(PQnfields);
    PG_BAGLA(PQfname);
    PG_BAGLA(PQgetvalue);
    PG_BAGLA(PQgetlength);
    PG_BAGLA(PQgetisnull);
    PG_BAGLA(PQcmdTuples);
    PG_BAGLA(PQclear);
    PG_BAGLA(PQsetNoticeProcessor);
#undef PG_BAGLA
    char *ilk_hata = NULL, *ikinci = NULL;
    const char *ad = vt_adres.ad && *vt_adres.ad ? vt_adres.ad : NULL;
    pg_vt = pg_baglan(ad, &ilk_hata);
    if (!pg_vt && ad) {
        /* Veritabanı yoksa oluşturulur: önce "postgres" veritabanına bağlanılır. */
        void *c = pg_baglan("postgres", &ikinci);
        if (c) {
            const char *p[1] = {ad};
            void *r = pg.PQexecParams(c, "SELECT 1 FROM pg_database WHERE datname = $1", 1, NULL, p, NULL, NULL, 0);
            int var = r && pg.PQntuples(r) > 0;
            if (r) pg.PQclear(r);
            if (!var) {
                Tampon q = {0};
                t_yaz(&q, "CREATE DATABASE \"");
                for (const char *x = ad; *x; x++) t_ekle(&q, *x == '"' ? "\"\"" : x, *x == '"' ? 2 : 1);
                t_yaz(&q, "\"");
                r = pg.PQexecParams(c, q.v, 0, NULL, NULL, NULL, NULL, 0);
                free(q.v);
                if (r) pg.PQclear(r);
            }
            pg.PQfinish(c);
            free(ilk_hata);
            ilk_hata = NULL;
            pg_vt = pg_baglan(ad, &ilk_hata);
        }
        free(ikinci);
    }
    if (!pg_vt) {
        char m[900];
        /* libpq aynı satırı (SSL'li ve SSL'siz deneme için) iki kez yazabilir: tekrarlar atılır. */
        if (ilk_hata) {
            Tampon t = {0};
            const char *onceki = NULL;
            size_t onceki_n = 0;
            for (const char *p = ilk_hata; *p;) {
                size_t n = strcspn(p, "\n");
                if (n && !(onceki && onceki_n == n && !strncmp(onceki, p, n))) {
                    if (t.n) t_yaz(&t, "\n");
                    t_ekle(&t, p, n);
                    onceki = p;
                    onceki_n = n;
                }
                p += n + (p[n] == '\n');
            }
            free(ilk_hata);
            ilk_hata = t.v;
        }
        size_t n = ilk_hata ? strlen(ilk_hata) : 0;
        while (n && (ilk_hata[n - 1] == '\n' || ilk_hata[n - 1] == ' ')) ilk_hata[--n] = 0;
        snprintf(m, sizeof m,
                 "PostgreSQL'e bağlanılamadı (%.100s): %.600s\nipucu: sunucu çalışıyor mu? Kullanıcı adı, şifre ve "
                 "ORHUNCA_VERITABANI adresini denetleyin",
                 vt_adres.sunucu && *vt_adres.sunucu ? vt_adres.sunucu : "localhost", ilk_hata ? ilk_hata : "");
        hata(satir, m);
    }
}

/* SQL'deki ? yerleri $1, $2 … olur; değerler metin olarak gider. */
static void pg_calistir(const char *sql, size_t n, const size_t *yerler, int k, VtDeger *d, VtSonuc *s, int64_t satir) {
    pg_ac(satir);
    Tampon q = {0};
    size_t onceki = 0;
    for (int i = 0; i < k; i++) {
        t_ekle(&q, sql + onceki, yerler[i] - onceki);
        char b[16];
        snprintf(b, sizeof b, "$%d", i + 1);
        t_yaz(&q, b);
        onceki = yerler[i] + 1;
    }
    t_ekle(&q, sql + onceki, n - onceki);
    const char **degerler = ham_ayir(sizeof(char *) * (size_t)(k ? k : 1));
    char **tampon = ham_ayir(sizeof(char *) * (size_t)(k ? k : 1));
    for (int i = 0; i < k; i++) {
        char b[64];
        tampon[i] = NULL;
        switch (d[i].tur) {
        case 1:
            snprintf(b, sizeof b, "%" PRId64, d[i].s);
            degerler[i] = tampon[i] = kopya_n(b, strlen(b));
            break;
        case 2:
            snprintf(b, sizeof b, "%.17g", d[i].o);
            degerler[i] = tampon[i] = kopya_n(b, strlen(b));
            break;
        case 3: degerler[i] = d[i].m; break;
        default: degerler[i] = NULL;
        }
    }
    void *r = pg.PQexecParams(pg_vt, q.v ? q.v : "", k, NULL, degerler, NULL, NULL, 0);
    free(q.v);
    for (int i = 0; i < k; i++) free(tampon[i]);
    free(tampon);
    free(degerler);
    int durum = r ? pg.PQresultStatus(r) : 7;
    if (durum != 0 && durum != 1 && durum != 2) {
        char m[800];
        const char *e = r ? pg.PQresultErrorMessage(r) : pg.PQerrorMessage(pg_vt);
        snprintf(m, sizeof m, "veritabanı hatası (SQL): %.600s", e);
        size_t mn = strlen(m);
        while (mn && (m[mn - 1] == '\n' || m[mn - 1] == ' ')) m[--mn] = 0;
        if (r) pg.PQclear(r);
        hata(satir, m);
    }
    if (durum == 2) {
        int c = pg.PQnfields(r), sayi = pg.PQntuples(r);
        vt_sonuc_sutunlar(s, c);
        for (int i = 0; i < c; i++) {
            const char *ad = pg.PQfname(r, i);
            s->adlar[i] = kopya_n(ad, strlen(ad));
        }
        for (int y = 0; y < sayi; y++) {
            char **h = vt_sonuc_satir(s);
            for (int i = 0; i < c; i++)
                if (!pg.PQgetisnull(r, y, i))
                    h[i] = kopya_n(pg.PQgetvalue(r, y, i), (size_t)pg.PQgetlength(r, y, i));
        }
    }
    s->degisen = strtoll(pg.PQcmdTuples(r), NULL, 10);
    pg.PQclear(r);
}

/* ---- MySQL / MariaDB -------------------------------------------------- */

typedef struct {
    void *(*mysql_init)(void *);
    int (*mysql_options)(void *, int, const void *);
    void *(*mysql_real_connect)(void *, const char *, const char *, const char *, const char *, unsigned, const char *,
                                unsigned long);
    const char *(*mysql_error)(void *);
    unsigned (*mysql_errno)(void *);
    void (*mysql_close)(void *);
    int (*mysql_set_character_set)(void *, const char *);
    int (*mysql_select_db)(void *, const char *);
    int (*mysql_real_query)(void *, const char *, unsigned long);
    void *(*mysql_store_result)(void *);
    unsigned (*mysql_field_count)(void *);
    unsigned (*mysql_num_fields)(void *);
    void *(*mysql_fetch_field_direct)(void *, unsigned);
    char **(*mysql_fetch_row)(void *);
    unsigned long *(*mysql_fetch_lengths)(void *);
    void (*mysql_free_result)(void *);
    uint64_t (*mysql_affected_rows)(void *);
    uint64_t (*mysql_insert_id)(void *);
    unsigned long (*mysql_real_escape_string)(void *, char *, const char *, unsigned long);
} MyIslevleri;

static MyIslevleri my;
static void *my_vt;

static void *my_baglan(const char *ad) {
    void *c = my.mysql_init(NULL);
    if (!c) return NULL;
    unsigned sure = 10;
    my.mysql_options(c, 0 /* MYSQL_OPT_CONNECT_TIMEOUT */, &sure);
    my.mysql_options(c, 7 /* MYSQL_SET_CHARSET_NAME */, "utf8mb4");
    const char *sunucu = vt_adres.sunucu && *vt_adres.sunucu ? vt_adres.sunucu : "localhost";
    /* Bağlanamazsa da tutamak döner: çağıran mysql_errno ile bakar. */
    my.mysql_real_connect(c, sunucu, vt_adres.kullanici, vt_adres.sifre ? vt_adres.sifre : "", ad,
                          (unsigned)vt_adres.kapi, NULL, 0);
    return c;
}

static void my_ac(int64_t satir) {
    if (my_vt) return;
    static const char *adlar[] = {
#ifdef _WIN32
        "libmariadb.dll", "libmysql.dll", "C:\\Program Files\\MySQL\\MySQL Server*|\\lib\\libmysql.dll",
        "C:\\Program Files\\MariaDB*|\\lib\\libmariadb.dll", "C:\\xampp\\mysql\\lib\\libmariadb.dll",
        "C:\\xampp\\mysql\\bin\\libmariadb.dll", "C:\\xampp\\mysql\\lib\\libmysql.dll",
#elif defined(__APPLE__)
        "libmysqlclient.dylib", "/opt/homebrew/opt/mysql-client/lib/libmysqlclient.dylib",
        "/opt/homebrew/lib/libmysqlclient.dylib", "/usr/local/mysql/lib/libmysqlclient.dylib",
        "/opt/homebrew/opt/mariadb-connector-c/lib/mariadb/libmariadb.3.dylib", "/opt/homebrew/lib/libmariadb.3.dylib",
#else
        "libmariadb.so.3", "libmysqlclient.so.24", "libmysqlclient.so.21", "libmysqlclient.so.20",
        "libmysqlclient.so.18", "libmariadb.so", "libmysqlclient.so",
#endif
    };
    void *k = vt_kutuphane(adlar, sizeof adlar / sizeof *adlar);
    if (!k)
        hata(satir, "MySQL/MariaDB istemci kütüphanesi bulunamadı (libmariadb ya da libmysql); MySQL'i ya da "
                    "MariaDB'yi kurun ya da yolunu ORHUNCA_VERITABANI_KUTUPHANESI ile verin");
#define MY_BAGLA(ad) *(void **)&my.ad = dinamik_bagla(k, "", #ad, satir, "MySQL")
    MY_BAGLA(mysql_init);
    MY_BAGLA(mysql_options);
    MY_BAGLA(mysql_real_connect);
    MY_BAGLA(mysql_error);
    MY_BAGLA(mysql_errno);
    MY_BAGLA(mysql_close);
    MY_BAGLA(mysql_set_character_set);
    MY_BAGLA(mysql_select_db);
    MY_BAGLA(mysql_real_query);
    MY_BAGLA(mysql_store_result);
    MY_BAGLA(mysql_field_count);
    MY_BAGLA(mysql_num_fields);
    MY_BAGLA(mysql_fetch_field_direct);
    MY_BAGLA(mysql_fetch_row);
    MY_BAGLA(mysql_fetch_lengths);
    MY_BAGLA(mysql_free_result);
    MY_BAGLA(mysql_affected_rows);
    MY_BAGLA(mysql_insert_id);
    MY_BAGLA(mysql_real_escape_string);
#undef MY_BAGLA
    const char *ad = vt_adres.ad && *vt_adres.ad ? vt_adres.ad : NULL;
    void *c = my_baglan(ad);
    if (c && ad && my.mysql_errno(c) == 1049 /* ER_BAD_DB_ERROR */) {
        /* Veritabanı yoksa oluşturulur. */
        my.mysql_close(c);
        c = my_baglan(NULL);
        if (c && !my.mysql_errno(c)) {
            Tampon q = {0};
            t_yaz(&q, "CREATE DATABASE IF NOT EXISTS `");
            for (const char *x = ad; *x; x++) t_ekle(&q, *x == '`' ? "``" : x, *x == '`' ? 2 : 1);
            t_yaz(&q, "` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci");
            my.mysql_real_query(c, q.v, (unsigned long)q.n);
            free(q.v);
            my.mysql_select_db(c, ad);
        }
    }
    if (!c || my.mysql_errno(c)) {
        char m[900];
        snprintf(m, sizeof m,
                 "MySQL'e bağlanılamadı (%.100s): %.600s\nipucu: sunucu çalışıyor mu? Kullanıcı adı, şifre ve "
                 "ORHUNCA_VERITABANI adresini denetleyin",
                 vt_adres.sunucu && *vt_adres.sunucu ? vt_adres.sunucu : "localhost",
                 c ? my.mysql_error(c) : "bellek yetmedi");
        if (c) my.mysql_close(c);
        hata(satir, m);
    }
    my.mysql_set_character_set(c, "utf8mb4");
    my_vt = c;
}

/* MySQL ve SQL Server'da değerler, kaçırılarak SQL metnine yazılır. */
static void deger_yaz(Tampon *q, VtDeger *d, int tur) {
    char b[64];
    switch (d->tur) {
    case 1:
        snprintf(b, sizeof b, "%" PRId64, d->s);
        t_yaz(q, b);
        break;
    case 2:
        snprintf(b, sizeof b, "%.17g", d->o);
        t_yaz(q, b);
        break;
    case 3:
        if (tur == VT_MYSQL) {
            size_t n = strlen(d->m);
            char *k = ham_ayir(2 * n + 1);
            unsigned long kn = my.mysql_real_escape_string(my_vt, k, d->m, (unsigned long)n);
            t_yaz(q, "'");
            t_ekle(q, k, kn);
            t_yaz(q, "'");
            free(k);
        } else {
            t_yaz(q, "N'");
            for (const char *p = d->m; *p; p++) t_ekle(q, *p == '\'' ? "''" : p, *p == '\'' ? 2 : 1);
            t_yaz(q, "'");
        }
        break;
    default: t_yaz(q, "NULL");
    }
}

static char *degerleri_yerlestir(const char *sql, size_t n, const size_t *yerler, int k, VtDeger *d, int tur) {
    Tampon q = {0};
    size_t onceki = 0;
    for (int i = 0; i < k; i++) {
        t_ekle(&q, sql + onceki, yerler[i] - onceki);
        deger_yaz(&q, &d[i], tur);
        onceki = yerler[i] + 1;
    }
    t_ekle(&q, sql + onceki, n - onceki);
    if (!q.v) t_yaz(&q, "");
    return q.v;
}

static void my_calistir(const char *sql, size_t n, const size_t *yerler, int k, VtDeger *d, VtSonuc *s, int64_t satir) {
    my_ac(satir);
    char *q = degerleri_yerlestir(sql, n, yerler, k, d, VT_MYSQL);
    int r = my.mysql_real_query(my_vt, q, (unsigned long)strlen(q));
    free(q);
    if (r) {
        char m[800];
        snprintf(m, sizeof m, "veritabanı hatası (SQL): %.600s", my.mysql_error(my_vt));
        hata(satir, m);
    }
    void *sonuc = my.mysql_store_result(my_vt);
    if (sonuc) {
        unsigned c = my.mysql_num_fields(sonuc);
        vt_sonuc_sutunlar(s, (int)c);
        for (unsigned i = 0; i < c; i++) {
            /* MYSQL_FIELD'in ilk üyesi her sürümde `char *name` */
            const char *ad = *(char **)my.mysql_fetch_field_direct(sonuc, i);
            s->adlar[i] = kopya_n(ad, strlen(ad));
        }
        char **satir_;
        while ((satir_ = my.mysql_fetch_row(sonuc))) {
            unsigned long *uz = my.mysql_fetch_lengths(sonuc);
            char **h = vt_sonuc_satir(s);
            for (unsigned i = 0; i < c; i++)
                if (satir_[i]) h[i] = kopya_n(satir_[i], uz[i]);
        }
        my.mysql_free_result(sonuc);
    } else if (my.mysql_field_count(my_vt)) {
        char m[800];
        snprintf(m, sizeof m, "veritabanı hatası (SQL): %.600s", my.mysql_error(my_vt));
        hata(satir, m);
    } else {
        s->degisen = (int64_t)my.mysql_affected_rows(my_vt);
        s->yeni_kimlik = (int64_t)my.mysql_insert_id(my_vt);
    }
}

/* ---- SQL Server (ODBC) ------------------------------------------------ */

typedef struct {
    short (*SQLAllocHandle)(short, void *, void **);
    short (*SQLSetEnvAttr)(void *, int, void *, int);
    short (*SQLDriverConnectW)(void *, void *, const uint16_t *, short, uint16_t *, short, short *, unsigned short);
    short (*SQLExecDirectW)(void *, const uint16_t *, int);
    short (*SQLNumResultCols)(void *, short *);
    short (*SQLDescribeColW)(void *, unsigned short, uint16_t *, short, short *, short *, uint64_t *, short *, short *);
    short (*SQLFetch)(void *);
    short (*SQLGetData)(void *, unsigned short, short, void *, int64_t, int64_t *);
    short (*SQLRowCount)(void *, int64_t *);
    short (*SQLMoreResults)(void *);
    short (*SQLFreeHandle)(short, void *);
    short (*SQLGetDiagRecW)(short, void *, short, uint16_t *, int *, uint16_t *, short, short *);
} OdbcIslevleri;

static OdbcIslevleri od;
static void *od_ortam, *ms_vt;

#define ODBC_TAMAM(r) ((r) == 0 || (r) == 1)

/* UTF-8 → UTF-16 (sonu 0) */
static uint16_t *u16_yap(const char *s) {
    size_t n = strlen(s);
    uint16_t *w = ham_ayir(sizeof(uint16_t) * (2 * n + 1));
    size_t j = 0;
    while (*s) {
        uint32_t c = u8_oku(&s);
        if (c >= 0x10000) {
            c -= 0x10000;
            w[j++] = (uint16_t)(0xD800 + (c >> 10));
            w[j++] = (uint16_t)(0xDC00 + (c & 0x3FF));
        } else {
            w[j++] = (uint16_t)c;
        }
    }
    w[j] = 0;
    return w;
}

/* UTF-16 → UTF-8 */
static void u16_yaz(Tampon *t, const uint16_t *w, size_t n) {
    for (size_t i = 0; i < n; i++) {
        uint32_t c = w[i];
        if (c >= 0xD800 && c < 0xDC00 && i + 1 < n && w[i + 1] >= 0xDC00 && w[i + 1] < 0xE000) {
            c = 0x10000 + ((c - 0xD800) << 10) + (w[i + 1] - 0xDC00);
            i++;
        }
        u8_yaz(t, c);
    }
}

/* Son ODBC hatası: "SQLSTATE: ileti" */
static char *odbc_hatasi(short tur, void *h, char durum[6]) {
    uint16_t d[6] = {0}, ileti[1024];
    int yerel = 0;
    short n = 0;
    Tampon t = {0};
    if (ODBC_TAMAM(od.SQLGetDiagRecW(tur, h, 1, d, &yerel, ileti, 1024, &n))) {
        for (int i = 0; i < 5; i++) durum[i] = (char)d[i];
        durum[5] = 0;
        u16_yaz(&t, ileti, (size_t)(n < 1023 ? n : 1023));
    } else {
        durum[0] = 0;
        t_yaz(&t, "bilinmeyen ODBC hatası");
    }
    return t.v;
}

static void ms_baglanti_ekle(Tampon *t, const char *ad, const char *deger) {
    t_yaz(t, ad);
    t_yaz(t, "={");
    for (const char *p = deger; *p; p++) t_ekle(t, *p == '}' ? "}}" : p, *p == '}' ? 2 : 1);
    t_yaz(t, "};");
}

static void ms_ac(int64_t satir);
static void ms_calistir_metin(const char *sql, VtSonuc *s, int64_t satir);

static void ms_ac(int64_t satir) {
    if (ms_vt) return;
    static const char *adlar[] = {
#ifdef _WIN32
        "odbc32.dll",
#elif defined(__APPLE__)
        "libodbc.2.dylib", "/opt/homebrew/lib/libodbc.2.dylib", "/usr/local/lib/libodbc.2.dylib",
#else
        "libodbc.so.2", "libodbc.so",
#endif
    };
    void *k = vt_kutuphane(adlar, sizeof adlar / sizeof *adlar);
    if (!k)
        hata(satir, "ODBC kütüphanesi bulunamadı (Linux'ta unixodbc, macOS'ta unixodbc paketi gerekir)");
#define OD_BAGLA(ad) *(void **)&od.ad = dinamik_bagla(k, "", #ad, satir, "ODBC")
    OD_BAGLA(SQLAllocHandle);
    OD_BAGLA(SQLSetEnvAttr);
    OD_BAGLA(SQLDriverConnectW);
    OD_BAGLA(SQLExecDirectW);
    OD_BAGLA(SQLNumResultCols);
    OD_BAGLA(SQLDescribeColW);
    OD_BAGLA(SQLFetch);
    OD_BAGLA(SQLGetData);
    OD_BAGLA(SQLRowCount);
    OD_BAGLA(SQLMoreResults);
    OD_BAGLA(SQLFreeHandle);
    OD_BAGLA(SQLGetDiagRecW);
#undef OD_BAGLA
    if (!ODBC_TAMAM(od.SQLAllocHandle(1 /* ENV */, NULL, &od_ortam)))
        hata(satir, "ODBC ortamı oluşturulamadı");
    od.SQLSetEnvAttr(od_ortam, 200 /* SQL_ATTR_ODBC_VERSION */, (void *)(intptr_t)3, 0);
    /* Kurulu olan ilk SQL Server sürücüsü kullanılır; ?sürücü=... ile seçilebilir. */
    const char *suruculer[] = {"ODBC Driver 18 for SQL Server", "ODBC Driver 17 for SQL Server",
                               "ODBC Driver 13 for SQL Server", "SQL Server Native Client 11.0", "SQL Server"};
    const char *istenen = NULL;
    Tampon ek = {0};
    if (vt_adres.secenekler) {
        char *s = kopya_n(vt_adres.secenekler, strlen(vt_adres.secenekler));
        for (char *parca = strtok(s, "&"); parca; parca = strtok(NULL, "&")) {
            char *esit = strchr(parca, '=');
            if (!esit) continue;
            *esit = 0;
            if (!strcmp(parca, "sürücü") || ascii_esit(parca, "surucu") || ascii_esit(parca, "driver"))
                istenen = kopya_n(esit + 1, strlen(esit + 1));
            else
                ms_baglanti_ekle(&ek, parca, esit + 1);
        }
        free(s);
    }
    char *son_hata = NULL;
    for (size_t i = 0; i < sizeof suruculer / sizeof *suruculer && !ms_vt; i++) {
        const char *surucu = istenen ? istenen : suruculer[i];
        Tampon t = {0};
        ms_baglanti_ekle(&t, "Driver", surucu);
        const char *sunucu = vt_adres.sunucu && *vt_adres.sunucu ? vt_adres.sunucu : "localhost";
        if (vt_adres.kapi) {
            char b[300];
            snprintf(b, sizeof b, "%s,%d", sunucu, vt_adres.kapi);
            ms_baglanti_ekle(&t, "Server", b);
        } else {
            ms_baglanti_ekle(&t, "Server", sunucu);
        }
        if (vt_adres.kullanici && *vt_adres.kullanici) {
            ms_baglanti_ekle(&t, "UID", vt_adres.kullanici);
            ms_baglanti_ekle(&t, "PWD", vt_adres.sifre ? vt_adres.sifre : "");
        } else {
            t_yaz(&t, "Trusted_Connection=yes;");
        }
        /* Bilgisayardaki SQL Server'ın kendi imzalı sertifikası kabul edilir. */
        t_yaz(&t, "TrustServerCertificate=yes;");
        if (ek.v) t_yaz(&t, ek.v);
        void *c = NULL;
        od.SQLAllocHandle(2 /* DBC */, od_ortam, &c);
        uint16_t *w = u16_yap(t.v);
        free(t.v);
        short r = od.SQLDriverConnectW(c, NULL, w, -3 /* SQL_NTS */, NULL, 0, NULL, 0 /* NOPROMPT */);
        free(w);
        if (ODBC_TAMAM(r)) {
            ms_vt = c;
            break;
        }
        char durum[6];
        free(son_hata);
        son_hata = odbc_hatasi(2, c, durum);
        od.SQLFreeHandle(2, c);
        /* IM002: bu sürücü kurulu değil, sıradakine geçilir */
        if (istenen || strcmp(durum, "IM002")) break;
    }
    free(ek.v);
    if (!ms_vt) {
        char m[1000];
        snprintf(m, sizeof m,
                 "SQL Server'a bağlanılamadı (%.100s): %.600s\nipucu: SQL Server çalışıyor mu? Örnek adını yazın "
                 "(ör. localhost\\SQLEXPRESS); sürücü yoksa \"ODBC Driver 18 for SQL Server\" kurun",
                 vt_adres.sunucu && *vt_adres.sunucu ? vt_adres.sunucu : "localhost", son_hata ? son_hata : "");
        hata(satir, m);
    }
    free(son_hata);
    if (vt_adres.ad && *vt_adres.ad) {
        /* Veritabanı yoksa oluşturulur. */
        Tampon q = {0}, ad = {0};
        for (const char *x = vt_adres.ad; *x; x++) t_ekle(&ad, *x == ']' ? "]]" : x, *x == ']' ? 2 : 1);
        t_yaz(&q, "IF DB_ID(N'");
        for (const char *x = vt_adres.ad; *x; x++) t_ekle(&q, *x == '\'' ? "''" : x, *x == '\'' ? 2 : 1);
        t_yaz(&q, "') IS NULL CREATE DATABASE [");
        t_yaz(&q, ad.v);
        t_yaz(&q, "]");
        VtSonuc s = {0};
        ms_calistir_metin(q.v, &s, satir);
        vt_sonuc_birak(&s);
        free(q.v);
        q = (Tampon){0};
        t_yaz(&q, "USE [");
        t_yaz(&q, ad.v);
        t_yaz(&q, "]");
        ms_calistir_metin(q.v, &s, satir);
        vt_sonuc_birak(&s);
        free(q.v);
        free(ad.v);
    }
}

static void ms_hata(void *st, int64_t satir) {
    char durum[6];
    char *e = odbc_hatasi(3, st, durum);
    char m[800];
    snprintf(m, sizeof m, "veritabanı hatası (SQL): %.600s", e ? e : "");
    free(e);
    od.SQLFreeHandle(3, st);
    hata(satir, m);
}

/* Bir toplu iş (batch): ilk sonuç kümesi alınır, değişen satır sayısı toplanır. */
static void ms_calistir_metin(const char *sql, VtSonuc *s, int64_t satir) {
    void *st = NULL;
    if (!ODBC_TAMAM(od.SQLAllocHandle(3 /* STMT */, ms_vt, &st))) hata(satir, "ODBC deyimi oluşturulamadı");
    uint16_t *w = u16_yap(sql);
    short r = od.SQLExecDirectW(st, w, -3);
    free(w);
    if (!ODBC_TAMAM(r) && r != 100 /* SQL_NO_DATA */) ms_hata(st, satir);
    int alindi = 0;
    for (;;) {
        short c = 0;
        od.SQLNumResultCols(st, &c);
        if (c > 0 && !alindi) {
            alindi = 1;
            vt_sonuc_sutunlar(s, c);
            for (int i = 0; i < c; i++) {
                uint16_t ad[256];
                short n = 0, tip, ondalik_, bos;
                uint64_t boy;
                od.SQLDescribeColW(st, (unsigned short)(i + 1), ad, 256, &n, &tip, &boy, &ondalik_, &bos);
                Tampon t = {0};
                u16_yaz(&t, ad, (size_t)(n < 255 ? n : 255));
                s->adlar[i] = t.v ? t.v : kopya_n("", 0);
            }
            while (ODBC_TAMAM(r = od.SQLFetch(st))) {
                char **h = vt_sonuc_satir(s);
                for (int i = 0; i < c; i++) {
                    uint16_t b[2048];
                    int64_t ind = 0;
                    Tampon t = {0};
                    int bos_ = 0;
                    for (;;) {
                        short g = od.SQLGetData(st, (unsigned short)(i + 1), -8 /* SQL_C_WCHAR */, b, sizeof b, &ind);
                        if (g == 100) break;
                        if (!ODBC_TAMAM(g)) ms_hata(st, satir);
                        if (ind == -1 /* SQL_NULL_DATA */) {
                            bos_ = 1;
                            break;
                        }
                        size_t gelen = (ind == -4 /* SQL_NO_TOTAL */ || ind >= (int64_t)sizeof b)
                                           ? sizeof b / 2 - 1
                                           : (size_t)ind / 2;
                        u16_yaz(&t, b, gelen);
                        if (g == 0) break; /* hepsi geldi */
                    }
                    h[i] = bos_ ? NULL : (t.v ? t.v : kopya_n("", 0));
                    if (bos_) free(t.v);
                }
            }
            if (r != 100) ms_hata(st, satir);
        } else if (c == 0) {
            int64_t n = -1;
            od.SQLRowCount(st, &n);
            if (n >= 0) s->degisen = n;
        }
        r = od.SQLMoreResults(st);
        if (r == 100) break;
        if (!ODBC_TAMAM(r)) ms_hata(st, satir);
    }
    od.SQLFreeHandle(3, st);
}

static void ms_calistir(const char *sql, size_t n, const size_t *yerler, int k, VtDeger *d, VtSonuc *s, int64_t satir) {
    ms_ac(satir);
    char *q = degerleri_yerlestir(sql, n, yerler, k, d, VT_MSSQL);
    ms_calistir_metin(q, s, satir);
    free(q);
}

/* ---- Ortak: tek deyim -------------------------------------------------- */

/* Metnin ilk deyimini çalıştırır, kalanını döndürür (bitince NULL ya da boş). `?` yer
 * tutucularının sayısı değerlerle aynı olmalı; `bagli` verilirse (ham SQL) değerler
 * yalnızca yer tutucusu olan ilk deyime bağlanır. */
static const char *vt_deyim(const char *sql, VtDeger *d, int n, int *bagli, VtSonuc *s, int64_t satir) {
    memset(s, 0, sizeof *s);
    int tur = vt_etkin();
    if (tur == VT_SQLITE) return sqlite_deyim(sql, d, n, bagli, s, satir);
    size_t yer_kucuk[64];
    size_t *yerler = yer_kucuk;
    int k = 0;
    const char *kalan = NULL;
    size_t uz = sql_tara(sql, tur, yer_kucuk, 64, &k, &kalan);
    if (k > 64) {
        yerler = ham_ayir(sizeof(size_t) * (size_t)k);
        sql_tara(sql, tur, yerler, k, &k, &kalan);
    }
    /* Yalnızca boşluk ve yorumdan oluşan deyim atlanır. */
    int bos = 1;
    for (size_t i = 0; i < uz && bos; i++) {
        if (sql[i] == '-' && sql[i + 1] == '-') {
            while (i < uz && sql[i] != '\n') i++;
        } else if (sql[i] == '/' && sql[i + 1] == '*') {
            i += 2;
            while (i + 1 < uz && !(sql[i] == '*' && sql[i + 1] == '/')) i++;
            i++;
        } else if (!isspace((unsigned char)sql[i])) {
            bos = 0;
        }
    }
    if (!bos) {
        if (k > 0) {
            if ((bagli && *bagli) || k != n) {
                char m[200];
                snprintf(m, sizeof m, "SQL sorgusunda %d yer tutucu (?) var ama %d değer verildi", k,
                         bagli && *bagli ? 0 : n);
                if (yerler != yer_kucuk) free(yerler);
                hata(satir, m);
            }
            if (bagli) *bagli = 1;
        }
        if (tur == VT_POSTGRES)
            pg_calistir(sql, uz, yerler, k, d, s, satir);
        else if (tur == VT_MYSQL)
            my_calistir(sql, uz, yerler, k, d, s, satir);
        else
            ms_calistir(sql, uz, yerler, k, d, s, satir);
    }
    if (yerler != yer_kucuk) free(yerler);
    return kalan && *kalan ? kalan : NULL;
}

/* Değersiz tek deyim, sonucu atılır */
static void vt_calistir(const char *sql, int64_t satir) {
    VtSonuc s;
    vt_deyim(sql, NULL, 0, NULL, &s, satir);
    vt_sonuc_birak(&s);
}

static void vt_ac(int64_t satir) {
    switch (vt_etkin()) {
    case VT_SQLITE: sqlite_ac(satir); break;
    case VT_POSTGRES: pg_ac(satir); break;
    case VT_MYSQL: my_ac(satir); break;
    default: ms_ac(satir);
    }
}

/* ---- Lehçe farkları ---------------------------------------------------- */

static void ad_tirnakla(Tampon *t, const char *ad) {
    int tur = vt_etkin();
    char ac = tur == VT_MYSQL ? '`' : tur == VT_MSSQL ? '[' : '"';
    char kapa = tur == VT_MSSQL ? ']' : ac;
    t_ekle(t, &ac, 1);
    for (const char *p = ad; *p; p++) {
        t_ekle(t, p, 1);
        if (*p == kapa) t_ekle(t, p, 1);
    }
    t_ekle(t, &kapa, 1);
}

static const char *sutun_tipi(AlanBilgisi *a) {
    int tur = vt_etkin();
    switch (a->kod % 8) {
    case KOD_SAYI:
        return tur == VT_SQLITE ? "INTEGER NOT NULL DEFAULT 0" : "BIGINT NOT NULL DEFAULT 0";
    case KOD_MANTIK:
        return tur == VT_SQLITE     ? "INTEGER NOT NULL DEFAULT 0"
               : tur == VT_POSTGRES ? "BOOLEAN NOT NULL DEFAULT FALSE"
               : tur == VT_MYSQL    ? "TINYINT(1) NOT NULL DEFAULT 0"
                                    : "BIT NOT NULL DEFAULT 0";
    case KOD_ONDALIK:
        return tur == VT_SQLITE     ? "REAL NOT NULL DEFAULT 0"
               : tur == VT_POSTGRES ? "DOUBLE PRECISION NOT NULL DEFAULT 0"
               : tur == VT_MYSQL    ? "DOUBLE NOT NULL DEFAULT 0"
                                    : "FLOAT NOT NULL DEFAULT 0";
    default:
        if (tur == VT_MYSQL) return a->secenekler ? "VARCHAR(255)" : "LONGTEXT";
        if (tur == VT_MSSQL) return a->secenekler ? "NVARCHAR(255)" : "NVARCHAR(MAX)";
        return "TEXT";
    }
}

static const char *kimlik_sutunu(void) {
    switch (vt_etkin()) {
    case VT_POSTGRES: return "\"kimlik\" BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY";
    case VT_MYSQL: return "`kimlik` BIGINT AUTO_INCREMENT PRIMARY KEY";
    case VT_MSSQL: return "[kimlik] BIGINT IDENTITY(1,1) PRIMARY KEY";
    default: return "\"kimlik\" INTEGER PRIMARY KEY AUTOINCREMENT";
    }
}

static void islem_baslat(int64_t satir) {
    int tur = vt_etkin();
    vt_calistir(tur == VT_MYSQL ? "START TRANSACTION" : tur == VT_MSSQL ? "BEGIN TRANSACTION" : "BEGIN", satir);
}

/* ---- Modeller ---------------------------------------------------------- */

/* Alanın değeri sorgu değerine (liste, sözlük ve model JSON metni olur; metni çağıran bırakır) */
static VtDeger alan_degeri(int64_t d, AlanBilgisi *a, char **ayrilan) {
    VtDeger v = {0};
    *ayrilan = NULL;
    switch (a->kod % 8) {
    case KOD_SAYI: v.tur = 1, v.s = d; break;
    case KOD_MANTIK: v.tur = 1, v.s = d ? 1 : 0; break;
    case KOD_ONDALIK: v.tur = 2, v.o = ondalik(d); break;
    case KOD_METIN: v.tur = 3, v.m = d ? M(d) : ""; break;
    default:
        if (a->kod % 8 == KOD_MODEL && !d) break;
        {
            Tampon t = {0};
            json_yaz(&t, d, a->kod, 0);
            if (!t.v) t_yaz(&t, "");
            *ayrilan = t.v;
            v.tur = 3, v.m = t.v;
        }
    }
    return v;
}

/* Kaydı (kimliğiyle ya da kimliksiz) tabloya ekler; kimliği döndürür. */
static int64_t vt_ekle(ModelBilgisi *m, int64_t n, int kimlikle, int64_t satir) {
    int tur = vt_etkin();
    Tampon q = {0}, d = {0};
    int64_t bas = kimlikle ? 0 : 1;
    int adet = (int)(m->alan_sayisi - bas);
    if (tur == VT_MSSQL && kimlikle) {
        t_yaz(&q, "SET IDENTITY_INSERT ");
        ad_tirnakla(&q, m->ad);
        t_yaz(&q, " ON; ");
    }
    t_yaz(&q, "INSERT INTO ");
    ad_tirnakla(&q, m->ad);
    if (adet == 0) {
        t_yaz(&q, tur == VT_MYSQL ? " () VALUES ()" : tur == VT_MSSQL ? " OUTPUT INSERTED.[kimlik] DEFAULT VALUES" : " DEFAULT VALUES");
    } else {
        t_yaz(&q, " (");
        for (int64_t i = bas; i < m->alan_sayisi; i++) {
            if (i > bas) {
                t_yaz(&q, ", ");
                t_yaz(&d, ", ");
            }
            ad_tirnakla(&q, m->alanlar[i].ad);
            t_yaz(&d, "?");
        }
        t_yaz(&q, ")");
        if (tur == VT_MSSQL && !kimlikle) t_yaz(&q, " OUTPUT INSERTED.[kimlik]");
        t_yaz(&q, " VALUES (");
        t_yaz(&q, d.v);
        t_yaz(&q, ")");
    }
    if (tur == VT_POSTGRES && !kimlikle) t_yaz(&q, " RETURNING \"kimlik\"");
    if (tur == VT_MSSQL && kimlikle) {
        t_yaz(&q, "; SET IDENTITY_INSERT ");
        ad_tirnakla(&q, m->ad);
        t_yaz(&q, " OFF");
    }
    free(d.v);
    VtDeger *degerler = ham_ayir(sizeof(VtDeger) * (size_t)(adet ? adet : 1));
    char **ayrilan = ham_ayir(sizeof(char *) * (size_t)(adet ? adet : 1));
    for (int i = 0; i < adet; i++) {
        int64_t alan = bas + i;
        degerler[i] = alan == 0 ? (VtDeger){1, ALAN(n, 0), 0, NULL}
                                : alan_degeri(ALAN(n, alan), &m->alanlar[alan], &ayrilan[i]);
        if (alan == 0) ayrilan[i] = NULL;
    }
    VtSonuc s;
    if (tur == VT_MSSQL) {
        /* SQL Server toplu işi tek seferde gider (? yerleri metne yazılır) */
        ms_ac(satir);
        size_t yer_kucuk[64];
        size_t *yerler = adet > 64 ? ham_ayir(sizeof(size_t) * (size_t)adet) : yer_kucuk;
        int k = 0;
        const char *kalan;
        /* Toplu işin tamamı taranır: ;'ler de dahil */
        size_t toplam = 0, parca;
        int toplam_k = 0;
        const char *p = q.v;
        while (*p) {
            parca = sql_tara(p, VT_MSSQL, yerler + toplam_k, adet - toplam_k, &k, &kalan);
            for (int i = 0; i < k; i++) yerler[toplam_k + i] += toplam;
            toplam_k += k;
            toplam += (size_t)(kalan - p);
            (void)parca;
            p = kalan;
        }
        char *son = degerleri_yerlestir(q.v, strlen(q.v), yerler, toplam_k, degerler, VT_MSSQL);
        memset(&s, 0, sizeof s);
        ms_calistir_metin(son, &s, satir);
        free(son);
        if (yerler != yer_kucuk) free(yerler);
    } else {
        vt_deyim(q.v, degerler, adet, NULL, &s, satir);
    }
    free(q.v);
    for (int i = 0; i < adet; i++) free(ayrilan[i]);
    free(ayrilan);
    free(degerler);
    int64_t kimlik = ALAN(n, 0);
    if (!kimlikle) {
        if ((tur == VT_POSTGRES || tur == VT_MSSQL) && s.satir_sayisi > 0 && s.hucreler[0])
            kimlik = strtoll(s.hucreler[0], NULL, 10);
        else
            kimlik = s.yeni_kimlik;
    }
    vt_sonuc_birak(&s);
    if (tur == VT_POSTGRES && kimlikle) {
        /* Elle verilen kimlikten sonra sayaç ileri alınır. */
        Tampon t = {0}, a = {0};
        ad_tirnakla(&a, m->ad);
        t_yaz(&t, "SELECT setval(pg_get_serial_sequence(?, 'kimlik'), (SELECT MAX(\"kimlik\") FROM ");
        t_yaz(&t, a.v);
        t_yaz(&t, "))");
        VtDeger v = {3, 0, 0, a.v};
        vt_deyim(t.v, &v, 1, NULL, &s, satir);
        vt_sonuc_birak(&s);
        free(t.v);
        free(a.v);
    }
    return kimlik;
}

/* Modelin tablosu: yoksa oluşturulur (veri/<Model>.json varsa içe aktarılır), eksik sütunlar eklenir. */
static void tablo_hazirla(ModelBilgisi *m, int64_t satir) {
    if (m->tablo_hazir) return;
    vt_ac(satir);
    int tur = vt_etkin();
    const char *var_sorgusu =
        tur == VT_SQLITE     ? "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?"
        : tur == VT_POSTGRES ? "SELECT table_name FROM information_schema.tables WHERE table_schema = current_schema() "
                               "AND table_name = ?"
        : tur == VT_MYSQL    ? "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() AND "
                               "table_name = ?"
                             : "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = SCHEMA_NAME() AND "
                               "TABLE_NAME = ?";
    VtDeger ad = {3, 0, 0, m->ad};
    VtSonuc s;
    vt_deyim(var_sorgusu, &ad, 1, NULL, &s, satir);
    int yeni = s.satir_sayisi == 0;
    vt_sonuc_birak(&s);
    Tampon q = {0};
    if (yeni) {
        t_yaz(&q, "CREATE TABLE ");
        ad_tirnakla(&q, m->ad);
        t_yaz(&q, " (");
        t_yaz(&q, kimlik_sutunu());
        for (int64_t i = 1; i < m->alan_sayisi; i++) {
            t_yaz(&q, ", ");
            ad_tirnakla(&q, m->alanlar[i].ad);
            t_yaz(&q, " ");
            t_yaz(&q, sutun_tipi(&m->alanlar[i]));
        }
        t_yaz(&q, tur == VT_MYSQL ? ") DEFAULT CHARSET=utf8mb4" : ")");
        vt_calistir(q.v, satir);
        free(q.v);
    } else {
        /* Modele sonradan eklenen alanlar için sütun eklenir; kaldırılan alanların sütunları kalır. */
        const char *sutun_sorgusu =
            tur == VT_SQLITE     ? "SELECT name FROM pragma_table_info(?)"
            : tur == VT_POSTGRES ? "SELECT column_name FROM information_schema.columns WHERE table_schema = "
                                   "current_schema() AND table_name = ?"
            : tur == VT_MYSQL    ? "SELECT column_name FROM information_schema.columns WHERE table_schema = DATABASE() "
                                   "AND table_name = ?"
                                 : "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = SCHEMA_NAME() "
                                   "AND TABLE_NAME = ?";
        vt_deyim(sutun_sorgusu, &ad, 1, NULL, &s, satir);
        char *var = ham_ayir((size_t)m->alan_sayisi);
        memset(var, 0, (size_t)m->alan_sayisi);
        for (int64_t r = 0; r < s.satir_sayisi; r++) {
            const char *a = s.hucreler[r * s.sutun];
            int64_t i = a ? alan_sirasi(m, a) : -1;
            if (i >= 0) var[i] = 1;
        }
        vt_sonuc_birak(&s);
        for (int64_t i = 1; i < m->alan_sayisi; i++) {
            if (var[i]) continue;
            Tampon a = {0};
            t_yaz(&a, "ALTER TABLE ");
            ad_tirnakla(&a, m->ad);
            t_yaz(&a, tur == VT_MSSQL ? " ADD " : " ADD COLUMN ");
            ad_tirnakla(&a, m->alanlar[i].ad);
            t_yaz(&a, " ");
            t_yaz(&a, sutun_tipi(&m->alanlar[i]));
            vt_calistir(a.v, satir);
            free(a.v);
        }
        free(var);
    }
    m->tablo_hazir = 1;
    if (yeni) {
        /* JSON'dan geçiş: önceki kayıtlar kimlikleriyle aktarılır (JSON dosyası silinmez). */
        int64_t liste = kayitlari_oku(m, satir);
        Liste *l = ORNEK(liste);
        if (l->uzunluk) {
            islem_baslat(satir);
            for (int64_t i = 0; i < l->uzunluk; i++) vt_ekle(m, l->ogeler[i], 1, satir);
            vt_calistir("COMMIT", satir);
        }
    }
}

/* Programın bütün modellerinin tabloları (ham SQL sorgularından önce) */
static void tablolari_hazirla(int64_t satir) {
    for (int64_t i = 0; i < kayitli_tanim_sayisi; i++) {
        ModelBilgisi *m = model_bilgisi(kayitli_tanimlar[i]);
        /* Web sunucusunun yerleşik modelleri kaydedilmez. */
        if (!strcmp(m->ad, "İstek") || !strcmp(m->ad, "Yanıt") || !strcmp(m->ad, "YüklenenDosya")) continue;
        tablo_hazirla(m, satir);
    }
}

/* Sonucun `r`. satırından model nesnesi: satır JSON nesnesine çevrilip okunur. */
static int64_t satirdan_nesne(VtSonuc *s, int64_t r, ModelBilgisi *m, int64_t satir) {
    Tampon t = {0};
    char k[64];
    t_yaz(&t, "{");
    int ilk = 1;
    for (int c = 0; c < s->sutun; c++) {
        const char *h = s->hucreler[r * s->sutun + c];
        int64_t i = alan_sirasi(m, s->adlar[c]);
        if (i < 0 || !h) continue;
        int64_t kod = m->alanlar[i].kod;
        if (!ilk) t_yaz(&t, ",");
        ilk = 0;
        json_metin(&t, m->alanlar[i].ad);
        t_yaz(&t, ":");
        switch (kod % 8) {
        case KOD_SAYI:
            snprintf(k, sizeof k, "%" PRId64, (int64_t)strtoll(h, NULL, 10));
            t_yaz(&t, k);
            break;
        case KOD_MANTIK:
            t_yaz(&t, (!strcmp(h, "1") || !strcmp(h, "t") || ascii_esit(h, "true")) ? "true" : "false");
            break;
        case KOD_ONDALIK: {
            double x = strtod(h, NULL);
            if (!isfinite(x)) x = 0;
            snprintf(k, sizeof k, "%.17g", x);
            t_yaz(&t, k);
            break;
        }
        case KOD_METIN: json_metin(&t, h); break;
        default: t_yaz(&t, *h ? h : "null");
        }
    }
    t_yaz(&t, "}");
    Json j = {t.v, NULL, 0};
    int64_t nesne = j_deger(&j, KOD_MODEL, m);
    if (j.hata) {
        char mesaj[600];
        snprintf(mesaj, sizeof mesaj, "'%.200s' tablosundaki kayıt okunamadı: %s", m->ad, j.hata);
        free(t.v);
        hata(satir, mesaj);
    }
    free(t.v);
    return nesne;
}

/* SELECT * FROM <Model> … */
static void vt_secim(ModelBilgisi *m, const char *bas, const char *son, VtDeger *d, int n, VtSonuc *s, int64_t satir) {
    tablo_hazirla(m, satir);
    Tampon q = {0};
    t_yaz(&q, bas);
    ad_tirnakla(&q, m->ad);
    t_yaz(&q, son);
    vt_deyim(q.v, d, n, NULL, s, satir);
    free(q.v);
}

static int64_t vt_hepsi(ModelBilgisi *m, int64_t satir) {
    VtSonuc s;
    int tur = vt_etkin();
    const char *sira = tur == VT_MSSQL ? " ORDER BY [kimlik]" : tur == VT_MYSQL ? " ORDER BY `kimlik`" : " ORDER BY \"kimlik\"";
    vt_secim(m, "SELECT * FROM ", sira, NULL, 0, &s, satir);
    int64_t liste = ohc_liste_yeni();
    for (int64_t r = 0; r < s.satir_sayisi; r++) ohc_liste_ekle(liste, satirdan_nesne(&s, r, m, satir));
    vt_sonuc_birak(&s);
    return liste;
}

static const char *kimlik_kosulu(void) {
    switch (vt_etkin()) {
    case VT_MYSQL: return " WHERE `kimlik` = ?";
    case VT_MSSQL: return " WHERE [kimlik] = ?";
    default: return " WHERE \"kimlik\" = ?";
    }
}

static int64_t vt_bul(ModelBilgisi *m, int64_t kimlik, int64_t varsayilan, int64_t satir) {
    VtSonuc s;
    VtDeger d = {1, kimlik, 0, NULL};
    vt_secim(m, "SELECT * FROM ", kimlik_kosulu(), &d, 1, &s, satir);
    int64_t n = s.satir_sayisi > 0 ? satirdan_nesne(&s, 0, m, satir) : varsayilan;
    vt_sonuc_birak(&s);
    return n;
}

static int vt_var(ModelBilgisi *m, int64_t kimlik, int64_t satir) {
    VtSonuc s;
    VtDeger d = {1, kimlik, 0, NULL};
    vt_secim(m, "SELECT 1 FROM ", kimlik_kosulu(), &d, 1, &s, satir);
    int var = s.satir_sayisi > 0;
    vt_sonuc_birak(&s);
    return var;
}

static int vt_sil(ModelBilgisi *m, int64_t kimlik, int64_t satir) {
    VtSonuc s;
    VtDeger d = {1, kimlik, 0, NULL};
    vt_secim(m, "DELETE FROM ", kimlik_kosulu(), &d, 1, &s, satir);
    int silindi = s.degisen > 0;
    vt_sonuc_birak(&s);
    return silindi;
}

static int64_t vt_kaydet(ModelBilgisi *m, int64_t n, int64_t satir) {
    tablo_hazirla(m, satir);
    int64_t kimlik = ALAN(n, 0);
    if (kimlik > 0 && vt_var(m, kimlik, satir)) {
        if (m->alan_sayisi < 2) return kimlik;
        Tampon q = {0};
        t_yaz(&q, "UPDATE ");
        ad_tirnakla(&q, m->ad);
        t_yaz(&q, " SET ");
        for (int64_t i = 1; i < m->alan_sayisi; i++) {
            if (i > 1) t_yaz(&q, ", ");
            ad_tirnakla(&q, m->alanlar[i].ad);
            t_yaz(&q, " = ?");
        }
        t_yaz(&q, kimlik_kosulu());
        int adet = (int)m->alan_sayisi;
        VtDeger *d = ham_ayir(sizeof(VtDeger) * (size_t)adet);
        char **ayrilan = ham_ayir(sizeof(char *) * (size_t)adet);
        for (int64_t i = 1; i < m->alan_sayisi; i++) d[i - 1] = alan_degeri(ALAN(n, i), &m->alanlar[i], &ayrilan[i - 1]);
        d[adet - 1] = (VtDeger){1, kimlik, 0, NULL};
        ayrilan[adet - 1] = NULL;
        VtSonuc s;
        vt_deyim(q.v, d, adet, NULL, &s, satir);
        vt_sonuc_birak(&s);
        free(q.v);
        for (int i = 0; i < adet; i++) free(ayrilan[i]);
        free(ayrilan);
        free(d);
        return kimlik;
    }
    kimlik = vt_ekle(m, n, kimlik > 0, satir);
    ALAN(n, 0) = kimlik;
    return kimlik;
}

/* Ham SQL: `?` yerlerine liste<metin> değerleri bağlanır. Birden çok deyim (;) sırayla çalışır.
 * `sonuc` verilirse satırlar sözlük<metin, metin> olarak eklenir. */
static int64_t sql_yurut(int64_t sorgu, int64_t degerler, int64_t sonuc, int64_t satir) {
    vt_ac(satir);
    if (vt_sql_mi()) tablolari_hazirla(satir);
    Liste *dl = degerler ? ORNEK(degerler) : NULL;
    int n = dl ? (int)dl->uzunluk : 0;
    VtDeger *d = ham_ayir(sizeof(VtDeger) * (size_t)(n ? n : 1));
    for (int i = 0; i < n; i++) d[i] = (VtDeger){3, 0, 0, M(dl->ogeler[i])};
    int64_t degisen = 0;
    int bagli = 0;
    const char *p = M(sorgu);
    while (p && *p) {
        VtSonuc s;
        p = vt_deyim(p, d, n, &bagli, &s, satir);
        for (int64_t r = 0; sonuc && r < s.satir_sayisi; r++) {
            int64_t sz = ohc_sozluk_yeni();
            for (int c = 0; c < s.sutun; c++) {
                const char *h = s.hucreler[r * s.sutun + c];
                ohc_sozluk_koy(sz, metin_yap(s.adlar[c], strlen(s.adlar[c])), h ? metin_yap(h, strlen(h)) : D(""),
                               KOD_METIN);
            }
            ohc_liste_ekle(sonuc, sz);
        }
        degisen = s.degisen;
        vt_sonuc_birak(&s);
    }
    free(d);
    return degisen;
}

int64_t ohc_sql_sorgu(int64_t sorgu, int64_t degerler, int64_t satir) {
    int64_t sonuc = ohc_liste_yeni();
    sql_yurut(sorgu, degerler, sonuc, satir);
    return sonuc;
}

int64_t ohc_sql_calistir(int64_t sorgu, int64_t degerler, int64_t satir) { return sql_yurut(sorgu, degerler, 0, satir); }
#else
static int vt_sql_mi(void) { return 0; }
static int64_t vt_hepsi(ModelBilgisi *m, int64_t satir) { (void)m; return satir; }
static int64_t vt_bul(ModelBilgisi *m, int64_t k, int64_t v, int64_t satir) { (void)m; (void)k; (void)satir; return v; }
static int vt_var(ModelBilgisi *m, int64_t k, int64_t satir) { (void)m; (void)k; (void)satir; return 0; }
static int vt_sil(ModelBilgisi *m, int64_t k, int64_t satir) { (void)m; (void)k; (void)satir; return 0; }
static int64_t vt_kaydet(ModelBilgisi *m, int64_t n, int64_t satir) { (void)m; (void)n; return satir; }
int64_t ohc_sql_sorgu(int64_t sorgu, int64_t degerler, int64_t satir) {
    (void)sorgu;
    (void)degerler;
    hata(satir, "SQL sorguları tarayıcıda (web hedefinde) çalışmaz");
    return 0;
}
int64_t ohc_sql_calistir(int64_t sorgu, int64_t degerler, int64_t satir) { return ohc_sql_sorgu(sorgu, degerler, satir); }
#endif

int64_t ohc_model_hepsi(int64_t tanim, int64_t satir) {
    if (vt_sql_mi()) return vt_hepsi(model_bilgisi(M(tanim)), satir);
    return kayitlari_oku(model_bilgisi(M(tanim)), satir);
}

/* Kimliği verilen kaydı döndürür; yoksa `varsayilan` nesnesini (kimlik 0). */
int64_t ohc_model_yukle(int64_t varsayilan, int64_t kimlik, int64_t satir) {
    if (vt_sql_mi()) return vt_bul(nesne_bilgisi(varsayilan), kimlik, varsayilan, satir);
    int64_t liste = kayitlari_oku(nesne_bilgisi(varsayilan), satir);
    int64_t i = kayit_sirasi(liste, kimlik);
    return i < 0 ? varsayilan : ORNEK(liste)->ogeler[i];
}

int64_t ohc_model_var(int64_t tanim, int64_t kimlik, int64_t satir) {
    if (vt_sql_mi()) return vt_var(model_bilgisi(M(tanim)), kimlik, satir);
    return kayit_sirasi(kayitlari_oku(model_bilgisi(M(tanim)), satir), kimlik) >= 0;
}

int64_t ohc_model_sil(int64_t tanim, int64_t kimlik, int64_t satir) {
    ModelBilgisi *m = model_bilgisi(M(tanim));
    if (vt_sql_mi()) return vt_sil(m, kimlik, satir);
    veri_kilitle(m);
    int64_t liste = kayitlari_oku(m, satir);
    int64_t i = kayit_sirasi(liste, kimlik);
    if (i < 0) {
        veri_kilidi_birak();
        return 0;
    }
    ohc_liste_sil(liste, i, satir);
    kayitlari_yaz(m, liste, satir);
    veri_kilidi_birak();
    return 1;
}

/* Yeni nesneye (kimlik 0) sıradaki kimliği verir; var olanı günceller. */
int64_t ohc_model_kaydet(int64_t n, int64_t satir) {
    if (!n) hata(satir, "boş bir model değeri kaydedilemez");
    ModelBilgisi *m = nesne_bilgisi(n);
    /* NaN ve sonsuz JSON'da yazılamaz; sessizce null/0 olmasın diye kayıt reddedilir. */
    for (int64_t i = 1; i < m->alan_sayisi; i++)
        if (m->alanlar[i].kod % 8 == KOD_ONDALIK && !isfinite(ondalik(ALAN(n, i)))) {
            char mesaj[256];
            snprintf(mesaj, sizeof mesaj,
                     "'%s' alanı geçerli bir sayı değil (NaN ya da sonsuz); kayıt yapılmadı",
                     m->alanlar[i].ad);
            hata(satir, mesaj);
        }
    if (vt_sql_mi()) return vt_kaydet(m, n, satir);
    veri_kilitle(m);
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
    veri_kilidi_birak();
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
            if (!isfinite(x)) {
                gorunen_ad(&t, a);
                t_yaz(&t, " geçerli bir sayı olmalı");
            } else if (a->en_az_var && x < a->en_az)
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


/* Yolun bir parçası gönderilemez mi: "." ile başlayan (.., .env, .git) ya da
 * Windows aygıt adı (con, nul, com1 ...; uzantılı da: con.txt). */
static int yasak_parca(const char *p, size_t n) {
    if (n == 0 || p[0] == '.') return 1;
    static const char *aygitlar[] = {"con", "prn", "aux", "nul", "com", "lpt", "conin$", "conout$"};
    size_t ad = 0;
    while (ad < n && p[ad] != '.') ad++;
    for (size_t k = 0; k < sizeof aygitlar / sizeof *aygitlar; k++) {
        size_t an = strlen(aygitlar[k]);
        if (ad < an || harf_duyarsiz_esit(p, aygitlar[k], an)) continue;
        if (ad == an && k != 4 && k != 5) return 1;
        if ((k == 4 || k == 5) && ad == an + 1 && p[an] >= '0' && p[an] <= '9') return 1;
    }
    return 0;
}

/* statik/ klasöründen dosya gönderir; dosya yoksa 0 döndürür. */
static int statik_gonder(Yanit *y, const char *yol) {
    if (strchr(yol, '\\') || strchr(yol, ':')) return 0;
    for (const char *p = yol + 1; *p;) {
        const char *e = strchr(p, '/');
        size_t n = e ? (size_t)(e - p) : strlen(p);
        if (yasak_parca(p, n)) return 0;
        p += n + (e ? 1 : 0);
    }
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
    if (!sistem_rastgele(b, n))
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
#define EN_COK_OTURUM 100000
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
        char *boy_sonu;
        long long boy = strtoll(v + i, &boy_sonu, 16);
        if (boy_sonu == v + i) return -1;
        if (boy < 0 || boy > en_buyuk_govde - (int64_t)cozulen->n) return -2;
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

/* CGI kipi: program bir web sunucusu (Apache, LiteSpeed) tarafından her istekte ayrı
 * çalıştırılır (paylaşımlı hosting). Bkz. cgi_isle. */
static int cgi_kipi;

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
                    /* Yol oturumu yeni bir sözlükle değiştirdiyse (ör. girişte `istek.oturum = {}`)
                     * yeni bir kimlik verilir: önceden ele geçirilmiş kimlik girişten sonra işe yaramaz. */
                    if (o && son_oturum != oturum) {
                        oturum_sil(o);
                        o = NULL;
                    }
                    if (!o) {
                        /* Bellek sınırı: en eski oturum yer açar. */
                        if (oturum_sayisi >= EN_COK_OTURUM) {
                            int64_t eski = 0;
                            for (int64_t k = 1; k < oturum_sayisi; k++)
                                if (oturumlar[k].son < oturumlar[eski].son) eski = k;
                            oturum_sil(&oturumlar[eski]);
                        }
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
    if (cgi_kipi) return;
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
    double istek_basi; /* bekleyen isteğin ilk baytı geldiğinde */
} Baglanti;

#define BOSTA_KALMA_SURESI 30.0
#define YARIM_ISTEK_SURESI 15.0
/* Başlıklar bu sürede tamamlanmalı (her bayt zamanlayıcıyı sıfırlasa da) */
#define BASLIK_SURESI 30.0
#define EN_COK_BAGLANTI 1000

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

#ifndef __wasm__
/* ---------------------------------------------------------------------- */
/* C kütüphanelerini çağırma (FFI)                                         */
/* ---------------------------------------------------------------------- */
/* `kütüphane "m":` bloğundaki işlevler. Derleyici her C işlevi için bir gövde üretir:
 * ohc_dis_bul ile işlevin adresi bulunur (ilk çağrıda kütüphane yüklenir, sonra önbellekten)
 * ve işlev platformun C çağrı kuralıyla doğrudan çağrılır. */
typedef struct {
    char *ad;
    void *tutamak;
} DisKutuphane;
typedef struct {
    char *kutuphane;
    char *ad;
    void *adres;
} DisAdres;
static DisKutuphane *dis_kutuphaneler;
static int64_t dis_kutuphane_sayisi;
static DisAdres *dis_adresler;
static int64_t dis_adres_sayisi;

static void *dis_yukle(const char *ad) {
#ifdef _WIN32
    wchar_t w[1024];
    if (!MultiByteToWideChar(CP_UTF8, 0, ad, -1, w, 1024)) return NULL;
    return (void *)LoadLibraryW(w);
#else
    return dlopen(ad, RTLD_NOW | RTLD_GLOBAL);
#endif
}

/* "m" → libm.so.6 (Linux), libm.dylib (macOS), m.dll (Windows); dosya adı da verilebilir. */
static void *dis_kutuphane_ac(const char *ad, int64_t satir) {
    for (int64_t i = 0; i < dis_kutuphane_sayisi; i++)
        if (!strcmp(dis_kutuphaneler[i].ad, ad)) return dis_kutuphaneler[i].tutamak;
    const char *istenen = ad;
    Tampon denenen = {0};
    void *t = NULL;
    char aday[600];
    int dosya_adi = strchr(ad, '.') || strchr(ad, '/') || strchr(ad, '\\');
#ifdef _WIN32
    const char *kaliplar[] = {"%s.dll", "lib%s.dll", "./%s.dll", NULL};
    if (!strcmp(ad, "c") || !strcmp(ad, "m")) ad = "msvcrt.dll", dosya_adi = 1;
#elif defined(__APPLE__)
    const char *kaliplar[] = {"lib%s.dylib", "./lib%s.dylib", "/usr/local/lib/lib%s.dylib",
                              "/opt/homebrew/lib/lib%s.dylib", NULL};
    if (!strcmp(ad, "c") || !strcmp(ad, "m")) ad = "libSystem.B.dylib", dosya_adi = 1;
#else
    const char *kaliplar[] = {"lib%s.so",   "lib%s.so.6", "lib%s.so.5", "lib%s.so.4", "lib%s.so.3",
                              "lib%s.so.2", "lib%s.so.1", "lib%s.so.0", "./lib%s.so", NULL};
#endif
    if (dosya_adi) {
        t = dis_yukle(ad);
        t_yaz(&denenen, ad);
    } else {
        for (int i = 0; kaliplar[i] && !t; i++) {
            snprintf(aday, sizeof aday, kaliplar[i], ad);
            t = dis_yukle(aday);
            if (denenen.n) t_yaz(&denenen, ", ");
            t_yaz(&denenen, aday);
        }
    }
    if (!t) {
        char m[900];
        snprintf(m, sizeof m, "'%s' C kütüphanesi bulunamadı (denenen: %s); kütüphane kurulu mu?", ad,
                 denenen.v ? denenen.v : "");
        free(denenen.v);
        hata(satir, m);
    }
    free(denenen.v);
    dis_kutuphaneler = ham_buyut(dis_kutuphaneler, sizeof(DisKutuphane) * (size_t)(dis_kutuphane_sayisi + 1));
    dis_kutuphaneler[dis_kutuphane_sayisi].ad = strdup(istenen);
    dis_kutuphaneler[dis_kutuphane_sayisi].tutamak = t;
    dis_kutuphane_sayisi++;
    return t;
}

/* C işlevinin adresi (önbellekli). */
int64_t ohc_dis_bul(int64_t kutuphane, int64_t ad, int64_t satir) {
    for (int64_t i = 0; i < dis_adres_sayisi; i++)
        if (!strcmp(dis_adresler[i].ad, M(ad)) && !strcmp(dis_adresler[i].kutuphane, M(kutuphane)))
            return D(dis_adresler[i].adres);
    void *t = dis_kutuphane_ac(M(kutuphane), satir);
#ifdef _WIN32
    void *a = (void *)GetProcAddress((HMODULE)t, M(ad));
#else
    void *a = dlsym(t, M(ad));
#endif
    if (!a) {
        char m[600];
        snprintf(m, sizeof m, "'%s' işlevi '%s' C kütüphanesinde bulunamadı", M(ad), M(kutuphane));
        hata(satir, m);
    }
    dis_adresler = ham_buyut(dis_adresler, sizeof(DisAdres) * (size_t)(dis_adres_sayisi + 1));
    dis_adresler[dis_adres_sayisi].kutuphane = strdup(M(kutuphane));
    dis_adresler[dis_adres_sayisi].ad = strdup(M(ad));
    dis_adresler[dis_adres_sayisi].adres = a;
    dis_adres_sayisi++;
    return D(a);
}

/* Metin → C'nin const char * değeri (UTF-8, NUL ile biter). Metin çağrı boyunca yaşar. */
int64_t ohc_dis_metin(int64_t m) { return D(M(m)); }

/* C'nin döndürdüğü char * → yeni bir metin (NULL ise boş metin). */
int64_t ohc_dis_metinden(int64_t p) {
    const char *s = (const char *)(uintptr_t)p;
    return s ? metin_yap(s, strlen(s)) : metin_yap("", 0);
}
#endif

#if !defined(_WIN32)
/* ---------------------------------------------------------------------- */
/* CGI (paylaşımlı hosting)                                                */
/* ---------------------------------------------------------------------- */
/* Web sunucusu programı her istekte ayrı çalıştırır; istek ortam değişkenlerinden ve
 * standart girdiden okunur, yanıt standart çıktıya "Status: ..." başlığıyla yazılır.
 * Programın kendi yazdıkları (yaz) yanıtı bozmasın diye standart çıktı geçici bir dosyaya
 * yönlendirilir. Program sun() çağırmadan biterse yazdıkları sayfa olarak gönderilir
 * (PHP gibi). Oturumlar her süreçte kaybolmasın diye veri/.oturumlar/ altında saklanır. */
static int cgi_cikti = -1, cgi_sunuldu;
static FILE *cgi_tampon;

static void cgi_yaz(const char *v, size_t n) {
    while (n) {
        ssize_t k = write(cgi_cikti, v, n);
        if (k < 0 && errno == EINTR) continue;
        if (k <= 0) return;
        v += k;
        n -= (size_t)k;
    }
}

/* HTTP yanıtını ("HTTP/1.1 200 OK\r\n...") CGI yanıtına çevirip gönderir. */
static void cgi_gonder(Tampon *t) {
    cgi_sunuldu = 1;
    const char *v = t->v ? t->v : "";
    const char *satir_sonu = bul_n(v, t->n, "\r\n");
    const char *bas_sonu = bul_n(v, t->n, "\r\n\r\n");
    if (!satir_sonu || !bas_sonu) return;
    Tampon c = {0};
    const char *durum = strchr(v, ' ');
    t_yaz(&c, "Status:");
    t_ekle(&c, durum ? durum : " 500", durum && durum < satir_sonu ? (size_t)(satir_sonu - durum) : 4);
    t_yaz(&c, "\r\n");
    for (const char *p = satir_sonu + 2; p < bas_sonu + 2;) {
        const char *e = bul_n(p, (size_t)(bas_sonu + 2 - p), "\r\n");
        if (!e) break;
        if (!((size_t)(e - p) >= 11 && !harf_duyarsiz_esit(p, "connection:", 11))) t_ekle(&c, p, (size_t)(e - p) + 2);
        p = e + 2;
    }
    t_yaz(&c, "\r\n");
    cgi_yaz(c.v, c.n);
    cgi_yaz(bas_sonu + 4, t->n - (size_t)(bas_sonu + 4 - v));
    free(c.v);
}

/* Program sun() çağırmadan bittiyse yazdıkları (ya da hata sayfası) gönderilir. */
static void cgi_bitir(void) {
    if (cgi_sunuldu) return;
    fflush(stdout);
    Tampon g = {0};
    if (cgi_tampon) {
        rewind(cgi_tampon);
        char b[8192];
        size_t n;
        while ((n = fread(b, 1, sizeof b, cgi_tampon)) > 0) t_ekle(&g, b, n);
    }
    Tampon t = {0};
    Yanit y = {&t, 0, 0, {0}};
    if (cgi_hatali) {
        hata_sayfasi(&y, 500, "Sunucu hatası", son_hata);
    } else {
        const char *v = g.v ? g.v : "";
        while (*v == ' ' || *v == '\n' || *v == '\r' || *v == '\t') v++;
        const char *tur = *v == '<' ? "text/html; charset=utf-8" : "text/plain; charset=utf-8";
        yanit_yaz(&y, 200, tur, g.v ? g.v : "", g.n, NULL, 0);
    }
    cgi_gonder(&t);
    free(t.v);
    free(g.v);
}

static void cgi_baslat(void) {
    const char *g = getenv("GATEWAY_INTERFACE");
    if (!g || strncmp(g, "CGI/", 4)) return;
    cgi_kipi = 1;
    cgi_cikti = dup(1);
    cgi_tampon = tmpfile();
    if (cgi_cikti < 0) return;
    fflush(stdout);
    dup2(cgi_tampon ? fileno(cgi_tampon) : 2, 1);
    atexit(cgi_bitir);
}

static int oturum_kimligi_gecerli(const char *k) {
    if (strlen(k) != 32) return 0;
    for (const char *p = k; *p; p++)
        if (!((*p >= '0' && *p <= '9') || (*p >= 'a' && *p <= 'f'))) return 0;
    return 1;
}

static char *oturum_dosyasi(const char *kimlik) {
    Tampon t = {0};
    t_yaz(&t, veri_klasoru());
    t_yaz(&t, "/.oturumlar/");
    t_yaz(&t, kimlik);
    return t.v;
}

/* Süresi geçmiş oturum dosyaları ara sıra silinir. */
static void eski_oturum_dosyalarini_sil(double simdi) {
    Tampon k = {0};
    t_yaz(&k, veri_klasoru());
    t_yaz(&k, "/.oturumlar");
    DIR *d = opendir(k.v);
    if (d) {
        struct dirent *g;
        while ((g = readdir(d))) {
            if (!oturum_kimligi_gecerli(g->d_name)) continue;
            char *yol = oturum_dosyasi(g->d_name);
            struct stat st;
            if (!stat(yol, &st) && simdi - (double)st.st_mtime > OTURUM_OMRU) unlink(yol);
            free(yol);
        }
        closedir(d);
    }
    free(k.v);
}

static void cgi_isle(const char *istek_tanimi) {
    const char *yontem = getenv("REQUEST_METHOD");
    if (!yontem || !*yontem) yontem = "GET";
    /* Yol: REQUEST_URI'den (yüzde kodlu, ham) programın klasörü çıkarılır; böylece
     * alt klasöre kurulan uygulamada da yollar "/" ile başlar. */
    const char *uri = getenv("REQUEST_URI");
    const char *betik = getenv("SCRIPT_NAME");
    Tampon hedef = {0};
    if (uri && *uri == '/') {
        size_t taban = 0;
        if (betik) {
            const char *bolu = strrchr(betik, '/');
            size_t bn = bolu ? (size_t)(bolu - betik) : 0;
            if (!strncmp(uri, betik, strlen(betik)))
                taban = strlen(betik); /* /uygulama.cgi/yol */
            else if (bn && !strncmp(uri, betik, bn) && (uri[bn] == '/' || uri[bn] == '?' || !uri[bn]))
                taban = bn;
        }
        t_yaz(&hedef, uri + taban);
    } else {
        const char *pi = getenv("PATH_INFO");
        t_yaz(&hedef, pi ? pi : "");
        const char *q = getenv("QUERY_STRING");
        if (q && *q) {
            t_yaz(&hedef, "?");
            t_yaz(&hedef, q);
        }
    }
    if (!hedef.n || hedef.v[0] != '/') {
        Tampon h = {0};
        t_yaz(&h, "/");
        if (hedef.v) t_yaz(&h, hedef.v);
        free(hedef.v);
        hedef = h;
    }

    /* İstek, sunucunun okuduğu biçimde yeniden kurulur. */
    Tampon r = {0};
    t_yaz(&r, yontem);
    t_yaz(&r, " ");
    t_yaz(&r, hedef.v);
    t_yaz(&r, " HTTP/1.0\r\n");
    extern char **environ;
    for (char **e = environ; *e; e++) {
        const char *ad = *e, *esit = strchr(ad, '=');
        if (!esit) continue;
        const char *b = NULL;
        size_t bn = 0;
        if (!strncmp(ad, "HTTP_", 5)) {
            b = ad + 5;
            bn = (size_t)(esit - b);
        } else if (!strncmp(ad, "CONTENT_TYPE=", 13) || !strncmp(ad, "CONTENT_LENGTH=", 15)) {
            b = ad;
            bn = (size_t)(esit - ad);
        }
        if (!b || !bn || !esit[1]) continue;
        for (size_t i = 0; i < bn; i++) {
            /* HTTP_USER_AGENT → user-agent */
            char c = b[i] == '_' ? '-' : (b[i] >= 'A' && b[i] <= 'Z') ? (char)(b[i] + 32) : b[i];
            t_ekle(&r, &c, 1);
        }
        t_yaz(&r, ": ");
        t_yaz(&r, esit + 1);
        t_yaz(&r, "\r\n");
    }
    t_yaz(&r, "\r\n");
    size_t govde_basi = r.n;

    Tampon t = {0};
    Yanit y = {&t, 0, 0, {0}};
    const char *uz = getenv("CONTENT_LENGTH");
    long long govde_n = uz ? atoll(uz) : 0;
    if (en_buyuk_govde < 0) {
        const char *e = getenv("ORHUNCA_EN_BUYUK_GOVDE");
        en_buyuk_govde = e && atoll(e) > 0 ? atoll(e) : 32 * 1024 * 1024;
    }
    if (govde_n < 0 || govde_n > en_buyuk_govde) {
        hata_sayfasi(&y, 413, "İstek gövdesi çok büyük", NULL);
    } else {
        for (long long okunan = 0; okunan < govde_n;) {
            char b[65536];
            size_t iste = (size_t)(govde_n - okunan) < sizeof b ? (size_t)(govde_n - okunan) : sizeof b;
            ssize_t k = read(0, b, iste);
            if (k < 0 && errno == EINTR) continue;
            if (k <= 0) break;
            t_ekle(&r, b, (size_t)k);
            okunan += k;
        }
        size_t gn = r.n - govde_basi;

        /* Oturum: çerezdeki kimliğin dosyası okunur. */
        double simdi = ondalik(ohc_zaman());
        char ilk[33] = {0};
        const char *cerez = getenv("HTTP_COOKIE");
        const char *o = cerez ? strstr(cerez, OTURUM_CEREZI "=") : NULL;
        if (o) {
            o += strlen(OTURUM_CEREZI) + 1;
            size_t n = strcspn(o, "; ");
            if (n == 32) {
                memcpy(ilk, o, 32);
                if (!oturum_kimligi_gecerli(ilk)) ilk[0] = 0;
            }
        }
        if (ilk[0]) {
            char *yol = oturum_dosyasi(ilk);
            struct stat st;
            FILE *f = !stat(yol, &st) && simdi - (double)st.st_mtime <= OTURUM_OMRU ? fopen(yol, "rb") : NULL;
            if (f) {
                Tampon j = {0};
                char b[8192];
                size_t n;
                while ((n = fread(b, 1, sizeof b, f)) > 0) t_ekle(&j, b, n);
                fclose(f);
                t_ekle(&j, "", 0);
                oturumlar = ham_buyut(oturumlar, sizeof(Oturum) * 1);
                oturum_kap = 1;
                memcpy(oturumlar[0].kimlik, ilk, 33);
                oturumlar[0].json = j.v;
                oturumlar[0].son = simdi;
                oturum_sayisi = 1;
            }
            free(yol);
        }
        const char *https = getenv("HTTPS");
        int tls = https && (!strcmp(https, "on") || !strcmp(https, "1"));
        istegi_isle(&y, r.v, r.n, govde_basi, r.v + govde_basi, gn, istek_tanimi, tls);

        /* Oturumlar geri yazılır; silinen oturumun dosyası kaldırılır. */
        if (ilk[0] && !oturum_bul(ilk)) {
            char *yol = oturum_dosyasi(ilk);
            unlink(yol);
            free(yol);
        }
        if (oturum_sayisi) {
            Tampon k = {0};
            t_yaz(&k, veri_klasoru());
            klasor_olustur(k.v);
            t_yaz(&k, "/.oturumlar");
            mkdir(k.v, 0700);
            free(k.v);
        }
        for (int64_t i = 0; i < oturum_sayisi; i++) {
            char *yol = oturum_dosyasi(oturumlar[i].kimlik);
            int fd = open(yol, O_WRONLY | O_CREAT | O_TRUNC, 0600);
            if (fd >= 0) {
                const char *j = oturumlar[i].json ? oturumlar[i].json : "{}";
                size_t n = strlen(j);
                while (n) {
                    ssize_t k = write(fd, j, n);
                    if (k <= 0) break;
                    j += k;
                    n -= (size_t)k;
                }
                close(fd);
            }
            free(yol);
        }
        if (rand() % 50 == 0) eski_oturum_dosyalarini_sil(simdi);
    }
    cgi_gonder(&t);
    free(t.v);
    free(r.v);
    free(hedef.v);
    free(y.cerezler.v);
}
#endif

/* Sunucuyu başlatır ve istekleri karşılar (dönmez). */
void ohc_sun(int64_t kapi, int64_t istek_tanimi) {
#if !defined(_WIN32)
    if (cgi_kipi) {
        cgi_isle(M(istek_tanimi));
        exit(0);
    }
#endif
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
                if (sayi >= EN_COK_BAGLANTI) {
                    /* Çok fazla açık bağlantı: yenisi hemen kapatılır. */
                    soket_kapat(s);
                    continue;
                }
                engelsiz_yap(s);
                if (sayi == kap) {
                    kap = kap ? kap * 2 : 16;
                    baglantilar = ham_buyut(baglantilar, sizeof *baglantilar * kap);
                }
                Baglanti *b = &baglantilar[sayi++];
                memset(b, 0, sizeof *b);
                b->s = s;
                b->son_etkinlik = b->istek_basi = simdi;
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
            if (!b->gelen.n) b->istek_basi = simdi;
            if (canli && b->gelen.n && simdi - b->istek_basi > BASLIK_SURESI &&
                !bul_n(b->gelen.v, b->gelen.n, "\r\n\r\n"))
                canli = 0;
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
