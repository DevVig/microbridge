import { useEffect, useRef, useState } from 'react';
import type { ControlId, DeviceBindings, JoyDir } from './microbridge-data';
import { CONTROLS, DEFAULT_BINDINGS, JOY_DIRS, REASONING_LEVELS, ROTATE_ACTIONS, sessionForAgentKey } from './microbridge-data';
import { DeviceTwin, CapIcon } from './DeviceKeys';
import { ActionPicker } from './ActionPicker';
import { Card, SectionLabel, StateChip } from './bits';
import { useTheme } from './theme';
export const KeysTab = ({
  onOpenAgentKeys
}: {
  onOpenAgentKeys: () => void;
}) => {
  const {
    t
  } = useTheme();
  const [selected, setSelected] = useState<ControlId>('ag1');
  const [bindings, setBindings] = useState<DeviceBindings>(DEFAULT_BINDINGS);
  const [listening, setListening] = useState(false);
  const listenTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => () => {
    if (listenTimeout.current) clearTimeout(listenTimeout.current);
  }, []);
  function handleListen() {
    if (listening) {
      if (listenTimeout.current) clearTimeout(listenTimeout.current);
      setListening(false);
      return;
    }
    setListening(true);
    listenTimeout.current = setTimeout(() => setListening(false), 3000);
  }
  const control = CONTROLS[selected];
  const session = control.kind === 'agent-key' ? sessionForAgentKey(selected) : null;
  const listenButton = <button type="button" onClick={handleListen} className={`shrink-0 rounded-lg px-3 py-1.5 text-[12px] font-medium transition-colors ${listening ? 'mb-listening' : ''}`} style={listening ? {
    backgroundColor: '#3D7EFF1A',
    color: '#3D7EFF',
    border: '1px solid #3D7EFF66'
  } : {
    backgroundColor: t.raised,
    color: t.textSecondary,
    border: `1px solid ${t.cardBorder}`
  }}>
    
      {listening ? 'Touch a control on your Micro…' : 'Listen'}
    </button>;
  return <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-6 lg:flex-row">
        {/* Device twin */}
        <div className="flex shrink-0 flex-col items-center gap-4">
          <DeviceTwin selected={selected} onSelect={setSelected} />
          <p className="max-w-[320px] text-center text-[11px] leading-relaxed" style={{
          color: t.textMuted
        }}>
            Click any control to configure it. Agent Keys glow with the live state of the thread they follow.
          </p>
        </div>

        {/* Inspector */}
        <Card className="min-w-0 flex-1 self-start p-5">
          <div className="mb-4 flex items-center justify-between gap-3">
            <div className="flex items-center gap-2.5">
              {control.icon && <span className="flex h-8 w-8 items-center justify-center rounded-lg" style={{
              backgroundColor: t.sunken,
              color: t.text
            }}>
                  <CapIcon icon={control.icon} size={16} />
                </span>}
              <h3 className="text-[15px] font-semibold" style={{
              color: t.text
            }}>
                {control.label}
              </h3>
            </div>
            {control.kind !== 'agent-key' && listenButton}
          </div>

          {/* Agent Key — read-only, thread comes through */}
          {control.kind === 'agent-key' && <div className="flex flex-col gap-4">
              {session ? <div className="rounded-xl p-4" style={{
            backgroundColor: t.sunken
          }}>
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-[11px] font-medium" style={{
                color: t.textSecondary
              }}>
                      {session.app}
                    </span>
                    <StateChip state={session.state} />
                  </div>
                  <p className="mt-1.5 truncate text-[14px] font-medium" style={{
              color: t.text
            }}>
                    {session.title}
                  </p>
                  <p className="mt-1 text-[11px]" style={{
              color: t.textMuted
            }}>
                    {session.focused ? 'Owns the deck · ' : ''}active {session.elapsed}
                  </p>
                </div> : <div className="rounded-xl p-4" style={{
            backgroundColor: t.sunken
          }}>
                  <p className="text-[13px]" style={{
              color: t.textSecondary
            }}>
                    No thread assigned — this key is unlit until a session fills the slot.
                  </p>
                </div>}
              <div className="flex items-start justify-between gap-4">
                <p className="text-[12px] leading-relaxed" style={{
              color: t.textSecondary
            }}>
                  Agent Keys follow your active threads automatically (key source: <span style={{
                color: t.text,
                fontWeight: 500
              }}>Focused app</span>).
                  Press switches the thread; double-press brings its window forward.
                </p>
                <button type="button" onClick={onOpenAgentKeys} className="shrink-0 rounded-lg px-3 py-1.5 text-[12px] font-medium transition-opacity hover:opacity-80" style={{
              backgroundColor: t.text,
              color: t.name === 'light' ? '#FFFFFF' : '#0D0D0D'
            }}>
                
                  Agent Keys settings
                </button>
              </div>
            </div>}

          {/* Dial */}
          {control.kind === 'knob' && <div className="flex flex-col gap-4">
              <div>
                <SectionLabel>Rotate</SectionLabel>
                <div className="flex flex-col gap-1.5">
                  {ROTATE_ACTIONS.map(r => <button key={r.id} type="button" onClick={() => setBindings(b => ({
                ...b,
                knobRotate: r.id
              }))} className="flex items-center justify-between rounded-lg px-3 py-2 text-left text-[13px] transition-colors" style={bindings.knobRotate === r.id ? {
                backgroundColor: t.sunken,
                color: t.text,
                fontWeight: 500
              } : {
                color: t.textSecondary
              }}>
                  
                      {r.label}
                      {bindings.knobRotate === r.id && <svg width="12" height="10" viewBox="0 0 12 10" fill="none">
                          <path d="M1 5L4.5 8.5L11 1" stroke="#3D7EFF" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                        </svg>}
                    </button>)}
                </div>
              </div>
              {bindings.knobRotate === 'reasoning_effort' && <div className="rounded-xl p-3" style={{
            backgroundColor: t.sunken
          }}>
                  <div className="flex items-center gap-1.5">
                    {REASONING_LEVELS.map((l, i) => <span key={l} className="rounded-full px-2 py-[3px] text-[11px] font-medium" style={i === 2 ? {
                backgroundColor: t.text,
                color: t.name === 'light' ? '#FFF' : '#0D0D0D'
              } : {
                color: t.textMuted
              }}>
                  
                        {l}
                      </span>)}
                  </div>
                  <p className="mt-2 text-[11px]" style={{
              color: t.textMuted
            }}>
                    Turn the dial to set the reasoning level for the focused thread.
                  </p>
                </div>}
              <ActionPicker label="Press" value={bindings.knobPress} onChange={id => setBindings(b => ({
            ...b,
            knobPress: id
          }))} />
            </div>}

          {/* Joystick */}
          {control.kind === 'joystick' && <div className="flex flex-col gap-4">
              <div>
                <SectionLabel>Flick to trigger skills</SectionLabel>
                <div className="flex flex-col gap-2">
                  {JOY_DIRS.map(d => <div key={d.id} className="flex items-center gap-3">
                      <span className="w-[72px] shrink-0 text-[12px]" style={{
                  color: t.textSecondary
                }}>
                        {d.label}
                      </span>
                      <div className="min-w-0 flex-1">
                        <ActionPicker compact value={bindings.joystick[d.id]} onChange={id => setBindings(b => ({
                    ...b,
                    joystick: {
                      ...b.joystick,
                      [d.id as JoyDir]: id
                    }
                  }))} />
                    
                      </div>
                    </div>)}
                </div>
              </div>
              <ActionPicker label="Press" value={bindings.joystickPress} onChange={id => setBindings(b => ({
            ...b,
            joystickPress: id
          }))} />
            </div>}

          {/* Touch sensor */}
          {control.kind === 'touch' && <div className="flex flex-col gap-4">
              <ActionPicker label="Tap" value={bindings.touch} onChange={id => setBindings(b => ({
            ...b,
            touch: id
          }))} />
              <p className="text-[12px] leading-relaxed" style={{
            color: t.textSecondary
          }}>
                The capacitive sensor next to the status LEDs. Default: pause LED updates without unplugging.
              </p>
            </div>}

          {/* Command keys */}
          {control.kind === 'command-key' && <div className="flex flex-col gap-4">
              <ActionPicker label="Action" value={bindings.commandKeys[selected]} onChange={id => setBindings(b => ({
            ...b,
            commandKeys: {
              ...b.commandKeys,
              [selected]: id
            }
          }))} />
              <div className="flex items-center gap-3 rounded-xl p-3" style={{
            backgroundColor: t.sunken
          }}>
                <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[9px] text-[#1D1D1B]" style={{
              background: 'radial-gradient(ellipse 90% 70% at 50% 38%, #FFFFFF 0%, #F6F6F3 55%, #ECECE8 100%)',
              border: '1px solid rgba(0,0,0,0.09)'
            }}>
                
                  {control.icon && <CapIcon icon={control.icon} size={15} />}
                </span>
                <p className="text-[11px] leading-relaxed" style={{
              color: t.textMuted
            }}>
                  Shipped cap shown. The Codex Icon Keyset includes 32 icon caps and 11 solid caps for re-capping remapped keys.
                </p>
              </div>
            </div>}
        </Card>
      </div>
    </div>;
};