import { useState } from 'react';
import { ADAPTERS, FOCUS_APPS_DEFAULT, STATE_COLORS, sessionForAgentKey } from './microbridge-data';
import { Card, SectionLabel, StateChip } from './bits';
import { Toggle } from './Toggle';
import { useTheme } from './theme';
type FocusMode = 'AUTO' | 'PINNED';
type KeySource = 'MOST_RECENT' | 'FOCUSED_APP' | 'PINNED' | 'PRIORITY' | 'CUSTOM';
const KEY_SOURCES: {
  id: KeySource;
  title: string;
  desc: string;
}[] = [{
  id: 'MOST_RECENT',
  title: 'Most recent',
  desc: 'Each Agent Key follows the most recently updated thread, across all apps.'
}, {
  id: 'FOCUSED_APP',
  title: 'Focused app',
  desc: 'All six keys show threads from whichever app owns the deck — Codex\u2019s six, Cursor\u2019s five, and so on.'
}, {
  id: 'PINNED',
  title: 'Pinned',
  desc: 'Agent Keys only show threads you have explicitly pinned.'
}, {
  id: 'PRIORITY',
  title: 'Priority',
  desc: 'Fill keys from the app priority list, highest first.'
}, {
  id: 'CUSTOM',
  title: 'Custom',
  desc: 'Assign specific threads to each of the six key slots.'
}];
const MODES: {
  id: FocusMode;
  title: string;
  desc: string;
}[] = [{
  id: 'AUTO',
  title: 'Auto',
  desc: 'The deck drives whichever agent app has focus.'
}, {
  id: 'PINNED',
  title: 'Pinned',
  desc: 'Lock deck focus to one thread until unpinned.'
}];
const AGENT_KEY_IDS = ['ag1', 'ag2', 'ag3', 'ag4', 'ag5', 'ag6'] as const;
function statusDotColor(appId: string) {
  const adapter = ADAPTERS.find(a => a.id === appId);
  if (!adapter) return '#9A9A94';
  if (adapter.status === 'connected') return '#30C463';
  if (adapter.status === 'beta') return '#FFB000';
  return '#9A9A94';
}
export const FocusTab = () => {
  const {
    t
  } = useTheme();
  const [keySource, setKeySource] = useState<KeySource>('MOST_RECENT');
  const [mode, setMode] = useState<FocusMode>('AUTO');
  const [appOrder, setAppOrder] = useState<string[]>(FOCUS_APPS_DEFAULT.map(a => a.id));
  const [approvalsInterrupt, setApprovalsInterrupt] = useState(true);
  function move(index: number, dir: -1 | 1) {
    setAppOrder(prev => {
      const next = [...prev];
      const target = index + dir;
      if (target < 0 || target >= next.length) return prev;
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  }
  const radioCard = (active: boolean) => ({
    backgroundColor: active ? t.sunken : 'transparent',
    border: `1px solid ${active ? t.cardBorder : 'transparent'}`
  });
  return <div className="flex max-w-[640px] flex-col gap-6">
      {/* Live assignments */}
      <div>
        <SectionLabel>Six keys, six threads</SectionLabel>
        <Card className="p-2">
          {AGENT_KEY_IDS.map((id, i) => {
          const session = sessionForAgentKey(id);
          const color = session ? STATE_COLORS[session.state] : 'transparent';
          return <div key={id} className="flex items-center gap-3 rounded-lg px-2.5 py-2">
                <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-[7px] text-[10px] font-semibold" style={{
              background: 'linear-gradient(180deg, rgba(255,255,255,0.9), rgba(238,238,235,0.9))',
              border: '1px solid rgba(0,0,0,0.11)',
              color: '#6E6E73',
              boxShadow: session ? `0 0 8px ${color}88` : 'none'
            }}>
                  
                  {i + 1}
                </span>
                {session ? <>
                    <span className="w-[84px] shrink-0 truncate text-[11px]" style={{
                color: t.textSecondary
              }}>
                      {session.app}
                    </span>
                    <span className="min-w-0 flex-1 truncate text-[13px]" style={{
                color: t.text,
                fontWeight: session.focused ? 600 : 400
              }}>
                      {session.title}
                    </span>
                    <StateChip state={session.state} />
                  </> : <span className="text-[12px]" style={{
              color: t.textMuted
            }}>
                    Empty slot
                  </span>}
              </div>;
        })}
        </Card>
        <p className="mt-2 text-[11px] leading-relaxed" style={{
        color: t.textMuted
      }}>
          Live view of what the six Agent Keys follow right now. Press a key to switch to its thread; double-press brings the window forward.
        </p>
      </div>

      <div>
        <SectionLabel>Key source</SectionLabel>
        <Card className="p-1.5">
          {KEY_SOURCES.map(s => <button key={s.id} type="button" onClick={() => setKeySource(s.id)} className="flex w-full items-start gap-3 rounded-lg p-3 text-left transition-colors" style={radioCard(keySource === s.id)}>
            
              <span className="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full" style={{
            border: `1.5px solid ${keySource === s.id ? '#3D7EFF' : t.textMuted}`
          }}>
              
                {keySource === s.id && <span className="h-2 w-2 rounded-full" style={{
              backgroundColor: '#3D7EFF'
            }} />}
              </span>
              <span>
                <span className="block text-[13px] font-medium" style={{
              color: t.text
            }}>
                  {s.title}
                </span>
                <span className="mt-0.5 block text-[12px] leading-relaxed" style={{
              color: t.textSecondary
            }}>
                  {s.desc}
                </span>
              </span>
            </button>)}
        </Card>
      </div>

      <div>
        <SectionLabel>Deck focus</SectionLabel>
        <Card className="p-1.5">
          {MODES.map(m => <button key={m.id} type="button" onClick={() => setMode(m.id)} className="flex w-full items-start gap-3 rounded-lg p-3 text-left transition-colors" style={radioCard(mode === m.id)}>
            
              <span className="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full" style={{
            border: `1.5px solid ${mode === m.id ? '#3D7EFF' : t.textMuted}`
          }}>
              
                {mode === m.id && <span className="h-2 w-2 rounded-full" style={{
              backgroundColor: '#3D7EFF'
            }} />}
              </span>
              <span>
                <span className="block text-[13px] font-medium" style={{
              color: t.text
            }}>
                  {m.title}
                </span>
                <span className="mt-0.5 block text-[12px] leading-relaxed" style={{
              color: t.textSecondary
            }}>
                  {m.desc}
                </span>
              </span>
            </button>)}
        </Card>
      </div>

      <div>
        <SectionLabel>App priority</SectionLabel>
        <Card className="p-1.5">
          {appOrder.map((id, i) => {
          const app = FOCUS_APPS_DEFAULT.find(a => a.id === id);
          if (!app) return null;
          return <div key={id} className="flex items-center gap-3 rounded-lg px-3 py-2">
                <span className="w-4 text-[12px] tabular-nums" style={{
              color: t.textMuted
            }}>
                  {i + 1}
                </span>
                <span className="h-[6px] w-[6px] rounded-full" style={{
              backgroundColor: statusDotColor(id)
            }} />
                <span className="flex-1 text-[13px]" style={{
              color: t.text
            }}>
                  {app.name}
                </span>
                <div className="flex items-center gap-1">
                  {([-1, 1] as const).map(dir => <button key={dir} type="button" onClick={() => move(i, dir)} disabled={dir === -1 ? i === 0 : i === appOrder.length - 1} aria-label={dir === -1 ? 'Move up' : 'Move down'} className="flex h-6 w-6 items-center justify-center rounded-md transition-colors disabled:opacity-30" style={{
                border: `1px solid ${t.cardBorder}`,
                color: t.textSecondary
              }}>
                    
                      <svg width="8" height="6" viewBox="0 0 8 6" style={{
                  transform: dir === 1 ? 'rotate(180deg)' : 'none'
                }}>
                        <path d="M1 5L4 1L7 5" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />
                      </svg>
                    </button>)}
                </div>
              </div>;
        })}
        </Card>
      </div>

      <Card className="flex items-start justify-between gap-4 p-4">
        <div>
          <span className="block text-[13px] font-medium" style={{
          color: t.text
        }}>
            Approvals interrupt
          </span>
          <span className="mt-0.5 block max-w-[420px] text-[12px] leading-relaxed" style={{
          color: t.textSecondary
        }}>
            When any thread needs approval, the Approve and Reject keys temporarily route to it.
          </span>
        </div>
        <Toggle checked={approvalsInterrupt} onChange={setApprovalsInterrupt} />
      </Card>

      <p className="text-[11px] leading-relaxed" style={{
      color: t.textMuted
    }}>
        These settings tell the daemon how to route the physical keys. Approve and reject always happen on the Micro, never in this window.
      </p>
    </div>;
};