# Honeydew AI music model

This is the continuing-training foundation for a multi-track, autoregressive music model. It predicts a stream of quantized musical events rather than raw audio, so outputs can become editable Honeydew sequencer clips.

## Install and train

```bash
python -m pip install torch
python tools/ai/train_music.py --data data/ai/train.jsonl --epochs 30 --out models/honeydew-music.pt
# Continue training from that checkpoint:
python tools/ai/train_music.py --data data/ai/more-midi.jsonl --epochs 60 --out models/honeydew-music.pt --resume
```

The model uses a causal Transformer (6 layers, width 192, 6 attention heads, 512-token context). It saves optimizer state as well as weights, so `--resume` continues training rather than starting over. Use a GPU if available; CPU works but is slower.

## JSONL song format

One song per line. Positions are in bars and sixteenth-note steps (0..63 per bar). Duration is in sixteenth-note steps.

```json
{"style":"hiphop","bpm":92,"key":"C","events":[
  {"track":"drums","bar":0,"step":0,"pitch":36,"velocity":120,"duration":1},
  {"track":"drums","bar":0,"step":8,"pitch":38,"velocity":105,"duration":1},
  {"track":"bass","bar":0,"step":0,"pitch":36,"velocity":100,"duration":8},
  {"track":"chords","bar":0,"step":0,"pitch":60,"velocity":85,"duration":16},
  {"track":"lead","bar":0,"step":12,"pitch":72,"velocity":95,"duration":2}
]}
```

Allowed tracks: `drums`, `bass`, `chords`, `lead`. Pitch is MIDI 0..127; velocity 1..127; duration 1..16. A song must contain at least one event. Keep training and validation/test songs separated by source/song, not by randomly splitting events from the same song, to reduce leakage.

## Import MIDI files\n\nInstall the extra parser and convert a folder of MIDI files into the training format:\n\n```bash\npython -m pip install -r tools/ai/requirements.txt\npython tools/ai/midi_to_jsonl.py ./my_midi_library --style hiphop --out data/ai/train.jsonl\npython tools/ai/train_music.py --data data/ai/train.jsonl --epochs 30 --out models/honeydew-music.pt\n```\n\nThe importer maps MIDI channels and track names to drums, bass, chords and lead. Review the output and correct track labels/style tags where needed; tempo is metadata and positions are quantized to sixteenth notes. Keep only files you are allowed to use for model training.\n\n## Token layout (v1)

Special tokens: PAD=0, BOS=1, EOS=2; style tokens 3..7 and 15; track tokens drums=8, bass=9, chords=10, lead=11; bar=16..31; step=32..95; pitch=128..255; velocity bucket=256..271; duration=272..287. Unused IDs are reserved for schema growth.

## Data quality and rights

Only train on MIDI/audio you created, have permission to use, or that is licensed for this purpose. Remove duplicate songs and corrupted files. The current small drum GRU is a separate experimental seed model trained on synthetic patterns; this Transformer is the general model intended for continued training. It is not yet connected to the live beatmaker UI or exported to the Android runtime.
