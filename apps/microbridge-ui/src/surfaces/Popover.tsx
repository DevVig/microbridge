import { useEffect, useRef } from "react";
import type { Snapshot } from "../lib/types";
import { STATE_COLORS, STATE_LABELS, elapsed } from "../lib/types";
import { DARK, LIGHT, type ThemeTokens } from "../lib/theme";
import { visibleThreads } from "../lib/threads";
import { DeviceEcho } from "../components/DeviceEcho";
import { fitPopover } from "../lib/bus";

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

function StateChip({
  state,
  t,
}: {
  state: Snapshot["sessions"][0]["state"];
  t: ThemeTokens;
}) {
  const c = STATE_COLORS[state];
  const isIdle = state === "idle";
  const pulse =
    state === "awaiting_approval"
      ? "mb-led-pulse"
      : state === "thinking" || state === "working"
        ? "mb-led-breathe"
        : "";
  return (
    <span
      className="inline-flex shrink-0 items-center gap-1.5 rounded-full px-2 py-[3px] text-[11px] font-medium"
      style={{
        backgroundColor: isIdle ? t.hoverBg : `${c}1F`,
        color: isIdle
          ? t.textSecondary
          : state === "awaiting_approval" && t.name === "light"
            ? "#8A6100"
            : c,
      }}
    >
      <span
        className={`h-[6px] w-[6px] rounded-full ${pulse}`}
        style={{
          backgroundColor: isIdle ? t.textMuted : c,
          boxShadow: isIdle ? "none" : `0 0 5px ${c}`,
        }}
      />
      {STATE_LABELS[state]}
    </span>
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
  const demo = snapshot.device_name === "demo-browser";
  const simulator = snapshot.device_name === "mock" || demo;
  const detected =
    !snapshot.device_connected && snapshot.device_name.includes("usb");
  // Show the live UI shell in simulator/detected modes; only "Connected"
  // means claimed HID (not yet shipped for production hardware).
  const showLiveShell =
    snapshot.device_connected || simulator || detected;
  const chipLabel = snapshot.device_connected
    ? "Connected"
    : detected
      ? "Detected"
      : demo
        ? "Demo"
        : simulator
          ? "Simulator"
          : "Disconnected";
  const chipTone = snapshot.device_connected
    ? "ok"
    : detected || simulator
      ? "warn"
      : "off";
  const focused = snapshot.sessions.find(
    (s) => s.id === snapshot.focused_session_id,
  );
  const liveCount = snapshot.agent_key_session_ids.filter(Boolean).length;
  const { threads, total, truncated } = visibleThreads(snapshot);
  const adapterLayoutKey = snapshot.adapters
    .map((adapter) => `${adapter.id}:${adapter.state}:${adapter.diagnostic}`)
    .join("|");
  const cardRef = useRef<HTMLDivElement>(null);
  const threadListRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const card = cardRef.current;
    if (!card) return;
    const measure = () => {
      const threadList = threadListRef.current;
      const current = card.getBoundingClientRect().height;
      const desired = threadList
        ? current - threadList.clientHeight + threadList.scrollHeight + 8
        : card.scrollHeight + 8;
      void fitPopover(Math.ceil(desired));
    };
    const observer = new ResizeObserver(measure);
    observer.observe(card);
    if (threadListRef.current) observer.observe(threadListRef.current);
    measure();
    return () => observer.disconnect();
  }, [threads.length, snapshot.device_connected, snapshot.device_name, adapterLayoutKey]);

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
      onMouseEnter={(e) => {
        e.currentTarget.style.backgroundColor = t.hoverBg;
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = active ? t.hoverBg : "transparent";
      }}
    >
      {label}
    </button>
  );

  return (
    <div
      className="flex h-screen w-full items-start justify-center overflow-hidden bg-transparent pt-1"
      style={{ fontFamily: "Inter, system-ui, sans-serif" }}
    >
      <div
        ref={cardRef}
        className="mb-frost flex max-h-[calc(100vh-4px)] w-[360px] flex-col overflow-hidden rounded-2xl"
        style={{
          backgroundColor: t.panel,
          border: `1px solid ${t.panelBorder}`,
          boxShadow:
            "0 24px 64px rgba(0,0,0,0.28), 0 2px 8px rgba(0,0,0,0.12)",
        }}
      >
        <div className="flex items-center gap-2 px-4 pb-3 pt-3.5">
          <span
            className="flex items-center gap-1.5 text-[13px] font-semibold"
            style={{ color: t.text }}
          >
            <MicroGlyph color={t.text} />
            Microbridge
          </span>
          <span
            className="ml-auto flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium"
            style={
              chipTone === "ok"
                ? { backgroundColor: "#30C4631F", color: "#30C463" }
                : chipTone === "warn"
                  ? { backgroundColor: "#F5A62322", color: "#C47F00" }
                  : { backgroundColor: t.hoverBg, color: t.textSecondary }
            }
          >
            <span
              className="h-[6px] w-[6px] rounded-full"
              style={{
                backgroundColor:
                  chipTone === "ok"
                    ? "#30C463"
                    : chipTone === "warn"
                      ? "#F5A623"
                      : t.textMuted,
                boxShadow:
                  chipTone === "ok"
                    ? "0 0 5px #30C463"
                    : chipTone === "warn"
                      ? "0 0 5px #F5A623"
                      : "none",
              }}
            />
            {chipLabel}
          </span>
        </div>

        {(simulator || detected) && (
          <p
            className="px-4 pb-2 text-[11px] leading-snug"
            style={{ color: t.textMuted }}
          >
            {demo
              ? "Browser demo data — start microbridged + the Tauri app for a live bus."
              : simulator
                ? "No Micro claimed — LED frames are simulated. Enable hardware control in Device settings to connect."
                : "USB Micro seen, but hardware control is disabled or another process owns the HID interface."}
          </p>
        )}

        {showLiveShell ? (
          <>
            <div className="px-3 pb-3">
              {focused ? (
                <div
                  className="rounded-xl p-3.5"
                  style={{ backgroundColor: t.sunken }}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span
                      className="text-[11px] font-medium"
                      style={{ color: t.textSecondary }}
                    >
                      {focused.app} · owns the deck
                    </span>
                    <StateChip state={focused.state} t={t} />
                  </div>
                  <p
                    className="mt-1.5 truncate text-[14px] font-semibold"
                    style={{ color: t.text }}
                  >
                    {focused.title || focused.id}
                  </p>
                  <div className="mt-2 flex items-center gap-2">
                    <span
                      className="text-[11px] tabular-nums"
                      style={{ color: t.textMuted }}
                    >
                      {elapsed(focused.updated_at_ms)}
                    </span>
                    <span
                      className="h-[3px] w-[3px] rounded-full"
                      style={{ backgroundColor: t.textMuted }}
                    />
                    <span
                      className="ml-auto text-[10px]"
                      style={{ color: t.textMuted }}
                    >
                      press = switch · double-press = open
                    </span>
                  </div>
                </div>
              ) : (
                <div
                  className="rounded-xl p-3.5 text-[12px]"
                  style={{ backgroundColor: t.sunken, color: t.textSecondary }}
                >
                  No thread owns the deck yet. Start an agent session — Agent
                  Keys light up when a thread goes live.
                </div>
              )}
            </div>

            <div className="flex justify-center px-3 pb-3">
              <DeviceEcho t={t} snapshot={snapshot} />
            </div>

            <div
              className="flex min-h-0 flex-1 flex-col px-3 pb-2"
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
                  {truncated ? ` · ${threads.length}/${total}` : ""}
                </span>
              </div>
              {threads.length === 0 ? (
                <p
                  className="px-2 py-2 text-[12px]"
                  style={{ color: t.textMuted }}
                >
                  No live sessions
                </p>
              ) : (
                <div ref={threadListRef} className="mb-scrollbar min-h-0 overflow-y-auto">
                  {threads.map((s) => (
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
              )}
            </div>
          </>
        ) : (
          <div className="flex flex-col items-center px-6 pb-6 pt-2 text-center">
            <DeviceEcho t={t} snapshot={snapshot} />
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
              Plug in over USB-C, then enable hardware control in Device
              settings. If another app owns the HID interface, Microbridge keeps
              observing threads without claiming the deck.
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
