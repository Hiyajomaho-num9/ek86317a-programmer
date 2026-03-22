import { useEffect, useState } from 'react';
import { useAppContext } from '../App';
import { CHIP_OPTIONS, getChipDisplayName, getChipStoragePrefix } from '../lib/chips';
import { CLOCK_OPTIONS } from '../lib/register-map';
import * as cmd from '../lib/tauri-commands';

function sanitizePanelName(name: string): string {
  return name.replace(/[\/:*?"<>|]/g, '_').trim();
}

function timestamp(): string {
  const date = new Date();
  const pad = (value: number) => value.toString().padStart(2, '0');
  return `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}_${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}`;
}

function formatDetectStatus(value: boolean | null | undefined, okLabel: string): string {
  if (value == null) return 'N/A';
  return value ? okLabel : 'missing';
}

function formatDeviceLabel(deviceId: string): string {
  if (deviceId === 'bridge:mock:development') {
    return 'Mock device bridge (development)';
  }

  if (deviceId.startsWith('bridge:ft232h:')) {
    const parts = deviceId.split(':');
    const index = parts[2] ?? '?';
    const desc = parts.slice(3).join(':');
    return desc !== '' ? `Device ${index} - ${desc}` : `Device ${index}`;
  }

  return deviceId;
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

  useEffect(() => {
    localStorage.setItem(LS_EXPORT_PATH, exportPath);
  }, [exportPath]);

  useEffect(() => {
    localStorage.setItem(LS_PANEL_NAME, panelName);
  }, [panelName]);

  const toolbarBusy = busyAction != null;

  const runToolbarAction = async (label: string, task: () => Promise<void>) => {
    if (toolbarBusy) return;
    setBusyAction(label);
    try {
      await task();
    } finally {
      setBusyAction(null);
    }
  };

  const handleChipChange = (value: string) => {
    if (ctx.connected || toolbarBusy) return;
    if (value !== 'ek86317a' && value !== 'iml8947k' && value !== 'lp6281') return;
    ctx.setChipModel(value);
    ctx.resetRegisters();
    setFirmwarePath('');
    setFirmwareName('');
    ctx.addLog('info', `Chip model set to ${getChipDisplayName(value)}`);
  };

  const handleScan = async () => {
    await runToolbarAction('Scanning devices...', async () => {
      try {
        const devices = await ctx.scan();
        ctx.addLog('info', `Scanned ${devices.length} device(s)`);
        if (selectedDevice && devices.includes(selectedDevice) !== true) {
          setSelectedDevice('');
        }
      } catch (error: unknown) {
        ctx.addLog('error', `Scan failed: ${error}`);
      }
    });
  };

  const handleConnect = async () => {
    if (selectedDevice === '') {
      ctx.addLog('warn', 'No device selected');
      return;
    }

    await runToolbarAction('Connecting device...', async () => {
      try {
        const info = await ctx.connect(selectedDevice, clockHz, ctx.chipModel);
        ctx.resetRegisters();
        ctx.addLog('success', `Connected to ${formatDeviceLabel(selectedDevice)} as ${getChipDisplayName(info.chip_model)}`);
        ctx.addLog(
          'info',
          `PMIC: ${info.pmic_detected ? 'ok' : 'missing'} | VCOM: ${formatDetectStatus(info.vcom_detected, 'ok')}`,
        );
      } catch (error: unknown) {
        ctx.addLog('error', `Connect failed: ${error}`);
      }
    });
  };

  const handleDisconnect = async () => {
    await runToolbarAction('Disconnecting device...', async () => {
      try {
        await ctx.disconnect();
        ctx.resetRegisters();
        ctx.addLog('info', 'Disconnected');
      } catch (error: unknown) {
        ctx.addLog('error', `Disconnect failed: ${error}`);
      }
    });
  };

  const handleDetect = async () => {
    await runToolbarAction('Detecting IC...', async () => {
      try {
        const info = await ctx.detect();
        ctx.addLog(
          'info',
          `Detect ${getChipDisplayName(info.chip_model)}: PMIC=${info.pmic_detected ? 'ok' : 'missing'}, VCOM=${formatDetectStatus(info.vcom_detected, 'ok')}`,
        );
      } catch (error: unknown) {
        ctx.addLog('error', `Detect failed: ${error}`);
      }
    });
  };

  const handleBrowse = async () => {
    await runToolbarAction('Loading firmware...', async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const result = await open({
          multiple: false,
          filters: [{ name: 'Firmware', extensions: ['bin'] }],
        });
        if (result == null) return;

        const path = typeof result === 'string' ? result : String(result);
        const preview = await cmd.loadFirmware(path, ctx.chipModel);
        setFirmwarePath(path);
        setFirmwareName(preview.file_name);
        const entries = preview.registers.map((reg) => [reg.address, reg.value] as [number, number]);
        ctx.replaceDacRegisters(entries);
        ctx.addLog('success', `Loaded ${preview.file_name}: ${preview.register_count} registers into UI`);
      } catch (error: unknown) {
        ctx.addLog('error', `Browse/load failed: ${error}`);
      }
    });
  };

  const handleFwWriteDac = async () => {
    if (firmwarePath === '') {
      ctx.addLog('warn', 'No firmware file loaded');
      return;
    }
    await runToolbarAction('Writing firmware to DAC...', async () => {
      try {
        const result = await cmd.programFirmware(firmwarePath, false);
        ctx.addLog('success', `Write DAC complete: ${result.registers_written} registers`);
      } catch (error: unknown) {
        ctx.addLog('error', `Write DAC failed: ${error}`);
      }
    });
  };

  const handleFwWriteEeprom = async () => {
    if (firmwarePath === '') {
      ctx.addLog('warn', 'No firmware file loaded');
      return;
    }
    await runToolbarAction('Writing firmware to EEPROM...', async () => {
      try {
        const result = await cmd.programFirmware(firmwarePath, true);
        ctx.addLog('success', `Write EEPROM complete: ${result.registers_written} registers written`);
      } catch (error: unknown) {
        ctx.addLog('error', `Write EEPROM failed: ${error}`);
      }
    });
  };

  const handleVerifyAll = async () => {
    if (firmwarePath === '') {
      ctx.addLog('warn', 'No firmware file loaded');
      return;
    }
    await runToolbarAction('Verifying DAC and EEPROM...', async () => {
      try {
        const result = await cmd.verifyAll(firmwarePath);
        ctx.addLog(
          result.dac_mismatches.length === 0 ? 'success' : 'error',
          `Verify DAC: ${result.dac_mismatches.length === 0 ? 'PASS' : 'FAIL'} (${result.dac_matched}/${result.total})`,
        );
        ctx.addLog(
          result.eeprom_mismatches.length === 0 ? 'success' : 'error',
          `Verify EEPROM: ${result.eeprom_mismatches.length === 0 ? 'PASS' : 'FAIL'} (${result.eeprom_matched}/${result.total})`,
        );
      } catch (error: unknown) {
        ctx.addLog('error', `Verify ALL failed: ${error}`);
      }
    });
  };

  const handleReadDacToUi = async () => {
    await runToolbarAction('Reading DAC into UI...', async () => {
      try {
        const regs = await cmd.readAllDac();
        ctx.replaceDacRegisters(regs.map((reg) => [reg.address, reg.value] as [number, number]));
        ctx.addLog('success', 'Read DAC -> UI complete');
      } catch (error: unknown) {
        ctx.addLog('error', `Read DAC -> UI failed: ${error}`);
      }
    });
  };

  const handleReadEepromToUi = async () => {
    await runToolbarAction('Reading EEPROM into UI...', async () => {
      try {
        const regs = await cmd.readAllEeprom();
        const entries = regs.map((reg) => [reg.address, reg.value] as [number, number]);
        ctx.replaceEepromRegisters(entries);
        ctx.replaceDacRegisters(entries);
        ctx.addLog('success', 'Read EEPROM -> UI complete');
      } catch (error: unknown) {
        ctx.addLog('error', `Read EEPROM -> UI failed: ${error}`);
      }
    });
  };

  const handleWriteUiToDac = async () => {
    await runToolbarAction('Writing UI registers to DAC...', async () => {
      try {
        const regs = Array.from(ctx.dacRegisters.entries()) as [number, number][];
        if (regs.length === 0) {
          ctx.addLog('warn', 'No UI register data to write');
          return;
        }
        const result = await cmd.writeAllDacRegisters(regs);
        ctx.addLog('success', `Write UI -> DAC complete: ${result.registers_written} registers`);
      } catch (error: unknown) {
        ctx.addLog('error', `Write UI -> DAC failed: ${error}`);
      }
    });
  };

  const handleWriteDacToEeprom = async () => {
    await runToolbarAction('Writing DAC to EEPROM...', async () => {
      try {
        await cmd.writeAllToEeprom();
        ctx.addLog('success', 'Write DAC -> EEPROM complete');
      } catch (error: unknown) {
        ctx.addLog('error', `Write DAC -> EEPROM failed: ${error}`);
      }
    });
  };

  const handleChangeExportPath = async () => {
    await runToolbarAction('Selecting export path...', async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const result = await open({ directory: true });
        if (result == null) return;
        const path = typeof result === 'string' ? result : String(result);
        setExportPath(path);
        ctx.addLog('info', `Export path set: ${path}`);
      } catch (error: unknown) {
        ctx.addLog('error', `Set export path failed: ${error}`);
      }
    });
  };

  const handleExportBin = async () => {
    const sanitizedName = sanitizePanelName(panelName);
    if (sanitizedName === '') {
      ctx.addLog('warn', 'Please enter a panel name');
      return;
    }

    let dir = exportPath;
    if (dir === '') {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const result = await open({ directory: true });
        if (result == null) return;
        dir = typeof result === 'string' ? result : String(result);
        setExportPath(dir);
      } catch (error: unknown) {
        ctx.addLog('error', `Set export path failed: ${error}`);
        return;
      }
    }

    const fileName = `PMU_${getChipStoragePrefix(ctx.chipModel)}_${sanitizedName}_${timestamp()}.bin`;
    const sep = dir.includes('\\') ? '\\' : '/';
    const fullPath = dir.endsWith(sep) ? `${dir}${fileName}` : `${dir}${sep}${fileName}`;

    await runToolbarAction('Exporting EEPROM to BIN...', async () => {
      try {
        await cmd.exportEeprom(fullPath);
        ctx.addLog('success', `Exported: ${fullPath}`);
      } catch (error: unknown) {
        ctx.addLog('error', `Export failed: ${error}`);
      }
    });
  };

  const btnBase = 'px-3 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';
  const btnBlue = `${btnBase} bg-blue-600 hover:bg-blue-700`;
  const btnGreen = `${btnBase} bg-green-700 hover:bg-green-600`;
  const btnRed = `${btnBase} bg-red-700 hover:bg-red-600`;
  const btnPurple = `${btnBase} bg-purple-700 hover:bg-purple-600`;
  const btnOrange = `${btnBase} bg-orange-700 hover:bg-orange-600`;
  const btnCyan = `${btnBase} bg-cyan-700 hover:bg-cyan-600`;
  const selectStyle = 'px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';
  const inputStyle = 'px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';

  return (
    <div className="bg-gray-800 border-b border-gray-700 px-4 py-2 space-y-1.5">
      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">Chip:</span>
        <select
          className={selectStyle}
          value={ctx.chipModel}
          onChange={(event) => handleChipChange(event.target.value)}
          disabled={ctx.connected || toolbarBusy}
        >
          {CHIP_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>
      </div>

      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">Devices:</span>
        <select
          className={`${selectStyle} min-w-[180px]`}
          value={selectedDevice}
          onChange={(event) => setSelectedDevice(event.target.value)}
          disabled={ctx.connected || toolbarBusy}
        >
          <option value="">Select Device</option>
          {ctx.devices.map((device) => (
            <option key={device} value={device}>{formatDeviceLabel(device)}</option>
          ))}
        </select>
        <button onClick={handleScan} className={btnBlue} disabled={ctx.connected || ctx.scanning || toolbarBusy}>
          {ctx.scanning || busyAction === 'Scanning devices...' ? 'Scanning...' : 'Scan'}
        </button>

        <span className="text-xs text-gray-400 font-medium">SCL:</span>
        <select
          className={selectStyle}
          value={clockHz}
          onChange={(event) => setClockHz(Number(event.target.value))}
          disabled={ctx.connected || toolbarBusy}
        >
          {CLOCK_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>

        {ctx.connected ? (
          <button onClick={handleDisconnect} className={btnRed} disabled={toolbarBusy}>Disconnect</button>
        ) : (
          <button onClick={handleConnect} className={btnGreen} disabled={selectedDevice === '' || toolbarBusy}>Connect</button>
        )}

        <div className="flex items-center gap-2 ml-2 flex-wrap">
          <span className="text-xs text-gray-400">Status:</span>
          <span className={`inline-block w-2 h-2 rounded-full ${ctx.connected ? 'bg-green-500' : 'bg-red-500'}`} />
          <span className={`text-xs ${ctx.connected ? 'text-green-400' : 'text-gray-500'}`}>
            {ctx.connected ? 'Connected' : 'Disconnected'}
          </span>
          {ctx.deviceInfo && (
            <>
              <span className="text-gray-600">|</span>
              <span className="text-xs text-gray-400">Chip: <span className="text-blue-300">{getChipDisplayName(ctx.deviceInfo.chip_model)}</span></span>
              <span className="text-xs text-gray-400">PMIC: <span className={ctx.deviceInfo.pmic_detected ? 'text-green-400' : 'text-red-400'}>{ctx.deviceInfo.pmic_detected ? '0x20' : 'missing'}</span></span>
              <span className="text-xs text-gray-400">VCOM: <span className={ctx.deviceInfo.vcom_detected == null ? 'text-gray-400' : ctx.deviceInfo.vcom_detected ? 'text-green-400' : 'text-red-400'}>{formatDetectStatus(ctx.deviceInfo.vcom_detected, '0x74')}</span></span>
            </>
          )}
          <button onClick={handleDetect} className={btnCyan} disabled={ctx.connected !== true || toolbarBusy}>Detect</button>
          {busyAction && <span className="text-xs text-amber-300">{busyAction}</span>}
        </div>
      </div>

      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">Firmware:</span>
        <button onClick={handleBrowse} className={btnBlue} disabled={toolbarBusy}>Browse</button>
        {firmwareName && (
          <span className="text-xs text-gray-300 font-mono bg-gray-700 px-2 py-0.5 rounded max-w-[240px] truncate">{firmwareName}</span>
        )}
        <button onClick={handleFwWriteDac} className={btnOrange} disabled={ctx.connected !== true || firmwarePath === '' || toolbarBusy}>Write DAC</button>
        <button onClick={handleFwWriteEeprom} className={btnOrange} disabled={ctx.connected !== true || firmwarePath === '' || toolbarBusy}>Write EEPROM</button>
        <button onClick={handleVerifyAll} className={btnPurple} disabled={ctx.connected !== true || firmwarePath === '' || toolbarBusy}>Verify ALL</button>
      </div>

      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">UI:</span>
        <button onClick={handleReadDacToUi} className={btnBlue} disabled={ctx.connected !== true || toolbarBusy}>Read DAC to UI</button>
        <button onClick={handleReadEepromToUi} className={btnBlue} disabled={ctx.connected !== true || toolbarBusy}>Read EEPROM to UI</button>
        <button onClick={handleWriteUiToDac} className={btnOrange} disabled={ctx.connected !== true || toolbarBusy}>Write UI to DAC</button>
        <button onClick={handleWriteDacToEeprom} className={btnOrange} disabled={ctx.connected !== true || toolbarBusy}>Write DAC to EEPROM</button>
      </div>

      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">Export:</span>
        <span className="text-xs text-gray-500 font-mono max-w-[300px] truncate">{exportPath ? `Path: ${exportPath}` : 'Path: (not set)'}</span>
        <button onClick={handleChangeExportPath} className={btnBlue} disabled={toolbarBusy}>{exportPath ? 'Change' : 'Set Path'}</button>
        <span className="text-xs text-gray-400 font-medium">Panel:</span>
        <input
          type="text"
          className={`${inputStyle} w-40`}
          value={panelName}
          onChange={(event) => setPanelName(event.target.value)}
          placeholder="e.g. V500DJ7-QE1"
          disabled={toolbarBusy}
        />
        <button onClick={handleExportBin} className={btnGreen} disabled={ctx.connected !== true || toolbarBusy}>Export BIN</button>
      </div>
    </div>
  );
}

export default Toolbar;
