# Claude Code hooks for Microbridge

Push-only PermissionRequest bridge. No daemon polling.

## Install

Microbridge Settings can merge hooks into `~/.claude/settings.json`, or add:

```json
{
  "hooks": {
    "PermissionRequest": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node \"/path/to/microbridge/adapters/claude/hooks/microbridge-permission.mjs\" permission"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node \"/path/to/microbridge/adapters/claude/hooks/microbridge-permission.mjs\" pretool"
          }
        ]
      }
    ]
  }
}
```

When enabling the Claude integration, Microbridge installs a copy under
`~/.microbridge/claude-hooks/` and points settings at that path.

## Privacy

Only session id + lifecycle state are sent to the local Microbridge socket.
Tool arguments and prompt text are not forwarded.
