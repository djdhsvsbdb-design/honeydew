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

The desktop app's **Project → AI Compose** panel now has a **Generate with Groq**
action. It starts the bundled local MCP adapter from the app's embedded Python
sources, sends the prompt to Groq, and lets the model inspect the open song,
instruments and effects before it calls Honeydew's authenticated loopback tools.
The resulting arrangement and sound-design edits are applied directly to the
open project; existing clips are preserved by default.

To use the in-app producer:
1. Enable **MCP Control** in Honeydew Settings (and make sure its local control
   token has been created).
2. Open **Project → AI Compose**.
3. Paste the Groq API key into the password field and click **Generate with Groq**.
   The key is held in app memory for the session and passed only to the local
   adapter process; it is not written to the project or settings.
4. Keep Honeydew open while the producer is working. Python 3 must be installed
   and available as `python3` (or `python` / `py -3` on Windows).

The separate `compose_song` MCP tool remains available to external MCP clients.
Without a configured API key it uses the deterministic offline procedural
composer. Neither mode can audition the rendered audio automatically.

## Groq AI producer

Set `GROQ_API_KEY` in the **environment of the MCP adapter process** (not
only in GitHub Actions secrets). On Windows PowerShell:

```powershell
[Environment]::SetEnvironmentVariable("GROQ_API_KEY", "your-groq-key", "User")
```

Then completely restart the MCP client so its adapter inherits the variable.
The adapter defaults to Groq's OpenAI-compatible endpoint and the configured
default model; override with `GROQ_BASE_URL` or `GROQ_MODEL` if needed.
Alternatively, use `HONEYDEW_AI_API_KEY` with `HONEYDEW_AI_PROVIDER=groq`,
and optionally set `HONEYDEW_AI_BASE_URL` and `HONEYDEW_AI_MODEL`.

When a key is present, `compose_song` runs an AI producer agent. It inspects
the open song and available instruments/effects, then uses bounded tool calls
to create an arrangement and adjust supported synth/effect parameters. Without
a key, it falls back to the offline procedural composer.

The AI producer can create MIDI note arrangements and configure supported
built-in sounds/effects. It cannot listen to or judge the rendered audio,
cannot create new third-party VST/CLAP instances, and does not guarantee a
professional mix. Existing project content is preserved by default.

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
