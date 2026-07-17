import type { Snapshot } from "../lib/types";
import { STATE_COLORS } from "../lib/types";
import type { ThemeTokens } from "../lib/theme";

/**
 * Miniature, read-only echo of the kbd-1.0 deck (MagicPath AgentKeyEcho).
 * White device in both themes; only Agent Key LEDs carry color.
 */

const U = 26;
const GAP = 6;

function MiniAgentKey({
  index,
  snapshot,
  connected,
}: {
  index: number;
  snapshot: Snapshot;
  connected: boolean;
}) {
  const id = connected ? snapshot.agent_key_session_ids[index] : null;
  const session = id ? snapshot.sessions.find((s) => s.id === id) : null;
  const color = session ? STATE_COLORS[session.state] : null;
  const focused = id != null && id === snapshot.focused_session_id;
  const pulse =
    session?.state === "awaiting_approval"
      ? "mb-led-pulse"
      : session?.state === "thinking"
        ? "mb-led-breathe"
        : "";

  return (
    <span
      className="relative rounded-[6px]"
      style={{
        width: U,
        height: U,
        background:
          "linear-gradient(180deg, rgba(255,255,255,0.62), rgba(238,238,235,0.55))",
        border: focused ? "1.5px solid #3D7EFF" : "1px solid rgba(0,0,0,0.11)",
        boxShadow: `inset 0 1px 0 rgba(255,255,255,0.9)${color ? `, 0 0 8px ${color}77` : ""}`,
      }}
    >
      {color && (
        <span
          className={`absolute inset-[2px] rounded-[4px] ${pulse}`}
          style={{
            background: `radial-gradient(circle at 50% 55%, ${color}${focused ? "E6" : "99"} 0%, ${color}33 55%, transparent 80%)`,
          }}
        />
      )}
      <span
        className="absolute left-1/2 top-1/2 h-[6px] w-[2px] -translate-x-1/2 -translate-y-1/2 rounded-[1px]"
        style={{ backgroundColor: "rgba(70,70,82,0.3)" }}
      />
      <span
        className="absolute left-1/2 top-1/2 h-[2px] w-[6px] -translate-x-1/2 -translate-y-1/2 rounded-[1px]"
        style={{ backgroundColor: "rgba(70,70,82,0.3)" }}
      />
    </span>
  );
}

function MiniWhiteKey({ wide = false }: { wide?: boolean }) {
  return (
    <span
      className="rounded-[6px]"
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
}: {
  t: ThemeTokens;
  snapshot: Snapshot;
}) {
  const connected = snapshot.device_connected;
  return (
    <div
      className="pointer-events-none flex flex-col items-center gap-2"
      aria-hidden="true"
    >
      <div
        className="rounded-[14px] p-[7px]"
        style={{
          background:
            "linear-gradient(180deg, rgba(226,226,224,0.6), rgba(206,206,204,0.5))",
          boxShadow:
            "0 8px 20px rgba(0,0,0,0.18), inset 0 1px 0 rgba(255,255,255,0.5)",
          opacity: connected ? 1 : 0.55,
        }}
      >
        <div
          className="rounded-[9px] p-[10px]"
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
              <MiniAgentKey index={0} snapshot={snapshot} connected={connected} />
              <MiniAgentKey index={1} snapshot={snapshot} connected={connected} />
              <span
                className="flex items-center justify-center rounded-[7px]"
                style={{
                  width: U,
                  height: U,
                  border: "1px dashed rgba(0,0,0,0.25)",
                }}
              >
                <span
                  className="rounded-full"
                  style={{
                    width: U - 6,
                    height: U - 6,
                    background:
                      "radial-gradient(circle at 35% 28%, #3E3E44 0%, #202024 55%, #101013 100%)",
                  }}
                />
              </span>
            </div>
            <div className="flex" style={{ gap: GAP }}>
              <MiniAgentKey index={2} snapshot={snapshot} connected={connected} />
              <MiniAgentKey index={3} snapshot={snapshot} connected={connected} />
              <MiniAgentKey index={4} snapshot={snapshot} connected={connected} />
              <MiniAgentKey index={5} snapshot={snapshot} connected={connected} />
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
                    width: 13,
                    height: 13,
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
        {connected ? "Live on your deck · read-only" : "Deck offline"}
      </span>
    </div>
  );
}
