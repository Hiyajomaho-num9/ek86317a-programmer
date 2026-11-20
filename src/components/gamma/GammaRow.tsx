import { memo, useState } from 'react';
import { useDeviceContext } from '../../context/DeviceContext';
import { useRegisterValue, useRegisters } from '../../context/RegistersContext';
import { useLogContext } from '../../context/LogContext';
import { useOperations } from '../../context/OperationsContext';
import { gammaRegAddresses, getDefaultRegisterValue } from '../../lib/register-map';
import { decodeGamma, gammaRegsToDacCode, dacCodeToGammaRegs, formatHex } from '../../lib/voltage-calc';

interface GammaRowProps { channel: number; avdd: number; }

function GammaRow({ channel, avdd }: GammaRowProps) {
  const { connected, chipModel } = useDeviceContext();
  const { readDacRegister, writeDacRegister, setDacValue } = useRegisters();
  const { addLog } = useLogContext();
  const { busy, runExclusive } = useOperations();
  const { high: addrH, low: addrL } = gammaRegAddresses(channel);
  const highReg = useRegisterValue(addrH) ?? getDefaultRegisterValue(chipModel, addrH);
  const lowReg = useRegisterValue(addrL) ?? getDefaultRegisterValue(chipModel, addrL);
  const dacCode = gammaRegsToDacCode(highReg, lowReg);
  const voltage = decodeGamma(highReg, lowReg, avdd);
  const [hexInput, setHexInput] = useState<string | null>(null);
  const [voltInput, setVoltInput] = useState<string | null>(null);

  const handleStep = (delta: number) => {
    const regs = dacCodeToGammaRegs(Math.max(0, Math.min(1023, dacCode + delta)));
    setDacValue(addrH, regs.high);
    setDacValue(addrL, regs.low);
  };
  const applyCode = (parsed: number, clear: () => void) => {
    if (!Number.isNaN(parsed)) {
      const code = Math.max(0, Math.min(1023, Math.round(parsed)));
      const regs = dacCodeToGammaRegs(code);
      setDacValue(addrH, regs.high);
      setDacValue(addrL, regs.low);
    }
    clear();
  };
  const handleHexBlur = () => {
    if (hexInput === null) return;
    applyCode(parseInt(hexInput.replace(/^0x/i, ''), 16), () => setHexInput(null));
  };
  const handleVoltBlur = () => {
    if (voltInput === null) return;
    applyCode((parseFloat(voltInput) / avdd) * 1024, () => setVoltInput(null));
  };
  const handleRead = async () => runExclusive(`Read GMA${channel}`, async () => {
    try {
      const high = await readDacRegister(addrH);
      const low = await readDacRegister(addrL);
      addLog('info', `Read GMA${channel}: ${formatHex(gammaRegsToDacCode(high, low), 4)} (${decodeGamma(high, low, avdd).toFixed(3)}V)`);
    } catch (error) { addLog('error', `Failed to read GMA${channel}: ${error}`); }
  });
  const handleWrite = async () => runExclusive(`Write GMA${channel}`, async () => {
    try {
      await writeDacRegister(addrH, highReg);
      await writeDacRegister(addrL, lowReg);
      addLog('info', `Write GMA${channel}: ${formatHex(dacCode, 4)} (${voltage.toFixed(3)}V)`);
    } catch (error) { addLog('error', `Failed to write GMA${channel}: ${error}`); }
  });

  const inputStyle = 'w-20 px-2 py-0.5 text-xs bg-gray-700 border border-gray-600 rounded text-gray-200 font-mono text-center focus:border-blue-500 focus:outline-none';
  const btnStep = 'w-6 h-6 text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 rounded';
  const btnSmall = 'px-2 py-0.5 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="flex items-center gap-2 px-3 py-1 bg-gray-800 rounded border border-gray-700 hover:border-gray-600">
      <span className="w-14 text-xs font-medium text-gray-300 shrink-0">GMA{channel}</span>
      <button onClick={() => handleStep(-1)} className={btnStep} title="DAC -1">−</button>
      <button onClick={() => handleStep(1)} className={btnStep} title="DAC +1">+</button>
      <span className="text-xs text-gray-500 shrink-0">HEX:</span>
      <input type="text" className={inputStyle} value={hexInput ?? formatHex(dacCode, 4)}
        onChange={(event) => setHexInput(event.target.value)} onBlur={handleHexBlur}
        onKeyDown={(event) => { if (event.key === 'Enter') handleHexBlur(); }} />
      <span className="text-xs text-gray-500 shrink-0">V:</span>
      <input type="text" className={inputStyle} value={voltInput ?? voltage.toFixed(3)}
        onChange={(event) => setVoltInput(event.target.value)} onBlur={handleVoltBlur}
        onKeyDown={(event) => { if (event.key === 'Enter') handleVoltBlur(); }} />
      <button onClick={handleRead} className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
        disabled={!connected || busy !== null} title={`Read GMA${channel}`}>R</button>
      <button onClick={handleWrite} className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
        disabled={!connected || busy !== null} title={`Write GMA${channel}`}>W</button>
    </div>
  );
}

export default memo(GammaRow);
