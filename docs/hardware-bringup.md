# Codex Micro hardware bring-up runbook

Everything that can be done without the device is already done (see
[`device-hid.md`](device-hid.md)). This runbook is the **day-one checklist for
when a physical Codex Micro is in hand** — it turns the "needs hardware" list
into a ~15-minute session that confirms detection, harvests the real input map,
and validates LED output.

Do the steps in order. Each one has an exact command and a "pass" condition.

## 0. Prep

- Quit **ChatGPT Desktop** (or pause its Agent Key ownership) so it isn't
  fighting for the interface during capture.
- If `microbridged` is running with a live claim, stop it for the capture step:
  it only needs to be running for the LED step (§4).
- Plug the Micro in over **USB-C** (BLE is out of scope for M2).

## 1. Confirm detection

```sh
system_profiler SPUSBDataType -detailLevel mini | grep -iA3 "work louder\|codex\|0x303a"
```

**Pass:** a record with `Vendor ID: 0x303a` and `Product ID: 0x8360`
(Codex Micro) or `0x8297` / `0x8298` (Creator Micro V2).

Then confirm the daemon sees it:

```sh
cargo run -p microbridged            # in one shell
cargo run -p microbridgectl status   # in another — device should show "Detected"
```

**Pass:** the snapshot reports the Micro as **Detected** (not `mock`).
Record the real `iProduct` string and confirmed PID:

| Field | Documented | Observed |
|---|---|---|
| Product ID | `0x8360` | |
| iProduct string | _(unknown)_ | |
| Manufacturer | `Work Louder` | |

## 2. Capture the real input map (the important one)

```sh
cargo run -p microbridgectl --features hid -- hid-capture 120
```

Then, while it runs, **press each control once, slowly**, in this order:

1. Each of the 6 Agent Keys, left→right, top→bottom
2. Approve, Reject, Interrupt (whatever the deck exposes)
3. New session / cycle focus keys
4. Rotate the dial each way, then press it
5. Flick the joystick up / down / left / right

The tool prints every `v.oai.hid` and `v.oai.rad` event live and, on exit,
a summary of the distinct key strings it saw. Fill this in from that summary:

| Physical control | Observed `k` | `act` | `ag` | → Microbridge action |
|---|---|---|---|---|
| Agent Key 1 | | | | `AgentKeyPress { index: 0 }` |
| Agent Key 2 | | | | `AgentKeyPress { index: 1 }` |
| Agent Key 3 | | | | `AgentKeyPress { index: 2 }` |
| Agent Key 4 | | | | `AgentKeyPress { index: 3 }` |
| Agent Key 5 | | | | `AgentKeyPress { index: 4 }` |
| Agent Key 6 | | | | `AgentKeyPress { index: 5 }` |
| Approve | | | | `Approve` |
| Reject | | | | `Reject` |
| Interrupt | | | | `Interrupt` |
| New session | | | | `NewSession` |
| Cycle focus | | | | `CycleFocus` |
| Dial rotate + / − | (`v.oai.rad`?) | | | `DialRotate` |
| Dial press | | | | `DialPress` |
| Joystick up/down/left/right | (`v.oai.rad` angle) | | | `JoystickFlick` |

Also note the **double-press window** for Agent Keys: press one key twice
quickly and confirm the timing threshold (ChatGPT Desktop uses ≤ 350 ms).

## 3. Apply the map in code

With the observed strings in hand, update:

- `agent_key_index` in [`crates/mb-device/src/lib.rs`](../crates/mb-device/src/lib.rs)
  if the real Agent Key strings differ from the current guess.
- `notify_to_input` in the same file to route Approve / Reject / Interrupt /
  New session / Cycle focus / dial press from their real `k` strings.
- The characterization tests in that file's `mod tests` — replace the guessed
  expectations with the confirmed strings so the map is locked by CI.

At that point, wire `device.poll_input()` into the daemon loop in
[`crates/microbridged/src/main.rs`](../crates/microbridged/src/main.rs) (a
dedicated blocking-read task feeding an mpsc channel of `DeviceInput`, so idle
CPU stays at zero). This step is intentionally left until the map is real —
routing guessed strings would just ship a broken deck.

## 4. Validate LED output

```sh
export MICROBRIDGE_HID_CLAIM=1
cargo run -p microbridged
# drive a session through its states (or run the reference echo adapter):
node adapters/reference-echo/index.mjs
```

**Pass:** the six frosted Agent Keys light in the Codex state palette and change
on state transitions; the focused key is distinguishable.

Check against ChatGPT Desktop's own visuals and tune if needed:

- effect / speed mapping (`e`, `s` in `v.oai.thstatus`)
- brightness (`b`) at the daemon's default vs. ChatGPT's
- packed color correctness (`c` = `0xRRGGBB`)

## 5. Ownership UX vs ChatGPT Desktop

Confirm the coexistence story with both running:

- With `MICROBRIDGE_HID_CLAIM=1` **and** ChatGPT Desktop open, do the LEDs
  fight? Document the winner and the recovery (Settings → Pause LEDs).
- Confirm non-exclusive open (macOS `set_open_exclusive(false)`) actually lets
  both read without erroring.

## Done criteria

- [ ] Detection confirmed with real PID + iProduct string (§1)
- [ ] Full input map captured and recorded (§2)
- [ ] Map applied to `agent_key_index` / `notify_to_input` + tests (§3)
- [ ] `poll_input` wired into the daemon loop (§3)
- [ ] LEDs render correct colors on transitions (§4)
- [ ] Ownership vs ChatGPT Desktop documented (§5)

When these are checked, cut a hardware-capable release (`v0.2.0`) and update the
status table in [`device-hid.md`](device-hid.md).
