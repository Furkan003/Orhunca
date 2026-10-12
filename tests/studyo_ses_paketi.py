"""Modelin boyutu ve SHA-256 özeti tutmayınca yerel ses yüklenmez."""
from pathlib import Path
import hashlib
import importlib.util
import json
import tempfile
import sys
sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location("ses_isci", Path(__file__).resolve().parents[1] / "src/studyo/ses_isci.py")
isci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(isci)
with tempfile.TemporaryDirectory() as tmp:
    kok = Path(tmp)
    veri = b"model-test-verisi"
    bilgi = {"file": "model.tflite", "bytes": len(veri), "sha256": hashlib.sha256(veri).hexdigest()}
    (kok / "config.json").write_text(json.dumps({"files": [bilgi]}))
    assert not isci.dosya_dogru(kok / bilgi["file"], bilgi)
    (kok / bilgi["file"]).write_bytes(veri)
    assert isci.modelleri_dogrula(kok)["files"][0]["bytes"] == len(veri)
    (kok / bilgi["file"]).write_bytes(b"x" * len(veri))
    try:
        isci.modelleri_dogrula(kok)
    except ValueError:
        pass
    else:
        raise AssertionError("Bozuk model kabul edildi")
    (kok / bilgi["file"]).write_bytes(veri + b"x")
    assert not isci.dosya_dogru(kok / bilgi["file"], bilgi)
print("Eksik dosya, yanlış boyut ve SHA-256 doğrulama senaryoları geçti.")
