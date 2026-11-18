#!/usr/bin/env python3
"""Generate/infill selected tracks in a MIDI file with MIDI-GPT.

Honeydew opens the resulting .mid directly (SONGS -> Import or Ctrl+O).
This is an optional desktop-side bridge; it does not bundle model weights.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Use a local MIDI-GPT checkpoint to generate selected MIDI tracks."
    )
    parser.add_argument("input", type=Path, help="Input .mid file; include empty tracks where you want new parts.")
    parser.add_argument("-o", "--output", type=Path, required=True, help="Output .mid file.")
    parser.add_argument("--model", default="prism_medium",
                        choices=("yellow_small", "yellow_medium", "prism_medium", "expressive_medium"))
    parser.add_argument("--model-dim", type=int, choices=(4, 8, 12, 16), default=8,
                        help="Context window in bars. yellow models support only 4 or 8.")
    parser.add_argument("--track", type=int, action="append", dest="tracks",
                        help="Zero-based track index to generate. Repeat to select several. Default: all tracks.")
    parser.add_argument("--temperature", type=float, default=1.0)
    parser.add_argument("--top-p", type=float, default=0.95)
    parser.add_argument("--seed", type=int, default=None)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.input.is_file():
        print(f"Input MIDI not found: {args.input}", file=sys.stderr)
        return 2
    if args.model.startswith("yellow") and args.model_dim not in (4, 8):
        print("yellow models support --model-dim 4 or 8.", file=sys.stderr)
        return 2
    if not 0 < args.temperature <= 3:
        print("--temperature must be greater than 0 and at most 3.", file=sys.stderr)
        return 2
    if not 0 < args.top_p <= 1:
        print("--top-p must be in (0, 1].", file=sys.stderr)
        return 2

    try:
        from midigpt import Score
        from midigpt.inference import (
            InferenceEngine, GenerationRequest, InferenceConfig, TrackPrompt,
        )
    except ImportError as exc:
        print('MIDI-GPT is not installed. Run: pip install "midigpt[inference]"',
              file=sys.stderr)
        print(f"Import error: {exc}", file=sys.stderr)
        return 2

    try:
        score = Score.from_midi(str(args.input))
        track_count = len(score.tracks)
        if track_count == 0:
            raise ValueError("The input MIDI contains no tracks.")
        selected = set(args.tracks if args.tracks is not None else range(track_count))
        invalid = sorted(i for i in selected if i < 0 or i >= track_count)
        if invalid:
            raise ValueError(f"Track index out of range: {invalid}; MIDI has {track_count} tracks.")

        prompts = []
        for index, track in enumerate(score.tracks):
            bar_count = len(track.bars)
            if index in selected:
                if bar_count == 0:
                    raise ValueError(f"Track {index} has no bars to generate.")
                prompts.append(TrackPrompt(id=index, bars=list(range(bar_count))))
            else:
                prompts.append(TrackPrompt(id=index, bars=[], ignore=True))

        print(f"Loading {args.model} (first run may download model weights)...", flush=True)
        engine = InferenceEngine.from_pretrained(args.model)
        config = InferenceConfig(
            temperature=args.temperature,
            top_p=args.top_p,
            model_dim=args.model_dim,
            mask_mode="attention",
        )
        request = GenerationRequest(tracks=prompts, config=config)
        result = engine.session(score, request).run()
        args.output.parent.mkdir(parents=True, exist_ok=True)
        result.to_midi(str(args.output))
        print(f"Generated MIDI saved to: {args.output}")
        print("Open this .mid in Honeydew with SONGS -> Import or Ctrl+O.")
        return 0
    except Exception as exc:
        print(f"MIDI-GPT generation failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
