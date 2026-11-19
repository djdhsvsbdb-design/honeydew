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
3. **Screen capture** — opt-in capture of Honeydew's own window/client area. Do not capture other apps or the desktop. Capture is a separate implementation item; no screenshot API has yet been verified in this fork.
4. **Action executor** — a serial queue of mouse move, click press/release, key press/release, wheel, and drag actions. Each action advances across frames, uses configurable pacing, and requests a fresh observation before the next decision.
5. **Visible feedback** — drawn cursor/target marker and action/status text, with Pause, Resume, Stop, and cancel.
6. **Manual mode guardrails** — never invoke bulk `arrange`/apply-arrangement while the agent is in manual mode; ask for confirmation before destructive project actions; keep the local control bridge loopback-only and authenticated.
7. **FCC connection** — FCC README confirms an OpenAI-compatible API at `http://127.0.0.1:8082/v1` (Chat Completions: `/chat/completions`) and model discovery at `/v1/models?view=messages`. Requests stream; the client must handle SSE. FCC proxy auth is optional, configured by the user. Endpoint/model/auth configuration must remain editable in Honeydew.

## Implementation stages

- [ ] A. UI panel and agent state (inactive by default; Pause/Stop controls).
- [ ] B. Serial UI action queue and egui event injection, with unit tests for press/release ordering and cancellation.
- [ ] C. Structured project context bridge.
- [ ] D. Window-only screenshot capture and visual context delivery (native desktop; validate capture API and privacy boundaries).
- [ ] E. FCC/provider connection and streamed chat (FCC API contract confirmed: local OpenAI-compatible endpoint; model discovery and SSE streaming).
- [ ] F. AI action/observation loop: observe, decide, execute one action, wait, observe, continue.
- [ ] G. Manual beat workflow: navigate to drum/piano editor, add notes visibly one at a time, verify each, audition, refine.
- [ ] H. Build, tests, and end-to-end manual QA on supported desktop OS.

## Safety, privacy, and scope

- Agent stays off until the user explicitly enables it.
- Only Honeydew window content is captured, not camera or unrelated desktop windows.
- Show active capture/control state clearly and allow immediate stop.
- Provider credentials are stored locally and never exposed to the MCP action queue.
- External model calls are not assumed to be free or private; disclose the provider being used.
- This feature is desktop-native first; web/mobile behavior needs separate feasibility review.

## Status

Architecture/source reconnaissance completed; FCC's API contract confirmed from its README. First documentation commit is on the feature branch. No feature code, screenshot capture, AI provider connection, or native build verification has been completed yet.
