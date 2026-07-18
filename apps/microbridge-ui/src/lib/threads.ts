import type { SessionStatus, Snapshot } from "./types";

function rank(session: SessionStatus, snapshot: Snapshot, onKeys: Set<string>): number {
  let score = 0;
  if (session.id === snapshot.focused_session_id) score += 1000;
  if (onKeys.has(session.id)) score += 100;
  switch (session.state) {
    case "awaiting_approval":
      score += 50;
      break;
    case "working":
    case "thinking":
      score += 30;
      break;
    case "error":
      score += 20;
      break;
    case "done":
      score += 5;
      break;
    default:
      break;
  }
  return score;
}

/** Threads shown in the menu bar popover — focused / on keys / active first. */
export function visibleThreads(
  snapshot: Snapshot,
): { threads: SessionStatus[]; total: number; truncated: boolean } {
  const onKeys = new Set(
    snapshot.agent_key_session_ids.filter((id): id is string => Boolean(id)),
  );
  const ranked = [...snapshot.sessions].sort((a, b) => {
    const diff = rank(b, snapshot, onKeys) - rank(a, snapshot, onKeys);
    if (diff !== 0) return diff;
    return b.updated_at_ms - a.updated_at_ms;
  });
  const threads = ranked;
  return {
    threads,
    total: snapshot.sessions.length,
    truncated: snapshot.sessions.length > threads.length,
  };
}
