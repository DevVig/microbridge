import type { SessionStatus, Snapshot } from "./types";

/**
 * Rows visible in the popover before the list scrolls. The ranking below is
 * what makes a viewport this size workable: whatever you're most likely to be
 * looking for is already at the top.
 */
export const VISIBLE_THREAD_ROWS = 10;

/** Fixed row height, so `VISIBLE_THREAD_ROWS` is exact rather than approximate. */
export const THREAD_ROW_HEIGHT = 28;

/**
 * Safety valve, not a product limit — the list scrolls, so threads past the
 * viewport are still reachable. This only exists so a runaway session count
 * can't put thousands of rows in the DOM.
 */
const RENDER_LIMIT = 50;

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
  limit = RENDER_LIMIT,
): { threads: SessionStatus[]; total: number; truncated: boolean } {
  const onKeys = new Set(
    snapshot.agent_key_session_ids.filter((id): id is string => Boolean(id)),
  );
  const ranked = [...snapshot.sessions].sort((a, b) => {
    const diff = rank(b, snapshot, onKeys) - rank(a, snapshot, onKeys);
    if (diff !== 0) return diff;
    return b.updated_at_ms - a.updated_at_ms;
  });
  const threads = ranked.slice(0, limit);
  return {
    threads,
    total: snapshot.sessions.length,
    truncated: snapshot.sessions.length > threads.length,
  };
}
