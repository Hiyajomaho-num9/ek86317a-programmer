import { useAppContext } from '../../App';
import GammaRow from './GammaRow';
import { GAMMA_CHANNELS, gammaRegAddresses } from '../../lib/register-map';
import { formatVoltage } from '../../lib/voltage-calc';

function GammaTab() {
  const ctx = useAppContext();
  const avdd = ctx.getAvddVoltage();

  /** Read all 14 GAMMA channels (28 registers). */
  const handleReadAll = async () => {
    try {
      for (let ch = 1; ch <= GAMMA_CHANNELS; ch++) {
        const { high, low } = gammaRegAddresses(ch);
        await ctx.readDacRegister(high);
        await ctx.readDacRegister(low);
      }
      ctx.addLog('success', 'Read all GAMMA channels complete');
    } catch (e: unknown) {
      ctx.addLog('error', `Read all GAMMA failed: ${e}`);
    }
  };

  /** Write all 14 GAMMA channels. */
  const handleWriteAll = async () => {
    try {
      for (let ch = 1; ch <= GAMMA_CHANNELS; ch++) {
        const { high: addrH, low: addrL } = gammaRegAddresses(ch);
        const hVal = ctx.getDacValue(addrH) ?? 0;
        const lVal = ctx.getDacValue(addrL) ?? 0;
        await ctx.writeDacRegister(addrH, hVal);
        await ctx.writeDacRegister(addrL, lVal);
      }
      ctx.addLog('success', 'Write all GAMMA channels complete');
    } catch (e: unknown) {
      ctx.addLog('error', `Write all GAMMA failed: ${e}`);
    }
  };

  const btnSmall = 'px-3 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-gray-200">GAMMA Channels</h2>
          <div className="text-xs text-gray-400">
            <span>
              AVDD Reference: <span className="text-blue-400 font-mono">{formatVoltage(avdd)}</span>
              {' | '}Formula: GAMMA_V = AVDD × DAC_CODE / 1024
            </span>
            <br />
            <span className="text-gray-500">
              When you input the panel GMA voltage, the tool will automatically recalculate the corresponding actual voltage.
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleReadAll}
            className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
            disabled={!ctx.connected}
          >
            Read All
          </button>
          <button
            onClick={handleWriteAll}
            className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
            disabled={!ctx.connected}
          >
            Write All
          </button>
        </div>
      </div>

      {/* GAMMA Rows */}
      <div className="space-y-0.5">
        {Array.from({ length: GAMMA_CHANNELS }, (_, i) => i + 1).map(ch => (
          <GammaRow key={ch} channel={ch} avdd={avdd} />
        ))}
      </div>
    </div>
  );
}

export default GammaTab;
