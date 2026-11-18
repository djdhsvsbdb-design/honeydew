//! The in-app prompt for creating a fresh multi-track arrangement.
use eframe::egui::{self, Id, TextEdit};

use super::{AppCtx, Nav};

pub struct State {
    prompt: String,
    bars: u32,
    seed: u64,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            prompt: "Uplifting melodic house, warm bass, wide chords and a clear lead".into(),
            bars: 8,
            seed: 7,
            error: None,
        }
    }
}

pub fn show(state: &mut State, ui: &mut egui::Ui, app: &mut AppCtx) -> Option<Nav> {
    let mut open = true;
    let mut nav = None;
    egui::Window::new("AI Composer")
        .id(Id::new("ai_composer"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .show(ui.ctx(), |ui| {
            ui.label("Describe the music. Honeydew will add four new tracks and keep existing clips.");
            ui.add(
                TextEdit::multiline(&mut state.prompt)
                    .desired_rows(4)
                    .hint_text("Genre, mood, tempo and sound…"),
            );
            ui.add(egui::Slider::new(&mut state.bars, 1..=16).text("Bars"));
            ui.add(egui::DragValue::new(&mut state.seed).prefix("Seed "));
            if let Some(error) = &state.error {
                ui.colored_label(ui.visuals().error_fg_color, error.as_str());
            }
            ui.horizontal(|ui| {
                if ui.button("Generate arrangement").clicked() {
                    match crate::song::ai_compose::compose(
                        app.song,
                        &state.prompt,
                        state.bars,
                        state.seed,
                    ) {
                        Ok(song) => {
                            *app.song = song;
                            app.song_edited(true);
                            state.error = None;
                            nav = Some(Nav::Close);
                        }
                        Err(error) => state.error = Some(error),
                    }
                }
                if ui.button("Cancel").clicked() {
                    nav = Some(Nav::Close);
                }
            });
            ui.small("Offline procedural generation. The result is editable and can be undone as one song edit.");
        });
    if !open {
        nav = Some(Nav::Close);
    }
    nav
}
