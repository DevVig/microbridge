# UI design

The interface direction for Microbridge's optional UI surfaces. Interactive
mockups live on MagicPath (links below); this document is the spec an
implementer should build from in M3.

## Design language — "Industrial Phosphor"

Work Louder's retro-industrial hardware aesthetic crossed with terminal
phosphor. Dark, precise, glanceable — a hardware companion, not a SaaS
dashboard.

| Token | Value |
|---|---|
| Background / panel / raised | `#0C0C0E` / `#131316` / `#1A1A1F` |
| Hairline borders | `#26262C`, 1px |
| Accent (phosphor orange) | `#FF6A00`; secondary amber `#FFB454` |
| Text | cream `#F4F0E6`; secondary `#9C9890`; muted `#5C5952` |
| Type | JetBrains Mono for labels/data/numerals; Inter for body; section labels 10px uppercase, 0.2em tracking |
| Motion | 150–250ms transitions; slow pulses for attention states; nothing decorative |

**State colors** (used identically on-screen and on the device LEDs):

| State | Color | Treatment |
|---|---|---|
| `idle` | `#4A4A52` | static |
| `thinking` | `#FFB454` | soft pulse |
| `working` | `#FF6A00` | solid glow |
| `awaiting_approval` | `#FF3D00` | attention pulse |
| `done` | `#3DDC84` | static |
| `error` | `#FF4757` | static |

Keycaps render as chunky rounded squares with a top highlight and inner
shadow — they should read as physical keys. The on-screen device mirror is
**descriptor-driven**: layout comes from what the device reports, never a
hardcoded grid.

## Surfaces

### 1. Menu bar popover — [interactive mockup](https://api.magicpath.ai/v1/safely-park-1411)

The daily driver. Header (wordmark + device connection chip), a hero focus
card (focused agent, state badge, elapsed time, live LED-strip preview),
the agents list (state dot, app, session title; click to focus;
awaiting-approval rows reveal inline approve/reject), an AUTO/PINNED focus
segmented control, and a footer (Settings, Pause LEDs, Quit).

### 2. Settings window — [interactive mockup](https://api.magicpath.ai/v1/cool-gulf-2537)

Four sections in a left rail: **KEYS** (device mirror + key inspector:
grouped action picker — Agent / System / Macro / Passthrough — LED behavior,
and a "listen" mode that selects whatever key you physically press;
per-app profile chips where app profiles override GLOBAL on focus),
**FOCUS** (auto/pinned mode, app priority order, "approvals interrupt"
toggle), **ADAPTERS** (native vs community badges, enable toggles,
per-adapter footprint), **DEVICE** (brightness, LED test, sleep, firmware,
and the zero-network badge).

### 3. Focus HUD — [interactive mockup](https://api.magicpath.ai/v1/sunnily-shadow-8075)

A transient overlay (volume-HUD energy) confirming deck ownership when focus
changes: app glyph, "FOCUS →" + app name, session title, state chip, 3-LED
echo, and a 2px drain bar that fades the card after ~2.5s.

## Interaction rules

- **No competing owner, ever.** Every surface reflects the single resolved
  focus from the daemon; UI never talks to the device.
- **Focus changes are always confirmed** — by the HUD on-screen and by the
  deck itself.
- **Remapping is direct**: click a key on the mirror *or* press it
  physically ("listen" mode), then assign. Changes apply live, no save
  button.
- **Approvals are privileged**: whatever is focused, an `awaiting_approval`
  session may temporarily claim the approve/reject keys (user-toggleable).
- The UI processes are optional; quitting them changes nothing about the
  daemon's behavior.

## Assets

MagicPath project: `Microbridge — Agent Control Surface`
(id `428891101303308288`) — components `safely-park-1411` (menu bar),
`cool-gulf-2537` (settings), `sunnily-shadow-8075` (HUD).
