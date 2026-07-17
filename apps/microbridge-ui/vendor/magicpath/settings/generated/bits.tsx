import type { ReactNode } from 'react';
import type { AgentState } from './microbridge-data';
import { STATE_COLORS, STATE_LABELS } from './microbridge-data';
import { useTheme } from './theme';
export const SectionLabel = ({
  children
}: {
  children: ReactNode;
}) => {
  const {
    t
  } = useTheme();
  return <span className="mb-2 block text-[12px] font-semibold" style={{
    color: t.text
  }}>
      {children}
    </span>;
};
export const Card = ({
  children,
  className = ''
}: {
  children: ReactNode;
  className?: string;
}) => {
  const {
    t
  } = useTheme();
  return <div className={`rounded-xl ${className}`} style={{
    backgroundColor: t.card,
    border: `1px solid ${t.cardBorder}`,
    boxShadow: t.name === 'light' ? '0 1px 2px rgba(0,0,0,0.04)' : 'none'
  }}>
      
      {children}
    </div>;
};

/** Codex-style status chip: soft tinted pill with a state dot. */
export const StateChip = ({
  state,
  colors = STATE_COLORS
}: {
  state: AgentState;
  colors?: Record<AgentState, string>;
}) => {
  const {
    t
  } = useTheme();
  const c = colors[state];
  const isIdle = state === 'idle';
  const pulse = state === 'awaiting-approval' ? 'mb-led-pulse' : state === 'thinking' || state === 'working' ? 'mb-led-breathe' : '';
  return <span className="inline-flex items-center gap-1.5 rounded-full px-2 py-[3px] text-[11px] font-medium" style={{
    backgroundColor: isIdle ? t.hoverBg : `${c}1F`,
    color: isIdle ? t.textSecondary : state === 'awaiting-approval' && t.name === 'light' ? '#8A6100' : c
  }}>
      
      <span className={`h-[6px] w-[6px] rounded-full ${pulse}`} style={{
      backgroundColor: isIdle ? t.textMuted : c,
      boxShadow: isIdle ? 'none' : `0 0 5px ${c}`
    }} />
      
      {STATE_LABELS[state]}
    </span>;
};

/** Neutral segmented control, macOS style. */
export const Segmented = <T extends string,>({
  options,
  value,
  onChange
}: {
  options: {
    id: T;
    label: string;
  }[];
  value: T;
  onChange: (v: T) => void;
}) => {
  const {
    t
  } = useTheme();
  return <div className="inline-flex rounded-lg p-[3px]" style={{
    backgroundColor: t.sunken
  }}>
      {options.map(o => <button key={o.id} type="button" onClick={() => onChange(o.id)} className="rounded-[7px] px-3 py-1 text-[12px] font-medium transition-all duration-150" style={value === o.id ? {
      backgroundColor: t.raised,
      color: t.text,
      boxShadow: '0 1px 2px rgba(0,0,0,0.12)'
    } : {
      color: t.textSecondary
    }}>
        
          {o.label}
        </button>)}
    </div>;
};