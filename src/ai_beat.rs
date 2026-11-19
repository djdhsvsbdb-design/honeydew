//! Deterministic, multi-track beat generation for Honeydew.
//!
//! The generator writes ordinary sequencer events. It does not render audio or replace the instrument engine:
//! generated tracks are regular Honeydew channels and remain editable in the piano roll and drum editor.

use crate::song::{ChannelKind, DrumGrid, Event, LaneKind, Song, SynthKind, PPQ};

/// Musical style used as the starting point for the arrangement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BeatStyle {
    #[default]
    HipHop,
    Trap,
    Drill,
    Phonk,
    House,
}

/// User-facing generation controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeatSettings {
    /// Number of bars, clamped to 1..=8.
    pub bars: u8,
    /// MIDI pitch class: C = 0, C# = 1, …, B = 11.
    pub key: u8,
    pub minor: bool,
    pub style: BeatStyle,
    /// Equal settings and seed always produce equal events.
    pub seed: u64,
}

impl Default for BeatSettings {
    fn default() -> Self {
        Self { bars: 4, key: 0, minor: true, style: BeatStyle::HipHop, seed: 1 }
    }
}

/// A generated part that can be copied directly into a Honeydew sequencer lane.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedTrack {
    pub name: &'static str,
    pub channel_kind: ChannelKind,
    pub lane_kind: LaneKind,
    pub events: Vec<Event>,
    /// Built-in audio effects (module type id, wet mix), appended after the instrument.
    pub effects: Vec<(u16, f32)>,
}

/// A complete arrangement, ready to become normal editable Honeydew channels.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedBeat {
    pub bars: u8,
    pub tracks: Vec<GeneratedTrack>,
}

impl GeneratedBeat {
    /// Append the generated parts to song. Returns the indices of channels created.
    /// Existing channels are left untouched. If Honeydew runs out of channel IDs, creation stops safely.
    pub fn append_to_song(&self, song: &mut Song) -> Vec<usize> {
        let beats_per_bar = song.time_sig.bar_beats();
        let loop_beats = f64::from(self.bars) * f64::from(beats_per_bar);
        let mut created = Vec::with_capacity(self.tracks.len());

        for track in &self.tracks {
            let Some(index) = song.insert_channel(song.channels.len(), track.channel_kind) else {
                break;
            };
            let Some(channel) = song.channels.get_mut(index) else {
                continue;
            };
            if let Some(lane) = channel.lanes.iter_mut().find(|lane| lane.kind == track.lane_kind) {
                if track.lane_kind == LaneKind::Drums {
                    lane.grid = DrumGrid::Steps;
                }
                let clip = lane.add_clip(0, loop_beats);
                clip.events.clone_from(&track.events);
                clip.fit_loop(beats_per_bar);
            }
            // AI FX are ordinary Honeydew rack modules, so they render through the existing audio engine
            // and remain editable in the channel rack after generation.
            for &(type_id, wet_mix) in &track.effects {
                let instance_id = channel.rack.add(type_id);
                if let Some(effect) = channel.rack.get_mut(instance_id) {
                    effect.common.0[1] = wet_mix.clamp(0.0, 1.0);
                }
            }
            created.push(index);
        }
        created
    }
}

/// Generate drums, bass, chords and a lead as four ordinary sequencer tracks.
pub fn generate(settings: BeatSettings) -> GeneratedBeat {
    let bars = settings.bars.clamp(1, 8);
    let key = settings.key % 12;
    let scale: &[i32] = if settings.minor { &[0, 2, 3, 5, 7, 8, 10] } else { &[0, 2, 4, 5, 7, 9, 11] };
    let mut rng = Rng::new(settings.seed);

    let drums = make_drums(bars, settings.style, &mut rng);
    let roots = progression(bars, settings.minor);
    let bass = make_bass(bars, key, scale, &roots, settings.style, &mut rng);
    let chords = make_chords(bars, key, scale, &roots);
    let lead = make_lead(bars, key, scale, &roots, &mut rng);

    GeneratedBeat {
        bars,
        tracks: vec![
            GeneratedTrack { name: "AI Drums", channel_kind: ChannelKind::Drums, lane_kind: LaneKind::Drums, events: drums, effects: drum_fx(settings.style) },
            GeneratedTrack { name: "AI Bass", channel_kind: ChannelKind::Synth(SynthKind::TransistorBass), lane_kind: LaneKind::Notes, events: bass, effects: bass_fx(settings.style) },
            GeneratedTrack { name: "AI Chords", channel_kind: ChannelKind::Synth(SynthKind::GmSynth), lane_kind: LaneKind::Notes, events: chords, effects: chord_fx(settings.style) },
            GeneratedTrack { name: "AI Lead", channel_kind: ChannelKind::Synth(SynthKind::MiniSynth), lane_kind: LaneKind::Notes, events: lead, effects: lead_fx(settings.style) },
        ],
    }
}


/// Style-aware starter chains. IDs are Honeydew's built-in effects: 8 compressor, 6 distortion,
/// 32 reverb, 16 trance delay and 11 chorus. Mix is the rack module's wet/dry control.
fn drum_fx(_style: BeatStyle) -> Vec<(u16, f32)> {
    vec![(8, 0.22)]
}

fn bass_fx(style: BeatStyle) -> Vec<(u16, f32)> {
    match style {
        BeatStyle::Phonk => vec![(6, 0.38), (8, 0.24)],
        BeatStyle::Trap | BeatStyle::Drill => vec![(6, 0.16), (8, 0.20)],
        BeatStyle::HipHop => vec![(8, 0.18)],
        BeatStyle::House => vec![(8, 0.26)],
    }
}

fn chord_fx(style: BeatStyle) -> Vec<(u16, f32)> {
    match style {
        BeatStyle::House => vec![(32, 0.20), (11, 0.24)],
        _ => vec![(32, 0.30)],
    }
}

fn lead_fx(style: BeatStyle) -> Vec<(u16, f32)> {
    match style {
        BeatStyle::Phonk => vec![(32, 0.20), (16, 0.12)],
        BeatStyle::House => vec![(32, 0.24), (16, 0.18)],
        _ => vec![(32, 0.36), (16, 0.16)],
    }
}

fn progression(bars: u8, minor: bool) -> Vec<i32> {
    let degrees: &[i32] = if minor { &[0, 5, 2, 6] } else { &[0, 4, 5, 3] };
    (0..bars as usize).map(|bar| degrees[bar % degrees.len()]).collect()
}

fn make_drums(bars: u8, style: BeatStyle, rng: &mut Rng) -> Vec<Event> {
    // Prefer the trained GRU model. The handcrafted pattern below remains a safe fallback
    // if an embedded model cannot be decoded on a target platform.
    let style_index = match style {
        BeatStyle::HipHop => 0, BeatStyle::Trap => 1, BeatStyle::Drill => 2,
        BeatStyle::Phonk => 3, BeatStyle::House => 4,
    };
    if let Some(pattern) = crate::ai_model::generate(style_index, usize::from(bars) * 16, 0.5) {
        let mut events = Vec::new();
        for (step, hits) in pattern.into_iter().enumerate() {
            let bar = (step / 16) as f64;
            let beat = (step % 16) as f64 * 0.25;
            for (pad, hit) in hits.into_iter().enumerate() {
                if hit {
                    let velocity = match pad {
                        0 => if step % 16 == 0 { 245 } else { 218 },
                        1 => 225,
                        2 => if step % 4 == 0 { 155 } else { 112 },
                        _ => 135,
                    };
                    events.push(Event::Drum {
                        pos: tick(bar * 4.0 + beat), len_beats: 0.05, pad: pad as u8, velocity,
                        pan: Event::DEFAULT_PAN, value: Event::DEFAULT_PITCH, repeat: 0,
                    });
                }
            }
        }
        events.sort_by_key(Event::pos);
        if !events.is_empty() { return events; }
    }
    let mut out = Vec::new();
    for bar in 0..bars as usize {
        let base = bar as f64 * 4.0;
        let mut hit = |beat: f64, pad: u8, velocity: u8| {
            out.push(Event::Drum {
                pos: tick(base + beat), len_beats: 0.05, pad, velocity,
                pan: Event::DEFAULT_PAN, value: Event::DEFAULT_PITCH, repeat: 0,
            });
        };
        match style {
            BeatStyle::House => {
                for beat in 0..4 { hit(beat as f64, 0, if beat == 0 { 245 } else { 215 }); }
                hit(1.0, 1, 210);
                hit(3.0, 1, 220);
                for step in 0..8 { hit(step as f64 * 0.5, 3, if step % 2 == 0 { 155 } else { 105 }); }
                hit(3.5, 4, 120);
            }
            _ => {
                hit(0.0, 0, 245);
                hit(2.0, 0, 220);
                if style == BeatStyle::Drill && rng.chance(0.55) {
                    hit(2.75, 0, 175);
                } else if style == BeatStyle::Trap && rng.chance(0.35) {
                    hit(3.5, 0, 165);
                }
                hit(1.0, 1, 225);
                hit(3.0, 1, 230);
                if style == BeatStyle::Phonk { hit(3.0, 2, 165); }
                let hat_step = if style == BeatStyle::Trap { 0.25 } else { 0.5 };
                let count = (4.0 / hat_step) as usize;
                for step in 0..count {
                    let beat = step as f64 * hat_step;
                    let accent = if step % 4 == 0 { 155 } else { 112 };
                    let velocity = (accent + rng.range(0, 24)).min(210) as u8;
                    if !(style == BeatStyle::Drill && step % 8 == 7 && rng.chance(0.45)) {
                        hit(beat, 3, velocity);
                    }
                }
                if rng.chance(0.65) { hit(3.5, 4, 135); }
            }
        }
    }
    out.sort_by_key(Event::pos);
    out
}

fn make_bass(bars: u8, key: u8, scale: &[i32], roots: &[i32], style: BeatStyle, rng: &mut Rng) -> Vec<Event> {
    let mut out = Vec::new();
    for bar in 0..bars as usize {
        let base = bar as f64 * 4.0;
        let root = roots[bar];
        let root_note = scale_note(key, scale, root, 2);
        let fifth = scale_note(key, scale, root + 4, 2);
        let pattern: &[(f64, bool)] = match style {
            BeatStyle::House => &[(0.0, false), (1.0, true), (2.0, false), (3.0, true)],
            BeatStyle::Drill => &[(0.0, false), (1.5, true), (2.75, false), (3.5, true)],
            BeatStyle::Trap | BeatStyle::Phonk => &[(0.0, false), (1.75, true), (2.5, false), (3.5, true)],
            BeatStyle::HipHop => &[(0.0, false), (2.0, true), (3.0, false)],
        };
        for &(beat, use_fifth) in pattern.iter() {
            let key = if use_fifth && rng.chance(0.6) { fifth } else { root_note };
            out.push(note(base + beat, if style == BeatStyle::House { 0.8 } else { 0.42 }, key, 220));
        }
    }
    out.sort_by_key(Event::pos);
    out
}

fn make_chords(bars: u8, key: u8, scale: &[i32], roots: &[i32]) -> Vec<Event> {
    let mut out = Vec::new();
    for bar in 0..bars as usize {
        let beat = bar as f64 * 4.0;
        for tone in [0, 2, 4] {
            out.push(note(beat, 3.8, scale_note(key, scale, roots[bar] + tone, 4), 158));
        }
    }
    out.sort_by_key(Event::pos);
    out
}

fn make_lead(bars: u8, key: u8, scale: &[i32], roots: &[i32], rng: &mut Rng) -> Vec<Event> {
    let mut out = Vec::new();
    for bar in 0..bars as usize {
        let base = bar as f64 * 4.0;
        let mut degree = roots[bar] + 4;
        for step in 0..8 {
            if step > 0 {
                let movement = [-1, 0, 1, 2][rng.range(0, 4) as usize];
                degree = (degree + movement).clamp(0, 13);
            }
            if step % 4 == 3 && rng.chance(0.45) { continue; }
            let note_key = scale_note(key, scale, degree, 5);
            out.push(note(base + step as f64 * 0.5, 0.42, note_key, 188 + rng.range(0, 36) as u8));
        }
    }
    out.sort_by_key(Event::pos);
    out
}

fn scale_note(key: u8, scale: &[i32], degree: i32, octave: i32) -> u8 {
    let count = scale.len() as i32;
    let index = degree.rem_euclid(count) as usize;
    let extra_octaves = degree.div_euclid(count);
    (48 + i32::from(key) + scale[index] + 12 * (octave + extra_octaves)).clamp(0, 127) as u8
}

fn note(beat: f64, len_beats: f64, key: u8, velocity: u8) -> Event {
    Event::Note {
        pos: tick(beat), len_beats, key, velocity,
        pan: Event::DEFAULT_PAN, slide: false,
    }
}

fn tick(beat: f64) -> u32 {
    (beat * f64::from(PPQ)).round() as u32
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1) }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, low: u32, high: u32) -> u32 {
        low + (self.next() % u64::from((high - low).max(1))) as u32
    }

    fn chance(&mut self, probability: f64) -> bool {
        ((self.next() >> 11) as f64) / ((1u64 << 53) as f64) < probability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_makes_the_same_arrangement() {
        let settings = BeatSettings { style: BeatStyle::Trap, seed: 42, ..BeatSettings::default() };
        assert_eq!(generate(settings), generate(settings));
    }

    #[test]
    fn arrangement_has_four_editable_parts_and_valid_events() {
        let beat = generate(BeatSettings { bars: 4, ..BeatSettings::default() });
        assert_eq!(beat.tracks.len(), 4);
        assert_eq!(beat.tracks[0].lane_kind, LaneKind::Drums);
        assert!(beat.tracks[0].events.iter().all(|e| matches!(e, Event::Drum { .. })));
        assert!(beat.tracks[1..].iter().all(|track| {
            track.lane_kind == LaneKind::Notes && track.events.iter().all(|e| matches!(e, Event::Note { .. }))
        }));
        let end = u32::from(beat.bars) * 4 * PPQ;
        assert!(beat.tracks.iter().flat_map(|t| &t.events).all(|e| e.pos() < end));
    }

    #[test]
    fn generated_tracks_receive_editable_style_aware_effects() {
        let beat = generate(BeatSettings { style: BeatStyle::Phonk, ..BeatSettings::default() });
        assert!(beat.tracks.iter().all(|track| !track.effects.is_empty()));
        let mut song = Song::default();
        beat.append_to_song(&mut song);
        let generated = &song.channels[1..];
        assert_eq!(generated.len(), 4);
        assert!(generated.iter().all(|channel| {
            channel.rack.modules.iter().any(|module| {
                honeydew_audio::effect_info(module.type_id).is_some()
                    && (0.0..=1.0).contains(&module.common.0[1])
            })
        }));
        let bass = generated.iter().find(|channel| channel.name.starts_with("Transistor Bass")).unwrap();
        assert!(bass.rack.modules.iter().any(|module| module.type_id == 6));
        let lead = generated.iter().find(|channel| channel.name.starts_with("MiniSynth")).unwrap();
        assert!(lead.rack.modules.iter().any(|module| module.type_id == 32));
    }

    #[test]
    fn bar_count_and_key_are_safely_clamped() {
        let beat = generate(BeatSettings { bars: 0, key: 255, ..BeatSettings::default() });
        assert_eq!(beat.bars, 1);
        assert_eq!(beat.tracks.len(), 4);
    }

    #[test]
    fn every_style_generates_a_complete_arrangement() {
        for style in [BeatStyle::HipHop, BeatStyle::Trap, BeatStyle::Drill, BeatStyle::Phonk, BeatStyle::House] {
            let beat = generate(BeatSettings { style, ..BeatSettings::default() });
            assert_eq!(beat.tracks.len(), 4, "{style:?}");
            assert!(beat.tracks.iter().all(|track| !track.events.is_empty()), "{style:?}");
        }
    }
}
