export type AgentState = 'idle' | 'thinking' | 'working' | 'awaiting-approval' | 'done' | 'error';

export const STATE_COLORS: Record<AgentState, string> = {
  idle: '#E9E9E6',
  thinking: '#3D7EFF',
  working: '#3D7EFF',
  'awaiting-approval': '#FFB000',
  done: '#30C463',
  error: '#FF453A'
};

/** Codex-default palette — used for "Reset to Codex defaults" */
export const CODEX_STATE_COLORS: Record<AgentState, string> = { ...STATE_COLORS };

/** Alternate "Phosphor" preset (orange-centric) */
export const PHOSPHOR_STATE_COLORS: Record<AgentState, string> = {
  idle: '#4A4A52',
  thinking: '#FFB454',
  working: '#FF6A00',
  'awaiting-approval': '#FF3D00',
  done: '#3DDC84',
  error: '#FF4757'
};

export const STATE_LABELS: Record<AgentState, string> = {
  idle: 'Idle',
  thinking: 'Thinking',
  working: 'Working',
  'awaiting-approval': 'Needs approval',
  done: 'Done',
  error: 'Error'
};

/* ------------------------------------------------------------------ */
/* Live sessions (what the Agent Keys follow)                          */
/* ------------------------------------------------------------------ */

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


/** Which session each of the six Agent Keys follows (key source: Focused app). */
export const AGENT_KEY_ASSIGNMENTS: Record<string, string | null> = {
  ag1: 's1',
  ag2: 's2',
  ag3: 's3',
  ag4: 's4',
  ag5: 's5',
  ag6: null
};

export function sessionForAgentKey(agentKeyId: string): Session | null {
  const sid = AGENT_KEY_ASSIGNMENTS[agentKeyId];
  return sid ? SESSIONS.find((s) => s.id === sid) ?? null : null;
}

/* ------------------------------------------------------------------ */
/* Device controls (real kbd-1.0 layout)                               */
/* ------------------------------------------------------------------ */

export type ControlKind = 'knob' | 'joystick' | 'touch' | 'agent-key' | 'command-key';

export type ControlId =
'knob' | 'joystick' | 'touch' |
'ag1' | 'ag2' | 'ag3' | 'ag4' | 'ag5' | 'ag6' |
'fast' | 'approve' | 'reject' | 'fork' | 'mic' | 'codex';

export interface ControlDef {
  id: ControlId;
  kind: ControlKind;
  label: string;
  /** Icon printed on the shipped keycap, if any */
  icon?: 'bolt' | 'check' | 'cross' | 'fork' | 'mic' | 'codex';
}

export const CONTROLS: Record<ControlId, ControlDef> = {
  knob: { id: 'knob', kind: 'knob', label: 'Dial' },
  joystick: { id: 'joystick', kind: 'joystick', label: 'Joystick' },
  touch: { id: 'touch', kind: 'touch', label: 'Touch sensor' },
  ag1: { id: 'ag1', kind: 'agent-key', label: 'Agent Key 1' },
  ag2: { id: 'ag2', kind: 'agent-key', label: 'Agent Key 2' },
  ag3: { id: 'ag3', kind: 'agent-key', label: 'Agent Key 3' },
  ag4: { id: 'ag4', kind: 'agent-key', label: 'Agent Key 4' },
  ag5: { id: 'ag5', kind: 'agent-key', label: 'Agent Key 5' },
  ag6: { id: 'ag6', kind: 'agent-key', label: 'Agent Key 6' },
  fast: { id: 'fast', kind: 'command-key', label: 'Fast key', icon: 'bolt' },
  approve: { id: 'approve', kind: 'command-key', label: 'Approve key', icon: 'check' },
  reject: { id: 'reject', kind: 'command-key', label: 'Reject key', icon: 'cross' },
  fork: { id: 'fork', kind: 'command-key', label: 'Fork key', icon: 'fork' },
  mic: { id: 'mic', kind: 'command-key', label: 'Mic bar', icon: 'mic' },
  codex: { id: 'codex', kind: 'command-key', label: 'Codex key', icon: 'codex' }
};

/* ------------------------------------------------------------------ */
/* Assignable actions                                                  */
/* ------------------------------------------------------------------ */

export type ActionGroup = 'AGENT' | 'SKILL' | 'SYSTEM' | 'MACRO';

export interface ActionDef {
  id: string;
  group: ActionGroup;
  label: string;
}

export const ACTIONS: ActionDef[] = [
{ id: 'approve', group: 'AGENT', label: 'Approve' },
{ id: 'reject', group: 'AGENT', label: 'Reject' },
{ id: 'fast_mode', group: 'AGENT', label: 'Toggle fast mode' },
{ id: 'fork_thread', group: 'AGENT', label: 'Fork thread' },
{ id: 'new_chat', group: 'AGENT', label: 'New chat' },
{ id: 'push_to_talk', group: 'AGENT', label: 'Push to talk' },
{ id: 'interrupt', group: 'AGENT', label: 'Interrupt' },
{ id: 'skill_review_pr', group: 'SKILL', label: 'Review PR' },
{ id: 'skill_debug', group: 'SKILL', label: 'Debug error' },
{ id: 'skill_refactor', group: 'SKILL', label: 'Refactor' },
{ id: 'skill_explain', group: 'SKILL', label: 'Explain code' },
{ id: 'skill_tests', group: 'SKILL', label: 'Write tests' },
{ id: 'pause_leds', group: 'SYSTEM', label: 'Pause LEDs' },
{ id: 'cycle_focus', group: 'SYSTEM', label: 'Cycle focus' },
{ id: 'custom_command', group: 'MACRO', label: 'Custom command…' },
{ id: 'send_keystroke', group: 'MACRO', label: 'Send keystroke' }];


export const ACTION_GROUP_LABELS: Record<ActionGroup, string> = {
  AGENT: 'Agent',
  SKILL: 'Skills',
  SYSTEM: 'System',
  MACRO: 'Macro'
};

export function getAction(id: string): ActionDef {
  return ACTIONS.find((a) => a.id === id) ?? ACTIONS[0];
}

/* Knob rotate options */
export interface RotateActionDef {
  id: string;
  label: string;
}

export const ROTATE_ACTIONS: RotateActionDef[] = [
{ id: 'reasoning_effort', label: 'Reasoning effort' },
{ id: 'scroll_thread', label: 'Scroll thread' },
{ id: 'brightness', label: 'LED brightness' },
{ id: 'cycle_sessions', label: 'Cycle sessions' }];


export type JoyDir = 'up' | 'down' | 'left' | 'right';
export const JOY_DIRS: {id: JoyDir;label: string;}[] = [
{ id: 'up', label: 'Flick up' },
{ id: 'down', label: 'Flick down' },
{ id: 'left', label: 'Flick left' },
{ id: 'right', label: 'Flick right' }];


/* ------------------------------------------------------------------ */
/* Bindings (factory defaults, matching the shipped caps)              */
/* ------------------------------------------------------------------ */

export interface DeviceBindings {
  commandKeys: Record<string, string>; // controlId -> actionId
  knobRotate: string;
  knobPress: string;
  joystick: Record<JoyDir, string>;
  joystickPress: string;
  touch: string;
}

export const DEFAULT_BINDINGS: DeviceBindings = {
  commandKeys: {
    fast: 'fast_mode',
    approve: 'approve',
    reject: 'reject',
    fork: 'fork_thread',
    mic: 'push_to_talk',
    codex: 'new_chat'
  },
  knobRotate: 'reasoning_effort',
  knobPress: 'cycle_focus',
  joystick: {
    up: 'skill_review_pr',
    down: 'skill_debug',
    left: 'skill_refactor',
    right: 'skill_explain'
  },
  joystickPress: 'pause_leds',
  touch: 'pause_leds'
};

export const REASONING_LEVELS = ['Light', 'Standard', 'High', 'Extra High'];

/* ------------------------------------------------------------------ */
/* Adapters                                                            */
/* ------------------------------------------------------------------ */

export interface AdapterDef {
  id: string;
  name: string;
  badge: 'NATIVE' | 'COMMUNITY';
  status: 'connected' | 'beta' | 'not_installed';
  detail: string;
  footprint?: string;
}

export const ADAPTERS: AdapterDef[] = [
{ id: 'codex', name: 'Codex', badge: 'NATIVE', status: 'connected', detail: 'watching ~/.codex/sessions', footprint: '0.0% CPU idle' },
{ id: 'claude', name: 'Claude Code', badge: 'NATIVE', status: 'connected', detail: 'hooks + session watcher', footprint: '0.1% CPU idle' },
{ id: 'cursor', name: 'Cursor', badge: 'COMMUNITY', status: 'beta', detail: 'polling workspace state', footprint: '0.3% CPU idle' },
{ id: 't3', name: 'T3 Code', badge: 'COMMUNITY', status: 'not_installed', detail: 'adapter not installed' }];


export interface FocusAppDef {
  id: string;
  name: string;
}

export const FOCUS_APPS_DEFAULT: FocusAppDef[] = [
{ id: 'codex', name: 'Codex' },
{ id: 'claude', name: 'Claude Code' },
{ id: 'cursor', name: 'Cursor' },
{ id: 't3', name: 'T3 Code' }];