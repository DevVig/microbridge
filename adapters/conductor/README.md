# Conductor integration

Conductor needs no pairing code for lifecycle detection. Codex and Claude
sessions running under `~/conductor/workspaces` are attributed to **Conductor**
by Microbridge's existing journal watchers, with no new poller or network call.

Conductor's public API exposes session creation, status, cancellation, and deep
links, but does not expose a reliable mapping from an underlying Codex/Claude
journal id to the Conductor session id or an active-session effort update.
Microbridge therefore advertises lifecycle only instead of guessing with local
database reads or keyboard automation.
