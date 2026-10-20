import { useCallback, useEffect, useRef, useState } from 'react';
import { useAppContext } from '../../App';
import { base64ToBytes, uartTermPoll, uartTermSend, uartTermStart, uartTermStop } from '../../lib/uart';
import type { UartConfig } from '../../lib/uart';
import {
  DEFAULT_QUICK_COMMANDS,
  loadQuickCommands,
  newQuickCommand,
  saveQuickCommands,
} from '../../lib/quick-commands';
import type { QuickCommand } from '../../lib/quick-commands';
import { TermBuffer } from './term-buffer';
import type { RenderedView } from './term-buffer';

const EMPTY_VIEW: RenderedView = { before: '', after: '', row: 0, col: 0 };

/** 粘贴分片大小。115200 大约 11.5 KB/s，一次怼进去必掉字节。 */
const PASTE_CHUNK = 8;
const PASTE_DELAY_MS = 12;
/** 轮询间隔。太短会把锁打满，太长打字会有延迟感。 */
const POLL_MS = 60;

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

interface Props {
  config: UartConfig;
}

/**
 * 内置交互终端。
 *
 * 设计要点：
 * - **不做本地回显**（除非手动打开）。串口控制台自己会回显，我们再显示一遍就重了。
 * - **Tab 直接透传 \t**，让板子上的 shell 自己补全。
 * - **粘贴分片发送**。
 * - **显示走 TermBuffer**，不是纯文本堆叠 —— `\b` / `\r` / `ESC[K` 要按终端规则处理，
 *   否则 Backspace 删不掉字、提示符会越叠越多。
 */
function UartTerminal({ config }: Props) {
  const ctx = useAppContext();

  const [running, setRunning] = useState(false);
  const [view, setView] = useState<RenderedView>(EMPTY_VIEW);
  const [localEcho, setLocalEcho] = useState(false);
  const [stats, setStats] = useState({ up: 0, down: 0 });
  const [quick, setQuick] = useState<QuickCommand[]>(loadQuickCommands);
  const [editing, setEditing] = useState(false);

  const inputRef = useRef<HTMLInputElement | null>(null);
  const preRef = useRef<HTMLPreElement | null>(null);
  const decoderRef = useRef<TextDecoder>(new TextDecoder('utf-8'));
  const termRef = useRef<TermBuffer>(new TermBuffer());
  const configRef = useRef(config);
  const echoRef = useRef(localEcho);
  const atBottomRef = useRef(true);

  useEffect(() => { configRef.current = config; }, [config]);
  useEffect(() => { echoRef.current = localEcho; }, [localEcho]);

  const refreshView = useCallback(() => {
    setView(termRef.current.renderParts());
  }, []);

  const append = useCallback((text: string) => {
    if (text === '') return;
    termRef.current.write(text);
    refreshView();
  }, [refreshView]);

  const send = useCallback(async (data: string) => {
    try {
      await uartTermSend(configRef.current, data);
      setStats((s) => ({ ...s, up: s.up + data.length }));
      if (echoRef.current) append(data);
    } catch (error: unknown) {
      ctx.addLog('error', `终端发送失败: ${error}`);
    }
  }, [append, ctx]);

  // 轮询设备输出
  useEffect(() => {
    if (!running) return;
    let cancelled = false;

    const loop = async () => {
      while (!cancelled) {
        try {
          const b64 = await uartTermPoll(configRef.current, POLL_MS);
          if (cancelled) break;
          if (b64 === '') {
            await sleep(25);
            continue;
          }
          const bytes = base64ToBytes(b64);
          // stream: true —— 多字节字符被拆到两次轮询之间也不会乱码
          const text = decoderRef.current.decode(bytes, { stream: true });
          if (text !== '') {
            setStats((s) => ({ ...s, down: s.down + bytes.length }));
            append(text);
          }
        } catch (error: unknown) {
          if (cancelled) break;
          ctx.addLog('error', `终端读取中断: ${error}`);
          setRunning(false);
          break;
        }
      }
    };

    void loop();
    return () => { cancelled = true; };
  }, [running, append, ctx]);

  // 自动滚到底（用户往上翻的时候不打扰）
  useEffect(() => {
    const el = preRef.current;
    if (el == null) return;
    if (atBottomRef.current) el.scrollTop = el.scrollHeight;
  }, [view]);

  const handleStart = async () => {
    try {
      await uartTermStart(configRef.current);
      decoderRef.current = new TextDecoder('utf-8');
      termRef.current.reset();
      setView(EMPTY_VIEW);
      setRunning(true);
      ctx.addLog('info', '终端已接管串口（I2C 命令暂停，停止终端后恢复）');
      await sleep(80);
      inputRef.current?.focus();
    } catch (error: unknown) {
      ctx.addLog('error', `启动终端失败: ${error}`);
    }
  };

  const handleStop = async () => {
    setRunning(false);
    try {
      await uartTermStop();
      ctx.addLog('info', '终端已释放串口，I2C 命令恢复可用');
    } catch (error: unknown) {
      ctx.addLog('error', `停止终端失败: ${error}`);
    }
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    const key = event.key;

    if (key === 'Enter') { event.preventDefault(); void send('\r'); return; }
    if (key === 'Backspace') { event.preventDefault(); void send('\u007f'); return; }
    if (key === 'Tab') { event.preventDefault(); void send('\t'); return; }
    if (key === 'ArrowUp') { event.preventDefault(); void send('\u001b[A'); return; }
    if (key === 'ArrowDown') { event.preventDefault(); void send('\u001b[B'); return; }
    if (key === 'ArrowRight') { event.preventDefault(); void send('\u001b[C'); return; }
    if (key === 'ArrowLeft') { event.preventDefault(); void send('\u001b[D'); return; }
    if (event.ctrlKey) {
      const lower = key.toLowerCase();
      if (lower === 'c') { event.preventDefault(); void send('\u0003'); ctx.addLog('warn', '已发送 Ctrl-C'); return; }
      if (lower === 'd') { event.preventDefault(); void send('\u0004'); return; }
      if (lower === 'l') { event.preventDefault(); void send('\u000c'); termRef.current.reset(); setView(EMPTY_VIEW); return; }
      if (lower === 'a') { event.preventDefault(); void send('\u0001'); return; }
      if (lower === 'e') { event.preventDefault(); void send('\u0005'); return; }
      if (lower === 'u') { event.preventDefault(); void send('\u0015'); return; }
      if (lower === 'w') { event.preventDefault(); void send('\u0017'); return; }
    }
  };

  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    event.target.value = '';
    if (value === '') return;
    void send(value);
  };

  const handlePaste = async (event: React.ClipboardEvent<HTMLInputElement>) => {
    event.preventDefault();
    const text = event.clipboardData.getData('text');
    if (text === '') return;
    ctx.addLog('info', `粘贴 ${text.length} 字符，分片发送中…`);
    for (let i = 0; i < text.length; i += PASTE_CHUNK) {
      await uartTermSend(configRef.current, text.slice(i, i + PASTE_CHUNK));
      await sleep(PASTE_DELAY_MS);
    }
    setStats((s) => ({ ...s, up: s.up + text.length }));
    if (echoRef.current) append(text);
  };

  // ---------------- 快捷命令管理 ----------------

  const commitQuick = (next: QuickCommand[]) => {
    saveQuickCommands(next);
    setQuick(next);
  };

  const updateQuick = (id: string, patch: Partial<QuickCommand>) => {
    commitQuick(quick.map((q) => (q.id === id ? { ...q, ...patch } : q)));
  };

  const removeQuick = (id: string) => {
    commitQuick(quick.filter((q) => q.id !== id));
  };

  const addQuick = () => {
    commitQuick([...quick, newQuickCommand()]);
  };

  const resetQuick = () => {
    commitQuick(DEFAULT_QUICK_COMMANDS.map((q) => ({ ...q })));
  };

  const runQuick = (q: QuickCommand) => {
    if (q.cmd.trim() === '') return;
    if (q.confirm && !window.confirm(`发送这条命令？\n\n${q.cmd}`)) return;
    void send(q.cmd + '\r');
    inputRef.current?.focus();
  };

  const btnBase = 'px-3 py-1 text-xs rounded text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed';
  const btnGreen = `${btnBase} bg-green-700 hover:bg-green-600`;
  const btnRed = `${btnBase} bg-red-700 hover:bg-red-600`;
  const btnGray = `${btnBase} bg-gray-700 hover:bg-gray-600`;
  const chip = 'px-2 py-0.5 text-[11px] rounded bg-gray-700 hover:bg-gray-600 text-gray-200 font-mono';
  const inputSm = 'px-1.5 py-0.5 text-[11px] bg-gray-900 border border-gray-600 rounded text-gray-200 font-mono focus:border-blue-500 focus:outline-none';

  return (
    <div className="bg-gray-800 border border-gray-700 rounded p-3 space-y-2">
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-xs text-gray-400 font-medium">Terminal</span>
        <span className={`inline-block w-2 h-2 rounded-full ${running ? 'bg-green-500' : 'bg-gray-500'}`} />

        {running ? (
          <button onClick={handleStop} className={btnRed}>停止终端</button>
        ) : (
          <button onClick={handleStart} className={btnGreen} disabled={config.port === ''}>启动终端</button>
        )}

        <button
          onClick={() => { termRef.current.reset(); setView(EMPTY_VIEW); atBottomRef.current = true; }}
          className={btnGray}
        >
          清屏
        </button>
        <button onClick={() => void send('\u0003')} className={btnGray} disabled={!running}>Ctrl-C</button>
        <button onClick={() => void send('\t')} className={btnGray} disabled={!running}>Tab</button>
        <button onClick={() => void send('\r')} className={btnGray} disabled={!running}>Enter</button>

        <label className="flex items-center gap-1 text-xs text-gray-400 cursor-pointer">
          <input type="checkbox" checked={localEcho} onChange={(e) => setLocalEcho(e.target.checked)} />
          本地回显
        </label>

        <span className="text-xs text-gray-500 font-mono">↑{stats.up} ↓{stats.down}</span>
      </div>

      {/* 快捷命令 */}
      <div className="flex items-center gap-1 flex-wrap border-t border-gray-700 pt-2">
        <span className="text-xs text-gray-500">快捷：</span>
        {quick.map((q) => (
          <button
            key={q.id}
            className={`${chip} ${q.confirm ? 'ring-1 ring-amber-600' : ''}`}
            title={q.cmd}
            onClick={() => runQuick(q)}
            disabled={!running}
          >
            {q.label}
            {q.confirm ? ' ⚠' : ''}
          </button>
        ))}
        <button
          className="px-2 py-0.5 text-[11px] rounded bg-gray-600 hover:bg-gray-500 text-gray-100"
          onClick={() => setEditing((v) => !v)}
        >
          {editing ? '收起' : '编辑'}
        </button>
      </div>

      {editing && (
        <div className="border border-gray-700 rounded p-2 space-y-1">
          <div className="grid grid-cols-[8rem_1fr_3.5rem_2rem] gap-1 text-[11px] text-gray-500 px-1">
            <span>按钮名</span>
            <span>命令（回车自动补）</span>
            <span>发送前确认</span>
            <span />
          </div>
          {quick.map((q) => (
            <div key={q.id} className="grid grid-cols-[8rem_1fr_3.5rem_2rem] gap-1 items-center">
              <input
                className={inputSm}
                value={q.label}
                placeholder="按钮名"
                onChange={(e) => updateQuick(q.id, { label: e.target.value })}
              />
              <input
                className={inputSm}
                value={q.cmd}
                placeholder="命令，比如 cp /sdcard/a.bin /vendor/b.bin"
                onChange={(e) => updateQuick(q.id, { cmd: e.target.value })}
              />
              <input
                type="checkbox"
                checked={q.confirm}
                onChange={(e) => updateQuick(q.id, { confirm: e.target.checked })}
              />
              <button
                className="text-[11px] text-red-400 hover:text-red-300"
                onClick={() => removeQuick(q.id)}
                title="删除"
              >
                ✕
              </button>
            </div>
          ))}
          <div className="flex items-center gap-2 pt-1">
            <button className={btnGray} onClick={addQuick}>+ 添加</button>
            <button className={btnGray} onClick={resetQuick}>恢复默认</button>
            <span className="text-[11px] text-gray-500">
              改动即时保存，重启不丢。勾了确认的按钮点下去会先问一句。
            </span>
          </div>
        </div>
      )}

      <div className="relative" onClick={() => inputRef.current?.focus()}>
        <pre
          ref={preRef}
          onScroll={(e) => {
            const el = e.currentTarget;
            atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
          }}
          className={`h-80 overflow-auto bg-gray-950 border border-gray-700 rounded p-2 text-xs font-mono text-gray-200 whitespace-pre-wrap break-all ${running ? 'cursor-text' : ''}`}
        >
          {view.before}
          {running && <span className="animate-pulse text-green-400">▊</span>}
          {view.after}
        </pre>

        <input
          ref={inputRef}
          className="absolute left-0 top-0 h-0 w-0 opacity-0"
          value=""
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          disabled={!running}
          autoComplete="off"
          spellCheck={false}
        />
      </div>
    </div>
  );
}

export default UartTerminal;
