# OpenCode integration

Enable OpenCode in **Microbridge Settings → Integrations**. Microbridge installs
one dependency-free global plugin at
`~/.config/opencode/plugins/microbridge.mjs`. OpenCode loads global plugins for
both its CLI and desktop surfaces.

The plugin uses OpenCode's documented plugin events and SDK client. It opens a
local Unix-socket connection to `microbridged`; there is no helper process,
polling loop, telemetry, or external network request.

## Supported behavior

- Lifecycle: session create/update/status/idle/error/delete, tool activity, and
  permission-waiting state.
- Interrupt: OpenCode's documented `client.session.abort()` method for the exact
  session owning the Agent Key.

OpenCode does not currently expose a stable exact-session focus operation or a
model-aware per-session reasoning-effort mutation through this plugin contract,
so Microbridge does not advertise those controls.

**Remove** deletes only the Microbridge-owned plugin. Restart OpenCode after
installing, repairing, or removing the integration if it was already running.
