import { useState } from 'react';
import { ADAPTERS } from './microbridge-data';
import { Card } from './bits';
import { Toggle } from './Toggle';
import { useTheme } from './theme';
export const AdaptersTab = () => {
  const {
    t
  } = useTheme();
  const [enabled, setEnabled] = useState<Record<string, boolean>>({
    codex: true,
    claude: true,
    cursor: true,
    t3: false
  });
  return <div className="flex max-w-[720px] flex-col gap-3">
      {ADAPTERS.map(a => <Card key={a.id} className="flex flex-wrap items-center gap-4 p-4">
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-[14px] font-medium" style={{
            color: t.text
          }}>
                {a.name}
              </span>
              <span className="rounded-full px-2 py-[2px] text-[10px] font-medium" style={a.badge === 'NATIVE' ? {
            backgroundColor: t.name === 'light' ? '#0D0D0D' : '#F5F5F4',
            color: t.name === 'light' ? '#FFFFFF' : '#0D0D0D'
          } : {
            backgroundColor: t.sunken,
            color: t.textSecondary
          }}>
              
                {a.badge === 'NATIVE' ? 'Native' : 'Community'}
              </span>
              <span className="flex items-center gap-1.5 text-[11px] font-medium">
                <span className="h-[6px] w-[6px] rounded-full" style={{
              backgroundColor: a.status === 'connected' ? '#30C463' : a.status === 'beta' ? '#FFB000' : t.textMuted
            }} />
              
                <span style={{
              color: a.status === 'connected' ? '#30C463' : a.status === 'beta' ? t.name === 'light' ? '#8A6100' : '#FFB000' : t.textMuted
            }}>
                  {a.status === 'connected' ? 'Connected' : a.status === 'beta' ? 'Beta' : 'Not installed'}
                </span>
              </span>
            </div>
            <p className="mt-1 font-mono text-[11px]" style={{
          color: t.textMuted
        }}>
              {a.detail}
            </p>
          </div>

          <div className="shrink-0 text-right">
            {a.footprint ? <span className="font-mono text-[11px]" style={{
          color: t.textMuted
        }}>
                {a.footprint}
              </span> : <a href="#" onClick={e => e.preventDefault()} className="text-[12px] font-medium transition-opacity hover:opacity-70" style={{
          color: '#3D7EFF'
        }}>
            
                Get adapter
              </a>}
          </div>

          <Toggle checked={enabled[a.id]} onChange={v => setEnabled(prev => ({
        ...prev,
        [a.id]: v
      }))} disabled={a.status === 'not_installed'} />
        
        </Card>)}
      <p className="mt-1 text-[11px] leading-relaxed" style={{
      color: t.textMuted
    }}>
        Adapters publish thread state to the daemon. Anyone can build one — the protocol is open.
      </p>
    </div>;
};