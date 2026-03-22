import { useAppContext } from '../../App';
import { formatHex } from '../../lib/voltage-calc';
import type { VoltageOption, VoltageRegisterDef } from '../../lib/register-map';

interface VoltageRowProps {
  reg: VoltageRegisterDef;
  options: VoltageOption[];
}

function VoltageRow({ reg, options }: VoltageRowProps) {
  const ctx = useAppContext();

  // Current register raw value from DAC state
  const rawValue = ctx.getDacValue(reg.address) ?? 0;

  // Extract the voltage-relevant portion of the value
  const voltageValue = rawValue & reg.valueMask;

  // Enable bit (if applicable) — may be in a separate register (enableAddress)
  const hasEnable = reg.hasEnable && reg.enableBit !== undefined;
  const enableAddr = reg.enableAddress ?? reg.address;
  const enableRaw = reg.enableAddress ? (ctx.getDacValue(reg.enableAddress) ?? 0) : rawValue;
  const enableChecked = hasEnable ? ((enableRaw >> reg.enableBit!) & 1) === 1 : false;

  // Select bit (if applicable, e.g., VGL_LT_HT bit6=LT/HT)
  const hasSelect = reg.hasSelect && reg.selectBit !== undefined;
  const selectChecked = hasSelect ? ((rawValue >> reg.selectBit!) & 1) === 1 : false;


  const handleVoltageChange = (newVoltageCode: number) => {
    // Preserve enable/select bits, update voltage bits
    let newRaw = newVoltageCode & reg.valueMask;
    // Only preserve enable bit in same register (not when enableAddress is separate)
    if (hasEnable && enableChecked && !reg.enableAddress) {
      newRaw |= (1 << reg.enableBit!);
    }
    if (hasSelect && selectChecked) {
      newRaw |= (1 << reg.selectBit!);
    }
    ctx.setDacValue(reg.address, newRaw);
  };

  const handleEnableChange = (checked: boolean) => {
    let newRaw = enableRaw;
    if (checked) {
      newRaw |= (1 << reg.enableBit!);
    } else {
      newRaw &= ~(1 << reg.enableBit!);
    }
    ctx.setDacValue(enableAddr, newRaw);
  };

  const handleSelectChange = (checked: boolean) => {
    let newRaw = rawValue;
    if (checked) {
      newRaw |= (1 << reg.selectBit!);
    } else {
      newRaw &= ~(1 << reg.selectBit!);
    }
    ctx.setDacValue(reg.address, newRaw);
  };

  const handleRead = async () => {
    try {
      const val = await ctx.readDacRegister(reg.address);
      ctx.addLog('info', `Read ${reg.name} (${formatHex(reg.address)}): ${formatHex(val)}`);
      if (reg.enableAddress) {
        const enVal = await ctx.readDacRegister(reg.enableAddress);
        ctx.addLog('info', `Read ${reg.name}_EN (${formatHex(reg.enableAddress)}): ${formatHex(enVal)}`);
      }
    } catch (e: unknown) {
      ctx.addLog('error', `Read ${reg.name} failed: ${e}`);
    }
  };

  const handleWrite = async () => {
    try {
      await ctx.writeDacRegister(reg.address, rawValue);
      ctx.addLog('success', `Write ${reg.name} (${formatHex(reg.address)}): ${formatHex(rawValue)}`);
      if (reg.enableAddress) {
        await ctx.writeDacRegister(reg.enableAddress, enableRaw);
        ctx.addLog('success', `Write ${reg.name}_EN (${formatHex(reg.enableAddress)}): ${formatHex(enableRaw)}`);
      }
    } catch (e: unknown) {
      ctx.addLog('error', `Write ${reg.name} failed: ${e}`);
    }
  };

  const selectStyle = 'px-2 py-1 text-xs bg-gray-700 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';
  const btnSmall = 'px-2 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="flex items-center gap-3 p-2 bg-gray-800 rounded hover:bg-gray-750">
      {/* Register Name */}
      <span className="w-24 text-sm font-medium text-gray-300 shrink-0">{reg.name}</span>
      <span className="w-12 text-xs text-gray-500 font-mono shrink-0">({formatHex(reg.address)})</span>

      {/* Enable checkbox (if applicable) */}
      {hasEnable && (
        <label className="flex items-center gap-1 shrink-0">
          <input
            type="checkbox"
            checked={enableChecked}
            onChange={e => handleEnableChange(e.target.checked)}
            className="w-3 h-3 rounded bg-gray-700 border-gray-500 text-blue-500 focus:ring-0"
          />
          <span className="text-xs text-gray-400">EN</span>
        </label>
      )}

      {/* Select checkbox (if applicable, e.g., LT/HT) */}
      {hasSelect && (
        <label className="flex items-center gap-1 shrink-0">
          <input
            type="checkbox"
            checked={selectChecked}
            onChange={e => handleSelectChange(e.target.checked)}
            className="w-3 h-3 rounded bg-gray-700 border-gray-500 text-blue-500 focus:ring-0"
          />
          <span className="text-xs text-gray-400">{selectChecked ? 'HT' : 'LT'}</span>
        </label>
      )}

      {/* Voltage dropdown */}
      <select
        className={`${selectStyle} min-w-[120px]`}
        value={voltageValue}
        onChange={e => handleVoltageChange(Number(e.target.value))}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>

      {/* HEX display */}
      <span className="text-xs text-gray-500 font-mono shrink-0 w-14 text-right">
        HEX: {formatHex(rawValue)}
      </span>

      {/* Read / Write buttons */}
      <button
        onClick={handleRead}
        className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
        disabled={!ctx.connected}
        title={`Read ${reg.name}`}
      >
        R
      </button>
      <button
        onClick={handleWrite}
        className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
        disabled={!ctx.connected}
        title={`Write ${reg.name}`}
      >
        W
      </button>

      {/* Description tooltip area */}
      <span className="text-xs text-gray-600 truncate hidden lg:inline" title={reg.description}>
        {reg.description}
      </span>
    </div>
  );
}

export default VoltageRow;
