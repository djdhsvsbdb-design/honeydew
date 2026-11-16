#!/usr/bin/env python3
"""Download a verified, rights-filtered MIDI subset from the PDMX Zenodo release."""
import argparse
import csv
import hashlib
import json
import tarfile
import urllib.request
from pathlib import Path, PurePosixPath

BASE = "https://zenodo.org/records/14984509/files"
CSV_URL = BASE + "/PDMX.csv?download=1"
MIDI_URL = BASE + "/mid.tar.gz?download=1"
CSV_MD5 = "106d194fa9401e32402fbf8b17936ead"
MIDI_MD5 = "d920a21b2fcd99a56d9c381b39debbb2"
SOURCE_URL = "https://zenodo.org/records/14984509"
ATTRIBUTION = (
    "PDMX dataset, Phillip Long, Zachary Novack, Taylor Berg-Kirkpatrick, "
    "Julian McAuley; public-domain/CC0 subset; https://zenodo.org/records/14984509"
)


def checksum(path):
    digest = hashlib.md5()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url, path, expected_md5):
    if not path.exists():
        print(f"Downloading {path.name}...")
        urllib.request.urlretrieve(url, path)
    actual = checksum(path)
    if actual != expected_md5:
        raise SystemExit(f"MD5 mismatch for {path.name}: expected {expected_md5}, got {actual}")


def truth(value):
    return str(value).strip().lower() in ("true", "1", "yes")


def safe_member_name(value):
    name = str(value).strip().replace("\\", "/")
    while name.startswith("./"):
        name = name[2:]
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"unsafe archive path: {value!r}")
    return path.as_posix()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out-dir", type=Path, default=Path("data/ai/external/pdmx"))
    ap.add_argument("--limit", type=int, default=10000)
    args = ap.parse_args()
    if args.limit < 1:
        ap.error("--limit must be >= 1")
    args.out_dir.mkdir(parents=True, exist_ok=True)
    csv_path = args.out_dir / "PDMX.csv"
    archive = args.out_dir / "mid.tar.gz"
    download(CSV_URL, csv_path, CSV_MD5)
    download(MIDI_URL, archive, MIDI_MD5)

    selected = {}
    with csv_path.open("r", encoding="utf-8-sig", newline="") as handle:
        reader = csv.DictReader(handle)
        required = {"license", "license_conflict", "mid"}
        if not required.issubset(reader.fieldnames or []):
            raise SystemExit(f"PDMX.csv is missing required columns: {sorted(required)}")
        for row in reader:
            license_name = str(row.get("license", "")).strip().lower().replace("-", "").replace("_", "")
            if license_name not in ("publicdomain", "cc0", "cc01.0"):
                continue
            if truth(row.get("license_conflict", "false")):
                continue
            if "is_best_unique_arrangement" in row and not truth(row["is_best_unique_arrangement"]):
                continue
            raw_mid = row.get("mid", "")
            if not raw_mid or raw_mid.strip().lower() in ("na", "n/a", "nan"):
                continue
            member_name = safe_member_name(raw_mid)
            genre = str(row.get("genres", "")).lower()
            style = "hiphop" if "hiphop" in genre or "hip-hop" in genre else "other"
            selected[member_name] = style
            if len(selected) >= args.limit:
                break

    if not selected:
        raise SystemExit("No eligible public-domain/CC0 MIDI files found in PDMX.csv")

    midi_root = args.out_dir / "midi"
    midi_root.mkdir(parents=True, exist_ok=True)
    manifest = args.out_dir / "external-manifest.jsonl"
    extracted = 0
    with tarfile.open(archive, "r:gz") as tar, manifest.open("w", encoding="utf-8") as out:
        for member in tar:
            if not member.isfile():
                continue
            name = safe_member_name(member.name)
            if name not in selected:
                continue
            relative = PurePosixPath(name)
            destination = midi_root.joinpath(*relative.parts[1:]) if relative.parts[0] == "mid" else midi_root / relative.name
            destination.parent.mkdir(parents=True, exist_ok=True)
            source = tar.extractfile(member)
            if source is None:
                continue
            with source, destination.open("wb") as target:
                while True:
                    chunk = source.read(1024 * 1024)
                    if not chunk:
                        break
                    target.write(chunk)
            out.write(json.dumps({
                "path": str(destination.resolve()),
                "style": selected[name],
                "license": "CC0 / Public Domain",
                "training_allowed": True,
                "source_name": "PDMX public-domain/CC0 subset",
                "source_url": SOURCE_URL,
                "attribution": ATTRIBUTION,
            }, ensure_ascii=False) + "\n")
            extracted += 1
            if extracted >= args.limit:
                break

    print(f"Verified PDMX archives. Imported {extracted} eligible MIDI files.")
    print(f"Manifest: {manifest}")
    if extracted < min(args.limit, len(selected)):
        print("Warning: fewer MIDI files were present in the archive than in the metadata selection.")


if __name__ == "__main__":
    main()
