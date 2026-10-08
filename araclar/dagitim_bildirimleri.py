#!/usr/bin/env python3
"""Paket yöneticisi bildirimlerini (Scoop, Homebrew, winget) bir sürüm için üretir.

    python3 araclar/dagitim_bildirimleri.py v0.9.0

Sürümün SHA256SUMS.txt dosyası GitHub'dan indirilir; şunlar yazılır:
- bucket/orhunca.json            Scoop: scoop bucket add orhunca https://github.com/Furkan003/Orhunca
- Formula/orhunca.rb             Homebrew: brew tap furkan003/orhunca https://github.com/Furkan003/Orhunca
- kurulum/winget/<sürüm>/        winget bildirimi (microsoft/winget-pkgs deposuna gönderilir)
Bu yollar paket yöneticilerinin beklediği yerlerdir; adları değiştirilmemeli.
"""
import os
import sys
import urllib.request

KOK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
DEPO = "Furkan003/Orhunca"
WINGET_KIMLIK = "Orhunca.OrhuncaStudyo"


def ozetler(surum):
    adres = f"https://github.com/{DEPO}/releases/download/{surum}/SHA256SUMS.txt"
    with urllib.request.urlopen(adres, timeout=60) as y:
        metin = y.read().decode()
    sonuc = {}
    for satir in metin.splitlines():
        if satir.strip():
            ozet, ad = satir.split(None, 1)
            sonuc[ad.strip().lstrip("*")] = ozet
    return sonuc


def yaz(yol, icerik):
    yol = os.path.join(KOK, yol)
    os.makedirs(os.path.dirname(yol), exist_ok=True)
    with open(yol, "w", encoding="utf-8", newline="\n") as f:
        f.write(icerik)
    print("yazıldı:", os.path.relpath(yol, KOK))


def indirme(surum, ad):
    return f"https://github.com/{DEPO}/releases/download/{surum}/{ad}"


def scoop(surum, s):
    sayi = surum.lstrip("v")
    return f"""{{
    "version": "{sayi}",
    "description": "Orhunca: Türkçe programlama dili (orhunca komutu ve tarayıcıda açılan Orhunca Stüdyo)",
    "homepage": "https://furkan003.github.io/Orhunca/",
    "license": "MIT",
    "architecture": {{
        "64bit": {{
            "url": "{indirme(surum, 'orhunca-windows-x86_64.zip')}",
            "hash": "{s['orhunca-windows-x86_64.zip']}"
        }}
    }},
    "bin": "orhunca.exe",
    "shortcuts": [
        [
            "orhunca.exe",
            "Orhunca Stüdyo",
            "stüdyo"
        ]
    ],
    "checkver": {{
        "github": "https://github.com/{DEPO}"
    }},
    "autoupdate": {{
        "architecture": {{
            "64bit": {{
                "url": "https://github.com/{DEPO}/releases/download/v$version/orhunca-windows-x86_64.zip",
                "hash": {{
                    "url": "https://github.com/{DEPO}/releases/download/v$version/SHA256SUMS.txt"
                }}
            }}
        }}
    }}
}}
"""


def homebrew(surum, s):
    sayi = surum.lstrip("v")
    return f"""# Homebrew: brew tap furkan003/orhunca https://github.com/{DEPO}
#           brew install orhunca
# Bu dosya üretilir: python3 araclar/dagitim_bildirimleri.py v{sayi}
class Orhunca < Formula
  desc "Türkçe programlama dili: derleyici, Orhunca Stüdyo ve dil sunucusu"
  homepage "https://furkan003.github.io/Orhunca/"
  version "{sayi}"
  license "MIT"

  on_macos do
    url "{indirme(surum, 'orhunca-macos.tar.gz')}"
    sha256 "{s['orhunca-macos.tar.gz']}"
  end

  on_linux do
    on_intel do
      url "{indirme(surum, 'orhunca-linux-x86_64.tar.gz')}"
      sha256 "{s['orhunca-linux-x86_64.tar.gz']}"
    end
    on_arm do
      url "{indirme(surum, 'orhunca-linux-aarch64.tar.gz')}"
      sha256 "{s['orhunca-linux-aarch64.tar.gz']}"
    end
  end

  def install
    bin.install "orhunca"
  end

  def caveats
    <<~EOS
      Orhunca Stüdyo'yu açmak için: orhunca stüdyo
      macOS'ta program derlemek için Xcode komut satırı araçları gerekir: xcode-select --install
    EOS
  end

  test do
    (testpath/"selam.ohc").write("\\"Merhaba\\"'yı yaz.\\n")
    assert_equal "Merhaba", shell_output("#{{bin}}/orhunca çalıştır selam.ohc").strip
  end
end
"""


def winget(surum, s):
    sayi = surum.lstrip("v")
    bas = f"PackageIdentifier: {WINGET_KIMLIK}\nPackageVersion: {sayi}\n"
    sema = "# yaml-language-server: $schema=https://aka.ms/winget-manifest.{tur}.1.6.0.schema.json\n\n"
    surum_dosyasi = sema.format(tur="version") + bas + "DefaultLocale: tr-TR\nManifestType: version\nManifestVersion: 1.6.0\n"
    kurulum = sema.format(tur="installer") + bas + f"""InstallerLocale: tr-TR
MinimumOSVersion: 10.0.17763.0
InstallerType: nullsoft
Scope: user
InstallModes:
- interactive
- silent
UpgradeBehavior: install
Commands:
- orhunca
FileExtensions:
- ohc
- ohcproj
Installers:
- Architecture: x64
  InstallerUrl: {indirme(surum, 'Orhunca-Studyo-Windows-Kurulum.exe')}
  InstallerSha256: {s['Orhunca-Studyo-Windows-Kurulum.exe'].upper()}
ManifestType: installer
ManifestVersion: 1.6.0
"""
    yerel = sema.format(tur="defaultLocale") + bas + f"""PackageLocale: tr-TR
Publisher: Orhunca
PublisherUrl: https://github.com/{DEPO}
PublisherSupportUrl: https://github.com/{DEPO}/issues
PackageName: Orhunca Stüdyo
PackageUrl: https://furkan003.github.io/Orhunca/
License: MIT
LicenseUrl: https://github.com/{DEPO}/blob/HEAD/LICENSE
ShortDescription: Türkçe programlama dili ve geliştirme ortamı
Description: Orhunca, anahtar kelimeleri Türkçe olan ve cümleleri Türkçe gibi kurulan bir programlama dilidir. Orhunca Stüdyo; düzenleyici, hata ayıklayıcı, dersler ve canlı önizleme içerir.
Tags:
- programlama-dili
- turkce
- egitim
- derleyici
ReleaseNotesUrl: https://github.com/{DEPO}/releases/tag/{surum}
ManifestType: defaultLocale
ManifestVersion: 1.6.0
"""
    k = f"kurulum/winget/{sayi}"
    return {
        f"{k}/{WINGET_KIMLIK}.yaml": surum_dosyasi,
        f"{k}/{WINGET_KIMLIK}.installer.yaml": kurulum,
        f"{k}/{WINGET_KIMLIK}.locale.tr-TR.yaml": yerel,
    }


def main():
    if len(sys.argv) != 2 or not sys.argv[1].startswith("v"):
        sys.exit("Kullanım: python3 araclar/dagitim_bildirimleri.py v0.9.0")
    surum = sys.argv[1]
    s = ozetler(surum)
    yaz("bucket/orhunca.json", scoop(surum, s))
    yaz("Formula/orhunca.rb", homebrew(surum, s))
    for yol, icerik in winget(surum, s).items():
        yaz(yol, icerik)


if __name__ == "__main__":
    main()
