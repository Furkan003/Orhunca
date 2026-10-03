/*
 * Orhunca çalışma zamanı (v0.1).
 *
 * Derleyicinin ürettiği makine kodu yazdırma, metin ve liste işlemleri için bu
 * küçük kütüphaneyi çağırır. Tüm değerler 64 bitlik tamsayı olarak taşınır:
 * metinler NUL ile biten UTF-8 dizilerine, listeler `Liste` yapısına işaretçidir.
 * Bellek v0.1'de geri verilmez; çöp toplama/sahiplik ileriki bir aşamadır.
 *
 * Tip kodları: 0 sayı, 1 metin, 2 mantık, 3 + 4*öğe = liste.
 */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#endif

typedef struct {
    int64_t uzunluk;
    int64_t kapasite;
    int64_t *ogeler;
} Liste;

static void *ayir(size_t n) {
    void *p = malloc(n ? n : 1);
    if (!p) {
        fflush(stdout);
        fprintf(stderr, "Çalışma hatası: bellek yetersiz\n");
        exit(1);
    }
    return p;
}

static void hata(int64_t satir, const char *mesaj) {
    fflush(stdout);
    fprintf(stderr, "Çalışma hatası (satır %" PRId64 "): %s\n", satir, mesaj);
    exit(1);
}

void ohc_basla(void) {
#ifdef _WIN32
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
#endif
}

/* ---------- yazdırma ---------- */

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
    switch (kod % 4) {
    case 0: printf("%" PRId64, d); break;
    case 1:
        if (ic) printf("\"%s\"", (const char *)(intptr_t)d);
        else fputs((const char *)(intptr_t)d, stdout);
        break;
    case 2: fputs(d ? "doğru" : "yanlış", stdout); break;
    default: liste_yaz((Liste *)(intptr_t)d, kod / 4); break;
    }
}

void ohc_yaz(int64_t d, int64_t kod) {
    deger_yaz(d, kod, 0);
    putchar('\n');
}

/* ---------- aritmetik ---------- */

int64_t ohc_bol(int64_t a, int64_t b, int64_t satir) {
    if (b == 0) hata(satir, "sıfıra bölme");
    if (a == INT64_MIN && b == -1) hata(satir, "bölmede taşma");
    return a / b;
}

int64_t ohc_mod(int64_t a, int64_t b, int64_t satir) {
    if (b == 0) hata(satir, "sıfıra göre kalan alınamaz");
    if (b == -1) return 0;
    return a % b;
}

/* ---------- metinler ---------- */

int64_t ohc_metin_birlestir(int64_t a, int64_t b) {
    const char *x = (const char *)(intptr_t)a, *y = (const char *)(intptr_t)b;
    size_t n = strlen(x), m = strlen(y);
    char *s = ayir(n + m + 1);
    memcpy(s, x, n);
    memcpy(s + n, y, m + 1);
    return (int64_t)(intptr_t)s;
}

int64_t ohc_metne_cevir(int64_t d, int64_t kod) {
    if (kod == 1) return d;
    if (kod == 2) return (int64_t)(intptr_t)(d ? "doğru" : "yanlış");
    char *s = ayir(24);
    snprintf(s, 24, "%" PRId64, d);
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

int64_t ohc_metinden_sayi(int64_t a, int64_t satir) {
    const char *s = (const char *)(intptr_t)a;
    char *son;
    while (*s == ' ' || *s == '\t') s++;
    int64_t n = strtoll(s, &son, 10);
    while (*son == ' ' || *son == '\t' || *son == '\r' || *son == '\n') son++;
    if (son == s || *son) {
        char mesaj[256];
        snprintf(mesaj, sizeof mesaj, "'%.200s' bir sayı değil", (const char *)(intptr_t)a);
        hata(satir, mesaj);
    }
    return n;
}

int64_t ohc_oku(void) {
    size_t kap = 64, n = 0;
    char *s = ayir(kap);
    int c;
    fflush(stdout);
    while ((c = getchar()) != EOF && c != '\n') {
        if (n + 1 >= kap) {
            kap *= 2;
            s = realloc(s, kap);
            if (!s) hata(0, "bellek yetersiz");
        }
        s[n++] = (char)c;
    }
    if (n && s[n - 1] == '\r') n--;
    s[n] = 0;
    return (int64_t)(intptr_t)s;
}

/* ---------- listeler ---------- */

int64_t ohc_liste_yeni(void) {
    Liste *l = ayir(sizeof(Liste));
    l->uzunluk = 0;
    l->kapasite = 4;
    l->ogeler = ayir(sizeof(int64_t) * 4);
    return (int64_t)(intptr_t)l;
}

void ohc_liste_ekle(int64_t lp, int64_t d) {
    Liste *l = (Liste *)(intptr_t)lp;
    if (l->uzunluk == l->kapasite) {
        l->kapasite *= 2;
        l->ogeler = realloc(l->ogeler, sizeof(int64_t) * l->kapasite);
        if (!l->ogeler) hata(0, "bellek yetersiz");
    }
    l->ogeler[l->uzunluk++] = d;
}

int64_t ohc_liste_uzunluk(int64_t lp) {
    return ((Liste *)(intptr_t)lp)->uzunluk;
}

static void sinir(Liste *l, int64_t i, int64_t satir) {
    if (i < 0 || i >= l->uzunluk) {
        char mesaj[128];
        snprintf(mesaj, sizeof mesaj, "liste sınırı aşıldı: indeks %" PRId64 ", uzunluk %" PRId64, i, l->uzunluk);
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
static const char *ALFABE[] = {"a", "b", "c", "ç", "d", "e", "f", "g", "ğ", "h", "ı", "i", "j", "k", "l",
                               "m", "n", "o", "ö", "p", "q", "r", "s", "ş", "t", "u", "ü", "v", "w", "x",
                               "y", "z"};
static const char *BUYUK_ALFABE[] = {"A", "B", "C", "Ç", "D", "E", "F", "G", "Ğ", "H", "I", "İ", "J", "K", "L",
                                     "M", "N", "O", "Ö", "P", "Q", "R", "S", "Ş", "T", "U", "Ü", "V", "W", "X",
                                     "Y", "Z"};

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

static int metin_kars(const void *a, const void *b) {
    return metin_karsilastir((const char *)(intptr_t) * (const int64_t *)a, (const char *)(intptr_t) * (const int64_t *)b);
}

void ohc_liste_sirala(int64_t lp, int64_t oge_kodu) {
    Liste *l = (Liste *)(intptr_t)lp;
    qsort(l->ogeler, (size_t)l->uzunluk, sizeof(int64_t), oge_kodu == 1 ? metin_kars : sayi_kars);
}
