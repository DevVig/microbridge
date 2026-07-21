import { basename } from "node:path";

const LIFECYCLE_STATES = {
  idle: "idle",
  stop: "idle",
  session_end: "idle",
  thinking: "thinking",
  before_submit_prompt: "thinking",
  after_agent_thought: "thinking",
  working: "working",
  pre_tool_use: "working",
  post_tool_use: "working",
  awaiting_approval: "awaiting_approval",
  done: "done",
  after_agent_response: "done",
  error: "error",
};

export function microbridgeEvent(input, lifecycle) {
  const conversationId =
    typeof input.conversation_id === "string" && input.conversation_id
      ? input.conversation_id
      : typeof input.session_id === "string" && input.session_id
        ? input.session_id
        : "unknown";
  const workspace =
    Array.isArray(input.workspace_roots) && typeof input.workspace_roots[0] === "string"
      ? input.workspace_roots[0]
      : typeof input.workspace_root === "string"
        ? input.workspace_root
        : typeof input.cwd === "string"
          ? input.cwd
          : "";
  return {
    conversationId,
    lifecycle,
    title: workspace ? `Cursor · ${basename(workspace)}` : "Cursor agent",
    workspace,
  };
}

export function lifecycleMessages(event, now = Date.now()) {
  const state = LIFECYCLE_STATES[event.lifecycle] ?? "working";
  return [
    {
      type: "hello",
      adapter: "cursor-hook",
      protocol_version: 0,
      role: "ui",
      adapter_version: "0.2.1",
      capabilities: {},
    },
    {
      type: "ingest_lifecycle",
      adapter_id: "cursor",
      session: {
        id: `cursor:${event.conversationId}`,
        app: "Cursor",
        title: event.title,
        state,
        updated_at_ms: now,
        focus_uri: event.workspace ? `cursor://file${event.workspace}` : null,
      },
      ttl_ms: event.lifecycle === "session_end" ? 1_000 : 30 * 60 * 1_000,
    },
  ];
}
