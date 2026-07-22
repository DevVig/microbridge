import { forwardRef, type ReactNode } from "react";
import type { ThemeTokens } from "../lib/theme";
import {
  TRAFFIC_COLORS,
  type TrafficLight,
} from "../lib/hosts";

function TileFace({
  name,
  iconSrc,
  diagnostic,
  light,
  label,
  theme,
  busy,
}: {
  name: string;
  iconSrc?: string;
  diagnostic: string;
  light: TrafficLight;
  label: string;
  theme: ThemeTokens;
  busy?: boolean;
}) {
  const colors = TRAFFIC_COLORS[light];
  return (
    <>
      <div className="flex w-full items-start justify-between gap-1">
        {iconSrc ? (
          <img
            src={iconSrc}
            alt=""
            width={28}
            height={28}
            className="h-7 w-7 shrink-0 rounded-[7px]"
            draggable={false}
          />
        ) : (
          <span
            className="flex h-7 w-7 shrink-0 items-center justify-center rounded-[7px] text-[11px] font-semibold"
            style={{ backgroundColor: theme.hoverBg, color: theme.textSecondary }}
            aria-hidden
          >
            {name.slice(0, 1)}
          </span>
        )}
        <span
          className={`mt-0.5 inline-block h-2 w-2 shrink-0 rounded-full${
            light === "green" || busy ? " mb-status-pulse" : ""
          }${light === "yellow" && label.toLowerCase().includes("connect") ? " mb-status-pulse" : ""}`}
          style={{ backgroundColor: colors.dot }}
          aria-hidden
        />
      </div>
      <div className="min-w-0 w-full">
        <div className="truncate text-[11px] font-semibold leading-tight">
          {name}
        </div>
        <div
          className="mt-0.5 truncate text-[9.5px] font-medium leading-tight"
          style={{ color: colors.fg }}
        >
          {busy ? "Installing…" : label}
        </div>
      </div>
      <div
        role="tooltip"
        className="pointer-events-none absolute bottom-[calc(100%+6px)] left-1/2 z-20 w-[min(200px,70vw)] -translate-x-1/2 rounded-lg px-2.5 py-2 text-[10.5px] leading-snug opacity-0 shadow-lg transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100"
        style={{
          backgroundColor: theme.name === "dark" ? "#1C1C1E" : "#FFFFFF",
          border: `1px solid ${theme.hairline}`,
          color: theme.textSecondary,
        }}
      >
        {diagnostic}
      </div>
    </>
  );
}

export function IntegrationCard({
  name,
  iconSrc,
  diagnostic,
  light,
  label,
  theme,
  expandable = false,
  expanded = false,
  busy = false,
  onSelect,
}: {
  name: string;
  iconSrc?: string;
  diagnostic: string;
  light: TrafficLight;
  label: string;
  theme: ThemeTokens;
  expandable?: boolean;
  expanded?: boolean;
  busy?: boolean;
  onSelect?: () => void;
}) {
  const colors = TRAFFIC_COLORS[light];
  const shellStyle = {
    backgroundColor: expanded ? `${colors.dot}14` : theme.panel,
    border: `1.5px solid ${expanded ? colors.dot : theme.hairline}`,
    boxShadow: expanded ? `0 0 0 2px ${colors.dot}28` : undefined,
    color: theme.text,
    opacity: busy ? 0.72 : 1,
  } as const;

  const face = (
    <TileFace
      name={name}
      iconSrc={iconSrc}
      diagnostic={diagnostic}
      light={light}
      label={label}
      theme={theme}
      busy={busy}
    />
  );

  const className =
    "group relative flex h-[72px] w-full cursor-pointer flex-col items-stretch justify-between rounded-xl px-2 py-1.5 text-left transition-[background-color,border-color,box-shadow,opacity]";

  return (
    <li className="relative list-none">
      <button
        type="button"
        onClick={onSelect}
        aria-label={`${name}: ${busy ? "Installing" : label}. ${diagnostic}`}
        aria-expanded={expandable ? expanded : undefined}
        aria-busy={busy || undefined}
        className={className}
        style={shellStyle}
      >
        {face}
      </button>
    </li>
  );
}

export const IntegrationDetail = forwardRef<
  HTMLDivElement,
  {
    name: string;
    iconSrc?: string;
    diagnostic: string;
    theme: ThemeTokens;
    guidance?: { title: string; steps: string[] } | null;
    /** Healthy connected guidance uses green; setup uses yellow. */
    guidanceTone?: TrafficLight;
    children?: ReactNode;
  }
>(function IntegrationDetail(
  { name, iconSrc, diagnostic, theme, guidance, guidanceTone = "yellow", children },
  ref,
) {
  const tone = TRAFFIC_COLORS[guidanceTone];
  return (
    <div
      ref={ref}
      className="mt-3 rounded-2xl px-3 py-3"
      style={{
        backgroundColor: theme.panel,
        border: `1px solid ${theme.hairline}`,
      }}
    >
      <div className="flex items-center gap-2">
        {iconSrc ? (
          <img
            src={iconSrc}
            alt=""
            width={22}
            height={22}
            className="h-[22px] w-[22px] rounded-[6px]"
            draggable={false}
          />
        ) : null}
        <div className="text-[12.5px] font-semibold">{name}</div>
      </div>
      <div className="mt-1 text-[11px]" style={{ color: theme.textSecondary }}>
        {diagnostic}
      </div>
      {guidance && guidance.steps.length > 0 && (
        <div
          className="mt-2 rounded-lg px-2.5 py-2 text-[11px] leading-snug"
          style={{
            backgroundColor: tone.bg,
            color: tone.fg,
          }}
        >
          <div className="font-medium">{guidance.title}</div>
          <ol className="mt-1.5 list-decimal space-y-1 pl-4">
            {guidance.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        </div>
      )}
      {children}
    </div>
  );
});
