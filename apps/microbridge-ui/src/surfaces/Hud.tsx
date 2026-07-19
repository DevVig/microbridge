import type { Snapshot } from "../lib/types";
import { STATE_COLORS, STATE_LABELS } from "../lib/types";
import { DARK, LIGHT } from "../lib/theme";

export function Hud({
  snapshot,
  dark,
}: {
  snapshot: Snapshot;
  dark: boolean;
}) {
  const t = dark ? DARK : LIGHT;
  const focused = snapshot.sessions.find(
    (s) => s.id === snapshot.focused_session_id,
  );
  if (!focused) return null;

  const color = STATE_COLORS[focused.state];
  const label = STATE_LABELS[focused.state];
  const focusIndex = snapshot.agent_key_session_ids.findIndex(
    (id) => id === focused.id,
  );
  const initials = focused.app
    .split(" ")
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();

  return (
    <div
      className="relative flex h-full min-h-screen w-full flex-col items-center justify-center bg-transparent px-4"
      style={{
        fontFamily: "Inter, system-ui, sans-serif",
      }}
    >
      <div
        className="mb-frost pointer-events-none relative flex w-full max-w-[340px] select-none flex-col overflow-hidden rounded-2xl"
        style={{
          backgroundColor: t.panel,
          border: `1px solid ${t.panelBorder}`,
          boxShadow: t.floatingShadow,
        }}
      >
        <div className="flex items-center gap-3 px-5 pb-4 pt-5">
          <span
            className="flex h-9 w-9 items-center justify-center rounded-xl text-[12px] font-semibold"
            style={{ backgroundColor: t.sunken, color: t.text }}
          >
            {initials}
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="text-[12px] font-medium" style={{ color: t.textSecondary }}>
                {focused.app}
              </span>
              <span
                className="rounded-full px-2 py-0.5 text-[10.5px] font-medium"
                style={{ backgroundColor: `${color}22`, color }}
              >
                {label}
              </span>
            </div>
            <p
              className="truncate text-[15px] font-semibold"
              style={{ color: t.text }}
            >
              {focused.title || focused.id}
            </p>
          </div>
        </div>

        <div className="flex justify-center gap-1.5 px-5 pb-4">
          {snapshot.agent_key_session_ids.map((id, i) => {
            const lit = i === focusIndex;
            const sess = id
              ? snapshot.sessions.find((s) => s.id === id)
              : null;
            const c = sess ? STATE_COLORS[sess.state] : color;
            return (
              <span
                key={i}
                className="relative block h-[18px] w-[18px] rounded-[5px]"
                style={{
                  background:
                    "linear-gradient(180deg, rgba(255,255,255,0.62), rgba(238,238,235,0.55))",
                  border: "1px solid rgba(0,0,0,0.12)",
                  boxShadow: lit ? `0 0 8px ${c}88` : "none",
                }}
              >
                {lit && (
                  <span
                    className="mb-led-pulse absolute inset-[1.5px] rounded-[3.5px]"
                    style={{
                      background: `radial-gradient(circle at 50% 55%, ${c}E6 0%, ${c}33 60%, transparent 85%)`,
                    }}
                  />
                )}
              </span>
            );
          })}
        </div>

        <p
          className="px-5 pb-3 text-center text-[11px]"
          style={{ color: t.textMuted }}
        >
          Press Agent Key to focus · actions stay on the Micro
        </p>
        <div className="h-[2px] w-full" style={{ backgroundColor: t.hairline }}>
          <div
            className="mb-drain h-full"
            style={{ backgroundColor: color, width: "100%" }}
          />
        </div>
      </div>
    </div>
  );
}
