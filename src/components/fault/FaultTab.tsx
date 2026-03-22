/**
 * FaultTab — Fault Flag display panel (read-only).
 * Reads from VCOM slave 0x74, register 0x02 via readFaultFlags() Tauri command.
 * Displays 7 fault bits in a table with status indicators.
 */

import { useState } from 'react';
import { useAppContext } from '../../App';
import { readFaultFlags } from '../../lib/tauri-commands';
import type { FaultFlags } from '../../lib/tauri-commands';

// ============================================================================
// Fault bit definitions
// ============================================================================

interface FaultBitDef {
  bit: number;
  name: string;
  key: keyof Omit<FaultFlags, 'raw'>;
}

const FAULT_BITS: FaultBitDef[] = [
  { bit: 7, name: 'OTP',               key: 'otp' },
  { bit: 5, name: 'VBK1 UVP/SCP',      key: 'vbk1' },
  { bit: 4, name: 'AVDD UVP/SCP',      key: 'avdd' },
  { bit: 3, name: 'VGH UVP/SCP',       key: 'vgh' },
  { bit: 2, name: 'VGL UVP/SCP',       key: 'vgl' },
  { bit: 1, name: 'VSS1 UVP',          key: 'vss1' },
  { bit: 0, name: 'HAVDD UVP/SCP/OVP', key: 'havdd' },
];

// ============================================================================
// FaultTab Component
// ============================================================================

function FaultTab() {
  const ctx = useAppContext();
  const [faultData, setFaultData] = useState<FaultFlags | null>(null);
  const [loading, setLoading] = useState(false);

  const handleReadFault = async () => {
    setLoading(true);
    try {
      const result = await readFaultFlags();
      setFaultData(result);
      ctx.addLog('info', `Read fault flags: 0x${result.raw.toString(16).toUpperCase().padStart(2, '0')}`);
    } catch (e: unknown) {
      ctx.addLog('error', `Read fault flags failed: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const btnSmall = 'px-3 py-1.5 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold text-gray-200">Fault Status</h2>

      <div className="bg-gray-800 border border-gray-700 rounded-lg p-4 max-w-2xl">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <span className="text-sm font-bold text-blue-400">
            Fault Flag (Slave 0x74, Reg 0x02)
          </span>
          <button
            onClick={handleReadFault}
            className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
            disabled={!ctx.connected || loading}
          >
            {loading ? 'Reading...' : 'Read Fault'}
          </button>
        </div>

        {/* Fault table */}
        <div className="rounded overflow-hidden border border-gray-700">
          {/* Table header */}
          <div className="grid grid-cols-[50px_1fr_1fr] bg-gray-700 text-xs font-medium text-gray-300">
            <div className="px-3 py-2">Bit</div>
            <div className="px-3 py-2">Flag</div>
            <div className="px-3 py-2">Status</div>
          </div>

          {/* Table rows */}
          {FAULT_BITS.map((fb, index) => {
            const isFault = faultData ? faultData[fb.key] : false;
            const rowBg = index % 2 === 0 ? 'bg-gray-800' : 'bg-gray-800/60';

            return (
              <div
                key={fb.bit}
                className={`grid grid-cols-[50px_1fr_1fr] text-xs ${rowBg} border-t border-gray-700/50`}
              >
                <div className="px-3 py-2 text-gray-400 font-mono">b{fb.bit}</div>
                <div className="px-3 py-2 text-gray-300">{fb.name}</div>
                <div className="px-3 py-2">
                  {faultData === null ? (
                    <span className="text-gray-500">—</span>
                  ) : isFault ? (
                    <span className="text-red-400 font-medium">⚠ FAULT</span>
                  ) : (
                    <span className="text-green-400">● Normal</span>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        {/* Raw value + note */}
        <div className="mt-3 space-y-1">
          <div className="text-xs text-gray-400 font-mono">
            Raw Value:{' '}
            {faultData !== null
              ? `0x${faultData.raw.toString(16).toUpperCase().padStart(2, '0')}`
              : '—'}
          </div>
          <div className="text-xs text-gray-500 italic">
            Read only. Power cycle IC to clear faults.
          </div>
        </div>
      </div>
    </div>
  );
}

export default FaultTab;
