# T3 Code adapter

The T3 Code integration runs inside `microbridged`; this directory documents
its host contract and replaces the former idle Node scaffold.

## Pairing

1. Enable T3 Code in **Microbridge Settings → Integrations**.
2. In T3 Code, open **Settings → Connections**, enable **Network access**, then
   create a one-time link under **Authorized clients**.
3. Paste the link into Microbridge. The one-time token is exchanged immediately.
4. The resulting bearer credential is stored in macOS Keychain under
   `ai.microbridge.t3code` and is removed by **Remove**.

Microbridge uses T3 Code's authenticated public endpoints:

- `GET /api/orchestration/shell` for lifecycle snapshots.
- `GET /api/orchestration/threads/:threadId` for pending approval identity.
- `POST /api/orchestration/dispatch` for approval and interrupt commands.
- `t3code://threads/:environmentId/:threadId` to open the owning thread.

It never reads T3 Code databases, bootstrap credentials, or desktop internals.

The compatibility suite is pinned to T3 server `0.0.28` and upstream contract
commit `ebe8afb1df357423a0e036b388af3e739d640205`. Other server versions are
reported as **Incompatible** until Microbridge verifies and ships their contract.
The adapter opens threads with T3 Code's semantic deep link. Reasoning effort
stays disabled because the current paired HTTP snapshot includes the selected
value but not the provider/model option descriptors needed to choose the next
valid level safely. Microbridge does not fake this with key presses or private
desktop state.

The compatibility target is the contract present in `pingdotgg/t3code` as of
July 18, 2026. Authentication failures return to **Needs setup**; transport
failures show **Connecting** and retry with the existing paired credential.
