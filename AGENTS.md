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
