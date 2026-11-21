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

#[cfg(test)]
mod tests {
    use super::*;

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
