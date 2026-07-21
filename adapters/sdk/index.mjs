// @microbridge/adapter-sdk
// Zero-dependency Node.js/JS SDK for building Microbridge adapters.

import net from "node:net";
import os from "node:os";
import path from "node:path";

export class MicrobridgeAdapter {
  constructor({ id, version = "1.0.0", capabilities = {} }) {
    this.id = id;
    this.version = version;
    this.capabilities = {
      lifecycle_observation: true,
      approval_acceptance: false,
      approval_rejection: false,
      interrupt: false,
      new_session: false,
      focus_open: false,
      reasoning_effort: false,
      tty_control: false,
      mcp_native: false,
      uri_focus: false,
      ...capabilities,
    };

    this.socketPath =
      process.env.MICROBRIDGE_SOCKET ||
      path.join(os.homedir(), ".microbridge", "microbridged.sock");
    this.socket = null;
    this.actionListeners = new Map();
  }

  connect() {
    return new Promise((resolve, reject) => {
      this.socket = net.createConnection(this.socketPath, () => {
        const hello = {
          type: "hello",
          adapter: this.id,
          protocol_version: 0,
          adapter_version: this.version,
          capabilities: this.capabilities,
        };
        this.socket.write(`${JSON.stringify(hello)}\n`);
        resolve(true);
      });

      this.socket.on("error", (err) => {
        reject(err);
      });

      let buffer = "";
      this.socket.on("data", (chunk) => {
        buffer += chunk.toString("utf8");
        const lines = buffer.split("\n");
        buffer = lines.pop();
        for (const line of lines) {
          if (!line.trim()) continue;
          try {
            const msg = JSON.parse(line);
            if (msg.type === "action") {
              const handler = this.actionListeners.get(msg.action);
              if (handler) handler(msg.session_id);
            }
          } catch (_) {}
        }
      });
    });
  }

  reportStatus({ id, app, title = "", state, focusUri }) {
    if (!this.socket || !this.socket.writable) return;
    const msg = {
      type: "status",
      session: {
        id: `${this.id}:${id}`,
        app: app || this.id,
        title,
        state,
        updated_at_ms: Date.now(),
        focus_uri: focusUri || null,
      },
    };
    this.socket.write(`${JSON.stringify(msg)}\n`);
  }

  reportBye(id) {
    if (!this.socket || !this.socket.writable) return;
    const msg = {
      type: "bye",
      session_id: `${this.id}:${id}`,
    };
    this.socket.write(`${JSON.stringify(msg)}\n`);
  }

  onAction(actionName, callback) {
    this.actionListeners.set(actionName, callback);
  }

  disconnect() {
    if (this.socket) {
      this.socket.end();
      this.socket = null;
    }
  }
}
