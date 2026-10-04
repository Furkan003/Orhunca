# İşlevler

> Kodu isimlendirip tekrar kullanmak: parametreler, döndür, tipler.

**İşlev**, bir işi yapan ve istendiğinde çağrılan kod parçasıdır. `işlev` ile tanımlanır,
`döndür` ile sonucunu verir:

```orhunca
işlev kare(n):
    döndür n * n

kare(7)'yi yaz.
(kare(3) + kare(4))'ü yaz.
```

```cikti
49
25
```

Parametrelerin ve sonucun tipi yazılabilir; bu, hataları erkenden yakalar:

```orhunca
işlev selamla(ad: metin, saat: sayı) -> metin:
    eğer saat 12'den küçükse:
        döndür "Günaydın, " + ad
    döndür "İyi günler, " + ad

selamla("Elif", 9)'u yaz.
selamla("Can", 15)'i yaz.
```

```cikti
Günaydın, Elif
İyi günler, Can
```

İşlevler kendilerini çağırabilir (özyineleme):

```orhunca
işlev faktöriyel(n):
    eğer n 1'den küçük veya eşitse:
        döndür 1
    döndür n * faktöriyel(n - 1)

faktöriyel(5)'i yaz.
```

```cikti
120
```

## Alıştırma: Asal mı?

`asal_mı(n)` işlevini yazın: n asalsa `doğru`, değilse `yanlış` döndürsün. Sonra 1'den 20'ye
kadar asal sayıları yazdırın.

```orhunca baslangic
işlev asal_mı(n) -> mantık:
    döndür yanlış

her i için 1'den 20'ye kadar:
    eğer asal_mı(i) ise:
        i'yi yaz.
```

```cikti
2
3
5
7
11
13
17
19
```

```orhunca cozum
işlev asal_mı(n) -> mantık:
    eğer n 2'den küçükse:
        döndür yanlış
    her b için 2'den n - 1'e kadar:
        eğer n % b == 0 ise:
            döndür yanlış
    döndür doğru

her i için 1'den 20'ye kadar:
    eğer asal_mı(i) ise:
        i'yi yaz.
```
