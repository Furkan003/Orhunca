//! `orhunca paketle --hedef ios`: arayüz programından bir iPhone/iPad uygulamasının
//! Xcode projesini üretir (mobil/ios şablonu). iOS uygulamaları yalnızca macOS'ta
//! Xcode ile derlenebilir ve Apple tarafından imzalanmalıdır; bu yüzden hazır bir
//! uygulama değil, açılıp çalıştırılmaya hazır bir proje üretilir. Projedeki
//! .github/workflows/ios.yml, Mac'i olmayanlar için GitHub'da imzasız .ipa derler.

use std::path::Path;

const UYGULAMA_SWIFT: &str = include_str!("../mobil/ios/Uygulama/AppDelegate.swift");
const SAYFA_SWIFT: &str = include_str!("../mobil/ios/Uygulama/SayfaDenetcisi.swift");
const INFO_PLIST: &str = include_str!("../mobil/ios/Uygulama/Info.plist");
const PROJE: &str = include_str!("../mobil/ios/Uygulama.xcodeproj/project.pbxproj");
const IS_AKISI: &str = include_str!("../mobil/ios/ios.yml");
const SIMGE: &[u8] = include_bytes!("../runtime/kabuk/ios-simge.png");

pub struct Uygulama<'a> {
    pub ad: &'a str,
    pub kimlik: &'a str,
    pub surum: &'a str,
    pub sayfa: &'a [u8],
    /// 1024×1024, saydamlıksız PNG; verilmezse Orhunca simgesi
    pub simge: Option<&'a [u8]>,
}

/// Apple paket kimliğinde alt çizgi olamaz: `org.orhunca.sinif_defteri` → `org.orhunca.sinif-defteri`
pub fn paket_kimligi(k: &str) -> String {
    k.replace('_', "-")
}

fn xml_kac(m: &str) -> String {
    m.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn proje_olustur(u: &Uygulama, klasor: &Path) -> Result<(), String> {
    if !crate::android::kimlik_gecerli_mi(u.kimlik) {
        return Err(format!("geçersiz paket kimliği '{}'", u.kimlik));
    }
    if !u.surum.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(format!(
            "iOS sürümü yalnızca rakam ve noktadan oluşmalı (ör. 1.0.0): '{}'",
            u.surum
        ));
    }
    let kimlik = paket_kimligi(u.kimlik);
    let yaz = |yol: &str, veri: &[u8]| -> Result<(), String> {
        let p = klasor.join(yol);
        if let Some(k) = p.parent() {
            std::fs::create_dir_all(k).map_err(|e| format!("'{}': {e}", k.display()))?;
        }
        std::fs::write(&p, veri).map_err(|e| format!("'{}' yazılamadı: {e}", p.display()))
    };
    yaz("Uygulama/AppDelegate.swift", UYGULAMA_SWIFT.as_bytes())?;
    yaz("Uygulama/SayfaDenetcisi.swift", SAYFA_SWIFT.as_bytes())?;
    yaz(
        "Uygulama/Info.plist",
        INFO_PLIST.replace("{{AD}}", &xml_kac(u.ad)).as_bytes(),
    )?;
    yaz("Uygulama/uygulama.html", u.sayfa)?;
    yaz(
        "Uygulama/Assets.xcassets/Contents.json",
        b"{\n  \"info\" : { \"author\" : \"xcode\", \"version\" : 1 }\n}\n",
    )?;
    yaz(
        "Uygulama/Assets.xcassets/AppIcon.appiconset/Contents.json",
        br#"{
  "images" : [
    { "filename" : "simge.png", "idiom" : "universal", "platform" : "ios", "size" : "1024x1024" }
  ],
  "info" : { "author" : "xcode", "version" : 1 }
}
"#,
    )?;
    yaz(
        "Uygulama/Assets.xcassets/AppIcon.appiconset/simge.png",
        u.simge.unwrap_or(SIMGE),
    )?;
    yaz(
        "Uygulama.xcodeproj/project.pbxproj",
        PROJE
            .replace("{{KIMLIK}}", &kimlik)
            .replace("{{SURUM}}", u.surum)
            .as_bytes(),
    )?;
    yaz(".github/workflows/ios.yml", IS_AKISI.as_bytes())?;
    yaz(
        "BENİOKU.md",
        format!(
            "# {ad} (iPhone ve iPad)\n\n\
             Bu klasör `orhunca paketle --hedef ios` ile üretilmiş bir Xcode projesidir.\n\
             Paket kimliği: `{kimlik}` · Sürüm: {surum}\n\n\
             ## Mac'te\n\n\
             1. `Uygulama.xcodeproj` dosyasını Xcode ile açın.\n\
             2. Sol üstte **Uygulama** hedefini seçin; *Signing & Capabilities* bölümünde\n   \
             **Team** olarak Apple hesabınızı seçin (ücretsiz hesap kendi telefonunuz için yeter).\n\
             3. iPhone'unuzu bağlayıp ▶ düğmesine basın.\n\n\
             App Store'da yayımlamak için Apple Developer Program üyeliği (yıllık ücretli) gerekir:\n\
             *Product → Archive → Distribute App*.\n\n\
             ## Mac olmadan\n\n\
             Bu klasörü bir GitHub deposuna yükleyin. `.github/workflows/ios.yml` her gönderimde\n\
             imzasız bir `Uygulama.ipa` üretir (*Actions → son çalışma → Artifacts*). İmzasız .ipa,\n\
             AltStore ya da Sideloadly ile ücretsiz Apple hesabınızla iPhone'a kurulabilir.\n",
            ad = u.ad,
            surum = u.surum
        )
        .as_bytes(),
    )?;
    Ok(())
}
