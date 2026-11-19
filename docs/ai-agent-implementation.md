# AI Agent integration plan

This document tracks the AI assistant integration in Honeydew. The implementation is staged and must be verified with a native build before it is described as working.

## Goal

An opt-in AI assistant that observes the active Honeydew project and visible UI, then edits a beat through observable, paced UI interactions (one action at a time). It is not a bulk arrangement generator.

## Verified source integration points

- Native app: `src/app.rs`, `HoneydewApp`, `eframe::App::update`, `HoneydewApp::frame`.
- UI rendering is egui. Main content and views are laid out in the `CentralPanel` closure inside `frame`.
- Desktop MCP bridge: `src/app/control.rs`, opt-in loopback TCP with token authentication; requests are processed on the egui thread via `control_frame`.
- Current MCP actions include song/project inspection and bulk arrangement. Bulk arrange must not be used in manual interaction mode.
- The piano roll interaction model is documented in `src/ui/piano_roll/mod.rs`: empty-grid taps add notes, and drag gestures manipulate selections.

## Required subsystems

1. **Agent panel** — dedicated horizontal 16:9 chat/agent surface, chat history, model/provider configuration, and current action display.
2. **Project/UI context** — structured context (song title, tempo, time signature, selected channel, active view, overlays, visible editor, instrument/channel data, and note/clip information).
3. **Screen capture and deletion** — explicit opt-in, capture Honeydew's own window/client area only; never capture other apps or the desktop. Use a short-lived in-memory image buffer only, transmit only to the configured local FCC endpoint, and clear the buffer immediately after the model response or on cancellation/error. Do not write screenshots to disk, logs, chat history, or project files. A buffer clear is best-effort memory cleanup, not a guarantee of forensic erasure or deletion from any provider/server memory. Capture API remains unverified in this fork.
4. **Action executor** — a serial queue of mouse move, click press/release, key press/release, wheel, and drag actions. Each action advances across frames, uses configurable pacing, and requests a fresh observation before the next decision.
5. **Visible feedback** — drawn cursor/target marker and action/status text, with Pause, Resume, Stop, and cancel.
6. **Manual mode guardrails** — never invoke bulk `arrange`/apply-arrangement while the agent is in manual mode; ask for confirmation before destructive project actions; keep the local control bridge loopback-only and authenticated.
7. **FCC connection** — FCC README confirms an OpenAI-compatible API at `http://127.0.0.1:8082/v1` (Chat Completions: `/chat/completions`) and model discovery at `/v1/models?view=messages`. Requests stream; the client must handle SSE. FCC proxy auth is optional, configured by the user. Endpoint/model/auth configuration must remain editable in Honeydew.

## Implementation stages

- [x] A (partial): opt-in AI chat side panel and state/status UI, hidden by default.
- [ ] B. Serial UI action queue and egui event injection, with unit tests for press/release ordering and cancellation.
- [x] C (partial): chat receives current tempo, active view, and selected channel index.
- [ ] D. Window-only screenshot capture and visual context delivery (native desktop; validate capture API, explicit opt-in, transient buffers, and post-response cleanup).
- [x] E (partial): local FCC chat via OpenAI-compatible `/v1/chat/completions` endpoint, asynchronous thread request, configurable model and optional token. Streaming/model discovery are not implemented.
- [ ] F. AI action/observation loop: observe, decide, execute one action, wait, observe, continue.
- [ ] G. Manual beat workflow: navigate to drum/piano editor, add notes visibly one at a time, verify each, audition, refine.
- [ ] H. Build, tests, and end-to-end manual QA on supported desktop OS.

## Safety, privacy, and scope

- Agent stays off until the user explicitly enables it.
- Screenshot capture requires a separate, explicit permission and a visible capture indicator.
- Only Honeydew window content is captured, not camera or unrelated desktop windows.
- Screenshot data is transient: no persistent screenshot files or history; clear temporary image buffers after each model request and on stop/error. Disclose that this cannot force remote/local model-server logs or memory to erase data.
- Show active capture/control state clearly and allow immediate stop.
- Provider credentials are stored locally and never exposed to the MCP action queue.
- External model calls are not assumed to be free or private; disclose the provider being used.
- This feature is desktop-native first; web/mobile behavior needs separate feasibility review.

## Status

- [x] A (partial): AI chat panel and local FCC chat request implemented in `src/app/ai_agent.rs`, integrated in `src/app.rs`.
- [x] C/E (partial): tempo/view/channel context and non-streaming chat call implemented.
- [ ] Screenshot capture and image context are not implemented; the transient-only/no-disk/no-history deletion policy is documented as a requirement.
- [ ] UI action executor and observe/act loop are not implemented.
- [ ] Native build and test verification have not been run.
