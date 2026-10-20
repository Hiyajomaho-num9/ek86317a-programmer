import { useCallback, useEffect, useState } from 'react';
import { useAppContext } from '../../App';
import UartTerminal from './UartTerminal';
import {
  BAUD_OPTIONS,
  DEFAULT_UART_CONFIG,
  SU_MODES,
  UART_DIALECTS,
  formatScanSummary,
  hex,
  uartClose,
  uartListPorts,
  uartOpen,
  uartScanBus,
} from '../../lib/uart';
import type { UartConfig, UartOpenResult, UartPortInfo, UartScanResult } from '../../lib/uart';

// 换了 key：老版本存的 su_mode='' 会把新默认值(session)盖掉，直接让旧配置失效
const LS_UART_CONFIG = 'ek86317a_uartConfig_v2';

/** 常见的 PMU / VCOM 从机地址，扫描到后直接标出来，省得对着网格数。 */
const KNOWN_ADDRESSES: Record<number, string> = {
  0x20: 'PMU (7bit) — 0x40/0x41 就是它的 8 位读写地址',
  0x74: 'VCOM 从机',
  0x50: 'EEPROM',
  0x3e: '常见于 LCD bias',
};

function loadConfig(): UartConfig {
  try {
    const stored = localStorage.getItem(LS_UART_CONFIG);
    if (stored == null) return { ...DEFAULT_UART_CONFIG };
    const parsed = JSON.parse(stored) as Partial<UartConfig>;
    return { ...DEFAULT_UART_CONFIG, ...parsed };
  } catch {
    return { ...DEFAULT_UART_CONFIG };
  }
}

/** 把 \r / \n 这些不可见字符显示成可读形式。 */
function visibleLineEnding(value: string): string {
  return value.replace(/\r/g, '\\r').replace(/\n/g, '\\n');
}

function UartTab() {
  const ctx = useAppContext();

  const [config, setConfig] = useState<UartConfig>(loadConfig);
  const [ports, setPorts] = useState<UartPortInfo[]>([]);
  const [opened, setOpened] = useState(false);
  const [linkInfo, setLinkInfo] = useState<UartOpenResult | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const [scan, setScan] = useState<UartScanResult | null>(null);
  useEffect(() => {
    localStorage.setItem(LS_UART_CONFIG, JSON.stringify(config));
  }, [config]);

  const patch = useCallback((partial: Partial<UartConfig>) => {
    setConfig((prev) => ({ ...prev, ...partial }));
  }, []);

  const run = useCallback(
    async (label: string, task: () => Promise<void>) => {
      if (busy != null) return;
      setBusy(label);
      try {
        await task();
      } finally {
        setBusy(null);
      }
    },
    [busy],
  );

  const handleListPorts = async () => {
    await run('枚举串口...', async () => {
      try {
        const found = await uartListPorts();
        setPorts(found);
        ctx.addLog('info', `发现 ${found.length} 个串口`);
        if (found.length > 0 && !found.some((p) => p.path === config.port)) {
          patch({ port: found[0].path });
        }
      } catch (error: unknown) {
        ctx.addLog('error', `枚举串口失败: ${error}`);
      }
    });
  };

  const handleOpen = async () => {
    await run('打开串口...', async () => {
      try {
        const info = await uartOpen(config);
        setLinkInfo(info);
        setOpened(true);
        ctx.addLog(
          info.root ? 'success' : 'warn',
          `串口已打开 ${config.port} @ ${config.baud} —— ${info.root ? '已有 root' : '不是 root（读写 /dev/i2c-* 会被拒）'} | ${info.id_line.trim()}`,
        );
      } catch (error: unknown) {
        setOpened(false);
        setLinkInfo(null);
        ctx.addLog('error', `打开串口失败: ${error}`);
      }
    });
  };

  const handleClose = async () => {
    await run('关闭串口...', async () => {
      try {
        await uartClose();
        setOpened(false);
        ctx.addLog('info', '串口已关闭');
      } catch (error: unknown) {
        ctx.addLog('error', `关闭串口失败: ${error}`);
      }
    });
  };

  const handleScan = async () => {
    await run('扫描总线...', async () => {
      try {
        const result = await uartScanBus(config, 0x03, 0x77);
        setScan(result);
        setOpened(true);
        ctx.addLog(
          result.addresses.length > 0 ? 'success' : 'warn',
          `bus ${config.bus} 扫描: ${formatScanSummary(result.addresses)}`,
        );
      } catch (error: unknown) {
        const message = String(error);
        ctx.addLog('error', `扫描失败: ${message}`);
        if (message.includes('Permission denied')) {
          ctx.addLog(
            'warn',
            '/dev/i2c-* 需要 root：把 Run as root 切成 "su -c" 再点 Open / Test',
          );
        }
      }
    });
  };

  const handleConnectPmu = async () => {
    await run('建立 PMU 会话...', async () => {
      try {
        const info = await ctx.connectUart(config, ctx.chipModel);
        ctx.addLog(
          'success',
          `串口 PMU 会话已建立: PMIC=${info.pmic_detected ? 'ok' : 'missing'}, VCOM=${info.vcom_detected == null ? 'N/A' : info.vcom_detected ? 'ok' : 'missing'}`,
        );
        ctx.resetRegisters();
        setOpened(true);
      } catch (error: unknown) {
        ctx.addLog('error', `建立 PMU 会话失败: ${error}`);
      }
    });
  };

  const selectStyle = 'px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 focus:border-blue-500 focus:outline-none';
  const inputStyle = 'px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 font-mono focus:border-blue-500 focus:outline-none';
  const btnBase = 'px-3 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';
  const btnBlue = `${btnBase} bg-blue-600 hover:bg-blue-700`;
  const btnGreen = `${btnBase} bg-green-700 hover:bg-green-600`;
  const btnRed = `${btnBase} bg-red-700 hover:bg-red-600`;

  return (
    <div className="space-y-4">
      <div className="bg-gray-800 border border-gray-700 rounded p-3 space-y-2">
        <div className="flex items-center gap-3 flex-wrap">
          <span className="text-xs text-gray-400 font-medium">UART Bridge</span>
          <span className={`inline-block w-2 h-2 rounded-full ${opened ? 'bg-green-500' : 'bg-gray-500'}`} />
          {linkInfo != null && (
            <span
              className={`text-xs px-2 py-0.5 rounded ${linkInfo.root ? 'bg-green-900 text-green-300' : 'bg-amber-900 text-amber-300'}`}
              title={linkInfo.id_line}
            >
              {linkInfo.root ? 'root' : 'not root'}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-xs text-gray-400">Port:</span>
          <select
            className={`${selectStyle} min-w-[220px]`}
            value={config.port}
            onChange={(e) => patch({ port: e.target.value })}
          >
            <option value="">Select port</option>
            {ports.map((p) => (
              <option key={p.path} value={p.path}>
                ${p.path}${p.description ? ' — ' + p.description : ''}`
              </option>
            ))}
          </select>
          <button onClick={handleListPorts} className={btnBlue} disabled={busy != null}>
            {busy === '枚举串口...' ? '...' : 'Refresh'}
          </button>

          <span className="text-xs text-gray-400">Baud:</span>
          <select
            className={selectStyle}
            value={config.baud}
            onChange={(e) => patch({ baud: Number(e.target.value) })}
          >
            {BAUD_OPTIONS.map((b) => (
              <option key={b} value={b}>{b}</option>
            ))}
          </select>

          <span className="text-xs text-gray-400">Bus:</span>
          <input
            className={`${inputStyle} w-12`}
            value={config.bus}
            onChange={(e) => patch({ bus: Number(e.target.value) || 0 })}
          />

          <span className="text-xs text-gray-400">Dialect:</span>
          <select
            className={selectStyle}
            value={config.dialect}
            onChange={(e) => patch({ dialect: e.target.value })}
          >
            {UART_DIALECTS.map((d) => (
              <option key={d.value} value={d.value}>{d.label}</option>
            ))}
          </select>

          <span className="text-xs text-gray-400">Addr:</span>
          <select
            className={selectStyle}
            value={config.addr_shift}
            onChange={(e) => patch({ addr_shift: Number(e.target.value) })}
          >
            <option value={0}>7-bit</option>
            <option value={1}>8-bit (x2)</option>
          </select>

          <span className="text-xs text-gray-400">EOL:</span>
          <select
            className={selectStyle}
            value={config.line_ending}
            onChange={(e) => patch({ line_ending: e.target.value })}
          >
            <option value={'\r'}>{visibleLineEnding('\r')}</option>
            <option value={'\r\n'}>{visibleLineEnding('\r\n')}</option>
            <option value={'\n'}>{visibleLineEnding('\n')}</option>
          </select>

          <span className="text-xs text-gray-400">Run as root:</span>
          <select
            className={selectStyle}
            value={config.su_mode}
            onChange={(e) => patch({ su_mode: e.target.value })}
          >
            {SU_MODES.map((m) => (
              <option key={m.value} value={m.value}>{m.label}</option>
            ))}
          </select>

          {opened ? (
            <button onClick={handleClose} className={btnRed} disabled={busy != null}>Close</button>
          ) : (
            <button onClick={handleOpen} className={btnGreen} disabled={busy != null || config.port === ''}>
              Open / Test
            </button>
          )}
          <button onClick={handleScan} className={btnBlue} disabled={busy != null || config.port === ''}>
            Scan Bus
          </button>
        </div>

        <button
          className="text-xs text-gray-400 hover:text-gray-200 underline"
          onClick={() => setShowAdvanced((v) => !v)}
        >
          {showAdvanced ? '收起高级设置' : '高级设置（抗丢字节 / 登录）'}
        </button>

        {showAdvanced && (
          <div className="flex items-center gap-3 flex-wrap pt-1 border-t border-gray-700">
            <span className="text-xs text-gray-400">Char delay (ms):</span>
            <input
              className={`${inputStyle} w-16`}
              value={config.char_delay_ms}
              onChange={(e) => patch({ char_delay_ms: Number(e.target.value) || 0 })}
            />
            <span className="text-xs text-gray-400">Timeout (ms):</span>
            <input
              className={`${inputStyle} w-20`}
              value={config.timeout_ms}
              onChange={(e) => patch({ timeout_ms: Number(e.target.value) || 0 })}
            />
            <span className="text-xs text-gray-400">Retries:</span>
            <input
              className={`${inputStyle} w-12`}
              value={config.retries}
              onChange={(e) => patch({ retries: Number(e.target.value) || 0 })}
            />
            <span className="text-xs text-gray-400">Settle (ms):</span>
            <input
              className={`${inputStyle} w-16`}
              value={config.settle_ms}
              onChange={(e) => patch({ settle_ms: Number(e.target.value) || 0 })}
            />
            <span className="text-xs text-gray-400">Login:</span>
            <input
              className={`${inputStyle} w-24`}
              placeholder="user"
              value={config.login_user}
              onChange={(e) => patch({ login_user: e.target.value })}
            />
            <input
              className={`${inputStyle} w-24`}
              placeholder="password"
              type="password"
              value={config.login_pass}
              onChange={(e) => patch({ login_pass: e.target.value })}
            />
          </div>
        )}

        <div className="text-xs text-gray-500 pt-1">
          乱码 / 丢字符时先把 <span className="font-mono">Char delay</span> 调到 1~2 ms；
          toybox 的 i2cget 不接受尾部 mode，工具已按 dialect 自动处理。
        </div>
      </div>

      {linkInfo?.warning != null && (
        <div className="bg-amber-950 border border-amber-700 rounded p-3 space-y-1">
          <div className="text-xs text-amber-300 font-medium">提权没成功 —— 终端仍然可用，可以进去手工排查</div>
          <div className="text-xs text-amber-200 whitespace-pre-wrap font-mono">{linkInfo.warning}</div>
        </div>
      )}

      {scan != null && (
        <div className="bg-gray-800 border border-gray-700 rounded p-3 space-y-2">
          <div className="text-xs text-gray-400 font-medium">
            Scan result — {formatScanSummary(scan.addresses)}
          </div>
          <pre className="text-xs text-gray-300 font-mono bg-gray-900 rounded p-2 overflow-x-auto">
            {scan.grid}
          </pre>
          <div className="flex flex-wrap gap-3">
            {scan.addresses.map((addr) => (
              <span key={addr} className="text-xs text-gray-400">
                <span className="font-mono text-blue-300">{hex(addr)}</span>
                {KNOWN_ADDRESSES[addr] ? ' — ' + KNOWN_ADDRESSES[addr] : ''}
              </span>
            ))}
          </div>
        </div>
      )}

      <UartTerminal config={config} />

      <div className="bg-gray-800 border border-gray-700 rounded p-3 space-y-2">
        <div className="text-xs text-gray-400 font-medium">PMU session over UART</div>
        <div className="flex items-center gap-3 flex-wrap">
          <button
            onClick={handleConnectPmu}
            className={btnGreen}
            disabled={busy != null || config.port === '' || ctx.connected}
          >
            Connect as PMU ({ctx.chipModel.toUpperCase()})
          </button>
        </div>
        {busy != null && <span className="text-xs text-amber-300">{busy}</span>}
      </div>
    </div>
  );
}

export default UartTab;
