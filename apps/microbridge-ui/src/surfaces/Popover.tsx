import { useMemo } from "react";
import type { Snapshot } from "../lib/types";
import { STATE_COLORS, STATE_LABELS, elapsed } from "../lib/types";
import { DARK, LIGHT, type ThemeTokens } from "../lib/theme";

const MicroGlyph = ({ color }: { color: string }) => (
  <svg
    width="15"
    height="15"
    viewBox="0 0 24 24"
    fill="none"
    stroke={color}
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <rect x="3.5" y="5" width="17" height="14" rx="3" />
    <path d="M7.5 9h.01M12 9h.01M16.5 9h.01M7.5 13h.01M12 13h.01M16.5 13h.01" />
  </svg>
);

function MiniEcho({
  t,
  snapshot,
}: {
  t: ThemeTokens;
  snapshot: Snapshot;
}) {
  const keys = snapshot.agent_key_session_ids;
  return (
    <div
      className="flex flex-col items-center gap-1.5 rounded-xl px-3 py-2.5"
      style={{ backgroundColor: t.sunken }}
    >
      <div className="grid grid-cols-3 gap-1.5">
        {keys.map((id, i) => {
          const session = id
            ? snapshot.sessions.find((s) => s.id === id)
            : null;
          const color = session ? STATE_COLORS[session.state] : "transparent";
          const focused = id === snapshot.focused_session_id;
          return (
            <span
              key={i}
              className="relative block h-[22px] w-[22px] rounded-[5px]"
              style={{
                background:
                  "linear-gradient(180deg, rgba(255,255,255,0.7), rgba(238,238,235,0.55))",
                border: focused
                  ? "1.5px solid #3D7EFF"
                  : "1px solid rgba(0,0,0,0.12)",
                boxShadow: session ? `0 0 6px ${color}66` : "none",
              }}
            >
              {session && (
                <span
                  className="absolute inset-[2px] rounded-[3px]"
                  style={{
                    background: `radial-gradient(circle at 50% 55%, ${color}E6 0%, ${color}33 60%, transparent 85%)`,
                  }}
                />
              )}
            </span>
          );
        })}
      </div>
      <span className="text-[10px]" style={{ color: t.textMuted }}>
        Device echo · read-only
      </span>
    </div>
  );
}

export function Popover({
  snapshot,
  dark,
  onOpenSettings,
  onTogglePause,
  onQuit,
}: {
  snapshot: Snapshot;
  dark: boolean;
  onOpenSettings: () => void;
  onTogglePause: () => void;
  onQuit: () => void;
}) {
  const t = dark ? DARK : LIGHT;
  const connected = snapshot.device_connected;
  const focused = snapshot.sessions.find(
    (s) => s.id === snapshot.focused_session_id,
  );
  const liveCount = snapshot.agent_key_session_ids.filter(Boolean).length;

  const footerButton = (
    label: string,
    onClick?: () => void,
    active = false,
  ) => (
    <button
      type="button"
      onClick={onClick}
      className="rounded-md px-2 py-1 text-[12px] font-medium transition-colors"
      style={{
        color: active ? t.text : t.textSecondary,
        backgroundColor: active ? t.hoverBg : "transparent",
      }}
    >
      {label}
    </button>
  );

  const frame = useMemo(
    () =>
      dark
        ? "radial-gradient(ellipse 120% 90% at 50% 0%, #131315 0%, #08080A 100%)"
        : "radial-gradient(ellipse 120% 90% at 50% 0%, #F1F1EF 0%, #E2E2DF 100%)",
    [dark],
  );

  return (
    <div
      className="flex min-h-screen w-full flex-col items-center transition-colors duration-200"
      style={{ background: frame, fontFamily: "Inter, system-ui, sans-serif" }}
    >
      <div
        className="mb-frost mt-8 flex w-[360px] flex-col overflow-hidden rounded-2xl"
        style={{
          backgroundColor: t.panel,
          border: `1px solid ${t.panelBorder}`,
          boxShadow:
            "0 24px 64px rgba(0,0,0,0.28), 0 2px 8px rgba(0,0,0,0.12)",
        }}
      >
        <div className="flex items-center gap-2 px-4 pb-3 pt-3.5">
          <span className="flex items-center gap-1.5 text-[13px] font-semibold" style={{ color: t.text }}>
            <MicroGlyph color={t.text} />
            Microbridge
          </span>
          <span
            className="ml-auto flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium"
            style={
              connected
                ? { backgroundColor: "#30C4631F", color: "#30C463" }
                : { backgroundColor: t.hoverBg, color: t.textSecondary }
            }
          >
            <span
              className="h-[6px] w-[6px] rounded-full"
              style={{
                backgroundColor: connected ? "#30C463" : t.textMuted,
                boxShadow: connected ? "0 0 5px #30C463" : "none",
              }}
            />
            {connected ? "Connected" : "Disconnected"}
          </span>
        </div>

        {focused ? (
          <>
            <div className="px-3 pb-3">
              <div
                className="rounded-xl px-3.5 py-3"
                style={{ backgroundColor: t.sunken }}
              >
                <div className="flex items-center gap-2">
                  <span
                    className="text-[11px] font-medium"
                    style={{ color: t.textSecondary }}
                  >
                    {focused.app}
                  </span>
                  <span
                    className="rounded-full px-2 py-0.5 text-[10.5px] font-medium"
                    style={{
                      backgroundColor: `${STATE_COLORS[focused.state]}22`,
                      color: STATE_COLORS[focused.state],
                    }}
                  >
                    {STATE_LABELS[focused.state]}
                  </span>
                  <span
                    className="ml-auto text-[10.5px] tabular-nums"
                    style={{ color: t.textMuted }}
                  >
                    {elapsed(focused.updated_at_ms)}
                  </span>
                </div>
                <p
                  className="mt-1.5 text-[13px] font-semibold leading-snug"
                  style={{ color: t.text }}
                >
                  {focused.title || focused.id}
                </p>
                <p className="mt-2 text-[11px]" style={{ color: t.textMuted }}>
                  Press Agent Key to focus · double-press brings window forward
                </p>
              </div>
            </div>
            <div className="flex justify-center px-3 pb-3">
              <MiniEcho t={t} snapshot={snapshot} />
            </div>
            <div
              className="px-3 pb-2"
              style={{ borderTop: `1px solid ${t.hairline}` }}
            >
              <div className="flex items-center justify-between px-2 pb-1 pt-2.5">
                <span
                  className="text-[11px] font-semibold"
                  style={{ color: t.textSecondary }}
                >
                  Threads
                </span>
                <span
                  className="text-[10.5px] tabular-nums"
                  style={{ color: t.textMuted }}
                >
                  {liveCount} on keys
                </span>
              </div>
              {snapshot.sessions.map((s) => (
                <div
                  key={s.id}
                  className="flex items-center gap-2 rounded-lg px-2 py-1.5"
                >
                  <span
                    className="h-2 w-2 rounded-full"
                    style={{ backgroundColor: STATE_COLORS[s.state] }}
                  />
                  <span
                    className="w-[72px] truncate text-[11px]"
                    style={{ color: t.textSecondary }}
                  >
                    {s.app}
                  </span>
                  <span
                    className="min-w-0 flex-1 truncate text-[12px]"
                    style={{ color: t.text }}
                  >
                    {s.title || s.id}
                  </span>
                  <span
                    className="text-[10.5px] tabular-nums"
                    style={{ color: t.textMuted }}
                  >
                    {elapsed(s.updated_at_ms)}
                  </span>
                </div>
              ))}
            </div>
          </>
        ) : (
          <div className="flex flex-col items-center px-6 pb-6 pt-2 text-center">
            <MiniEcho t={t} snapshot={snapshot} />
            <p
              className="mt-4 text-[14px] font-semibold"
              style={{ color: t.text }}
            >
              Connect your Codex Micro
            </p>
            <p
              className="mt-1 max-w-[240px] text-[12px] leading-relaxed"
              style={{ color: t.textSecondary }}
            >
              Plug in over USB-C. Quit ChatGPT desktop if it owns the LEDs.
              Agent Keys light up when a thread goes live.
            </p>
          </div>
        )}

        <div
          className="flex items-center gap-1 px-2.5 py-2"
          style={{ borderTop: `1px solid ${t.hairline}` }}
        >
          {footerButton("Settings", onOpenSettings)}
          {footerButton(
            snapshot.config.pause_leds ? "Resume LEDs" : "Pause LEDs",
            onTogglePause,
            snapshot.config.pause_leds,
          )}
          <span className="ml-auto">{footerButton("Quit", onQuit)}</span>
        </div>
      </div>
    </div>
  );
}
