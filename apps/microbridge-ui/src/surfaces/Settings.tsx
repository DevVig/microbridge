import { useEffect, useRef, useState } from "react";
import type { AdapterCapabilities, DaemonConfig, Snapshot, StateColors } from "../lib/types";
import {
  CODEX_PALETTE,
  PHOSPHOR_PALETTE,
  STATE_COLORS,
  STATE_LABELS,
} from "../lib/types";
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
import { forgetAdapter, pairAdapter, setAdapterEnabled } from "../lib/bus";
import {
  canLaunchAtLogin,
  launchAtLoginEnabled,
  setLaunchAtLogin,
} from "../lib/autostart";
import {
  IntegrationCard,
  IntegrationDetail,
} from "../components/IntegrationCard";
import {
  TRAFFIC_COLORS,
  integrationView,
  isHostAttributed,
} from "../lib/hosts";
import { integrationIcon } from "../lib/integrationIcons";
import { openHostApp, openableHostApp } from "../lib/openHostApp";
import { integrationGuidance } from "../lib/integrationSetup";
import { MeshBackground } from "../components/MeshBackground";

const LIGHTING_STATES: { id: keyof StateColors; label: string }[] = [
  { id: "idle", label: "Idle" },
  { id: "thinking", label: "Thinking" },
  { id: "working", label: "Working" },
  { id: "awaiting_approval", label: "Needs approval" },
  { id: "done", label: "Complete" },
  { id: "error", label: "Error" },
];

const CAPABILITIES: { id: keyof AdapterCapabilities; label: string }[] = [
  { id: "lifecycle_observation", label: "Live state" },
  { id: "approval_acceptance", label: "Approve" },
  { id: "approval_rejection", label: "Reject" },
  { id: "interrupt", label: "Interrupt" },
  { id: "new_session", label: "New session" },
  { id: "focus_open", label: "Open" },
  { id: "reasoning_effort", label: "Effort" },
];

const INTEGRATION_ORDER = [
  "chatgpt",
  "claude_desktop",
  "claude",
  "codex",
  "cnvs",
  "synara",
  "conductor",
  "cursor",
  "t3code",
  "factory",
  "opencode",
];

const KEY_SOURCES: {
  id: DaemonConfig["key_source"];
  label: string;
  hint: string;
}[] = [
  {
    id: "focused_app",
    label: "Focused app",
    hint: "Owning app — newest threads (ChatGPT, Claude, Codex, CNVS, Cursor, Synara, T3, Conductor, Factory, OpenCode)",
  },
  {
    id: "most_recent",
    label: "Most recent",
    hint: "Cross-app — six newest threads",
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

type Tab = "general" | "keys" | "agent" | "integrations" | "device" | "updates";

export function Settings({
  snapshot,
  dark,
  tab,
  onTab,
  onConfig,
  onClose,
  onAgentKey,
}: {
  snapshot: Snapshot;
  dark: boolean;
  tab: Tab;
  onTab: (t: Tab) => void;
  onConfig: (config: DaemonConfig) => void;
  onClose: () => void;
  onAgentKey?: (index: number, open: boolean) => void;
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
  const [pairingUrl, setPairingUrl] = useState("");
  const [adapterMessage, setAdapterMessage] = useState<string | null>(null);
  const [adapterBusy, setAdapterBusy] = useState<Set<string>>(() => new Set());
  const [selectedIntegration, setSelectedIntegration] = useState<string | null>(
    null,
  );
  const pairingInputRef = useRef<HTMLInputElement>(null);
  const integrationDetailRef = useRef<HTMLDivElement>(null);
  // null until the login item has been read, and permanently null where a login
  // item is meaningless: outside Tauri, or in a dev build whose executable path
  // points into `target/debug`.
  const [atLogin, setAtLogin] = useState<boolean | null>(null);

  const selectIntegration = (adapterId: string) => {
    setSelectedIntegration(adapterId);
    requestAnimationFrame(() => {
      integrationDetailRef.current?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
      });
    });
  };

  useEffect(() => {
    void appVersion().then(setVersion);
    void updateChannel().then(setChannel);
    void canLaunchAtLogin().then(async (supported) => {
      if (supported) setAtLogin(await launchAtLoginEnabled());
    });
  }, []);

  const runAdapterOperation = async (adapterId: string, work: () => Promise<string>) => {
    setAdapterBusy((current) => new Set(current).add(adapterId));
    setAdapterMessage(null);
    try {
      setAdapterMessage(await work());
    } catch (error) {
      setAdapterMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setAdapterBusy((current) => {
        const next = new Set(current);
        next.delete(adapterId);
        return next;
      });
    }
  };

  // Write first, then adopt what the system actually reports — a failed write
  // must not leave the checkbox claiming something that isn't true.
  const toggleAtLogin = async (next: boolean) => {
    setAtLogin(next);
    try {
      await setLaunchAtLogin(next);
    } catch {
      /* fall through to the re-read below */
    }
    setAtLogin(await launchAtLoginEnabled());
  };

  const tabs: { id: Tab; label: string }[] = [
    { id: "general", label: "General" },
    { id: "keys", label: "Keys" },
    { id: "agent", label: "Agent Keys" },
    { id: "integrations", label: "Integrations" },
    { id: "device", label: "Device" },
    { id: "updates", label: "Updates" },
  ];

  return (
    <div
      className="relative flex min-h-screen w-full"
      style={{
        background: dark ? "#0A0A0B" : "#E9E9E7",
        fontFamily: "Inter, system-ui, sans-serif",
        color: t.text,
      }}
    >
      <MeshBackground dark={dark} active />
      <aside
        className="relative z-10 flex w-[200px] flex-col border-r px-3 py-4"
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

      <main className="relative z-10 flex-1 overflow-auto p-6">
        {tab === "general" && (
          <section>
            <h1 className="text-[18px] font-semibold">General</h1>
            <p className="mt-1 text-[12.5px]" style={{ color: t.textSecondary }}>
              Microbridge lives in the menu bar — it has to be running to drive
              your deck.
            </p>

            <label
              className="mt-5 flex max-w-md cursor-pointer items-start gap-2.5 rounded-2xl p-4"
              style={{
                backgroundColor: t.panel,
                border: `1px solid ${t.hairline}`,
              }}
            >
              <input
                type="checkbox"
                className="mt-0.5"
                checked={atLogin === true}
                disabled={atLogin === null}
                onChange={(e) => void toggleAtLogin(e.target.checked)}
              />
              <span>
                <span className="block text-[12.5px] font-medium">
                  Launch Microbridge at login
                </span>
                <span
                  className="mt-0.5 block text-[11px] leading-relaxed"
                  style={{ color: t.textSecondary }}
                >
                  {atLogin === null
                    ? "Only available in the installed app."
                    : "Adds a login item so the menu bar icon comes back after a restart. Takes effect at your next login."}
                </span>
              </span>
            </label>
          </section>
        )}

        {tab === "keys" && (
          <section className="flex flex-col gap-6 lg:flex-row">
            <div className="min-w-0 flex-1">
              <h1 className="text-[18px] font-semibold">Keys</h1>
              <p
                className="mt-1 text-[12.5px]"
                style={{ color: t.textSecondary }}
              >
                Click a control on the twin to inspect it. Agent Keys show the
                live thread; commands route only when hardware control is enabled
                and the focused integration advertises the action.
              </p>
              <div className="mt-5 flex justify-center lg:justify-start">
                <DeviceTwin
                  snapshot={snapshot}
                  selected={selected}
                  onSelect={setSelected}
                  onAgentKey={onAgentKey}
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
                  <button
                    type="button"
                    key={i}
                    disabled={!s}
                    onClick={() => onAgentKey?.(i, false)}
                    onDoubleClick={() => onAgentKey?.(i, true)}
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
                  </button>
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
              Appearance, lighting, and sleep. Device control stays on this Mac.
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
            <p className="mt-1 max-w-2xl text-[11px] leading-relaxed" style={{ color: t.textMuted }}>
              Lighting maps agent lifecycle states to the Agent Key LEDs. Codex Defaults is the
              recommended palette, Phosphor is a warmer alternate, and Custom lets you choose each
              state color. Changes apply immediately and persist on this Mac.
            </p>
            <div className="mt-3 grid max-w-2xl grid-cols-3 gap-2">
              {[
                { id: "codex" as const, label: "Codex Defaults", palette: CODEX_PALETTE },
                { id: "phosphor" as const, label: "Phosphor", palette: PHOSPHOR_PALETTE },
                { id: "custom" as const, label: "Custom", palette: cfg.state_colors },
              ].map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  className="rounded-xl p-3 text-left text-[12px] font-medium"
                  style={{
                    backgroundColor: cfg.lighting_preset === preset.id ? t.hoverBg : t.panel,
                    border: `1px solid ${cfg.lighting_preset === preset.id ? t.textSecondary : t.hairline}`,
                  }}
                  onClick={() => onConfig({ ...cfg, lighting_preset: preset.id })}
                >
                  <span className="block">{preset.label}</span>
                  <span className="mt-2 flex gap-1">
                    {Object.values(preset.palette).map((color, index) => (
                      <span key={`${color}-${index}`} className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: color }} />
                    ))}
                  </span>
                </button>
              ))}
            </div>

            {cfg.lighting_preset === "custom" && (
              <div className="mt-3 grid max-w-2xl grid-cols-2 gap-2 sm:grid-cols-3">
                {LIGHTING_STATES.map((state) => (
                  <label
                    key={state.id}
                    className="flex items-center gap-2 rounded-xl px-3 py-2 text-[11px]"
                    style={{ backgroundColor: t.panel, border: `1px solid ${t.hairline}` }}
                  >
                    <input
                      type="color"
                      value={cfg.state_colors[state.id]}
                      onChange={(event) =>
                        onConfig({
                          ...cfg,
                          lighting_preset: "custom",
                          state_colors: { ...cfg.state_colors, [state.id]: event.target.value.toUpperCase() },
                        })
                      }
                      className="h-6 w-8 border-0 bg-transparent"
                    />
                    {state.label}
                  </label>
                ))}
              </div>
            )}
            <button
              type="button"
              className="mt-3 rounded-lg px-3 py-1.5 text-[11px] font-medium"
              style={{ backgroundColor: t.panel, border: `1px solid ${t.hairline}` }}
              onClick={() =>
                onConfig({ ...cfg, lighting_preset: "codex", state_colors: CODEX_PALETTE })
              }
            >
              Reset to Codex Defaults
            </button>

            <h2 className="mt-6 text-[13px] font-semibold">Hardware control</h2>
            <label className="mt-2 flex max-w-2xl items-start gap-2 text-[12.5px]">
              <input
                type="checkbox"
                checked={cfg.hardware_control_enabled}
                onChange={(event) =>
                  onConfig({ ...cfg, hardware_control_enabled: event.target.checked })
                }
                className="mt-0.5"
              />
              <span>
                Claim the Codex Micro for keys, dial, joystick, and lighting
                <span className="mt-0.5 block text-[11px]" style={{ color: t.textMuted }}>
                  Off by default to avoid competing with another device owner. Changes apply
                  immediately.
                </span>
              </span>
            </label>

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
                  ? " · USB detected; hardware control disabled or interface busy"
                  : snapshot.device_name === "mock"
                    ? " · simulator"
                    : " · not connected"}
              {" · "}local USB control
            </p>
          </section>
        )}

        {tab === "integrations" && (() => {
          const views = [...snapshot.adapters]
            .sort(
              (left, right) =>
                INTEGRATION_ORDER.indexOf(left.id) -
                INTEGRATION_ORDER.indexOf(right.id),
            )
            .map((adapter) => ({
              adapter,
              view: integrationView(adapter, snapshot.sessions, {
                enabled: cfg.adapters[adapter.id]?.enabled,
              }),
              optIn:
                adapter.kind === "community" && !isHostAttributed(adapter.id),
            }));
          const connected = views.filter((item) => item.view.connectedGroup);
          const notConnected = views.filter((item) => !item.view.connectedGroup);
          const ordered = [
            {
              label: "Connected",
              light: "green" as const,
              items: connected,
            },
            {
              label: "Not connected",
              light: "yellow" as const,
              items: notConnected,
            },
          ];
          const activeId = selectedIntegration;
          const selected = views.find((item) => item.adapter.id === activeId);
          const guidance = selected
            ? integrationGuidance(selected.adapter.id, selected.adapter.state, {
                enabled: cfg.adapters[selected.adapter.id]?.enabled,
                label: selected.view.label,
              })
            : null;
          const openAppLabel = selected
            ? openableHostApp(selected.adapter.id)
            : null;
          return (
          <section>
            <h1 className="text-[18px] font-semibold">Integrations</h1>
            <p className="mt-1 text-[12.5px]" style={{ color: t.textSecondary }}>
              Click any tile for details and next steps. Cursor, Factory, T3
              Code, and OpenCode install on first click when they&apos;re off;
              they turn green or Limited only after that host talks to
              Microbridge (reload / restart / pair).
            </p>
            <p className="mt-2 text-[11px]" style={{ color: t.textMuted }}>
              Synara and the desktop apps share Claude/Codex journals — Idle
              means waiting for sessions, not a broken setup. For T3 controls,
              enable Network access in T3 Code Settings → Connections, then
              paste a pairing link below.
            </p>
            {adapterMessage && (
              <p className="mt-3 rounded-lg px-3 py-2 text-[11px]" style={{ backgroundColor: t.hoverBg }}>
                {adapterMessage}
              </p>
            )}
            <div className="mt-5 grid grid-cols-3 gap-1.5 sm:grid-cols-4 lg:grid-cols-5">
              {ordered.map((group) => (
                <div key={group.label} className="contents">
                  <div
                    className="col-span-3 mt-1 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.08em] first:mt-0 sm:col-span-4 lg:col-span-5"
                    style={{ color: t.textSecondary }}
                  >
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{
                        backgroundColor: group.items.length
                          ? TRAFFIC_COLORS[group.light].dot
                          : t.textMuted,
                      }}
                    />
                    {group.label} · {group.items.length}
                  </div>
                  {group.items.map(({ adapter, view, optIn }) => (
                    <IntegrationCard
                      key={adapter.id}
                      name={adapter.display_name}
                      iconSrc={integrationIcon(adapter.id)}
                      diagnostic={view.diagnostic}
                      light={view.light}
                      label={view.label}
                      theme={t}
                      expandable={optIn}
                      expanded={adapter.id === activeId}
                      busy={adapterBusy.has(adapter.id)}
                      onSelect={() => {
                        selectIntegration(adapter.id);
                        if (
                          optIn &&
                          (adapter.state === "disabled" ||
                            (adapter.state === "needs_setup" &&
                              cfg.adapters[adapter.id]?.enabled === false))
                        ) {
                          requestAnimationFrame(() => {
                            void runAdapterOperation(adapter.id, () =>
                              setAdapterEnabled(adapter.id, true),
                            );
                          });
                          return;
                        }
                        if (
                          adapter.id === "t3code" &&
                          adapter.state === "needs_setup"
                        ) {
                          queueMicrotask(() =>
                            pairingInputRef.current?.focus(),
                          );
                        }
                      }}
                    />
                  ))}
                </div>
              ))}
            </div>
            {!selected && (
              <p className="mt-3 text-[11px]" style={{ color: t.textMuted }}>
                Select a tile for details.
              </p>
            )}
            {selected && (
              <IntegrationDetail
                ref={integrationDetailRef}
                name={selected.adapter.display_name}
                iconSrc={integrationIcon(selected.adapter.id)}
                diagnostic={selected.view.diagnostic}
                theme={t}
                guidance={
                  guidance
                    ? { title: guidance.title, steps: guidance.steps }
                    : null
                }
              >
                  <div className="mt-2 flex flex-wrap gap-1">
                    {CAPABILITIES.map((capability) => (
                      <span
                        key={capability.id}
                        className="rounded-full px-2 py-0.5 text-[9.5px]"
                        style={{
                          backgroundColor: selected.adapter.capabilities[capability.id] ? "#30C46316" : t.hoverBg,
                          color: selected.adapter.capabilities[capability.id] ? "#278B48" : t.textMuted,
                        }}
                      >
                        {selected.adapter.capabilities[capability.id] ? "✓ " : "— "}{capability.label}
                      </span>
                    ))}
                  </div>
                  {selected.adapter.id === "t3code" && selected.adapter.state !== "disabled" && (
                    <div className="mt-3 flex max-w-xl gap-2">
                      <input
                        ref={pairingInputRef}
                        type="password"
                        value={pairingUrl}
                        onChange={(event) => setPairingUrl(event.target.value)}
                        placeholder="Paste one-time T3 Code pairing link"
                        className="min-w-0 flex-1 rounded-lg px-3 py-1.5 text-[11px]"
                        style={{ backgroundColor: t.sunken, border: `1px solid ${t.hairline}` }}
                      />
                      <button
                        type="button"
                        disabled={!pairingUrl.trim() || adapterBusy.has(selected.adapter.id)}
                        className="rounded-lg px-3 py-1.5 text-[11px] font-medium disabled:opacity-40"
                        style={{ backgroundColor: t.hoverBg, border: `1px solid ${t.hairline}` }}
                        onClick={() =>
                          void runAdapterOperation(selected.adapter.id, async () => {
                            const message = await pairAdapter(selected.adapter.id, pairingUrl.trim());
                            setPairingUrl("");
                            return message;
                          })
                        }
                      >
                        Pair
                      </button>
                    </div>
                  )}
                  <div className="mt-3 flex flex-wrap gap-2">
                    {openAppLabel && (
                      <button
                        type="button"
                        className="rounded-lg px-3 py-1.5 text-[11px] font-medium"
                        style={{ backgroundColor: t.hoverBg, border: `1px solid ${t.hairline}` }}
                        onClick={() => void openHostApp(selected.adapter.id)}
                      >
                        Open {openAppLabel}
                      </button>
                    )}
                    {selected.adapter.kind === "community" && !cfg.adapters[selected.adapter.id]?.enabled && (
                      <button
                        type="button"
                        disabled={adapterBusy.has(selected.adapter.id)}
                        className="rounded-lg px-3 py-1.5 text-[11px] font-medium"
                        style={{ backgroundColor: t.hoverBg, border: `1px solid ${t.hairline}` }}
                        onClick={() =>
                          void runAdapterOperation(selected.adapter.id, () => setAdapterEnabled(selected.adapter.id, true))
                        }
                      >
                        {selected.adapter.id === "cursor"
                          ? "Enable Cursor"
                          : selected.adapter.id === "factory"
                            ? "Enable Factory"
                            : selected.adapter.id === "opencode"
                              ? "Enable OpenCode"
                            : "Enable integration"}
                      </button>
                    )}
                    {selected.adapter.kind === "community" && cfg.adapters[selected.adapter.id]?.enabled && (
                      <>
                        {(selected.adapter.id === "cursor" || selected.adapter.id === "factory" || selected.adapter.id === "opencode") && (
                          <button
                            type="button"
                            disabled={adapterBusy.has(selected.adapter.id)}
                            className="rounded-lg px-3 py-1.5 text-[11px] font-medium"
                            style={{ backgroundColor: t.hoverBg, border: `1px solid ${t.hairline}` }}
                            onClick={() =>
                              void runAdapterOperation(selected.adapter.id, () => setAdapterEnabled(selected.adapter.id, true))
                            }
                          >
                            Repair bundled integration
                          </button>
                        )}
                        <button
                          type="button"
                          disabled={adapterBusy.has(selected.adapter.id)}
                          className="rounded-lg px-3 py-1.5 text-[11px]"
                          style={{ border: `1px solid ${t.hairline}`, color: t.textSecondary }}
                          onClick={() =>
                            void runAdapterOperation(selected.adapter.id, () => setAdapterEnabled(selected.adapter.id, false))
                          }
                        >
                          Disconnect
                        </button>
                        <button
                          type="button"
                          disabled={adapterBusy.has(selected.adapter.id)}
                          className="rounded-lg px-3 py-1.5 text-[11px]"
                          style={{ border: `1px solid ${t.hairline}`, color: t.textSecondary }}
                          onClick={() =>
                            void runAdapterOperation(selected.adapter.id, () => forgetAdapter(selected.adapter.id))
                          }
                        >
                          Remove
                        </button>
                      </>
                    )}
                  </div>
              </IntegrationDetail>
            )}
          </section>
          );
        })()}

        {tab === "updates" && (
          <section>
            <h1 className="text-[18px] font-semibold">Updates</h1>
            <p className="mt-1 text-[12.5px]" style={{ color: t.textSecondary }}>
              Update checks run only when requested or explicitly enabled. A
              paired T3 Code integration contacts only its approved environment.
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
              Check for updates automatically (at most once a day)
            </label>
            <p
              className="mt-2 max-w-md text-[11px]"
              style={{ color: t.textMuted }}
            >
              Off by default. When on, Microbridge quietly checks at launch only
              if 24 hours have passed, and speaks up only when an update is ready.
              No background polling, ever.
            </p>
          </section>
        )}
      </main>
    </div>
  );
}
