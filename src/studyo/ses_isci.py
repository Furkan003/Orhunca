"""Stüdyo'nun isteğe bağlı EMA LiteRT kurucusu ve kalıcı ses işçisi."""
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.request
import wave
from collections import OrderedDict

SURUM = "4720c1c02f8cbf3835189264bbf468378b16e8c5"
PAKETLER = ["ai-edge-litert==2.2.0", "normalizer-tr==0.7.0", "numpy==2.2.6",
            "backports-strenum==1.2.8", "flatbuffers==25.12.19", "ml-dtypes==0.6.0",
            "protobuf==7.36.2", "tqdm==4.70.1", "typing-extensions==4.16.0"]


def durum_yaz(metin, yuzde):
    print(json.dumps({"ileti": metin, "yuzde": yuzde}, ensure_ascii=False), flush=True)


def dosya_dogru(yol, bilgi):
    if not yol.is_file() or yol.stat().st_size != bilgi["bytes"]:
        return False
    with yol.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest() == bilgi["sha256"]


def modelleri_dogrula(kok):
    ayar = json.loads((kok / "config.json").read_text(encoding="utf-8"))
    for bilgi in ayar["files"]:
        if not dosya_dogru(kok / bilgi["file"], bilgi):
            raise ValueError("Ses modeli eksik veya bozulmuş; ses paketini yeniden kurun.")
    return ayar


def kur(kok):
    if not (3, 11) <= sys.version_info[:2] <= (3, 13):
        raise ValueError("Ses paketi için Python 3.11–3.13 gerekiyor.")
    (kok / "hazir.json").unlink(missing_ok=True)
    ortam = kok / "ortam"
    python = ortam / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    ortam_degiskenleri = dict(os.environ, PIP_CACHE_DIR=str(kok / "pip-onbellek"),
                             PIP_DISABLE_PIP_VERSION_CHECK="1", PYTHONNOUSERSITE="1")
    # Kullanıcının pip ayarı başka bir kaynağa yönlendirmesin; TLS doğrulaması korunur.
    ortam_degiskenleri["PIP_CONFIG_FILE"] = os.devnull
    for ad in ("PIP_INDEX_URL", "PIP_EXTRA_INDEX_URL", "PIP_TRUSTED_HOST"):
        ortam_degiskenleri.pop(ad, None)
    durum_yaz("Ses çalışma ortamı hazırlanıyor", 2)
    subprocess.run([sys.executable, "-m", "venv", str(ortam)], check=True,
                   stdout=sys.stderr, stderr=sys.stderr, timeout=120)
    durum_yaz("LiteRT çalışma motoru indiriliyor; toplam kurulum modelden büyüktür", 10)
    subprocess.run([str(python), "-m", "pip", "install", "--only-binary=:all:",
                    "--index-url", "https://pypi.org/simple", *PAKETLER],
                   env=ortam_degiskenleri, check=True, stdout=sys.stderr,
                   stderr=sys.stderr, timeout=600)
    ayar = json.loads((kok / "config.json").read_text(encoding="utf-8"))
    toplam = sum(f["bytes"] for f in ayar["files"])
    tamamlanan = 0
    for bilgi in ayar["files"]:
        yol = kok / bilgi["file"]
        if not dosya_dogru(yol, bilgi):
            gecici = yol.with_suffix(yol.suffix + ".parca")
            try:
                adres = ("https://huggingface.co/litert-community/EMA-Lightning-LiteRT/resolve/"
                         + SURUM + "/" + bilgi["file"])
                with urllib.request.urlopen(adres, timeout=30) as kaynak, gecici.open("wb") as hedef:
                    indirilen = 0
                    while True:
                        parca = kaynak.read(256 * 1024)
                        if not parca:
                            break
                        indirilen += len(parca)
                        if indirilen > bilgi["bytes"]:
                            raise ValueError("Ses dosyası beklenen boyutu aşıyor.")
                        hedef.write(parca)
                        durum_yaz("Türkçe model indiriliyor", 35 + int(60 * (tamamlanan + indirilen) / toplam))
                    hedef.flush()
                    os.fsync(hedef.fileno())
                if not dosya_dogru(gecici, bilgi):
                    raise ValueError("Ses dosyasının SHA-256 doğrulaması başarısız.")
                os.replace(gecici, yol)
            finally:
                gecici.unlink(missing_ok=True)
        tamamlanan += bilgi["bytes"]
    modelleri_dogrula(kok)
    durum_yaz("Türkçe ses denetleniyor", 97)
    subprocess.run([str(python), str(Path(__file__).resolve()), "dene"],
                   stdout=sys.stderr, stderr=sys.stderr, timeout=90)
    (kok / "hazir.json").write_text(json.dumps({"surum": SURUM}), encoding="utf-8")
    durum_yaz("EMA Türkçe ses paketi hazır", 100)


def motor_ac(kok):
    modelleri_dogrula(kok)
    sys.path.insert(0, str(kok))
    from ema_host import EmaLiteRT
    return EmaLiteRT(threads=min(2, os.cpu_count() or 1))


def wav_yap(ses):
    import numpy as np
    pcm = (np.clip(ses, -1, 1) * 32767).round().astype("<i2")
    bellek = io.BytesIO()
    with wave.open(bellek, "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(48000)
        f.writeframes(pcm.tobytes())
    return bellek.getvalue()


def calis(kok):
    motor = motor_ac(kok)
    # Küçük kovaları bir kez hazırla; metin veya ses diske kaydedilmez.
    motor.say("Merhaba.", seed=42)
    onbellek = OrderedDict()
    boyut = 0
    for satir in sys.stdin:
        try:
            istek = json.loads(satir)
            metin = istek.get("metin", "")
            hiz = istek.get("hiz", 1.0)
            if not isinstance(metin, str) or not metin.strip() or len(metin) > 1200:
                raise ValueError("Okunacak metin 1–1200 karakter olmalı.")
            if isinstance(hiz, bool) or not isinstance(hiz, (float, int)) or not 0.75 <= hiz <= 1.5:
                raise ValueError("Ses hızı 0,75 ile 1,5 arasında olmalı.")
            anahtar = (metin, hiz)
            baslangic = time.perf_counter()
            veri = onbellek.pop(anahtar, None)
            if veri is None:
                ses, _ = motor.say(metin, seed=42, speed=hiz)
                veri = wav_yap(ses)
                if len(veri) > 16 * 1024 * 1024:
                    raise ValueError("Üretilen ses çok uzun; daha kısa metin deneyin.")
                boyut += len(veri)
            onbellek[anahtar] = veri
            while boyut > 32 * 1024 * 1024 or len(onbellek) > 48:
                _, eski = onbellek.popitem(last=False)
                boyut -= len(eski)
            yanit = {"bayt": len(veri), "sure_ms": round((time.perf_counter() - baslangic) * 1000)}
        except Exception:
            # Proje kodu ve özel metin hata kaydına düşmez.
            veri = b""
            yanit = {"hata": "EMA ses üretemedi; ses paketini yeniden kurmayı deneyin.", "bayt": 0}
        sys.stdout.buffer.write((json.dumps(yanit) + "\n").encode())
        sys.stdout.buffer.write(veri)
        sys.stdout.buffer.flush()


def main():
    kok = Path(__file__).resolve().parent
    try:
        if sys.argv[1] == "kur":
            kur(kok)
        elif sys.argv[1] == "dene":
            motor = motor_ac(kok)
            ses, _ = motor.say("Merhaba, Orhunca.", seed=42)
            if len(ses) == 0:
                raise ValueError("Ses denemesi boş sonuç verdi.")
        elif sys.argv[1] == "calis":
            calis(kok)
        else:
            raise ValueError("Bilinmeyen ses işlemi.")
    except Exception as e:
        mesaj = str(e) if isinstance(e, ValueError) else "Ses paketi kurulamadı. Python sürümünü, internet erişimini ve boş disk alanını kontrol edin."
        durum_yaz(mesaj, 0)
        sys.exit(1)


if __name__ == "__main__":
    main()
