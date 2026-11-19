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

## Execution speed and interaction modes

Interaction method is strictly visual and manual: screenshot of Honeydew → AI decision → one mouse/keyboard UI action → verify via screenshot. Do not use direct internal APIs, MCP actions, bulk note/arrangement edits, or data-level shortcuts to speed up the workflow.

Speed optimization must preserve this visual-only method:
- Minimize model round trips by batching reasoning/planning locally, but execute UI actions one at a time.
- Capture screenshots only when needed to establish context or verify meaningful UI state changes; never skip verification when target/state is uncertain.
- Use adaptive, bounded wait for UI response instead of long fixed delays.
- Stream model responses where supported.
- Reuse a screenshot only while the UI has not changed in a way that invalidates it.
- Keep Pause/Resume/Stop and human takeover responsive; Stop must halt further actions.
- Temporary screenshot data stays in memory and must not be written to files, chat history, or logs.

## Full application capability coverage

The intended agent is not limited to a small fixed MCP action list. It should be able to reach the full Honeydew feature set through visible, sequential UI control, while using structured app context where available. The capability inventory must be maintained against the actual UI and feature modules; unsupported actions must be reported as unavailable rather than simulated or claimed completed.

Capability groups to cover and verify:

- Project lifecycle: create/open/save/save-as, project versions, export, import MIDI, project metadata.
- Timeline/playlist: tracks/channels, clips, sections, arrangement navigation and playback controls.
- Note and rhythm editing: piano roll, drum/step editor, note add/move/resize/delete, velocity and note properties.
- Instruments and sound: built-in synths, drum kits, samplers, sample selection/import, SoundFont, presets, instrument building.
- Rack and plugins: add/remove/reorder modules, plugin browser and supported plugin editors.
- Mixer and audio: channel levels/pan, routing, sends, master, meters and playback/recording controls.
- Automation: create/edit automation lanes and points, link controls, MIDI CC-related settings.
- Audio clips/editing: clip selection, waveform editor, sample replacement and related operations.
- Settings/preferences: application and input/output settings, keyboard/MIDI setup, themes and other visible preferences.
- Help, credits and app information: navigation and accessible dialogs.

## Access strategy and action protocol

- Discover current capabilities at runtime rather than hard-code an assumed complete list.
- Use the UI as the compatibility path for every feature, including dialogs and plugin interfaces.
- Each agent gesture is serialized: observe, perform exactly one UI action, wait for the UI to settle, observe again, and verify the result.
- Provide Pause, Resume, Stop, cancellation, visible cursor/target feedback and a human takeover mode.
- Structured internal APIs may assist context or narrowly scoped operations, but must not silently bypass the visible manual workflow or invoke bulk arrangement when manual mode is active.
- Require confirmation for destructive actions such as deleting tracks, overwriting projects, replacing samples, or changing global device configuration.
- Maintain an honest capability/status list: mark a feature unavailable if the agent cannot currently navigate or verify it; never report an action as completed without evidence.

## Acceptance criteria for “all app functions”

“All functions” is a coverage goal, not a claim that every Honeydew operation is already implemented. Before declaring full coverage, each feature group above must have a mapped UI route/action sequence, test cases (including cancel/failure paths), and verification criteria. Plugin windows and platform-specific dialogs may need separate support and can remain explicitly unsupported until tested.

## Implementation stages

- [x] A (partial): opt-in AI chat side panel and state/status UI, hidden by default.
- [ ] B. Serial UI action queue and egui event injection, with unit tests for press/release ordering and cancellation. This is the foundation for broad feature coverage.
- [x] C (partial): chat receives current tempo, active view, and selected channel index.
- [ ] D. Window-only screenshot capture and visual context delivery (native desktop; validate capture API, explicit opt-in, transient buffers, and post-response cleanup).
- [x] E (partial): local FCC chat via OpenAI-compatible `/v1/chat/completions` endpoint, asynchronous thread request, configurable model and optional token. Streaming/model discovery are not implemented.
- [ ] F. AI action/observation loop with runtime capability discovery: observe, decide, execute one action, wait, observe, continue. Use the full capability inventory above and keep a verified coverage matrix.
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
