#!/usr/bin/env node
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { lifecycleMessages, microbridgeEvent } from "./event.mjs";

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

const input = await readStdin();
const lifecycle = process.argv[2] ?? "working";
const event = microbridgeEvent(input, lifecycle);
const socketPath =
  process.env.MICROBRIDGE_SOCKET ??
  path.join(os.homedir(), ".microbridge", "microbridged.sock");

// Never send prompt, response, transcript, or tool argument content. Hook
// failures are intentionally non-blocking for the user's Cursor workflow.
await new Promise((resolve) => {
  const socket = net.createConnection(socketPath);
  const finish = () => {
    socket.destroy();
    resolve();
  };
  socket.setTimeout(1_250, finish);
  socket.on("error", finish);
  socket.on("data", finish);
  socket.on("connect", () => {
    for (const message of lifecycleMessages(event)) {
      socket.write(`${JSON.stringify(message)}\n`);
    }
  });
});

process.stdout.write("{}\n");
