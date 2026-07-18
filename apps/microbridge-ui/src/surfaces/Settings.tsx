import { useEffect, useState } from "react";
import type { DaemonConfig, Snapshot } from "../lib/types";
import { STATE_COLORS, STATE_LABELS } from "../lib/types";
import { DARK, LIGHT } from "../lib/theme";
import {
  appVersion,
  autoCheckEnabled,
  runUpdateCheck,
  setAutoCheckEnabled,
  updateChannel,
  type UpdateChannel,
} from "../lib/updater";
import {
  controlInspector,
  DeviceTwin,
  type ControlId,
} from "../components/DeviceTwin";

const KEY_SOURCES: {
  id: DaemonConfig["key_source"];
  label: string;
  hint: string;
}[] = [
  {
    id: "most_recent",
    label: "Most recent",
    hint: "Cross-app — six newest threads (default)",
  },
  {
    id: "focused_app",
    label: "Focused app",
    hint: "Repopulate from whichever IDE owns the deck",
  },
  {
    id: "pinned",
    label: "Pinned",
    hint: "Follow the first six pinned sessions",
  },
  {
    id: "priority",
    label: "Priority",
    hint: "Approvals and active threads first",
  },
  {
    id: "custom",
    label: "Custom",
    hint: "Pin specific threads to specific keys",
  },
];

type Tab = "keys" | "agent" | "adapters" | "device" | "updates";

export function Settings({
  snapshot,
  dark,
  tab,
  onTab,
  onConfig,
  onClose,
}: {
  snapshot: Snapshot;
  dark: boolean;
  tab: Tab;
  onTab: (t: Tab) => void;
  onConfig: (config: DaemonConfig) => void;
  onClose: () => void;
}) {
  const t = dark ? DARK : LIGHT;
  const cfg = snapshot.config;
  const [selected, setSelected] = useState<ControlId | null>("ag1");
  const inspector = selected
    ? controlInspector(selected, snapshot)
    : null;

  const [version, setVersion] = useState<string | null>(null);
  const [channel, setChannel] = useState<UpdateChannel | null>(null);
  const [autoCheck, setAutoCheck] = useState<boolean>(() => autoCheckEnabled());

  useEffect(() => {
    void appVersion().then(setVersion);
    void updateChannel().then(setChannel);
  }, []);

  const tabs: { id: Tab; label: string }[] = [
    { id: "keys", label: "Keys" },
    { id: "agent", label: "Agent Keys" },
    { id: "adapters", label: "Adapters" },
    { id: "device", label: "Device" },
    { id: "updates", label: "Updates" },
  ];

  return (
    <div
      className="flex min-h-screen w-full"
      style={{
        background: dark ? "#0A0A0B" : "#E9E9E7",
        fontFamily: "Inter, system-ui, sans-serif",
        color: t.text,
      }}
    >
      <aside
        className="flex w-[200px] flex-col border-r px-3 py-4"
        style={{
          borderColor: t.hairline,
          backgroundColor: t.panel,
        }}
      >
        <div className="mb-4 px-2 text-[13px] font-semibold">Microbridge</div>
        {tabs.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onTab(item.id)}
            className="mb-0.5 rounded-lg px-2.5 py-2 text-left text-[12.5px] font-medium"
            style={{
              backgroundColor: tab === item.id ? t.hoverBg : "transparent",
              color: tab === item.id ? t.text : t.textSecondary,
            }}
          >
            {item.label}
          </button>
        ))}
        <button
          type="button"
          onClick={onClose}
          className="mt-auto rounded-lg px-2.5 py-2 text-left text-[12px]"
          style={{ color: t.textSecondary }}
        >
          Close
        </button>
      </aside>

      <main className="flex-1 overflow-auto p-6">
        {tab === "keys" && (
          <section className="flex flex-col gap-6 lg:flex-row">
            <div className="min-w-0 flex-1">
              <h1 className="text-[18px] font-semibold">Keys</h1>
              <p
                className="mt-1 text-[12.5px]"
                style={{ color: t.textSecondary }}
              >
                Click a control on the twin to inspect it. Agent Keys show the
                live thread; command bindings land with HID.
              </p>
              <div className="mt-5 flex justify-center lg:justify-start">
                <DeviceTwin
                  snapshot={snapshot}
                  selected={selected}
                  onSelect={setSelected}
                />
              </div>
            </div>
            <div
              className="w-full max-w-sm shrink-0 rounded-2xl p-4"
              style={{
                backgroundColor: t.panel,
                border: `1px solid ${t.hairline}`,
              }}
            >
              {inspector ? (
                <>
                  <h2 className="text-[14px] font-semibold">{inspector.title}</h2>
                  <p
                    className="mt-2 text-[12.5px] leading-relaxed"
                    style={{ color: t.textSecondary }}
                  >
                    {inspector.body}
                  </p>
                  {inspector.agent && (
                    <button
                      type="button"
                      className="mt-4 rounded-lg px-3 py-1.5 text-[12px] font-medium"
                      style={{
                        backgroundColor: t.hoverBg,
                        color: t.text,
                      }}
                      onClick={() => onTab("agent")}
                    >
                      Open Agent Keys
                    </button>
                  )}
                </>
              ) : (
                <p className="text-[12.5px]" style={{ color: t.textMuted }}>
                  Select a control on the twin
                </p>
              )}
            </div>
          </section>
        )}

        {tab === "agent" && (
          <section>
            <h1 className="text-[18px] font-semibold">Agent Keys</h1>
            <p className="mt-1 text-[12.5px]" style={{ color: t.textSecondary }}>
              Six keys, six threads. Command presses always route to the focused
              thread.
            </p>
            <div className="mt-4 grid grid-cols-3 gap-2">
              {snapshot.agent_key_session_ids.map((id, i) => {
                const s = id
                  ? snapshot.sessions.find((x) => x.id === id)
                  : null;
                return (
                  <div
                    key={i}
                    className="rounded-xl p-3"
                    style={{
                      backgroundColor: t.panel,
                      border: `1px solid ${t.hairline}`,
                    }}
                  >
                    <div className="text-[11px]" style={{ color: t.textMuted }}>
                      AG{i + 1}
                    </div>
                    {s ? (
                      <>
                        <div className="mt-1 text-[12px] font-medium">{s.app}</div>
                        <div
                          className="truncate text-[11px]"
                          style={{ color: t.textSecondary }}
                        >
                          {s.title || s.id}
                        </div>
                        <span
                          className="mt-2 inline-block rounded-full px-2 py-0.5 text-[10px] font-medium"
                          style={{
                            backgroundColor: `${STATE_COLORS[s.state]}22`,
                            color: STATE_COLORS[s.state],
                          }}
                        >
                          {STATE_LABELS[s.state]}
                        </span>
                      </>
                    ) : (
                      <div
                        className="mt-2 text-[12px]"
                        style={{ color: t.textMuted }}
                      >
                        Unassigned
                      </div>
                    )}
                  </div>
                );
              })}
            </div>

            <h2 className="mt-6 text-[13px] font-semibold">Key source</h2>
            <div className="mt-2 space-y-1.5">
              {KEY_SOURCES.map((src) => (
                <label
                  key={src.id}
                  className="flex cursor-pointer items-start gap-2 rounded-xl px-3 py-2.5"
                  style={{
                    backgroundColor:
                      cfg.key_source === src.id ? t.hoverBg : t.panel,
                    border: `1px solid ${t.hairline}`,
                  }}
                >
                  <input
                    type="radio"
                    name="key_source"
                    checked={cfg.key_source === src.id}
                    onChange={() => onConfig({ ...cfg, key_source: src.id })}
                    className="mt-1"
                  />
                  <span>
                    <span className="block text-[12.5px] font-medium">
                      {src.label}
                    </span>
                    <span
                      className="block text-[11px]"
                      style={{ color: t.textSecondary }}
                    >
                      {src.hint}
                    </span>
                  </span>
                </label>
              ))}
            </div>

            <label className="mt-4 flex items-center gap-2 text-[12.5px]">
              <input
                type="checkbox"
                checked={cfg.approvals_interrupt}
                onChange={(e) =>
                  onConfig({ ...cfg, approvals_interrupt: e.target.checked })
                }
              />
              Approvals interrupt focus
            </label>

            {cfg.frontmost_app && (
              <p
                className="mt-3 text-[11px]"
                style={{ color: t.textMuted }}
              >
                Frontmost app (live): {cfg.frontmost_app}
              </p>
            )}
          </section>
        )}

        {tab === "device" && (
          <section>
            <h1 className="text-[18px] font-semibold">Device</h1>
            <p className="mt-1 text-[12.5px]" style={{ color: t.textSecondary }}>
              Appearance, lighting, and sleep. Zero network — local socket + USB
              only.
            </p>

            <h2 className="mt-5 text-[13px] font-semibold">Appearance</h2>
            <p className="mt-1 text-[11px]" style={{ color: t.textMuted }}>
              One coherent look per mode — no toggle in the menu bar.
            </p>
            <div className="mt-2 flex gap-2">
              {(["system", "light", "dark"] as const).map((a) => (
                <button
                  key={a}
                  type="button"
                  onClick={() => onConfig({ ...cfg, appearance: a })}
                  className="rounded-lg px-3 py-1.5 text-[12px] font-medium capitalize"
                  style={{
                    backgroundColor:
                      cfg.appearance === a ? t.hoverBg : t.panel,
                    border: `1px solid ${t.hairline}`,
                    color: t.text,
                  }}
                >
                  {a}
                </button>
              ))}
            </div>

            <h2 className="mt-5 text-[13px] font-semibold">Lighting</h2>
            <div className="mt-2 flex gap-2">
              <button
                type="button"
                className="rounded-lg px-3 py-1.5 text-[12px] font-medium"
                style={{
                  backgroundColor: t.panel,
                  border: `1px solid ${t.hairline}`,
                }}
                onClick={() => onConfig({ ...cfg, lighting_preset: "codex" })}
              >
                Codex defaults
              </button>
              <button
                type="button"
                className="rounded-lg px-3 py-1.5 text-[12px] font-medium"
                style={{
                  backgroundColor: t.panel,
                  border: `1px solid ${t.hairline}`,
                }}
                onClick={() =>
                  onConfig({ ...cfg, lighting_preset: "phosphor" })
                }
              >
                Phosphor
              </button>
            </div>

            <label className="mt-5 block text-[12.5px]">
              Brightness ({cfg.brightness}%)
              <input
                type="range"
                min={0}
                max={100}
                value={cfg.brightness}
                onChange={(e) =>
                  onConfig({ ...cfg, brightness: Number(e.target.value) })
                }
                className="mt-1 block w-full max-w-sm"
              />
            </label>

            <label className="mt-4 block text-[12.5px]">
              Sleep after idle ({cfg.sleep_minutes} min)
              <input
                type="range"
                min={1}
                max={30}
                value={cfg.sleep_minutes}
                onChange={(e) =>
                  onConfig({ ...cfg, sleep_minutes: Number(e.target.value) })
                }
                className="mt-1 block w-full max-w-sm"
              />
            </label>

            <p className="mt-6 text-[11px]" style={{ color: t.textMuted }}>
              Device: {snapshot.device_name}
              {snapshot.device_connected
                ? " · connected"
                : snapshot.device_name.includes("usb")
                  ? " · USB detected (HID map pending)"
                  : snapshot.device_name === "mock"
                    ? " · simulator"
                    : " · not connected"}
              {" · "}zero network
            </p>
          </section>
        )}

        {tab === "adapters" && (
          <section>
            <h1 className="text-[18px] font-semibold">Adapters</h1>
            <p className="mt-1 text-[12.5px]" style={{ color: t.textSecondary }}>
              First-party adapters run in-process. Community adapters speak
              NDJSON on the local socket.
            </p>
            <ul className="mt-4 space-y-2">
              {[
                {
                  name: "Codex CLI",
                  kind: "Native",
                  note: "watches ~/.codex/sessions",
                },
                {
                  name: "Claude Code",
                  kind: "Native",
                  note: "watches ~/.claude/projects",
                },
                {
                  name: "Cursor",
                  kind: "Community",
                  note: "scaffold only — not production (adapters/cursor)",
                },
                {
                  name: "T3 Code",
                  kind: "Community",
                  note: "scaffold only — not production (adapters/t3code)",
                },
              ].map((a) => (
                <li
                  key={a.name}
                  className="flex items-center justify-between rounded-xl px-3 py-2.5"
                  style={{
                    backgroundColor: t.panel,
                    border: `1px solid ${t.hairline}`,
                  }}
                >
                  <div>
                    <div className="text-[12.5px] font-medium">{a.name}</div>
                    <div
                      className="text-[11px]"
                      style={{ color: t.textSecondary }}
                    >
                      {a.note}
                    </div>
                  </div>
                  <span
                    className="rounded-full px-2 py-0.5 text-[10px] font-medium"
                    style={{
                      backgroundColor: t.hoverBg,
                      color: t.textSecondary,
                    }}
                  >
                    {a.kind}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        )}

        {tab === "updates" && (
          <section>
            <h1 className="text-[18px] font-semibold">Updates</h1>
            <p className="mt-1 text-[12.5px]" style={{ color: t.textSecondary }}>
              The daemon stays zero-network. This is the only place Microbridge
              reaches out — and only when you ask it to.
            </p>

            <div
              className="mt-5 rounded-2xl p-4"
              style={{
                backgroundColor: t.panel,
                border: `1px solid ${t.hairline}`,
              }}
            >
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-[12.5px] font-medium">
                    Microbridge {version ? `v${version}` : ""}
                  </div>
                  <div
                    className="mt-0.5 text-[11px]"
                    style={{ color: t.textSecondary }}
                  >
                    {channel === "brew"
                      ? "Managed by Homebrew — brew upgrade microbridge"
                      : channel === "direct"
                        ? "Direct install — updates in place"
                        : "Checking install type…"}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => void runUpdateCheck({ silent: false })}
                  className="rounded-lg px-3 py-1.5 text-[12px] font-medium"
                  style={{
                    backgroundColor: t.hoverBg,
                    color: t.text,
                    border: `1px solid ${t.hairline}`,
                  }}
                >
                  Check for Updates
                </button>
              </div>
            </div>

            <label className="mt-4 flex items-center gap-2 text-[12.5px]">
              <input
                type="checkbox"
                checked={autoCheck}
                onChange={(e) => {
                  setAutoCheck(e.target.checked);
                  setAutoCheckEnabled(e.target.checked);
                }}
              />
              Check for updates automatically when Microbridge starts
            </label>
            <p
              className="mt-2 max-w-md text-[11px]"
              style={{ color: t.textMuted }}
            >
              Off by default. When on, Microbridge quietly checks once at launch
              and only speaks up if an update is ready. No background polling,
              ever.
            </p>
          </section>
        )}
      </main>
    </div>
  );
}
