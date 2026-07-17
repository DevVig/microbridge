import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
export type ThemeChoice = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';
export interface ThemeTokens {
  name: ResolvedTheme;
  /** desk behind the window */
  frame: string;
  /** frosted window material */
  panel: string;
  panelBorder: string;
  /** cards inside the window */
  card: string;
  cardBorder: string;
  sunken: string;
  raised: string;
  hairline: string;
  text: string;
  textSecondary: string;
  textMuted: string;
  /** selection ring on interactive controls */
  ring: string;
  hoverBg: string;
}
export const LIGHT: ThemeTokens = {
  name: 'light',
  frame: '#E9E9E7',
  panel: 'rgba(252,252,251,0.86)',
  panelBorder: 'rgba(0,0,0,0.10)',
  card: '#FFFFFF',
  cardBorder: 'rgba(0,0,0,0.08)',
  sunken: '#F4F4F2',
  raised: '#FFFFFF',
  hairline: 'rgba(0,0,0,0.08)',
  text: '#0D0D0D',
  textSecondary: '#6E6E73',
  textMuted: '#AEAEB2',
  ring: '#0D0D0D',
  hoverBg: 'rgba(0,0,0,0.04)'
};
export const DARK: ThemeTokens = {
  name: 'dark',
  frame: '#0A0A0B',
  panel: 'rgba(24,24,26,0.88)',
  panelBorder: 'rgba(255,255,255,0.10)',
  card: 'rgba(255,255,255,0.05)',
  cardBorder: 'rgba(255,255,255,0.08)',
  sunken: 'rgba(0,0,0,0.25)',
  raised: 'rgba(255,255,255,0.09)',
  hairline: 'rgba(255,255,255,0.09)',
  text: '#F5F5F4',
  textSecondary: '#A0A0A6',
  textMuted: '#5E5E66',
  ring: '#F5F5F4',
  hoverBg: 'rgba(255,255,255,0.06)'
};
interface ThemeContextValue {
  choice: ThemeChoice;
  setChoice: (c: ThemeChoice) => void;
  resolved: ResolvedTheme;
  t: ThemeTokens;
}
const ThemeContext = createContext<ThemeContextValue>({
  choice: 'light',
  setChoice: () => {},
  resolved: 'light',
  t: LIGHT
});
export function ThemeProvider({
  children,
  defaultChoice = 'light'
}: {
  children: ReactNode;
  defaultChoice?: ThemeChoice;
}) {
  const [choice, setChoice] = useState<ThemeChoice>(defaultChoice);
  const [systemDark, setSystemDark] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    setSystemDark(mq.matches);
    const onChange = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, []);
  const resolved: ResolvedTheme = choice === 'system' ? systemDark ? 'dark' : 'light' : choice;
  const value = useMemo(() => ({
    choice,
    setChoice,
    resolved,
    t: resolved === 'dark' ? DARK : LIGHT
  }), [choice, resolved]);
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}
export function useTheme() {
  return useContext(ThemeContext);
}