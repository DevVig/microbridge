# Factory integration

Enable Factory in **Microbridge Settings → Adapters**. The app transactionally
merges Microbridge-owned commands into Factory's official user hooks at
`~/.factory/hooks.json` and installs a signed helper at
`~/.microbridge/integrations/factory/microbridgectl`. Existing hooks are
preserved; **Remove** deletes only Microbridge-owned entries and the helper.

## Supported behavior

- Lifecycle: `SessionStart`, `UserPromptSubmit`, `Notification`, `PreToolUse`,
  `PostToolUse`, `Stop`, and `SessionEnd`.
- Interrupt: Factory's public `droid.interrupt_session` JSON-RPC method.
- Knob: Factory's public `droid.update_session_settings` method. Microbridge
  reads the active session's documented settings and cycles only the reasoning
  levels advertised by the installed Droid model catalogue.

There is no resident Factory bridge process and no polling. Hooks run on state
transitions; control starts `droid` only for the requested hardware action.
Microbridge finds Droid in Factory.app, `~/.local/bin`, or common Homebrew
locations; custom installations can set `FACTORY_DROID_PATH` for the daemon.
