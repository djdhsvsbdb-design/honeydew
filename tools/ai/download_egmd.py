#!/usr/bin/env python3
"""Download Google's official E-GMD MIDI-only archive and write a reviewed manifest."""
import argparse
import csv
import hashlib
import json
import urllib.request
import zipfile
from pathlib import Path

URL = "https://storage.googleapis.com/magentadata/datasets/e-gmd/v1.0.0/e-gmd-v1.0.0-midi.zip"
SHA256 = "5e70a6f4d760385a5e5ec986a2f02179d96f61181a920e592876b577a75844d3"
SOURCE = "Expanded Groove MIDI Dataset v1.0.0"
ATTRIBUTION = (
    "Google LLC, Expanded Groove MIDI Dataset v1.0.0, "
    "https://magenta.withgoogle.com/datasets/e-gmd, CC BY 4.0"
)


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out-dir", type=Path, default=Path("data/ai/external/egmd"))
    ap.add_argument("--limit", type=int, default=10000)
    args = ap.parse_args()
    if args.limit < 1:
        ap.error("--limit must be >= 1")
    args.out_dir.mkdir(parents=True, exist_ok=True)
    archive = args.out_dir / "e-gmd-v1.0.0-midi.zip"
    if not archive.exists():
        print("Downloading official E-GMD MIDI-only archive (about 103 MB)...")
        urllib.request.urlretrieve(URL, archive)
    actual = digest(archive)
    if actual != SHA256:
        raise SystemExit(f"SHA-256 mismatch: expected {SHA256}, got {actual}")

    extract_dir = args.out_dir / "unzipped"
    extract_dir.mkdir(exist_ok=True)
    with zipfile.ZipFile(archive) as zf:
        zf.extractall(extract_dir)

    midi_files = sorted(
        p for p in extract_dir.rglob("*")
        if p.is_file() and p.suffix.lower() in (".mid", ".midi")
    )
    if not midi_files:
        raise SystemExit("No MIDI files found in the verified archive")

    manifest = args.out_dir / "external-manifest.jsonl"
    count = 0
    with manifest.open("w", encoding="utf-8") as f:
        for midi in midi_files[:args.limit]:
            row = {
                "path": str(midi.resolve()),
                # E-GMD is a human drum-performance corpus, not a labelled
                # Hip-Hop/Trap/Drill/Phonk/House song collection.
                "style": "other",
                "license": "CC BY 4.0",
                "training_allowed": True,
                "source_name": SOURCE,
                "source_url": "https://magenta.withgoogle.com/datasets/e-gmd",
                "attribution": ATTRIBUTION,
            }
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
            count += 1
    print(f"Verified archive SHA-256: {actual}")
    print(f"Prepared {count} MIDI entries: {manifest}")
    print("Note: E-GMD supplies drum performances; it does not provide full multi-track songs.")


if __name__ == "__main__":
    main()
