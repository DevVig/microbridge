import { STATE_COLORS, type Session, type ThemeTokens } from './microbridge-types';
export const AgentRow = ({
  session,
  t
}: {
  session: Session;
  t: ThemeTokens;
}) => {
  const c = STATE_COLORS[session.state];
  const isIdle = session.state === 'idle';
  const pulse = session.state === 'awaiting-approval' ? 'mb-led-pulse' : session.state === 'thinking' ? 'mb-led-breathe' : '';
  return <div className="flex items-center gap-2.5 rounded-lg px-2 py-[7px]">
      <span className={`h-[7px] w-[7px] shrink-0 rounded-full ${pulse}`} style={{
      backgroundColor: isIdle ? t.textMuted : c,
      boxShadow: isIdle ? 'none' : `0 0 5px ${c}`
    }} />
      
      <span className="w-[76px] shrink-0 truncate text-[11px]" style={{
      color: t.textSecondary
    }}>
        {session.app}
      </span>
      <span className="min-w-0 flex-1 truncate text-[12.5px]" style={{
      color: t.text,
      fontWeight: session.focused ? 600 : 400
    }}>
        
        {session.title}
      </span>
      <span className="shrink-0 text-[10.5px] tabular-nums" style={{
      color: t.textMuted
    }}>
        {session.elapsed}
      </span>
    </div>;
};