# Değişkenler ve sayılar

> Değerleri isimlendirmek, aritmetik işlemler, sayı ve ondalık.

**Değişken**, bir değere verdiğimiz isimdir. `=` ile atanır; sonra ismiyle kullanılır:

```orhunca
yaş = 14
ad = "Zeynep"
(ad + " " + yaş + " yaşında.")'yı yaz.
```

```cikti
Zeynep 14 yaşında.
```

Metne bir sayı eklendiğinde sayı kendiliğinden metne çevrilir.

Sayılarla dört işlem yapılır: `+` toplama, `-` çıkarma, `*` çarpma, `/` bölme. Ayrıca `//`
tam bölme (küsuratı atar), `%` bölümden kalanı verir:

```orhunca
a = 17
b = 5
(a + b)'yi yaz.
(a * b)'yi yaz.
(a / b)'yi yaz.
(a // b)'yi yaz.
(a % b)'yi yaz.
```

```cikti
22
85
3.4
3
2
```

`3.4` gibi virgüllü sayılara **ondalık** denir; Orhunca'da ondalık ayıracı noktadır.

Bir değişkenin değeri değiştirilebilir. `+=` ve `-=` kısaltmadır: `puan += 10`,
`puan = puan + 10` demektir:

```orhunca
puan = 50
puan += 10
puan -= 5
("Puan: " + puan)'ı yaz.
```

```cikti
Puan: 55
```

## Alıştırma: Dikdörtgen

Kenarları `uzun = 8` ve `kısa = 3` olan dikdörtgenin alanını ve çevresini yazdırın.

```orhunca baslangic
uzun = 8
kısa = 3
# alan = ...
# çevre = ...
```

```cikti
Alan: 24
Çevre: 22
```

```orhunca cozum
uzun = 8
kısa = 3
alan = uzun * kısa
çevre = 2 * (uzun + kısa)
("Alan: " + alan)'ı yaz.
("Çevre: " + çevre)'yi yaz.
```
