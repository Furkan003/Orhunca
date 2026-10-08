//! `orhunca yayınla`: web programını bir Linux sunucusuna (VPS) yayınlar.
//!
//! Program bu bilgisayarda Linux (x86_64) için derlenir; sunucuda Orhunca kurulu olması
//! gerekmez. Yayın klasörüne (`cikti/yayın/`) program, `statik/` klasörü, varsa
//! `.env.sunucu` (sunucuda `.env` olur), bir systemd hizmeti ve `kur.sh` yazılır. Sunucu
//! verildiyse klasör SSH ile gönderilir ve `kur.sh` sunucuda çalıştırılır: program
//! `/srv/<ad>` klasörüne kurulur, hizmet olarak başlatılır, alan adı verildiyse Caddy ile
//! kendiliğinden HTTPS sertifikası alınır. Aynı komut güncellemek için de kullanılır;
//! sunucudaki veri dosyalarına dokunulmaz. Bkz. docs/yayinlama.md.

use crate::derleme;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub struct Ayarlar {
    /// `kullanıcı@sunucu`; verilmezse yalnızca yayın klasörü hazırlanır.
    pub sunucu: Option<String>,
    /// Alan adı (ör. `ornek.com`); verilirse Caddy ile HTTPS kurulur.
    pub alan: Option<String>,
    pub kapi: u16,
    /// Sunucudaki kurulum klasörü (varsayılan `/srv/<ad>`).
    pub klasor: Option<String>,
    pub ssh_kapi: Option<u16>,
}

/// Hizmet ve dosya adı olarak kullanılacak sade ad: `Ürün Takibi` → `urun-takibi`.
pub fn hizmet_adi(ad: &str) -> String {
    let mut s = String::new();
    for c in ad.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'ç' => 'c',
            'ğ' => 'g',
            // 'İ' küçültülünce 'i' ve birleşen nokta (U+0307) olur; nokta atılır.
            '\u{307}' => continue,
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() {
        "uygulama".into()
    } else {
        s
    }
}

fn alan_gecerli(alan: &str) -> bool {
    !alan.is_empty()
        && alan.len() <= 253
        && alan.contains('.')
        && alan
            .split('.')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

fn klasor_gecerli(k: &str) -> bool {
    k.starts_with('/')
        && k.len() > 1
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
        && !k.split('/').any(|p| p == "..")
}

/// Yayın klasörünü hazırlar; (klasör, hizmet adı) döndürür.
pub fn hazirla(giris: &Path, a: &Ayarlar) -> Result<(PathBuf, String), String> {
    if let Some(alan) = &a.alan {
        if !alan_gecerli(alan) {
            return Err(format!("geçersiz alan adı '{alan}' (ör. ornek.com)"));
        }
    }
    let kok = derleme::proje_koku(giris);
    let proje_adi = derleme::proje_dosyasi(&kok)
        .and_then(|p| derleme::proje_ayari(&p, &["ad"]))
        .or_else(|| {
            kok.canonicalize()
                .ok()
                .and_then(|k| k.file_name().map(|a| a.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "uygulama".into());
    let ad = hizmet_adi(&proje_adi);
    let hedef = a.klasor.clone().unwrap_or_else(|| format!("/srv/{ad}"));
    if !klasor_gecerli(&hedef) {
        return Err(format!(
            "geçersiz sunucu klasörü '{hedef}' (tam yol, yalnızca harf, rakam ve / . _ -)"
        ));
    }

    let program = derleme::yukle(giris).map_err(|h| h.metin)?;
    if program.arayuz_programi() {
        return Err(
            "bu bir arayüz programı; sunucu gerekmez\nipucu: orhunca derle --hedef web \
             ile tek bir .html dosyası üretip herhangi bir web barındırmaya yükleyin"
                .into(),
        );
    }

    let yayin = kok.join("cikti").join("yayın");
    if yayin.exists() {
        std::fs::remove_dir_all(&yayin)
            .map_err(|e| format!("'{}' silinemedi: {e}", yayin.display()))?;
    }
    std::fs::create_dir_all(&yayin).map_err(|e| e.to_string())?;
    derleme::derle(giris, &yayin.join(&ad), Some("linux")).map_err(|h| h.metin)?;
    let statik = kok.join("statik");
    if statik.is_dir() {
        klasor_kopyala(&statik, &yayin.join("statik"))?;
    }
    // Bu bilgisayardaki .env geliştirme ayarlarıdır; sunucuya .env.sunucu gönderilir.
    let env = kok.join(".env.sunucu");
    if env.is_file() {
        std::fs::copy(&env, yayin.join(".env")).map_err(|e| e.to_string())?;
    }
    let yaz = |dosya: &str, icerik: String| {
        std::fs::write(yayin.join(dosya), icerik).map_err(|e| e.to_string())
    };
    yaz(&format!("{ad}.service"), hizmet_dosyasi(&ad, &hedef, a))?;
    yaz("kur.sh", kurulum_betigi(&ad, &hedef, a))?;
    Ok((yayin, ad))
}

pub(crate) fn klasor_kopyala(kaynak: &Path, hedef: &Path) -> Result<(), String> {
    std::fs::create_dir_all(hedef).map_err(|e| e.to_string())?;
    for g in std::fs::read_dir(kaynak)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let p = g.path();
        let h = hedef.join(g.file_name());
        if p.is_dir() {
            klasor_kopyala(&p, &h)?;
        } else {
            std::fs::copy(&p, &h).map_err(|e| format!("'{}' kopyalanamadı: {e}", p.display()))?;
        }
    }
    Ok(())
}

fn hizmet_dosyasi(ad: &str, hedef: &str, a: &Ayarlar) -> String {
    // Alan adıyla Caddy önünde durur, program yalnızca sunucunun içinden erişilebilir.
    let adres = if a.alan.is_some() {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    format!(
        "# orhunca yayınla tarafından üretildi.\n\
         [Unit]\n\
         Description={ad} (Orhunca web uygulaması)\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         User=orhunca\n\
         Group=orhunca\n\
         WorkingDirectory={hedef}\n\
         ExecStart={hedef}/{ad}\n\
         Environment=ORHUNCA_KAPI={kapi}\n\
         Environment=ORHUNCA_ADRES={adres}\n\
         AmbientCapabilities=CAP_NET_BIND_SERVICE\n\
         Restart=always\n\
         RestartSec=2\n\
         NoNewPrivileges=true\n\
         ProtectSystem=full\n\
         ProtectHome=true\n\
         PrivateTmp=true\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        kapi = a.kapi
    )
}

fn kurulum_betigi(ad: &str, hedef: &str, a: &Ayarlar) -> String {
    let kapi = a.kapi;
    let mut s = format!(
        r#"#!/bin/sh
# '{ad}' uygulamasını kurar ya da günceller (orhunca yayınla tarafından üretildi).
# Kullanım (sunucuda): sudo sh kur.sh
set -eu
AD={ad}
HEDEF={hedef}
KAPI={kapi}
KAYNAK=$(cd "$(dirname "$0")" && pwd)

if [ "$(id -u)" != 0 ]; then
    echo "kur.sh yönetici olarak çalıştırılmalı: sudo sh $0" >&2
    exit 1
fi
case "$(uname -m)" in
    x86_64 | amd64) ;;
    *)
        echo "Bu sunucunun işlemcisi $(uname -m). orhunca yayınla şimdilik x86_64 (amd64) sunuculara yayınlar." >&2
        echo "Sunucuya Orhunca'yı kurup programı orada derleyebilirsiniz: orhunca derle" >&2
        exit 1
        ;;
esac
if [ ! -d /run/systemd/system ]; then
    echo "Bu sunucuda systemd yok; programı kendiniz başlatın: cd $KAYNAK && ORHUNCA_ADRES=0.0.0.0 ./$AD" >&2
    exit 1
fi

echo "→ $AD kuruluyor: $HEDEF"
id orhunca >/dev/null 2>&1 || useradd --system --home-dir /nonexistent --shell /usr/sbin/nologin orhunca
mkdir -p "$HEDEF"
# Çalışan programın üzerine yazılamaz; yenisi yanına kopyalanıp adı değiştirilir.
cp "$KAYNAK/$AD" "$HEDEF/.$AD.yeni"
chmod 755 "$HEDEF/.$AD.yeni"
mv -f "$HEDEF/.$AD.yeni" "$HEDEF/$AD"
# statik/ silinmez, üzerine kopyalanır: programın oraya taşıdığı yüklemeler korunur.
if [ -d "$KAYNAK/statik" ]; then
    mkdir -p "$HEDEF/statik"
    cp -r "$KAYNAK/statik/." "$HEDEF/statik/"
fi
if [ -f "$KAYNAK/.env" ]; then
    cp "$KAYNAK/.env" "$HEDEF/.env"
    chmod 600 "$HEDEF/.env"
fi
# Veri dosyaları (veri/, kayıtlar, yüklenen dosyalar) HEDEF klasöründe kalır; dokunulmaz.
chown -R orhunca:orhunca "$HEDEF"
cp "$KAYNAK/$AD.service" "/etc/systemd/system/$AD.service"
systemctl daemon-reload
systemctl enable "$AD" >/dev/null 2>&1
systemctl restart "$AD"
sleep 1
if ! systemctl is-active --quiet "$AD"; then
    echo "Program başlatılamadı. Son kayıtlar:" >&2
    journalctl -u "$AD" -n 25 --no-pager >&2 || true
    exit 1
fi
"#
    );
    match &a.alan {
        Some(alan) => s.push_str(&format!(
            r##"
# Alan adı: Caddy isteği programa iletir ve HTTPS sertifikasını kendiliğinden alır.
ALAN={alan}
if ! command -v caddy >/dev/null 2>&1; then
    echo "→ Caddy kuruluyor"
    if command -v apt-get >/dev/null 2>&1; then
        apt-get update -qq || true
        if ! apt-get install -y -qq caddy; then
            # Eski Ubuntu/Debian sürümlerinde Caddy'nin kendi paket deposu eklenir.
            apt-get install -y -qq curl gnupg debian-keyring debian-archive-keyring apt-transport-https || true
            curl -1sLf https://dl.cloudsmith.io/public/caddy/stable/gpg.key \
                | gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
            curl -1sLf https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt \
                > /etc/apt/sources.list.d/caddy-stable.list
            apt-get update -qq && apt-get install -y -qq caddy || true
        fi
    elif command -v dnf >/dev/null 2>&1; then
        dnf install -y -q caddy || true
    fi
fi
if ! command -v caddy >/dev/null 2>&1; then
    echo "Caddy kurulamadı. Kurulum: https://caddyserver.com/docs/install" >&2
    echo "Sonra yeniden yayınlayın; program şimdilik 127.0.0.1:$KAPI adresinde çalışıyor." >&2
    exit 1
fi
mkdir -p /etc/caddy
touch /etc/caddy/Caddyfile
# Bu uygulamanın önceki bloğu (alan adı ya da kapı değişmiş olabilir) silinip yeniden yazılır.
awk -v m="# orhunca: $AD" '$0 == m {{ atla = 3; next }} atla > 0 {{ atla--; next }} {{ print }}' \
    /etc/caddy/Caddyfile > /etc/caddy/Caddyfile.yeni
printf '# orhunca: %s\n%s {{\n    reverse_proxy 127.0.0.1:%s\n}}\n' "$AD" "$ALAN" "$KAPI" >> /etc/caddy/Caddyfile.yeni
mv /etc/caddy/Caddyfile.yeni /etc/caddy/Caddyfile
if command -v ufw >/dev/null 2>&1 && ufw status | grep -q "Status: active"; then
    ufw allow 80/tcp >/dev/null && ufw allow 443/tcp >/dev/null
fi
systemctl enable caddy >/dev/null 2>&1 || true
systemctl reload caddy 2>/dev/null || systemctl restart caddy
echo "✓ Yayında: https://$ALAN"
echo "  (Alan adının A kaydı bu sunucunun IP adresini göstermeli; sertifika ilk ziyarette alınır.)"
"##
        )),
        None => s.push_str(
            r#"
if command -v ufw >/dev/null 2>&1 && ufw status | grep -q "Status: active"; then
    ufw allow "$KAPI/tcp" >/dev/null
fi
IP=$(hostname -I 2>/dev/null | cut -d' ' -f1)
echo "✓ Yayında: http://${IP:-sunucu-adresi}:$KAPI"
echo "  Alan adı ve HTTPS için: orhunca yayınla <sunucu> --alan ornek.com"
"#,
        ),
    }
    s.push_str(
        "echo \"  Kayıtlar: journalctl -u $AD -f   Yeniden başlat: systemctl restart $AD\"\n",
    );
    s
}

/// Klasörü bellekte bir tar arşivine (ustar) çevirir.
pub fn tar(klasor: &Path) -> Result<Vec<u8>, String> {
    fn ekle(cikti: &mut Vec<u8>, ad: &str, veri: Option<&[u8]>, kip: u32) -> Result<(), String> {
        let (ad_b, onek) = if ad.len() <= 100 {
            (ad.to_string(), String::new())
        } else {
            // Uzun adlar: önek (155) + ad (100), '/' sınırından bölünür.
            let b = ad
                .char_indices()
                .filter(|(i, c)| *c == '/' && *i <= 155 && ad.len() - i - 1 <= 100)
                .map(|(i, _)| i)
                .next_back()
                .ok_or_else(|| format!("dosya adı çok uzun: {ad}"))?;
            (ad[b + 1..].to_string(), ad[..b].to_string())
        };
        let mut b = [0u8; 512];
        let koy = |b: &mut [u8; 512], yer: usize, uz: usize, m: &[u8]| {
            b[yer..yer + m.len().min(uz)].copy_from_slice(&m[..m.len().min(uz)]);
        };
        let boy = veri.map_or(0, |v| v.len());
        koy(&mut b, 0, 100, ad_b.as_bytes());
        koy(&mut b, 100, 8, format!("{kip:07o}\0").as_bytes());
        koy(&mut b, 108, 8, b"0000000\0");
        koy(&mut b, 116, 8, b"0000000\0");
        koy(&mut b, 124, 12, format!("{boy:011o}\0").as_bytes());
        let zaman = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        koy(&mut b, 136, 12, format!("{zaman:011o}\0").as_bytes());
        koy(&mut b, 148, 8, b"        ");
        b[156] = if veri.is_some() { b'0' } else { b'5' };
        koy(&mut b, 257, 8, b"ustar\x0000");
        koy(&mut b, 345, 155, onek.as_bytes());
        let toplam: u32 = b.iter().map(|&x| x as u32).sum();
        koy(&mut b, 148, 8, format!("{toplam:06o}\0 ").as_bytes());
        cikti.extend_from_slice(&b);
        if let Some(v) = veri {
            cikti.extend_from_slice(v);
            cikti.resize(cikti.len().div_ceil(512) * 512, 0);
        }
        Ok(())
    }
    fn gez(k: &Path, goreli: &str, cikti: &mut Vec<u8>) -> Result<(), String> {
        let mut g: Vec<_> = std::fs::read_dir(k)
            .map_err(|e| e.to_string())?
            .flatten()
            .collect();
        g.sort_by_key(|x| x.file_name());
        for x in g {
            let ad = format!("{goreli}{}", x.file_name().to_string_lossy());
            let p = x.path();
            if p.is_dir() {
                ekle(cikti, &format!("{ad}/"), None, 0o755)?;
                gez(&p, &format!("{ad}/"), cikti)?;
            } else {
                let v = std::fs::read(&p).map_err(|e| e.to_string())?;
                ekle(cikti, &ad, Some(&v), 0o644)?;
            }
        }
        Ok(())
    }
    let mut cikti = Vec::new();
    gez(klasor, "", &mut cikti)?;
    cikti.extend_from_slice(&[0u8; 1024]);
    Ok(cikti)
}

fn ssh(a: &Ayarlar, sunucu: &str, tty: bool) -> std::process::Command {
    let mut k = crate::komut("ssh");
    if let Some(p) = a.ssh_kapi {
        k.arg("-p").arg(p.to_string());
    }
    if tty {
        k.arg("-t");
    }
    k.arg(sunucu);
    k
}

/// Yayın klasörünü sunucuya gönderir ve kurulum betiğini çalıştırır.
pub fn gonder(yayin: &Path, ad: &str, sunucu: &str, a: &Ayarlar) -> Result<(), String> {
    if sunucu.starts_with('-') || sunucu.contains(char::is_whitespace) {
        return Err(format!("geçersiz sunucu '{sunucu}' (ör. kok@203.0.113.5)"));
    }
    let arsiv = tar(yayin)?;
    let gecici = format!("$HOME/.orhunca-yayin/{ad}");
    println!(
        "→ {sunucu} sunucusuna gönderiliyor ({} KB)",
        arsiv.len() / 1024
    );
    let mut c = ssh(a, sunucu, false)
        .arg(format!(
            "rm -rf \"{gecici}\" && mkdir -p \"{gecici}\" && tar -xf - -C \"{gecici}\""
        ))
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "ssh çalıştırılamadı: {e}\nipucu: OpenSSH istemcisi gerekir (Windows 10 ve sonrasında hazır gelir; \
                 Ayarlar → Uygulamalar → İsteğe bağlı özellikler → OpenSSH İstemcisi)"
            )
        })?;
    if let Some(mut g) = c.stdin.take() {
        g.write_all(&arsiv)
            .map_err(|e| format!("sunucuya gönderilemedi: {e}"))?;
    }
    let d = c.wait().map_err(|e| e.to_string())?;
    if !d.success() {
        return Err(
            "sunucuya bağlanılamadı ya da dosyalar gönderilemedi (yukarıdaki ssh iletisine bakın)"
                .into(),
        );
    }
    println!("→ sunucuda kuruluyor");
    let d = ssh(a, sunucu, true)
        .arg(format!(
            "if [ \"$(id -u)\" = 0 ]; then sh \"{gecici}/kur.sh\"; else sudo sh \"{gecici}/kur.sh\"; fi"
        ))
        .status()
        .map_err(|e| format!("ssh çalıştırılamadı: {e}"))?;
    if !d.success() {
        return Err("sunucuda kurulum başarısız (yukarıdaki iletilere bakın)".into());
    }
    Ok(())
}

#[cfg(test)]
mod sinamalar {
    use super::*;

    #[test]
    fn hizmet_adlari() {
        assert_eq!(hizmet_adi("Ürün Takibi"), "urun-takibi");
        assert_eq!(hizmet_adi("İŞLEM_Günlüğü 2"), "islem-gunlugu-2");
        assert_eq!(hizmet_adi("  --  "), "uygulama");
    }

    #[test]
    fn tar_arsivi() {
        let k = std::env::temp_dir().join(format!("orhunca-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&k);
        let uzun = format!("{}/{}", "ç".repeat(40), "dosya_".repeat(15));
        std::fs::create_dir_all(k.join(&uzun)).unwrap();
        std::fs::write(k.join(&uzun).join("ı.txt"), "merhaba").unwrap();
        std::fs::write(k.join("boş"), "").unwrap();
        let a = tar(&k).unwrap();
        assert_eq!(a.len() % 512, 0);
        // Sistemdeki tar ile açılıp aynı içerik bulunmalı.
        let cikis = k.with_extension("acik");
        let _ = std::fs::remove_dir_all(&cikis);
        std::fs::create_dir_all(&cikis).unwrap();
        let mut c = match std::process::Command::new("tar")
            .arg("-xf")
            .arg("-")
            .arg("-C")
            .arg(&cikis)
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        c.stdin.take().unwrap().write_all(&a).unwrap();
        assert!(c.wait().unwrap().success());
        assert_eq!(
            std::fs::read_to_string(cikis.join(&uzun).join("ı.txt")).unwrap(),
            "merhaba"
        );
        assert!(cikis.join("boş").is_file());
        std::fs::remove_dir_all(&k).unwrap();
        std::fs::remove_dir_all(&cikis).unwrap();
    }
}
