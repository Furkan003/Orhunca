# Kullanıcıdan girdi

> oku() ile klavyeden satır okumak, metni sayıya çevirmek.

`oku()` kullanıcının yazdığı bir satırı metin olarak verir:

```orhunca
"Adın ne?"'yi yaz.
ad = oku()
("Merhaba, " + ad + "!")'yı yaz.
```

```girdi
Ali
```

```cikti
Adın ne?
Merhaba, Ali!
```

Stüdyo'da program girdi beklerken terminale yazıp Enter'a basın. Tarayıcıda deneme
sayfasında girdiler "Program girdisi" kutusuna önceden yazılır.

Okunan her şey metindir. Hesap yapmak için `sayı(...)` ile sayıya çevirin
(virgüllü sayılar için `ondalık(...)`):

```orhunca
"Doğum yılın?"'nı yaz.
yıl = sayı(oku())
("2030 yılında " + (2030 - yıl) + " yaşında olacaksın.")'ı yaz.
```

```girdi
2012
```

```cikti
Doğum yılın?
2030 yılında 18 yaşında olacaksın.
```

## Alıştırma: Toplama makinesi

Kullanıcıdan iki sayı okuyun ve toplamlarını `Toplam: ...` biçiminde yazdırın.

```orhunca baslangic
birinci = sayı(oku())
# ikinci sayıyı okuyun ve toplamı yazdırın
```

```girdi
12
30
```

```cikti
Toplam: 42
```

```orhunca cozum
birinci = sayı(oku())
ikinci = sayı(oku())
("Toplam: " + (birinci + ikinci))'yı yaz.
```
