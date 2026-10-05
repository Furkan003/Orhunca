# Yapay zekâ ile Orhunca

Orhunca'yı yapay zekâ ile iki yoldan kullanabilirsiniz:

1. **Stüdyo asistanı.** Stüdyo'nun içindeki sohbet paneli, kendi Anthropic API anahtarınızla çalışır.
2. **Kendi ajanınız.** Claude Code, Claude Desktop, Cursor ya da VS Code gibi bir araç, `orhunca mcp`
   sunucusuna bağlanıp Orhunca kodu yazabilir, denetleyebilir ve çalıştırabilir.

[← README](../README.md)

## Stüdyo asistanı

Sağ üstteki **Asistan** düğmesine tıklayın (ya da <kbd>Ctrl+I</kbd>); asistan, Cursor'daki gibi
sağda bir sohbet paneli olarak açılır. Panelin genişliği sol kenarından sürüklenerek ayarlanır.
İlk açılışta Anthropic API anahtarınızı girin. Anahtarı
[console.anthropic.com](https://console.anthropic.com) adresinden alabilirsiniz.

- **Model seçimi.** Hesabınızın kullanabildiği modeller listelenir; ilk kurulumda en yenisi seçilir.
  Modeli istediğiniz zaman değiştirebilirsiniz.
- **Açık dosyanız** sorunuzla birlikte gönderilir. Asistan yazdığı kodu önce kendisi denetler ve
  çalıştırır, çıktısına bakar, hata varsa düzeltir.
- **Dosya değişiklikleri.** Asistan dosyanızı kendisi değiştirmez; yeni içeriği öneri olarak
  gösterir. **Uygula** düğmesine bastığınızda değişiklik yazılır. <kbd>Ctrl+Z</kbd> ile geri alabilirsiniz.
- **Öğrenciler için.** Asistan Türkçe ve sade anlatır. Bir alıştırmayı çözmeye çalışan öğrenciye
  cevabı hemen vermez, ipucuyla yol gösterir.
- **Anahtar ve ücret.** Anahtar yalnızca bu bilgisayarda saklanır (`~/.config/orhunca/asistan.json`,
  Windows'ta `%APPDATA%\Orhunca\asistan.json`) ve Stüdyo arayüzüne bile geri gönderilmez.
  Kullanım ücreti kendi hesabınıza yansır.

### Okullar için: asistanı kapatmak

Sınav ya da ders ortamında asistanı tamamen kapatmak için Stüdyo'yu şu ortam değişkeniyle başlatın:

```sh
ORHUNCA_YAPAY_ZEKA=kapali orhunca stüdyo
```

Windows'ta bu değişkeni bütün kullanıcılar için sistem ortam değişkeni olarak tanımlayın. Böylece
asistan simgesi görünmez ve anahtar girilemez. Kullanıcılar asistanı
**Ayarlar → Yapay zekâ asistanı** seçeneğiyle yalnızca gizleyebilir.

## Kendi ajanınızı bağlamak (MCP)

`orhunca mcp` komutu bir [Model Context Protocol](https://modelcontextprotocol.io) sunucusu
başlatır. Sunucu stdin/stdout üzerinden konuşur. Sunduğu araçlar:

| Araç | Ne yapar |
|---|---|
| `orhunca_rehber` | Dil rehberini döndürür: söz dizimi, hâl ekleri, standart kütüphane, arayüz dili |
| `orhunca_denetle` | Kodu derler; Türkçe hata mesajlarını, satır numarasını ve ipucunu ya da "Hata yok."u döndürür |
| `orhunca_calistir` | Programı süre sınırıyla çalıştırır; girdi verilebilir ve çıktı döndürülür |
| `orhunca_bicimlendir` | Kodu standart biçime getirir |

Araçlara kod metin olarak (`kod`) ya da diskteki bir dosyanın yoluyla (`dosya`) verilebilir. Rehber,
`orhunca://rehber` kaynağı olarak da sunulur.

**Claude Code**

```sh
claude mcp add orhunca -- orhunca mcp
```

**Claude Desktop** (`claude_desktop_config.json`) ve **Cursor** (`.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "orhunca": { "command": "orhunca", "args": ["mcp"] }
  }
}
```

**VS Code** (`.vscode/mcp.json`)

```json
{
  "servers": {
    "orhunca": { "type": "stdio", "command": "orhunca", "args": ["mcp"] }
  }
}
```

Bağlantıdan sonra ajana örneğin şunu yazabilirsiniz: *"Orhunca ile bir not ortalaması programı
yaz ve çalıştırıp dene."* Ajan önce rehberi okur, sonra kodu yazar, denetler ve çalıştırır.

Ajanınız terminal kullanabiliyorsa MCP olmadan da çalışabilir: `orhunca denetle dosya.ohc` ve
`orhunca çalıştır dosya.ohc` komutları aynı Türkçe hata mesajlarını verir. Dil rehberinin
tamamı tek dosya olarak [llms-full.txt](https://furkan003.github.io/Orhunca/llms-full.txt)
adresinde de bulunur.
