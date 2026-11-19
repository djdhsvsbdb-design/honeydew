//! Honeydew AI chat integration with FCC model discovery and a validated visual-agent protocol.
//! Screenshot capture/action execution are not implemented yet.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// One proposed visual UI action. Coordinates are normalized to the screenshot dimensions.
/// This protocol is deliberately single-step: capture/execute/verify must happen between steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum UiAction {
    Click { x: f32, y: f32, button: String },
    Move { x: f32, y: f32 },
    Key { key: String },
    TypeText { text: String },
    Wait { milliseconds: u64 },
    Done { summary: String },
}

fn parse_ui_action(text: &str) -> Result<UiAction, String> {
    let trimmed = text.trim();
    let candidate = if trimmed.starts_with("```") {
        trimmed.trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim()
    } else {
        trimmed
    };
    let action: UiAction = serde_json::from_str(candidate).map_err(|e| format!("Expected one UI-action JSON object: {e}"))?;
    match &action {
        UiAction::Click { x, y, button } => {
            if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(x) || !(0.0..=1.0).contains(y) {
                return Err("Coordinates must be normalized between 0 and 1".into());
            }
            if !matches!(button.as_str(), "left" | "right" | "middle") {
                return Err("Unsupported mouse button".into());
            }
        }
        UiAction::Move { x, y } => {
            if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(x) || !(0.0..=1.0).contains(y) {
                return Err("Coordinates must be normalized between 0 and 1".into());
            }
        }
        UiAction::Key { key } => {
            if key.is_empty() || key.len() > 32 { return Err("Key name must be 1–32 characters".into()); }
        }
        UiAction::TypeText { text } => {
            if text.len() > 2000 { return Err("Text action exceeds 2000 bytes".into()); }
        }
        UiAction::Wait { milliseconds } => {
            if *milliseconds > 5000 { return Err("Wait action is limited to 5000 ms".into()); }
        }
        UiAction::Done { summary } => {
            if summary.len() > 500 { return Err("Summary exceeds 500 characters".into()); }
        }
    }
    Ok(action)
}

#[derive(Clone)]
struct Message {
    role: &'static str,
    text: String,
}

pub(crate) struct AgentPanel {
    pub(crate) open: bool,
    endpoint: String,
    model: String,
    api_key: String,
    input: String,
    messages: Vec<Message>,
    pending: Option<Receiver<Result<String, String>>>,
    models: Vec<String>,
    pending_models: Option<Receiver<Result<Vec<String>, String>>>,
    status: String,
}

impl Default for AgentPanel {
    fn default() -> Self {
        Self {
            open: false,
            endpoint: "http://127.0.0.1:8082/v1/chat/completions".into(),
            model: String::new(),
            api_key: String::new(),
            input: String::new(),
            messages: Vec::new(),
            pending: None,
            models: Vec::new(),
            pending_models: None,
            status: "Configure FCC endpoint and load models".into(),
        }
    }
}

impl AgentPanel {
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        project: &str,
        bpm: f64,
        view: &str,
        selected_channel: usize,
    ) {
        if let Some(rx) = self.pending_models.as_ref() {
            if let Ok(result) = rx.try_recv() {
                self.pending_models = None;
                match result {
                    Ok(models) => {
                        let count = models.len();
                        if self.model.is_empty() {
                            if let Some(first) = models.first() { self.model = first.clone(); }
                        }
                        self.models = models;
                        self.status = format!("Loaded {count} models from FCC");
                    }
                    Err(err) => self.status = err,
                }
            }
        }

        if let Some(rx) = self.pending.as_ref() {
            if let Ok(result) = rx.try_recv() {
                self.pending = None;
                match result {
                    Ok(text) => {
                        self.messages.push(Message { role: "assistant", text });
                        self.status = "Response received".into();
                    }
                    Err(err) => self.status = err,
                }
            }
        }

        egui::SidePanel::right("honeydew_ai_chat")
            .resizable(true)
            .default_width(340.0)
            .min_width(280.0)
            .max_width(520.0)
            .show_animated(ctx, self.open, |ui| {
                ui.heading("AI Assistant");
                ui.label("Local model · visual-agent protocol scaffold (actions not executed yet)");
                ui.separator();

                egui::CollapsingHeader::new("Connection")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label("FCC endpoint (local HTTP only; no TLS)");
                        ui.text_edit_singleline(&mut self.endpoint);
                        ui.label("Model ID");
                        ui.text_edit_singleline(&mut self.model);
                        ui.label("API token (optional)");
                        ui.add(egui::TextEdit::singleline(&mut self.api_key).password(true));
                        if ui.add_enabled(self.pending_models.is_none(), egui::Button::new(if self.pending_models.is_some() { "Loading…" } else { "Load models from FCC" })).clicked() {
                            self.load_models(ctx);
                        }
                    });

                if !self.models.is_empty() {
                    egui::ComboBox::from_label("Available FCC models")
                        .selected_text(if self.model.is_empty() { "Select a model" } else { self.model.as_str() })
                        .show_ui(ui, |ui| {
                            for model in &self.models {
                                ui.selectable_value(&mut self.model, model.clone(), model);
                            }
                        });
                }

                ui.horizontal(|ui| {
                    if ui.button("Clear chat").clicked() {
                        self.messages.clear();
                    }
                    ui.label(if self.pending.is_some() { "Working…" } else { self.status.as_str() });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for message in &self.messages {
                            ui.group(|ui| {
                                ui.strong(if message.role == "user" { "You" } else { "AI" });
                                ui.label(&message.text);
                            });
                        }
                    });

                ui.separator();
                ui.label("Read-only context. Visual actions are protocol-only until screenshot capture and input executor are added.");
                ui.label(format!("Project: {project} · Tempo: {bpm:.1} BPM"));
                ui.label(format!("View: {view} · Channel: {}", selected_channel + 1));
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .hint_text("Ask about a beat, sound, or arrangement…")
                        .desired_rows(3),
                );
                let can_send = self.pending.is_none()
                    && !self.input.trim().is_empty()
                    && !self.model.trim().is_empty();
                if ui.add_enabled(can_send, egui::Button::new("Send")).clicked() {
                    self.send(ctx, project, bpm, view, selected_channel);
                }
            });
    }

    fn load_models(&mut self, ctx: &egui::Context) {
        let endpoint = self.endpoint.clone();
        let key = self.api_key.clone();
        let (tx, rx) = mpsc::channel();
        self.pending_models = Some(rx);
        self.status = "Loading FCC model catalog…".into();
        ctx.request_repaint();

        let worker_ctx = ctx.clone();
        thread::spawn(move || {
            let result = http_models(&endpoint, &key);
            let _ = tx.send(result);
            worker_ctx.request_repaint();
        });
    }

    fn send(&mut self, ctx: &egui::Context, project: &str, bpm: f64, view: &str, channel: usize) {
        let user_text = std::mem::take(&mut self.input);
        self.messages.push(Message { role: "user", text: user_text.clone() });
        let messages: Vec<Value> = self.messages.iter()
            .map(|m| json!({"role": m.role, "content": m.text}))
            .collect();
        let system = format!(
            "You are the visual UI agent inside Honeydew, a digital audio workstation. Help with music tasks.
Current read-only context: project={project:?}, tempo={bpm:.1} BPM, view={view:?}, selected channel index={channel}.
Visual-action protocol (not currently executable by the app): when asked to act, propose ONLY one next action as a JSON object:
{{"action":"click","x":0.0,"y":0.0,"button":"left"}} or {{"action":"move","x":0.0,"y":0.0}} or {{"action":"key","key":"Enter"}} or {{"action":"type_text","text":"..."}} or {{"action":"wait","milliseconds":300}} or {{"action":"done","summary":"..."}}.
Coordinates are normalized 0..1 relative to the Honeydew window screenshot. Never propose more than one action at a time. After every action, a screenshot must be captured and inspected before proposing the next action. Never claim an action was executed or verified; screenshot capture and input execution are not connected yet. Avoid destructive actions unless the user explicitly confirms."
        );
        let endpoint = self.endpoint.clone();
        let model = self.model.clone();
        let key = self.api_key.clone();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.status = "Sending request…".into();
        ctx.request_repaint();

        let worker_ctx = ctx.clone();
        thread::spawn(move || {
            let result = http_chat(&endpoint, &model, &key, &system, &messages);
            let _ = tx.send(result);
            worker_ctx.request_repaint();
        });
    }
}

fn http_chat(endpoint: &str, model: &str, api_key: &str, system: &str, messages: &[Value]) -> Result<String, String> {
    let (host, port, path) = parse_http_url(endpoint)?;
    let mut all_messages = vec![json!({"role": "system", "content": system})];
    all_messages.extend_from_slice(messages);
    let body = json!({"model": model, "messages": all_messages, "stream": false}).to_string();
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("Cannot connect to FCC: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(90))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(30))).map_err(|e| e.to_string())?;

    let auth = if api_key.is_empty() { String::new() } else { format!("Authorization: Bearer {api_key}\r\n") };
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(|e| format!("Send failed: {e}"))?;
    let mut response = Vec::new();
    stream.take(4 * 1024 * 1024 + 1).read_to_end(&mut response).map_err(|e| format!("Read failed: {e}"))?;
    if response.len() > 4 * 1024 * 1024 {
        return Err("Response exceeds 4 MiB limit".into());
    }
    let split = response.windows(4).position(|w| w == b"\r\n\r\n")
        .ok_or("Invalid HTTP response")?;
    let headers = String::from_utf8_lossy(&response[..split]).to_ascii_lowercase();
    let mut payload = response[split + 4..].to_vec();
    if headers.starts_with("http/") && !headers.contains(" 200 ") {
        let status = headers.lines().next().unwrap_or("HTTP error");
        return Err(format!("FCC returned {status}"));
    }
    if headers.contains("transfer-encoding: chunked") {
        payload = decode_chunked(&payload).ok_or("Malformed chunked HTTP response")?;
    }
    let data: Value = serde_json::from_slice(&payload).map_err(|e| format!("Invalid JSON response: {e}"))?;
    if let Some(error) = data.get("error") {
        return Err(format!("FCC error: {error}"));
    }
    data.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Unexpected response shape (expected choices[0].message.content)".into())
}

/// Read the OpenAI-compatible model catalog exposed by FCC at GET /v1/models.
fn http_models(endpoint: &str, api_key: &str) -> Result<Vec<String>, String> {
    let (host, port, _) = parse_http_url(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("Cannot connect to FCC: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(20))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
    let auth = if api_key.is_empty() { String::new() } else { format!("Authorization: Bearer {api_key}\\r\\n") };
    let request = format!(
        "GET /v1/models HTTP/1.1\\r\\nHost: {host}:{port}\\r\\nAccept: application/json\\r\\n{auth}Connection: close\\r\\n\\r\\n"
    );
    stream.write_all(request.as_bytes()).map_err(|e| format!("Send failed: {e}"))?;
    let mut response = Vec::new();
    stream.take(2 * 1024 * 1024 + 1).read_to_end(&mut response).map_err(|e| format!("Read failed: {e}"))?;
    if response.len() > 2 * 1024 * 1024 {
        return Err("FCC model catalog exceeds 2 MiB".into());
    }
    let split = response.windows(4).position(|w| w == b"\\r\\n\\r\\n")
        .ok_or("Invalid HTTP response")?;
    let headers = String::from_utf8_lossy(&response[..split]).to_ascii_lowercase();
    let status = headers.lines().next().unwrap_or("HTTP error");
    if !status.contains(" 200 ") {
        return Err(format!("FCC model catalog returned {status}"));
    }
    let mut payload = response[split + 4..].to_vec();
    if headers.contains("transfer-encoding: chunked") {
        payload = decode_chunked(&payload).ok_or("Malformed chunked HTTP response")?;
    }
    let data: Value = serde_json::from_slice(&payload).map_err(|e| format!("Invalid FCC model catalog JSON: {e}"))?;
    let rows = data.get("data").and_then(Value::as_array)
        .ok_or("Unexpected FCC model catalog (expected data array)")?;
    let mut models: Vec<String> = rows.iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(str::trim))
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();
    models.sort();
    models.dedup();
    if models.is_empty() {
        return Err("FCC returned an empty model catalog".into());
    }
    Ok(models)
}

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url.strip_prefix("http://").ok_or("Only plain HTTP endpoints are supported; use the local FCC proxy")?;
    let cut = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..cut];
    let suffix = &rest[cut..];
    let path = if suffix.is_empty() { "/".to_owned() } else if suffix.starts_with('?') { format!("/{suffix}") } else { suffix.to_owned() };
    let (host, port) = match authority.split_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| "Invalid endpoint port")?),
        None => (authority, 80),
    };
    if !matches!(host, "127.0.0.1" | "localhost") || port == 0 || path.contains(' ') {
        return Err("For security, FCC must use localhost or 127.0.0.1 over HTTP".into());
    }
    Ok((host.to_owned(), port, path))
}

fn decode_chunked(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut cursor = 0;
    let mut output = Vec::new();
    loop {
        let end = bytes.get(cursor..)?.windows(2).position(|w| w == b"\r\n")? + cursor;
        let size_text = std::str::from_utf8(&bytes[cursor..end]).ok()?.split(';').next()?;
        let size = usize::from_str_radix(size_text.trim(), 16).ok()?;
        cursor = end + 2;
        if size == 0 {
            return Some(output);
        }
        let chunk = bytes.get(cursor..cursor + size)?;
        output.extend_from_slice(chunk);
        cursor += size;
        if bytes.get(cursor..cursor + 2)? != b"\r\n" {
            return None;
        }
        cursor += 2;
    }
}


#[cfg(test)]
mod tests {
    use super::{decode_chunked, parse_http_url, parse_ui_action, UiAction};

    #[test]
    fn parses_one_normalized_click_action() {
        assert_eq!(
            parse_ui_action(r#"{"action":"click","x":0.4,"y":0.7,"button":"left"}"#).unwrap(),
            UiAction::Click { x: 0.4, y: 0.7, button: "left".into() }
        );
    }

    #[test]
    fn rejects_out_of_bounds_click() {
        assert!(parse_ui_action(r#"{"action":"click","x":1.2,"y":0.7,"button":"left"}"#).is_err());
    }

    #[test]
    fn rejects_wait_over_limit() {
        assert!(parse_ui_action(r#"{"action":"wait","milliseconds":6000}"#).is_err());
    }

    #[test]
    fn parses_done_action() {
        assert!(matches!(
            parse_ui_action(r#"{"action":"done","summary":"Finished"}"#),
            Ok(UiAction::Done { .. })
        ));
    }

    #[test]
    fn parses_local_fcc_endpoint() {
        assert_eq!(
            parse_http_url("http://127.0.0.1:8082/v1/chat/completions").unwrap(),
            ("127.0.0.1".to_owned(), 8082, "/v1/chat/completions".to_owned())
        );
    }

    #[test]
    fn rejects_remote_http_hosts() {
        assert!(parse_http_url("http://example.com/v1/chat/completions").is_err());
        assert!(parse_http_url("https://127.0.0.1/v1/chat/completions").is_err());
    }

    #[test]
    fn handles_query_only_endpoint_suffix() {
        assert_eq!(
            parse_http_url("http://localhost:8082?view=messages").unwrap().2,
            "/?view=messages"
        );
    }

    #[test]
    fn decodes_chunked_response() {
        assert_eq!(
            decode_chunked(b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n").unwrap(),
            b"Wikipedia"
        );
    }
}
