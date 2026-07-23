# UI design

The menu bar app is the **primary** way people deal with the keyboard.
**MagicPath interactive mockups are the go-to reference**; this document
describes what they show and why.

## Lean companion principle

Microbridge is a **status + setup** companion, not a second agent cockpit.

- **The Micro owns actions.** Approve, reject, interrupt, and switch focus happen on the physical keys.
- **The UI connects and configures.** Connection status, pause LEDs, key remapping, lighting, adapters, appearance.
- **The UI never competes.** It mirrors daemon-resolved focus; it never talks to HID.

On-screen surfaces may *show* state colors and which session owns the deck, but
do not expose those states as buttons. Pinning and key-source live in
Settings → Agent Keys, not in the daily menu bar popover.

## Design language — "Device White"

The UI borrows its material language from the hardware itself: the Codex
Micro's white sandblasted-polycarbonate body, frosted translucent keycaps, and
LEDs that carry all of the color. Layered on OpenAI's Codex app conventions:
near-monochrome neutrals, sentence-case Inter/OpenAI-Sans type, rounded cards
with hairline borders, and tinted status chips as the only color in the chrome.

**Light is the designed-first default.** Appearance follows the system and is
configurable only in Settings → Device → Appearance (System / Light / Dark).
There is no theme toggle in the popover or titlebars — one coherent look per
mode. Dark mode is "the device on a dark desk": the same frosted physics with
the white device unchanged.

| Token | Light (default) | Dark |
|---|---|---|
| Desk / frame | `#E9E9E7` radial | `#0A0A0B` radial |
| Window material | `rgba(252,252,251,0.86)` + 36px backdrop blur | `rgba(24,24,26,0.88)` + blur |
| Card / raised | `#FFFFFF` / `#F4F4F2` sunken | `rgba(255,255,255,0.05)` / `rgba(0,0,0,0.25)` |
| Hairline | `rgba(0,0,0,0.08)` | `rgba(255,255,255,0.09)` |
| Text primary / secondary / muted | `#0D0D0D` / `#6E6E73` / `#AEAEB2` | `#F5F5F4` / `#A0A0A6` / `#5E5E66` |
| Selection ring | `#3D7EFF` (macOS-style focus blue) | same |
| Type | Inter (OpenAI Sans stand-in), sentence case; mono only for data (firmware, footprints) | same |
| Motion | 150–250ms transitions; slow LED pulses for attention states; nothing decorative | same |

Status is rendered as Codex-style chips — soft tinted pills ("Working",
"Needs approval", "Done") — never as buttons.

## State colors — Codex defaults, user-customizable

LED colors are client-side rendering config (the protocol carries states,
never colors). The default palette matches the color language the Codex
Micro ships with:

| State | Default | Treatment |
|---|---|---|
| `idle` | white `#E9E9E6` | soft static glow |
| `thinking` | blue `#3D7EFF` | slow breathe |
| `working` | blue `#3D7EFF` | solid |
| `awaiting_approval` | amber `#FFB000` | attention pulse |
| `done` | green `#30C463` | static |
| `error` | red `#FF453A` | static |
| unassigned | off | unlit |

Every state color is editable in Settings → Device → Lighting, with one-click
**Reset to Codex defaults** and an alternate built-in preset ("Phosphor").

## The device

Design targets the real kbd-1.0 hardware, laid out exactly as shipped:

```
(dial)   [AG1]  [AG2]   {joystick}
[AG3]    [AG4]  [AG5]   [AG6]
[⚡Fast] [✓Approve] [✗Reject] [↗Fork]
(touch·LEDs) [   Mic 2U   ] [Codex]
```

13 mechanical switches, a rotary dial (default: reasoning effort), a planar
joystick (flicks trigger skills — review PR, debug, refactor, explain), a
capacitive touch sensor beside three status LEDs, a 2U push-to-talk mic bar,
and the Codex key (new chat). Six frosted **Agent Keys** glow with the state
of the thread each one follows; the command caps ship printed with their
icons. The box includes 32 icon keycaps and 11 solid caps for re-capping
remapped keys.

Agent Key press semantics (parity with ChatGPT desktop): single press
switches the followed thread in the background; double press (≤350ms) also
brings its window forward. Which threads the six Agent Keys follow is the
**key source**: focused app (default — the deck re-populates with the
owning IDE's newest threads) / most recent (cross-app) / pinned / priority /
custom assignment. Agent Keys follow one IDE at a time by default; switch
to most recent for a cross-app monitoring surface. Command keys always
route to the single daemon-resolved focused thread.

The on-screen **device twin** is a photo-accurate vector rendering of the
actual hardware — white plate (white in both themes), frosted agent caps with
the switch stem visible through the frost, printed command icons, dial,
joystick, touch sensor, corner screws, and plate print. Vector rather than a
photo so the Agent Keys can light with live state colors. Every control is
clickable for setup; the twin is **descriptor-driven**: layout comes from what
the device reports, never a hardcoded grid.

**Agent Keys are never blank remappable keys.** On the twin they render the
live LED color of their thread, and selecting one shows the thread it follows
(app, title, state, deck ownership) with a pointer to Agent Keys settings —
not an action picker.

## Surfaces (MagicPath go-to)

### 1. Menu bar popover — [interactive mockup](https://api.magicpath.ai/v1/safely-park-1411)

The daily driver. **Read-mostly** — no agent actions, no theme toggle.

- Header: wordmark + device connection chip
- Hero focus card: app, thread title, read-only state chip, elapsed time, reasoning pill (dial echo), press-behavior hint
- **Mini device echo**: a passive miniature of the real deck (dial, joystick, six lit Agent Keys, command row) — read-only, labeled as such
- Threads list: state dot + app + title + elapsed — no approve/reject, no click-to-focus
- Contextual hardware card: Claim when detected, Release when connected, Retry
  for recoverable claim failures such as missing Input Monitoring permission or
  another process owning the HID interface.
- Footer: Settings · Pause LEDs · Quit

When disconnected, the popover shows a connection-first empty state ("Connect
your Codex Micro") with the echo unlit — no fake agent chrome.

### 2. Settings window — [interactive mockup](https://api.magicpath.ai/v1/cool-gulf-2537)

Keyboard setup. Four sections in a left rail:

- **Keys** — the full device twin + per-control inspector: action picker and
  listen mode for command keys (with the shipped cap shown), rotate/press for
  the dial (reasoning-effort preview), four flick-to-skill bindings for the
  joystick, tap action for the touch sensor. Agent Keys show their live
  thread and route to Agent Keys settings.
- **Agent Keys** — live "six keys, six threads" view, key source, deck focus
  mode (auto/pinned), app priority order, approvals-interrupt toggle (policy
  only — no live approve UI)
- **Integrations** — dense icon tile grid (3–5 columns, ~72px tall), grouped by
  connected / not connected. Each tile shows the host app icon, name, and
  traffic-light status; hover reveals the diagnostic. Click Cursor / Factory /
  T3 / OpenCode to open a detail strip for enable / repair / pair.
  Host-attributed apps (Synara, ChatGPT, Claude Desktop, Conductor) are status +
  hover only
- **Device** — Appearance (System/Light/Dark), Lighting (Codex defaults +
  Phosphor preset + reset), brightness, LED test, sleep timer (default 3 min),
  firmware, zero-network note

### 3. Focus HUD — [interactive mockup](https://api.magicpath.ai/v1/sunnily-shadow-8075)

A transient, **non-interactive** frosted overlay confirming **deck focus** —
which single thread currently owns Approve / Reject / dial / command keys.

**When it appears (you don’t open it):** whenever the daemon’s focused
session changes, for about 2.5 seconds — for example after you press an
Agent Key, an approval preempts another thread, or auto-follow moves the
deck to another app. It is a glanceable confirmation, not a settings
screen. No buttons on the HUD card.

Contents: app badge, app name, thread title, state chip, a six-key echo
with the focused key lit, press-behavior hint, and a 2px drain bar.

## Interaction rules

- **No competing owner, ever.** Every surface reflects the single resolved
  focus from the daemon; UI never talks to the device.
- **UI is read-mostly.** Only connection and config controls are interactive
  in the popover; agent actions stay on the Micro.
- **Focus changes are always confirmed** — by the HUD on-screen and by the
  deck itself.
- **Remapping is direct**: click a control on the twin *or* press it
  physically ("listen" mode), then assign. That is configuring the keyboard,
  not driving agents from the UI.
- **Approvals are privileged on the deck**, not in the popover. An
  `awaiting_approval` thread may temporarily claim the approve/reject keys
  (user-toggleable in Settings → Agent Keys).
- Quitting the menu bar app does not stop the daemon or LEDs; you lose
  status/setup chrome until you reopen it.

## Assets

MagicPath project: `Microbridge — Agent Control Surface`
(id `428891101303308288`) — components `safely-park-1411` (menu bar),
`cool-gulf-2537` (settings), `sunnily-shadow-8075` (HUD).

Hardware reference: OpenAI Supply Co. × Work Louder product page
(`openai.com/supply/co-lab/work-louder/`) — source of the layout, shipped
icon set, and material language.

When mockups and this doc differ, **trust the mockups**.

## Implementation

The shipping companion lives in [`apps/microbridge-ui`](../../apps/microbridge-ui)
(Tauri 2 + React): macOS **menu bar tray** → popover / settings / HUD windows,
live bus subscribe, MagicPath-faithful device twin + echo. Vendored MagicPath
exports for reference are under `apps/microbridge-ui/vendor/magicpath/`.

Frontmost-app auto-follow is owned by `microbridged` (`NSWorkspace`), not the UI.

Static screenshots for the README live in
[`docs/screenshots/`](../screenshots/).
