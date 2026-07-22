# Lean overhead gate

Microbridge must stay a light menu-bar + daemon companion. Control planes are
**event-driven or attach-on-demand** — never new always-on agent children or
extra poll loops for Codex/Claude.

## Rules

- MCP TCP listener is **off** unless `MICROBRIDGE_MCP=1`
- Cursor ACP spawns `agent acp` only on New Session
- Codex control uses `codex app-server proxy` against an existing socket, then exits
- Claude approvals use push hooks + decision files (no daemon poll)

## Baseline check (local)

With the daemon idle (no Micro HID claim, no active agent turns):

```sh
# process count / RSS snapshot (macOS)
pgrep -lf microbridged
ps -o pid,rss,pcpu,comm -p "$(pgrep -n microbridged)"
```

After a change that adds a control path, re-run the same commands. Idle RSS and
CPU should stay within noise; `pgrep` should not show an extra `codex`/`agent`
child while nothing is focused.
