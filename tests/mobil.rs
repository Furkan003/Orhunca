//! Telefon uygulamaları: `--hedef android` (APK'nın yapısı ve imzası; Android SDK
//! varsa apksigner ve aapt2 ile de doğrulanır) ve `--hedef ios` (Xcode projesi).

use std::path::Path;
use std::process::Command;

#[test]
fn android_paketi() {
    let k = std::env::temp_dir().join(format!("orhunca-android-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    std::fs::create_dir_all(&k).unwrap();
    let apk = k.join("sayac.apk");
    let calistir = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_orhunca"))
            .args(args)
            .env("ORHUNCA_ANDROID_ANAHTARI", k.join("anahtar"))
            .output()
            .unwrap()
    };
    let c = calistir(&[
        "paketle",
        "örnekler/arayüz/sayaç.ohc",
        "--hedef",
        "android",
        "-o",
        apk.to_str().unwrap(),
    ]);
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let v = std::fs::read(&apk).unwrap();
    // ZIP sonu → merkez dizin; hemen önünde APK imza bloğu
    let son = v.len() - 22;
    assert_eq!(&v[son..son + 4], &[0x50, 0x4b, 0x05, 0x06]);
    let merkez = u32::from_le_bytes(v[son + 16..son + 20].try_into().unwrap()) as usize;
    assert_eq!(&v[merkez - 16..merkez], b"APK Sig Block 42");
    let gecerli = |ara: &[u8]| v.windows(ara.len()).any(|w| w == ara);
    assert!(gecerli(b"assets/uygulama.html"));
    assert!(
        gecerli("<title>Sayaç</title>".as_bytes()),
        "sayfa APK'da olmalı"
    );

    // Aynı anahtar ikinci paketlemede de kullanılır (güncellemeler için şart)
    let anahtar = std::fs::read_to_string(k.join("anahtar")).unwrap();
    let apk2 = k.join("iki.apk");
    assert!(calistir(&[
        "paketle",
        "örnekler/arayüz/sayaç.ohc",
        "--hedef",
        "android",
        "-o",
        apk2.to_str().unwrap()
    ])
    .status
    .success());
    assert_eq!(std::fs::read_to_string(k.join("anahtar")).unwrap(), anahtar);

    // Arayüzü olmayan program reddedilir
    let c = calistir(&[
        "paketle",
        "örnekler/merhaba.ohc",
        "--hedef",
        "android",
        "-o",
        k.join("x.apk").to_str().unwrap(),
    ]);
    assert!(!c.status.success());
    assert!(String::from_utf8_lossy(&c.stderr).contains("arayüz"));

    if let Some(sdk) = std::env::var_os("ANDROID_HOME") {
        let araclar = std::fs::read_dir(Path::new(&sdk).join("build-tools"))
            .ok()
            .and_then(|d| d.filter_map(|g| g.ok()).map(|g| g.path()).max());
        if let Some(a) = araclar {
            let c = Command::new(a.join("apksigner"))
                .args(["verify", "--verbose"])
                .arg(&apk)
                .output()
                .unwrap();
            let m = String::from_utf8_lossy(&c.stdout);
            assert!(
                c.status.success() && m.contains("v2 scheme (APK Signature Scheme v2): true"),
                "{m}{}",
                String::from_utf8_lossy(&c.stderr)
            );
            let c = Command::new(a.join("aapt2"))
                .args(["dump", "badging"])
                .arg(&apk)
                .output()
                .unwrap();
            let m = String::from_utf8_lossy(&c.stdout);
            assert!(
                m.contains("package: name='org.orhunca.sayac'")
                    && m.contains("application-label:'Sayaç'"),
                "{m}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn ios_projesi() {
    let k = std::env::temp_dir().join(format!("orhunca-ios-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&k);
    let c = Command::new(env!("CARGO_BIN_EXE_orhunca"))
        .args([
            "paketle",
            "örnekler/arayüz/sınıf_defteri.ohc",
            "--hedef",
            "ios",
            "-o",
        ])
        .arg(&k)
        .output()
        .unwrap();
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let proje = std::fs::read_to_string(k.join("Uygulama.xcodeproj/project.pbxproj")).unwrap();
    // Apple paket kimliğinde alt çizgi olamaz
    assert!(
        proje.contains("PRODUCT_BUNDLE_IDENTIFIER = \"org.orhunca.sinif-defteri\";"),
        "{proje}"
    );
    assert!(!proje.contains("{{"));
    let plist = std::fs::read_to_string(k.join("Uygulama/Info.plist")).unwrap();
    assert!(plist.contains("<string>Sınıf defteri</string>"));
    for f in [
        "Uygulama/uygulama.html",
        "Uygulama/Assets.xcassets/AppIcon.appiconset/simge.png",
        ".github/workflows/ios.yml",
        "BENİOKU.md",
    ] {
        assert!(k.join(f).is_file(), "{f}");
    }
    let _ = std::fs::remove_dir_all(&k);
}
