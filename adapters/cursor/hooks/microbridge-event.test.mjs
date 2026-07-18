import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { lifecycleMessages, microbridgeEvent } from "./event.mjs";

test("maps only local identifiers and display metadata", () => {
  const event = microbridgeEvent(
    {
      conversation_id: "conversation-1",
      workspace_roots: ["/Users/test/Example"],
      prompt: "must never be forwarded",
      text: "must never be forwarded",
      tool_args: { secret: true },
    },
    "working",
  );
  assert.deepEqual(event, {
    conversationId: "conversation-1",
    lifecycle: "working",
    title: "Cursor · Example",
    workspace: "/Users/test/Example",
  });
  assert.equal(JSON.stringify(event).includes("must never"), false);
});

test("missing daemon never blocks Cursor", () => {
  const hook = fileURLToPath(new URL("./microbridge-event.mjs", import.meta.url));
  const result = spawnSync(process.execPath, [hook, "thinking"], {
    input: JSON.stringify({ conversation_id: "conversation-2" }),
    encoding: "utf8",
    env: { ...process.env, MICROBRIDGE_SOCKET: "/definitely/missing/microbridged.sock" },
  });
  assert.equal(result.status, 0);
  assert.equal(result.stdout, "{}\n");
});

test("creates a self-contained lifecycle socket message", () => {
  const [hello, ingest] = lifecycleMessages(
    microbridgeEvent(
      {
        conversation_id: "conversation-3",
        workspace_root: "/tmp/example",
        prompt: "private prompt",
      },
      "after_agent_response",
    ),
    1234,
  );
  assert.deepEqual(hello, {
    type: "hello",
    adapter: "cursor-hook",
    protocol_version: 0,
    role: "ui",
    adapter_version: "0.2.1",
    capabilities: {},
  });
  assert.equal(ingest.type, "ingest_lifecycle");
  assert.equal(ingest.session.id, "cursor:conversation-3");
  assert.equal(ingest.session.state, "done");
  assert.equal(ingest.session.updated_at_ms, 1234);
  assert.equal(JSON.stringify(ingest).includes("private prompt"), false);
});

test("duplicate hook payloads normalize to the same event", () => {
  const payload = { session_id: "session-1", cwd: "/tmp/project" };
  assert.deepEqual(
    microbridgeEvent(payload, "done"),
    microbridgeEvent(payload, "done"),
  );
});
