#!/usr/bin/env python3
"""Stüdyo'nun yazı tiplerini Google Fonts'tan indirir (yalnızca Latin, Latin-ext ve
gereken simgeler) ve studio/yazitipleri/ altına yerel bir CSS ile koyar.

Simge eklemek için SIMGELER listesini güncelleyip yeniden çalıştırın:
    python3 araclar/yazitipi_indir.py
"""
import os
import re
import subprocess

KLASOR = os.path.join(os.path.dirname(__file__), "..", "studio", "yazitipleri")
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36"

SIMGELER = sorted(set("""
account_tree add arrow_back arrow_forward arrow_upward calculate check check_circle
chevron_right close cloud_download content_copy crop_square data_object delete
deployed_code description desktop_windows dns draft error expand_more extension folder
folder_open fork_right grid_view home image info keyboard_return language lock
menu_book more_horiz new_releases note_add palette play_arrow play_circle refresh remove
rocket_launch save school search settings sports_esports stacks stop swap_vert terminal
tune warning web code bug_report history open_in_new javascript html css visibility visibility_off
redo arrow_downward pause task_alt select_window slow_motion_video school edit
download upload_file smart_toy auto_awesome send stop_circle key add_comment key_off edit_document done smartphone translate
""".split()))

# (dosya adı öneki, Google Fonts sorgusu, alınacak alt kümeler)
AILELER = [
    ("ibm-plex-sans", "family=IBM+Plex+Sans:wght@400;500;600", {"latin", "latin-ext"}),
    ("jetbrains-mono", "family=JetBrains+Mono:wght@400;500", {"latin", "latin-ext"}),
    ("noto-sans-old-turkic", "family=Noto+Sans+Old+Turkic", {"old-turkic"}),
    ("material-symbols-rounded",
     "family=Material+Symbols+Rounded:opsz,wght,FILL,GRAD@20..48,300..500,0..1,0&icon_names=" + ",".join(SIMGELER),
     {"fallback", None}),
]


def indir(url):
    return subprocess.run(["curl", "-sSf", "-A", UA, url], check=True, capture_output=True).stdout


def main():
    os.makedirs(KLASOR, exist_ok=True)
    for f in os.listdir(KLASOR):
        if f.endswith(".woff2"):
            os.remove(os.path.join(KLASOR, f))
    cikti = ["/* Bu dosya araclar/yazitipi_indir.py ile üretilir; elle düzenlemeyin. */"]
    for ad, sorgu, istenen in AILELER:
        css = indir(f"https://fonts.googleapis.com/css2?{sorgu}&display=block").decode()
        # Her @font-face bloğunun önünde /* altküme */ yorumu bulunur (simge yazı tipinde yok).
        bloklar = re.findall(r"(?:/\* ([a-z-]+) \*/\s*)?(@font-face \{.*?\})", css, re.S)
        sira = 0
        for altkume, blok in bloklar:
            if (altkume or None) not in istenen:
                continue
            url = re.search(r"url\((https://[^)]+)\)", blok).group(1)
            agirlik = re.search(r"font-weight: ([0-9 ]+);", blok).group(1).replace(" ", "-")
            dosya = f"{ad}-{agirlik}-{altkume or 'tam'}-{sira}.woff2"
            sira += 1
            with open(os.path.join(KLASOR, dosya), "wb") as f:
                f.write(indir(url))
            cikti.append(blok.replace(url, dosya))
    with open(os.path.join(KLASOR, "yazitipleri.css"), "w") as f:
        f.write("\n".join(cikti) + "\n")
    toplam = sum(os.path.getsize(os.path.join(KLASOR, f)) for f in os.listdir(KLASOR))
    print(f"{len(cikti) - 1} yazı tipi dosyası, toplam {toplam // 1024} KB")


if __name__ == "__main__":
    main()
