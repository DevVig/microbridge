import assert from "node:assert/strict";
import test from "node:test";

import { sessionIdFromEvent, stateFromEvent, titleFromInfo } from "./microbridge.mjs";

test("maps official OpenCode lifecycle events", () => {
  assert.equal(
    sessionIdFromEvent({ type: "session.status", properties: { sessionID: "ses_1" } }),
    "ses_1",
  );
  assert.equal(
    sessionIdFromEvent({ type: "session.updated", properties: { info: { id: "ses_2" } } }),
    "ses_2",
  );
  assert.equal(
    stateFromEvent({ type: "session.status", properties: { status: { type: "busy" } } }),
    "working",
  );
  assert.equal(stateFromEvent({ type: "permission.asked", properties: {} }), "awaiting_approval");
  assert.equal(stateFromEvent({ type: "session.idle", properties: {} }), "done");
  assert.equal(stateFromEvent({ type: "session.error", properties: {} }), "error");
});

test("uses session title and falls back to the workspace name", () => {
  assert.equal(titleFromInfo({ title: "Ship OpenCode support" }, "/tmp/repo"), "Ship OpenCode support");
  assert.equal(titleFromInfo({}, "/Users/me/dev/microbridge"), "microbridge");
});
