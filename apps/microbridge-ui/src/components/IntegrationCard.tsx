import type { ReactNode } from "react";
import type { ThemeTokens } from "../lib/theme";
import {
  TRAFFIC_COLORS,
  type TrafficLight,
} from "../lib/hosts";

function TileFace({
  name,
  diagnostic,
  light,
  label,
  theme,
}: {
  name: string;
  diagnostic: string;
  light: TrafficLight;
  label: string;
  theme: ThemeTokens;
}) {
  const colors = TRAFFIC_COLORS[light];
  return (
    <>
      <span
        className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
        style={{ backgroundColor: colors.dot }}
        aria-hidden
      />
      <div className="min-w-0 w-full">
        <div className="truncate text-[12.5px] font-semibold leading-tight">
          {name}
        </div>
        <div
          className="mt-1 truncate text-[10px] font-medium"
          style={{ color: colors.fg }}
        >
          {label}
        </div>
      </div>
      <div
        role="tooltip"
        className="pointer-events-none absolute bottom-[calc(100%+6px)] left-1/2 z-20 w-[min(220px,70vw)] -translate-x-1/2 rounded-lg px-2.5 py-2 text-[10.5px] leading-snug opacity-0 shadow-lg transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100"
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
  diagnostic,
  light,
  label,
  theme,
  expandable = false,
  expanded = false,
  onSelect,
}: {
  name: string;
  diagnostic: string;
  light: TrafficLight;
  label: string;
  theme: ThemeTokens;
  expandable?: boolean;
  expanded?: boolean;
  onSelect?: () => void;
}) {
  const colors = TRAFFIC_COLORS[light];
  const shellStyle = {
    backgroundColor: theme.panel,
    border: `1px solid ${expanded ? colors.dot : theme.hairline}`,
    boxShadow: expanded ? `0 0 0 1px ${colors.dot}33` : undefined,
    color: theme.text,
  } as const;

  const face = (
    <TileFace
      name={name}
      diagnostic={diagnostic}
      light={light}
      label={label}
      theme={theme}
    />
  );

  return (
    <li className="relative list-none">
      {onSelect ? (
        <button
          type="button"
          onClick={onSelect}
          title={diagnostic}
          aria-label={`${name}: ${label}. ${diagnostic}`}
          aria-expanded={expandable ? expanded : undefined}
          className="group relative flex aspect-square w-full flex-col items-start justify-between rounded-2xl p-3 text-left transition-colors"
          style={shellStyle}
        >
          {face}
        </button>
      ) : (
        <div
          title={diagnostic}
          aria-label={`${name}: ${label}. ${diagnostic}`}
          className="group relative flex aspect-square w-full flex-col items-start justify-between rounded-2xl p-3"
          style={shellStyle}
        >
          {face}
        </div>
      )}
    </li>
  );
}

export function IntegrationDetail({
  name,
  diagnostic,
  theme,
  children,
}: {
  name: string;
  diagnostic: string;
  theme: ThemeTokens;
  children?: ReactNode;
}) {
  return (
    <div
      className="mt-3 rounded-2xl px-3 py-3"
      style={{
        backgroundColor: theme.panel,
        border: `1px solid ${theme.hairline}`,
      }}
    >
      <div className="text-[12.5px] font-semibold">{name}</div>
      <div className="mt-1 text-[11px]" style={{ color: theme.textSecondary }}>
        {diagnostic}
      </div>
      {children}
    </div>
  );
}
