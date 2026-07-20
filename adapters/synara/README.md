# Synara integration

Synara needs no separate adapter process or pairing code. Its Codex and Claude
Agent SDK sessions use the standard agent journals, and Microbridge's built-in
watchers attribute them to **Synara** from the journal's official host metadata
or Synara worktree path.

Lifecycle LEDs and Agent Key assignment work automatically. Commands remain
limited to capabilities exposed by the underlying Codex or Claude contract;
Microbridge does not inspect Synara private state or synthesize keystrokes.
