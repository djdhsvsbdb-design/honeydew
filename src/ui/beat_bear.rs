//! Bear AI: a small local intent planner for turning a music prompt into a generator brief.
//!
//! This is deliberately an offline first version: it does not pretend to be a hosted language model.
//! It extracts a compact, typed brief which the beat generator can consume. A model-backed planner can
//! later implement the same interface without changing the chat or generator contract.

/// A generation task understood by the music generator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeatBrief {
    pub style: Style,
    pub bpm: Option<u16>,
    pub mood: Mood,
    pub bars: u8,
    pub include_drums: bool,
    pub include_bass: bool,
    pub include_chords: bool,
    pub include_melody: bool,
}

/// Musical style selected from a user's message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Trap,
    HipHop,
    House,
    Techno,
    DrumAndBass,
    LoFi,
    Pop,
    Default,
}

/// Broad emotional direction for the arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Dark,
    Bright,
    Chill,
    Energetic,
    Neutral,
}

impl Default for BeatBrief {
    fn default() -> Self {
        Self {
            style: Style::Default,
            bpm: None,
            mood: Mood::Neutral,
            bars: 4,
            include_drums: true,
            include_bass: true,
            include_chords: true,
            include_melody: true,
        }
    }
}

/// The assistant's response and the task it proposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub message: String,
    pub task: BeatBrief,
}

/// Interpret a natural-language music request using local, deterministic rules.
/// The parser intentionally keeps defaults when a request is ambiguous.
pub fn think(prompt: &str) -> Reply {
    let text = prompt.to_lowercase();
    let mut task = BeatBrief::default();

    task.style = if has_any(&text, &["drum and bass", "dnb", "драм-н-бейс", "драм энд бейс"]) {
        Style::DrumAndBass
    } else if has_any(&text, &["trap", "трэп"]) {
        Style::Trap
    } else if has_any(&text, &["hip hop", "hip-hop", "hiphop", "хип-хоп", "рэп"]) {
        Style::HipHop
    } else if has_any(&text, &["techno", "техно"]) {
        Style::Techno
    } else if has_any(&text, &["house", "хаус"]) {
        Style::House
    } else if has_any(&text, &["lofi", "lo-fi", "лоу-фай", "лофай"]) {
        Style::LoFi
    } else if has_any(&text, &["pop", "поп"]) {
        Style::Pop
    } else {
        Style::Default
    };

    task.mood = if has_any(&text, &["dark", "мрач", "тём", "темн", "злой", "агрессив"]) {
        Mood::Dark
    } else if has_any(&text, &["bright", "светл", "радост", "весёл", "весел"]) {
        Mood::Bright
    } else if has_any(&text, &["chill", "спокой", "расслаб", "мягк", "лоу-фай"]) {
        Mood::Chill
    } else if has_any(&text, &["energetic", "энергич", "быстр", "мощн", "жёстк", "жестк"]) {
        Mood::Energetic
    } else {
        Mood::Neutral
    };

    task.bpm = find_bpm(&text);
    task.bars = if has_any(&text, &["8 bars", "8-bar", "8 такт", "восемь такт"]) {
        8
    } else if has_any(&text, &["2 bars", "2-bar", "2 такт", "два такт"]) {
        2
    } else {
        4
    };

    task.include_drums = !has_any(&text, &["без ударных", "no drums", "без барабан"]);
    task.include_bass = !has_any(&text, &["без баса", "no bass"]);
    task.include_chords = !has_any(&text, &["без аккордов", "no chords"]);
    task.include_melody = !has_any(&text, &["без мелодии", "no melody"]);

    let style = match task.style {
        Style::Trap => "trap",
        Style::HipHop => "hip-hop",
        Style::House => "house",
        Style::Techno => "techno",
        Style::DrumAndBass => "drum & bass",
        Style::LoFi => "lo-fi",
        Style::Pop => "pop",
        Style::Default => "универсальный",
    };
    let bpm = task.bpm.map_or_else(|| "темп проекта".to_owned(), |n| format!("{n} BPM"));
    let mood = match task.mood {
        Mood::Dark => "мрачное настроение",
        Mood::Bright => "светлое настроение",
        Mood::Chill => "спокойное настроение",
        Mood::Energetic => "энергичное настроение",
        Mood::Neutral => "нейтральное настроение",
    };
    let message = format!(
        "Я разобрал запрос: {style}, {bpm}, {mood}, {} такта. Подготовил задание для генератора: ударные {}, бас {}, аккорды {}, мелодия {}.",
        task.bars,
        yes_no(task.include_drums),
        yes_no(task.include_bass),
        yes_no(task.include_chords),
        yes_no(task.include_melody),
    );

    Reply { message, task }
}

fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}

fn yes_no(value: bool) -> &'static str {
    if value { "включены" } else { "выключены" }
}

fn find_bpm(text: &str) -> Option<u16> {
    for (index, _) in text.match_indices("bpm") {
        let prefix = text[..index].trim_end();
        let digits: String = prefix.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            continue;
        }
        let number: String = digits.chars().rev().collect();
        if let Ok(value) = number.parse::<u16>() {
            if (40..=240).contains(&value) {
                return Some(value);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn understands_a_russian_trap_request() {
        let reply = think("Сделай мрачный trap на 140 BPM, 8 тактов");
        assert_eq!(reply.task.style, Style::Trap);
        assert_eq!(reply.task.mood, Mood::Dark);
        assert_eq!(reply.task.bpm, Some(140));
        assert_eq!(reply.task.bars, 8);
        assert!(reply.task.include_drums && reply.task.include_bass);
    }

    #[test]
    fn can_exclude_requested_parts() {
        let task = think("lofi chill 82 BPM без ударных и без мелодии").task;
        assert_eq!(task.style, Style::LoFi);
        assert_eq!(task.mood, Mood::Chill);
        assert_eq!(task.bpm, Some(82));
        assert!(!task.include_drums);
        assert!(!task.include_melody);
    }

    #[test]
    fn ignores_out_of_range_tempos() {
        assert_eq!(think("make house at 999 BPM").task.bpm, None);
    }
}
