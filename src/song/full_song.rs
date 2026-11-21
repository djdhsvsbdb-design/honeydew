//! Full-song arrangement templates. This is the first generation layer: it plans song sections
//! in bars before MIDI/event generation and UI wiring are added.

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
                ("Intro", 4), ("Verse 1", 4), ("Build-up", 4), ("Chorus", 4),
                ("Verse 2", 4), ("Bridge", 4), ("Final Chorus", 4), ("Outro", 4),
            ],
            SongForm::Standard => &[
                ("Intro", 4), ("Verse 1", 8), ("Build-up", 4), ("Chorus", 8),
                ("Verse 2", 8), ("Bridge", 4), ("Final Chorus", 8), ("Outro", 4),
            ],
            SongForm::Extended => &[
                ("Intro", 4), ("Verse 1", 8), ("Build-up", 4), ("Chorus", 8),
                ("Verse 2", 8), ("Build-up 2", 4), ("Chorus 2", 8),
                ("Bridge", 8), ("Final Chorus", 8), ("Outro", 4),
            ],
        };

        let mut cursor = 0_u16;
        let sections = template.iter().map(|&(name, bars)| {
            let section = SongSection { name, start_bar: cursor, end_bar: cursor + bars };
            cursor += bars;
            section
        }).collect();

        Self { form, sections, total_bars: cursor }
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
    pub const PRESETS: [u16; 13] = [
        60, 75, 90, 105, 120, 135, 150, 165, 180, 195, 210, 225, 240,
    ];

    pub fn new(seconds: u16) -> Self {
        Self { seconds: seconds.clamp(Self::MIN_SECONDS, Self::MAX_SECONDS) }
    }

    /// Selects the nearest available preset for an arbitrary requested duration.
    pub fn from_seconds(seconds: u16) -> Self {
        let clamped = seconds.clamp(Self::MIN_SECONDS, Self::MAX_SECONDS);
        let nearest = ((clamped - Self::MIN_SECONDS + Self::STEP_SECONDS / 2)
            / Self::STEP_SECONDS) * Self::STEP_SECONDS;
        Self::new(Self::MIN_SECONDS + nearest)
    }

    /// Constructs a duration from whole minutes (clamped to 1..=4).
    pub fn from_minutes(minutes: u8) -> Self {
        Self::new(u16::from(minutes.clamp(1, 4)) * 60)
    }

    pub fn seconds(self) -> u16 { self.seconds }

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
    fn default() -> Self { Self::new(180) }
}

impl SongStructure {
    /// Builds a full-song section plan sized for the requested duration.
    /// Whole-bar quantization means actual duration may differ slightly.
    pub fn for_duration(duration: SongDuration, tempo_bpm: f64, beats_per_bar: u8) -> Self {
        let bpm = if tempo_bpm.is_finite() { tempo_bpm.clamp(20.0, 400.0) } else { 120.0 };
        let beats = f64::from(beats_per_bar.max(1));
        let bar_seconds = 60.0 * beats / bpm;
        let total_bars = (f64::from(duration.seconds()) / bar_seconds).round()
            .max(8.0).min(f64::from(u16::MAX)) as u16;

        const WEIGHTS: [u16; 8] = [1, 2, 1, 2, 2, 1, 2, 1];
        const NAMES: [&str; 8] = [
            "Intro", "Verse 1", "Build-up", "Chorus",
            "Verse 2", "Bridge", "Final Chorus", "Outro",
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
            sections.push(SongSection { name: NAMES[index], start_bar: cursor, end_bar: end });
            cursor = end;
        }

        Self { form: SongForm::Standard, sections, total_bars }
    }

    pub fn estimated_duration_seconds(&self, tempo_bpm: f64, beats_per_bar: u8) -> f64 {
        let bpm = if tempo_bpm.is_finite() { tempo_bpm.clamp(20.0, 400.0) } else { 120.0 };
        f64::from(self.total_bars) * f64::from(beats_per_bar.max(1)) * 60.0 / bpm
    }
}

#[cfg(test)]
mod tests {
    use super::*;


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
            assert_eq!(structure.sections.last().unwrap().end_bar, structure.total_bars);
            assert!(structure.sections.iter().all(|s| s.bars() > 0));
            for pair in structure.sections.windows(2) {
                assert_eq!(pair[0].end_bar, pair[1].start_bar);
            }
            assert!((structure.estimated_duration_seconds(120.0, 4) - f64::from(seconds)).abs() <= 1.0);
        }
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
