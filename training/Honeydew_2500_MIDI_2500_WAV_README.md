# Honeydew audio/MIDI dataset generator

Open in Google Colab:

https://colab.research.google.com/github/djdhsvsbdb-design/honeydew/blob/honeydew-generator-full-song-export/training/Honeydew_2500_MIDI_2500_WAV.ipynb

The notebook generates 2,500 original MIDI compositions and renders a matching 2,500 mono WAV files (16 kHz), then trains a small next-note LSTM baseline. It does not download copyrighted commercial tracks.

Important:
- The dataset is created in the Colab runtime when the notebook is run; binary outputs are not committed to GitHub.
- This is synthetic algorithmic data. The model mostly learns the patterns of the generator and is a pipeline baseline, not evidence of human-level composition.
- The WAVs are renders of their paired MIDI files.
- Use a Colab runtime with enough disk space. Rendering 2,500 WAVs and training can take a while.
