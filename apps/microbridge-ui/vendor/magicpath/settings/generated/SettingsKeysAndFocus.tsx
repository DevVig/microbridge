import { useState } from 'react';
import { KeysTab } from './KeysTab';
import { FocusTab } from './FocusTab';
import { AdaptersTab } from './AdaptersTab';
import { DeviceTab } from './DeviceTab';
import { ThemeProvider, useTheme } from './theme';
type TabId = 'KEYS' | 'AGENT_KEYS' | 'ADAPTERS' | 'DEVICE';
const TabIcon = ({
  id,
  size = 15
}: {
  id: TabId;
  size?: number;
}) => {
  const common = {
    width: size,
    height: size,
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.7,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const
  };
  switch (id) {
    case 'KEYS':
      return <svg {...common} aria-hidden="true">
          <rect x="3.5" y="5.5" width="17" height="13" rx="2.5" />
          <path d="M7.5 9.5h.01M12 9.5h.01M16.5 9.5h.01M7.5 14.5h9" />
        </svg>;
    case 'AGENT_KEYS':
      return <svg {...common} aria-hidden="true">
          <rect x="4" y="4" width="7" height="7" rx="2" />
          <rect x="13" y="4" width="7" height="7" rx="2" />
          <rect x="4" y="13" width="7" height="7" rx="2" />
          <rect x="13" y="13" width="7" height="7" rx="2" />
        </svg>;
    case 'ADAPTERS':
      return <svg {...common} aria-hidden="true">
          <path d="M9 7V4M15 7V4M7.5 7h9v5a4.5 4.5 0 0 1-9 0V7zM12 16.5V21" />
        </svg>;
    case 'DEVICE':
      return <svg {...common} aria-hidden="true">
          <circle cx="12" cy="12" r="3.2" />
          <path d="M12 2.8v2.4M12 18.8v2.4M2.8 12h2.4M18.8 12h2.4M5.5 5.5l1.7 1.7M16.8 16.8l1.7 1.7M5.5 18.5l1.7-1.7M16.8 7.2l1.7-1.7" />
        </svg>;
  }
};
const TABS: {
  id: TabId;
  label: string;
}[] = [{
  id: 'KEYS',
  label: 'Keys'
}, {
  id: 'AGENT_KEYS',
  label: 'Agent Keys'
}, {
  id: 'ADAPTERS',
  label: 'Adapters'
}, {
  id: 'DEVICE',
  label: 'Device'
}];
const SettingsWindow = () => {
  const {
    t
  } = useTheme();
  const [tab, setTab] = useState<TabId>('KEYS');
  return <div className="flex h-screen w-full items-center justify-center p-6 transition-colors duration-200" style={{
    background: t.name === 'light' ? 'radial-gradient(ellipse 120% 90% at 50% 0%, #F1F1EF 0%, #E4E4E1 100%)' : 'radial-gradient(ellipse 120% 90% at 50% 0%, #131315 0%, #08080A 100%)'
  }}>
      
      <div className="mb-frost flex h-full max-h-[780px] w-full max-w-[1120px] min-w-[860px] flex-col overflow-hidden rounded-2xl" style={{
      backgroundColor: t.panel,
      border: `1px solid ${t.panelBorder}`,
      boxShadow: '0 32px 80px rgba(0,0,0,0.30), 0 2px 8px rgba(0,0,0,0.12)'
    }}>
        
        {/* Titlebar */}
        <div className="flex h-[44px] shrink-0 items-center gap-2 px-4" style={{
        borderBottom: `1px solid ${t.hairline}`
      }}>
          <div className="flex items-center gap-[7px]">
            <span className="h-[11px] w-[11px] rounded-full bg-[#FF5F57]" />
            <span className="h-[11px] w-[11px] rounded-full bg-[#FEBC2E]" />
            <span className="h-[11px] w-[11px] rounded-full bg-[#28C840]" />
          </div>
          <span className="ml-2 text-[13px] font-semibold" style={{
          color: t.text
        }}>
            Microbridge
          </span>
          <span className="text-[13px]" style={{
          color: t.textMuted
        }}>
            Settings
          </span>
          <div className="ml-auto flex items-center gap-2">
            <span className="flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium" style={{
            backgroundColor: '#30C4631F',
            color: '#30C463'
          }}>
              
              <span className="h-[6px] w-[6px] rounded-full" style={{
              backgroundColor: '#30C463',
              boxShadow: '0 0 5px #30C463'
            }} />
              Codex Micro connected
            </span>
          </div>
        </div>

        <div className="flex min-h-0 flex-1">
          {/* Rail */}
          <nav className="flex w-[176px] shrink-0 flex-col gap-0.5 p-3" style={{
          borderRight: `1px solid ${t.hairline}`
        }}>
            {TABS.map(tb => <button key={tb.id} type="button" onClick={() => setTab(tb.id)} className="flex items-center gap-2.5 rounded-lg px-3 py-2 text-left text-[13px] font-medium transition-colors" style={tab === tb.id ? {
            backgroundColor: t.name === 'light' ? 'rgba(0,0,0,0.07)' : 'rgba(255,255,255,0.10)',
            color: t.text
          } : {
            color: t.textSecondary
          }}>
              
                <TabIcon id={tb.id} />
                {tb.label}
              </button>)}
            <div className="mt-auto pt-3" style={{
            borderTop: `1px solid ${t.hairline}`
          }}>
              <p className="px-1 text-[10px] leading-relaxed" style={{
              color: t.textMuted
            }}>
                Keyboard setup. Actions live on the Micro, not in this window.
              </p>
            </div>
          </nav>

          {/* Content */}
          <div className="mb-scrollbar min-w-0 flex-1 overflow-y-auto p-6">
            {tab === 'KEYS' && <KeysTab onOpenAgentKeys={() => setTab('AGENT_KEYS')} />}
            {tab === 'AGENT_KEYS' && <FocusTab />}
            {tab === 'ADAPTERS' && <AdaptersTab />}
            {tab === 'DEVICE' && <DeviceTab />}
          </div>
        </div>
      </div>
    </div>;
};
export const SettingsKeysAndFocus = () => <ThemeProvider defaultChoice="light">
    <SettingsWindow />
  </ThemeProvider>;