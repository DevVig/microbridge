import { useEffect, useRef, useState } from 'react';
import type { ActionGroup } from './microbridge-data';
import { ACTIONS, ACTION_GROUP_LABELS, getAction } from './microbridge-data';
import { useTheme } from './theme';
interface ActionPickerProps {
  value: string;
  onChange: (id: string) => void;
  label?: string;
  compact?: boolean;
}
const GROUPS: ActionGroup[] = ['AGENT', 'SKILL', 'SYSTEM', 'MACRO'];
export const ActionPicker = ({
  value,
  onChange,
  label,
  compact = false
}: ActionPickerProps) => {
  const {
    t
  } = useTheme();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const current = getAction(value);
  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, []);
  return <div ref={ref} className="relative min-w-0">
      {label && <span className="mb-1.5 block text-[12px] font-semibold" style={{
      color: t.text
    }}>
          {label}
        </span>}
      <button type="button" onClick={() => setOpen(o => !o)} aria-expanded={open} className={`flex w-full items-center justify-between gap-2 rounded-lg text-left text-[13px] transition-colors ${compact ? 'px-2.5 py-1.5' : 'px-3 py-2'}`} style={{
      backgroundColor: t.raised,
      border: `1px solid ${t.cardBorder}`,
      color: t.text
    }}>
        
        <span className="flex min-w-0 items-center gap-2">
          {!compact && <span className="shrink-0 rounded-[5px] px-1.5 py-0.5 text-[10px] font-medium" style={{
          backgroundColor: t.sunken,
          color: t.textSecondary
        }}>
              {ACTION_GROUP_LABELS[current.group]}
            </span>}
          <span className="truncate">{current.label}</span>
        </span>
        <svg width="10" height="6" viewBox="0 0 10 6" className={`shrink-0 transition-transform ${open ? 'rotate-180' : ''}`}>
          <path d="M1 1L5 5L9 1" stroke={t.textSecondary} strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
      {open && <div className="mb-scrollbar absolute left-0 right-0 top-[calc(100%+4px)] z-20 max-h-[240px] overflow-y-auto rounded-lg p-1.5 mb-frost" style={{
      backgroundColor: t.panel,
      border: `1px solid ${t.panelBorder}`,
      boxShadow: '0 12px 32px rgba(0,0,0,0.25)'
    }}>
        
          {GROUPS.map(group => <div key={group} className="mb-1 last:mb-0">
              <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.08em]" style={{
          color: t.textMuted
        }}>
                {ACTION_GROUP_LABELS[group]}
              </div>
              {ACTIONS.filter(a => a.group === group).map(a => <button key={a.id} type="button" onClick={() => {
          onChange(a.id);
          setOpen(false);
        }} className="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-[13px] transition-colors" style={a.id === value ? {
          backgroundColor: t.hoverBg,
          color: t.text,
          fontWeight: 500
        } : {
          color: t.text
        }} onMouseEnter={e => e.currentTarget.style.backgroundColor = t.hoverBg} onMouseLeave={e => e.currentTarget.style.backgroundColor = a.id === value ? t.hoverBg : 'transparent'}>
            
                  {a.label}
                  {a.id === value && <svg width="12" height="10" viewBox="0 0 12 10" fill="none">
                      <path d="M1 5L4.5 8.5L11 1" stroke="#3D7EFF" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>}
                </button>)}
            </div>)}
        </div>}
    </div>;
};