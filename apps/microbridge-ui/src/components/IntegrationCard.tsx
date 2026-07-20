import type { ReactNode } from "react";
import type { ThemeTokens } from "../lib/theme";
import {
  TRAFFIC_COLORS,
  type TrafficLight,
} from "../lib/hosts";

export function IntegrationCard({
  name,
  badge,
  diagnostic,
  light,
  label,
  theme,
  children,
}: {
  name: string;
  badge?: string;
  diagnostic: string;
  light: TrafficLight;
  label: string;
  theme: ThemeTokens;
  children?: ReactNode;
}) {
  const colors = TRAFFIC_COLORS[light];
  return (
    <li
      className="rounded-xl px-3 py-3"
      style={{
        backgroundColor: theme.panel,
        border: `1px solid ${theme.hairline}`,
      }}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2 text-[12.5px] font-medium">
            <span
              className="inline-block h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: colors.dot }}
              aria-hidden
            />
            {name}
            {badge ? (
              <span
                className="rounded-full px-2 py-0.5 text-[9.5px] capitalize"
                style={{
                  backgroundColor: theme.hoverBg,
                  color: theme.textSecondary,
                }}
              >
                {badge}
              </span>
            ) : null}
          </div>
          <div
            className="mt-1 text-[11px]"
            style={{ color: theme.textSecondary }}
          >
            {diagnostic}
          </div>
        </div>
        <span
          className="shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium"
          style={{ backgroundColor: colors.bg, color: colors.fg }}
        >
          {label}
        </span>
      </div>
      {children}
    </li>
  );
}
