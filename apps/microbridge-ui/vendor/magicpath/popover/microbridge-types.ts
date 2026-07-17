export type AgentState = 'idle' | 'thinking' | 'working' | 'awaiting-approval' | 'done' | 'error';

export const STATE_COLORS: Record<AgentState, string> = {
  idle: '#E9E9E6',
  thinking: '#3D7EFF',
  working: '#3D7EFF',
  'awaiting-approval': '#FFB000',
  done: '#30C463',
  error: '#FF453A'
};

export const STATE_LABELS: Record<AgentState, string> = {
  idle: 'Idle',
  thinking: 'Thinking',
  working: 'Working',
  'awaiting-approval': 'Needs approval',
  done: 'Done',
  error: 'Error'
};

export interface Session {
  id: string;
  app: string;
  title: string;
  state: AgentState;
  elapsed: string;
  focused?: boolean;
}

export const SESSIONS: Session[] = [
{ id: 's1', app: 'Codex', title: 'microbridge — HID reconnect on wake', state: 'working', elapsed: '12m', focused: true },
{ id: 's2', app: 'Claude Code', title: 'adapters — cursor beta cleanup', state: 'awaiting-approval', elapsed: '4m' },
{ id: 's3', app: 'Cursor', title: 'synara — onboarding empty states', state: 'thinking', elapsed: '1m' },
{ id: 's4', app: 'Codex', title: 'protocol v0 — golden vectors', state: 'done', elapsed: '22m' },
{ id: 's5', app: 'T3 Code', title: 't3code — session watcher spike', state: 'idle', elapsed: '38m' }];


/** Which session each of the six Agent Keys follows (key source: Most recent). */
export const AGENT_KEY_SESSIONS: (Session | null)[] = [SESSIONS[0], SESSIONS[1], SESSIONS[2], SESSIONS[3], SESSIONS[4], null];

export interface ThemeTokens {
  name: 'light' | 'dark';
  frame: string;
  panel: string;
  panelBorder: string;
  sunken: string;
  hairline: string;
  text: string;
  textSecondary: string;
  textMuted: string;
  hoverBg: string;
}

export const LIGHT: ThemeTokens = {
  name: 'light',
  frame: '#E9E9E7',
  panel: 'rgba(252,252,251,0.88)',
  panelBorder: 'rgba(0,0,0,0.10)',
  sunken: '#F4F4F2',
  hairline: 'rgba(0,0,0,0.08)',
  text: '#0D0D0D',
  textSecondary: '#6E6E73',
  textMuted: '#AEAEB2',
  hoverBg: 'rgba(0,0,0,0.04)'
};

export const DARK: ThemeTokens = {
  name: 'dark',
  frame: '#0A0A0B',
  panel: 'rgba(26,26,28,0.90)',
  panelBorder: 'rgba(255,255,255,0.10)',
  sunken: 'rgba(0,0,0,0.24)',
  hairline: 'rgba(255,255,255,0.09)',
  text: '#F5F5F4',
  textSecondary: '#A0A0A6',
  textMuted: '#5E5E66',
  hoverBg: 'rgba(255,255,255,0.06)'
};