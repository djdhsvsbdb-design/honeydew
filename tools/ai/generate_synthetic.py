#!/usr/bin/env python3
"""Create original procedural MIDI-like event sequences for Honeydew model experiments.

These are synthetic training examples, not recordings or human-composed songs.
"""
import argparse
import json
import random
from pathlib import Path

STYLES = ("hiphop", "trap", "drill", "phonk", "house")
SCALES = {
    "minor": [0, 2, 3, 5, 7, 8, 10],
    "major": [0, 2, 4, 5, 7, 9, 11],
}
ROOTS = [36, 38, 40, 41, 43, 45, 47, 48]

def choose(rng, values):
    return values[rng.randrange(len(values))]

def make_song(rng, index):
    style = choose(rng, STYLES)
    root = choose(rng, ROOTS)
    scale = choose(rng, list(SCALES.values()))
    bars = choose(rng, [4, 4, 4, 8])
    bpm_ranges = {
        "hiphop": (78, 100), "trap": (130,  ik := 160),
        "drill": (138, 152), "phonk": (140, 170), "house": (118, 130)
    }
    bpm = rng.randint(*bpm_ranges[style])
    events = []

    def add(track, bar, step, pitch, velocity, duration):
        events.append({
            "track": track, "bar": bar, "step": step, "pitch": pitch,
            "velocity": velocity, "duration": duration
        })

    # Four-on-the-floor house; syncopated hip-hop/trap/drill/phonk drums.
    for bar in range(bars):
        for step in range(16):
            if style == "house":
                if step in (0, 4, 8, 12):
                    add("drums", bar, step, 36, rng.randint(92, 118), 1)
                if step in (4, 12):
                    add("drums", bar, step, 38, rng.randint(75, 100), 1)
                if step % 2 == 0 or (step in (3, 7, 11, 15) and rng.random() < .45):
                    add("drums", bar, step, 42, rng.randint(45, 86), 1)
                if step in (6, 14) and rng.random() < .45:
                    add("drums", bar, step, 46, rng.randint(55, 85), 2)
            else:
                if step in (0, 8) and rng.random() < .92:
                    add("drums", bar, step, 36, rng.randint(88, 120), 1)
                snare_steps = (4, 12) if style in ("hiphop", "phonk") else (4, 12)
                if step in snare_steps and rng.random() < .94:
                    add("drums", bar, step, 38, rng.randint(78, 112), 1)
                hat_prob = {"hiphop": .58, "trap": .72, "drill": .66, "phonk": .62}[style]
                if step % 2 == 0 and rng.random() < hat_prob:
                    add("drums", bar, step, 42, rng.randint(38, 82), 1)
                if style in ("trap", "drill") and step in (2, 3, 6, 7, 10, 11, 14, 15) and rng.random() < .24:
                    add("drums", bar, step, 42, rng.randint(28, 68), 1)
                if step in (7, 15) and rng.random() < .10:
                    add("drums", bar, step, 46, rng.randint(55, 90), 2)

    # Bass: roots and fifths, with style-specific rhythmic density.
    for bar in range(bars):
        root_pitch = root + choose(rng, [0, 0, 7, 12])
        steps = {
            "hiphop": [0, 6, 8, 14], "trap": [0, 3, 6, 8, 11, 14],
            "drill": [0, 5, 7, 10, 13], "phonk": [0, 4, 8, 12],
            "house": [0, 4, 8, 12]
        }[style]
        for step in steps:
            pitch = root_pitch if rng.random() < .78 else root_pitch + 7
            add("bass", bar, step, max(28, min(60, pitch)), rng.randint(78, 116), choose(rng, [2, 3, 4]))

    # Chord stabs/pads using scale tones.
    chord_roots = [0, 3, 5, 4] if scale == SCALES["minor"] else [0, 5, 3, 4]
    for bar in range(bars):
        degree = chord_roots[bar % len(chord_roots)]
        chord = [root + scale[degree], root + scale[(degree + 2) % 7], root + scale[(degree + 4) % 7]]
        chord_step = 0 if style == "house" else choose(rng, [0, 0, 4, 8])
        for pitch in chord:
            add("chords", bar, chord_step, min(90, pitch + 12), rng.randint(48, 82), 8 if style == "house" else 6)

    # Short melodic motifs; repeat with small variations so the model sees structure.
    motif = [choose(rng, [0, 2, 3, 4, 5, 7, 10]), choose(rng, [2, 3, 5, 7, 9]),
             choose(rng, [0, 3, 5, 7, 10]), choose(rng, [2, 4, 7, 9, 12])]
    for bar in range(bars):
        density = 3 if style in ("trap", "drill") else 2
        for j in range(density):
            step = choose(rng, [1, 3, 5, 7, 9, 11, 13, 15])
            interval = motif[(j + bar) % len(motif)]
            pitch = max(48, min(88, root + 24 + scale[interval % 7] + (12 if interval > 7 else 0)))
            if rng.random() < .22:
                pitch += choose(rng, [-12, 12])
            add("lead", bar, step, max(40, min(96, pitch)), rng.randint(45, 98), choose(rng, [1, 2, 3, 4]))

    events.sort(key=lambda e: (e["bar"], e["step"], e["track"], e["pitch"]))
    return {"id": index, "style": style, "bpm": bpm, "bars": bars, "events": events}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--songs", type=int, default=12000)
    ap.add_argument("--out", default="data/ai/synthetic.jsonl")
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.songs < 1:
        ap.error("--songs must be >= 1")
    rng = random.Random(args.seed)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as f:
        for i in range(args.songs):
            f.write(json.dumps(make_song(rng, i), separators=(",", ":")) + "\n")
    print(f"Created {args.songs} synthetic songs: {out}")

if __name__ == "__main__":
    main()
