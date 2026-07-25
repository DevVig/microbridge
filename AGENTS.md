## Learned User Preferences

- Aim for Claude-parity Cursor integration (approve/reject/interrupt/focus/new session), not lifecycle-only Limited status.
- When shipping a Microbridge release, confirm version bumps and that DMG (and other release) assets were published—not only that the PR merged.
- Do not admin-bypass failing required CI on Microbridge merges; CodeRabbit rate-limit failures alone are non-blocking.

## Learned Workspace Facts

- Local checkout `t3PRHelp` is the GitHub repo `DevVig/microbridge` (Tauri UI in `apps/microbridge-ui`, daemon in `crates/microbridged`).
- Integrations split into community adapters (Cursor, Factory, T3 Code, OpenCode) and host-attributed/native hosts (Claude Code, Codex, Synara, CNVS); host-attributed names appear on sessions and may not get separate Integrations tiles.
- Claude reaches full control via FSEvents on `~/.claude/projects` journals; Cursor historically used hooks-only `ingest_lifecycle` (Limited)—ACP is the intended path toward Claude-parity.
- Community adapter “Setup needed” / “Waiting” usually means hooks or the host app are not yet delivering lifecycle events, not only a UI selection bug.
- Release pipeline is tag-driven: GitHub Release (DMGs/tarballs) → Homebrew formula bump PR → `finalize-release` smoke/promote; Intel finalize must use `brew services list --json` (not pipe-to-awk/grep) to avoid Broken pipe flakes.

## Cursor Cloud specific instructions

Standard commands live in the `Makefile` (`make test|clippy|fmt|ci|build|ui`) and `apps/microbridge-ui/package.json`; `.github/workflows/ci.yml` is the source of truth for the required checks (`rust`, `ui`, `adapters`). Non-obvious caveats for this Linux cloud VM:

- System libs `libudev-dev` and `libhidapi-dev` are required to build the daemon/CLI, because the `hid` feature is on by default in `mb-device`/`microbridgectl`. They are baked into the VM snapshot (not the update script). If a build fails with a missing `hidapi`/`libudev` at link time, reinstall them via apt, or build with `--no-default-features` to skip HID.
- Services (run each in its own tmux window; none are backgrounded by the update script):
  - Daemon (core, required): `cargo run -p microbridged`. Creates a Unix socket at `~/.microbridge/microbridged.sock`; config persists to `~/.microbridge/config.toml`.
  - CLI (observe): `./target/debug/microbridgectl status` (also `cargo run -p microbridgectl -- status`).
  - UI (primary app): the full Tauri menu-bar app (`npm run tauri dev`) targets macOS and needs webkit2gtk/GTK libs not installed here. On this VM use the browser-only dev server `npm run dev` (Vite, `http://localhost:1420/`), which renders a demo snapshot without a live daemon.
- Adapters are consent-gated. A newly connecting socket adapter (e.g. `adapters/reference-echo`) is rejected with "pending approval" and its `status` messages are ignored until it is enabled. Only the fixed opt-in/native adapters (cursor, codex, claude, …) are seeded from `config.toml` at startup; ad-hoc adapters like `reference-echo` are NOT re-seeded, so after a daemon restart they must be re-discovered (connect once) before a UI `set_adapter_enabled` will stick. To enable one without the GUI, connect a UI-role client and send `{"type":"set_adapter_enabled","adapter_id":"<id>","enabled":true}` (a UI `hello` with `"role":"ui"` is required).
- Software E2E smoke (no hardware): start the daemon, `node adapters/reference-echo/index.mjs` to walk a fake session through `thinking → working → awaiting_approval → working → done`, and watch `microbridgectl status` reflect the session state and `agent_key_led_frame` colors. HID hardware control stays off unless `MICROBRIDGE_HID_CLAIM=1`.
