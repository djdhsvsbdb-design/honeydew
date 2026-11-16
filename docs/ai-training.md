# Honeydew AI music model

Honeydew's research model predicts quantized multi-track MIDI events, not raw audio. Generated events can be exported as MIDI and are intended to remain editable in the sequencer.

## Cloud training (Google Colab or a GPU PC)

Install dependencies:

```bash
python -m pip install -r tools/ai/requirements.txt
```

Create a procedural smoke-test dataset:

```bash
python tools/ai/generate_synthetic.py --songs 12000 --out data/ai/synthetic.jsonl --seed 42
```

Train with a song-level validation split. The main checkpoint is updated after every epoch and can be resumed; the best validation checkpoint is saved separately:

```bash
python tools/ai/train_music.py --data data/ai/synthetic.jsonl --epochs 20 --batch-size 8 --block 256 --val-fraction 0.1 --out models/honeydew-music.pt
python tools/ai/train_music.py --data data/ai/more-midi.jsonl --epochs 40 --batch-size 8 --block 256 --resume --out models/honeydew-music.pt
```

Each epoch prints training loss and validation loss. Lower validation loss is a useful signal, but it does not prove that generated music sounds good. Compare MIDI samples by listening as well. Checkpoints contain model weights, optimizer state, epoch, train/validation metrics and model configuration.

## Generate MIDI and continue an existing arrangement

Generate an eight-bar editable MIDI arrangement from the best checkpoint:

```bash
python tools/ai/generate_music.py --checkpoint models/honeydew-music.best.pt --style trap --bars 8 --bpm 145 --out output/honeydew-generated
```

This writes both `output/honeydew-generated.mid` and `output/honeydew-generated.json`. To condition generation on an existing Honeydew event-JSON file, pass `--prompt-json path/to/song.json`. Sampling controls include `--temperature`, `--top-k`, `--max-events` and `--seed`.

The sampler constrains token types to valid event fields. It is an experimental sampler: it does not guarantee musical quality, and a small synthetic-only model may produce repetitive or incoherent arrangements. Use licensed or self-created MIDI for meaningful musical training.

## JSONL song format

One song per line. Positions are bars and sixteenth-note steps (0..15 within each bar); duration is in sixteenth-note steps.

```json
{"style":"hiphop","bpm":92,"key":"C","events":[
  {"track":"drums","bar":0,"step":0,"pitch":36,"velocity":120,"duration":1},
  {"track":"drums","bar":0,"step":8,"pitch":38,"velocity":105,"duration":1},
  {"track":"bass","bar":0,"step":0,"pitch":36,"velocity":100,"duration":8},
  {"track":"chords","bar":0,"step":0,"pitch":60,"velocity":85,"duration":16},
  {"track":"lead","bar":0,"step":12,"pitch":72,"velocity":95,"duration":2}
]}
```

Allowed tracks: `drums`, `bass`, `chords`, `lead`. Pitch is MIDI 0..127; velocity 1..127; duration 1..16. Keep training and validation/test songs separated by source/song, not by splitting events from the same song.

## Import licensed MIDI

```bash
python tools/ai/midi_to_jsonl.py ./my_midi_library --style hiphop --out data/ai/train.jsonl
python tools/ai/train_music.py --data data/ai/train.jsonl --epochs 30 --out models/honeydew-music.pt
```

The importer maps MIDI channels and track names to drums, bass, chords and lead. Review track labels and style tags. Only use MIDI/audio you created, have permission to use, or that is licensed for model training.

## Token layout (v1)

Special tokens: PAD=0, BOS=1, EOS=2; style tokens 3..7 and 15; track tokens drums=8, bass=9, chords=10, lead=11; bar=16..31; step=32..47; pitch=128..255; velocity bucket=256..271; duration=272..287.

## Product integration status

- The Rust beatmaker already creates editable drum, bass, chord and lead tracks with style-aware built-in effect presets. This is deterministic procedural generation, not inference from the Transformer.
- The Python Transformer trainer, validation, resumable checkpoints and MIDI sampler are research tooling.
- The trained PyTorch checkpoint is not yet converted to a compact mobile format or connected to the Android inference/UI. An in-app natural-language assistant and learned sound-design model are future integration work; do not treat the Colab checkpoint as an Android-ready model.
