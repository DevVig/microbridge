// MICROBRIDGE_OPENCODE_PLUGIN
// Installed by Microbridge into ~/.config/opencode/plugins/microbridge.mjs.

import net from "node:net";
import os from "node:os";
import path from "node:path";

const ADAPTER_ID = "opencode";
const ADAPTER_VERSION = "__MICROBRIDGE_VERSION__";
const MAX_PENDING_MESSAGES = 128;
const MIN_RECONNECT_DELAY_MS = 1_000;
const MAX_RECONNECT_DELAY_MS = 30_000;

const capabilities = {
  lifecycle_observation: true,
  approval_acceptance: false,
  approval_rejection: false,
  interrupt: true,
  new_session: false,
  focus_open: false,
  reasoning_effort: false,
};

export function sessionIdFromEvent(event) {
  const properties = event?.properties ?? {};
  return (
    properties.sessionID ??
    properties.sessionId ??
    properties.info?.id ??
    properties.session?.id ??
    null
  );
}

export function stateFromEvent(event) {
  const type = event?.type ?? "";
  if (type === "permission.asked") return "awaiting_approval";
  if (type === "session.error") return "error";
  if (type === "session.idle") return "done";
  if (type === "session.created" || type === "session.updated") return "idle";
  if (type !== "session.status") return null;

  const status = event?.properties?.status;
  const value = typeof status === "string" ? status : status?.type;
  switch (value) {
    case "busy":
    case "active":
    case "working":
      return "working";
    case "retry":
    case "thinking":
      return "thinking";
    case "error":
      return "error";
    case "idle":
      return "done";
    default:
      return null;
  }
}

export function titleFromInfo(info, directory) {
  const title = info?.title?.trim();
  if (title) return title.slice(0, 72);
  const candidate = info?.directory || directory;
  const basename = candidate ? path.basename(candidate) : "";
  return basename || "OpenCode session";
}

function createBridge({ client, directory }) {
  const socketPath =
    process.env.MICROBRIDGE_SOCKET ||
    path.join(os.homedir(), ".microbridge", "microbridged.sock");
  const metadata = new Map();
  const latestStates = new Map();
  const pending = [];
  let socket = null;
  let reconnectTimer = null;
  let reconnectDelayMs = MIN_RECONNECT_DELAY_MS;
  let disposed = false;
  let input = "";

  const send = (message) => {
    const line = `${JSON.stringify(message)}\n`;
    if (socket?.writable) {
      socket.write(line);
      return;
    }
    pending.push(line);
    if (pending.length > MAX_PENDING_MESSAGES) pending.shift();
  };

  const publish = async (sessionID, state, suppliedInfo) => {
    if (!sessionID || !state) return;
    if (suppliedInfo) metadata.set(sessionID, suppliedInfo);
    let info = metadata.get(sessionID);
    if (!info) {
      try {
        const response = await client.session.get({ path: { id: sessionID } });
        info = response?.data ?? response;
        if (info) metadata.set(sessionID, info);
      } catch {
        // Lifecycle truth is still useful when a title lookup races deletion.
      }
    }
    latestStates.set(sessionID, state);
    send({
      type: "status",
      session: {
        id: `${ADAPTER_ID}:${sessionID}`,
        app: "OpenCode",
        title: titleFromInfo(info, directory),
        state,
        updated_at_ms: Date.now(),
      },
    });
  };

  const handleAction = async (message) => {
    if (message?.type !== "action" || message.action !== "interrupt") return;
    const prefix = `${ADAPTER_ID}:`;
    if (!message.session_id?.startsWith(prefix)) return;
    const sessionID = message.session_id.slice(prefix.length);
    if (!sessionID) return;
    try {
      await client.session.abort({ path: { id: sessionID } });
    } catch {
      // The daemon keeps capability routing honest; OpenCode owns user-facing errors.
    }
  };

  const scheduleReconnect = () => {
    if (disposed || reconnectTimer) return;
    const delay = reconnectDelayMs;
    reconnectDelayMs = Math.min(reconnectDelayMs * 2, MAX_RECONNECT_DELAY_MS);
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, delay);
  };

  const connect = () => {
    if (disposed || socket) return;
    const next = net.createConnection({ path: socketPath });
    socket = next;
    next.setEncoding("utf8");
    next.on("connect", () => {
      reconnectDelayMs = MIN_RECONNECT_DELAY_MS;
      next.write(
        `${JSON.stringify({
          type: "hello",
          adapter: ADAPTER_ID,
          protocol_version: 0,
          adapter_version: ADAPTER_VERSION,
          capabilities,
        })}\n`,
      );
      for (const line of pending.splice(0)) next.write(line);
      for (const [sessionID, state] of latestStates) {
        void publish(sessionID, state, metadata.get(sessionID));
      }
    });
    next.on("data", (chunk) => {
      input += chunk;
      while (input.includes("\n")) {
        const newline = input.indexOf("\n");
        const line = input.slice(0, newline).trim();
        input = input.slice(newline + 1);
        if (!line) continue;
        try {
          void handleAction(JSON.parse(line));
        } catch {
          // Ignore malformed daemon lines and retain the connection.
        }
      }
    });
    next.on("error", () => {});
    next.on("close", () => {
      if (socket === next) socket = null;
      input = "";
      scheduleReconnect();
    });
  };

  connect();

  return {
    async event(event) {
      const sessionID = sessionIdFromEvent(event);
      if (event?.type === "session.deleted") {
        if (sessionID) {
          metadata.delete(sessionID);
          latestStates.delete(sessionID);
          send({ type: "bye", session_id: `${ADAPTER_ID}:${sessionID}` });
        }
        return;
      }
      const info = event?.properties?.info ?? event?.properties?.session;
      const state = stateFromEvent(event);
      if (sessionID && (state || info)) {
        await publish(sessionID, state ?? latestStates.get(sessionID) ?? "idle", info);
      }
    },
    async activity(sessionID, state) {
      await publish(sessionID, state);
    },
    dispose() {
      disposed = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      reconnectTimer = null;
      socket?.destroy();
      socket = null;
    },
  };
}

export const Microbridge = async ({ client, directory }) => {
  const bridge = createBridge({ client, directory });
  return {
    event: async ({ event }) => bridge.event(event),
    "chat.message": async ({ sessionID }) => bridge.activity(sessionID, "thinking"),
    "permission.ask": async (input, output) => {
      if (output.status === "ask") await bridge.activity(input.sessionID, "awaiting_approval");
    },
    "tool.execute.before": async ({ sessionID }) => bridge.activity(sessionID, "working"),
    "tool.execute.after": async ({ sessionID }) => bridge.activity(sessionID, "working"),
    dispose: async () => bridge.dispose(),
  };
};
