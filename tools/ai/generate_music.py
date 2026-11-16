#!/usr/bin/env python3
"""Sample Honeydew's event Transformer and export editable JSON + MIDI."""
import argparse
import json
import random
from pathlib import Path

import torch
import mido

from train_music import (
    GPT, BOS, EOS, STYLES, TRACKS, BAR_BASE, STEP_BASE,
    PITCH_BASE, VEL_BASE, DUR_BASE, encode_song,
)

TRACK_NAMES = {value: key for key, value in TRACKS.items()}


def choose_next(logits, allowed, temperature, top_k):
    scores = logits[allowed].float() / max(0.05, temperature)
    if top_k > 0 and scores.numel() > top_k:
        values, indices = torch.topk(scores, top_k)
        probs = torch.softmax(values, dim=-1)
        selected = indices[torch.multinomial(probs, 1)].item()
        return allowed[selected]
    probs = torch.softmax(scores, dim=-1)
    selected = torch.multinomial(probs, 1).item()
    return allowed[selected]


def generate_tokens(model, device, style, bars, max_events, temperature, top_k, seed, prompt=None):
    random.seed(seed)
    torch.manual_seed(seed)
    style_id = STYLES.get(style, STYLES["other"])
    prefix = [BOS, style_id]
    if prompt:
        prefix = encode_song(prompt)[:-1]
    tokens = list(prefix)
    # The first token of each event is a track. The boundary prediction can
    # become the next event's track directly, avoiding a duplicated field.
    pending_track = None
    for _ in range(max_events):
        if pending_track is None:
            context = torch.tensor([tokens[-model.block:]], dtype=torch.long, device=device)
            with torch.no_grad():
                logits = model(context)[0, -1]
            pending_track = choose_next(logits, list(TRACK_NAMES), temperature, top_k)
        tokens.append(pending_track)
        pending_track = None
        fields = [
            list(range(BAR_BASE, BAR_BASE + bars)),
            list(range(STEP_BASE, STEP_BASE + 16)),
            list(range(PITCH_BASE, PITCH_BASE + 128)),
            list(range(VEL_BASE, VEL_BASE + 16)),
            list(range(DUR_BASE, DUR_BASE + 16)),
        ]
        for allowed in fields:
            context = torch.tensor([tokens[-model.block:]], dtype=torch.long, device=device)
            with torch.no_grad():
                logits = model(context)[0, -1]
            tokens.append(choose_next(logits, allowed, temperature, top_k))
        context = torch.tensor([tokens[-model.block:]], dtype=torch.long, device=device)
        with torch.no_grad():
            logits = model(context)[0, -1]
        boundary = choose_next(logits, [EOS, *TRACK_NAMES], temperature, top_k)
        if boundary == EOS:
            tokens.append(EOS)
            break
        pending_track = boundary
    return tokens


def decode_events(tokens):
    events = []
    i = 2
    while i + 5 < len(tokens):
        track = tokens[i]
        if track not in TRACK_NAMES:
            i += 1
            continue
        try:
            bar = tokens[i + 1] - BAR_BASE
            step = tokens[i + 2] - STEP_BASE
            pitch = tokens[i + 3] - PITCH_BASE
            velocity = min(127, (tokens[i + 4] - VEL_BASE) * 8 + 4)
            duration = tokens[i + 5] - DUR_BASE + 1
            if 0 <= bar < 16 and 0 <= step < 16 and 0 <= pitch < 128:
                events.append({
                    "track": TRACK_NAMES[track], "bar": bar, "step": step,
                    "pitch": pitch, "velocity": max(1, velocity),
                    "duration": max(1, duration),
                })
        except (IndexError, TypeError):
            pass
        i += 6
    return events


def write_midi(song, path):
    midi = mido.MidiFile(ticks_per_beat=480)
    tempo = mido.bpm2tempo(song["bpm"])
    by_track = {name: [] for name in TRACKS}
    for event in song["events"]:
        tick = (event["bar"] * 16 + event["step"]) * 120
        end = tick + event["duration"] * 120
        channel = 9 if event["track"] == "drums" else {
            "bass": 1, "chords": 2, "lead": 3
        }[event["track"]]
        by_track[event["track"]].append((tick, True, event, channel))
        by_track[event["track"]].append((end, False, event, channel))
    for name, items in by_track.items():
        track = mido.MidiTrack()
        midi.tracks.append(track)
        track.append(mido.MetaMessage("track_name", name=name, time=0))
        if not track:  # pragma: no cover - track always has its name
            continue
        if name == "drums" or (name == "bass" and not by_track["drums"]):
            track.append(mido.MetaMessage("set_tempo", tempo=tempo, time=0))
        items.sort(key=lambda item: (item[0], item[1]))
        previous = 0
        for tick, is_on, event, channel in items:
            message = mido.Message(
                "note_on" if is_on else "note_off",
                channel=channel, note=event["pitch"],
                velocity=event["velocity"] if is_on else 0,
                time=max(0, tick - previous),
            )
            track.append(message)
            previous = tick
    midi.save(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("--out", default="honeydew-generated")
    parser.add_argument("--style", choices=sorted(STYLES), default="trap")
    parser.add_argument("--bars", type=int, default=8)
    parser.add_argument("--bpm", type=int, default=145)
    parser.add_argument("--max-events", type=int, default=180)
    parser.add_argument("--temperature", type=float, default=0.9)
    parser.add_argument("--top-k", type=int, default=12)
    parser.add_argument("--seed", type=int, default=123)
    parser.add_argument("--prompt-json", help="Optional existing song JSON to continue")
    args = parser.parse_args()
    if not 1 <= args.bars <= 16 or not 40 <= args.bpm <= 240 or args.max_events < 1:
        parser.error("bars must be 1..16, BPM 40..240, and max-events positive")
    device = "cuda" if torch.cuda.is_available() else "cpu"
    checkpoint = torch.load(args.checkpoint, map_location=device, weights_only=False)
    config = checkpoint["config"]
    model = GPT(config["block"], config["width"], config["layers"], config["heads"]).to(device)
    model.load_state_dict(checkpoint["model"])
    model.eval()
    prompt = None
    if args.prompt_json:
        prompt = json.loads(Path(args.prompt_json).read_text(encoding="utf-8"))
        prompt.setdefault("style", args.style)
        if not isinstance(prompt.get("events"), list):
            parser.error("prompt JSON must contain an events array")
    tokens = generate_tokens(model, device, args.style, args.bars, args.max_events,
                             args.temperature, args.top_k, args.seed, prompt)
    events = decode_events(tokens)
    if not events:
        raise SystemExit("Model generated no decodable events; train longer or adjust sampling.")
    song = {"style": args.style, "bpm": args.bpm, "bars": args.bars, "events": events}
    base = Path(args.out)
    base.parent.mkdir(parents=True, exist_ok=True)
    json_path = base.with_suffix(".json")
    midi_path = base.with_suffix(".mid")
    json_path.write_text(json.dumps(song, ensure_ascii=False, indent=2), encoding="utf-8")
    write_midi(song, midi_path)
    print(f"Generated {len(events)} editable events")
    print(f"JSON: {json_path}")
    print(f"MIDI: {midi_path}")


if __name__ == "__main__":
    main()
