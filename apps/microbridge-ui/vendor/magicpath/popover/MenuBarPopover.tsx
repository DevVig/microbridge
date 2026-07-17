import { useState } from 'react';
import { AGENT_KEY_SESSIONS, DARK, LIGHT, SESSIONS } from './microbridge-types';
import { AgentKeyEcho } from './AgentKeyEcho';
import { FocusCard } from './FocusCard';
import { AgentRow } from './AgentRow';
const MicroGlyph = ({
  color
}: {
  color: string;
}) => <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <rect x="3.5" y="5" width="17" height="14" rx="3" />
    <path d="M7.5 9h.01M12 9h.01M16.5 9h.01M7.5 13h.01M12 13h.01M16.5 13h.01" />
  </svg>;
export const MenuBarPopover = () => {
  const [dark, setDark] = useState(false);
  const [connected, setConnected] = useState(true);
  const [ledsPaused, setLedsPaused] = useState(false);
  const t = dark ? DARK : LIGHT;
  const focused = SESSIONS.find(s => s.focused) ?? SESSIONS[0];
  const liveCount = AGENT_KEY_SESSIONS.filter(Boolean).length;
  const footerButton = (label: string, onClick?: () => void, active = false) => <button type="button" onClick={onClick} className="rounded-md px-2 py-1 text-[12px] font-medium transition-colors" style={{
    color: active ? t.text : t.textSecondary,
    backgroundColor: active ? t.hoverBg : 'transparent'
  }} onMouseEnter={e => e.currentTarget.style.backgroundColor = t.hoverBg} onMouseLeave={e => e.currentTarget.style.backgroundColor = active ? t.hoverBg : 'transparent'}>
    
      {label}
    </button>;
  return <div className="flex min-h-screen w-full flex-col items-center transition-colors duration-200" style={{
    background: t.name === 'light' ? 'radial-gradient(ellipse 120% 90% at 50% 0%, #F1F1EF 0%, #E2E2DF 100%)' : 'radial-gradient(ellipse 120% 90% at 50% 0%, #131315 0%, #08080A 100%)'
  }}>
      
      {/* macOS menu bar */}
      <div className="mb-frost flex h-[30px] w-full shrink-0 items-center justify-end gap-4 px-4" style={{
      backgroundColor: t.panel,
      borderBottom: `1px solid ${t.hairline}`
    }}>
        
        <span className="rounded-md px-1.5 py-[3px]" style={{
        backgroundColor: t.hoverBg
      }}>
          <MicroGlyph color={t.text} />
        </span>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke={t.textSecondary} strokeWidth="1.8" strokeLinecap="round" aria-hidden="true">
          <path d="M5 12.5a10 10 0 0 1 14 0M8 15.5a6 6 0 0 1 8 0M11 18.5a2.5 2.5 0 0 1 2 0" />
        </svg>
        <span className="text-[12px] font-medium tabular-nums" style={{
        color: t.textSecondary
      }}>
          Thu 9:41 AM
        </span>
      </div>

      {/* Popover */}
      <div className="mb-frost mt-2 flex w-[360px] flex-col overflow-hidden rounded-2xl" style={{
      backgroundColor: t.panel,
      border: `1px solid ${t.panelBorder}`,
      boxShadow: '0 24px 64px rgba(0,0,0,0.28), 0 2px 8px rgba(0,0,0,0.12)'
    }}>
        
        {/* Header */}
        <div className="flex items-center gap-2 px-4 pb-3 pt-3.5">
          <span className="text-[13px] font-semibold" style={{
          color: t.text
        }}>
            Microbridge
          </span>
          <button type="button" onClick={() => setConnected(c => !c)} title="Demo: toggle connection" className="ml-auto flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium transition-colors" style={connected ? {
          backgroundColor: '#30C4631F',
          color: '#30C463'
        } : {
          backgroundColor: t.hoverBg,
          color: t.textSecondary
        }}>
            
            <span className="h-[6px] w-[6px] rounded-full" style={{
            backgroundColor: connected ? '#30C463' : t.textMuted,
            boxShadow: connected ? '0 0 5px #30C463' : 'none'
          }} />
            
            {connected ? 'Connected' : 'Disconnected'}
          </button>
        </div>

        {connected ? <>
            <div className="px-3 pb-3">
              <FocusCard session={focused} t={t} />
            </div>

            <div className="flex justify-center px-3 pb-3">
              <AgentKeyEcho t={t} connected />
            </div>

            <div className="px-3 pb-2" style={{
          borderTop: `1px solid ${t.hairline}`
        }}>
              <div className="flex items-center justify-between px-2 pb-1 pt-2.5">
                <span className="text-[11px] font-semibold" style={{
              color: t.textSecondary
            }}>
                  Threads
                </span>
                <span className="text-[10.5px] tabular-nums" style={{
              color: t.textMuted
            }}>
                  {liveCount} on keys
                </span>
              </div>
              {SESSIONS.map(s => <AgentRow key={s.id} session={s} t={t} />)}
            </div>
          </> : <div className="flex flex-col items-center px-6 pb-6 pt-2 text-center">
            <AgentKeyEcho t={t} connected={false} />
            <p className="mt-4 text-[14px] font-semibold" style={{
          color: t.text
        }}>
              Connect your Codex Micro
            </p>
            <p className="mt-1 max-w-[240px] text-[12px] leading-relaxed" style={{
          color: t.textSecondary
        }}>
              Plug in over USB-C or pair over Bluetooth. Your Agent Keys light up the moment a thread goes live.
            </p>
          </div>}

        {/* Footer */}
        <div className="flex items-center gap-1 px-2.5 py-2" style={{
        borderTop: `1px solid ${t.hairline}`
      }}>
          {footerButton('Settings')}
          {footerButton(ledsPaused ? 'Resume LEDs' : 'Pause LEDs', () => setLedsPaused(p => !p), ledsPaused)}
          <span className="ml-auto">{footerButton('Quit')}</span>
        </div>
      </div>

      {/* Canvas-only theme preview, not part of the popover */}
      <button type="button" onClick={() => setDark(d => !d)} className="mt-6 rounded-full px-3 py-1.5 text-[11px] font-medium transition-colors" style={{
      backgroundColor: t.hoverBg,
      color: t.textSecondary
    }}>
        
        Preview {dark ? 'light' : 'dark'} appearance
      </button>
      <p className="mt-1.5 max-w-[280px] pb-6 text-center text-[10px] leading-relaxed" style={{
      color: t.textMuted
    }}>
        Appearance follows the system (configurable in Settings). This switch is a preview control, not part of the popover.
      </p>
    </div>;
};