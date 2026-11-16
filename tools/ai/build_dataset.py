#!/usr/bin/env python3
"""Build a mixed Honeydew dataset from original synthetic songs and licensed MIDI.

External MIDI is never downloaded or accepted blindly: every file must have a
manifest row documenting its source, license, attribution, and training permission.
"""
import argparse
import hashlib
import json
import random
from pathlib import Path

from generate_synthetic import make_song
from midi_to_jsonl import convert

ALLOWED_LICENSE_MARKERS = ("CC0", "CC-BY-4.0", "CC BY 4.0", "PUBLIC-DOMAIN")


def load_manifest(path):
    rows = []
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path}:{line_no}: invalid JSON: {exc}") from exc
        required = ("path", "style", "license", "source_url", "source_name", "attribution")
        missing = [key for key in required if not row.get(key)]
        if missing:
            raise ValueError(f"{path}:{line_no}: missing fields: {', '.join(missing)}")
        if row.get("training_allowed") is not True:
            raise ValueError(f"{path}:{line_no}: training_allowed must be true")
        license_name = str(row["license"]).upper()
        if not any(marker in license_name for marker in ALLOWED_LICENSE_MARKERS):
            raise ValueError(
                f"{path}:{line_no}: license {row['license']!r} is not on the "
                "automatic allowlist; review permissions and extend the script deliberately"
            )
        if row["style"] not in ("hiphop", "trap", "drill", "phonk", "house", "other"):
            raise ValueError(f"{path}:{line_no}: unsupported style {row['style']!r}")
        rows.append(row)
    return rows


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--synthetic-songs", type=int, default=10000)
    parser.add_argument("--external-manifest", type=Path, required=True)
    parser.add_argument("--external-limit", type=int, default=10000)
    parser.add_argument("--out", type=Path, default=Path("data/ai/honeydew-20k.jsonl"))
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--allow-shortfall", action="store_true",
                        help="write fewer than the requested external count if the manifest is smaller")
    args = parser.parse_args()
    if args.synthetic_songs < 1 or args.external_limit < 0:
        parser.error("synthetic-songs must be >= 1 and external-limit must be >= 0")

    rows = load_manifest(args.external_manifest)
    rng = random.Random(args.seed)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    seen_hashes = set()
    synthetic_count = external_count = skipped = 0

    with args.out.open("w", encoding="utf-8") as output:
        for index in range(args.synthetic_songs):
            song = make_song(rng, index)
            song.update({"source_type": "synthetic", "source_name": "Honeydew procedural generator"})
            output.write(json.dumps(song, ensure_ascii=False, separators=(",", ":")) + "\n")
            synthetic_count += 1

        for row in rows:
            if external_count >= args.external_limit:
                break
            midi_path = Path(row["path"]).expanduser()
            if not midi_path.is_file():
                print(f"skip missing MIDI: {midi_path}")
                skipped += 1
                continue
            digest = sha256(midi_path)
            if digest in seen_hashes:
                skipped += 1
                continue
            seen_hashes.add(digest)
            try:
                song = convert(midi_path, row["style"])
            except Exception as exc:
                print(f"skip invalid MIDI {midi_path}: {exc}")
                skipped += 1
                continue
            if not song:
                skipped += 1
                continue
            song.update({
                "source_type": "licensed_external",
                "source_name": row["source_name"],
                "source_url": row["source_url"],
                "license": row["license"],
                "attribution": row["attribution"],
                "source_sha256": digest,
            })
            output.write(json.dumps(song, ensure_ascii=False, separators=(",", ":")) + "\n")
            external_count += 1

    print(f"synthetic={synthetic_count}, external={external_count}, skipped={skipped}")
    print(f"dataset: {args.out}")
    if external_count < args.external_limit and not args.allow_shortfall:
        raise SystemExit(
            f"Only {external_count}/{args.external_limit} licensed external MIDI files were "
            "usable. Add more reviewed manifest rows or pass --allow-shortfall."
        )


if __name__ == "__main__":
    main()
