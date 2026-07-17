import { useEffect, useRef, useState } from 'react';
import type { AgentState } from './microbridge-data';
import { CODEX_STATE_COLORS, PHOSPHOR_STATE_COLORS, STATE_COLORS, STATE_LABELS } from './microbridge-data';
import { Card, SectionLabel, Segmented } from './bits';
import { useTheme, type ThemeChoice } from './theme';
const STATE_SEQUENCE: AgentState[] = ['idle', 'thinking', 'working', 'awaiting-approval', 'done', 'error'];
const SLEEP_OPTIONS = ['3 minutes', '5 minutes', '15 minutes', '30 minutes', 'Never'];
const APPEARANCE_OPTIONS: {
  id: ThemeChoice;
  label: string;
}[] = [{
  id: 'system',
  label: 'System'
}, {
  id: 'light',
  label: 'Light'
}, {
  id: 'dark',
  label: 'Dark'
}];
export const DeviceTab = () => {
  const {
    t,
    choice,
    setChoice
  } = useTheme();
  const [lighting, setLighting] = useState<Record<AgentState, string>>({
    ...STATE_COLORS
  });
  const [brightness, setBrightness] = useState(72);
  const [testState, setTestState] = useState<AgentState | null>(null);
  const [sleepAfter, setSleepAfter] = useState('3 minutes');
  const [sleepOpen, setSleepOpen] = useState(false);
  const cycleRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const sleepRef = useRef<HTMLDivElement>(null);
  useEffect(() => () => {
    if (cycleRef.current) clearInterval(cycleRef.current);
  }, []);
  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (sleepRef.current && !sleepRef.current.contains(e.target as Node)) setSleepOpen(false);
    }
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, []);
  function runLedTest() {
    if (cycleRef.current) {
      clearInterval(cycleRef.current);
      cycleRef.current = null;
    }
    let i = 0;
    setTestState(STATE_SEQUENCE[0]);
    cycleRef.current = setInterval(() => {
      i += 1;
      if (i >= STATE_SEQUENCE.length) {
        if (cycleRef.current) clearInterval(cycleRef.current);
        cycleRef.current = null;
        setTestState(null);
        return;
      }
      setTestState(STATE_SEQUENCE[i]);
    }, 450);
  }
  const previewColor = testState ? lighting[testState] : lighting.idle;
  const previewOpacity = Math.max(0.15, brightness / 100);
  const presetButton = (label: string, onClick: () => void) => <button type="button" onClick={onClick} className="rounded-lg px-2.5 py-1 text-[11px] font-medium transition-colors" style={{
    backgroundColor: t.raised,
    border: `1px solid ${t.cardBorder}`,
    color: t.textSecondary
  }}>
    
      {label}
    </button>;
  return <div className="flex max-w-[640px] flex-col gap-5">
      {/* Appearance */}
      <Card className="flex items-center justify-between gap-4 p-4">
        <div>
          <span className="block text-[13px] font-medium" style={{
          color: t.text
        }}>
            Appearance
          </span>
          <span className="mt-0.5 block text-[12px]" style={{
          color: t.textSecondary
        }}>
            One coherent look per mode — no toggle in the menu bar.
          </span>
        </div>
        <Segmented options={APPEARANCE_OPTIONS} value={choice} onChange={setChoice} />
      </Card>

      {/* Lighting */}
      <Card className="p-4">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <SectionLabel>Lighting</SectionLabel>
          <div className="flex flex-wrap gap-2">
            {presetButton('Reset to Codex defaults', () => setLighting({
            ...CODEX_STATE_COLORS
          }))}
            {presetButton('Phosphor preset', () => setLighting({
            ...PHOSPHOR_STATE_COLORS
          }))}
          </div>
        </div>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
          {STATE_SEQUENCE.map(state => <div key={state} className="flex items-center gap-2.5 rounded-lg px-2.5 py-2" style={{
          backgroundColor: t.sunken
        }}>
              <input type="color" value={lighting[state]} onChange={e => setLighting(prev => ({
            ...prev,
            [state]: e.target.value
          }))} className="h-7 w-7 shrink-0 cursor-pointer rounded-md border-0 bg-transparent p-0" aria-label={`Color for ${STATE_LABELS[state]}`} />
            
              <span className="text-[12px]" style={{
            color: t.textSecondary
          }}>
                {STATE_LABELS[state]}
              </span>
            </div>)}
        </div>
        <p className="mt-3 text-[11px] leading-relaxed" style={{
        color: t.textMuted
      }}>
          Colors are rendering config on this machine — the protocol carries states, never colors.
        </p>
      </Card>

      {/* Brightness */}
      <Card className="p-4">
        <div className="mb-3 flex items-center justify-between">
          <SectionLabel>Brightness</SectionLabel>
          <span className="text-[13px] font-medium tabular-nums" style={{
          color: t.text
        }}>
            {brightness}%
          </span>
        </div>
        <div className="flex items-center gap-4">
          <input type="range" min={0} max={100} value={brightness} onChange={e => setBrightness(Number(e.target.value))} className="h-1.5 flex-1 cursor-pointer appearance-none rounded-full accent-[#3D7EFF]" style={{
          backgroundColor: t.sunken
        }} />
          
          <div className="h-8 w-8 shrink-0 rounded-full transition-all duration-150" style={{
          backgroundColor: previewColor,
          opacity: previewOpacity,
          border: `1px solid ${t.cardBorder}`,
          boxShadow: `0 0 ${10 * previewOpacity}px 1px ${previewColor}`
        }} />
          
        </div>
        <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
          <p className="text-[11px]" style={{
          color: t.textMuted
        }}>
            {testState ? `Testing: ${STATE_LABELS[testState]}` : 'Also on the dial when set to LED brightness.'}
          </p>
          {presetButton('Run LED test', runLedTest)}
        </div>
      </Card>

      {/* Sleep */}
      <Card className="p-4">
        <SectionLabel>Sleep after</SectionLabel>
        <div ref={sleepRef} className="relative">
          <button type="button" onClick={() => setSleepOpen(o => !o)} aria-expanded={sleepOpen} className="flex w-full items-center justify-between rounded-lg px-3 py-2 text-[13px] transition-colors" style={{
          backgroundColor: t.raised,
          border: `1px solid ${t.cardBorder}`,
          color: t.text
        }}>
            
            {sleepAfter}
            <svg width="10" height="6" viewBox="0 0 10 6" className={`transition-transform ${sleepOpen ? 'rotate-180' : ''}`}>
              <path d="M1 1L5 5L9 1" stroke={t.textSecondary} strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
          {sleepOpen && <div className="mb-frost absolute left-0 right-0 top-[calc(100%+4px)] z-20 rounded-lg p-1.5" style={{
          backgroundColor: t.panel,
          border: `1px solid ${t.panelBorder}`,
          boxShadow: '0 12px 32px rgba(0,0,0,0.25)'
        }}>
            
              {SLEEP_OPTIONS.map(opt => <button key={opt} type="button" onClick={() => {
            setSleepAfter(opt);
            setSleepOpen(false);
          }} className="block w-full rounded-md px-2 py-1.5 text-left text-[13px] transition-colors" style={opt === sleepAfter ? {
            backgroundColor: t.hoverBg,
            color: t.text,
            fontWeight: 500
          } : {
            color: t.text
          }}>
              
                  {opt}
                </button>)}
            </div>}
        </div>
        <p className="mt-2 text-[11px]" style={{
        color: t.textMuted
      }}>
          LEDs fade out when every thread has been idle this long. Any state change wakes them.
        </p>
      </Card>

      {/* Firmware */}
      <Card className="flex items-center justify-between p-4">
        <div>
          <span className="block text-[13px] font-medium" style={{
          color: t.text
        }}>
            Firmware
          </span>
          <span className="mt-0.5 block font-mono text-[12px]" style={{
          color: t.textSecondary
        }}>
            kbd-1.0 · fw 1.4.2
          </span>
        </div>
        <span className="flex items-center gap-1.5 text-[12px] font-medium" style={{
        color: '#30C463'
      }}>
          <span className="h-[6px] w-[6px] rounded-full" style={{
          backgroundColor: '#30C463'
        }} />
          Up to date
        </span>
      </Card>

      <Card className="flex items-center gap-3 p-4">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke={t.textSecondary} strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <rect x="4" y="10" width="16" height="10" rx="2" />
          <path d="M8 10V7a4 4 0 0 1 8 0v3" />
        </svg>
        <p className="text-[12px] leading-relaxed" style={{
        color: t.textSecondary
      }}>
          <span style={{
          color: t.text,
          fontWeight: 500
        }}>Zero network.</span> Microbridge never touches the network — all state stays on this machine.
        </p>
      </Card>
    </div>;
};