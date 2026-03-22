/**
 * RegisterPanel — Generic register configuration panel.
 * Displays a register with HEX input, [R]/[W] buttons, and child BitField components.
 *
 * Layout:
 * ┌─ title ──────────────────┐
 * │ [0xFF]           [R] [W] │  ← HEX value + read/write buttons
 * │                          │
 * │ (children area)          │  ← BitField components
 * └──────────────────────────┘
 */

import { useState } from 'react';
import { useAppContext } from '../../App';

interface RegisterPanelProps {
  title: string;              // e.g. "Channel On/Off Set (28h)"
  address: number;            // Register address
  children: React.ReactNode;  // BitField component list
  extraButtons?: React.ReactNode; // Optional extra buttons (e.g. "Enable All")
}

function RegisterPanel({ title, address, children, extraButtons }: RegisterPanelProps) {
  const ctx = useAppContext();
  const currentValue = ctx.getDacValue(address) ?? 0;

  // Local hex input state for direct editing
  const [hexInput, setHexInput] = useState<string | null>(null);

  const displayHex = hexInput !== null ? hexInput : `0x${currentValue.toString(16).toUpperCase().padStart(2, '0')}`;

  const handleHexChange = (text: string) => {
    setHexInput(text);
  };

  const handleHexBlur = () => {
    if (hexInput !== null) {
      const parsed = parseInt(hexInput.replace(/^0x/i, ''), 16);
      if (!isNaN(parsed) && parsed >= 0 && parsed <= 0xff) {
        ctx.setDacValue(address, parsed);
      }
      setHexInput(null);
    }
  };

  const handleHexKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleHexBlur();
    }
  };

  const handleRead = async () => {
    try {
      const val = await ctx.readDacRegister(address);
      ctx.addLog('info', `Read reg 0x${address.toString(16).toUpperCase()}: 0x${val.toString(16).toUpperCase().padStart(2, '0')}`);
    } catch (e: unknown) {
      ctx.addLog('error', `Read reg 0x${address.toString(16).toUpperCase()} failed: ${e}`);
    }
  };

  const handleWrite = async () => {
    try {
      await ctx.writeDacRegister(address, currentValue);
      ctx.addLog('success', `Write reg 0x${address.toString(16).toUpperCase()}: 0x${currentValue.toString(16).toUpperCase().padStart(2, '0')}`);
    } catch (e: unknown) {
      ctx.addLog('error', `Write reg 0x${address.toString(16).toUpperCase()} failed: ${e}`);
    }
  };

  const btnSmall = 'px-2 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="bg-gray-800 border border-gray-700 rounded-lg p-3 flex flex-col gap-2">
      {/* Header: title + hex input + R/W buttons */}
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-bold text-blue-400 truncate">{title}</span>
        <div className="flex items-center gap-1.5 shrink-0">
          <input
            type="text"
            className="w-14 px-1.5 py-0.5 text-xs bg-gray-700 border border-gray-600 rounded text-gray-200 font-mono text-center focus:border-blue-500 focus:outline-none"
            value={displayHex}
            onChange={(e) => handleHexChange(e.target.value)}
            onBlur={handleHexBlur}
            onKeyDown={handleHexKeyDown}
          />
          <button
            onClick={handleRead}
            className={`${btnSmall} bg-blue-600 hover:bg-blue-700`}
            disabled={!ctx.connected}
            title="Read register"
          >
            R
          </button>
          <button
            onClick={handleWrite}
            className={`${btnSmall} bg-orange-600 hover:bg-orange-700`}
            disabled={!ctx.connected}
            title="Write register"
          >
            W
          </button>
        </div>
      </div>

      {/* Children area — BitField components */}
      <div className="flex flex-col gap-0.5">
        {children}
      </div>

      {/* Extra buttons (optional) */}
      {extraButtons && (
        <div className="flex gap-2 pt-1 border-t border-gray-700">
          {extraButtons}
        </div>
      )}
    </div>
  );
}

export default RegisterPanel;
