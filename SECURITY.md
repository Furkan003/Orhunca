# Güvenlik

## Desteklenen sürümler

Güvenlik düzeltmeleri en son sürüme yapılır. Stüdyo yeni sürümleri kendisi bildirir;
komut satırında `orhunca güncelle` ile güncelleyin.

## Güvenlik açığı bildirmek

Lütfen güvenlik açıklarını herkese açık bir konu (issue) olarak **açmayın**. Bunun yerine
GitHub'ın gizli bildirim özelliğini kullanın:
[Security → Report a vulnerability](https://github.com/Furkan003/Orhunca/security/advisories/new).

Bildiriminizde açığın nasıl tetiklendiğini (mümkünse küçük bir örnekle), etkisini ve hangi
sürümde gördüğünüzü yazın. En geç bir hafta içinde yanıt vermeye çalışırız; düzeltme
yayımlandıktan sonra bildirene (isterse) teşekkür edilir.

## Kapsam

- Derleyici ve Stüdyo (`orhunca stüdyo`): Stüdyo yalnızca bu bilgisayardan (127.0.0.1) ve oturum
  anahtarıyla erişilebilir olmalıdır; açılan projenin dışındaki dosyalara erişilememelidir.
- Orhunca ile yazılmış web sunucuları (`sun()`): istek ayrıştırma, statik dosyalar, oturumlar.
- Kurulum dosyaları ve güncelleme (indirilen dosyalar SHA-256 ile doğrulanır).
