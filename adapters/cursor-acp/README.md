# Cursor Agent (ACP)

Separate from the **Cursor** IDE Composer tile. This integration drives
[Cursor ACP](https://cursor.com/docs/cli/acp) (`agent acp`) for agents that
Microbridge owns — not the already-open IDE chat panel.

## Enable

1. Install the Cursor CLI so `agent` or `cursor-agent` is on `PATH`.
2. Microbridge Settings → Integrations → **Cursor Agent (ACP)** → Enable.
3. Use **New Session** / Interrupt / Approve / Reject from Microbridge when a
   session is bound to this adapter.

## Capabilities (when CLI is present)

| Action | ACP method |
|---|---|
| New session | `session/new` |
| Interrupt | `session/cancel` |
| Approve | `session/request_permission` → `allow-once` |
| Reject | `session/request_permission` → `reject-once` |

IDE Composer open/focus by `conversation_id` remains unavailable from public
Cursor APIs; keep using the Cursor tile for lifecycle + workspace deep links.

## Privacy

ACP traffic stays on the local machine. Prompt content is not mirrored onto the
Microbridge status bus unless a future opt-in is added.
