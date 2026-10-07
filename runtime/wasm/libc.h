/*
 * WebAssembly için en küçük C kütüphanesi: Orhunca çalışma zamanının
 * (orhunca_rt.c) kullandığı işlevler. Sistem çağrısı gerektirenler (çıktı,
 * zaman, dosyalar, ondalık sayı biçimleme) tarayıcıdaki ya da Node'daki
 * JavaScript'ten içe aktarılır (`orhunca.js`).
 */
#ifndef ORHUNCA_WASM_LIBC_H
#define ORHUNCA_WASM_LIBC_H

#include <float.h>
#include <limits.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

#define JS(ad) __attribute__((import_module("js"), import_name(#ad)))

/* ---- JavaScript'ten gelenler ---- */
JS(yaz) void js_yaz(int32_t akis, const char *p, int32_t n);
JS(satir_oku) char *js_satir_oku(void); /* malloc ile ayrılmış satır; dosya sonunda 0 */
JS(zaman) double js_zaman(void);        /* 1970'ten beri saniye */
JS(tarih) void js_tarih(char *tampon);  /* "2026-10-04 14:30:00" + NUL (en az 20 bayt) */
JS(cik) void js_cik(int32_t kod) __attribute__((noreturn));
/* printf'in %g / %f / %e biçimleri: tur 'g', 'f' ya da 'e'; yazılan bayt sayısını döndürür */
JS(ondalik_yaz) int32_t js_ondalik_yaz(double x, int32_t hassasiyet, int32_t tur, char *tampon, int32_t kap);
JS(ondalik_oku) double js_ondalik_oku(const char *p, int32_t *okunan);
JS(matematik) double js_matematik(int32_t islem, double a, double b);
JS(ortam) char *js_ortam(const char *ad);   /* malloc'lu değer ya da 0 */
JS(dosya_oku) char *js_dosya_oku(const char *yol, int32_t *uzunluk); /* malloc'lu ya da 0 */
JS(dosya_yaz) int32_t js_dosya_yaz(const char *yol, const char *veri, int32_t n, int32_t ekle);
JS(dosya_sil) int32_t js_dosya_sil(const char *yol);
JS(dosya_tasi) int32_t js_dosya_tasi(const char *eski, const char *yeni);
JS(klasor_olustur) int32_t js_klasor_olustur(const char *yol);
JS(bekle) void js_bekle(double saniye);
/* HTTP isteği (yöntem 0: GET, 1: POST): malloc'lu gövde, uzunluğu *n'ye; *durum HTTP
 * durum kodu, 0 ise istek başarısız ve dönen metin hata açıklaması. Desteklenmiyorsa 0. */
JS(http) char *js_http(int32_t yontem, const char *adres, const char *govde, int32_t gn, int32_t *n, int32_t *durum);
JS(rastgele_tohum) uint32_t js_rastgele_tohum(void);
JS(arguman_sayisi) int32_t js_arguman_sayisi(void); /* program adı dahil */
JS(arguman) char *js_arguman(int32_t sira);         /* malloc'lu */
/* Çalışma hatası: bir `dene:` bloğunun içindeysek JavaScript istisnası fırlatır
 * (yakala bloğuna geri sarılır); değilse geri döner ve program biter. */
JS(hata_yakala) void js_hata_yakala(void);

/* ---- errno ---- */
extern int errno;
#define EINTR 4
#define ENOENT 2
#define ESRCH 3
#define EACCES 13
#define EISDIR 21
#define ERANGE 34
char *strerror(int hata);

/* ---- inttypes ---- */
#define PRId64 "lld"
#define PRIu64 "llu"

/* ---- stdlib ---- */
void *malloc(size_t n);
void *calloc(size_t a, size_t b);
void *realloc(void *p, size_t n);
void free(void *p);
void exit(int kod) __attribute__((noreturn));
char *getenv(const char *ad);
long long strtoll(const char *s, char **son, int taban);
unsigned long long strtoull(const char *s, char **son, int taban);
double strtod(const char *s, char **son);
long atol(const char *s);
int atoi(const char *s);
void qsort(void *taban, size_t n, size_t boy, int (*kars)(const void *, const void *));

/* ---- string ---- */
size_t strlen(const char *s);
int strcmp(const char *a, const char *b);
int strncmp(const char *a, const char *b, size_t n);
char *strchr(const char *s, int c);
char *strrchr(const char *s, int c);
char *strstr(const char *s, const char *a);
char *strpbrk(const char *s, const char *k);
char *strncat(char *h, const char *k, size_t n);
void *memcpy(void *h, const void *k, size_t n);
void *memmove(void *h, const void *k, size_t n);
void *memset(void *h, int c, size_t n);
int memcmp(const void *a, const void *b, size_t n);
void *memchr(const void *s, int c, size_t n);

/* ---- stdio (çıktı JavaScript'e gider) ---- */
typedef struct DOSYA FILE;
extern FILE *stdin, *stdout, *stderr;
#define EOF (-1)
int printf(const char *bicim, ...);
int fprintf(FILE *f, const char *bicim, ...);
int snprintf(char *t, size_t n, const char *bicim, ...);
int vsnprintf(char *t, size_t n, const char *bicim, va_list a);
int fflush(FILE *f);
size_t fwrite(const void *p, size_t boy, size_t n, FILE *f);
int getchar(void);
/* Dosyalar bütün olarak okunur/yazılır: fopen okur, fclose yazar. */
FILE *fopen(const char *yol, const char *kip);
size_t fread(void *p, size_t boy, size_t n, FILE *f);
int ferror(FILE *f);
int fclose(FILE *f);
int remove(const char *yol);
int rename(const char *eski, const char *yeni);
int mkdir(const char *yol, int kip);

/* ---- math ---- */
#define sqrt(x) __builtin_sqrt(x)
#define fabs(x) __builtin_fabs(x)
#define floor(x) __builtin_floor(x)
#define isfinite(x) __builtin_isfinite(x)
#define isnan(x) __builtin_isnan(x)
#define isinf(x) __builtin_isinf(x)
double sin(double x);
double cos(double x);
double tan(double x);
double log(double x);
double pow(double a, double b);
long long llround(double x);

#endif
