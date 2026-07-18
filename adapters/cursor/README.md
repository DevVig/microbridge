# Microbridge for Cursor

This directory is the Cursor integration bundled with Microbridge. It reports
agent lifecycle state to the local Microbridge daemon without reading Cursor
databases, installing global hooks, using Accessibility automation, or creating
replacement sessions.

## Install and consent

1. Open **Microbridge Settings → Adapters**.
2. Click **Enable Cursor**. Microbridge installs its bundled integration into
   Cursor's supported local-plugin directory after this explicit consent.
3. Reload Cursor once if it is already open. **Remove** disables the adapter
   and deletes only the Microbridge-owned local plugin directory.

The hook talks directly to `~/.microbridge/microbridged.sock` and sends only the
conversation id, lifecycle state, and workspace-derived display label. It does
not depend on `microbridgectl` being on Cursor's PATH. Prompt, response,
transcript, and tool argument content are not sent.

The same source is public here for review and development. A separate
Marketplace download is not required, and integration updates ship with the
Microbridge app.

## Capability boundary

Lifecycle observation is implemented. Cursor does not currently expose stable
public APIs for authoritative approval acceptance, session-scoped interrupt,
opening an existing thread, or reasoning-effort changes. Microbridge therefore
reports this adapter as **Limited** and never falls back to private storage or
Accessibility scripting.

Run a hook locally:

```sh
printf '{"conversation_id":"demo","workspace_root":"/tmp/example"}' \
  | node hooks/microbridge-event.mjs working
```
