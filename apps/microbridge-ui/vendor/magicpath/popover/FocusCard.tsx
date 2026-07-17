import { STATE_COLORS, STATE_LABELS, type Session, type ThemeTokens } from './microbridge-types';
export const StateChip = ({
  state,
  t
}: {
  state: Session['state'];
  t: ThemeTokens;
}) => {
  const c = STATE_COLORS[state];
  const isIdle = state === 'idle';
  const pulse = state === 'awaiting-approval' ? 'mb-led-pulse' : state === 'thinking' || state === 'working' ? 'mb-led-breathe' : '';
  return <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full px-2 py-[3px] text-[11px] font-medium" style={{
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
export const FocusCard = ({
  session,
  t
}: {
  session: Session;
  t: ThemeTokens;
}) => <div className="rounded-xl p-3.5" style={{
  backgroundColor: t.sunken
}}>
    <div className="flex items-center justify-between gap-2">
      <span className="text-[11px] font-medium" style={{
      color: t.textSecondary
    }}>
        {session.app} · owns the deck
      </span>
      <StateChip state={session.state} t={t} />
    </div>
    <p className="mt-1.5 truncate text-[14px] font-semibold" style={{
    color: t.text
  }}>
      {session.title}
    </p>
    <div className="mt-2 flex items-center gap-2">
      <span className="text-[11px] tabular-nums" style={{
      color: t.textMuted
    }}>
        {session.elapsed}
      </span>
      <span className="h-[3px] w-[3px] rounded-full" style={{
      backgroundColor: t.textMuted
    }} />
      <span className="whitespace-nowrap rounded-full px-1.5 py-[2px] text-[10px] font-medium" style={{
      backgroundColor: t.hoverBg,
      color: t.textSecondary
    }}>
      
        High reasoning
      </span>
      <span className="ml-auto text-[10px]" style={{
      color: t.textMuted
    }}>
        press = switch · double-press = open
      </span>
    </div>
  </div>;