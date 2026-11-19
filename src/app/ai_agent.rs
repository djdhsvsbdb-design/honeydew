//! A small opt-in local AI chat panel. This first integration connects to FCC's local
//! OpenAI-compatible HTTP endpoint; it does not yet drive DAW UI gestures or capture screenshots.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use eframe::egui;
use serde_json::{Value, json};

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
            status: "Configure FCC endpoint and model".into(),
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
                ui.label("FCC local chat · not controlling DAW yet");
                ui.separator();

                egui::CollapsingHeader::new("Connection")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label("FCC endpoint (HTTP only)");
                        ui.text_edit_singleline(&mut self.endpoint);
                        ui.label("Model ID");
                        ui.text_edit_singleline(&mut self.model);
                        ui.label("API token (optional)");
                        ui.add(egui::TextEdit::singleline(&mut self.api_key).password(true));
                    });

                ui.horizontal(|ui| {
                    if ui.button("Clear chat").clicked() {
                        self.messages.clear();
                    }
                    ui.label(if self.pending.is_some() { "Working…" } else { &self.status });
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
                ui.label("Project context is read-only in this initial chat.");
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

    fn send(&mut self, ctx: &egui::Context, project: &str, bpm: f64, view: &str, channel: usize) {
        let user_text = std::mem::take(&mut self.input);
        self.messages.push(Message { role: "user", text: user_text.clone() });
        let messages: Vec<Value> = self.messages.iter()
            .map(|m| json!({"role": m.role, "content": m.text}))
            .collect();
        let system = format!(
            "You are an assistant inside Honeydew, a digital audio workstation.             Help the user with music production and explain suggestions clearly.             Current read-only project context: project={project:?}, tempo={bpm:.1} BPM,             view={view:?}, selected channel index={channel}.             You cannot click controls, create notes, modify the project, or see screenshots.             Do not claim to have performed DAW actions."
        );
        let endpoint = self.endpoint.clone();
        let model = self.model.clone();
        let key = self.api_key.clone();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.status = "Sending request…".into();
        ctx.request_repaint();

        thread::spawn(move || {
            let result = http_chat(&endpoint, &model, &key, &system, &messages);
            let _ = tx.send(result);
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
    stream.read_to_end(&mut response).map_err(|e| format!("Read failed: {e}"))?;
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

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url.strip_prefix("http://").ok_or("Only plain HTTP endpoints are supported; use the local FCC proxy")?;
    let (authority, path) = rest.split_once('/').map_or((rest, "/"), |(a, p)| (a, format!("/{p}")));
    let (host, port) = authority.split_once(':').map_or((authority, 80), |(h, p)| {
        (h, p.parse::<u16>().unwrap_or(0))
    });
    if host.is_empty() || port == 0 || host.contains('@') || host.contains(' ') {
        return Err("Invalid HTTP endpoint".into());
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
