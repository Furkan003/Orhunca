//! Stüdyo'nun Git paneli: durum, fark, hazırlama, işleme (commit), gönderme ve çekme.
//!
//! Git, deponun kendi ayarlarındaki komutları çalıştırabilir (fsmonitor, filtreler, kancalar);
//! bu yüzden panel yalnızca güvenilen projelerde açılır (api.rs). Farklarda dış fark araçları
//! ve textconv yine de kapatılır. Ağ işlemlerinde Git parola sormaz (GIT_TERMINAL_PROMPT=0):
//! kimlik bilgisi yoksa hata hemen döner.

use serde_json::{json, Value};
use std::path::Path;

fn git(kok: &Path, args: &[&str]) -> Result<String, String> {
    let c = crate::komut("git")
        .args(["-c", "core.quotepath=false", "-c", "color.ui=false"])
        .args(args)
        .current_dir(kok)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "Git kurulu değil: https://git-scm.com adresinden kurun.".to_string()
            } else {
                format!("git çalıştırılamadı: {e}")
            }
        })?;
    if c.status.success() {
        Ok(String::from_utf8_lossy(&c.stdout).into_owned())
    } else {
        let hata = String::from_utf8_lossy(&c.stderr).trim().to_string();
        Err(turkcelestir(&hata))
    }
}

/// Sık görülen Git hatalarını Türkçe açıklar (özgün metin de kalır).
fn turkcelestir(h: &str) -> String {
    let aciklama = if h.contains("Please tell me who you are") || h.contains("user.email") {
        "Git adınızı ve e-postanızı bilmiyor. Terminalde bir kez çalıştırın:\n  git config --global user.name \"Adınız\"\n  git config --global user.email \"eposta@ornek.com\""
    } else if h.contains("could not read Username") || h.contains("Authentication failed") {
        "Uzak depo kimlik doğrulaması istiyor. GitHub için terminalde bir kez `gh auth login` ya da Git Credential Manager ile giriş yapın."
    } else if h.contains("No configured push destination")
        || h.contains("does not appear to be a git repository")
    {
        "Bu depoya bağlı uzak depo (remote) yok: git remote add origin <adres>"
    } else if h.contains("has no upstream branch") {
        "Bu dal henüz uzak depoda yok; ilk gönderimde oluşturulur."
    } else if h.contains("rejected") && h.contains("fetch first") {
        "Uzak depoda sizde olmayan değişiklikler var: önce Çek (pull), sonra Gönder."
    } else if h.contains("nothing to commit") {
        "İşlenecek değişiklik yok."
    } else {
        return if h.is_empty() {
            "Git işlemi başarısız.".into()
        } else {
            h.to_string()
        };
    };
    format!("{aciklama}\n\n({h})")
}

pub fn depo_mu(kok: &Path) -> bool {
    git(kok, &["rev-parse", "--is-inside-work-tree"]).is_ok_and(|c| c.trim() == "true")
}

/// `git status --porcelain=v1 -z --branch`
pub fn durum(kok: &Path) -> Result<Value, String> {
    if !depo_mu(kok) {
        return Ok(json!({ "depo": false }));
    }
    let cikti = git(
        kok,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
    )?;
    let mut dal = String::new();
    let (mut onde, mut geride) = (0, 0);
    let mut degisiklikler = Vec::new();
    let mut parcalar = cikti.split('\0').filter(|p| !p.is_empty());
    while let Some(p) = parcalar.next() {
        if let Some(b) = p.strip_prefix("## ") {
            // "ana...origin/ana [ahead 1, behind 2]" ya da "No commits yet on ana"
            let b = b.strip_prefix("No commits yet on ").unwrap_or(b);
            dal = b
                .split("...")
                .next()
                .unwrap_or(b)
                .split(' ')
                .next()
                .unwrap_or("")
                .to_string();
            if let Some(i) = b.find('[') {
                for parca in b[i + 1..].trim_end_matches(']').split(", ") {
                    if let Some(n) = parca.strip_prefix("ahead ") {
                        onde = n.parse().unwrap_or(0);
                    } else if let Some(n) = parca.strip_prefix("behind ") {
                        geride = n.parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }
        if p.len() < 4 {
            continue;
        }
        let (x, y) = (&p[0..1], &p[1..2]);
        let yol = p[3..].to_string();
        // Yeniden adlandırmada eski ad ayrı bir parça olarak gelir.
        if x == "R" || x == "C" {
            parcalar.next();
        }
        if x == "?" {
            degisiklikler.push(json!({ "yol": yol, "durum": "?", "hazir": false }));
            continue;
        }
        if x != " " {
            degisiklikler.push(json!({ "yol": yol, "durum": x, "hazir": true }));
        }
        if y != " " {
            degisiklikler.push(json!({ "yol": yol, "durum": y, "hazir": false }));
        }
    }
    let uzak = git(kok, &["remote"])
        .map(|r| !r.trim().is_empty())
        .unwrap_or(false);
    Ok(json!({
        "depo": true, "dal": dal, "onde": onde, "geride": geride, "uzak": uzak,
        "degisiklikler": degisiklikler,
    }))
}

/// Göreli yolun proje içinde kaldığını denetler (`..` ve mutlak yol yok).
fn guvenli_yol(yol: &str) -> Result<(), String> {
    let p = Path::new(yol);
    if yol.is_empty()
        || p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!("geçersiz yol: {yol}"));
    }
    Ok(())
}

/// Dosyanın farkı (birleşik fark biçiminde). Takip edilmeyen dosya tümüyle eklenmiş görünür.
pub fn fark(kok: &Path, yol: &str, hazir: bool) -> Result<String, String> {
    guvenli_yol(yol)?;
    let mut args = vec!["diff", "--no-ext-diff", "--no-textconv", "--no-color"];
    if hazir {
        args.push("--cached");
    }
    args.extend(["--", yol]);
    let f = git(kok, &args)?;
    if !f.is_empty() || hazir {
        return Ok(f);
    }
    // Takip edilmeyen dosya
    let icerik = std::fs::read_to_string(kok.join(yol)).unwrap_or_default();
    let mut m = format!(
        "--- /dev/null\n+++ b/{yol}\n@@ -0,0 +1,{} @@\n",
        icerik.lines().count()
    );
    for s in icerik.lines() {
        m.push('+');
        m.push_str(s);
        m.push('\n');
    }
    Ok(m)
}

pub fn hazirla(kok: &Path, yollar: &[String], geri: bool) -> Result<(), String> {
    for y in yollar {
        guvenli_yol(y)?;
    }
    let mut args: Vec<&str> = if geri {
        // Henüz hiç işleme yoksa HEAD yoktur: rm --cached
        if git(kok, &["rev-parse", "--verify", "HEAD"]).is_ok() {
            vec!["reset", "-q", "HEAD", "--"]
        } else {
            vec!["rm", "-q", "--cached", "--"]
        }
    } else {
        vec!["add", "-A", "--"]
    };
    args.extend(yollar.iter().map(String::as_str));
    git(kok, &args).map(|_| ())
}

/// Çalışma klasöründeki değişikliği atar; dosyanın son hâli önce yerel geçmişe yazılır.
pub fn degisikligi_at(kok: &Path, yol: &str, takipsiz: bool) -> Result<(), String> {
    guvenli_yol(yol)?;
    let tam = kok.join(yol);
    if let Ok(icerik) = std::fs::read_to_string(&tam) {
        super::gecmis::ilk_hali_sakla(&tam);
        super::gecmis::kaydet(&tam, &icerik);
    }
    if takipsiz {
        return std::fs::remove_file(&tam).map_err(|e| format!("silinemedi: {e}"));
    }
    git(kok, &["checkout", "--", yol]).map(|_| ())
}

pub fn isle(kok: &Path, mesaj: &str) -> Result<String, String> {
    let mesaj = mesaj.trim();
    if mesaj.is_empty() {
        return Err("Ne değiştirdiğinizi anlatan bir mesaj yazın.".into());
    }
    git(kok, &["commit", "-q", "-m", mesaj])?;
    Ok(git(kok, &["log", "-1", "--format=%h %s"])?
        .trim()
        .to_string())
}

pub fn gonder(kok: &Path) -> Result<String, String> {
    match git(kok, &["push"]) {
        Err(e) if e.contains("henüz uzak depoda yok") => {
            let dal = git(kok, &["rev-parse", "--abbrev-ref", "HEAD"])?;
            git(kok, &["push", "-u", "origin", dal.trim()])
        }
        r => r,
    }
    .map(|_| "Gönderildi.".into())
}

pub fn cek(kok: &Path) -> Result<String, String> {
    git(kok, &["pull", "--ff-only"]).map(|c| {
        if c.contains("Already up to date") {
            "Zaten güncel.".into()
        } else {
            "Değişiklikler çekildi.".into()
        }
    })
}

pub fn baslat(kok: &Path) -> Result<(), String> {
    git(kok, &["init", "-q", "--initial-branch=ana"])
        .or_else(|_| git(kok, &["init", "-q"]))
        .map(|_| ())
}

/// Son işlemeler: [{kisa, mesaj, yazar, zaman}]
pub fn gecmis(kok: &Path) -> Value {
    let l = git(
        kok,
        &[
            "log",
            "-30",
            "--date=format:%d.%m.%Y %H:%M",
            "--format=%h%x1f%s%x1f%an%x1f%ad",
        ],
    )
    .unwrap_or_default();
    Value::Array(
        l.lines()
            .filter_map(|s| {
                let p: Vec<&str> = s.split('\u{1f}').collect();
                (p.len() == 4)
                    .then(|| json!({ "kisa": p[0], "mesaj": p[1], "yazar": p[2], "zaman": p[3] }))
            })
            .collect(),
    )
}
