# Honeydew live MCP (local desktop)

This opt-in adapter lets a local MCP client edit the **open** Honeydew song. It
can inspect channels, identify configured VST/CLAP instruments, list installed
instruments, create note/drum clips, adjust built-in synth parameters, and build
built-in effect chains. Existing VST/CLAP effect parameters can be edited when
the plugin is loaded. An
arrangement is one undoable edit. Honeydew's existing synth knob changes are
not individually undoable, including those made through this adapter.
External plugin presets are kept as-is; adding a new external plugin or choosing
its preset is not supported yet. The adapter does not listen on the network:
it talks only to Honeydew's authenticated `127.0.0.1` bridge.

## Start

Enable MCP in Honeydew Settings and use `Generate` under Authentication token.
The Python adapter reads the same protected token file automatically. The port
defaults to 57831; set `HONEYDEW_CONTROL_PORT` only when using a custom port.
`HONEYDEW_CONTROL_TOKEN` remains available as a compatibility override.

From this repository:

```sh
cargo run --release --bin honeydew-native
```

For a one-command token setup, use the launcher instead:

```sh
tools/honeydew-mcp/honeydew-mcp.sh app
```

It stores a private token at `~/.config/honeydew/control-token`. Configure the
MCP client to launch `tools/honeydew-mcp/honeydew-mcp.sh adapter`; that mode
reads the same token and starts the stdio adapter automatically.

The MCP client starts the adapter as a **stdio** server; normally you do not
run it by hand. Configure its command as `python3` and its argument as the
absolute path to `tools/honeydew-mcp/server.py`. In Codex, for example, add
the stdio server in Settings → MCP servers, then use its tools in a new chat.
ChatGPT web requires a separate remote/tunneled MCP connection;
this local stdio configuration does not automatically make it available there.

## Multi-track composition

The desktop app also has a **Project → AI Compose** panel. It creates a fresh
four-track arrangement (drums, bass, chords and lead), sets tempo, and appends
matching built-in effects. It does not replace existing clips; the result is
one undoable song edit. The offline note generator is deterministic for a given
seed. It is a procedural composer, not a trained MIDI model.

The `compose_song` MCP tool provides the same multi-track workflow to local
MCP clients and can optionally use an OpenAI-compatible model to interpret the
brief before generating notes.

Optional AI brief interpretation uses an OpenAI-compatible chat-completions
endpoint. Set `HONEYDEW_AI_API_KEY`, and optionally
`HONEYDEW_AI_BASE_URL` and `HONEYDEW_AI_MODEL`, in the environment of the
MCP adapter. Without a key, genre/mood/tempo classification uses local rules and
composition still works offline. When configured, the prompt is sent to that
endpoint for classification. No API key is required for the local fallback.

Example: “Create 8 bars of dark melodic techno, sparse drums, a driving bass,
wide chords and a restrained lead.” Then listen and revise the generated tracks.

## Example request

Ask the client: “Use the Honeydew tools to inspect my open song. Make an
original uplifting trance groove in eight bars using my existing Vital bass
track if there is one; put drums on a new Drums track. Show me the track IDs
and ask me to listen before revising.”

The `get_song` tool returns the channel IDs, their instrument names, clip IDs,
and drum pad indices. `apply_arrangement` accepts 1–16 tracks and optional
`tempo_bpm`. Each track uses
an existing channel ID or creates a new *built-in* synth/Drums track. Clip
positions are zero-based bars; notes' `beat` and `length_beats` are quarter-note
beats **relative to the clip**. A note's `key` is a MIDI pitch (0–127) or a
drum pad index, and velocity is 1–255. To revise a clip, use `replace_clip_id`
with its existing `start_bar` and `bars`. Invalid input changes nothing.

To shape a built-in synth, first call `get_song` for its `channel_id`, then
`get_instrument_params` for its `module_id`, parameter indexes, names, current
values, and defaults. `set_instrument_params` accepts 1–32 changes on that
module as normalized values from 0 to 1. All changes are validated together
and sent to the live audio engine; a bad index or value changes nothing. For
Transistor Bass, for example, indexes 2 and 9 are Cutoff and Drive. The tool
does not load named presets or alter a VST/CLAP plugin's private state.

To discover available built-in effects, call `list_effect_catalog`. To append
an effect chain to a track, call `apply_sound_design` with those exact effect
names and normalized parameter values. To inspect or tune built-in and loaded
VST/CLAP audio effects, call `list_effects` with the
channel ID. It returns each controllable built-in effect's module ID, parameter
indexes, names, current normalized values, defaults and display text.
`set_effect_params` changes 1–32 parameters on one effect (values 0–1), after
validating the entire request. Module parameter indexes are non-negative; common
module controls use -1 (Post Gain), -2 (Mix) and -3 (Wet Pan). The audio engine receives the changes live and
the song state is updated. These parameter edits are not individually undoable.
Existing VST/CLAP plugin effects are listed using the parameter metadata saved
in the song and can be adjusted through the same normalized parameter path when
their plugin instance is loaded. Creating a new third-party plugin instance or
loading its private preset is not supported by this bridge. Use
`apply_sound_design` to append built-in effects to a channel; effect names
must match the built-in catalogue.

Only enable the bridge for trusted local clients. The token is a bearer secret:
any local process that obtains it can edit the song. The bridge binds only to
loopback and validates/bounds each request, but it does not implement per-tool
permissions or expose a public endpoint.
