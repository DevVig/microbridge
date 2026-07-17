import type { CSSProperties } from "react";
import type { AgentState, Snapshot } from "../lib/types";
import { STATE_COLORS, STATE_LABELS } from "../lib/types";

/** Control ids on the kbd-1.0 twin (MagicPath DeviceKeys layout). */
export type ControlId =
  | "knob"
  | "ag1"
  | "ag2"
  | "ag3"
  | "ag4"
  | "ag5"
  | "ag6"
  | "joystick"
  | "fast"
  | "approve"
  | "reject"
  | "fork"
  | "touch"
  | "mic"
  | "codex";

const U = 58;
const GAP = 12;
const SELECTION = "#3D7EFF";

const LABELS: Record<ControlId, string> = {
  knob: "Dial · reasoning effort",
  ag1: "Agent Key 1",
  ag2: "Agent Key 2",
  ag3: "Agent Key 3",
  ag4: "Agent Key 4",
  ag5: "Agent Key 5",
  ag6: "Agent Key 6",
  joystick: "Joystick · skill flicks",
  fast: "Fast",
  approve: "Approve",
  reject: "Reject",
  fork: "Fork",
  touch: "Touch sensor",
  mic: "Mic (2U)",
  codex: "Codex · new chat",
};

const AG_INDEX: Partial<Record<ControlId, number>> = {
  ag1: 0,
  ag2: 1,
  ag3: 2,
  ag4: 3,
  ag5: 4,
  ag6: 5,
};

function selectableStyle(selected: boolean): CSSProperties {
  return selected
    ? { boxShadow: `0 0 0 2px #FFFFFF, 0 0 0 4px ${SELECTION}` }
    : {};
}

function CapIcon({ icon, size = 20 }: { icon: string; size?: number }) {
  const s = { width: size, height: size };
  const common = {
    fill: "none" as const,
    stroke: "currentColor",
    strokeWidth: 1.7,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
  };
  switch (icon) {
    case "bolt":
      return (
        <svg {...s} viewBox="0 0 24 24" {...common} aria-hidden="true">
          <path d="M13 2.5 5.5 13.5h5.4l-1.2 8 7.8-11h-5.6l1.1-8z" />
        </svg>
      );
    case "check":
      return (
        <svg {...s} viewBox="0 0 24 24" {...common} aria-hidden="true">
          <circle cx="12" cy="12" r="8.6" />
          <path d="m8.4 12.4 2.4 2.4 4.8-5.2" />
        </svg>
      );
    case "cross":
      return (
        <svg {...s} viewBox="0 0 24 24" {...common} aria-hidden="true">
          <circle cx="12" cy="12" r="8.6" />
          <path d="m9.3 9.3 5.4 5.4m0-5.4-5.4 5.4" />
        </svg>
      );
    case "fork":
      return (
        <svg {...s} viewBox="0 0 24 24" {...common} aria-hidden="true">
          <path d="M4.5 19.5v-6c0-3 2-5 5-5h9" />
          <path d="m14.5 4.5 4 4-4 4" />
        </svg>
      );
    case "mic":
      return (
        <svg {...s} viewBox="0 0 24 24" {...common} aria-hidden="true">
          <path d="M12 3a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0V6a3 3 0 0 1 3-3z" />
          <path d="M6.5 11a5.5 5.5 0 0 0 11 0M12 16.5V20" />
        </svg>
      );
    case "codex":
      return (
        <svg {...s} viewBox="0 0 24 24" {...common} aria-hidden="true">
          <path d="M17.8 17.5H8.2a4.1 4.1 0 0 1-1.1-8.05A5.6 5.6 0 0 1 17.9 8a4.8 4.8 0 0 1-.1 9.5z" />
          <path d="m10.2 11.4 1.8 1.6-1.8 1.6M13.6 14.6h1.9" />
        </svg>
      );
    default:
      return null;
  }
}

function AgentKeycap({
  id,
  selected,
  onSelect,
  snapshot,
}: {
  id: ControlId;
  selected: boolean;
  onSelect: () => void;
  snapshot: Snapshot;
}) {
  const index = AG_INDEX[id] ?? 0;
  const sid = snapshot.agent_key_session_ids[index];
  const session = sid ? snapshot.sessions.find((s) => s.id === sid) : null;
  const lit = session != null;
  const color = session ? STATE_COLORS[session.state as AgentState] : "transparent";
  const focused = sid != null && sid === snapshot.focused_session_id;
  const pulse =
    session?.state === "awaiting_approval"
      ? "mb-led-pulse"
      : session?.state === "thinking"
        ? "mb-led-breathe"
        : "";

  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      title={LABELS[id]}
      className="relative rounded-[13px] transition-transform duration-150 hover:scale-[1.04]"
      style={{
        width: U,
        height: U,
        background:
          "linear-gradient(180deg, rgba(255,255,255,0.62), rgba(238,238,235,0.55))",
        border: "1px solid rgba(0,0,0,0.11)",
        boxShadow: `inset 0 1px 0 rgba(255,255,255,0.9), inset 0 -3px 5px rgba(0,0,0,0.07), 0 1px 2px rgba(0,0,0,0.12)${lit ? `, 0 0 16px 1px ${color}66` : ""}`,
        ...selectableStyle(selected),
      }}
    >
      {lit && (
        <span
          className={`absolute inset-[3px] rounded-[10px] ${pulse}`}
          style={{
            background: `radial-gradient(circle at 50% 55%, ${color}${focused ? "E6" : "99"} 0%, ${color}33 55%, transparent 78%)`,
          }}
        />
      )}
      <span
        className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2"
        style={{ filter: "blur(0.4px)" }}
      >
        <span
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-[1.5px]"
          style={{
            width: 15,
            height: 4.5,
            backgroundColor: lit ? "rgba(50,50,64,0.42)" : "rgba(70,70,82,0.30)",
          }}
        />
        <span
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-[1.5px]"
          style={{
            width: 4.5,
            height: 15,
            backgroundColor: lit ? "rgba(50,50,64,0.42)" : "rgba(70,70,82,0.30)",
          }}
        />
      </span>
    </button>
  );
}

function CommandKeycap({
  icon,
  selected,
  onSelect,
  title,
  wide = false,
}: {
  icon: string;
  selected: boolean;
  onSelect: () => void;
  title: string;
  wide?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      title={title}
      className="relative flex items-center justify-center rounded-[13px] text-[#1D1D1B] transition-transform duration-150 hover:scale-[1.04]"
      style={{
        width: wide ? U * 2 + GAP : U,
        height: U,
        background:
          "radial-gradient(ellipse 90% 70% at 50% 38%, #FFFFFF 0%, #F6F6F3 55%, #ECECE8 100%)",
        border: "1px solid rgba(0,0,0,0.09)",
        boxShadow:
          "inset 0 1px 0 rgba(255,255,255,1), inset 0 -3px 5px rgba(0,0,0,0.08), 0 1.5px 3px rgba(0,0,0,0.14)",
        ...selectableStyle(selected),
      }}
    >
      <CapIcon icon={icon} />
    </button>
  );
}

function Dial({
  selected,
  onSelect,
  title,
}: {
  selected: boolean;
  onSelect: () => void;
  title: string;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      title={title}
      className="relative flex items-center justify-center transition-transform duration-150 hover:scale-[1.04]"
      style={{ width: U, height: U }}
    >
      <span
        className="absolute rounded-full"
        style={{
          width: U,
          height: U,
          background:
            "radial-gradient(circle at 34% 28%, #FFFFFF 0%, #EDEDEA 45%, #D9D9D5 100%)",
          border: "1px solid rgba(0,0,0,0.13)",
          boxShadow:
            "inset 0 1px 0 rgba(255,255,255,1), 0 3px 6px rgba(0,0,0,0.18)",
          ...selectableStyle(selected),
        }}
      />
      <span
        className="absolute rounded-full"
        style={{
          width: 3.5,
          height: 20,
          left: "50%",
          top: 5,
          transform: "translateX(-50%) rotate(38deg)",
          transformOrigin: "50% 130%",
          backgroundColor: "rgba(0,0,0,0.18)",
        }}
      />
    </button>
  );
}

function Joystick({
  selected,
  onSelect,
  title,
}: {
  selected: boolean;
  onSelect: () => void;
  title: string;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      title={title}
      className="relative flex items-center justify-center rounded-[14px] transition-transform duration-150 hover:scale-[1.04]"
      style={{
        width: U,
        height: U,
        border: "1.5px dashed rgba(0,0,0,0.28)",
        ...selectableStyle(selected),
      }}
    >
      <span
        className="relative flex items-center justify-center rounded-full"
        style={{
          width: 46,
          height: 46,
          background:
            "radial-gradient(circle at 35% 28%, #3E3E44 0%, #202024 55%, #101013 100%)",
          boxShadow:
            "inset 0 1px 1px rgba(255,255,255,0.18), 0 3px 6px rgba(0,0,0,0.35)",
        }}
      >
        <svg
          width="22"
          height="22"
          viewBox="0 0 24 24"
          fill="none"
          stroke="#8B8B92"
          strokeWidth="1.8"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M12 4.5 12 7M12 17v2.5M4.5 12H7m10 0h2.5" />
          <path d="m10 5.5 2-2 2 2M10 18.5l2 2 2-2M5.5 10l-2 2 2 2M18.5 10l2 2-2 2" />
        </svg>
      </span>
    </button>
  );
}

function TouchSensor({
  selected,
  onSelect,
  title,
}: {
  selected: boolean;
  onSelect: () => void;
  title: string;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      title={title}
      className="relative flex items-center transition-transform duration-150 hover:scale-[1.04]"
      style={{ width: U, height: U }}
    >
      <span className="absolute left-[2px] top-1/2 flex -translate-y-1/2 flex-col gap-[5px]">
        <span
          className="h-[4px] w-[6px] rounded-[1px]"
          style={{ backgroundColor: "#D8D8D4" }}
        />
        <span
          className="h-[4px] w-[6px] rounded-[1px]"
          style={{ backgroundColor: "#F4D06A", boxShadow: "0 0 4px #F4D06A" }}
        />
        <span
          className="h-[4px] w-[6px] rounded-[1px]"
          style={{ backgroundColor: "#8FE3A6", boxShadow: "0 0 4px #8FE3A6" }}
        />
      </span>
      <span
        className="ml-[16px] rounded-full"
        style={{
          width: 32,
          height: 32,
          background:
            "radial-gradient(circle at 35% 28%, #2E2E33 0%, #151518 60%, #0B0B0D 100%)",
          boxShadow:
            "inset 0 1px 1px rgba(255,255,255,0.15), 0 1px 3px rgba(0,0,0,0.3)",
          ...selectableStyle(selected),
        }}
      />
    </button>
  );
}

function Screw() {
  return (
    <span
      className="absolute rounded-full"
      style={{
        width: 11,
        height: 11,
        background:
          "radial-gradient(circle at 35% 30%, #4A4A50 0%, #232327 60%, #111114 100%)",
        boxShadow: "inset 0 1px 1px rgba(255,255,255,0.2)",
      }}
    >
      <span className="absolute left-1/2 top-1/2 h-[5px] w-[5px] -translate-x-1/2 -translate-y-1/2 rounded-[1px] bg-[#0A0A0C]" />
    </span>
  );
}

export function DeviceTwin({
  snapshot,
  selected,
  onSelect,
}: {
  snapshot: Snapshot;
  selected: ControlId | null;
  onSelect: (id: ControlId) => void;
}) {
  const common = (id: ControlId) => ({
    selected: selected === id,
    onSelect: () => onSelect(id),
    title: LABELS[id],
  });

  return (
    <div
      className="relative rounded-[30px] p-[15px]"
      style={{
        background:
          "linear-gradient(180deg, rgba(226,226,224,0.65), rgba(206,206,204,0.55))",
        boxShadow:
          "0 18px 40px rgba(0,0,0,0.22), inset 0 1px 0 rgba(255,255,255,0.55)",
      }}
    >
      <div
        className="relative rounded-[20px] px-[26px] pb-[30px] pt-[26px]"
        style={{
          background: "linear-gradient(180deg, #FBFBF9 0%, #F2F2EF 100%)",
          border: "1px solid rgba(0,0,0,0.07)",
          boxShadow:
            "inset 0 1px 0 rgba(255,255,255,0.95), 0 1px 3px rgba(0,0,0,0.1)",
        }}
      >
        <span className="absolute left-[9px] top-[9px]">
          <Screw />
        </span>
        <span className="absolute right-[20px] top-[9px]">
          <Screw />
        </span>
        <span className="absolute bottom-[20px] left-[9px]">
          <Screw />
        </span>
        <span className="absolute bottom-[20px] right-[20px]">
          <Screw />
        </span>

        <div className="flex flex-col" style={{ gap: GAP }}>
          <div className="flex" style={{ gap: GAP }}>
            <Dial {...common("knob")} />
            <AgentKeycap id="ag1" snapshot={snapshot} {...common("ag1")} />
            <AgentKeycap id="ag2" snapshot={snapshot} {...common("ag2")} />
            <Joystick {...common("joystick")} />
          </div>
          <div className="flex" style={{ gap: GAP }}>
            <AgentKeycap id="ag3" snapshot={snapshot} {...common("ag3")} />
            <AgentKeycap id="ag4" snapshot={snapshot} {...common("ag4")} />
            <AgentKeycap id="ag5" snapshot={snapshot} {...common("ag5")} />
            <AgentKeycap id="ag6" snapshot={snapshot} {...common("ag6")} />
          </div>
          <div className="flex" style={{ gap: GAP }}>
            <CommandKeycap icon="bolt" {...common("fast")} />
            <CommandKeycap icon="check" {...common("approve")} />
            <CommandKeycap icon="cross" {...common("reject")} />
            <CommandKeycap icon="fork" {...common("fork")} />
          </div>
          <div className="flex" style={{ gap: GAP }}>
            <TouchSensor {...common("touch")} />
            <CommandKeycap icon="mic" wide {...common("mic")} />
            <CommandKeycap icon="codex" {...common("codex")} />
          </div>
        </div>
      </div>
    </div>
  );
}

export function controlInspector(
  id: ControlId,
  snapshot: Snapshot,
): { title: string; body: string; agent?: boolean } {
  const ag = AG_INDEX[id];
  if (ag != null) {
    const sid = snapshot.agent_key_session_ids[ag];
    const session = sid ? snapshot.sessions.find((s) => s.id === sid) : null;
    if (!session) {
      return {
        title: LABELS[id],
        body: "Unassigned — configure key source under Agent Keys. Agent Keys are never blank remappable actions.",
        agent: true,
      };
    }
    return {
      title: LABELS[id],
      body: `${session.app} · ${session.title || session.id} · ${STATE_LABELS[session.state]}. Configure assignment under Agent Keys.`,
      agent: true,
    };
  }
  if (id === "knob") {
    return {
      title: LABELS[id],
      body: "Rotate sets reasoning effort on the focused thread. Physical listen-mode remapping lands with HID (M2).",
    };
  }
  if (id === "joystick") {
    return {
      title: LABELS[id],
      body: "Four flick directions map to skills (review / debug / refactor / explain). Binding UI lands with HID.",
    };
  }
  return {
    title: LABELS[id],
    body: "Command key — actions always route to the daemon-resolved focused thread. Cap remapping / listen mode lands with HID (M2).",
  };
}
