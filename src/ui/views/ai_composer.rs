//! The in-app prompt for creating a fresh multi-track arrangement.
#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufRead, BufReader, BufWriter, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Id, TextEdit};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Value, json};

use super::{AppCtx, Nav};

#[cfg(not(target_arch = "wasm32"))]
const MCP_SERVER: &str = include_str!("../../../tools/honeydew-mcp/server.py");
#[cfg(not(target_arch = "wasm32"))]
const MCP_COMPOSER: &str = include_str!("../../../tools/honeydew-mcp/composer.py");

#[cfg(not(target_arch = "wasm32"))]
struct ChildGuard(std::process::Child);

#[cfg(not(target_arch = "wasm32"))]
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub struct State {
    prompt: String,
    bars: u32,
    seed: u64,
    groq_api_key: String,
    worker: Option<Receiver<Result<String, String>>>,
    status: Option<String>,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            prompt: "Uplifting melodic house, warm bass, wide chords and a clear lead".into(),
            bars: 8,
            seed: 7,
            groq_api_key: String::new(),
            worker: None,
            status: None,
            error: None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_rpc(reader: &mut BufReader<std::process::ChildStdout>) -> Result<Value, String> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("MCP read failed: {e}"))?;
    if line.is_empty() {
        return Err("The local MCP producer exited without a response. Check that Python 3 is installed.".into());
    }
    serde_json::from_str(&line).map_err(|e| format!("Invalid MCP response: {e}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_groq_producer(prompt: String, bars: u32, seed: u64, api_key: String) -> Result<String, String> {
    if !api_key.trim().is_empty() && api_key.trim().len() < 12 {
        return Err("Groq API key looks incomplete.".into());
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_nanos();
    let dir = std::env::temp_dir().join(format!("honeydew-ai-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot prepare local AI adapter: {e}"))?;
    let server_path = dir.join("server.py");
    let composer_path = dir.join("composer.py");
    let result = (|| {
        std::fs::write(&server_path, MCP_SERVER)
            .map_err(|e| format!("Cannot write local MCP server: {e}"))?;
        std::fs::write(&composer_path, MCP_COMPOSER)
            .map_err(|e| format!("Cannot write local AI producer: {e}"))?;

        #[cfg(windows)]
        let mut child = ChildGuard({
            let mut command = Command::new("python");
            command.arg(&server_path);
            command
                .current_dir(&dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .env("PYTHONUTF8", "1");
            if !api_key.trim().is_empty() {
                command.env("GROQ_API_KEY", api_key.trim());
            }
            command.spawn().or_else(|_| {
                let mut fallback = Command::new("py");
                fallback.arg("-3").arg(&server_path)
                    .current_dir(&dir).stdin(Stdio::piped()).stdout(Stdio::piped())
                    .stderr(Stdio::null()).env("PYTHONUTF8", "1");
                if !api_key.trim().is_empty() { fallback.env("GROQ_API_KEY", api_key.trim()); }
                fallback.spawn()
            }).map_err(|e| format!("Python 3 is required to run the local AI producer: {e}"))?
        });
        #[cfg(not(windows))]
        let mut child = ChildGuard({
            let mut command = Command::new("python3");
            command.arg(&server_path).current_dir(&dir)
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
                .env("PYTHONUTF8", "1");
            if !api_key.trim().is_empty() {
                command.env("GROQ_API_KEY", api_key.trim());
            }
            command.spawn().map_err(|e| format!("Python 3 is required to run the local AI producer: {e}"))?
        });

        let mut input = BufWriter::new(child.0.stdin.take().ok_or("Cannot open MCP input")?);
        let output = child.0.stdout.take().ok_or("Cannot open MCP output")?;
        let mut reader = BufReader::new(output);

        writeln!(input, "{}", json!({
            "jsonrpc":"2.0", "id":1, "method":"initialize",
            "params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"Honeydew AI Composer","version":"1.0"}}
        })).map_err(|e| format!("MCP initialize failed: {e}"))?;
        input.flush().map_err(|e| e.to_string())?;
        let initialized = read_rpc(&mut reader)?;
        if let Some(error) = initialized.get("error") {
            return Err(format!("MCP initialize error: {error}"));
        }
        writeln!(input, "{}", json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .map_err(|e| e.to_string())?;
        writeln!(input, "{}", json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/call",
            "params":{"name":"compose_song","arguments":{"prompt":prompt,"bars":bars,"seed":seed}}
        })).map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        let response = read_rpc(&mut reader)?;
        if let Some(error) = response.get("error") {
            return Err(format!("MCP tool call failed: {error}"));
        }
        let result = response.get("result").ok_or("MCP response has no result")?;
        if result.get("isError").and_then(Value::as_bool).unwrap_or(false) {
            let message = result.get("content").and_then(Value::as_array)
                .and_then(|items| items.first()).and_then(|item| item.get("text"))
                .and_then(Value::as_str).unwrap_or("Groq producer failed.");
            return Err(message.to_owned());
        }
        let text = result.get("content").and_then(Value::as_array)
            .and_then(|items| items.first()).and_then(|item| item.get("text"))
            .and_then(Value::as_str).unwrap_or("AI producer completed.");
        Ok(text.to_owned())
    })();
    // The MCP process is per-generation. It is stopped and its temporary source files are removed.
    let _ = std::fs::remove_dir_all(&dir);
    result
}

pub fn show(state: &mut State, ui: &mut egui::Ui, app: &mut AppCtx) -> Option<Nav> {
    let mut open = true;
    let mut nav = None;

    if let Some(worker) = &state.worker {
        match worker.try_recv() {
            Ok(Ok(summary)) => {
                state.status = Some(summary);
                state.worker = None;
                state.error = None;
            }
            Ok(Err(error)) => {
                state.error = Some(error);
                state.worker = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.error = Some("AI producer stopped unexpectedly.".into());
                state.worker = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    egui::Window::new("AI Composer")
        .id(Id::new("ai_composer"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(440.0)
        .show(ui.ctx(), |ui| {
            ui.label("Groq AI producer inspects the open project and edits it through Honeydew's local MCP tools.");
            ui.add(
                TextEdit::multiline(&mut state.prompt)
                    .desired_rows(4)
                    .hint_text("Genre, mood, groove, instruments, arrangement…"),
            );
            ui.add(egui::Slider::new(&mut state.bars, 1..=16).text("Bars"));
            ui.add(egui::DragValue::new(&mut state.seed).prefix("Seed "));
            ui.add_space(4.0);
            ui.label("Groq API key (kept in memory only):");
            ui.add(TextEdit::singleline(&mut state.groq_api_key).password(true)
                .hint_text("Paste GROQ_API_KEY"));
            if !app.settings.mcp_enabled {
                ui.colored_label(ui.visuals().warn_fg_color,
                    "Enable MCP Control in Settings first. It provides the authenticated local bridge.");
            }
            if let Some(status) = &state.status {
                ui.label(status);
            }
            if let Some(error) = &state.error {
                ui.colored_label(ui.visuals().error_fg_color, error.as_str());
            }
            ui.horizontal(|ui| {
                let busy = state.worker.is_some();
                if ui.add_enabled(!busy && app.settings.mcp_enabled,
                    egui::Button::new(if busy { "Producing…" } else { "Generate with Groq" })).clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    let key = state.groq_api_key.trim().to_owned();
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if key.is_empty() && std::env::var("GROQ_API_KEY").is_err() {
                            state.error = Some("Paste your Groq API key above or set GROQ_API_KEY before starting Honeydew.".into());
                        } else {
                            let prompt = state.prompt.clone();
                            let bars = state.bars;
                            let seed = state.seed;
                            let (tx, rx) = mpsc::channel();
                            state.worker = Some(rx);
                            state.status = Some("Groq is inspecting the project and composing…".into());
                            state.error = None;
                            let ctx = ui.ctx().clone();
                            thread::spawn(move || {
                                let result = run_groq_producer(prompt, bars, seed, key);
                                let _ = tx.send(result);
                                ctx.request_repaint();
                            });
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        state.error = Some("Groq AI Composer is available in the desktop build only.".into());
                    }
                }
                if ui.button("Cancel").clicked() {
                    nav = Some(Nav::Close);
                }
            });
            ui.small("The generated tracks and sound-design changes are applied directly to the open song. Existing clips are preserved. Undo remains available.");
            ui.small("The model cannot audition audio; listen to the result and iterate with a revised prompt.");
        });
    if state.worker.is_some() {
        ui.ctx().request_repaint_after(Duration::from_millis(150));
    }
    if !open {
        nav = Some(Nav::Close);
    }
    nav
}
