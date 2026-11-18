//! A small offline multi-track composer used by the AI Composer panel.
//! It turns a text brief into a fresh, deterministic arrangement without overwriting existing clips.
use super::{ChannelKind, Event, PPQ, Song, SynthKind};
use crate::params::{self, ParamKind, Role};

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, min: u8, max: u8) -> u8 {
        min + (self.next() % u64::from(max - min + 1)) as u8
    }
}

fn add_track(song: &mut Song, kind: ChannelKind, name: &str, bars: u32) -> Result<usize, String> {
    let index = song
        .insert_channel(song.channels.len(), kind)
        .ok_or("channel limit reached")?;
    let channel = &mut song.channels[index];
    channel.name = name.to_owned();
    let beats = f64::from(song.time_sig.bar_beats()) * f64::from(bars);
    channel.lanes[0].add_clip(0, beats);
    Ok(index)
}

fn add_notes(song: &mut Song, channel: usize, notes: Vec<Event>) {
    if let Some(clip) = song.channels[channel].lanes[0].clips.last_mut() {
        clip.events = notes;
        clip.events.sort_by_key(Event::pos);
    }
}

fn tune_instrument(channel: &mut super::Channel, role: &str) {
    let Some(module) = channel.rack.modules.first_mut() else {
        return;
    };
    let targets: &[(&str, f32)] = match role {
        "bass" => &[("cutoff", 0.46), ("drive", 0.34), ("attack", 0.08), ("release", 0.24)],
        "chords" => &[("attack", 0.28), ("release", 0.62), ("cutoff", 0.72)],
        "lead" => &[("attack", 0.12), ("release", 0.48), ("cutoff", 0.68)],
        _ => &[],
    };
    let changes: Vec<(i32, f32)> = (0..module.params.len())
        .filter_map(|index| {
            let spec = module.param_spec(index)?;
            if !matches!(spec.kind, ParamKind::Continuous { .. }) {
                return None;
            }
            let name = spec.name.to_lowercase();
            targets
                .iter()
                .find(|(target, _)| name.contains(*target))
                .map(|(_, value)| (index as i32, *value))
        })
        .collect();
    for (index, value) in changes {
        if let Some(param) = module.param_mut(index) {
            *param = value;
        }
        module.init_follows(index);
    }
}

fn add_effect(channel: &mut super::Channel, contains: &str, mix: f32) {
    let found = params::modules()
        .chain(params::LEMON_MODULES.iter())
        .chain([
            &params::DYNAMIC_EQ,
            &params::MODULATOR,
            &params::DE_ESSER,
            &params::DENOISE,
            &params::HALF_TIME,
        ])
        .find(|spec| {
            spec.role == Role::Effect
                && spec.display_name.to_lowercase().contains(contains)
        });
    if let Some(spec) = found {
        let id = channel.rack.add(spec.id);
        if let Some(module) = channel.rack.get_mut(id) {
            module.common.0[1] = mix.clamp(0.0, 1.0);
        }
    }
}

fn note(beat: f64, length: f64, key: u8, velocity: u8) -> Event {
    Event::Note {
        pos: (beat * f64::from(PPQ)).round() as u32,
        len_beats: length,
        key,
        velocity,
        pan: Event::DEFAULT_PAN,
        slide: false,
    }
}

fn drum(beat: f64, pad: u8, velocity: u8) -> Event {
    Event::Drum {
        pos: (beat * f64::from(PPQ)).round() as u32,
        len_beats: 0.15,
        pad,
        velocity,
        pan: Event::DEFAULT_PAN,
        value: Event::DEFAULT_PITCH,
        repeat: 0,
    }
}

/// Creates four new tracks and a role-aware effect chain. Existing clips are never replaced.
pub fn compose(song: &Song, prompt: &str, bars: u32, seed: u64) -> Result<Song, String> {
    if !(1..=16).contains(&bars) {
        return Err("bars must be 1–16".into());
    }
    if prompt.trim().is_empty() || prompt.chars().count() > 2000 {
        return Err("describe the music in 1–2000 characters".into());
    }
    let text = prompt.to_lowercase();
    let ambient = text.contains("ambient") || text.contains("cinematic");
    let lofi = text.contains("lofi") || text.contains("lo-fi") || text.contains("chill");
    let trance = text.contains("trance") || text.contains("uplifting");
    let techno = text.contains("techno");
    let dnb = text.contains("dnb") || text.contains("drum and bass");
    let major = text.contains("happy") || text.contains("bright") || text.contains("major");
    let bpm = if let Some(at) = text.find("bpm") {
        let digits: String = text[..at].chars().rev().take_while(|c| c.is_ascii_digit() || *c == ' ')
            .collect::<String>().chars().rev().collect();
        digits.trim().parse::<f64>().ok()
    } else {
        None
    };
    let tempo = bpm.unwrap_or(if ambient { 92.0 } else if lofi { 84.0 } else if dnb { 174.0 }
        else if trance { 138.0 } else if techno { 130.0 } else { 124.0 });
    let mut result = song.clone();
    result.set_tempo(tempo.clamp(60.0, 190.0));

    let drums = add_track(&mut result, ChannelKind::Drums, "AI Drums", bars)?;
    let bass = add_track(&mut result, ChannelKind::Synth(SynthKind::TransistorBass), "AI Bass", bars)?;
    let chords = add_track(&mut result, ChannelKind::Synth(SynthKind::SuperSaw), "AI Chords", bars)?;
    let lead = add_track(&mut result, ChannelKind::Synth(SynthKind::MiniSynth), "AI Lead", bars)?;

    let bpb = f64::from(result.time_sig.bar_beats());
    let mut rng = Rng::new(seed);
    let scale: [u8; 7] = if major { [0, 2, 4, 5, 7, 9, 11] } else { [0, 2, 3, 5, 7, 8, 10] };
    let progression: [usize; 4] = if major { [0, 4, 5, 3] } else { [0, 5, 3, 6] };
    let mut drum_notes = Vec::new();
    let mut bass_notes = Vec::new();
    let mut chord_notes = Vec::new();
    let mut lead_notes = Vec::new();

    for bar in 0..bars {
        let base = f64::from(bar) * bpb;
        let degree = progression[bar as usize % progression.len()];
        let root = 48 + scale[degree];
        if !ambient {
            for beat in 0..4 {
                let at = base + f64::from(beat) * bpb / 4.0;
                drum_notes.push(drum(at, 0, if beat % 2 == 0 { 195 } else { 150 }));
            }
            for beat in [1.0, 3.0] {
                drum_notes.push(drum(base + beat * bpb / 4.0, 1, 170));
            }
            let hat_step = if lofi { 1.0 } else { 0.5 };
            let mut at = 0.0;
            while at < bpb {
                drum_notes.push(drum(base + at, 3, rng.range(65, 105)));
                at += hat_step;
            }
            if trance || dnb || techno {
                drum_notes.push(drum(base + bpb - 0.5, 4, 95));
            }
        }

        let intervals: &[u8] = if major { &[0, 4, 7] } else { &[0, 3, 7] };
        for interval in intervals {
            chord_notes.push(note(base, bpb * 0.92, (root + *interval) as u8, 125));
        }
        if trance || ambient {
            chord_notes.push(note(base, bpb * 0.92, (root + 12) as u8, 95));
        }

        let bass_steps = if ambient { 1 } else if lofi { 4 } else { 8 };
        for step in 0..bass_steps {
            let beat = base + f64::from(step) * bpb / f64::from(bass_steps);
            let key = root.saturating_sub(12) + if step % 4 == 0 { 7 } else { 0 };
            bass_notes.push(note(beat, (bpb / f64::from(bass_steps) * 0.88).max(0.2),
                                 key, rng.range(105, 150)));
        }

        if !ambient {
            let melody_steps = if lofi { 4 } else { 8 };
            for step in 0..melody_steps {
                let degree = [1usize, 2, 4, 5, 6][(rng.next() as usize) % 5];
                let octave = if step % 4 == 0 { 12 } else { 0 };
                let key = 60 + scale[degree] + octave;
                let beat = base + f64::from(step) * bpb / f64::from(melody_steps);
                lead_notes.push(note(beat, (bpb / f64::from(melody_steps) * 0.72).max(0.2),
                                     key.min(127), rng.range(80, 125)));
            }
        }
    }

    add_notes(&mut result, drums, drum_notes);
    add_notes(&mut result, bass, bass_notes);
    add_notes(&mut result, chords, chord_notes);
    add_notes(&mut result, lead, lead_notes);
    tune_instrument(&mut result.channels[bass], "bass");
    tune_instrument(&mut result.channels[chords], "chords");
    tune_instrument(&mut result.channels[lead], "lead");

    add_effect(&mut result.channels[drums], "compress", 0.65);
    add_effect(&mut result.channels[bass], "compress", 0.45);
    add_effect(&mut result.channels[chords], "reverb", 0.3);
    add_effect(&mut result.channels[chords], "chorus", 0.25);
    add_effect(&mut result.channels[lead], "reverb", 0.28);
    add_effect(&mut result.channels[lead], "delay", 0.2);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_creates_four_new_tracks_without_replacing_existing_content() {
        let song = Song::default();
        let result = compose(&song, "uplifting trance", 8, 42).unwrap();
        assert_eq!(result.channels.len(), song.channels.len() + 4);
        assert_eq!(song.channels.len(), 1);
        assert_eq!(result.tempo_bpm, 138.0);
        for channel in result.channels.iter().skip(1) {
            assert_eq!(channel.lanes[0].clips.len(), 1);
        }
        assert!(crate::flm::write(&result).is_ok());
    }

    #[test]
    fn compose_rejects_invalid_briefs_without_mutating_song() {
        let song = Song::default();
        assert!(compose(&song, "", 4, 1).is_err());
        assert!(compose(&song, "house", 17, 1).is_err());
        assert_eq!(song.channels.len(), 1);
    }
}
