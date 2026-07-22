import type { Snapshot } from "../lib/types";
import { agentKeyLedFrame } from "../lib/types";
import type { ThemeTokens } from "../lib/theme";

/**
 * Miniature, read-only echo of the kbd-1.0 deck (MagicPath AgentKeyEcho).
 * White device in both themes; only Agent Key LEDs carry color.
 */

/**
 * Key unit and gap, in px. Every other measurement here is derived from these,
 * so the deck scales as one piece.
 *
 * These were 26/6. The echo is the single tallest thing in the popover, and at
 * that size it cost ~193px — enough that a 10-row thread list pushed the card
 * past 760px, which is a lot of menu bar to take over. Scaled to 18/5 it reads
 * the same and costs ~145px.
 */
const U = 18;
const GAP = 5;

function MiniAgentKey({
  index,
  snapshot,
  connected,
  onActivate,
}: {
  index: number;
  snapshot: Snapshot;
  connected: boolean;
  onActivate?: (index: number, open: boolean) => void;
}) {
  const frame = agentKeyLedFrame(snapshot);
  const led = frame.keys[index];
  const id = connected ? led?.session_id : null;
  const session = id ? snapshot.sessions.find((s) => s.id === id) : null;
  const color = frame.paused ? null : (led?.color ?? null);
  const focused = Boolean(led?.focused);
  const pulse =
    session?.state === "awaiting_approval"
      ? "mb-led-pulse"
      : session?.state === "thinking" || session?.state === "working"
        ? "mb-led-breathe"
        : "";

  return (
    <button
      type="button"
      disabled={!id || !onActivate}
      onClick={() => onActivate?.(index, false)}
      onDoubleClick={() => onActivate?.(index, true)}
      aria-label={session ? `Agent Key ${index + 1}: ${session.title || session.id}` : `Agent Key ${index + 1}: unassigned`}
      title={session ? `${session.app} · ${session.title || session.id}` : "Unassigned"}
      className="relative rounded-[5px]"
      style={{
        width: U,
        height: U,
        background:
          "linear-gradient(180deg, rgba(255,255,255,0.62), rgba(238,238,235,0.55))",
        border: focused ? "1.5px solid #3D7EFF" : "1px solid rgba(0,0,0,0.11)",
        boxShadow: `inset 0 1px 0 rgba(255,255,255,0.9)${color ? `, 0 0 8px ${color}77` : ""}`,
        opacity: color ? Math.max(0.2, frame.brightness / 100) : 1,
      }}
    >
      {color && (
        <span
          className={`absolute inset-[2px] rounded-[3px] ${pulse}`}
          style={{
            background: `radial-gradient(circle at 50% 55%, ${color}${focused ? "E6" : "99"} 0%, ${color}33 55%, transparent 80%)`,
          }}
        />
      )}
      <span
        className="absolute left-1/2 top-1/2 h-[5px] w-[1.5px] -translate-x-1/2 -translate-y-1/2 rounded-[1px]"
        style={{ backgroundColor: "rgba(70,70,82,0.3)" }}
      />
      <span
        className="absolute left-1/2 top-1/2 h-[1.5px] w-[5px] -translate-x-1/2 -translate-y-1/2 rounded-[1px]"
        style={{ backgroundColor: "rgba(70,70,82,0.3)" }}
      />
    </button>
  );
}

function MiniWhiteKey({ wide = false }: { wide?: boolean }) {
  return (
    <span
      className="rounded-[5px]"
      style={{
        width: wide ? U * 2 + GAP : U,
        height: U,
        background:
          "radial-gradient(ellipse 90% 70% at 50% 38%, #FFFFFF 0%, #F5F5F2 60%, #ECECE8 100%)",
        border: "1px solid rgba(0,0,0,0.09)",
        boxShadow:
          "inset 0 1px 0 rgba(255,255,255,1), 0 1px 1.5px rgba(0,0,0,0.10)",
      }}
    />
  );
}

export function DeviceEcho({
  t,
  snapshot,
  onAgentKey,
}: {
  t: ThemeTokens;
  snapshot: Snapshot;
  onAgentKey?: (index: number, open: boolean) => void;
}) {
  const connected =
    snapshot.device_connected ||
    snapshot.device_name === "mock" ||
    snapshot.device_name.includes("usb");
  return (
    <div className="flex flex-col items-center gap-1.5">
      <div
        className="rounded-[12px] p-[6px]"
        style={{
          background:
            "linear-gradient(180deg, rgba(226,226,224,0.6), rgba(206,206,204,0.5))",
          boxShadow:
            "0 8px 20px rgba(0,0,0,0.18), inset 0 1px 0 rgba(255,255,255,0.5)",
          opacity: connected ? 1 : 0.55,
        }}
      >
        <div
          className="rounded-[8px] p-[8px]"
          style={{
            background: "linear-gradient(180deg, #FBFBF9 0%, #F2F2EF 100%)",
            border: "1px solid rgba(0,0,0,0.07)",
          }}
        >
          <div className="flex flex-col" style={{ gap: GAP }}>
            <div className="flex" style={{ gap: GAP }}>
              <span
                className="rounded-full"
                style={{
                  width: U,
                  height: U,
                  background:
                    "radial-gradient(circle at 34% 28%, #FFFFFF 0%, #EDEDEA 45%, #D9D9D5 100%)",
                  border: "1px solid rgba(0,0,0,0.13)",
                  boxShadow: "0 1.5px 3px rgba(0,0,0,0.16)",
                }}
              />
              <MiniAgentKey index={0} snapshot={snapshot} connected={connected} onActivate={onAgentKey} />
              <MiniAgentKey index={1} snapshot={snapshot} connected={connected} onActivate={onAgentKey} />
              <span
                className="flex items-center justify-center rounded-[6px]"
                style={{
                  width: U,
                  height: U,
                  border: "1px dashed rgba(0,0,0,0.25)",
                }}
              >
                <span
                  className="rounded-full"
                  style={{
                    width: U - 5,
                    height: U - 5,
                    background:
                      "radial-gradient(circle at 35% 28%, #3E3E44 0%, #202024 55%, #101013 100%)",
                  }}
                />
              </span>
            </div>
            <div className="flex" style={{ gap: GAP }}>
              <MiniAgentKey index={2} snapshot={snapshot} connected={connected} onActivate={onAgentKey} />
              <MiniAgentKey index={3} snapshot={snapshot} connected={connected} onActivate={onAgentKey} />
              <MiniAgentKey index={4} snapshot={snapshot} connected={connected} onActivate={onAgentKey} />
              <MiniAgentKey index={5} snapshot={snapshot} connected={connected} onActivate={onAgentKey} />
            </div>
            <div className="flex" style={{ gap: GAP }}>
              <MiniWhiteKey />
              <MiniWhiteKey />
              <MiniWhiteKey />
              <MiniWhiteKey />
            </div>
            <div className="flex items-center" style={{ gap: GAP }}>
              <span
                className="flex items-center justify-center"
                style={{ width: U, height: U }}
              >
                <span
                  className="rounded-full"
                  style={{
                    width: 10,
                    height: 10,
                    background:
                      "radial-gradient(circle at 35% 28%, #2E2E33 0%, #131316 70%)",
                  }}
                />
              </span>
              <MiniWhiteKey wide />
              <MiniWhiteKey />
            </div>
          </div>
        </div>
      </div>
      <span className="text-[10px] font-medium" style={{ color: t.textMuted }}>
        {snapshot.device_connected
          ? "Live on your deck · read-only"
          : snapshot.device_name.includes("usb")
            ? "USB detected · HID map pending"
            : snapshot.device_name === "mock"
              ? "Simulator · read-only"
              : "Deck offline"}
      </span>
    </div>
  );
}
