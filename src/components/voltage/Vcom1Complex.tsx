import { useMemo } from 'react';
import { useAppContext } from '../../App';
import {
  REG_VCOM_MAX,
  REG_VCOM_MIN,
  generateVcomLimitOptions,
  getDefaultRegisterValue,
  getPrimaryVcomAddress,
  getPrimaryVcomName,
} from '../../lib/register-map';
import { decodeVcom1Nt, decodeVcomLimit, formatHex, formatVoltage } from '../../lib/voltage-calc';

function Vcom1Complex() {
  const ctx = useAppContext();
  const avdd = ctx.getAvddVoltage();
  const primaryAddr = getPrimaryVcomAddress(ctx.chipModel);
  const primaryName = getPrimaryVcomName(ctx.chipModel);

  const vcomMinRaw = ctx.getDacValue(REG_VCOM_MIN) ?? getDefaultRegisterValue(ctx.chipModel, REG_VCOM_MIN);
  const vcomMaxRaw = ctx.getDacValue(REG_VCOM_MAX) ?? getDefaultRegisterValue(ctx.chipModel, REG_VCOM_MAX);
  const primaryRaw = ctx.getDacValue(primaryAddr) ?? getDefaultRegisterValue(ctx.chipModel, primaryAddr);

  const vcomMinV = decodeVcomLimit(vcomMinRaw, avdd);
  const vcomMaxV = decodeVcomLimit(vcomMaxRaw, avdd);
  const primaryV = decodeVcom1Nt(primaryRaw, vcomMinV, vcomMaxV);
  const dacCode = (primaryRaw >> 1) & 0x7f;

  const vcomLimitOptions = useMemo(() => generateVcomLimitOptions(avdd), [avdd]);

  const handleSliderChange = (newDacCode: number) => {
    ctx.setDacValue(primaryAddr, (newDacCode & 0x7f) << 1);
  };

  const handleRead = async (addr: number, name: string) => {
    try {
      const value = await ctx.readDacRegister(addr);
      ctx.addLog('info', `Read ${name} (${formatHex(addr)}): ${formatHex(value)}`);
    } catch (error: unknown) {
      ctx.addLog('error', `Read ${name} failed: ${error}`);
    }
  };

  const handleWrite = async (addr: number, name: string) => {
    const value = ctx.getDacValue(addr) ?? 0;
    try {
      await ctx.writeDacRegister(addr, value);
      ctx.addLog('success', `Write ${name} (${formatHex(addr)}): ${formatHex(value)}`);
    } catch (error: unknown) {
      ctx.addLog('error', `Write ${name} failed: ${error}`);
    }
  };

  const selectStyle = 'px-2 py-1 text-xs bg-gray-700 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';
  const btnSmall = 'px-2 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="p-3 bg-gray-800 rounded border border-gray-700">
      <div className="text-sm font-medium text-gray-200 mb-3">{primaryName} Range Control</div>

      <div className="mb-4">
        <div className="flex items-center justify-between mb-1">
          <span className="text-xs text-blue-400 font-mono">VCOM_MIN: {formatVoltage(vcomMinV)}</span>
          <span className="text-xs text-gray-400">
            {primaryName}: <span className="text-green-400 font-mono">{formatVoltage(primaryV)}</span>
          </span>
          <span className="text-xs text-blue-400 font-mono">VCOM_MAX: {formatVoltage(vcomMaxV)}</span>
        </div>

        <input
          type="range"
          min={0}
          max={127}
          value={dacCode}
          onChange={(event) => handleSliderChange(Number(event.target.value))}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
        />

        <div className="flex items-center justify-center gap-4 mt-1">
          <span className="text-xs text-gray-500 font-mono">HEX: {formatHex(primaryRaw)}</span>
          <span className="text-xs text-gray-500 font-mono">DAC: {dacCode}</span>
        </div>
      </div>

      <div className="space-y-2">
        <div className="flex items-center gap-3">
          <span className="w-28 text-xs text-gray-400 shrink-0">VCOM_MIN (0x0B):</span>
          <select
            className={`${selectStyle} min-w-[160px]`}
            value={vcomMinRaw}
            onChange={(event) => ctx.setDacValue(REG_VCOM_MIN, Number(event.target.value))}
          >
            {vcomLimitOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {formatHex(option.value)} = {option.label}
              </option>
            ))}
          </select>
          <button onClick={() => handleRead(REG_VCOM_MIN, 'VCOM_MIN')} className={`${btnSmall} bg-blue-600 hover:bg-blue-700`} disabled={ctx.connected === false}>R</button>
          <button onClick={() => handleWrite(REG_VCOM_MIN, 'VCOM_MIN')} className={`${btnSmall} bg-orange-600 hover:bg-orange-700`} disabled={ctx.connected === false}>W</button>
        </div>

        <div className="flex items-center gap-3">
          <span className="w-28 text-xs text-gray-400 shrink-0">VCOM_MAX (0x0A):</span>
          <select
            className={`${selectStyle} min-w-[160px]`}
            value={vcomMaxRaw}
            onChange={(event) => ctx.setDacValue(REG_VCOM_MAX, Number(event.target.value))}
          >
            {vcomLimitOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {formatHex(option.value)} = {option.label}
              </option>
            ))}
          </select>
          <button onClick={() => handleRead(REG_VCOM_MAX, 'VCOM_MAX')} className={`${btnSmall} bg-blue-600 hover:bg-blue-700`} disabled={ctx.connected === false}>R</button>
          <button onClick={() => handleWrite(REG_VCOM_MAX, 'VCOM_MAX')} className={`${btnSmall} bg-orange-600 hover:bg-orange-700`} disabled={ctx.connected === false}>W</button>
        </div>

        <div className="flex items-center gap-3">
          <span className="w-28 text-xs text-gray-400 shrink-0">{primaryName} ({formatHex(primaryAddr)}):</span>
          <span className="text-xs text-gray-300 font-mono min-w-[180px]">
            {formatVoltage(primaryV)} (DAC={dacCode}, {formatHex(primaryRaw)})
          </span>
          <button onClick={() => handleRead(primaryAddr, primaryName)} className={`${btnSmall} bg-blue-600 hover:bg-blue-700`} disabled={ctx.connected === false}>R</button>
          <button onClick={() => handleWrite(primaryAddr, primaryName)} className={`${btnSmall} bg-orange-600 hover:bg-orange-700`} disabled={ctx.connected === false}>W</button>
        </div>
      </div>
    </div>
  );
}

export default Vcom1Complex;
