# microbridge-ui

Primary Microbridge UI — a **macOS menu bar app** (status + setup). Agent
actions (approve / reject / interrupt) stay on the physical Codex Micro.

| Surface | Behavior |
|---|---|
| Menu bar icon | Left-click toggles the popover; right-click exposes claim/release, updates, Settings, and Quit |
| Popover | Contextual Codex Micro claim/release/retry, focus card, device echo, threads, and utilities |
| Settings | Keys (device twin) · Agent Keys · Integrations · Device · Updates |
| Focus HUD | ~2.5s toast when deck focus changes |

MagicPath mockups remain the visual go-to (`vendor/magicpath/`).

## Develop

```sh
# terminal 1 — daemon
cargo run -p microbridged

# terminal 2 — menu bar app
cd apps/microbridge-ui && npm install && npm run tauri dev
```

Browser-only (no tray): `npm run dev` — uses a demo snapshot if the daemon
is unavailable.

## Build

```sh
npm run build          # frontend only (CI)
npm run tauri build    # Microbridge.app
```

Installed macOS builds register the signed main app through `SMAppService` for
Launch at Login. Development binaries under `target/debug` never register.

The UI connects as `role:ui` on the Microbridge Unix socket, keeps a live
subscribe, and never opens HID.
