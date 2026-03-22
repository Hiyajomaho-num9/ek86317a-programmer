import { useState } from 'react';
import { useAppContext } from '../../App';
import { gammaRegAddresses } from '../../lib/register-map';
import {
  decodeGamma, encodeGamma, gammaRegsToDacCode, dacCodeToGammaRegs,
  formatHex,
} from '../../lib/voltage-calc';

interface GammaRowProps {
  channel: number; // 1~14
  avdd: number;
}

function GammaRow({ channel, avdd }: GammaRowProps) {
  const ctx = useAppContext();
  const { high: addrH, low: addrL } = gammaRegAddresses(channel);

  // Current register values
  const highReg = ctx.getDacValue(addrH) ?? 0;
  const lowReg = ctx.getDacValue(addrL) ?? 0;

  // 10-bit DAC code and voltage
  const dacCode = gammaRegsToDacCode(highReg, lowReg);
  const voltage = decodeGamma(highReg, lowReg, avdd);

  // Local editing states for hex and voltage input
  const [hexInput, setHexInput] = useState<string | null>(null);
  const [voltInput, setVoltInput] = useState<string | null>(null);

  // ── Handlers ───────────────────────────────────────────────────

  const handleStep = (delta: number) => {
    const newCode = Math.max(0, Math.min(1023, dacCode + delta));
    const regs = dacCodeToGammaRegs(newCode);
    ctx.setDacValue(addrH, regs.high);
    ctx.setDacValue(addrL, regs.low);
  };

  const handleHexBlur = () => {
    if (hexInput === null) return;
    const parsed = parseInt(hexInput.replace(/^0x/i, ''), 16);
    if (!isNaN(parsed) && parsed >= 0 && parsed <= 0x3ff) {
      const regs = dacCodeToGammaRegs(parsed);
      ctx.setDacValue(addrH, regs.high);
      ctx.setDacValue(addrL, regs.low);
    }
    setHexInput(null);
  };

  const handleVoltBlur = () => {
    if (voltInput === null) return;
    const parsed = parseFloat(voltInput);
    if (!isNaN(parsed)) {
      const result = encodeGamma(parsed, avdd);
      if (result) {
        ctx.setDacValue(addrH, result.high);
        ctx.setDacValue(addrL, result.low);
      }
    }
    setVoltInput(null);
  };

  const handleRead = async () => {
    try {
      const h = await ctx.readDacRegister(addrH);
      const l = await ctx.readDacRegister(addrL);
      ctx.addLog('info', `Read GMA${channel}: H=${formatHex(h)}, L=${formatHex(l)}`);
    } catch (e: unknown) {
      ctx.addLog('error', `Read GMA${channel} failed: ${e}`);
    }
  };

  const handleWrite = async () => {
    try {
      await ctx.writeDacRegister(addrH, highReg);
      await ctx.writeDacRegister(addrL, lowReg);
      ctx.addLog('success', `Write GMA${channel}: H=${formatHex(highReg)}, L=${formatHex(lowReg)}`);
    } catch (e: unknown) {
      ctx.addLog('error', `Write GMA${channel} failed: ${e}`);
    }
  };

  const inputStyle = 'w-20 px-1.5 py-0.5 text-xs bg-gray-700 border border-gray-600 rounded text-gray-200 font-mono focus:border-blue-500 focus:outline-none text-center';
  const btnSmall = 'px-2 py-0.5 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';
  const btnStep = 'px-1.5 py-0.5 text-xs rounded text-white font-bold transition-colors bg-gray-600 hover:bg-gray-500 active:bg-gray-400';

  return (
    <div className="flex items-center gap-2 p-1.5 bg-gray-800 rounded hover:bg-gray-750">
      {/* Channel label */}
      <span className="w-14 text-xs font-medium text-gray-300 shrink-0">
        GMA{channel}
      </span>

      {/* Stepper buttons */}
      <button onClick={() => handleStep(-1)} className={btnStep} title="DAC -1">−</button>
      <button onClick={() => handleStep(1)} className={btnStep} title="DAC +1">+</button>

      {/* HEX input */}
      <span className="text-xs text-gray-500 shrink-0">HEX:</span>
      <input
        type="text"
        className={inputStyle}
        value={hexInput !== null ? hexInput : formatHex(dacCode, 4)}
        onChange={e => setHexInput(e.target.value)}
        onBlur={handleHexBlur}
        onKeyDown={e => { if (e.key === 'Enter') handleHexBlur(); }}
      />

      {/* Voltage input */}
      <span className="text-xs text-gray-500 shrink-0">V:</span>
      <input
        type="text"
        className={inputStyle}
        value={voltInput !== null ? voltInput : voltage.toFixed(3)}
        onChange={e => setVoltInput(e.target.value)}
        onBlur={handleVoltBlur}
        onKeyDown={e => { if (e.key === 'Enter') handleVoltBlur(); }}
      />

      {/* R / W buttons */}
      <button
        onClick={handleRead}
        className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
        disabled={!ctx.connected}
        title={`Read GMA${channel}`}
      >
        R
      </button>
      <button
        onClick={handleWrite}
        className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
        disabled={!ctx.connected}
        title={`Write GMA${channel}`}
      >
        W
      </button>
    </div>
  );
}

export default GammaRow;
