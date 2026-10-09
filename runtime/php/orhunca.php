<?php
/*
 * Orhunca çalışma zamanı (PHP). `orhunca yayınla --php` bu dosyayı orhunca/calisma.php
 * olarak programın yanına koyar. Orhunca'nın C çalışma zamanının (runtime/orhunca_rt.c)
 * PHP karşılığıdır: değerler, listeler, sözlükler, metin biçimi, modeller (PDO ile MySQL
 * ya da SQLite), web yolları, oturumlar ve hata sayfaları. Ön kütüphanenin (Orhunca ile
 * yazılmış işlevler: düzenli ifadeler, tarihler, CSV...) çevirisi programın içindedir.
 * PHP 8.1 ya da sonrası gerekir.
 */

declare(strict_types=0);
mb_internal_encoding('UTF-8');

/** Orhunca çalışma hatası: mesajı Orhunca'nın verdiği mesajdır. */
final class OHata extends Exception
{
    /** Ön kütüphanenin verdiği hatalarda çağıranın satırı */
    public ?int $ohc_satir = null;
}

function o_hata(string $m): never
{
    throw new OHata($m);
}

/** hata_ver_satırda: ön kütüphanenin hataları çağıranın satırıyla */
function o_hata_satirda(string $m, int $satir): never
{
    $e = new OHata($m);
    $e->ohc_satir = $satir;
    throw $e;
}

// Xdebug kuruluysa çağrı derinliği sınırı (öntanımlı 256/512) özyinelemeli programları keser;
// derinliği Orhunca kendisi sınırlar (ODerinlik).
@ini_set('xdebug.max_nesting_level', '100000');

// PHP uyarıları (boş nesnenin alanı, tanımsız değişken...) çalışma hatasına çevrilir.
set_error_handler(function (int $no, string $m) {
    if (!(error_reporting() & $no)) {
        return false;
    }
    if (str_contains($m, 'on null')) {
        o_hata('boş bir model değerinin alanı kullanılamaz (değişkene henüz değer atanmamış olabilir)');
    }
    if (str_contains($m, 'Division by zero') || str_contains($m, 'Modulo by zero')) {
        o_hata('sıfıra bölme');
    }
    o_hata($m);
});

/** Özyineleme sınırı: her kullanıcı işlevi bir tane oluşturur, işlevden çıkınca yok olur. */
final class ODerinlik
{
    public static int $d = 0;

    public function __construct()
    {
        if (++self::$d > 20000) {
            self::$d--;
            o_hata('çok derin özyineleme: işlevler birbirini bitmeyecek kadar çok çağırıyor (bitiş koşulunu denetleyin)');
        }
    }

    public function __destruct()
    {
        self::$d--;
    }
}

// ---------------------------------------------------------------------------
// Listeler ve sözlükler (Orhunca'da başvuru tipleridir: işleve verilen liste değişir)
// ---------------------------------------------------------------------------

final class OListe implements ArrayAccess, Countable, IteratorAggregate
{
    public array $o;

    public function __construct(array $o = [])
    {
        $this->o = array_values($o);
    }

    private function sinir($i): void
    {
        if (!is_int($i) || $i < 0 || $i >= count($this->o)) {
            o_hata('liste sınırı aşıldı: sıra ' . o_metin($i) . ', uzunluk ' . count($this->o));
        }
    }

    public function offsetExists($i): bool
    {
        return is_int($i) && $i >= 0 && $i < count($this->o);
    }

    public function offsetGet($i): mixed
    {
        $this->sinir($i);
        return $this->o[$i];
    }

    public function offsetSet($i, $v): void
    {
        if ($i === null) {
            $this->o[] = $v;
            return;
        }
        $this->sinir($i);
        $this->o[$i] = $v;
    }

    public function offsetUnset($i): void
    {
        $this->sinir($i);
        array_splice($this->o, $i, 1);
    }

    public function count(): int
    {
        return count($this->o);
    }

    public function getIterator(): Iterator
    {
        return new ArrayIterator($this->o);
    }
}

final class OSozluk implements ArrayAccess, Countable, IteratorAggregate
{
    /** iç anahtar => [anahtar, değer] (PHP "1" anahtarını sayıya çevirdiği için) */
    public array $o = [];

    public static function ic($k): string
    {
        return (is_int($k) ? 'i' : 's') . $k;
    }

    public static function yap(array $ciftler): OSozluk
    {
        $s = new OSozluk();
        foreach ($ciftler as [$k, $v]) {
            $s->o[self::ic($k)] = [$k, $v];
        }
        return $s;
    }

    public function offsetExists($k): bool
    {
        return isset($this->o[self::ic($k)]);
    }

    public function offsetGet($k): mixed
    {
        $c = $this->o[self::ic($k)] ?? null;
        if ($c === null) {
            o_hata('sözlükte ' . o_metin($k, true) . ' anahtarı yok (önce içerir(sözlük, anahtar) ile denetleyin)');
        }
        return $c[1];
    }

    public function offsetSet($k, $v): void
    {
        $this->o[self::ic($k)] = [$k, $v];
    }

    public function offsetUnset($k): void
    {
        unset($this->o[self::ic($k)]);
    }

    public function count(): int
    {
        return count($this->o);
    }

    /** Döngüde anahtarlar gezilir */
    public function getIterator(): Iterator
    {
        return new ArrayIterator(array_column(array_values($this->o), 0));
    }

    public function anahtarlar(): array
    {
        return array_column(array_values($this->o), 0);
    }

    public function degerler(): array
    {
        return array_column(array_values($this->o), 1);
    }
}

function o_liste(...$o): OListe
{
    return new OListe($o);
}

// ---------------------------------------------------------------------------
// Metin biçimi (Orhunca'nın yazdığı gibi)
// ---------------------------------------------------------------------------

/** C'nin %.15g biçimi: 2 → "2.0", 0.1 + 0.2 → "0.3", 1e20 → "1e+20" */
function o_ondalik_metni(float $x): string
{
    if (is_nan($x)) {
        return 'nan';
    }
    if (is_infinite($x)) {
        return $x > 0 ? 'inf' : '-inf';
    }
    if ($x == 0.0) {
        return fdiv(1.0, $x) < 0 ? '-0.0' : '0.0';
    }
    $e = sprintf('%.14e', $x);
    [$g, $us] = explode('e', $e);
    $u = (int)$us;
    if ($u < -4 || $u >= 15) {
        $g = rtrim(rtrim($g, '0'), '.');
        $m = $g . 'e' . ($u < 0 ? '-' : '+') . str_pad((string)abs($u), 2, '0', STR_PAD_LEFT);
    } else {
        $m = sprintf('%.' . (14 - $u) . 'f', $x);
        if (str_contains($m, '.')) {
            $m = rtrim(rtrim($m, '0'), '.');
        }
    }
    return preg_match('/[.eni]/', $m) ? $m : $m . '.0';
}

function o_metin($x, bool $ic = false): string
{
    if (is_string($x)) {
        return $ic ? '"' . $x . '"' : $x;
    }
    if (is_int($x)) {
        return (string)$x;
    }
    if (is_bool($x)) {
        return $x ? 'doğru' : 'yanlış';
    }
    if (is_float($x)) {
        return o_ondalik_metni($x);
    }
    if ($x instanceof OListe) {
        return '[' . implode(', ', array_map(fn($o) => o_metin($o, true), $x->o)) . ']';
    }
    if ($x instanceof OSozluk) {
        $p = [];
        foreach ($x->o as [$k, $v]) {
            $p[] = o_metin($k, true) . ': ' . o_metin($v, true);
        }
        return '{' . implode(', ', $p) . '}';
    }
    if ($x instanceof OModel) {
        $p = [];
        foreach (array_keys($x::ALANLAR) as $a) {
            $p[] = $a . ': ' . o_metin($x->$a, true);
        }
        return o_sinif_adi($x) . '(' . implode(', ', $p) . ')';
    }
    if ($x === null) {
        return 'boş';
    }
    return (string)$x;
}

function o_yaz($x): void
{
    echo o_metin($x), "\n";
}

// ---------------------------------------------------------------------------
// Sayılar
// ---------------------------------------------------------------------------

function o_bol($a, $b): float
{
    if ($b == 0) {
        o_hata('sıfıra bölme');
    }
    return $a / $b;
}

function o_tambol(int $a, int $b): int
{
    if ($b == 0) {
        o_hata('sıfıra bölme');
    }
    return intdiv($a, $b) - ((($a % $b) != 0 && (($a < 0) != ($b < 0))) ? 1 : 0);
}

function o_kalan($a, $b)
{
    if (is_int($a) && is_int($b)) {
        if ($b == 0) {
            o_hata('sıfıra göre kalan alınamaz');
        }
        $k = $a % $b;
        return ($k != 0 && (($k < 0) != ($b < 0))) ? $k + $b : $k;
    }
    if ($b == 0) {
        o_hata('sıfıra göre kalan alınamaz');
    }
    $k = fmod($a, $b);
    return ($k != 0 && (($k < 0) != ($b < 0))) ? $k + $b : $k;
}

/** Türk alfabesine göre sıra: büyük ve küçük harf aynı yerde (I → ı, İ → i) */
function o_harf_sirasi(string $c): int
{
    static $sira = null;
    if ($sira === null) {
        $sira = [];
        $k = ['a', 'b', 'c', 'ç', 'd', 'e', 'f', 'g', 'ğ', 'h', 'ı', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'ö', 'p', 'q', 'r', 's', 'ş', 't', 'u', 'ü', 'v', 'w', 'x', 'y', 'z'];
        $b = ['A', 'B', 'C', 'Ç', 'D', 'E', 'F', 'G', 'Ğ', 'H', 'I', 'İ', 'J', 'K', 'L', 'M', 'N', 'O', 'Ö', 'P', 'Q', 'R', 'S', 'Ş', 'T', 'U', 'Ü', 'V', 'W', 'X', 'Y', 'Z'];
        foreach ($k as $i => $h) {
            $sira[$h] = 1000 + $i;
            $sira[$b[$i]] = 1000 + $i;
        }
    }
    if (isset($sira[$c])) {
        return $sira[$c];
    }
    $n = mb_ord($c);
    return $n < 0x80 ? $n : 100000 + $n;
}

function o_kars(string $a, string $b): int
{
    $x = mb_str_split($a);
    $y = mb_str_split($b);
    $n = min(count($x), count($y));
    for ($i = 0; $i < $n; $i++) {
        $p = o_harf_sirasi($x[$i]);
        $q = o_harf_sirasi($y[$i]);
        if ($p != $q) {
            return $p < $q ? -1 : 1;
        }
    }
    if (count($x) != count($y)) {
        return count($x) > count($y) ? 1 : -1;
    }
    return $a <=> $b;
}

function o_genel_kars($a, $b): int
{
    return is_string($a) ? o_kars($a, $b) : ($a <=> $b);
}

// ---------------------------------------------------------------------------
// Yerleşik işlevler
// ---------------------------------------------------------------------------

function o_uzunluk($x): int
{
    return is_string($x) ? mb_strlen($x) : count($x);
}

function o_cevirme_hatasi($a, string $ne): never
{
    o_hata("'" . mb_substr((string)$a, 0, 200) . "' bir $ne değil");
}

function o_sayi($x): int
{
    if (is_int($x)) {
        return $x;
    }
    if (is_float($x)) {
        if (!is_finite($x) || abs($x) >= 9.2e18) {
            o_hata('ondalık sayı tamsayıya sığmıyor');
        }
        return (int)$x;
    }
    if (preg_match('/^\s*([-+]?\d+)\s*$/', $x, $m)) {
        $v = intval($m[1]);
        if (strlen(ltrim($m[1], '+-0')) > 18 && (string)$v !== ltrim($m[1], '+')) {
            o_cevirme_hatasi($x, 'sayı');
        }
        return $v;
    }
    o_cevirme_hatasi($x, 'sayı');
}

function o_ondalik($x): float
{
    if (!is_string($x)) {
        return (float)$x;
    }
    $t = str_replace(',', '.', trim($x));
    if ($t === '' || !is_numeric($t)) {
        o_cevirme_hatasi($x, 'ondalık sayı');
    }
    return (float)$t;
}

function o_sayi_mi(string $m): bool
{
    return (bool)preg_match('/^\s*[-+]?\d+\s*$/', $m);
}

function o_ondalik_mi(string $m): bool
{
    $t = str_replace(',', '.', trim($m));
    return $t !== '' && is_numeric($t);
}

function o_yuvarla($x, int $basamak = -1)
{
    if ($basamak < 0) {
        $y = floor(abs($x) + 0.5);
        return (int)($x < 0 ? -$y : $y);
    }
    $k = 10 ** $basamak;
    $y = floor(abs($x) * $k + 0.5) / $k;
    return (float)($x < 0 ? -$y : $y);
}

function o_parca($x, int $bas, int $uz)
{
    $bas = max($bas, 0);
    $uz = max($uz, 0);
    if (is_string($x)) {
        return mb_substr($x, $bas, $uz);
    }
    return new OListe(array_slice($x->o, $bas, $uz));
}

function o_birlestir(OListe $l, string $ayrac): string
{
    return implode($ayrac, array_map('o_metin', $l->o));
}

function o_esit($a, $b): bool
{
    if ($a instanceof OListe && $b instanceof OListe) {
        if (count($a->o) != count($b->o)) {
            return false;
        }
        foreach ($a->o as $i => $v) {
            if (!o_esit($v, $b->o[$i])) {
                return false;
            }
        }
        return true;
    }
    return $a === $b || ((is_int($a) || is_float($a)) && (is_int($b) || is_float($b)) && $a == $b);
}

function o_icerir($x, $aranan): bool
{
    if (is_string($x)) {
        return str_contains($x, $aranan);
    }
    if ($x instanceof OSozluk) {
        return isset($x->o[OSozluk::ic($aranan)]);
    }
    foreach ($x->o as $v) {
        if (o_esit($v, $aranan)) {
            return true;
        }
    }
    return false;
}

function o_bul($x, $aranan): int
{
    if (is_string($x)) {
        $i = mb_strpos($x, $aranan);
        return $i === false ? -1 : $i;
    }
    foreach ($x->o as $i => $v) {
        if (o_esit($v, $aranan)) {
            return $i;
        }
    }
    return -1;
}

function o_harfler(string $m): OListe
{
    return new OListe($m === '' ? [] : mb_str_split($m));
}

function o_kodlar(string $m): OListe
{
    return new OListe($m === '' ? [] : array_map('mb_ord', mb_str_split($m)));
}

function o_gecerli_kod(int $n): void
{
    if ($n <= 0 || $n > 0x10FFFF || ($n >= 0xD800 && $n <= 0xDFFF)) {
        o_hata("$n geçerli bir karakter kodu değil");
    }
}

function o_kodlardan(OListe $l): string
{
    $s = '';
    foreach ($l->o as $n) {
        o_gecerli_kod($n);
        $s .= mb_chr($n);
    }
    return $s;
}

function o_kod(string $m): int
{
    return $m === '' ? 0 : mb_ord(mb_substr($m, 0, 1));
}

function o_karakter(int $n): string
{
    o_gecerli_kod($n);
    return mb_chr($n);
}

function o_harf(string $m, int $i): string
{
    $n = mb_strlen($m);
    if ($i < 0 || $i >= $n) {
        o_hata("metnin sınırı aşıldı: sıra $i, uzunluk $n");
    }
    return mb_substr($m, $i, 1);
}

function o_sil($x, $k)
{
    if ($x instanceof OSozluk) {
        unset($x->o[OSozluk::ic($k)]);
        return null;
    }
    $v = $x[$k];
    array_splice($x->o, $k, 1);
    return $v;
}

function o_cikar(OListe $l, $v): void
{
    $i = o_bul($l, $v);
    if ($i >= 0) {
        array_splice($l->o, $i, 1);
    }
}

function o_ters($x)
{
    if (is_string($x)) {
        return implode('', array_reverse(mb_str_split($x)));
    }
    return new OListe(array_reverse($x->o));
}

function o_karistir(OListe $l): void
{
    shuffle($l->o);
}

function o_kopya($x)
{
    if ($x instanceof OSozluk) {
        $s = new OSozluk();
        $s->o = $x->o;
        return $s;
    }
    return new OListe($x->o);
}

function o_sirala(OListe $l): void
{
    usort($l->o, 'o_genel_kars');
}

function o_en(array $a, int $yon)
{
    if (count($a) == 1 && $a[0] instanceof OListe) {
        $a = $a[0]->o;
        if (!$a) {
            o_hata($yon > 0 ? 'boş listenin en büyük öğesi yok' : 'boş listenin en küçük öğesi yok');
        }
    }
    $en = $a[0];
    foreach ($a as $v) {
        $k = o_genel_kars($v, $en);
        if ($yon > 0 ? $k > 0 : $k < 0) {
            $en = $v;
        }
    }
    return $en;
}

function o_en_buyuk(...$a)
{
    return o_en($a, 1);
}

function o_en_kucuk(...$a)
{
    return o_en($a, -1);
}

function o_toplam(OListe $l)
{
    $t = 0;
    foreach ($l->o as $v) {
        $t += $v;
    }
    return $t;
}

function o_anahtarlar(OSozluk $s): OListe
{
    return new OListe($s->anahtarlar());
}

function o_degerler(OSozluk $s): OListe
{
    return new OListe($s->degerler());
}

function o_ekle(OListe $l, $v): void
{
    $l->o[] = $v;
}

// Dosyalar
function o_dosya_oku(string $yol): string
{
    $m = @file_get_contents($yol);
    if ($m === false) {
        o_hata("'$yol' okunamadı: dosya bulunamadı");
    }
    return $m;
}

function o_dosyaya_yaz(string $yol, $m): void
{
    if (@file_put_contents($yol, o_metin($m)) === false) {
        o_hata("'$yol' dosyasına yazılamadı");
    }
}

function o_dosyaya_ekle(string $yol, $m): void
{
    if (@file_put_contents($yol, o_metin($m), FILE_APPEND) === false) {
        o_hata("'$yol' dosyasına yazılamadı");
    }
}

function o_dosya_var(string $yol): bool
{
    return file_exists($yol);
}

function o_dosya_sil(string $yol): bool
{
    return is_file($yol) && @unlink($yol);
}

function o_dosya_tasi(string $eski, string $yeni): bool
{
    $k = dirname($yeni);
    if (!is_dir($k)) {
        @mkdir($k, 0775, true);
    }
    return @rename($eski, $yeni);
}

// Matematik
function o_karekok($x): float
{
    if ($x < 0) {
        o_hata('negatif sayının karekökü alınamaz');
    }
    return sqrt($x);
}

function o_us($a, $b)
{
    if (is_int($a) && is_int($b)) {
        if ($b < 0) {
            o_hata('tamsayılarda üs negatif olamaz; ondalık kullanın: üs(2.0, -1)');
        }
        $r = $a ** $b;
        if (is_float($r)) {
            o_hata('tamsayı taşması: sonuç 64 bitlik sayı sınırını aştı (çok büyük değerler için ondalık kullanın)');
        }
        return $r;
    }
    if ($a < 0 && floor($b) != $b) {
        o_hata('negatif bir sayının kesirli üssü alınamaz (ör. üs(-8.0, 0.5))');
    }
    if ($a == 0 && $b < 0) {
        o_hata('sıfırın negatif üssü alınamaz (sıfıra bölme)');
    }
    return (float)($a ** $b);
}

function o_mutlak($x)
{
    return abs($x);
}

// Bit işlemleri (64 bit). Kaydırma miktarı 0..63 dışındaysa 0; sağa kaydırma mantıksaldır.
function o_bit_ve(int $a, int $b): int
{
    return $a & $b;
}

function o_bit_veya(int $a, int $b): int
{
    return $a | $b;
}

function o_bit_xor(int $a, int $b): int
{
    return $a ^ $b;
}

function o_sola_kaydir(int $a, int $n): int
{
    return $n < 0 || $n > 63 ? 0 : $a << $n;
}

function o_saga_kaydir(int $a, int $n): int
{
    if ($n < 0 || $n > 63) {
        return 0;
    }
    return $n === 0 ? $a : ($a >> $n) & (PHP_INT_MAX >> ($n - 1));
}

function o_sinus($x): float
{
    return sin($x);
}

function o_kosinus($x): float
{
    return cos($x);
}

function o_tanjant($x): float
{
    return tan($x);
}

function o_logaritma($x, $taban = null): float
{
    if ($x <= 0) {
        o_hata('logaritma yalnızca pozitif sayılar için tanımlıdır');
    }
    if ($taban === null) {
        return log($x);
    }
    if ($taban <= 0 || $taban == 1.0) {
        o_hata("logaritma tabanı pozitif ve 1'den farklı olmalı");
    }
    return log($x) / log($taban);
}

function o_rastgele($a = null, $b = null)
{
    if ($a === null) {
        return mt_rand() / (mt_getrandmax() + 1);
    }
    if ($a > $b) {
        o_hata("rastgele(a, b) için a, b'den büyük olamaz");
    }
    return random_int($a, $b);
}

// Zaman ve sistem
function o_zaman(): float
{
    return microtime(true);
}

function o_tarih(): string
{
    return date('Y-m-d H:i:s');
}

function o_bekle($sn): void
{
    usleep((int)($sn * 1e6));
}

function o_oku(): string
{
    $s = fgets(STDIN);
    return $s === false ? '' : rtrim($s, "\r\n");
}

function o_argumanlar(): OListe
{
    return new OListe(array_slice($GLOBALS['argv'] ?? [], 1));
}

function o_ortam(string $ad): string
{
    $v = getenv($ad);
    return $v === false ? '' : $v;
}

function o_cik(int $kod): never
{
    exit($kod);
}

function o_hata_ver($m): never
{
    o_hata(o_metin($m));
}

function o_bos_mu($x): bool
{
    return $x === null;
}

// Telefon, arayüz ve oyun işlevleri sunucuda etkisizdir.
function o_etkisiz(...$a): void
{
}

// ---------------------------------------------------------------------------
// JSON ve web yardımcıları
// ---------------------------------------------------------------------------

function o_json_metin(string $m): string
{
    $s = '"';
    foreach (mb_str_split($m) as $c) {
        $s .= match ($c) {
            '"' => '\\"',
            '\\' => '\\\\',
            "\n" => '\\n',
            "\r" => '\\r',
            "\t" => '\\t',
            default => mb_ord($c) < 0x20 ? sprintf('\\u%04x', mb_ord($c)) : $c,
        };
    }
    return $s . '"';
}

function o_json($x): string
{
    if (is_string($x)) {
        return o_json_metin($x);
    }
    if (is_bool($x)) {
        return $x ? 'true' : 'false';
    }
    if (is_int($x)) {
        return (string)$x;
    }
    if (is_float($x)) {
        return is_finite($x) ? o_ondalik_metni($x) : 'null';
    }
    if ($x === null) {
        return 'null';
    }
    if ($x instanceof OListe) {
        return '[' . implode(',', array_map('o_json', $x->o)) . ']';
    }
    if ($x instanceof OSozluk) {
        $p = [];
        foreach ($x->o as [$k, $v]) {
            $p[] = o_json_metin((string)$k) . ':' . o_json($v);
        }
        return '{' . implode(',', $p) . '}';
    }
    if ($x instanceof OModel) {
        $p = [];
        foreach (array_keys($x::ALANLAR) as $a) {
            if ($a === 'kimlik' && !$x->kimlik) {
                continue;
            }
            $p[] = o_json_metin($a) . ':' . o_json($x->$a);
        }
        return '{' . implode(',', $p) . '}';
    }
    return 'null';
}

function o_para($x): string
{
    $x = (float)$x;
    if (is_nan($x) || abs($x) > 9e15) {
        return o_ondalik_metni($x);
    }
    $kurus = (int)round(abs($x) * 100);
    $tam = (string)intdiv($kurus, 100);
    $s = '';
    $n = strlen($tam);
    for ($i = 0; $i < $n; $i++) {
        if ($i && ($n - $i) % 3 == 0) {
            $s .= '.';
        }
        $s .= $tam[$i];
    }
    return ($x < 0 && $kurus ? '-' : '') . $s . ',' . str_pad((string)($kurus % 100), 2, '0', STR_PAD_LEFT);
}

function o_ham($x): string
{
    return o_metin($x);
}

function o_http(string $adres, ?string $govde, string $yontem = '', array $basliklar = []): string
{
    if ($yontem === '') {
        $yontem = $govde === null ? 'GET' : 'POST';
    }
    $baslik = '';
    $tur_var = false;
    foreach ($basliklar as $ad => $d) {
        if (!preg_match('/^[A-Za-z0-9_-]+$/', (string)$ad) || preg_match('/["\\\\\r\n]/', (string)$d)) {
            o_hata("geçersiz HTTP başlığı '$ad'");
        }
        $tur_var = $tur_var || strtolower((string)$ad) === 'content-type';
        $baslik .= "$ad: $d\r\n";
    }
    if ($govde !== null && !$tur_var) {
        $baslik .= preg_match('/^[{[]/', $govde) ? "Content-Type: application/json\r\n" : "Content-Type: application/x-www-form-urlencoded\r\n";
    }
    $baglam = stream_context_create(['http' => [
        'method' => $yontem,
        'header' => $baslik,
        'content' => $govde ?? '',
        'ignore_errors' => true,
        'timeout' => 30,
    ]]);
    $m = @file_get_contents($adres, false, $baglam);
    if ($m === false) {
        o_hata("'$adres' adresine bağlanılamadı");
    }
    $durum = 0;
    foreach ($http_response_header ?? [] as $b) {
        if (preg_match('#^HTTP/\S+\s+(\d+)#', $b, $e)) {
            $durum = (int)$e[1];
        }
    }
    if ($durum >= 400) {
        o_hata("HTTP $durum: '$adres' isteği başarısız oldu");
    }
    return $m;
}

function o_http_al(string $adres): string
{
    return o_http($adres, null);
}

function o_http_gonder(string $adres, string $govde): string
{
    return o_http($adres, $govde);
}

function o_http_iste(string $yontem, string $adres, string $govde, OSozluk $sozluk): string
{
    $basliklar = array_combine($sozluk->anahtarlar(), $sozluk->degerler()) ?: [];
    $y = strtoupper($yontem);
    if (!in_array($y, ['GET', 'POST', 'PUT', 'PATCH', 'DELETE'], true)) {
        o_hata("geçersiz HTTP yöntemi '$yontem' (GET, POST, PUT, PATCH ya da DELETE olmalı)");
    }
    if (count($basliklar) > 16) {
        o_hata('en çok 16 başlık gönderilebilir');
    }
    return o_http($adres, $y === 'GET' ? null : $govde, $y, $basliklar);
}

// ---------------------------------------------------------------------------
// Seçenekler
// ---------------------------------------------------------------------------

function o_secenek(string $m, array $degerler, string $tur): string
{
    if (!in_array($m, $degerler, true)) {
        o_hata("'$m' bir $tur değeri değil (değerler: " . implode(', ', $degerler) . ')');
    }
    return $m;
}

// ---------------------------------------------------------------------------
// Modeller: doğrulama, form bağlama, JSON ve kalıcı kayıt
// ---------------------------------------------------------------------------

/**
 * Bütün modellerin (İstek, Yanıt dahil) atası. Program her model için bir sınıf tanımlar:
 * AD (Orhunca'daki adı), ALANLAR (alan adı → tip ve kurallar) ve yeni() (varsayılanlarla).
 * Tip tanımı: 's' sayı, 'o' ondalık, 'm' metin, 'b' mantık, ['e', [değerler]] seçenek,
 * ['l', tip] liste, ['d', anahtar tipi, değer tipi] sözlük, ['M', sınıf] model.
 */
abstract class OModel
{
    /** Formdan bağlanamayan alanların hataları: alan adı → mesaj */
    public array $ohc_baglama = [];
}

function o_sinif_adi(OModel $n): string
{
    return $n::AD;
}

/** Hata mesajlarındaki ad: etiket ya da "doğum_tarihi" → "Doğum tarihi" */
function o_gorunen_ad(string $ad, array $a): string
{
    if (isset($a['etiket'])) {
        return $a['etiket'];
    }
    $m = str_replace('_', ' ', $ad);
    $ilk = mb_substr($m, 0, 1);
    $ilk = match ($ilk) {
        'i' => 'İ',
        'ı' => 'I',
        default => mb_strtoupper($ilk),
    };
    return $ilk . mb_substr($m, 1);
}

/** 100 → "100", 0.5 → "0,5" */
function o_kural_sayisi($x): string
{
    if ($x == floor($x) && abs($x) < 1e15) {
        return (string)(int)$x;
    }
    return str_replace('.', ',', o_ondalik_metni((float)$x));
}

function o_e_posta_mi(string $s): bool
{
    $s = ltrim($s);
    $at = strpos($s, '@');
    if ($at === false || $at === 0 || strpos($s, '@', $at + 1) !== false) {
        return false;
    }
    $nokta = strrpos($s, '.');
    if ($nokta === false || $nokta < $at || $nokta == $at + 1 || $nokta == strlen($s) - 1) {
        return false;
    }
    return !preg_match('/[ \t<>,]\S/', $s);
}

function o_model_hatalar(?OModel $n): OListe
{
    $sonuc = new OListe();
    if (!$n) {
        return $sonuc;
    }
    foreach ($n::ALANLAR as $ad => $a) {
        if ($ad === 'kimlik') {
            continue;
        }
        if (isset($n->ohc_baglama[$ad])) {
            $sonuc->o[] = $n->ohc_baglama[$ad];
            continue;
        }
        $d = $n->$ad;
        $t = $a['t'];
        $g = fn() => o_gorunen_ad($ad, $a);
        $m = null;
        $tur = is_array($t) ? $t[0] : $t;
        if ($tur === 'm' || $tur === 'e') {
            $uz = mb_strlen($d);
            if ($tur === 'e' && !in_array($d, $t[1], true)) {
                $m = $g() . ' şunlardan biri olmalı: ' . implode(', ', $t[1]);
            } elseif (trim($d) === '') {
                if (!empty($a['zorunlu'])) {
                    $m = $g() . ' boş bırakılamaz';
                }
            } elseif (!empty($a['e_posta']) && !o_e_posta_mi($d)) {
                $m = $g() . ' geçerli bir e-posta adresi olmalı';
            } elseif (isset($a['en_az']) && $uz < $a['en_az']) {
                $m = $g() . ' en az ' . o_kural_sayisi($a['en_az']) . ' karakter olmalı';
            } elseif (isset($a['en_fazla']) && $uz > $a['en_fazla']) {
                $m = $g() . ' en fazla ' . o_kural_sayisi($a['en_fazla']) . ' karakter olabilir';
            }
        } elseif ($tur === 's' || $tur === 'o') {
            if (!is_finite((float)$d)) {
                $m = $g() . ' geçerli bir sayı olmalı';
            } elseif (isset($a['en_az']) && $d < $a['en_az']) {
                $m = $g() . ' en az ' . o_kural_sayisi($a['en_az']) . ' olmalı';
            } elseif (isset($a['en_fazla']) && $d > $a['en_fazla']) {
                $m = $g() . ' en fazla ' . o_kural_sayisi($a['en_fazla']) . ' olabilir';
            }
        } elseif ($tur === 'b') {
            if (!empty($a['zorunlu']) && !$d) {
                $m = $g() . ' işaretlenmeli';
            }
        } elseif ($tur === 'M') {
            if ($d === null) {
                if (!empty($a['zorunlu'])) {
                    $m = $g() . ' boş bırakılamaz';
                }
            } else {
                foreach (o_model_hatalar($d)->o as $h) {
                    $sonuc->o[] = $g() . ': ' . $h;
                }
            }
        } elseif ($tur === 'l' || $tur === 'd') {
            $uz = count($d);
            if ($tur === 'l' && is_array($t[1]) && $t[1][0] === 'M') {
                foreach ($d->o as $i => $ic) {
                    foreach (o_model_hatalar($ic)->o as $h) {
                        $sonuc->o[] = $g() . ' ' . ($i + 1) . ': ' . $h;
                    }
                }
            }
            if (!empty($a['zorunlu']) && $uz == 0) {
                $m = $g() . ' boş bırakılamaz';
            } elseif (isset($a['en_az']) && $uz < $a['en_az']) {
                $m = $g() . ' en az ' . o_kural_sayisi($a['en_az']) . ' öğe içermeli';
            } elseif (isset($a['en_fazla']) && $uz > $a['en_fazla']) {
                $m = $g() . ' en fazla ' . o_kural_sayisi($a['en_fazla']) . ' öğe içerebilir';
            }
        }
        if ($m !== null) {
            $sonuc->o[] = $m;
        }
    }
    return $sonuc;
}

function o_model_gecerli_mi(?OModel $n): bool
{
    return count(o_model_hatalar($n)) == 0;
}

function o_dogru_mu(string $v): bool
{
    return in_array($v, ['on', 'true', 'doğru', '1', 'evet', 'yes', 'Doğru', 'Evet', 'True', 'On'], true);
}

/** Model.formdan(istek): formdaki değerler alanlara yazılır; çevrilemeyenler hata olur. */
function o_model_formdan(string $sinif, $istek)
{
    $n = $sinif::yeni();
    $form = $istek->form;
    foreach ($sinif::ALANLAR as $ad => $a) {
        if ($ad === 'kimlik' || !isset($form[$ad])) {
            continue;
        }
        $v = $form[$ad];
        $bos = trim($v) === '';
        $t = is_array($a['t']) ? $a['t'][0] : $a['t'];
        $m = null;
        if ($t === 'm' || $t === 'e') {
            $n->$ad = $v;
        } elseif ($t === 's') {
            if ($bos) {
                $m = empty($a['zorunlu']) ? null : ' boş bırakılamaz';
            } elseif (o_sayi_mi($v)) {
                $n->$ad = o_sayi($v);
            } else {
                $m = ' bir tam sayı olmalı';
            }
        } elseif ($t === 'o') {
            if ($bos) {
                $m = empty($a['zorunlu']) ? null : ' boş bırakılamaz';
            } elseif (o_ondalik_mi($v)) {
                $n->$ad = o_ondalik($v);
            } else {
                $m = ' bir sayı olmalı';
            }
        } elseif ($t === 'b') {
            $n->$ad = o_dogru_mu(trim($v));
        }
        if ($m !== null) {
            $n->ohc_baglama[$ad] = o_gorunen_ad($ad, $a) . $m;
        }
    }
    return $n;
}

/** JSON'dan (ya da veritabanından) gelen değeri tipine çevirir. */
function o_coz($v, $t)
{
    if ($v === null) {
        return null;
    }
    $tur = is_array($t) ? $t[0] : $t;
    switch ($tur) {
        case 's':
            return (int)$v;
        case 'o':
            return (float)$v;
        case 'b':
            return (bool)$v;
        case 'm':
        case 'e':
            return (string)$v;
        case 'l':
            return new OListe(array_map(fn($x) => o_coz($x, $t[1]), (array)$v));
        case 'd':
            $s = new OSozluk();
            foreach ((array)$v as $k => $x) {
                $k = $t[1] === 's' ? (int)$k : (string)$k;
                $s->o[OSozluk::ic($k)] = [$k, o_coz($x, $t[2])];
            }
            return $s;
        case 'M':
            $sinif = $t[1];
            $n = $sinif::yeni();
            foreach ($sinif::ALANLAR as $ad => $a) {
                if (array_key_exists($ad, (array)$v)) {
                    $n->$ad = o_coz($v[$ad], $a['t']);
                }
            }
            return $n;
    }
    return $v;
}

// Veritabanı: ayarlar.php'de MySQL bilgileri verilirse MySQL, yoksa veri/orhunca.sqlite.
final class OVT
{
    public static ?PDO $pdo = null;
    public static bool $mysql = false;
    public static array $hazir = [];
}

function o_vt(): PDO
{
    if (OVT::$pdo) {
        return OVT::$pdo;
    }
    $a = is_file(OHC_KOK . '/orhunca/ayarlar.php') ? (require OHC_KOK . '/orhunca/ayarlar.php') : [];
    try {
        if (!empty($a['veritabani'])) {
            OVT::$mysql = true;
            OVT::$pdo = new PDO(
                'mysql:host=' . ($a['sunucu'] ?? 'localhost') . ';dbname=' . $a['veritabani'] . ';charset=utf8mb4',
                $a['kullanici'] ?? '',
                $a['sifre'] ?? '',
                [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION]
            );
        } else {
            if (!is_dir(OHC_KOK . '/veri')) {
                @mkdir(OHC_KOK . '/veri', 0775, true);
            }
            OVT::$pdo = new PDO('sqlite:' . OHC_KOK . '/veri/orhunca.sqlite', null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION]);
        }
    } catch (PDOException $e) {
        o_hata('veritabanına bağlanılamadı (orhunca/ayarlar.php): ' . $e->getMessage());
    }
    return OVT::$pdo;
}

function o_ad_tirnakla(string $ad): string
{
    return OVT::$mysql ? '`' . str_replace('`', '``', $ad) . '`' : '"' . str_replace('"', '""', $ad) . '"';
}

function o_sutun_tipi($t): string
{
    $tur = is_array($t) ? $t[0] : $t;
    if (OVT::$mysql) {
        return match ($tur) {
            's' => 'BIGINT NOT NULL DEFAULT 0',
            'o' => 'DOUBLE NOT NULL DEFAULT 0',
            'b' => 'TINYINT(1) NOT NULL DEFAULT 0',
            'e' => 'VARCHAR(255)',
            default => 'LONGTEXT',
        };
    }
    return match ($tur) {
        's', 'b' => 'INTEGER NOT NULL DEFAULT 0',
        'o' => 'REAL NOT NULL DEFAULT 0',
        default => 'TEXT',
    };
}

/** Modelin tablosu: yoksa oluşturulur, eksik sütunlar eklenir; veri/<Model>.json içe aktarılır. */
function o_tablo(string $sinif): string
{
    $ad = $sinif::AD;
    $q = o_ad_tirnakla($ad);
    if (isset(OVT::$hazir[$ad])) {
        return $q;
    }
    $vt = o_vt();
    $sutunlar = [];
    foreach ($sinif::ALANLAR as $a => $tanim) {
        if ($a !== 'kimlik') {
            $sutunlar[] = o_ad_tirnakla($a) . ' ' . o_sutun_tipi($tanim['t']);
        }
    }
    $kimlik = OVT::$mysql ? '`kimlik` BIGINT AUTO_INCREMENT PRIMARY KEY' : '"kimlik" INTEGER PRIMARY KEY AUTOINCREMENT';
    $yeni = !o_tablo_var($q, $ad);
    $vt->exec("CREATE TABLE IF NOT EXISTS $q ($kimlik" . ($sutunlar ? ', ' . implode(', ', $sutunlar) : '') . ')'
        . (OVT::$mysql ? ' DEFAULT CHARSET=utf8mb4' : ''));
    if (!$yeni) {
        $var = OVT::$mysql
            ? array_column($vt->query("SHOW COLUMNS FROM $q")->fetchAll(PDO::FETCH_ASSOC), 'Field')
            : array_column($vt->query("PRAGMA table_info($q)")->fetchAll(PDO::FETCH_ASSOC), 'name');
        foreach ($sinif::ALANLAR as $a => $tanim) {
            if (!in_array($a, $var, true)) {
                $vt->exec("ALTER TABLE $q ADD COLUMN " . o_ad_tirnakla($a) . ' ' . o_sutun_tipi($tanim['t']));
            }
        }
    }
    OVT::$hazir[$ad] = true;
    // Orhunca'nın kendi sunucusundan taşınan kayıtlar: veri/<Model>.json
    $dosya = OHC_KOK . '/veri/' . $ad . '.json';
    if ($yeni && is_file($dosya)) {
        $kayitlar = json_decode(file_get_contents($dosya), true) ?: [];
        foreach ($kayitlar as $k) {
            $n = o_coz($k, ['M', $sinif]);
            $n->kimlik = (int)($k['kimlik'] ?? 0);
            o_ekle_kayit($n, true);
        }
    }
    return $q;
}

function o_tablo_var(string $q, string $ad): bool
{
    $vt = o_vt();
    if (OVT::$mysql) {
        return (bool)$vt->query('SHOW TABLES LIKE ' . $vt->quote($ad))->fetch();
    }
    return (bool)$vt->query("SELECT name FROM sqlite_master WHERE type='table' AND name=" . $vt->quote($ad))->fetch();
}

function o_sutun_degeri($v, $t)
{
    $tur = is_array($t) ? $t[0] : $t;
    return match ($tur) {
        's', 'o' => $v,
        'b' => $v ? 1 : 0,
        'm', 'e' => $v,
        default => $v === null ? null : o_json($v),
    };
}

function o_satirdan(string $sinif, array $r): OModel
{
    $n = $sinif::yeni();
    foreach ($sinif::ALANLAR as $a => $tanim) {
        if (!array_key_exists($a, $r)) {
            continue;
        }
        $v = $r[$a];
        $tur = is_array($tanim['t']) ? $tanim['t'][0] : $tanim['t'];
        if (in_array($tur, ['l', 'd', 'M'], true)) {
            $v = $v === null ? null : json_decode($v, true);
        }
        $n->$a = o_coz($v, $tanim['t']);
    }
    return $n;
}

function o_ekle_kayit(OModel $n, bool $kimlikle): int
{
    $sinif = get_class($n);
    $q = o_tablo($sinif);
    $adlar = [];
    $degerler = [];
    foreach ($sinif::ALANLAR as $a => $tanim) {
        if ($a === 'kimlik' && !$kimlikle) {
            continue;
        }
        $adlar[] = o_ad_tirnakla($a);
        $degerler[] = $a === 'kimlik' ? $n->kimlik : o_sutun_degeri($n->$a, $tanim['t']);
    }
    $s = o_vt()->prepare("INSERT INTO $q (" . implode(', ', $adlar) . ') VALUES (' . implode(', ', array_fill(0, count($adlar), '?')) . ')');
    $s->execute($degerler);
    return $kimlikle ? $n->kimlik : (int)o_vt()->lastInsertId();
}

function o_model_kaydet(?OModel $n): int
{
    if (!$n) {
        o_hata('boş bir model değeri kaydedilemez');
    }
    $sinif = get_class($n);
    $q = o_tablo($sinif);
    if ($n->kimlik > 0 && o_model_var_mi($sinif, $n->kimlik)) {
        $parcalar = [];
        $degerler = [];
        foreach ($sinif::ALANLAR as $a => $tanim) {
            if ($a !== 'kimlik') {
                $parcalar[] = o_ad_tirnakla($a) . ' = ?';
                $degerler[] = o_sutun_degeri($n->$a, $tanim['t']);
            }
        }
        $degerler[] = $n->kimlik;
        if ($parcalar) {
            o_vt()->prepare("UPDATE $q SET " . implode(', ', $parcalar) . ' WHERE ' . o_ad_tirnakla('kimlik') . ' = ?')->execute($degerler);
        }
        return $n->kimlik;
    }
    $n->kimlik = $n->kimlik > 0 ? o_ekle_kayit($n, true) : o_ekle_kayit($n, false);
    return $n->kimlik;
}

function o_model_hepsi(string $sinif): OListe
{
    $q = o_tablo($sinif);
    $l = new OListe();
    foreach (o_vt()->query("SELECT * FROM $q ORDER BY " . o_ad_tirnakla('kimlik'))->fetchAll(PDO::FETCH_ASSOC) as $r) {
        $l->o[] = o_satirdan($sinif, $r);
    }
    return $l;
}

function o_model_bul(string $sinif, int $kimlik): OModel
{
    $q = o_tablo($sinif);
    $s = o_vt()->prepare("SELECT * FROM $q WHERE " . o_ad_tirnakla('kimlik') . ' = ?');
    $s->execute([$kimlik]);
    $r = $s->fetch(PDO::FETCH_ASSOC);
    return $r ? o_satirdan($sinif, $r) : $sinif::yeni();
}

function o_model_var_mi(string $sinif, int $kimlik): bool
{
    $q = o_tablo($sinif);
    $s = o_vt()->prepare("SELECT 1 FROM $q WHERE " . o_ad_tirnakla('kimlik') . ' = ?');
    $s->execute([$kimlik]);
    return (bool)$s->fetch();
}

function o_model_sil_kimlik(string $sinif, int $kimlik): bool
{
    $q = o_tablo($sinif);
    $s = o_vt()->prepare("DELETE FROM $q WHERE " . o_ad_tirnakla('kimlik') . ' = ?');
    $s->execute([$kimlik]);
    return $s->rowCount() > 0;
}

/** Ham SQL: modellerin tabloları önce hazırlanır; `?` yerlerine değerler bağlanır. */
function o_sql_hazir(string $sorgu, ?OListe $degerler)
{
    foreach (get_declared_classes() as $c) {
        if (is_subclass_of($c, 'OModel') && defined("$c::AD")) {
            o_tablo($c);
        }
    }
    try {
        $s = o_vt()->prepare($sorgu);
        $s->execute($degerler ? array_map('strval', $degerler->o) : []);
    } catch (PDOException $e) {
        o_hata('veritabanı hatası (SQL): ' . ($e->errorInfo[2] ?? $e->getMessage()));
    }
    return $s;
}

function o_sql_sorgu(string $sorgu, ?OListe $degerler = null): OListe
{
    $s = o_sql_hazir($sorgu, $degerler);
    $l = new OListe();
    foreach ($s->fetchAll(PDO::FETCH_ASSOC) as $r) {
        $ciftler = [];
        foreach ($r as $k => $v) {
            $ciftler[] = [(string)$k, $v === null ? '' : (string)$v];
        }
        $l->o[] = OSozluk::yap($ciftler);
    }
    return $l;
}

function o_sql_calistir(string $sorgu, ?OListe $degerler = null): int
{
    return o_sql_hazir($sorgu, $degerler)->rowCount();
}

function o_model_sil(?OModel $n): bool
{
    if (!$n) {
        o_hata('boş bir model değeri silinemez');
    }
    return o_model_sil_kimlik(get_class($n), $n->kimlik);
}

// ---------------------------------------------------------------------------
// Web: yollar, istek, yanıt, oturum, hata sayfaları
// ---------------------------------------------------------------------------

final class OWeb
{
    /** [yöntem, kalıp parçaları, işlev] */
    public static array $yollar = [];
    public static bool $web = false;
}

function o_yol(string $yontem, string $kalip, string $islev): void
{
    OWeb::$yollar[] = [$yontem, array_values(array_filter(explode('/', $kalip), fn($p) => $p !== '')), $islev];
}

/** Programın başında: web programıysa ana programın çıktısı sayfaya karışmaz. */
function o_baslat(bool $web): void
{
    OWeb::$web = $web && PHP_SAPI !== 'cli';
    if (OWeb::$web) {
        ob_start();
    }
    set_exception_handler('o_yakalanmayan');
}

function o_html_kacir($m): string
{
    return htmlspecialchars((string)$m, ENT_QUOTES, 'UTF-8');
}

function o_sayfa(int $durum, string $baslik, ?string $ayrinti = null): void
{
    // Yerel sunucunun hata sayfasıyla aynı biçim
    $k = fn ($m) => strtr((string)$m, ['&' => '&amp;', '<' => '&lt;', '>' => '&gt;', '"' => '&quot;', "'" => '&#39;']);
    http_response_code($durum);
    header('Content-Type: text/html; charset=utf-8');
    echo '<!DOCTYPE html><html lang="tr"><head><meta charset="utf-8">'
        . '<meta name="viewport" content="width=device-width, initial-scale=1"><title>' . $k($baslik)
        . '</title><style>body{font-family:system-ui,sans-serif;margin:0;padding:48px 24px;'
        . 'background:#f5f3ee;color:#1b1d21}main{max-width:720px;margin:auto}h1{font-size:22px;margin:0 0 12px}'
        . 'pre{white-space:pre-wrap;background:#1b1d21;color:#f5f3ee;padding:16px;border-radius:8px;'
        . 'font-size:13px}small{color:#6b7078}</style></head><body><main><h1>' . $k($baslik) . '</h1>'
        . ($ayrinti !== null && $ayrinti !== '' ? '<pre>' . $k($ayrinti) . '</pre>' : '')
        . '<small>Orhunca web sunucusu</small></main></body></html>';
}

/** Hatanın Orhunca kaynağındaki satırı (programın satır tablosundan) */
function o_hata_satiri(Throwable $e): ?int
{
    if ($e instanceof OHata && $e->ohc_satir) {
        return $e->ohc_satir;
    }
    $tablo = function_exists('ohc_satirlar') ? ohc_satirlar() : [];
    $yerler = array_merge([['file' => $e->getFile(), 'line' => $e->getLine()]], $e->getTrace());
    foreach ($yerler as $y) {
        if (($y['file'] ?? '') === OHC_PROGRAM && isset($tablo[$y['line']])) {
            return $tablo[$y['line']];
        }
    }
    return null;
}

function o_hata_metni(Throwable $e): string
{
    $m = $e instanceof OHata ? $e->getMessage() : o_php_hatasi($e);
    $s = o_hata_satiri($e);
    return $s ? "Çalışma hatası (satır $s): $m" : "Çalışma hatası: $m";
}

/** PHP'nin kendi hataları (Orhunca karşılığı varsa) */
function o_php_hatasi(Throwable $e): string
{
    if ($e instanceof DivisionByZeroError) {
        return 'sıfıra bölme';
    }
    if ($e instanceof ArithmeticError) {
        return 'tamsayı taşması';
    }
    return $e->getMessage();
}

/** `dene: ... yakala hata:` → hata mesajı */
function o_yakalanan(Throwable $e): string
{
    return $e instanceof OHata ? $e->getMessage() : o_php_hatasi($e);
}

function o_yakalanmayan(Throwable $e): void
{
    $m = o_hata_metni($e);
    if (PHP_SAPI === 'cli') {
        fwrite(STDERR, $m . "\n");
        exit(1);
    }
    while (ob_get_level()) {
        ob_end_clean();
    }
    error_log('Orhunca: ' . $m);
    o_sayfa(500, 'Sunucu hatası', $m);
    exit;
}

function o_temel_yol(): string
{
    $b = rtrim(str_replace('\\', '/', dirname($_SERVER['SCRIPT_NAME'] ?? '/')), '/');
    return $b === '.' ? '' : $b;
}

function o_dize_sozluk(array $a): OSozluk
{
    $s = new OSozluk();
    foreach ($a as $k => $v) {
        if (is_array($v)) {
            $v = reset($v);
        }
        $k = (string)$k;
        $s->o[OSozluk::ic($k)] = [$k, is_string($v) ? $v : o_metin($v)];
    }
    return $s;
}

function o_istek_yap(string $yol, array $parametreler)
{
    $i = İstek::yeni();
    $y = strtoupper($_SERVER['REQUEST_METHOD'] ?? 'GET');
    $i->yöntem = $y === 'HEAD' ? 'GET' : $y;
    $i->yol = $yol;
    $i->sorgu = o_dize_sozluk($_GET);
    $i->gövde = (string)file_get_contents('php://input');
    $form = $_POST;
    if (!$form && str_contains($_SERVER['CONTENT_TYPE'] ?? '', 'json')) {
        $j = json_decode($i->gövde, true);
        if (is_array($j)) {
            $form = array_map(fn($v) => is_string($v) ? $v : (is_bool($v) ? ($v ? 'doğru' : 'yanlış') : (is_scalar($v) ? (string)$v : json_encode($v, JSON_UNESCAPED_UNICODE))), $j);
        }
    }
    $i->form = o_dize_sozluk($form);
    $i->parametreler = o_dize_sozluk($parametreler);
    $basliklar = [];
    foreach ($_SERVER as $k => $v) {
        if (str_starts_with($k, 'HTTP_')) {
            $basliklar[strtolower(str_replace('_', '-', substr($k, 5)))] = $v;
        } elseif (in_array($k, ['CONTENT_TYPE', 'CONTENT_LENGTH'], true)) {
            $basliklar[strtolower(str_replace('_', '-', $k))] = $v;
        }
    }
    $i->başlıklar = o_dize_sozluk($basliklar);
    $i->çerezler = o_dize_sozluk($_COOKIE);
    if (session_status() === PHP_SESSION_NONE) {
        session_name('orhunca_oturum');
    }
    if (isset($_COOKIE[session_name()]) && session_status() === PHP_SESSION_NONE) {
        o_oturum_ac();
    }
    $i->oturum = o_dize_sozluk(session_status() === PHP_SESSION_ACTIVE ? ($_SESSION['ohc'] ?? []) : []);
    $dosyalar = new OSozluk();
    foreach ($_FILES as $alan => $f) {
        if (!is_string($f['name'] ?? null) || ($f['error'] ?? 1) !== UPLOAD_ERR_OK) {
            continue;
        }
        $klasor = OHC_KOK . '/veri/yüklemeler';
        if (!is_dir($klasor)) {
            @mkdir($klasor, 0775, true);
        }
        $ad = preg_replace('/[\/\\\\:\x00]/u', '_', basename($f['name']));
        $hedef = $klasor . '/' . bin2hex(random_bytes(4)) . '_' . $ad;
        move_uploaded_file($f['tmp_name'], $hedef);
        $d = YüklenenDosya::yeni();
        $d->ad = $f['name'];
        $d->tür = (string)($f['type'] ?? '');
        $d->yol = 'veri/yüklemeler/' . basename($hedef);
        $d->boyut = (int)$f['size'];
        $dosyalar->o[OSozluk::ic($alan)] = [$alan, $d];
    }
    $i->dosyalar = $dosyalar;
    return $i;
}

function o_oturum_ac(): void
{
    session_set_cookie_params([
        'path' => o_temel_yol() . '/',
        'httponly' => true,
        'samesite' => 'Lax',
        'secure' => !empty($_SERVER['HTTPS']) && $_SERVER['HTTPS'] !== 'off',
    ]);
    session_start();
}

function o_oturum_yaz(OSozluk $o): void
{
    $veri = [];
    foreach ($o->o as [$k, $v]) {
        $veri[(string)$k] = o_metin($v);
    }
    if (!$veri) {
        if (session_status() === PHP_SESSION_ACTIVE) {
            $_SESSION = [];
            session_destroy();
            o_cerez(session_name(), '', true);
        }
        return;
    }
    if (session_status() === PHP_SESSION_NONE) {
        o_oturum_ac();
    }
    $_SESSION['ohc'] = $veri;
}

/** Yol kalıbı eşleşirse parametreler, eşleşmezse null; sabit parça sayısı da döner. */
function o_eslestir(array $kalip, array $parcalar): ?array
{
    if (count($kalip) != count($parcalar)) {
        return null;
    }
    $p = [];
    $sabit = 0;
    foreach ($kalip as $i => $k) {
        if (preg_match('/^\{([^:}]+)(?::([^}]+))?\}$/u', $k, $m)) {
            $deger = $parcalar[$i];
            if (($m[2] ?? '') === 'sayı' && !preg_match('/^-?\d+$/', $deger)) {
                return null;
            }
            $p[$m[1]] = $deger;
        } elseif ($k === $parcalar[$i]) {
            $sabit++;
        } else {
            return null;
        }
    }
    return [$p, $sabit];
}

/** Programın sonunda: isteği uygun yola gönderir ve yanıtı yazar. */
function o_sun(...$a): void
{
    if (PHP_SAPI === 'cli') {
        fwrite(STDERR, "Bu bir web programı: PHP sunucusunda çalışır (php -S localhost:3000 index.php).\n");
        return;
    }
    while (ob_get_level()) {
        ob_end_clean();
    }
    $adres = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH) ?: '/';
    $taban = o_temel_yol();
    if ($taban !== '' && str_starts_with($adres, $taban)) {
        $adres = substr($adres, strlen($taban));
    }
    $yol = '/' . ltrim(rawurldecode($adres), '/');
    $parcalar = array_values(array_filter(explode('/', $yol), fn($p) => $p !== ''));
    $yontem = strtoupper($_SERVER['REQUEST_METHOD'] ?? 'GET');
    if ($yontem === 'HEAD') {
        $yontem = 'GET';
    }
    // Yerel sunucu gibi: klasör adreslerinde statik/ içindeki index.html yollardan önce gelir.
    if ($yontem === 'GET' && !array_intersect($parcalar, ['..', 'orhunca', 'veri'])) {
        $dizin = dirname(OHC_PROGRAM) . rtrim($yol, '/') . '/index.html';
        if (is_file($dizin)) {
            header('Content-Type: text/html; charset=utf-8');
            readfile($dizin);
            return;
        }
    }
    $secilen = null;
    $en_iyi = -1;
    $yontem_farkli = false;
    foreach (OWeb::$yollar as [$y, $kalip, $islev]) {
        $e = o_eslestir($kalip, $parcalar);
        if ($e === null) {
            continue;
        }
        if ($y !== $yontem) {
            $yontem_farkli = true;
            continue;
        }
        if ($e[1] > $en_iyi) {
            $en_iyi = $e[1];
            $secilen = [$islev, $e[0]];
        }
    }
    if (!$secilen) {
        o_sayfa($yontem_farkli ? 405 : 404, $yontem_farkli ? 'Bu adres bu yöntemle kullanılamaz' : 'Sayfa bulunamadı', "$yontem $yol");
        return;
    }
    $istek = o_istek_yap($yol, $secilen[1]);
    ob_start();
    $y = ($secilen[0])($istek);
    ob_end_clean();
    o_oturum_yaz($istek->oturum);
    if ($y === null) {
        http_response_code(204);
        ini_set('default_mimetype', '');
        header_remove('Content-Type');
        return;
    }
    o_yanit_gonder($y);
}

function o_yanit_gonder($y): void
{
    http_response_code($y->durum);
    header('Content-Type: ' . $y->tür);
    if ($y->konum !== '') {
        $k = $y->konum;
        if (str_starts_with($k, '/') && !str_starts_with($k, '//')) {
            $k = o_temel_yol() . $k;
        }
        // ASCII dışı karakterler yüzde kodlanır (Orhunca'nın sunucusu gibi)
        header('Location: ' . preg_replace_callback('/[^\x21-\x7e]+/', fn($e) => rawurlencode($e[0]), $k));
    }
    foreach ($y->başlıklar->o as [$k, $v]) {
        header($k . ': ' . o_metin($v));
    }
    foreach ($y->çerezler->o as [$k, $v]) {
        o_cerez((string)$k, o_metin($v), false);
    }
    echo $y->gövde;
}

/** Yerel sunucudaki gibi çerez başlığı (boş değer çerezi siler) */
function o_cerez(string $ad, string $deger, bool $http_only): void
{
    $kodla = fn ($m) => preg_replace_callback('/[^A-Za-z0-9\-_.~]/', fn ($e) => sprintf('%%%02X', ord($e[0])), $m);
    $c = $kodla($ad) . '=' . $kodla($deger) . '; Path=' . o_temel_yol() . '/; SameSite=Lax';
    if ($deger === '') {
        $c .= '; Max-Age=0';
    }
    if ($http_only) {
        $c .= '; HttpOnly';
    }
    if (!empty($_SERVER['HTTPS']) && $_SERVER['HTTPS'] !== 'off') {
        $c .= '; Secure';
    }
    header('Set-Cookie: ' . $c, false);
}
