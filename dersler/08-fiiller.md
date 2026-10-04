# Kendi fiillerin

> Türkçe cümle gibi çağrılan işlevler: parametreler hâl ekleriyle.

Orhunca'da kendi fiillerinizi tanımlayabilirsiniz. Parametreler hâl ekleriyle yazılır; çağrı
bir Türkçe cümleye benzer:

```orhunca
fiil (kişi: metin)'yi selamla:
    ("Merhaba, " + kişi + "!")'yı yaz.

"Ayşe"'yi selamla.
"Mehmet"'i selamla.
```

```cikti
Merhaba, Ayşe!
Merhaba, Mehmet!
```

Ekler parametrenin görevini anlatır: `'yi` belirtme (nesne), `'ye` yönelme (hedef), `'den`
ayrılma (kaynak). Bu yüzden çağrıda sıra önemli değildir:

```orhunca
fiil (miktar: ondalık)'ı (bakiye: ondalık)'dan düş -> ondalık:
    döndür bakiye - miktar

cüzdan = 50.0
cüzdan = 12.5'i cüzdan'dan düş
cüzdan = cüzdan'dan 7.5'i düş
cüzdan'ı yaz.
```

```cikti
30.0
```

Değer döndüren fiiller ifadelerde ve koşullarda kullanılabilir:

```orhunca
fiil sayı'yı karele:
    döndür sayı * sayı

eğer 5'i karele 20'den büyükse:
    "25, 20'den büyük"'ü yaz.
```

```cikti
25, 20'den büyük
```

## Alıştırma: Geri sayım

`n'den geri_say` fiilini tanımlayın: n'den 1'e kadar sayıları yazsın, sonra `Kalkış!` yazsın.

```orhunca baslangic
fiil n'den geri_say:
    "Kalkış!"'ı yaz.

3'ten geri_say.
```

```cikti
3
2
1
Kalkış!
```

```orhunca cozum
fiil n'den geri_say:
    n 0'dan büyükken:
        n'yi yaz.
        n -= 1
    "Kalkış!"'ı yaz.

3'ten geri_say.
```
