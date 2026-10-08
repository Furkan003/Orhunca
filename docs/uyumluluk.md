# Geriye uyumluluk

Orhunca güncellendiğinde yazdığınız programlar bozulmaz. Bu sayfa bunun nasıl sağlandığını anlatır.

[← README](../README.md)

## Kural

**Aynı dil sürümünde yazılmış hiçbir program, Orhunca güncellenince çalışmaz hâle gelmez.**

- Orhunca'nın sürümleri (0.7, 0.8 …) yeni özellik ve düzeltme getirir. Yeni bir özellik ekleniyorsa
  bugüne kadar hata veren bir yazım kabul edilmeye başlayabilir. Bugün çalışan bir yazım ise
  kaldırılmaz.
- Bir yazım değişecekse eski yazım **en az bir sürüm boyunca** çalışmaya devam eder ve uyarı verir:

  ```
  1:9: uyarı: 'eski' 0.9 sürümünde eskidi; yerine 'yeni' yazın (orhunca düzelt kendiliğinden çevirir)
  ```

- Eski yazım ancak **dil sürümü** artarken kaldırılabilir. Dil sürümü, Orhunca'nın sürüm
  numarasından ayrıdır ve çok seyrek değişir.
- Hata düzeltmeleri bu kuralın dışındadır. Örneğin 0.8'de `ondalık("NaN")` artık bir sayı
  sayılmıyor; bu bir yazım değişikliği değil, sessizce yanlış sonuç veren bir hatanın düzeltilmesi.

## Dil sürümü

Her proje, `.ohcproj` dosyasında hangi dil sürümü için yazıldığını belirtir:

```
ad = "notlar"
sürüm = "0.1.0"
giriş = "ana.ohc"
dil = "1"
```

`dil` satırı olmayan eski projeler 1. sürüm sayılır. Proje, kurulu Orhunca'nın tanıdığından daha yeni
bir dil sürümü istiyorsa derleme anlaşılmaz hatalar yerine şunu söyler:

```
bu proje Orhunca dilinin 2. sürümü için yazılmış; kurulu Orhunca (0.9.0) en çok 1. sürümü tanıyor
ipucu: Orhunca'yı güncelleyin: orhunca güncelle
```

## `orhunca düzelt`

Eskiyen yazımları kendiliğinden yenisine çevirir. Metinlere ve yorumlara dokunmaz.

```sh
orhunca düzelt              # bu klasördeki bütün .ohc dosyaları
orhunca düzelt ana.ohc      # tek dosya
orhunca düzelt --denetle    # yalnızca göster, değiştirme (eski yazım varsa çıkış kodu 1)
```

Stüdyo'da eskiyen yazımlar sarı uyarı olarak altı çizili görünür.

Şu an eskiyen bir yazım yok (dil sürümü 1).

## Geliştiriciler için

Bir yazım değiştirilirken (`src/goc.rs`):

1. Yeni yazım eklenir; eski yazım çalışmaya devam eder.
2. `GOCLER` tablosuna `{ eski, yeni, surum }` eklenir. Böylece eski yazım uyarı verir ve
   `orhunca düzelt` onu çevirir.
3. Eski yazım en erken bir sonraki **dil sürümünde** (`DIL_SURUMU`) kaldırılır. Bu, sürüm
   notlarında ve bu sayfada duyurulur.
