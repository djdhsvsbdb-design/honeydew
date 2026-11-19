# Free Claude Code (FCC) integration

## Attribution and licensing

The optional backend is [Free Claude Code](https://github.com/Alishahryar1/free-claude-code), authored by **Ali Khokhar** (copyright holder and lead maintainer/owner). The upstream maintainer list also names **Rahul Rai**. FCC is licensed under **AGPL-3.0-only**; consult the upstream [LICENSE](https://github.com/Alishahryar1/free-claude-code/blob/main/LICENSE) before copying, modifying, bundling, or distributing any FCC source.

Honeydew currently connects to FCC as an external local service. This integration does not copy FCC source code into Honeydew.

## Current connection contract

- Default local endpoint: `http://127.0.0.1:8082/v1/chat/completions`
- Request format: OpenAI-compatible Chat Completions JSON.
- Authentication: optional bearer token, matching FCC's Proxy Authentication setting.
- Transport: loopback HTTP only; the current client rejects remote hosts and HTTPS.
- Response: non-streaming `choices[0].message.content`.
- FCC's model inventory endpoint is `GET /v1/models`; model discovery is not yet wired into the Honeydew panel.

FCC itself also exposes other APIs, including Anthropic Messages, OpenAI Responses, and Gemini. Honeydew currently uses only Chat Completions.

## Visual-agent boundary

Honeydew's action schema is deliberately one-step-at-a-time. The intended control cycle is:

1. Capture the Honeydew window only, with explicit user consent and a visible capture indicator.
2. Send the screenshot and task context to the model.
3. Receive and validate exactly one proposed UI action.
4. Execute that single action through the desktop input layer.
5. Wait for the UI to settle, capture a fresh screenshot, and verify the result before asking for another action.

Pause, resume, stop, and human takeover must remain available. Destructive operations require explicit confirmation. Screenshots should be transient and must not become a persistent screenshot history.

**Not implemented yet:** screenshot capture, screenshot upload/vision input, mouse and keyboard execution, the verified action loop, pause/resume/stop, and human takeover. The current action JSON parser validates proposals only; it does not execute them. The panel's read-only project context is not a substitute for visual observation.

## Next implementation steps

1. Add FCC model discovery through `GET /v1/models`.
2. Replace the hand-written HTTP/1.1 client with a maintained HTTP client that supports local FCC endpoints robustly, while preserving loopback-only restrictions.
3. Add a non-streaming multimodal request path for screenshots, gated by consent and transient handling.
4. Implement a single-action executor and screenshot verification loop behind an explicit desktop-only capability gate.
5. Add tests for endpoint validation, API response parsing, action validation, cancellation, and screenshot lifecycle.
6. Keep the visual agent disabled on web/mobile until equivalent safe capture and input facilities exist.

