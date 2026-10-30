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

## Before opening a pull request

1. **Search first.** Before creating a branch or PR for a named task, list open PRs in this repo (`gh pr list --state open` or GitHub search) and look for an existing PR whose title, branch, or description matches the same task (including drafts). Match on task intent, not only exact title — e.g. "independent PR review fallback", "Codex review fallback", same Linear/issue id.
2. **At most one open PR per task.** If an open PR already covers the task, do **not** open another. Update that PR's branch in place, or stop and report the existing PR URL. Renaming a branch (e.g. `codex/…` → `chore/…`) is **not** a reason to open a second PR: push the rename to the existing PR or close the old one in the same change and keep a single open PR.
3. **Fallback / review-policy PRs.** At most one open fallback, review-policy, or review-evidence setup PR per repo for the same design. If a newer approved design already merged (or is open) for that purpose, do not open a parallel draft of an older design — stop and report the superseding PR.
4. **Multi-repo rollouts.** When copying the same change across repos, check each target repo independently before opening. A shared payload hash or identical title across org repos is not permission to open a second PR in a repo that already has one.

If unsure whether two PRs are the same task, prefer updating the earlier open PR and ask the owner before opening another.
