/**
 * BitField — Generic bit-field component for register configuration.
 * Supports two modes:
 *   - checkbox (single bit): toggle on/off
 *   - dropdown (multi-bit): select from predefined options
 */

interface BitFieldProps {
  label: string;              // e.g. "b7: Group_B_EN"
  type: 'checkbox' | 'dropdown';
  // checkbox mode
  checked?: boolean;
  onCheckChange?: (checked: boolean) => void;
  // dropdown mode
  options?: { value: number; label: string }[];
  selectedValue?: number;
  onSelectChange?: (value: number) => void;
}

function BitField({ label, type, checked, onCheckChange, options, selectedValue, onSelectChange }: BitFieldProps) {
  if (type === 'checkbox') {
    return (
      <label className="flex items-center gap-2 py-0.5 cursor-pointer hover:bg-gray-700/50 rounded px-1">
        <input
          type="checkbox"
          checked={checked ?? false}
          onChange={(e) => onCheckChange?.(e.target.checked)}
          className="w-3.5 h-3.5 rounded bg-gray-700 border-gray-500 text-blue-500 accent-blue-500 focus:ring-0 cursor-pointer"
        />
        <span className="text-xs text-gray-300">{label}</span>
      </label>
    );
  }

  // dropdown mode
  return (
    <div className="flex items-center gap-2 py-0.5 px-1">
      <span className="text-xs text-gray-300 shrink-0 min-w-[80px]">{label}</span>
      <select
        className="flex-1 px-2 py-0.5 text-xs bg-gray-700 text-gray-200 border border-gray-600 rounded focus:border-blue-500 focus:outline-none cursor-pointer"
        value={selectedValue ?? 0}
        onChange={(e) => onSelectChange?.(Number(e.target.value))}
      >
        {(options ?? []).map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

export default BitField;
