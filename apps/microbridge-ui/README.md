# microbridge-ui

Primary Microbridge UI (Tauri 2 menu bar app). **Status + setup** for the
keyboard — agent actions (approve / reject / interrupt) stay on the physical
Codex Micro.

MagicPath mockups remain the visual go-to:

| Surface | MagicPath | App route |
|---|---|---|
| Menu bar popover | `safely-park-1411` | `?view=popover` (default) |
| Settings | `cool-gulf-2537` | `?view=settings` |
| Focus HUD | `sunnily-shadow-8075` | `?view=hud` |

Vendored MagicPath exports (reference): [`vendor/magicpath/`](vendor/magicpath/).

## Develop

```sh
# terminal 1 — daemon
cargo run -p microbridged

# terminal 2 — web UI (demo snapshot if daemon/Tauri unavailable)
cd apps/microbridge-ui && npm install && npm run dev

# or full Tauri shell (needs Xcode CLT)
npm run tauri dev
```

## Build

```sh
npm run build          # frontend only (CI)
npm run tauri build    # macOS app bundle
```

The UI connects as `role:ui` on the Microbridge Unix socket and never opens HID.
