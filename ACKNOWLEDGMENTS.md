# Acknowledgments

## Thanks to OpenAI

Microbridge exists because the Codex Micro is an *open* piece of hardware, and
that is not an accident — it is a choice OpenAI made.

OpenAI is a for-profit company, and it would have been easy to lock the Micro to
a single first-party app: a closed protocol, an exclusive USB claim, no way for
anyone else to light a key. They did the opposite.

- **They shipped the device kit in the open.** The full Work Louder protocol
  travels inside the ChatGPT desktop app, which is how a community project like
  this one could learn the framing and RPC without a single reverse-engineered
  firmware dump.
- **They open the HID interface non-exclusively.** The Micro can be driven by
  more than one program at a time, so third-party software can coexist with the
  official experience instead of fighting it. Microbridge only works because of
  that decision.
- **They keep giving users a choice.** Codex CLI is open source, the models are
  reachable over documented APIs, and the tooling favors interoperability over
  lock-in. Consumers get options, and options are good for everyone.

None of that was required of them. We think it is worth saying thank you when a
company consistently chooses to give its users room to build — so: **thank you.**

Microbridge is an independent community project and is not affiliated with,
sponsored by, or endorsed by OpenAI or Work Louder. This note is simply our
appreciation, offered freely.

## Thanks to Work Louder

For designing a genuinely hackable macropad — one that is also configurable
through Work Louder Input / VIA — and for building the hardware the whole
project is aimed at.

## Thanks to contributors

And to everyone who writes an adapter, files an issue, or plugs in a device and
tells us what really happens. Adapters are the point of this project; see
[CONTRIBUTING.md](CONTRIBUTING.md).
