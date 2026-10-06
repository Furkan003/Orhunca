# Yapay zekâ ile Orhunca

Orhunca'yı yapay zekâ ile iki yoldan kullanabilirsiniz:

1. **Stüdyo asistanı.** Stüdyo'nun sağındaki sohbet paneli. Claude, GPT, Gemini gibi bulut
   modelleriyle ya da bilgisayarınızda çalışan **yerel modellerle** (Ollama, LM Studio, llama.cpp…)
   çalışır.
2. **Kendi ajanınız.** Claude Code, Codex, Gemini CLI, Cursor, VS Code (Copilot), Windsurf, Cline,
   Continue, Zed gibi araçlar `orhunca mcp` sunucusuna bağlanıp Orhunca kodu yazabilir, denetleyebilir
   ve çalıştırabilir.

[← README](../README.md)

## Stüdyo asistanı

Sağ üstteki **Asistan** düğmesine tıklayın (ya da <kbd>Ctrl+I</kbd>); asistan, Cursor'daki gibi
sağda bir sohbet paneli olarak açılır. Panelin genişliği sol kenarından sürüklenerek ayarlanır.
İlk açılışta bir sağlayıcı seçin.

### Sağlayıcılar

| Sağlayıcı | Gereken | Not |
|---|---|---|
| Anthropic (Claude) | API anahtarı ([console.anthropic.com](https://console.anthropic.com)) | |
| OpenAI (GPT) | API anahtarı ([platform.openai.com](https://platform.openai.com/api-keys)) | |
| Google Gemini | API anahtarı ([aistudio.google.com](https://aistudio.google.com/apikey)) | Ücretsiz kotası var |
| OpenRouter | API anahtarı ([openrouter.ai](https://openrouter.ai/keys)) | Tek anahtarla yüzlerce model |
| Groq, Mistral, DeepSeek, xAI (Grok) | API anahtarı | |
| **Ollama** | [Ollama](https://ollama.com/download) kurulu olmalı | Yerel, ücretsiz, internetsiz |
| **LM Studio** | [LM Studio](https://lmstudio.ai) sunucusu açık olmalı | Yerel, ücretsiz, internetsiz |
| **Başka (OpenAI uyumlu)** | Sunucu adresi | llama.cpp, vLLM, Jan, LocalAI, text-generation-webui… |

Her sağlayıcının anahtarı, adresi ve seçili modeli ayrı saklanır; sağlayıcılar arasında geçiş
yaparken yeniden girmeniz gerekmez. Panelin üstündeki sağlayıcı adına tıklayarak değiştirebilirsiniz.

- **Model seçimi.** Sağlayıcının model listesi gösterilir; ilk kurulumda listenin başındaki seçilir.
  Listede olmayan bir model için **Başka bir model adı yaz…** seçeneğini kullanın.
- **Açık dosyanız** sorunuzla birlikte gönderilir. Asistan yazdığı kodu önce kendisi denetler ve
  çalıştırır, çıktısına bakar, hata varsa düzeltir.
- **Dosya değişiklikleri.** Asistan dosyanızı kendisi değiştirmez; yeni içeriği öneri olarak
  gösterir. **Uygula** düğmesine bastığınızda değişiklik yazılır. <kbd>Ctrl+Z</kbd> ile geri alabilirsiniz.
  Yanıttaki her Orhunca kod bloğunun altında da **Uygula** ve **Kopyala** düğmeleri bulunur.
- **Araç kullanamayan modeller.** Bazı (çoğunlukla küçük ya da eski) modeller araç çağıramaz. Asistan
  bunu kendiliğinden anlar ve düz sohbete geçer; bu durumda kodu denetleyip çalıştıramaz, kodu bir
  kod bloğunda verir. En iyi sonuç için araç destekleyen bir model seçin.
- **Az belirteç.** Asistan her istekte tam rehber (~7.400 belirteç) yerine kısa bir özet (~1.400)
  gönderir; model ayrıntı gerektiğinde rehberin yalnızca ilgili bölümünü okur. Uzun program
  çıktıları modele kırpılarak gönderilir.
- **Yeni başlayanlar için.** Asistan Türkçe ve sade anlatır. Bir alıştırmayı çözmeye çalışan
  kullanıcıya cevabı hemen vermez, ipucuyla yol gösterir.
- **Anahtar ve ücret.** Anahtarlar yalnızca bu bilgisayarda saklanır (`~/.config/orhunca/asistan.json`,
  Windows'ta `%APPDATA%\Orhunca\asistan.json`) ve Stüdyo arayüzüne bile geri gönderilmez.
  Bulut sağlayıcılarında kullanım ücreti kendi hesabınıza yansır.

### Yerel modeller (ücretsiz, internetsiz)

Yerel modellerde kodunuz bilgisayarınızdan çıkmaz ve ücret ödenmez. Model ne kadar büyükse o kadar
iyi sonuç verir ama o kadar çok bellek ister.

**Ollama**

1. [ollama.com/download](https://ollama.com/download) adresinden Ollama'yı kurun.
2. Terminalde araç kullanabilen bir model indirin:

   ```sh
   ollama pull qwen2.5-coder:7b     # 8 GB bellek için iyi bir başlangıç
   ollama pull qwen2.5-coder:14b    # 16 GB ve üstü
   ```

   `llama3.1`, `qwen3`, `mistral-nemo`, `gpt-oss` gibi araç destekleyen başka modeller de olur.
3. Stüdyo'da **Asistan → Ollama → Bağlan**'a tıklayın ve modeli seçin.

Stüdyo, dil rehberinin sığması için Ollama'dan 16 bin belirteçlik bağlam penceresi ister. Ollama
başka bir bilgisayarda çalışıyorsa **Sunucu adresi** alanına onun adresini yazın
(ör. `http://192.168.1.20:11434`).

**LM Studio**

1. [lmstudio.ai](https://lmstudio.ai) adresinden LM Studio'yu kurun ve bir model indirin
   (ör. *Qwen2.5 Coder 7B Instruct*).
2. Modeli yüklerken **Context Length** değerini en az `16384` yapın.
3. **Developer** sekmesinde sunucuyu başlatın (varsayılan adres `http://localhost:1234`).
4. Stüdyo'da **Asistan → LM Studio → Bağlan**.

**Diğer sunucular.** llama.cpp (`llama-server --jinja -c 16384 -m model.gguf`), vLLM, Jan, LocalAI
gibi OpenAI uyumlu her sunucu **Başka (OpenAI uyumlu)** seçeneğiyle kullanılır. Adresi `/v1` ile
biten biçimde yazın (ör. `http://localhost:8080/v1`). Sunucu anahtar istiyorsa anahtarı da girin.

### Asistanı kapatmak (okullar ve kurumlar)

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
| `orhunca_rehber` | Kısa rehberi (dilin özü, ~1.400 belirteç) döndürür; `bolum` ile tek bir bölüm (ör. `Modeller`), `hepsi` ile tam rehber |
| `orhunca_denetle` | Kodu derler; Türkçe hata mesajlarını, satır numarasını ve ipucunu ya da "Hata yok."u döndürür |
| `orhunca_calistir` | Programı süre sınırıyla çalıştırır; girdi verilebilir ve çıktı döndürülür |
| `orhunca_bicimlendir` | Kodu standart biçime getirir |

Araçlara kod metin olarak (`kod`) ya da diskteki bir dosyanın yoluyla (`dosya`) verilebilir. Rehber,
`orhunca://rehber` kaynağı olarak da sunulur.

**En kolayı:** Stüdyo'da **Asistan → Kendi ajanınızı bağlayın**. Pencere, kullandığınız araç için
hazır yapılandırmayı bilgisayarınızdaki Orhunca'nın tam yoluyla gösterir; **Kopyala** deyip
yapıştırmanız yeter. Yalnızca masaüstü uygulaması (Orhunca Stüdyo) kuruluysa komut
`orhunca-studyo mcp` olur; aşağıdaki örneklerde `orhunca` yerine onu yazın.

**Claude Code**

```sh
claude mcp add orhunca -- orhunca mcp
```

**Codex CLI**

```sh
codex mcp add orhunca -- orhunca mcp
```

ya da `~/.codex/config.toml`:

```toml
[mcp_servers.orhunca]
command = "orhunca"
args = ["mcp"]
```

**Gemini CLI**

```sh
gemini mcp add orhunca orhunca mcp
```

**Cursor** (`.cursor/mcp.json`), **Claude Desktop** (`claude_desktop_config.json`), **Windsurf**
(`~/.codeium/windsurf/mcp_config.json`), **Cline / Roo Code** (`cline_mcp_settings.json`),
**Gemini CLI** (`~/.gemini/settings.json`) ve MCP destekleyen diğer araçların çoğu:

```json
{
  "mcpServers": {
    "orhunca": { "command": "orhunca", "args": ["mcp"] }
  }
}
```

**VS Code / GitHub Copilot** (`.vscode/mcp.json`)

```json
{
  "servers": {
    "orhunca": { "type": "stdio", "command": "orhunca", "args": ["mcp"] }
  }
}
```

**Continue** (`.continue/mcpServers/orhunca.yaml`)

```yaml
name: Orhunca
version: 0.0.1
schema: v1
mcpServers:
  - name: orhunca
    command: orhunca
    args: ["mcp"]
```

**Zed** (`settings.json`)

```json
{
  "context_servers": {
    "orhunca": { "source": "custom", "command": "orhunca", "args": ["mcp"] }
  }
}
```

**OpenCode** (`opencode.json`)

```json
{
  "mcp": {
    "orhunca": { "type": "local", "command": ["orhunca", "mcp"], "enabled": true }
  }
}
```

**Yerel modellerle ajan.** Cline, Roo Code, Continue, Goose ve Zed, Ollama ya da LM Studio'daki
yerel modellerle de çalışır; MCP bağlantısı aynıdır. Böylece internetsiz ve ücretsiz bir Orhunca
ajanı kurabilirsiniz.

### AGENTS.md

MCP desteklemeyen ya da terminal kullanan ajanlar (Codex, Copilot, Cursor, Jules, Aider…) proje
klasöründeki `AGENTS.md` dosyasını okur. Stüdyo'da **Kendi ajanınızı bağlayın → Projeye AGENTS.md
ekle** ile projeye Orhunca'nın yazım kurallarını ve komutlarını anlatan bir `AGENTS.md` eklenir.

Bağlantıdan sonra ajana örneğin şunu yazabilirsiniz: *"Orhunca ile bir not ortalaması programı
yaz ve çalıştırıp dene."* Ajan önce rehberi okur, sonra kodu yazar, denetler ve çalıştırır.

Ajanınız terminal kullanabiliyorsa MCP olmadan da çalışabilir: `orhunca denetle dosya.ohc` ve
`orhunca çalıştır dosya.ohc` komutları aynı Türkçe hata mesajlarını verir. Dil rehberinin
tamamı tek dosya olarak [llms-full.txt](https://furkan003.github.io/Orhunca/llms-full.txt)
adresinde de bulunur.
