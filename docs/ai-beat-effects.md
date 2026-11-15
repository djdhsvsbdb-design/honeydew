# AI beat effects

The AI beat generator now attaches ordinary Honeydew rack effects to each generated channel. These are native effect modules, not a baked audio render: users can open the channel rack, change parameters, reorder modules, or remove them after generation.

Current starter chains are style-aware:

- Drums: light compression.
- Bass: compression; Trap/Drill/Phonk also add distortion, with stronger saturation for Phonk.
- Chords: reverb, with a little chorus in House.
- Lead: reverb and tempo-synced delay; wet levels vary by style.

The generator sets each module's common Mix parameter and leaves the effect's own controls at Honeydew's built-in defaults. This keeps the initial sound safe and editable across desktop, web, and Android, using the existing audio engine and effect rack.

This is currently a rule-based FX preset layer, not a trained neural FX model. Once licensed MIDI/audio examples are available, the training format can be extended with per-track effect IDs and normalized parameter values so the model can learn effect choices and settings.
