#!/usr/bin/env node
// Minimal Microbridge adapter: registers one fake session and walks it
// through the state machine so you can watch the daemon (and device) react.
//
//   cargo run -p microbridged     # shell 1
//   node index.mjs                # shell 2

import net from "node:net";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";

const socketPath =
  process.env.MICROBRIDGE_SOCKET ??
  path.join(os.homedir(), ".microbridge", "microbridged.sock");

const socket = net.createConnection(socketPath);
const send = (message) => socket.write(`${JSON.stringify(message)}\n`);

const session = {
  id: "echo:demo",
  app: "Echo",
  title: "reference adapter demo",
  state: "idle",
  updated_at_ms: Date.now(),
};

socket.on("connect", async () => {
  send({ type: "hello", adapter: "reference-echo", protocol_version: 0 });
  for (const state of ["thinking", "working", "awaiting_approval", "working", "done"]) {
    session.state = state;
    session.updated_at_ms = Date.now();
    send({ type: "status", session });
    console.log(`→ ${state}`);
    await sleep(1500);
  }
  send({ type: "bye", session_id: session.id });
  socket.end();
});

socket.on("error", (error) => {
  console.error(`cannot reach microbridged at ${socketPath}: ${error.message}`);
  console.error("is the daemon running? (cargo run -p microbridged)");
  process.exit(1);
});
