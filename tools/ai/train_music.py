#!/usr/bin/env python3
"""Train Honeydew's autoregressive multi-track event Transformer.

JSONL contains one song per line. Splits are made by song before event chunking
so chunks from one song can never leak between train and validation.
"""
import argparse
import json
import random
from pathlib import Path

import torch
from torch import nn
from torch.utils.data import Dataset, DataLoader

VOCAB = 512
PAD, BOS, EOS = 0, 1, 2
STYLES = {"hiphop": 3, "trap": 4, "drill": 5, "phonk": 6, "house": 7, "other": 15}
STYLE_NAMES = {v: k for k, v in STYLES.items()}
TRACKS = {"drums": 8, "bass": 9, "chords": 10, "lead": 11}
BAR_BASE, STEP_BASE, PITCH_BASE, VEL_BASE, DUR_BASE = 16, 32, 128, 256, 272


def encode_song(song):
    tokens = [BOS, STYLES.get(str(song.get("style", "other")).lower(), 15)]
    events = sorted(song["events"], key=lambda e: (
        int(e.get("bar", 0)), int(e.get("step", 0)),
        TRACKS.get(str(e.get("track", "lead")).lower(), 11),
        int(e.get("pitch", 60))))
    for event in events:
        track = TRACKS.get(str(event.get("track", "lead")).lower())
        if track is None:
            continue
        bar = max(0, min(15, int(event.get("bar", 0))))
        step = max(0, min(15, int(event.get("step", 0))))
        pitch = max(0, min(127, int(event.get("pitch", 60))))
        velocity = max(1, min(127, int(event.get("velocity", 100))))
        duration = max(1, min(16, int(event.get("duration", 4))))
        tokens.extend([track, BAR_BASE + bar, STEP_BASE + step, PITCH_BASE + pitch,
                       VEL_BASE + min(15, (velocity - 1) // 8), DUR_BASE + duration - 1])
    tokens.append(EOS)
    return tokens


def chunk_song(tokens, block):
    if block < 9:
        raise ValueError("--block must be at least 9")
    per_chunk = max(1, (block - 3) // 6)
    event_tokens = tokens[2:-1]
    chunks = [event_tokens[i:i + per_chunk * 6]
              for i in range(0, len(event_tokens), per_chunk * 6)]
    return [[BOS, tokens[1], *chunk] +
            ([EOS] if index == len(chunks) - 1 else [])
            for index, chunk in enumerate(chunks)]


class SongChunks(Dataset):
    def __init__(self, songs, block):
        self.rows = []
        for song in songs:
            if not isinstance(song.get("events"), list) or not song["events"]:
                continue
            self.rows.extend(torch.tensor(row, dtype=torch.long)
                             for row in chunk_song(encode_song(song), block))
        if not self.rows:
            raise ValueError("Dataset split contains no usable songs")

    def __len__(self):
        return len(self.rows)

    def __getitem__(self, index):
        return self.rows[index]


def collate(rows):
    width = max(row.numel() for row in rows)
    padded = torch.full((len(rows), width), PAD, dtype=torch.long)
    for index, row in enumerate(rows):
        padded[index, :row.numel()] = row
    return padded[:, :-1], padded[:, 1:]


class GPT(nn.Module):
    def __init__(self, block=512, width=192, layers=6, heads=6, dropout=0.1):
        super().__init__()
        self.block = block
        self.tok = nn.Embedding(VOCAB, width, padding_idx=PAD)
        self.pos = nn.Embedding(block, width)
        layer = nn.TransformerEncoderLayer(
            width, heads, width * 4, dropout, batch_first=True,
            norm_first=True, activation="gelu")
        self.stack = nn.TransformerEncoder(
            layer, layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(width)
        self.head = nn.Linear(width, VOCAB, bias=False)
        self.head.weight = self.tok.weight

    def forward(self, tokens):
        batch, length = tokens.shape
        if length > self.block:
            tokens = tokens[:, -self.block:]
            length = self.block
        causal = torch.triu(torch.ones(
            length, length, device=tokens.device, dtype=torch.bool), diagonal=1)
        padding = tokens.eq(PAD)
        hidden = self.tok(tokens) + self.pos(
            torch.arange(length, device=tokens.device))[None, :, :]
        hidden = self.stack(hidden, mask=causal, src_key_padding_mask=padding)
        return self.head(self.norm(hidden))


def read_songs(path):
    songs = []
    for line_no, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            song = json.loads(line)
            if isinstance(song.get("events"), list) and song["events"]:
                songs.append(song)
        except Exception as exc:
            raise ValueError(f"{path}:{line_no}: {exc}") from exc
    if len(songs) < 2:
        raise ValueError("Need at least two valid songs for train/validation split")
    return songs


def token_loss(model, loader, device, scaler=None):
    model.eval()
    total_loss = 0.0
    total_tokens = 0
    with torch.no_grad():
        for x, y in loader:
            x, y = x.to(device), y.to(device)
            with torch.autocast(device_type="cuda", enabled=device == "cuda"):
                logits = model(x)
                loss_sum = nn.functional.cross_entropy(
                    logits.reshape(-1, VOCAB), y.reshape(-1),
                    ignore_index=PAD, reduction="sum")
            count = int(y.ne(PAD).sum().item())
            total_loss += float(loss_sum.item())
            total_tokens += count
    return total_loss / max(1, total_tokens)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", required=True)
    parser.add_argument("--out", default="models/honeydew-music.pt")
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--epochs", type=int, default=30)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--block", type=int, default=512)
    parser.add_argument("--lr", type=float, default=3e-4)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--val-fraction", type=float, default=0.1)
    args = parser.parse_args()
    if args.epochs < 1 or args.batch_size < 1 or args.block < 9 or args.lr <= 0:
        parser.error("epochs/batch-size must be positive, block >= 9, and lr > 0")
    if not 0.05 <= args.val_fraction <= 0.4:
        parser.error("--val-fraction must be between 0.05 and 0.4")

    random.seed(args.seed)
    torch.manual_seed(args.seed)
    device = "cuda" if torch.cuda.is_available() else "cpu"
    songs = read_songs(args.data)
    random.Random(args.seed).shuffle(songs)
    val_count = max(1, int(len(songs) * args.val_fraction))
    train_songs, val_songs = songs[val_count:], songs[:val_count]
    train_ds = SongChunks(train_songs, args.block)
    val_ds = SongChunks(val_songs, args.block)
    train_loader = DataLoader(train_ds, batch_size=args.batch_size,
                              shuffle=True, collate_fn=collate)
    val_loader = DataLoader(val_ds, batch_size=args.batch_size,
                            shuffle=False, collate_fn=collate)

    model = GPT(args.block).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr, weight_decay=0.1)
    start_epoch, best_val = 0, float("inf")
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    best_out = out.with_name(out.stem + ".best" + out.suffix)
    if args.resume and out.exists():
        checkpoint = torch.load(out, map_location=device, weights_only=False)
        model.load_state_dict(checkpoint["model"])
        optimizer.load_state_dict(checkpoint["optimizer"])
        start_epoch = int(checkpoint.get("epoch", 0))
        best_val = float(checkpoint.get("best_val_loss", float("inf")))

    scaler = torch.amp.GradScaler("cuda", enabled=device == "cuda")
    print(f"songs={len(songs)} train={len(train_songs)} validation={len(val_songs)} "
          f"train_chunks={len(train_ds)} val_chunks={len(val_ds)} device={device}", flush=True)

    for epoch in range(start_epoch, args.epochs):
        model.train()
        total_loss, total_tokens = 0.0, 0
        for x, y in train_loader:
            x, y = x.to(device), y.to(device)
            optimizer.zero_grad(set_to_none=True)
            with torch.autocast(device_type="cuda", enabled=device == "cuda"):
                logits = model(x)
                loss = nn.functional.cross_entropy(
                    logits.reshape(-1, VOCAB), y.reshape(-1), ignore_index=PAD)
            scaler.scale(loss).backward()
            scaler.unscale_(optimizer)
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            scaler.step(optimizer)
            scaler.update()
            count = int(y.ne(PAD).sum().item())
            total_loss += float(loss.item()) * count
            total_tokens += count

        train_loss = total_loss / max(1, total_tokens)
        val_loss = token_loss(model, val_loader, device)
        improved = val_loss < best_val
        if improved:
            best_val = val_loss
        checkpoint = {
            "model": model.state_dict(), "optimizer": optimizer.state_dict(),
            "epoch": epoch + 1, "best_val_loss": best_val,
            "train_loss": train_loss, "val_loss": val_loss,
            "config": {"vocab": VOCAB, "block": args.block, "width": 192,
                       "layers": 6, "heads": 6},
            "dataset": str(args.data), "styles": STYLES,
        }
        # The main checkpoint is updated after every epoch and is resumable.
        torch.save(checkpoint, out)
        if improved:
            torch.save(checkpoint, best_out)
        print(f"epoch {epoch + 1}/{args.epochs} train_loss={train_loss:.4f} "
              f"val_loss={val_loss:.4f} best_val={best_val:.4f} "
              f"checkpoint={out} best={'updated' if improved else 'unchanged'}",
              flush=True)


if __name__ == "__main__":
    main()
