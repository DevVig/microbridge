import { useState } from 'react';

type AgentState = 'thinking' | 'working' | 'awaiting-approval' | 'done' | 'idle' | 'error';

interface AgentFocus {
  app: string;
  session: string;
  state: AgentState;
  keyIndex: number;
}

const TOTAL_KEYS = 6;

/** Canonical demo: Claude Code needs approval on Agent Key 2 */
const DEMO_AGENT: AgentFocus = {
  app: 'Claude Code',
  session: 'adapters — cursor beta cleanup',
  state: 'awaiting-approval',
  keyIndex: 1
};

const STATE_META: Record<AgentState, { label: string; color: string; pulse: boolean }> = {
  thinking: { label: 'Thinking', color: '#3D7EFF', pulse: true },
  working: { label: 'Working', color: '#3D7EFF', pulse: false },
  'awaiting-approval': { label: 'Needs approval', color: '#FFB000', pulse: true },
  done: { label: 'Done', color: '#30C463', pulse: false },
  idle: { label: 'Idle', color: '#E9E9E6', pulse: false },
  error: { label: 'Error', color: '#FF453A', pulse: false }
};

const TOKENS = {
  light: {
    frame: 'radial-gradient(ellipse 120% 90% at 50% 0%, #F1F1EF 0%, #E2E2DF 100%)',
    panel: 'rgba(252,252,251,0.88)',
    panelBorder: 'rgba(0,0,0,0.10)',
    sunken: '#F0F0EE',
    text: '#0D0D0D',
    secondary: '#6E6E73',
    muted: '#AEAEB2',
    hoverBg: 'rgba(0,0,0,0.05)'
  },
  dark: {
    frame: 'radial-gradient(ellipse 120% 90% at 50% 0%, #131315 0%, #08080A 100%)',
    panel: 'rgba(26,26,28,0.90)',
    panelBorder: 'rgba(255,255,255,0.10)',
    sunken: 'rgba(0,0,0,0.24)',
    text: '#F5F5F4',
    secondary: '#A0A0A6',
    muted: '#5E5E66',
    hoverBg: 'rgba(255,255,255,0.07)'
  }
} as const;

/** Mini frosted Agent Key, echoing the real translucent caps */
const MiniKey = ({ lit, color, pulse }: { lit: boolean; color: string; pulse: boolean }) => (
  <span
    className="relative block rounded-[5px]"
    style={{
      width: 18,
      height: 18,
      background: 'linear-gradient(180deg, rgba(255,255,255,0.62), rgba(238,238,235,0.55))',
      border: '1px solid rgba(0,0,0,0.12)',
      boxShadow: lit ? `0 0 8px ${color}88` : 'none'
    }}
  >
    {lit && (
      <span
        className={`absolute inset-[1.5px] rounded-[3.5px] ${pulse ? 'mb-led-pulse' : ''}`}
        style={{ background: `radial-gradient(circle at 50% 55%, ${color}E6 0%, ${color}33 60%, transparent 85%)` }}
      />
    )}
    <span className="absolute left-1/2 top-1/2 h-[5px] w-[1.5px] -translate-x-1/2 -translate-y-1/2 rounded-[1px]" style={{ backgroundColor: 'rgba(70,70,82,0.3)' }} />
    <span className="absolute left-1/2 top-1/2 h-[1.5px] w-[5px] -translate-x-1/2 -translate-y-1/2 rounded-[1px]" style={{ backgroundColor: 'rgba(70,70,82,0.3)' }} />
  </span>
);

/** Non-interactive focus confirmation — actions stay on the Micro */
export const FocusHUD = () => {
  const [dark, setDark] = useState(false);
  const t = dark ? TOKENS.dark : TOKENS.light;
  const agent = DEMO_AGENT;
  const { label, color, pulse } = STATE_META[agent.state];
  const isIdle = agent.state === 'idle';
  const initials = agent.app
    .split(' ')
    .map((w) => w[0])
    .join('')
    .slice(0, 2)
    .toUpperCase();

  return (
    <div
      className="relative flex h-full min-h-screen w-full flex-col items-center justify-center px-6 transition-colors duration-200"
      style={{ background: t.frame }}
    >
      <div
        className="hud-card mb-frost pointer-events-none relative flex w-[440px] max-w-[92vw] select-none flex-col overflow-hidden rounded-2xl"
        style={{
          backgroundColor: t.panel,
          border: `1px solid ${t.panelBorder}`,
          boxShadow: '0 24px 64px rgba(0,0,0,0.28), 0 2px 8px rgba(0,0,0,0.12)'
        }}
        role="status"
        aria-live="polite"
        aria-label={`Focus changed to ${agent.app}, ${label}`}
      >
        <div className="flex items-center gap-3 px-4 pb-3 pt-3.5">
          <div
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl text-[13px] font-semibold"
            style={{ backgroundColor: t.sunken, color: t.text }}
          >
            {initials}
          </div>

          <div className="min-w-0 flex-1">
            <div className="text-[10.5px] font-medium" style={{ color: t.muted }}>
              Deck focus
            </div>
            <div className="truncate text-[15px] font-semibold leading-tight" style={{ color: t.text }}>
              {agent.app}
            </div>
            <div className="truncate text-[12px] leading-tight" style={{ color: t.secondary }}>
              {agent.session}
            </div>
          </div>

          <div className="flex shrink-0 flex-col items-end gap-2">
            <span
              className={`inline-flex items-center gap-1.5 whitespace-nowrap rounded-full px-2 py-[3px] text-[11px] font-medium ${pulse ? 'mb-led-breathe' : ''}`}
              style={{
                backgroundColor: isIdle ? t.hoverBg : `${color}1F`,
                color: isIdle ? t.secondary : agent.state === 'awaiting-approval' && !dark ? '#8A6100' : color
              }}
            >
              <span
                className="h-[6px] w-[6px] rounded-full"
                style={{ backgroundColor: isIdle ? t.muted : color, boxShadow: isIdle ? 'none' : `0 0 5px ${color}` }}
              />
              {label}
            </span>

            <div className="grid grid-cols-3 gap-1" aria-hidden="true">
              {Array.from({ length: TOTAL_KEYS }).map((_, i) => (
                <MiniKey key={i} lit={i === agent.keyIndex} color={color} pulse={pulse && i === agent.keyIndex} />
              ))}
            </div>
          </div>
        </div>

        <div className="px-4 pb-2.5 text-[10px]" style={{ color: t.muted }}>
          Press = switch focus · double-press = bring the window forward
        </div>

        <div className="absolute inset-x-0 bottom-0 h-[2px] overflow-hidden" style={{ backgroundColor: t.hoverBg }}>
          <div className="hud-progress-fill h-full w-full" style={{ backgroundColor: color }} />
        </div>
      </div>

      {/* Canvas-only theme preview, not part of the HUD */}
      <button
        type="button"
        onClick={() => setDark((d) => !d)}
        className="mt-8 rounded-full px-3 py-1.5 text-[11px] font-medium transition-colors"
        style={{ backgroundColor: t.hoverBg, color: t.secondary }}
      >
        Preview {dark ? 'light' : 'dark'} appearance
      </button>
    </div>
  );
};
