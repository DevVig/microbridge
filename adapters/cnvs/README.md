# CNVS integration

CNVS support is compiled into `microbridged` and enabled by default. It uses
CNVS's authenticated local control API; it does not scrape CNVS internals,
read its private database, modify a workspace, or install a plugin.

## Setup

There is no pairing code. Start CNVS and Microbridge. CNVS publishes a
short-lived loopback endpoint descriptor, and Microbridge connects
automatically while both apps are running.

Each session has the stable identity `cnvs:<canvas-id>:<node-id>`. This lets an
Agent Key route to the exact CNVS workspace and terminal even when different
canvases are running different harnesses such as Codex or Claude Code.

## Capabilities

- Lifecycle: active CNVS agent terminals and their working, waiting, done, or
  error state.
- Open/focus: focuses the exact canvas and terminal node.
- Interrupt: stops the agent running in that exact terminal.

CNVS does not currently expose stable controls for approval/rejection, starting
a new agent, or reasoning effort through this contract, so Microbridge does not
advertise those actions.

## Footprint and privacy

CNVS's state contract is snapshot-based. Microbridge refreshes the local API
every 2 seconds while an agent is active and every 10 seconds while idle, then
emits only state transitions. It accepts only loopback endpoints, rereads the
descriptor for every scan or action, and never logs or persists the token.

When a CNVS terminal maps exactly to a raw Codex or Claude journal by runtime
and working directory, the native CNVS session replaces that raw card while
CNVS owns it. The journal observation remains cached and returns if CNVS exits.
Cursor's current managed lifecycle contract does not include enough stable
workspace identity for equivalent exact reconciliation, so a Cursor-hosted
terminal may still appear twice if both integrations report it.
