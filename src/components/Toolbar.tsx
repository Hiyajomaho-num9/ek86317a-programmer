import { useState, useEffect } from 'react';
import { useAppContext } from '../App';
import { CLOCK_OPTIONS } from '../lib/register-map';
import * as cmd from '../lib/tauri-commands';

/** Sanitize panel name for use in filename — replace illegal chars with _ */
function sanitizePanelName(name: string): string {
  return name.replace(/[\\/:*?"<>|]/g, '_').trim();
}

/** Generate timestamp string: YYYYMMDD_HHmmss */
function timestamp(): string {
  const d = new Date();
  const pad = (n: number) => n.toString().padStart(2, '0');
  return `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}_${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`;
}

const LS_EXPORT_PATH = 'ek86317a_exportPath';
const LS_PANEL_NAME = 'ek86317a_panelName';

function Toolbar() {
  const ctx = useAppContext();
  const [selectedDevice, setSelectedDevice] = useState('');
  const [clockHz, setClockHz] = useState(400_000);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [firmwarePath, setFirmwarePath] = useState('');
  const [firmwareName, setFirmwareName] = useState('');
  const [exportPath, setExportPath] = useState(() => localStorage.getItem(LS_EXPORT_PATH) ?? '');
  const [panelName, setPanelName] = useState(() => localStorage.getItem(LS_PANEL_NAME) ?? '');

  // Persist export path & panel name
  useEffect(() => { localStorage.setItem(LS_EXPORT_PATH, exportPath); }, [exportPath]);
  useEffect(() => { localStorage.setItem(LS_PANEL_NAME, panelName); }, [panelName]);

  const runToolbarAction = async (label: string, task: () => Promise<void>) => {
    if (busyAction) return;

    setBusyAction(label);
    try {
      await task();
    } finally {
      setBusyAction(null);
    }
  };

  // ── Row 1: Device Connection ───────────────────────────────────

  const handleScan = async () => {
    await runToolbarAction('Scanning devices...', async () => {
      try {
        const devs = await ctx.scan();
        ctx.addLog('info', `Scanned ${devs.length} device(s)`);
        if (selectedDevice && !devs.includes(selectedDevice)) {
          setSelectedDevice('');
          ctx.addLog('warn', 'Previously selected device is no longer present; please select again');
        }
      } catch (e: unknown) {
        ctx.addLog('error', `Scan failed: ${e}`);
      }
    });
  };

  const handleConnect = async () => {
    if (!selectedDevice) {
      ctx.addLog('warn', 'No device selected');
      return;
    }
    await runToolbarAction('Connecting device...', async () => {
      try {
        const info = await ctx.connect(selectedDevice, clockHz);
        ctx.resetRegisters();
        ctx.addLog('success', `Connected to ${selectedDevice}`);
        ctx.addLog('info', `PMIC: ${info.pmic_detected ? '✓' : '✗'} | VCOM: ${info.vcom_detected ? '✓' : '✗'}`);
      } catch (e: unknown) {
        ctx.addLog('error', `Connect failed: ${e}`);
      }
    });
  };

  const handleDisconnect = async () => {
    await runToolbarAction('Disconnecting device...', async () => {
      try {
        await ctx.disconnect();
        ctx.resetRegisters();
        ctx.addLog('info', 'Disconnected');
      } catch (e: unknown) {
        ctx.addLog('error', `Disconnect failed: ${e}`);
      }
    });
  };

  const handleDetect = async () => {
    await runToolbarAction('Detecting IC...', async () => {
      try {
        const info = await ctx.detect();
        ctx.addLog('info', `Detect: PMIC: ${info.pmic_detected ? '✓' : '✗'} | VCOM: ${info.vcom_detected ? '✓' : '✗'}`);
      } catch (e: unknown) {
        ctx.addLog('error', `Detect failed: ${e}`);
      }
    });
  };

  // ── Row 2: Firmware Operations ─────────────────────────────────

  const handleBrowse = async () => {
    await runToolbarAction('Loading firmware...', async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const result = await open({
          multiple: false,
          filters: [{ name: 'Firmware', extensions: ['bin'] }],
        });
        if (result) {
          const path = typeof result === 'string' ? result : String(result);
          setFirmwarePath(path);
          setFirmwareName(path.split(/[\\/]/).pop() ?? '');
          const preview = await cmd.loadFirmware(path);
          ctx.replaceDacRegisters(preview.registers.map((reg) => [reg.address, reg.value] as [number, number]));
          ctx.addLog('success', `Loaded ${preview.file_name}: ${preview.register_count} registers → UI`);
        }
      } catch (e: unknown) {
        ctx.addLog('error', `Browse/Load failed: ${e}`);
      }
    });
  };

  const handleFwWriteDac = async () => {
    if (!firmwarePath) {
      ctx.addLog('warn', 'No firmware file loaded');
      return;
    }
    await runToolbarAction('Writing firmware to DAC...', async () => {
      try {
        const result = await cmd.programFirmware(firmwarePath, false);
        if (result.success) {
          ctx.addLog('success', `Write DAC complete: ${result.registers_written} registers`);
        }
      } catch (e: unknown) {
        ctx.addLog('error', `Write DAC failed: ${e}`);
      }
    });
  };

  const handleFwWriteEeprom = async () => {
    if (!firmwarePath) {
      ctx.addLog('warn', 'No firmware file loaded');
      return;
    }
    await runToolbarAction('Writing firmware to EEPROM...', async () => {
      try {
        const result = await cmd.programFirmware(firmwarePath, true);
        if (result.success) {
          ctx.addLog('success', `Write EEPROM complete: ${result.registers_written} registers written, EEPROM burned`);
        }
      } catch (e: unknown) {
        ctx.addLog('error', `Write EEPROM failed: ${e}`);
      }
    });
  };

  const handleVerifyAll = async () => {
    if (!firmwarePath) {
      ctx.addLog('warn', 'No firmware file loaded');
      return;
    }
    await runToolbarAction('Verifying DAC and EEPROM...', async () => {
      try {
        const r = await cmd.verifyAll(firmwarePath);
        if (r.dac_mismatches.length === 0) {
          ctx.addLog('success', `Verify DAC: PASS (${r.dac_matched}/${r.total})`);
        } else {
          ctx.addLog('error', `Verify DAC: FAIL — ${r.dac_mismatches.length} mismatches`);
          for (const [addr, expected, actual] of r.dac_mismatches) {
            ctx.addLog('error', `  DAC 0x${addr.toString(16).toUpperCase().padStart(2, '0')}: expected 0x${expected.toString(16).toUpperCase().padStart(2, '0')}, got 0x${actual.toString(16).toUpperCase().padStart(2, '0')}`);
          }
        }
        if (r.eeprom_mismatches.length === 0) {
          ctx.addLog('success', `Verify EEPROM: PASS (${r.eeprom_matched}/${r.total})`);
        } else {
          ctx.addLog('error', `Verify EEPROM: FAIL — ${r.eeprom_mismatches.length} mismatches`);
          for (const [addr, expected, actual] of r.eeprom_mismatches) {
            ctx.addLog('error', `  EEPROM 0x${addr.toString(16).toUpperCase().padStart(2, '0')}: expected 0x${expected.toString(16).toUpperCase().padStart(2, '0')}, got 0x${actual.toString(16).toUpperCase().padStart(2, '0')}`);
          }
        }
      } catch (e: unknown) {
        ctx.addLog('error', `Verify ALL failed: ${e}`);
      }
    });
  };

  // ── Row 3: UI Operations ───────────────────────────────────────

  const handleReadDacToUi = async () => {
    await runToolbarAction('Reading DAC into UI...', async () => {
      try {
        await ctx.readAllDac();
        ctx.addLog('success', 'Read DAC → UI complete');
      } catch (e: unknown) {
        ctx.addLog('error', `Read DAC → UI failed: ${e}`);
      }
    });
  };

  const handleReadEepromToUi = async () => {
    await runToolbarAction('Reading EEPROM into UI...', async () => {
      try {
        await ctx.readAllEeprom();
        ctx.addLog('success', 'Read EEPROM → UI complete');
      } catch (e: unknown) {
        ctx.addLog('error', `Read EEPROM → UI failed: ${e}`);
      }
    });
  };

  const handleWriteUiToDac = async () => {
    await runToolbarAction('Writing UI registers to DAC...', async () => {
      try {
        const regs: [number, number][] = [];
        ctx.dacRegisters.forEach((value, addr) => {
          regs.push([addr, value]);
        });
        if (regs.length === 0) {
          ctx.addLog('warn', 'No UI register data to write');
          return;
        }
        const result = await cmd.writeAllDacRegisters(regs);
        ctx.addLog('success', `Write UI → DAC complete: ${result.registers_written} registers`);
      } catch (e: unknown) {
        ctx.addLog('error', `Write UI → DAC failed: ${e}`);
      }
    });
  };

  const handleWriteDacToEeprom = async () => {
    await runToolbarAction('Writing DAC to EEPROM...', async () => {
      try {
        await cmd.writeAllToEeprom();
        ctx.addLog('success', 'Write DAC → EEPROM complete');
      } catch (e: unknown) {
        ctx.addLog('error', `Write DAC → EEPROM failed: ${e}`);
      }
    });
  };

  // ── Row 4: Export ──────────────────────────────────────────────

  const handleChangeExportPath = async () => {
    await runToolbarAction('Selecting export path...', async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const result = await open({ directory: true });
        if (result) {
          const path = typeof result === 'string' ? result : String(result);
          setExportPath(path);
          ctx.addLog('info', `Export path set: ${path}`);
        }
      } catch (e: unknown) {
        ctx.addLog('error', `Set export path failed: ${e}`);
      }
    });
  };

  const handleExportBin = async () => {
    const name = sanitizePanelName(panelName);
    if (!name) {
      ctx.addLog('warn', 'Please enter a Panel Name');
      return;
    }

    let dir = exportPath;
    if (!dir) {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const result = await open({ directory: true });
        if (result) {
          dir = typeof result === 'string' ? result : String(result);
          setExportPath(dir);
        } else {
          return;
        }
      } catch (e: unknown) {
        ctx.addLog('error', `Set export path failed: ${e}`);
        return;
      }
    }

    const fileName = `PMU_${name}_${timestamp()}.bin`;
    const sep = dir.includes('\\') ? '\\' : '/';
    const fullPath = dir.endsWith(sep) ? `${dir}${fileName}` : `${dir}${sep}${fileName}`;

    await runToolbarAction('Exporting EEPROM to BIN...', async () => {
      try {
        await cmd.exportEeprom(fullPath);
        ctx.addLog('success', `Exported: ${fullPath}`);
      } catch (e: unknown) {
        ctx.addLog('error', `Export failed: ${e}`);
      }
    });
  };

  // ── Render ─────────────────────────────────────────────────────

  const btnBase = 'px-3 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';
  const btnBlue = `${btnBase} bg-blue-600 hover:bg-blue-700`;
  const btnGreen = `${btnBase} bg-green-700 hover:bg-green-600`;
  const btnRed = `${btnBase} bg-red-700 hover:bg-red-600`;
  const btnPurple = `${btnBase} bg-purple-700 hover:bg-purple-600`;
  const btnOrange = `${btnBase} bg-orange-700 hover:bg-orange-600`;
  const btnCyan = `${btnBase} bg-cyan-700 hover:bg-cyan-600`;
  const toolbarBusy = busyAction !== null;
  const selectStyle = 'px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';
  const inputStyle = 'px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';

  return (
    <div className="bg-gray-800 border-b border-gray-700 px-4 py-2 space-y-1.5">
      {/* Row 1: Device Connection + Status + Detect */}
      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">FT232H:</span>
        <select
          className={`${selectStyle} min-w-[180px]`}
          value={selectedDevice}
          onChange={e => setSelectedDevice(e.target.value)}
          disabled={ctx.connected || toolbarBusy}
        >
          <option value="">Select Device</option>
          {ctx.devices.map((d, i) => (
            <option key={i} value={d}>{d}</option>
          ))}
        </select>
        <button onClick={handleScan} className={btnBlue} disabled={ctx.connected || ctx.scanning || toolbarBusy}>
          {ctx.scanning || busyAction === 'Scanning devices...' ? 'Scanning...' : 'Scan'}
        </button>

        <span className="text-xs text-gray-400 font-medium">SCL:</span>
        <select
          className={selectStyle}
          value={clockHz}
          onChange={e => setClockHz(Number(e.target.value))}
          disabled={ctx.connected || toolbarBusy}
        >
          {CLOCK_OPTIONS.map(opt => (
            <option key={opt.value} value={opt.value}>{opt.label}</option>
          ))}
        </select>

        {ctx.connected ? (
          <button onClick={handleDisconnect} className={btnRed} disabled={toolbarBusy}>Disconnect</button>
        ) : (
          <button onClick={handleConnect} className={btnGreen} disabled={!selectedDevice || toolbarBusy}>Connect</button>
        )}

        <div className="flex items-center gap-2 ml-2">
          <span className="text-xs text-gray-400">Status:</span>
          <span className={`inline-block w-2 h-2 rounded-full ${ctx.connected ? 'bg-green-500' : 'bg-red-500'}`} />
          <span className={`text-xs ${ctx.connected ? 'text-green-400' : 'text-gray-500'}`}>
            {ctx.connected ? 'Connected' : 'Disconnected'}
          </span>
          {ctx.deviceInfo && (
            <>
              <span className="text-gray-600">|</span>
              <span className="text-xs text-gray-400">
                PMIC: <span className={ctx.deviceInfo.pmic_detected ? 'text-green-400' : 'text-red-400'}>
                  {ctx.deviceInfo.pmic_detected ? '✓ 0x20' : '✗'}
                </span>
              </span>
              <span className="text-xs text-gray-400">
                VCOM: <span className={ctx.deviceInfo.vcom_detected ? 'text-green-400' : 'text-red-400'}>
                  {ctx.deviceInfo.vcom_detected ? '✓ 0x74' : '✗'}
                </span>
              </span>
            </>
          )}
          <button onClick={handleDetect} className={btnCyan} disabled={!ctx.connected || toolbarBusy}>
            Detect
          </button>
          {busyAction && <span className="text-xs text-amber-300">{busyAction}</span>}
        </div>
      </div>

      {/* Row 2: Firmware Operations */}
      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">Firmware:</span>
        <button onClick={handleBrowse} className={btnBlue} disabled={toolbarBusy}>Browse..</button>
        {firmwareName && (
          <span className="text-xs text-gray-300 font-mono bg-gray-700 px-2 py-0.5 rounded max-w-[200px] truncate">
            {firmwareName}
          </span>
        )}
        <button onClick={handleFwWriteDac} className={btnOrange} disabled={!ctx.connected || !firmwarePath || toolbarBusy}>
          Write DAC
        </button>
        <button onClick={handleFwWriteEeprom} className={btnOrange} disabled={!ctx.connected || !firmwarePath || toolbarBusy}>
          Write EEPROM
        </button>
        <button onClick={handleVerifyAll} className={btnPurple} disabled={!ctx.connected || !firmwarePath || toolbarBusy}>
          Verify ALL
        </button>
      </div>

      {/* Row 3: UI Register Operations */}
      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">UI:</span>
        <button onClick={handleReadDacToUi} className={btnBlue} disabled={!ctx.connected || toolbarBusy}>
          Read DAC→UI
        </button>
        <button onClick={handleReadEepromToUi} className={btnBlue} disabled={!ctx.connected || toolbarBusy}>
          Read EEPROM→UI
        </button>
        <button onClick={handleWriteUiToDac} className={btnOrange} disabled={!ctx.connected || toolbarBusy}>
          Write UI→DAC
        </button>
        <button onClick={handleWriteDacToEeprom} className={btnOrange} disabled={!ctx.connected || toolbarBusy}>
          Write DAC→EEPROM
        </button>
      </div>

      {/* Row 4: Export */}
      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">Export:</span>
        <span className="text-xs text-gray-500 font-mono max-w-[300px] truncate">
          {exportPath ? `Path: ${exportPath}` : 'Path: (not set)'}
        </span>
        <button onClick={handleChangeExportPath} className={btnBlue} disabled={toolbarBusy}>
          {exportPath ? 'Change' : 'Set Path'}
        </button>
        <span className="text-xs text-gray-400 font-medium">Panel:</span>
        <input
          type="text"
          className={`${inputStyle} w-40`}
          value={panelName}
          onChange={e => setPanelName(e.target.value)}
          placeholder="e.g. V500DJ7-QE1"
          disabled={toolbarBusy}
        />
        <button onClick={handleExportBin} className={btnGreen} disabled={!ctx.connected || toolbarBusy}>
          Export BIN
        </button>
      </div>
    </div>
  );
}

export default Toolbar;
