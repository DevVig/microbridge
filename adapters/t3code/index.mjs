#!/usr/bin/env node
// T3 Code community adapter scaffold — connects, says hello, then idles.

import net from "node:net";
import os from "node:os";
import path from "node:path";

const socketPath =
  process.env.MICROBRIDGE_SOCKET ??
  path.join(os.homedir(), ".microbridge", "microbridged.sock");

const socket = net.createConnection(socketPath);
const send = (message) => socket.write(`${JSON.stringify(message)}\n`);

socket.on("connect", () => {
  send({ type: "hello", adapter: "t3code", protocol_version: 0 });
  console.log("t3code adapter connected (idle — no session source yet)");
});

socket.on("data", (buf) => {
  for (const line of buf.toString("utf8").split("\n")) {
    if (!line.trim()) continue;
    try {
      const msg = JSON.parse(line);
      if (msg.type === "action") {
        console.log("action (no-op until implemented):", msg);
      }
    } catch {
      /* ignore */
    }
  }
});

socket.on("error", (error) => {
  console.error(`cannot reach microbridged at ${socketPath}: ${error.message}`);
  process.exit(1);
});
