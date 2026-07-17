import { useTheme } from './theme';
interface ToggleProps {
  checked: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
}
export const Toggle = ({
  checked,
  onChange,
  disabled
}: ToggleProps) => {
  const {
    t
  } = useTheme();
  return <button type="button" role="switch" aria-checked={checked} disabled={disabled} onClick={() => onChange(!checked)} className="relative h-[22px] w-[38px] shrink-0 rounded-full transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-40" style={{
    backgroundColor: checked ? '#30C463' : t.name === 'light' ? 'rgba(0,0,0,0.16)' : 'rgba(255,255,255,0.18)'
  }}>
      
      <span className="absolute top-[2px] h-[18px] w-[18px] rounded-full bg-white transition-all duration-150" style={{
      left: checked ? 18 : 2,
      boxShadow: '0 1px 3px rgba(0,0,0,0.25)'
    }} />
      
    </button>;
};