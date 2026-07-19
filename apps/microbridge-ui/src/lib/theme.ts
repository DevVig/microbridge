export interface ThemeTokens {
  name: "light" | "dark";
  frame: string;
  panel: string;
  panelBorder: string;
  floatingShadow: string;
  sunken: string;
  hairline: string;
  text: string;
  textSecondary: string;
  textMuted: string;
  hoverBg: string;
}

export const LIGHT: ThemeTokens = {
  name: "light",
  frame: "#E9E9E7",
  panel: "rgba(252,252,251,0.88)",
  panelBorder: "rgba(0,0,0,0.10)",
  floatingShadow:
    "0 8px 20px rgba(0,0,0,0.18), 0 1px 4px rgba(0,0,0,0.10)",
  sunken: "#F4F4F2",
  hairline: "rgba(0,0,0,0.08)",
  text: "#0D0D0D",
  textSecondary: "#6E6E73",
  textMuted: "#AEAEB2",
  hoverBg: "rgba(0,0,0,0.04)",
};

export const DARK: ThemeTokens = {
  name: "dark",
  frame: "#0A0A0B",
  panel: "rgba(26,26,28,0.90)",
  panelBorder: "rgba(255,255,255,0.10)",
  floatingShadow:
    "0 8px 20px rgba(0,0,0,0.38), 0 1px 4px rgba(0,0,0,0.22)",
  sunken: "rgba(0,0,0,0.24)",
  hairline: "rgba(255,255,255,0.09)",
  text: "#F5F5F4",
  textSecondary: "#A0A0A6",
  textMuted: "#5E5E66",
  hoverBg: "rgba(255,255,255,0.06)",
};

export function resolveAppearance(
  preference: "system" | "light" | "dark",
): "light" | "dark" {
  if (preference === "light") return "light";
  if (preference === "dark") return "dark";
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return "light";
}
