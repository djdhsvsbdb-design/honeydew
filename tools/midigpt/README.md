# MIDI-GPT bridge for Honeydew (experimental)

This is the first integration step: a local Python helper uses MIDI-GPT to
generate or complete MIDI tracks, then Honeydew opens the resulting MIDI using
its existing MIDI importer. The model is not embedded in the Rust audio engine;
MIDI-GPT produces symbolic notes, and Honeydew's instruments render them.

## Requirements

- Desktop Honeydew.
- Python 3.10+ with the MIDI-GPT inference package:
  `python -m pip install "midigpt[inference]"`
- Access to the selected checkpoint on Hugging Face. Some checkpoints are
  gated; accept the model terms and authenticate with `hf auth login` or
  `HF_TOKEN` if required.
- A MIDI source file. For new parts, include an empty MIDI track in the source.

## Generate

From the Honeydew repository root:

```sh
python tools/midigpt/generate.py input.mid --output generated.mid --model prism_medium --model-dim 8
```

Generate only tracks 1 and 3 (zero-based indexes):

```sh
python tools/midigpt/generate.py input.mid --output generated.mid --track 1 --track 3
```

Then open `generated.mid` in Honeydew with SONGS -> Import or Ctrl+O.
Honeydew maps MIDI channels to its closest bundled General MIDI instruments;
MIDI channel 10 becomes a drum kit. Review the instrument assignments and
listen to the result before using it in a project.

The helper preserves unselected tracks through MIDI-GPT's `ignore` prompt.
Selected tracks are generated across their existing bars. To add a new part,
put an empty track of the desired length in the input MIDI and select its
zero-based index. This initial bridge does not yet provide a text prompt,
automatic genre detection, a Honeydew toolbar button, live streaming, or
automatic effect/preset selection.

## Model and data licensing

The MIDI-GPT Python code and pretrained weights have separate licenses.
The upstream pretrained weights are under the Metacreation Non-Commercial
Research License (research/education only). Do not package or use those weights
in a commercial Honeydew release without written permission or a separately
licensed model. The helper deliberately does not download or redistribute
weights itself; the upstream library manages its model cache and access terms.

See https://github.com/Metacreation-Lab/MIDI-GPT and
https://huggingface.co/Metacreation/MIDI-GPT for upstream details.
