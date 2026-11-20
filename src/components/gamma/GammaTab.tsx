import { useDeviceContext } from '../../context/DeviceContext';
import { useRegisterValue, useRegisters } from '../../context/RegistersContext';
import { useLogContext } from '../../context/LogContext';
import { useOperations } from '../../context/OperationsContext';
import GammaRow from './GammaRow';
import { GAMMA_CHANNELS, REG_AVDD, getDefaultRegisterValue } from '../../lib/register-map';
import { decodeAvdd, formatVoltage } from '../../lib/voltage-calc';
import { readAllGamma, writeAllGamma } from '../../lib/gamma-operations';

function GammaTab() {
  const { chipModel, connected } = useDeviceContext();
  const registers = useRegisters();
  const { getDacValue, replaceDacRegisters } = registers;
  const { addLog } = useLogContext();
  const { busy, runExclusive } = useOperations();
  const mode = useRegisterValue(0x46) ?? getDefaultRegisterValue(chipModel, 0x46);
  const avdd = decodeAvdd(useRegisterValue(REG_AVDD) ?? getDefaultRegisterValue(chipModel, REG_AVDD), chipModel, mode);
  const handleReadAll = () => runExclusive('Read All GAMMA', async () => {
    try {
      await readAllGamma((values) => {
        const next = new Map(registers.store.getSnapshot('dac'));
        values.forEach(([address, value]) => next.set(address, value));
        replaceDacRegisters(next);
      });
      addLog('success', 'Read all GAMMA channels complete');
    } catch (error) { addLog('error', `Read all GAMMA failed: ${error}`); }
  });
  const handleWriteAll = () => runExclusive('Write All GAMMA', async () => {
    try {
      await writeAllGamma(getDacValue, undefined, chipModel);
      addLog('success', 'Write all GAMMA channels complete');
    } catch (error) { addLog('error', `Write all GAMMA failed: ${error}`); }
  });
  const btnSmall = 'px-3 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-gray-200">GAMMA Channels</h2>
          <div className="text-xs text-gray-400">
            <span>AVDD Reference: <span className="text-blue-400 font-mono">{formatVoltage(avdd)}</span>{' | '}Formula: GAMMA_V = AVDD × DAC_CODE / 1024</span><br />
            <span className="text-gray-500">When you input the panel GMA voltage, the tool will automatically recalculate the corresponding actual voltage.</span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={handleReadAll} className={`${btnSmall} bg-blue-600 hover:bg-blue-700`} disabled={!connected || busy !== null}>Read All</button>
          <button onClick={handleWriteAll} className={`${btnSmall} bg-orange-600 hover:bg-orange-700`} disabled={!connected || busy !== null}>Write All</button>
        </div>
      </div>
      <div className="space-y-0.5">
        {Array.from({ length: GAMMA_CHANNELS }, (_, index) => index + 1).map((channel) => <GammaRow key={channel} channel={channel} avdd={avdd} />)}
      </div>
    </div>
  );
}

export default GammaTab;
