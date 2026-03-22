import { useMemo } from 'react';
import { useAppContext } from '../../App';
import {
  DEFAULT_VCOM_MAX_REG_VALUE,
  DEFAULT_VCOM_MIN_REG_VALUE,
  DEFAULT_VCOM1_NT_REG_VALUE,
  REG_VCOM1_NT, REG_VCOM_MAX, REG_VCOM_MIN,
  generateVcomLimitOptions,
} from '../../lib/register-map';
import {
  decodeVcomLimit, decodeVcom1Nt,
  formatVoltage, formatHex,
} from '../../lib/voltage-calc';

/**
 * VCOM1_NT complex component with slider and VCOM_MIN/VCOM_MAX controls.
 *
 * Layout:
 *   VCOM_MIN ◄━━━━━━━━●━━━━━━━► VCOM_MAX
 *   [slider: DAC_CODE 0~127]
 *   VCOM1_NT: voltage  HEX  DAC
 *   VCOM_MIN(0x0B): [dropdown]  [R] [W]
 *   VCOM_MAX(0x0A): [dropdown]  [R] [W]
 *   VCOM1_NT(0x08):              [R] [W]
 */
function Vcom1Complex() {
  const ctx = useAppContext();
  const avdd = ctx.getAvddVoltage();

  // Register raw values
  const vcomMinRaw = ctx.getDacValue(REG_VCOM_MIN) ?? DEFAULT_VCOM_MIN_REG_VALUE;
  const vcomMaxRaw = ctx.getDacValue(REG_VCOM_MAX) ?? DEFAULT_VCOM_MAX_REG_VALUE;
  const vcom1NtRaw = ctx.getDacValue(REG_VCOM1_NT) ?? DEFAULT_VCOM1_NT_REG_VALUE;

  // Decoded voltages
  const vcomMinV = decodeVcomLimit(vcomMinRaw, avdd);
  const vcomMaxV = decodeVcomLimit(vcomMaxRaw, avdd);
  const vcom1NtV = decodeVcom1Nt(vcom1NtRaw, vcomMinV, vcomMaxV);

  // DAC code for slider (bits [7:1])
  const dacCode = (vcom1NtRaw >> 1) & 0x7f;

  // Generate options for VCOM_MIN and VCOM_MAX dropdowns (depends on AVDD)
  const vcomLimitOptions = useMemo(() => generateVcomLimitOptions(avdd), [avdd]);

  // ── Handlers ───────────────────────────────────────────────────

  const handleSliderChange = (newDacCode: number) => {
    const newReg = (newDacCode & 0x7f) << 1;
    ctx.setDacValue(REG_VCOM1_NT, newReg);
  };

  const handleVcomMinChange = (newValue: number) => {
    ctx.setDacValue(REG_VCOM_MIN, newValue);
  };

  const handleVcomMaxChange = (newValue: number) => {
    ctx.setDacValue(REG_VCOM_MAX, newValue);
  };

  const handleRead = async (addr: number, name: string) => {
    try {
      const val = await ctx.readDacRegister(addr);
      ctx.addLog('info', `Read ${name} (${formatHex(addr)}): ${formatHex(val)}`);
    } catch (e: unknown) {
      ctx.addLog('error', `Read ${name} failed: ${e}`);
    }
  };

  const handleWrite = async (addr: number, name: string) => {
    const val = ctx.getDacValue(addr) ?? 0;
    try {
      await ctx.writeDacRegister(addr, val);
      ctx.addLog('success', `Write ${name} (${formatHex(addr)}): ${formatHex(val)}`);
    } catch (e: unknown) {
      ctx.addLog('error', `Write ${name} failed: ${e}`);
    }
  };

  const selectStyle = 'px-2 py-1 text-xs bg-gray-700 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';
  const btnSmall = 'px-2 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="p-3 bg-gray-800 rounded border border-gray-700">
      <div className="text-sm font-medium text-gray-200 mb-3">
        VCOM1 Normal Temperature
      </div>

      {/* Slider Section */}
      <div className="mb-4">
        {/* Labels above slider */}
        <div className="flex items-center justify-between mb-1">
          <span className="text-xs text-blue-400 font-mono">
            VCOM_MIN: {formatVoltage(vcomMinV)}
          </span>
          <span className="text-xs text-gray-400">
            VCOM1_NT: <span className="text-green-400 font-mono">{formatVoltage(vcom1NtV)}</span>
          </span>
          <span className="text-xs text-blue-400 font-mono">
            VCOM_MAX: {formatVoltage(vcomMaxV)}
          </span>
        </div>

        {/* Range slider */}
        <input
          type="range"
          min={0}
          max={127}
          value={dacCode}
          onChange={e => handleSliderChange(Number(e.target.value))}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
        />

        {/* DAC info below slider */}
        <div className="flex items-center justify-center gap-4 mt-1">
          <span className="text-xs text-gray-500 font-mono">
            HEX: {formatHex(vcom1NtRaw)}
          </span>
          <span className="text-xs text-gray-500 font-mono">
            DAC: {dacCode}
          </span>
        </div>
      </div>

      {/* VCOM_MIN / VCOM_MAX / VCOM1_NT Controls */}
      <div className="space-y-2">
        {/* VCOM_MIN */}
        <div className="flex items-center gap-3">
          <span className="w-28 text-xs text-gray-400 shrink-0">VCOM_MIN (0x0B):</span>
          <select
            className={`${selectStyle} min-w-[160px]`}
            value={vcomMinRaw}
            onChange={e => handleVcomMinChange(Number(e.target.value))}
          >
            {vcomLimitOptions.map(opt => (
              <option key={opt.value} value={opt.value}>
                {formatHex(opt.value)} = {opt.label}
              </option>
            ))}
          </select>
          <button
            onClick={() => handleRead(REG_VCOM_MIN, 'VCOM_MIN')}
            className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
            disabled={!ctx.connected}
          >
            R
          </button>
          <button
            onClick={() => handleWrite(REG_VCOM_MIN, 'VCOM_MIN')}
            className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
            disabled={!ctx.connected}
          >
            W
          </button>
        </div>

        {/* VCOM_MAX */}
        <div className="flex items-center gap-3">
          <span className="w-28 text-xs text-gray-400 shrink-0">VCOM_MAX (0x0A):</span>
          <select
            className={`${selectStyle} min-w-[160px]`}
            value={vcomMaxRaw}
            onChange={e => handleVcomMaxChange(Number(e.target.value))}
          >
            {vcomLimitOptions.map(opt => (
              <option key={opt.value} value={opt.value}>
                {formatHex(opt.value)} = {opt.label}
              </option>
            ))}
          </select>
          <button
            onClick={() => handleRead(REG_VCOM_MAX, 'VCOM_MAX')}
            className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
            disabled={!ctx.connected}
          >
            R
          </button>
          <button
            onClick={() => handleWrite(REG_VCOM_MAX, 'VCOM_MAX')}
            className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
            disabled={!ctx.connected}
          >
            W
          </button>
        </div>

        {/* VCOM1_NT */}
        <div className="flex items-center gap-3">
          <span className="w-28 text-xs text-gray-400 shrink-0">VCOM1_NT (0x08):</span>
          <span className="text-xs text-gray-300 font-mono min-w-[160px]">
            {formatVoltage(vcom1NtV)} (DAC={dacCode}, {formatHex(vcom1NtRaw)})
          </span>
          <button
            onClick={() => handleRead(REG_VCOM1_NT, 'VCOM1_NT')}
            className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
            disabled={!ctx.connected}
          >
            R
          </button>
          <button
            onClick={() => handleWrite(REG_VCOM1_NT, 'VCOM1_NT')}
            className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
            disabled={!ctx.connected}
          >
            W
          </button>
        </div>
      </div>
    </div>
  );
}

export default Vcom1Complex;
