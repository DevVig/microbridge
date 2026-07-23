#!/usr/bin/env node
/**
 * Claude Code PermissionRequest / PreToolUse bridge for Microbridge.
 * Zero idle cost: only runs when Claude invokes the hook.
 *
 * Pending approvals: ~/.microbridge/claude-pending/<id>.json
 * Decisions:         ~/.microbridge/claude-pending/<id>.decision (or latest.decision)
 */

import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const PENDING = path.join(os.homedir(), ".microbridge", "claude-pending");
const SOCKET =
  process.env.MICROBRIDGE_SOCKET ||
  path.join(os.homedir(), ".microbridge", "microbridged.sock");

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  const text = Buffer.concat(chunks).toString("utf8").trim();
  if (!text) return {};
  try {
    return JSON.parse(text);
  } catch {
    return {};
  }
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function sessionId(input) {
  return (
    input.session_id ||
    input.sessionId ||
    input.conversation_id ||
    input.transcript_path ||
    "latest"
  );
}

function ingestLifecycle(id, state) {
  return new Promise((resolve) => {
    const socket = net.createConnection(SOCKET);
    const finish = () => {
      socket.destroy();
      resolve();
    };
    socket.setTimeout(800, finish);
    socket.on("error", finish);
    socket.on("close", finish);
    socket.on("connect", () => {
      const messages = [
        {
          type: "hello",
          adapter: "claude-hook",
          protocol_version: 0,
          role: "ui",
          adapter_version: "0.3.8",
          capabilities: {},
        },
        {
          type: "ingest_lifecycle",
          adapter_id: "claude",
          session: {
            id: `claude:${id}`,
            app: "Claude Code",
            title: "Claude · awaiting approval",
            state,
            updated_at_ms: Date.now(),
            focus_uri: null,
          },
          ttl_ms: 10 * 60 * 1000,
        },
      ];
      for (const message of messages) {
        socket.write(`${JSON.stringify(message)}\n`);
      }
      // Fire-and-forget: daemon does not reply; close so we do not wait on timeout.
      socket.end();
    });
  });
}

async function waitDecision(id, timeoutMs) {
  ensureDir(PENDING);
  const specific = path.join(PENDING, `${id}.decision`);
  const latest = path.join(PENDING, "latest.decision");
  const interruptFlag = path.join(PENDING, "interrupt", `${id}.flag`);
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    for (const file of [specific, latest]) {
      try {
        if (fs.existsSync(file)) {
          const decision = fs.readFileSync(file, "utf8").trim();
          fs.unlinkSync(file);
          return decision;
        }
      } catch {
        /* ignore */
      }
    }
    try {
      if (fs.existsSync(interruptFlag)) {
        fs.unlinkSync(interruptFlag);
        return "deny_interrupt";
      }
    } catch {
      /* ignore */
    }
    await delay(50);
  }
  return null;
}

function decisionOutput(hookEventName, decision) {
  if (decision === "allow") {
    return {
      hookSpecificOutput: {
        hookEventName,
        decision: { behavior: "allow" },
      },
    };
  }
  if (decision === "deny_interrupt") {
    return {
      hookSpecificOutput: {
        hookEventName,
        decision: {
          behavior: "deny",
          message: "Interrupted from Microbridge",
          interrupt: true,
        },
      },
    };
  }
  return {
    hookSpecificOutput: {
      hookEventName,
      decision: {
        behavior: "deny",
        message: "Rejected from Microbridge",
      },
    },
  };
}

const mode = process.argv[2] || "permission";
const input = await readStdin();
const id = String(sessionId(input)).replace(/[^a-zA-Z0-9._-]/g, "_");

if (mode === "pretool") {
  const interruptFlag = path.join(PENDING, "interrupt", `${id}.flag`);
  if (fs.existsSync(interruptFlag)) {
    try {
      fs.unlinkSync(interruptFlag);
    } catch {
      /* ignore */
    }
    process.stdout.write(
      JSON.stringify(decisionOutput("PreToolUse", "deny_interrupt")),
    );
    process.exit(0);
  }
  process.stdout.write("{}");
  process.exit(0);
}

ensureDir(PENDING);
fs.writeFileSync(
  path.join(PENDING, `${id}.json`),
  JSON.stringify({ id, at: Date.now(), tool: input.tool_name || null }),
);
await ingestLifecycle(id, "awaiting_approval");

const decision = await waitDecision(id, 10 * 60 * 1000);
if (!decision) {
  // Timeout: let Claude's normal UI handle it (do not silently deny).
  process.stdout.write("{}");
  process.exit(0);
}

process.stdout.write(
  JSON.stringify(decisionOutput("PermissionRequest", decision)),
);
