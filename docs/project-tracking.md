# Project tracking

**Source of truth: Linear team VIGDEV**, project **Microbridge Production Launch**.

GitHub milestones/issues were a temporary stand-in and have been closed. Do not
re-open them for planning.

## Bootstrap / recreate Linear project

1. Create a personal API key: https://linear.app/settings/api  
2. Run:

```sh
LINEAR_API_KEY=lin_api_... node scripts/bootstrap-linear-project.mjs
```

That creates the project, marks Phase A items Done (already shipped in `v0.1.0`),
leaves the native action pipeline open, and files Phase B HID issues with
`Blocked Dependency` until the Micro arrives (2026-07-22).

Related Linear tracking (Cursor ≈ Claude path):

- [VIGDEV-795](https://linear.app/vigdev/issue/VIGDEV-795/microbridge-native-action-pipeline-claudecodex-approverejectinterrupt) — Native Claude/Codex action pipeline (Codex app-server attach + Claude PermissionRequest hooks)
- [VIGDEV-796](https://linear.app/vigdev/issue/VIGDEV-796/microbridge-cursor-acp-control-plane-session-binding-and-effort) — Cursor ACP session binding / effort

**Deferred until existing 12 integrations are lever-complete:** Zed Settings card, then Cmux. Do not start those adapters yet.

## Phases

| Phase | Window | Focus |
|---|---|---|
| A | → 2026-07-21 | Public software release (mostly shipped) |
| B | from 2026-07-22 | Real Micro HID |
