/*
 * WebAssembly için en küçük C kütüphanesi (bkz. libc.h). Bellek ayırıcı,
 * metin işlevleri, biçimli yazma ve sıralama burada; sistemle ilgili işler
 * JavaScript'e devredilir.
 */
#include "libc.h"

int errno;

/* ====================================================================== */
/* Bellek: ikinin kuvveti boyutlarında sınıflar, her sınıfa bir boş liste   */
/* ====================================================================== */

extern unsigned char __heap_base;
static uintptr_t yigin_tepe; /* yığının (heap) kullanılmamış başı */
static void *bos_listeler[40];

#define BASLIK_BOYU 8u

static unsigned sinif_bul(size_t toplam) {
    unsigned k = 4; /* en küçük blok 16 bayt */
    while (((size_t)1 << k) < toplam) k++;
    return k;
}

static void *ham_al(size_t boy) {
    if (!yigin_tepe) yigin_tepe = ((uintptr_t)&__heap_base + 15) & ~(uintptr_t)15;
    uintptr_t p = yigin_tepe;
    uintptr_t son = p + boy;
    size_t kapasite = __builtin_wasm_memory_size(0) * 65536u;
    if (son > kapasite) {
        size_t gereken = (son - kapasite + 65535u) / 65536u;
        if (__builtin_wasm_memory_grow(0, gereken) == (size_t)-1) return 0;
    }
    yigin_tepe = son;
    return (void *)p;
}

void *malloc(size_t n) {
    unsigned k = sinif_bul(n + BASLIK_BOYU);
    if (k >= 40) return 0;
    unsigned char *b = bos_listeler[k];
    if (b) {
        bos_listeler[k] = *(void **)b;
    } else {
        b = ham_al((size_t)1 << k);
        if (!b) return 0;
    }
    *(uint32_t *)b = k;
    return b + BASLIK_BOYU;
}

void free(void *p) {
    if (!p) return;
    unsigned char *b = (unsigned char *)p - BASLIK_BOYU;
    unsigned k = *(uint32_t *)b;
    *(void **)b = bos_listeler[k];
    bos_listeler[k] = b;
}

void *calloc(size_t a, size_t b) {
    if (b && a > (size_t)-1 / b) return 0;
    void *p = malloc(a * b);
    if (p) memset(p, 0, a * b);
    return p;
}

void *realloc(void *p, size_t n) {
    if (!p) return malloc(n);
    unsigned char *b = (unsigned char *)p - BASLIK_BOYU;
    size_t kapasite = ((size_t)1 << *(uint32_t *)b) - BASLIK_BOYU;
    if (n <= kapasite) return p;
    void *yeni = malloc(n);
    if (!yeni) return 0;
    memcpy(yeni, p, kapasite);
    free(p);
    return yeni;
}

/* JavaScript'in metin göndermek için bellek ayırması */
__attribute__((export_name("ohc_js_ayir"))) void *ohc_js_ayir(int32_t n) { return malloc((size_t)n); }

/* ====================================================================== */
/* Metinler ve bellek blokları                                             */
/* ====================================================================== */

size_t strlen(const char *s) {
    const char *p = s;
    while (*p) p++;
    return (size_t)(p - s);
}

int strcmp(const char *a, const char *b) {
    while (*a && *a == *b) a++, b++;
    return (unsigned char)*a - (unsigned char)*b;
}

int strncmp(const char *a, const char *b, size_t n) {
    for (; n; n--, a++, b++) {
        if (*a != *b) return (unsigned char)*a - (unsigned char)*b;
        if (!*a) return 0;
    }
    return 0;
}

char *strchr(const char *s, int c) {
    for (;; s++) {
        if (*s == (char)c) return (char *)s;
        if (!*s) return 0;
    }
}

char *strrchr(const char *s, int c) {
    const char *son = 0;
    for (;; s++) {
        if (*s == (char)c) son = s;
        if (!*s) return (char *)son;
    }
}

char *strstr(const char *s, const char *a) {
    size_t n = strlen(a);
    if (!n) return (char *)s;
    for (; *s; s++)
        if (*s == *a && !strncmp(s, a, n)) return (char *)s;
    return 0;
}

char *strpbrk(const char *s, const char *k) {
    for (; *s; s++)
        if (strchr(k, *s)) return (char *)s;
    return 0;
}

char *strncat(char *h, const char *k, size_t n) {
    char *p = h + strlen(h);
    while (n-- && *k) *p++ = *k++;
    *p = 0;
    return h;
}

void *memcpy(void *h, const void *k, size_t n) {
    unsigned char *a = h;
    const unsigned char *b = k;
    while (n--) *a++ = *b++;
    return h;
}

void *memmove(void *h, const void *k, size_t n) {
    unsigned char *a = h;
    const unsigned char *b = k;
    if (a < b) {
        while (n--) *a++ = *b++;
    } else {
        while (n--) a[n] = b[n];
    }
    return h;
}

void *memset(void *h, int c, size_t n) {
    unsigned char *a = h;
    while (n--) *a++ = (unsigned char)c;
    return h;
}

int memcmp(const void *a, const void *b, size_t n) {
    const unsigned char *x = a, *y = b;
    for (; n; n--, x++, y++)
        if (*x != *y) return *x - *y;
    return 0;
}

void *memchr(const void *s, int c, size_t n) {
    const unsigned char *p = s;
    for (; n; n--, p++)
        if (*p == (unsigned char)c) return (void *)p;
    return 0;
}

char *strerror(int hata) {
    switch (hata) {
    case ENOENT: return "dosya ya da klasör bulunamadı";
    case EACCES: return "izin yok";
    case EISDIR: return "bu bir klasör";
    default: return "bilinmeyen hata";
    }
}

/* ====================================================================== */
/* Sayı okuma                                                               */
/* ====================================================================== */

static int bosluk_mu(char c) { return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\v' || c == '\f'; }

unsigned long long strtoull(const char *s, char **son, int taban) {
    const char *p = s;
    while (bosluk_mu(*p)) p++;
    int eksi = 0;
    if (*p == '+' || *p == '-') eksi = *p++ == '-';
    if ((taban == 0 || taban == 16) && p[0] == '0' && (p[1] == 'x' || p[1] == 'X')) {
        p += 2;
        taban = 16;
    } else if (taban == 0) {
        taban = 10;
    }
    unsigned long long n = 0;
    int okundu = 0, tasti = 0;
    for (;; p++) {
        int r;
        if (*p >= '0' && *p <= '9') r = *p - '0';
        else if (*p >= 'a' && *p <= 'z') r = *p - 'a' + 10;
        else if (*p >= 'A' && *p <= 'Z') r = *p - 'A' + 10;
        else break;
        if (r >= taban) break;
        okundu = 1;
        if (n > (ULLONG_MAX - (unsigned)r) / (unsigned)taban) tasti = 1;
        n = n * (unsigned)taban + (unsigned)r;
    }
    if (son) *son = (char *)(okundu ? p : s);
    if (tasti) {
        errno = ERANGE;
        return ULLONG_MAX;
    }
    return eksi ? -n : n;
}

long long strtoll(const char *s, char **son, int taban) {
    const char *p = s;
    while (bosluk_mu(*p)) p++;
    int eksi = *p == '-';
    char *bitis;
    int onceki = errno;
    errno = 0;
    unsigned long long n = strtoull(p, &bitis, taban);
    if (son) *son = bitis == p ? (char *)s : bitis;
    if (eksi) n = -n; /* strtoull eksiyi uygulamıştı; mutlak değere dön */
    if (errno == ERANGE || (!eksi && n > (unsigned long long)LLONG_MAX) ||
        (eksi && n > (unsigned long long)LLONG_MAX + 1)) {
        errno = ERANGE;
        return eksi ? LLONG_MIN : LLONG_MAX;
    }
    errno = onceki;
    return eksi ? (long long)(0 - n) : (long long)n;
}

long atol(const char *s) { return (long)strtoll(s, 0, 10); }
int atoi(const char *s) { return (int)strtoll(s, 0, 10); }

double strtod(const char *s, char **son) {
    int32_t okunan = 0;
    double d = js_ondalik_oku(s, &okunan);
    if (son) *son = (char *)s + okunan;
    return d;
}

/* ====================================================================== */
/* Sıralama (birleştirmeli; kararlı)                                       */
/* ====================================================================== */

static void birlestir_sirala(unsigned char *a, unsigned char *gecici, size_t n, size_t boy,
                             int (*kars)(const void *, const void *)) {
    if (n < 2) return;
    size_t orta = n / 2;
    birlestir_sirala(a, gecici, orta, boy, kars);
    birlestir_sirala(a + orta * boy, gecici, n - orta, boy, kars);
    size_t i = 0, j = orta, k = 0;
    while (i < orta && j < n) {
        if (kars(a + j * boy, a + i * boy) < 0) memcpy(gecici + k++ * boy, a + j++ * boy, boy);
        else memcpy(gecici + k++ * boy, a + i++ * boy, boy);
    }
    while (i < orta) memcpy(gecici + k++ * boy, a + i++ * boy, boy);
    while (j < n) memcpy(gecici + k++ * boy, a + j++ * boy, boy);
    memcpy(a, gecici, n * boy);
}

void qsort(void *taban, size_t n, size_t boy, int (*kars)(const void *, const void *)) {
    if (n < 2) return;
    unsigned char *gecici = malloc(n * boy);
    if (!gecici) return;
    birlestir_sirala(taban, gecici, n, boy, kars);
    free(gecici);
}

/* ====================================================================== */
/* Biçimli yazma                                                           */
/* ====================================================================== */

typedef struct {
    char *t;
    size_t kap, n;
} Cikti;

static void c_ekle(Cikti *c, char ch) {
    if (c->n + 1 < c->kap) c->t[c->n] = ch;
    c->n++;
}

static void c_dolgu(Cikti *c, char ch, int adet) {
    while (adet-- > 0) c_ekle(c, ch);
}

int vsnprintf(char *t, size_t kap, const char *b, va_list a) {
    Cikti c = {t, kap, 0};
    for (; *b; b++) {
        if (*b != '%') {
            c_ekle(&c, *b);
            continue;
        }
        b++;
        int sola = 0, sifir = 0, genislik = 0, hassasiyet = -1, uzun = 0, boyut = 0;
        for (;; b++) {
            if (*b == '-') sola = 1;
            else if (*b == '0') sifir = 1;
            else if (*b == '+' || *b == ' ' || *b == '#') {
            } else break;
        }
        while (*b >= '0' && *b <= '9') genislik = genislik * 10 + (*b++ - '0');
        if (*b == '.') {
            b++;
            hassasiyet = 0;
            while (*b >= '0' && *b <= '9') hassasiyet = hassasiyet * 10 + (*b++ - '0');
        }
        while (*b == 'l') uzun++, b++;
        if (*b == 'z') boyut = 1, b++;
        char sayi[72];
        int sn = 0;
        switch (*b) {
        case 'd':
        case 'i': {
            long long v = uzun >= 2 ? va_arg(a, long long) : uzun ? (long long)va_arg(a, long) : (long long)va_arg(a, int);
            unsigned long long m = v < 0 ? 0ull - (unsigned long long)v : (unsigned long long)v;
            do sayi[sn++] = (char)('0' + m % 10); while (m /= 10);
            if (v < 0) sayi[sn++] = '-';
            goto tamsayi;
        }
        case 'u':
        case 'x':
        case 'X': {
            unsigned long long m = boyut ? (unsigned long long)va_arg(a, size_t)
                                   : uzun >= 2 ? va_arg(a, unsigned long long)
                                   : uzun ? (unsigned long long)va_arg(a, unsigned long)
                                          : (unsigned long long)va_arg(a, unsigned);
            unsigned taban = *b == 'u' ? 10 : 16;
            const char *rakam = *b == 'X' ? "0123456789ABCDEF" : "0123456789abcdef";
            do sayi[sn++] = rakam[m % taban]; while (m /= taban);
            goto tamsayi;
        }
        tamsayi:
            if (!sola) {
                if (sifir) {
                    /* eksi işareti dolgudan önce */
                    if (sayi[sn - 1] == '-') {
                        c_ekle(&c, '-');
                        sn--;
                        genislik--;
                    }
                    c_dolgu(&c, '0', genislik - sn);
                } else {
                    c_dolgu(&c, ' ', genislik - sn);
                }
            }
            for (int i = sn - 1; i >= 0; i--) c_ekle(&c, sayi[i]);
            if (sola) c_dolgu(&c, ' ', genislik - sn);
            break;
        case 'c': c_ekle(&c, (char)va_arg(a, int)); break;
        case 's': {
            const char *s = va_arg(a, const char *);
            if (!s) s = "(null)";
            int n = 0;
            while (s[n] && (hassasiyet < 0 || n < hassasiyet)) n++;
            if (!sola) c_dolgu(&c, ' ', genislik - n);
            for (int i = 0; i < n; i++) c_ekle(&c, s[i]);
            if (sola) c_dolgu(&c, ' ', genislik - n);
            break;
        }
        case 'g':
        case 'f':
        case 'e': {
            double x = va_arg(a, double);
            char d[64];
            int n = js_ondalik_yaz(x, hassasiyet < 0 ? 6 : hassasiyet, *b, d, sizeof d);
            if (!sola) c_dolgu(&c, ' ', genislik - n);
            for (int i = 0; i < n; i++) c_ekle(&c, d[i]);
            if (sola) c_dolgu(&c, ' ', genislik - n);
            break;
        }
        case '%': c_ekle(&c, '%'); break;
        default: c_ekle(&c, '%'); c_ekle(&c, *b); break;
        }
    }
    if (kap) t[c.n < kap ? c.n : kap - 1] = 0;
    return (int)c.n;
}

int snprintf(char *t, size_t n, const char *b, ...) {
    va_list a;
    va_start(a, b);
    int r = vsnprintf(t, n, b, a);
    va_end(a);
    return r;
}

/* ====================================================================== */
/* Akışlar                                                                  */
/* ====================================================================== */

struct DOSYA {
    int32_t no; /* 0 girdi, 1 çıktı, 2 hata; -1 diskteki dosya */
    char *yol;
    char *veri;
    size_t n, kap, konum;
    int yazma, ekle;
};
static FILE girdi = {.no = 0}, cikti = {.no = 1}, hata_akisi = {.no = 2};
FILE *stdin = &girdi, *stdout = &cikti, *stderr = &hata_akisi;

static char *kopya(const char *s) {
    size_t n = strlen(s) + 1;
    char *k = malloc(n);
    if (k) memcpy(k, s, n);
    return k;
}

FILE *fopen(const char *yol, const char *kip) {
    FILE *f = calloc(1, sizeof(FILE));
    if (!f) return 0;
    f->no = -1;
    if (kip[0] == 'r') {
        int32_t n = 0;
        f->veri = js_dosya_oku(yol, &n);
        if (!f->veri) {
            free(f);
            errno = ENOENT;
            return 0;
        }
        f->n = (size_t)n;
    } else {
        f->yazma = 1;
        f->ekle = kip[0] == 'a';
        f->yol = kopya(yol);
    }
    return f;
}

size_t fread(void *p, size_t boy, size_t n, FILE *f) {
    if (!boy || f->yazma || f->no >= 0) return 0;
    size_t kalan = (f->n - f->konum) / boy;
    if (n > kalan) n = kalan;
    memcpy(p, f->veri + f->konum, n * boy);
    f->konum += n * boy;
    return n;
}

int ferror(FILE *f) {
    (void)f;
    return 0;
}

int fclose(FILE *f) {
    int sonuc = 0;
    if (f->yazma && !js_dosya_yaz(f->yol, f->veri ? f->veri : "", (int32_t)f->n, f->ekle)) {
        errno = EACCES;
        sonuc = EOF;
    }
    free(f->veri);
    free(f->yol);
    free(f);
    return sonuc;
}

int remove(const char *yol) { return js_dosya_sil(yol) ? 0 : -1; }
int rename(const char *eski, const char *yeni) { return js_dosya_tasi(eski, yeni) ? 0 : -1; }
int mkdir(const char *yol, int kip) {
    (void)kip;
    return js_klasor_olustur(yol) ? 0 : -1;
}

static int akisa_yaz(FILE *f, const char *b, va_list a) {
    char kucuk[512];
    va_list k;
    va_copy(k, a);
    int n = vsnprintf(kucuk, sizeof kucuk, b, k);
    va_end(k);
    if (n < (int)sizeof kucuk) {
        js_yaz(f->no, kucuk, n);
        return n;
    }
    char *buyuk = malloc((size_t)n + 1);
    vsnprintf(buyuk, (size_t)n + 1, b, a);
    js_yaz(f->no, buyuk, n);
    free(buyuk);
    return n;
}

int printf(const char *b, ...) {
    va_list a;
    va_start(a, b);
    int n = akisa_yaz(stdout, b, a);
    va_end(a);
    return n;
}

int fprintf(FILE *f, const char *b, ...) {
    va_list a;
    va_start(a, b);
    int n = akisa_yaz(f, b, a);
    va_end(a);
    return n;
}

int fflush(FILE *f) {
    (void)f;
    return 0;
}

size_t fwrite(const void *p, size_t boy, size_t n, FILE *f) {
    size_t bayt = boy * n;
    if (f->no >= 0) {
        js_yaz(f->no, p, (int32_t)bayt);
        return n;
    }
    if (!f->yazma) return 0;
    if (f->n + bayt > f->kap) {
        size_t kap = f->kap ? f->kap : 256;
        while (kap < f->n + bayt) kap *= 2;
        char *yeni = realloc(f->veri, kap);
        if (!yeni) return 0;
        f->veri = yeni;
        f->kap = kap;
    }
    memcpy(f->veri + f->n, p, bayt);
    f->n += bayt;
    return n;
}

/* Klavyeden satır satır okunur; satır sonu eklenir. */
static char *okunan_satir;
static size_t okunan_konum;
static int girdi_bitti;

int getchar(void) {
    for (;;) {
        if (okunan_satir) {
            unsigned char c = (unsigned char)okunan_satir[okunan_konum];
            if (c) {
                okunan_konum++;
                return c;
            }
            free(okunan_satir);
            okunan_satir = 0;
            return '\n';
        }
        if (girdi_bitti) return EOF;
        okunan_satir = js_satir_oku();
        okunan_konum = 0;
        if (!okunan_satir) {
            girdi_bitti = 1;
            return EOF;
        }
    }
}

/* ====================================================================== */
/* Sistem                                                                   */
/* ====================================================================== */

void exit(int kod) { js_cik(kod); }

char *getenv(const char *ad) { return js_ortam(ad); }

double sin(double x) { return js_matematik(1, x, 0); }
double cos(double x) { return js_matematik(2, x, 0); }
double tan(double x) { return js_matematik(3, x, 0); }
double log(double x) { return js_matematik(4, x, 0); }
double pow(double a, double b) { return js_matematik(5, a, b); }

long long llround(double x) {
    double y = x < 0 ? __builtin_ceil(x - 0.5) : __builtin_floor(x + 0.5);
    return (long long)y;
}
