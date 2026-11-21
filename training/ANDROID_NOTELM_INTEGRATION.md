# Honeydew NoteLM Android integration

The Generate panel's **New full song** target calls the pure-Rust NoteLM inference module for melody pitches when the trained checkpoint is available.

## Required model asset

Place the converted checkpoint at:

`assets/honeydew_note_lm.f32`

Expected format: 259,712 little-endian float32 values (1,038,848 bytes), in this order:

1. `emb.weight` — 128 × 96
2. `rnn.weight_ih_l0` — 768 × 96
3. `rnn.weight_hh_l0` — 768 × 192
4. `rnn.bias_ih_l0` — 768
5. `rnn.bias_hh_l0` — 768
6. `head.weight` — 128 × 192
7. `head.bias` — 128

This is the exported state dictionary from `honeydew_note_lm.pt`, not the original PyTorch archive.

## Android packaging

`tools/android-dist.sh` copies the file to the APK asset path:

`assets/models/honeydew_note_lm.f32`

At runtime `src/song/ai_model.rs` reads it through Android's asset manager. The build verifies its exact size and byte-for-byte packaging when the source asset exists.

If the asset is absent, the app remains buildable and the melody generator uses its existing deterministic fallback. That fallback is not the trained model.

## Model scope

The trained network predicts only the next MIDI pitch. The arrangement code still controls note timing, durations, key/scale filtering, velocity, harmony, drums, bass and chord tracks. This integration does not turn the checkpoint into an audio generator or a complete arrangement model.
