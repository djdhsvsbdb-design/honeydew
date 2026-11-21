//! Full-song arrangement templates. This is the first generation layer: it plans song sections
//! in bars before MIDI/event generation and UI wiring are added.

/// Musical hints extracted from a natural-language generation brief.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptProfile {
    pub minor: Option<bool>,
    pub tempo_bpm: Option<u16>,
    pub energy: u8,
    pub genre: Genre,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    Electronic,
    House,
    Techno,
    DrumAndBass,
    Trap,
    HipHop,
    Pop,
    Rock,
    Ambient,
    Cinematic,
    Unknown,
}

impl PromptProfile {
    /// Lightweight offline interpretation. It deliberately extracts explicit musical constraints;
    /// it does not claim to be a remote generative AI model.
    pub fn parse(description: &str) -> Self {
        let text = description.to_lowercase();
        let minor = if [
            "minor",
            "dark",
            "sad",
            "melancholic",
            "мрач",
            "груст",
            "минор",
        ]
        .iter()
        .any(|word| text.contains(word))
        {
            Some(true)
        } else if [
            "major",
            "happy",
            "bright",
            "uplifting",
            "весел",
            "мажор",
            "светл",
        ]
        .iter()
        .any(|word| text.contains(word))
        {
            Some(false)
        } else {
            None
        };
        let genre = if text.contains("drum and bass")
            || text.contains("dnb")
            || text.contains("драм-н-бейс")
        {
            Genre::DrumAndBass
        } else if text.contains("house") || text.contains("хаус") {
            Genre::House
        } else if text.contains("techno") || text.contains("техно") {
            Genre::Techno
        } else if text.contains("trap") || text.contains("трэп") {
            Genre::Trap
        } else if text.contains("hip-hop") || text.contains("hip hop") || text.contains("хип-хоп")
        {
            Genre::HipHop
        } else if text.contains("rock") || text.contains("рок") {
            Genre::Rock
        } else if text.contains("pop") || text.contains("поп") {
            Genre::Pop
        } else if text.contains("ambient") || text.contains("эмбиент") {
            Genre::Ambient
        } else if text.contains("cinematic") || text.contains("кинемат") {
            Genre::Cinematic
        } else if text.contains("edm") || text.contains("electronic") || text.contains("электрон")
        {
            Genre::Electronic
        } else {
            Genre::Unknown
        };
        let energy = if [
            "aggressive",
            "energetic",
            " мощ",
            "энерг",
            "жестк",
            "интенсив",
        ]
        .iter()
        .any(|word| text.contains(word.trim()))
        {
            90
        } else if ["calm", "soft", "chill", "спокой", "мягк", "расслаб"]
            .iter()
            .any(|word| text.contains(word))
        {
            25
        } else {
            60
        };
        let tempo_bpm = text
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<u16>().ok())
            .find(|bpm| (40..=220).contains(bpm));
        Self {
            minor,
            tempo_bpm,
            energy,
            genre,
        }
    }
}

/// Length preset for a full arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongForm {
    /// 32 bars: compact, with two shorter chorus passes.
    Compact,
    /// 48 bars: conventional verse/chorus form with a bridge and final chorus.
    Standard,
    /// 64 bars: longer form with repeated chorus and an extended bridge.
    Extended,
}

/// A named section occupying a contiguous range of bars (start is zero-based, end is exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongSection {
    pub name: &'static str,
    pub start_bar: u16,
    pub end_bar: u16,
}

impl SongSection {
    /// Length of this section in bars.
    pub fn bars(self) -> u16 {
        self.end_bar - self.start_bar
    }
}

/// A complete section plan, with contiguous sections covering the full arrangement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SongStructure {
    pub form: SongForm,
    pub sections: Vec<SongSection>,
    pub total_bars: u16,
}

impl SongStructure {
    /// Creates a deterministic section layout for the chosen form.
    pub fn build(form: SongForm) -> Self {
        let template: &[(&str, u16)] = match form {
            SongForm::Compact => &[
                ("Intro", 4),
                ("Verse 1", 4),
                ("Build-up", 4),
                ("Chorus", 4),
                ("Verse 2", 4),
                ("Bridge", 4),
                ("Final Chorus", 4),
                ("Outro", 4),
            ],
            SongForm::Standard => &[
                ("Intro", 4),
                ("Verse 1", 8),
                ("Build-up", 4),
                ("Chorus", 8),
                ("Verse 2", 8),
                ("Bridge", 4),
                ("Final Chorus", 8),
                ("Outro", 4),
            ],
            SongForm::Extended => &[
                ("Intro", 4),
                ("Verse 1", 8),
                ("Build-up", 4),
                ("Chorus", 8),
                ("Verse 2", 8),
                ("Build-up 2", 4),
                ("Chorus 2", 8),
                ("Bridge", 8),
                ("Final Chorus", 8),
                ("Outro", 4),
            ],
        };

        let mut cursor = 0_u16;
        let sections = template
            .iter()
            .map(|&(name, bars)| {
                let section = SongSection {
                    name,
                    start_bar: cursor,
                    end_bar: cursor + bars,
                };
                cursor += bars;
                section
            })
            .collect();

        Self {
            form,
            sections,
            total_bars: cursor,
        }
    }
}

/// Requested song length, constrained to the supported one-to-four-minute range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongDuration {
    seconds: u16,
}

impl SongDuration {
    pub const MIN_SECONDS: u16 = 60;
    pub const MAX_SECONDS: u16 = 240;
    pub const STEP_SECONDS: u16 = 15;

    /// Available user-selectable lengths: 1:00, 1:15, ... 4:00.
    pub const PRESETS: [u16; 13] = [60, 75, 90, 105, 120, 135, 150, 165, 180, 195, 210, 225, 240];

    pub fn new(seconds: u16) -> Self {
        Self {
            seconds: seconds.clamp(Self::MIN_SECONDS, Self::MAX_SECONDS),
        }
    }

    /// Selects the nearest available preset for an arbitrary requested duration.
    pub fn from_seconds(seconds: u16) -> Self {
        let clamped = seconds.clamp(Self::MIN_SECONDS, Self::MAX_SECONDS);
        let nearest = ((clamped - Self::MIN_SECONDS + Self::STEP_SECONDS / 2) / Self::STEP_SECONDS)
            * Self::STEP_SECONDS;
        Self::new(Self::MIN_SECONDS + nearest)
    }

    /// Constructs a duration from whole minutes (clamped to 1..=4).
    pub fn from_minutes(minutes: u8) -> Self {
        Self::new(u16::from(minutes.clamp(1, 4)) * 60)
    }

    pub fn seconds(self) -> u16 {
        self.seconds
    }

    /// Display label suitable for a duration picker.
    pub fn label(self) -> String {
        format!("{}:{:02}", self.seconds / 60, self.seconds % 60)
    }

    /// All selectable durations in ascending order.
    pub fn choices() -> &'static [u16; 13] {
        &Self::PRESETS
    }
}

impl Default for SongDuration {
    fn default() -> Self {
        Self::new(180)
    }
}

impl SongStructure {
    /// Builds a full-song section plan sized for the requested duration.
    /// Whole-bar quantization means actual duration may differ slightly.
    pub fn for_duration(duration: SongDuration, tempo_bpm: f64, beats_per_bar: u8) -> Self {
        let bpm = if tempo_bpm.is_finite() {
            tempo_bpm.clamp(20.0, 400.0)
        } else {
            120.0
        };
        let beats = f64::from(beats_per_bar.max(1));
        let bar_seconds = 60.0 * beats / bpm;
        let total_bars = (f64::from(duration.seconds()) / bar_seconds)
            .round()
            .max(8.0)
            .min(f64::from(u16::MAX)) as u16;

        const WEIGHTS: [u16; 8] = [1, 2, 1, 2, 2, 1, 2, 1];
        const NAMES: [&str; 8] = [
            "Intro",
            "Verse 1",
            "Build-up",
            "Chorus",
            "Verse 2",
            "Bridge",
            "Final Chorus",
            "Outro",
        ];
        let weight_sum: u32 = WEIGHTS.iter().map(|&w| u32::from(w)).sum();
        let mut cursor = 0_u16;
        let mut sections = Vec::with_capacity(NAMES.len());

        for index in 0..NAMES.len() {
            let end = if index == NAMES.len() - 1 {
                total_bars
            } else {
                let cumulative: u32 = WEIGHTS[..=index].iter().map(|&w| u32::from(w)).sum();
                ((u32::from(total_bars) * cumulative + weight_sum / 2) / weight_sum) as u16
            };
            let remaining = (NAMES.len() - index - 1) as u16;
            let end = end.max(cursor + 1).min(total_bars - remaining);
            sections.push(SongSection {
                name: NAMES[index],
                start_bar: cursor,
                end_bar: end,
            });
            cursor = end;
        }

        Self {
            form: SongForm::Standard,
            sections,
            total_bars,
        }
    }

    pub fn estimated_duration_seconds(&self, tempo_bpm: f64, beats_per_bar: u8) -> f64 {
        let bpm = if tempo_bpm.is_finite() {
            tempo_bpm.clamp(20.0, 400.0)
        } else {
            120.0
        };
        f64::from(self.total_bars) * f64::from(beats_per_bar.max(1)) * 60.0 / bpm
    }
}

/// The generated parts of a full arrangement. Each vector is intended for its own MIDI lane.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedParts {
    pub drums: Vec<crate::song::Event>,
    pub bass: Vec<crate::song::Event>,
    pub piano: Vec<crate::song::Event>,
    pub melody: Vec<crate::song::Event>,
}

/// The instrument to place on a generated notes channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedInstrument {
    Drums,
    Synth(crate::song::SynthKind),
    Plugin(&'static str),
}

/// Which generated instrument layers should become new project channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerSelection {
    pub drums: bool,
    pub bass: bool,
    pub piano: bool,
    pub melody: bool,
    pub bass_instrument: GeneratedInstrument,
    pub piano_instrument: GeneratedInstrument,
    pub melody_instrument: GeneratedInstrument,
}

impl Default for LayerSelection {
    fn default() -> Self {
        Self {
            drums: true,
            bass: true,
            piano: true,
            melody: true,
            bass_instrument: GeneratedInstrument::Synth(crate::song::SynthKind::TransistorBass),
            piano_instrument: GeneratedInstrument::Synth(crate::song::SynthKind::MiniSynth),
            melody_instrument: GeneratedInstrument::Synth(crate::song::SynthKind::SuperSaw),
        }
    }
}

/// Generates a complete drum part over every section, with restrained intro/bridge and stronger choruses.
/// Pad indices follow the default drum kit: kick 0, snare 1, clap 2, closed hat 3, open hat 4.
pub fn generate_drum_events(
    structure: &SongStructure,
    beats_per_bar: u8,
    seed: u64,
) -> Vec<crate::song::Event> {
    use crate::song::{Event, PPQ};
    let bpb = u32::from(beats_per_bar.max(1));
    let mut events = Vec::new();
    for section in &structure.sections {
        for bar in section.start_bar..section.end_bar {
            let section_name = section.name.to_ascii_lowercase();
            let intro_or_outro = section_name.contains("intro") || section_name.contains("outro");
            let bridge = section_name.contains("bridge");
            let chorus = section_name.contains("chorus");
            let bar_tick = u32::from(bar) * bpb * PPQ;
            for beat in 0..bpb {
                let kick = beat == 0 || (!intro_or_outro && !bridge && bpb >= 4 && beat == 2);
                if kick {
                    events.push(Event::Drum {
                        pos: bar_tick + beat * PPQ,
                        len_beats: 0.25,
                        pad: 0,
                        velocity: if chorus { 235 } else { 205 },
                        pan: Event::DEFAULT_PAN,
                        value: Event::DEFAULT_PITCH,
                        repeat: 0,
                    });
                }
                if bpb >= 4 && (beat == 1 || beat == 3) && !intro_or_outro {
                    events.push(Event::Drum {
                        pos: bar_tick + beat * PPQ,
                        len_beats: 0.25,
                        pad: if chorus && beat == 3 { 2 } else { 1 },
                        velocity: if chorus { 225 } else { 195 },
                        pan: Event::DEFAULT_PAN,
                        value: Event::DEFAULT_PITCH,
                        repeat: 0,
                    });
                }
            }
            for step in 0..(bpb * 2) {
                if !intro_or_outro || step % 2 == 0 {
                    events.push(Event::Drum {
                        pos: bar_tick + step * PPQ / 2,
                        len_beats: 0.125,
                        pad: if chorus && step + 1 == bpb * 2 { 4 } else { 3 },
                        velocity: if step % 2 == 0 { 150 } else { 112 },
                        pan: Event::DEFAULT_PAN,
                        value: Event::DEFAULT_PITCH,
                        repeat: 0,
                    });
                }
            }
            if !intro_or_outro && (u64::from(bar) + seed) % 8 == 7 && bpb >= 4 {
                events.push(Event::Drum {
                    pos: bar_tick + (bpb - 1) * PPQ + PPQ / 2,
                    len_beats: 0.125,
                    pad: 2,
                    velocity: 185,
                    pan: Event::DEFAULT_PAN,
                    value: Event::DEFAULT_PITCH,
                    repeat: 0,
                });
            }
        }
    }
    events.sort_by_key(Event::pos);
    events
}

/// Generates a piano chord part in the requested key. Chords change once per bar and follow I–V–vi–IV.
pub fn generate_piano_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    _seed: u64,
) -> Vec<crate::song::Event> {
    use crate::song::{Event, PPQ};
    let bpb = f64::from(beats_per_bar.max(1));
    let scale: [i32; 7] = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    let degrees: [usize; 4] = if minor { [0, 5, 2, 6] } else { [0, 4, 5, 3] };
    let mut events = Vec::new();
    for bar in 0..structure.total_bars {
        let degree = degrees[usize::from(bar) % degrees.len()];
        let section = structure
            .sections
            .iter()
            .find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        let chord = [degree, degree + 2, degree + 4];
        for scale_index in chord {
            let key = (60
                + i32::from(root % 12)
                + (scale_index / 7) as i32 * 12
                + scale[scale_index % 7])
                .clamp(0, 127) as u8;
            let pos = (f64::from(bar) * bpb * f64::from(PPQ)).round() as u32;
            events.push(Event::Note {
                pos,
                len_beats: if sparse { bpb } else { (bpb / 2.0).max(0.5) },
                key,
                velocity: if sparse { 145 } else { 180 },
                pan: Event::DEFAULT_PAN,
                slide: false,
            });
        }
    }
    events
}

/// Generates a bass line that follows the chord roots, with a simple section-aware rhythm.
pub fn generate_bass_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    _seed: u64,
) -> Vec<crate::song::Event> {
    use crate::song::{Event, PPQ};
    let bpb = u32::from(beats_per_bar.max(1));
    let scale = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    let degrees = if minor {
        [0_usize, 5, 2, 6]
    } else {
        [0_usize, 4, 5, 3]
    };
    let mut events = Vec::new();
    for bar in 0..structure.total_bars {
        let section = structure
            .sections
            .iter()
            .find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        let degree = degrees[usize::from(bar) % degrees.len()];
        let key = (36 + i32::from(root % 12) + scale[degree]).clamp(0, 127) as u8;
        let start = u32::from(bar) * bpb * PPQ;
        let positions: &[u32] = if sparse || bpb < 4 { &[0] } else { &[0, 2] };
        for &beat in positions {
            events.push(Event::Note {
                pos: start + beat * PPQ,
                len_beats: 0.9,
                key,
                velocity: 190,
                pan: Event::DEFAULT_PAN,
                slide: false,
            });
        }
    }
    events
}

/// Uses the trained pitch-only NoteLM when its checkpoint is packaged; otherwise returns None.
fn generate_ai_melody_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    seed: u64,
) -> Option<Vec<crate::song::Event>> {
    use crate::song::{Event, PPQ};
    use std::sync::OnceLock;

    static MODEL: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    let weights = MODEL.get_or_init(crate::song::ai_model::load_model_weights).as_deref()?;
    let scale: [u8; 7] = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    // A short in-key prompt initializes the LSTM state. The model then chooses subsequent pitches.
    const SEED_MOTIF: [usize; 8] = [0, 2, 4, 2, 1, 3, 5, 3];
    let context: Vec<u8> = SEED_MOTIF
        .iter()
        .map(|degree| 72_u16 + u16::from(root % 12) + u16::from(scale[*degree]))
        .map(|pitch| pitch.min(127) as u8)
        .collect();

    let bpb = u32::from(beats_per_bar.max(1));
    let mut count = 0_usize;
    for bar in 0..structure.total_bars {
        let section = structure.sections.iter().find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        count += if sparse { 2 } else { 4 };
    }
    let pitches = crate::song::ai_model::generate_pitch_sequence(
        weights, &context, count, root, minor, seed,
    )?;
    let mut pitches = pitches.into_iter();
    let mut events = Vec::with_capacity(count);
    for bar in 0..structure.total_bars {
        let section = structure.sections.iter().find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        let steps = if sparse { 2_u32 } else { 4_u32 };
        let start = u32::from(bar) * bpb * PPQ;
        for step in 0..steps {
            let beat = step * bpb / steps;
            let key = pitches.next()?;
            events.push(Event::Note {
                pos: start + beat * PPQ,
                len_beats: 0.45,
                key,
                velocity: if sparse { 145 } else { 175 },
                pan: Event::DEFAULT_PAN,
                slide: false,
            });
        }
    }
    Some(events)
}

/// Generates a scale-constrained lead motif with section-dependent density.
/// If the trained NoteLM weights are present, they choose pitches; the arrangement still controls timing.
pub fn generate_melody_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    seed: u64,
//! Full-song arrangement templates. This is the first generation layer: it plans song sections
//! in bars before MIDI/event generation and UI wiring are added.

/// Musical hints extracted from a natural-language generation brief.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptProfile {
    pub minor: Option<bool>,
    pub tempo_bpm: Option<u16>,
    pub energy: u8,
    pub genre: Genre,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    Electronic,
    House,
    Techno,
    DrumAndBass,
    Trap,
    HipHop,
    Pop,
    Rock,
    Ambient,
    Cinematic,
    Unknown,
}

impl PromptProfile {
    /// Lightweight offline interpretation. It deliberately extracts explicit musical constraints;
    /// it does not claim to be a remote generative AI model.
    pub fn parse(description: &str) -> Self {
        let text = description.to_lowercase();
        let minor = if [
            "minor",
            "dark",
            "sad",
            "melancholic",
            "мрач",
            "груст",
            "минор",
        ]
        .iter()
        .any(|word| text.contains(word))
        {
            Some(true)
        } else if [
            "major",
            "happy",
            "bright",
            "uplifting",
            "весел",
            "мажор",
            "светл",
        ]
        .iter()
        .any(|word| text.contains(word))
        {
            Some(false)
        } else {
            None
        };
        let genre = if text.contains("drum and bass")
            || text.contains("dnb")
            || text.contains("драм-н-бейс")
        {
            Genre::DrumAndBass
        } else if text.contains("house") || text.contains("хаус") {
            Genre::House
        } else if text.contains("techno") || text.contains("техно") {
            Genre::Techno
        } else if text.contains("trap") || text.contains("трэп") {
            Genre::Trap
        } else if text.contains("hip-hop") || text.contains("hip hop") || text.contains("хип-хоп")
        {
            Genre::HipHop
        } else if text.contains("rock") || text.contains("рок") {
            Genre::Rock
        } else if text.contains("pop") || text.contains("поп") {
            Genre::Pop
        } else if text.contains("ambient") || text.contains("эмбиент") {
            Genre::Ambient
        } else if text.contains("cinematic") || text.contains("кинемат") {
            Genre::Cinematic
        } else if text.contains("edm") || text.contains("electronic") || text.contains("электрон")
        {
            Genre::Electronic
        } else {
            Genre::Unknown
        };
        let energy = if [
            "aggressive",
            "energetic",
            " мощ",
            "энерг",
            "жестк",
            "интенсив",
        ]
        .iter()
        .any(|word| text.contains(word.trim()))
        {
            90
        } else if ["calm", "soft", "chill", "спокой", "мягк", "расслаб"]
            .iter()
            .any(|word| text.contains(word))
        {
            25
        } else {
            60
        };
        let tempo_bpm = text
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<u16>().ok())
            .find(|bpm| (40..=220).contains(bpm));
        Self {
            minor,
            tempo_bpm,
            energy,
            genre,
        }
    }
}

/// Length preset for a full arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongForm {
    /// 32 bars: compact, with two shorter chorus passes.
    Compact,
    /// 48 bars: conventional verse/chorus form with a bridge and final chorus.
    Standard,
    /// 64 bars: longer form with repeated chorus and an extended bridge.
    Extended,
}

/// A named section occupying a contiguous range of bars (start is zero-based, end is exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongSection {
    pub name: &'static str,
    pub start_bar: u16,
    pub end_bar: u16,
}

impl SongSection {
    /// Length of this section in bars.
    pub fn bars(self) -> u16 {
        self.end_bar - self.start_bar
    }
}

/// A complete section plan, with contiguous sections covering the full arrangement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SongStructure {
    pub form: SongForm,
    pub sections: Vec<SongSection>,
    pub total_bars: u16,
}

impl SongStructure {
    /// Creates a deterministic section layout for the chosen form.
    pub fn build(form: SongForm) -> Self {
        let template: &[(&str, u16)] = match form {
            SongForm::Compact => &[
                ("Intro", 4),
                ("Verse 1", 4),
                ("Build-up", 4),
                ("Chorus", 4),
                ("Verse 2", 4),
                ("Bridge", 4),
                ("Final Chorus", 4),
                ("Outro", 4),
            ],
            SongForm::Standard => &[
                ("Intro", 4),
                ("Verse 1", 8),
                ("Build-up", 4),
                ("Chorus", 8),
                ("Verse 2", 8),
                ("Bridge", 4),
                ("Final Chorus", 8),
                ("Outro", 4),
            ],
            SongForm::Extended => &[
                ("Intro", 4),
                ("Verse 1", 8),
                ("Build-up", 4),
                ("Chorus", 8),
                ("Verse 2", 8),
                ("Build-up 2", 4),
                ("Chorus 2", 8),
                ("Bridge", 8),
                ("Final Chorus", 8),
                ("Outro", 4),
            ],
        };

        let mut cursor = 0_u16;
        let sections = template
            .iter()
            .map(|&(name, bars)| {
                let section = SongSection {
                    name,
                    start_bar: cursor,
                    end_bar: cursor + bars,
                };
                cursor += bars;
                section
            })
            .collect();

        Self {
            form,
            sections,
            total_bars: cursor,
        }
    }
}

/// Requested song length, constrained to the supported one-to-four-minute range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongDuration {
    seconds: u16,
}

impl SongDuration {
    pub const MIN_SECONDS: u16 = 60;
    pub const MAX_SECONDS: u16 = 240;
    pub const STEP_SECONDS: u16 = 15;

    /// Available user-selectable lengths: 1:00, 1:15, ... 4:00.
    pub const PRESETS: [u16; 13] = [60, 75, 90, 105, 120, 135, 150, 165, 180, 195, 210, 225, 240];

    pub fn new(seconds: u16) -> Self {
        Self {
            seconds: seconds.clamp(Self::MIN_SECONDS, Self::MAX_SECONDS),
        }
    }

    /// Selects the nearest available preset for an arbitrary requested duration.
    pub fn from_seconds(seconds: u16) -> Self {
        let clamped = seconds.clamp(Self::MIN_SECONDS, Self::MAX_SECONDS);
        let nearest = ((clamped - Self::MIN_SECONDS + Self::STEP_SECONDS / 2) / Self::STEP_SECONDS)
            * Self::STEP_SECONDS;
        Self::new(Self::MIN_SECONDS + nearest)
    }

    /// Constructs a duration from whole minutes (clamped to 1..=4).
    pub fn from_minutes(minutes: u8) -> Self {
        Self::new(u16::from(minutes.clamp(1, 4)) * 60)
    }

    pub fn seconds(self) -> u16 {
        self.seconds
    }

    /// Display label suitable for a duration picker.
    pub fn label(self) -> String {
        format!("{}:{:02}", self.seconds / 60, self.seconds % 60)
    }

    /// All selectable durations in ascending order.
    pub fn choices() -> &'static [u16; 13] {
        &Self::PRESETS
    }
}

impl Default for SongDuration {
    fn default() -> Self {
        Self::new(180)
    }
}

impl SongStructure {
    /// Builds a full-song section plan sized for the requested duration.
    /// Whole-bar quantization means actual duration may differ slightly.
    pub fn for_duration(duration: SongDuration, tempo_bpm: f64, beats_per_bar: u8) -> Self {
        let bpm = if tempo_bpm.is_finite() {
            tempo_bpm.clamp(20.0, 400.0)
        } else {
            120.0
        };
        let beats = f64::from(beats_per_bar.max(1));
        let bar_seconds = 60.0 * beats / bpm;
        let total_bars = (f64::from(duration.seconds()) / bar_seconds)
            .round()
            .max(8.0)
            .min(f64::from(u16::MAX)) as u16;

        const WEIGHTS: [u16; 8] = [1, 2, 1, 2, 2, 1, 2, 1];
        const NAMES: [&str; 8] = [
            "Intro",
            "Verse 1",
            "Build-up",
            "Chorus",
            "Verse 2",
            "Bridge",
            "Final Chorus",
            "Outro",
        ];
        let weight_sum: u32 = WEIGHTS.iter().map(|&w| u32::from(w)).sum();
        let mut cursor = 0_u16;
        let mut sections = Vec::with_capacity(NAMES.len());

        for index in 0..NAMES.len() {
            let end = if index == NAMES.len() - 1 {
                total_bars
            } else {
                let cumulative: u32 = WEIGHTS[..=index].iter().map(|&w| u32::from(w)).sum();
                ((u32::from(total_bars) * cumulative + weight_sum / 2) / weight_sum) as u16
            };
            let remaining = (NAMES.len() - index - 1) as u16;
            let end = end.max(cursor + 1).min(total_bars - remaining);
            sections.push(SongSection {
                name: NAMES[index],
                start_bar: cursor,
                end_bar: end,
            });
            cursor = end;
        }

        Self {
            form: SongForm::Standard,
            sections,
            total_bars,
        }
    }

    pub fn estimated_duration_seconds(&self, tempo_bpm: f64, beats_per_bar: u8) -> f64 {
        let bpm = if tempo_bpm.is_finite() {
            tempo_bpm.clamp(20.0, 400.0)
        } else {
            120.0
        };
        f64::from(self.total_bars) * f64::from(beats_per_bar.max(1)) * 60.0 / bpm
    }
}

/// The generated parts of a full arrangement. Each vector is intended for its own MIDI lane.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedParts {
    pub drums: Vec<crate::song::Event>,
    pub bass: Vec<crate::song::Event>,
    pub piano: Vec<crate::song::Event>,
    pub melody: Vec<crate::song::Event>,
}

/// The instrument to place on a generated notes channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedInstrument {
    Drums,
    Synth(crate::song::SynthKind),
    Plugin(&'static str),
}

/// Which generated instrument layers should become new project channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerSelection {
    pub drums: bool,
    pub bass: bool,
    pub piano: bool,
    pub melody: bool,
    pub bass_instrument: GeneratedInstrument,
    pub piano_instrument: GeneratedInstrument,
    pub melody_instrument: GeneratedInstrument,
}

impl Default for LayerSelection {
    fn default() -> Self {
        Self {
            drums: true,
            bass: true,
            piano: true,
            melody: true,
            bass_instrument: GeneratedInstrument::Synth(crate::song::SynthKind::TransistorBass),
            piano_instrument: GeneratedInstrument::Synth(crate::song::SynthKind::MiniSynth),
            melody_instrument: GeneratedInstrument::Synth(crate::song::SynthKind::SuperSaw),
        }
    }
}

/// Generates a complete drum part over every section, with restrained intro/bridge and stronger choruses.
/// Pad indices follow the default drum kit: kick 0, snare 1, clap 2, closed hat 3, open hat 4.
pub fn generate_drum_events(
    structure: &SongStructure,
    beats_per_bar: u8,
    seed: u64,
) -> Vec<crate::song::Event> {
    use crate::song::{Event, PPQ};
    let bpb = u32::from(beats_per_bar.max(1));
    let mut events = Vec::new();
    for section in &structure.sections {
        for bar in section.start_bar..section.end_bar {
            let section_name = section.name.to_ascii_lowercase();
            let intro_or_outro = section_name.contains("intro") || section_name.contains("outro");
            let bridge = section_name.contains("bridge");
            let chorus = section_name.contains("chorus");
            let bar_tick = u32::from(bar) * bpb * PPQ;
            for beat in 0..bpb {
                let kick = beat == 0 || (!intro_or_outro && !bridge && bpb >= 4 && beat == 2);
                if kick {
                    events.push(Event::Drum {
                        pos: bar_tick + beat * PPQ,
                        len_beats: 0.25,
                        pad: 0,
                        velocity: if chorus { 235 } else { 205 },
                        pan: Event::DEFAULT_PAN,
                        value: Event::DEFAULT_PITCH,
                        repeat: 0,
                    });
                }
                if bpb >= 4 && (beat == 1 || beat == 3) && !intro_or_outro {
                    events.push(Event::Drum {
                        pos: bar_tick + beat * PPQ,
                        len_beats: 0.25,
                        pad: if chorus && beat == 3 { 2 } else { 1 },
                        velocity: if chorus { 225 } else { 195 },
                        pan: Event::DEFAULT_PAN,
                        value: Event::DEFAULT_PITCH,
                        repeat: 0,
                    });
                }
            }
            for step in 0..(bpb * 2) {
                if !intro_or_outro || step % 2 == 0 {
                    events.push(Event::Drum {
                        pos: bar_tick + step * PPQ / 2,
                        len_beats: 0.125,
                        pad: if chorus && step + 1 == bpb * 2 { 4 } else { 3 },
                        velocity: if step % 2 == 0 { 150 } else { 112 },
                        pan: Event::DEFAULT_PAN,
                        value: Event::DEFAULT_PITCH,
                        repeat: 0,
                    });
                }
            }
            if !intro_or_outro && (u64::from(bar) + seed) % 8 == 7 && bpb >= 4 {
                events.push(Event::Drum {
                    pos: bar_tick + (bpb - 1) * PPQ + PPQ / 2,
                    len_beats: 0.125,
                    pad: 2,
                    velocity: 185,
                    pan: Event::DEFAULT_PAN,
                    value: Event::DEFAULT_PITCH,
                    repeat: 0,
                });
            }
        }
    }
    events.sort_by_key(Event::pos);
    events
}

/// Generates a piano chord part in the requested key. Chords change once per bar and follow I–V–vi–IV.
pub fn generate_piano_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    _seed: u64,
) -> Vec<crate::song::Event> {
    use crate::song::{Event, PPQ};
    let bpb = f64::from(beats_per_bar.max(1));
    let scale: [i32; 7] = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    let degrees: [usize; 4] = if minor { [0, 5, 2, 6] } else { [0, 4, 5, 3] };
    let mut events = Vec::new();
    for bar in 0..structure.total_bars {
        let degree = degrees[usize::from(bar) % degrees.len()];
        let section = structure
            .sections
            .iter()
            .find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        let chord = [degree, degree + 2, degree + 4];
        for scale_index in chord {
            let key = (60
                + i32::from(root % 12)
                + (scale_index / 7) as i32 * 12
                + scale[scale_index % 7])
                .clamp(0, 127) as u8;
            let pos = (f64::from(bar) * bpb * f64::from(PPQ)).round() as u32;
            events.push(Event::Note {
                pos,
                len_beats: if sparse { bpb } else { (bpb / 2.0).max(0.5) },
                key,
                velocity: if sparse { 145 } else { 180 },
                pan: Event::DEFAULT_PAN,
                slide: false,
            });
        }
    }
    events
}

/// Generates a bass line that follows the chord roots, with a simple section-aware rhythm.
pub fn generate_bass_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    _seed: u64,
) -> Vec<crate::song::Event> {
    use crate::song::{Event, PPQ};
    let bpb = u32::from(beats_per_bar.max(1));
    let scale = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    let degrees = if minor {
        [0_usize, 5, 2, 6]
    } else {
        [0_usize, 4, 5, 3]
    };
    let mut events = Vec::new();
    for bar in 0..structure.total_bars {
        let section = structure
            .sections
            .iter()
            .find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        let degree = degrees[usize::from(bar) % degrees.len()];
        let key = (36 + i32::from(root % 12) + scale[degree]).clamp(0, 127) as u8;
        let start = u32::from(bar) * bpb * PPQ;
        let positions: &[u32] = if sparse || bpb < 4 { &[0] } else { &[0, 2] };
        for &beat in positions {
            events.push(Event::Note {
                pos: start + beat * PPQ,
                len_beats: 0.9,
                key,
                velocity: 190,
                pan: Event::DEFAULT_PAN,
                slide: false,
            });
        }
    }
    events
}

/// Uses the trained pitch-only NoteLM when its checkpoint is packaged; otherwise returns None.
fn generate_ai_melody_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    seed: u64,
) -> Option<Vec<crate::song::Event>> {
    use crate::song::{Event, PPQ};
    use std::sync::OnceLock;

    static MODEL: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    let weights = MODEL.get_or_init(crate::song::ai_model::load_model_weights).as_deref()?;
    let scale: [u8; 7] = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    // A short in-key prompt initializes the LSTM state. The model then chooses subsequent pitches.
    const SEED_MOTIF: [usize; 8] = [0, 2, 4, 2, 1, 3, 5, 3];
    let context: Vec<u8> = SEED_MOTIF
        .iter()
        .map(|degree| 72_u16 + u16::from(root % 12) + u16::from(scale[*degree]))
        .map(|pitch| pitch.min(127) as u8)
        .collect();

    let bpb = u32::from(beats_per_bar.max(1));
    let mut count = 0_usize;
    for bar in 0..structure.total_bars {
        let section = structure.sections.iter().find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        count += if sparse { 2 } else { 4 };
    }
    let pitches = crate::song::ai_model::generate_pitch_sequence(
        weights, &context, count, root, minor, seed,
    )?;
    let mut pitches = pitches.into_iter();
    let mut events = Vec::with_capacity(count);
    for bar in 0..structure.total_bars {
        let section = structure.sections.iter().find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        let steps = if sparse { 2_u32 } else { 4_u32 };
        let start = u32::from(bar) * bpb * PPQ;
        for step in 0..steps {
            let beat = step * bpb / steps;
            let key = pitches.next()?;
            events.push(Event::Note {
                pos: start + beat * PPQ,
                len_beats: 0.45,
                key,
                velocity: if sparse { 145 } else { 175 },
                pan: Event::DEFAULT_PAN,
                slide: false,
            });
        }
    }
    Some(events)
}

/// Generates a scale-constrained lead motif with section-dependent density.
/// If the trained NoteLM weights are present, they choose pitches; the arrangement still controls timing.
pub fn generate_melody_events(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    seed: u64,
 ) -> Vec<crate::song::Event> {
    if let Some(events) = generate_ai_melody_events(structure, root, minor, beats_per_bar, seed) {
        return events;
    }
    use crate::song::{Event, PPQ};
    let bpb = u32::from(beats_per_bar.max(1));
    let scale = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    let mut events = Vec::new();
    for bar in 0..structure.total_bars {
        let section = structure
            .sections
            .iter()
            .find(|s| bar >= s.start_bar && bar < s.end_bar);
        let sparse = section.is_some_and(|s| {
            let name = s.name.to_ascii_lowercase();
            name.contains("intro") || name.contains("bridge") || name.contains("outro")
        });
        let start = u32::from(bar) * bpb * PPQ;
        let steps: u32 = if sparse { 2 } else { 4 };
        for step in 0..steps {
            let beat = step * bpb / steps;
            let phrase = (usize::from(bar) % 2) * 4 + step as usize;
            // A recurring two-bar motif gives the melody a recognizable identity instead of
            // choosing unrelated scale notes on every beat. Seed changes the motif's starting degree.
            const MOTIF: [usize; 8] = [0, 2, 4, 2, 1, 3, 5, 3];
            let motif_degree = (MOTIF[phrase] + (seed as usize % 3)) % 7;
            let chorus = section.is_some_and(|s| s.name.to_ascii_lowercase().contains("chorus"));
            let octave = if chorus { 12 } else { 0 };
            let key = (72 + octave + i32::from(root % 12) + scale[motif_degree]).clamp(0, 127) as u8;
            events.push(Event::Note {
                pos: start + beat * PPQ,
                len_beats: 0.45,
                key,
                velocity: if sparse { 145 } else { 175 },
                pan: Event::DEFAULT_PAN,
                slide: false,
            });
        }
    }
    events
}

/// Builds all four independently editable layers for the arrangement.
pub fn generate_parts(
    structure: &SongStructure,
    root: u8,
    minor: bool,
    beats_per_bar: u8,
    seed: u64,
) -> GeneratedParts {
    GeneratedParts {
        drums: generate_drum_events(structure, beats_per_bar, seed),
        bass: generate_bass_events(structure, root, minor, beats_per_bar, seed),
        piano: generate_piano_events(structure, root, minor, beats_per_bar, seed),
        melody: generate_melody_events(structure, root, minor, beats_per_bar, seed),
    }
}

/// Adds selected generated layers as brand-new channels and clips. Existing project channels are never modified.
pub fn create_channels(
    song: &mut crate::song::Song,
    structure: &SongStructure,
    root: u8,
    minor: bool,
    selection: LayerSelection,
    seed: u64,
) -> Vec<usize> {
    use crate::song::ChannelKind;
    if !(selection.drums || selection.bass || selection.piano || selection.melody) {
        return Vec::new();
    }
    // Generated channels are explicitly tagged so the next full-song generation can replace
    // only its own previous output and leave the user's hand-made tracks untouched.
    let mut previous: Vec<usize> = song.channels.iter().enumerate()
        .filter_map(|(index, channel)| (index > 0 && channel.name.starts_with("AI Generated - ")).then_some(index))
        .collect();
    previous.sort_unstable_by(|a, b| b.cmp(a));
    for index in previous { song.delete_channel(index); }

    let bpb = song.time_sig.bar_beats();
    let parts = generate_parts(structure, root, minor, bpb as u8, seed);
    let loop_beats = f64::from(structure.total_bars) * f64::from(bpb);
    if !(selection.drums || selection.bass || selection.piano || selection.melody) {
        return Vec::new();
    }
    let mut created = Vec::new();

    let mut add = |instrument: GeneratedInstrument, lane_kind, events: Vec<crate::song::Event>| {
        let kind = match instrument {
            GeneratedInstrument::Drums => ChannelKind::Drums,
            GeneratedInstrument::Synth(synth) => ChannelKind::Synth(synth),
            GeneratedInstrument::Plugin(_) => ChannelKind::Instrument,
        };
        let Some(channel_index) = song.insert_channel(song.channels.len(), kind) else {
            return;
        };
        let mut failed = false;
        {
            let channel = &mut song.channels[channel_index];
            channel.name = format!("AI Generated - {}", channel.name);
            if channel
                .lanes
                .first()
                .is_none_or(|lane| lane.kind != lane_kind)
            {
                failed = true;
            } else {
                if let GeneratedInstrument::Plugin(uid) = instrument
                    && crate::plugins::add(&mut channel.rack, channel.id, uid, 0).is_err()
                {
                    failed = true;
                }
                if !failed {
                    let clip = channel.lanes[0].add_clip(0, loop_beats);
                    clip.events = events;
                    clip.fit_loop(bpb);
                    // The internal loop is bar-quantized; the visible clip ends at the requested arrangement length.
                    clip.visible_len_beats = loop_beats;
                }
            }
        }
        if failed {
            song.delete_channel(channel_index);
        } else {
            created.push(channel_index);
        }
    };

    if selection.drums {
        add(
            GeneratedInstrument::Drums,
            crate::song::LaneKind::Drums,
            parts.drums,
        );
    }
    if selection.bass {
        add(
            selection.bass_instrument,
            crate::song::LaneKind::Notes,
            parts.bass,
        );
    }
    if selection.piano {
        add(
            selection.piano_instrument,
            crate::song::LaneKind::Notes,
            parts.piano,
        );
    }
    if selection.melody {
        add(
            selection.melody_instrument,
            crate::song::LaneKind::Notes,
            parts.melody,
        );
    }
    if !created.is_empty() {
        for section in &structure.sections {
            let start = u32::from(section.start_bar) * bpb * crate::song::PPQ;
            let end = u32::from(section.end_bar) * bpb * crate::song::PPQ;
            let index = song.section_at(start, end);
            song.sections[index].name = section.name.to_owned();
        }
    }
    created
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_parser_extracts_genre_mood_and_tempo() {
        let profile =
            PromptProfile::parse("Dark melodic drum and bass, 174 BPM, powerful and energetic");
        assert_eq!(profile.genre, Genre::DrumAndBass);
        assert_eq!(profile.minor, Some(true));
        assert_eq!(profile.tempo_bpm, Some(174));
        assert_eq!(profile.energy, 90);

        let calm = PromptProfile::parse("Спокойный светлый ambient");
        assert_eq!(calm.genre, Genre::Ambient);
        assert_eq!(calm.minor, Some(false));
        assert_eq!(calm.energy, 25);
        let russian = PromptProfile::parse("СПОКОЙНЫЙ светлый драм-н-бейс");
        assert_eq!(russian.genre, Genre::DrumAndBass);
        assert_eq!(russian.minor, Some(false));
        assert_eq!(russian.energy, 25);
    }

    #[test]
    fn duration_presets_are_selectable_and_labeled() {
        assert_eq!(SongDuration::choices().len(), 13);
        assert_eq!(SongDuration::from_minutes(1).label(), "1:00");
        assert_eq!(SongDuration::from_minutes(4).label(), "4:00");
        assert_eq!(SongDuration::from_seconds(100).seconds(), 105);
        assert_eq!(SongDuration::from_seconds(101).seconds(), 105);
        assert_eq!(SongDuration::from_seconds(240).seconds(), 240);
    }

    #[test]
    fn duration_is_clamped_and_structure_covers_requested_length() {
        assert_eq!(SongDuration::new(10).seconds(), 60);
        assert_eq!(SongDuration::new(500).seconds(), 240);
        for seconds in [60, 90, 180, 240] {
            let structure = SongStructure::for_duration(SongDuration::new(seconds), 120.0, 4);
            assert_eq!(structure.sections.first().unwrap().start_bar, 0);
            assert_eq!(
                structure.sections.last().unwrap().end_bar,
                structure.total_bars
            );
            assert!(structure.sections.iter().all(|s| s.bars() > 0));
            for pair in structure.sections.windows(2) {
                assert_eq!(pair[0].end_bar, pair[1].start_bar);
            }
            assert!(
                (structure.estimated_duration_seconds(120.0, 4) - f64::from(seconds)).abs() <= 1.0
            );
        }
    }

    #[test]
    fn generated_parts_cover_the_arrangement_and_use_separate_event_types() {
        use crate::song::Event;
        let structure = SongStructure::for_duration(SongDuration::new(60), 120.0, 4);
        let parts = generate_parts(&structure, 0, false, 4, 0);
        assert!(!parts.drums.is_empty());
        assert!(!parts.piano.is_empty());
        assert!(!parts.bass.is_empty());
        assert!(!parts.melody.is_empty());
        assert!(
            parts
                .drums
                .iter()
                .all(|event| matches!(event, Event::Drum { .. }))
        );
        assert!(
            parts
                .piano
                .iter()
                .all(|event| matches!(event, Event::Note { .. }))
        );
        assert!(
            parts
                .bass
                .iter()
                .all(|event| matches!(event, Event::Note { .. }))
        );
        assert!(
            parts
                .melody
                .iter()
                .all(|event| matches!(event, Event::Note { .. }))
        );
        let end_tick = u32::from(structure.total_bars) * 4 * crate::song::PPQ;
        assert!(parts.drums.iter().all(|event| event.pos() < end_tick));
        assert!(parts.piano.iter().all(|event| event.pos() < end_tick));
    }

    #[test]
    fn selected_layers_create_new_editable_channels_without_touching_existing_ones() {
        use crate::song::{ChannelKind, LaneKind};
        let mut song = crate::song::Song::default();
        let original_count = song.channels.len();
        let structure = SongStructure::for_duration(SongDuration::new(60), 120.0, 4);
        let created = create_channels(
            &mut song,
            &structure,
            0,
            false,
            LayerSelection::default(),
            0,
        );
        assert_eq!(created.len(), 4);
        assert_eq!(song.channels.len(), original_count + 4);
        assert_eq!(song.channels[created[0]].kind, ChannelKind::Drums);
        assert_eq!(song.channels[created[0]].lanes[0].kind, LaneKind::Drums);
        assert!(
            song.channels[created[0]].lanes[0].clips[0]
                .events
                .iter()
                .all(|event| matches!(event, crate::song::Event::Drum { .. }))
        );
        assert!(created[1..].iter().all(|&i| {
            song.channels[i].lanes[0].kind == LaneKind::Notes
                && !song.channels[i].lanes[0].clips[0].events.is_empty()
        }));
    }

    #[test]
    fn generating_again_replaces_only_previous_ai_generated_channels() {
        let mut song = crate::song::Song::default();
        let user_channel = song.insert_channel(song.channels.len(), crate::song::ChannelKind::AudioClip).unwrap();
        song.channels[user_channel].name = "My own track".to_owned();
        let structure = SongStructure::for_duration(SongDuration::new(60), 120.0, 4);
        create_channels(&mut song, &structure, 0, false, LayerSelection::default(), 11);
        create_channels(&mut song, &structure, 0, false, LayerSelection::default(), 22);
        assert_eq!(song.channels.iter().filter(|c| c.name.starts_with("AI Generated - ")).count(), 4);
        assert!(song.channels.iter().any(|c| c.name == "My own track"));
    }

    #[test]
    fn generated_piano_chords_stay_in_the_selected_scale() {
        use crate::song::Event;
        let structure = SongStructure::build(SongForm::Compact);
        let events = generate_piano_events(&structure, 2, false, 4, 0);
        let allowed = [2, 4, 6, 7, 9, 11, 1];
        assert!(events.iter().all(|event| match event {
            Event::Note { key, .. } => allowed.contains(&(key % 12)),
            _ => false,
        }));
    }

    #[test]
    fn forms_have_expected_lengths_and_contiguous_sections() {
        for (form, expected_bars) in [
            (SongForm::Compact, 32),
            (SongForm::Standard, 48),
            (SongForm::Extended, 64),
        ] {
            let structure = SongStructure::build(form);
            assert_eq!(structure.total_bars, expected_bars);
            assert_eq!(structure.sections.first().unwrap().start_bar, 0);
            assert_eq!(structure.sections.last().unwrap().end_bar, expected_bars);
            for pair in structure.sections.windows(2) {
                assert_eq!(pair[0].end_bar, pair[1].start_bar);
            }
            assert!(structure.sections.iter().all(|section| section.bars() > 0));
        }
    }
}
