/**
 * 快捷命令：可自定义、可加提示、持久化。
 */

export interface QuickCommand {
  id: string;
  /** 按钮上显示的名字 */
  label: string;
  /** 实际发到串口的内容（自动补回车） */
  cmd: string;
  /** 发送前弹确认 —— 给 rm / cp 这类破坏性命令用 */
  confirm: boolean;
}

const LS_KEY = 'ek86317a_quickCommands';

function makeId(): string {
  return Math.random().toString(36).slice(2, 10);
}

export const DEFAULT_QUICK_COMMANDS: QuickCommand[] = [
  { id: 'q1', label: 'id', cmd: 'id', confirm: false },
  { id: 'q2', label: 'ls /sdcard', cmd: 'ls /sdcard', confirm: false },
  { id: 'q3', label: 'ls /data/local/tmp', cmd: 'ls /data/local/tmp', confirm: false },
  { id: 'q4', label: 'df -h', cmd: 'df -h', confirm: false },
  { id: 'q5', label: 'mount | grep -i mmc', cmd: 'mount | grep -i mmc', confirm: false },
  { id: 'q6', label: 'sync', cmd: 'sync', confirm: false },
];

function sanitize(list: unknown): QuickCommand[] {
  if (!Array.isArray(list)) return [];
  const out: QuickCommand[] = [];
  for (const item of list) {
    if (item == null || typeof item !== 'object') continue;
    const rec = item as Record<string, unknown>;
    const label = typeof rec.label === 'string' ? rec.label : '';
    const cmd = typeof rec.cmd === 'string' ? rec.cmd : '';
    if (label.trim() === '' && cmd.trim() === '') continue;
    out.push({
      id: typeof rec.id === 'string' && rec.id !== '' ? rec.id : makeId(),
      label: label.trim() === '' ? cmd.trim() : label,
      cmd,
      confirm: rec.confirm === true,
    });
  }
  return out;
}

export function loadQuickCommands(): QuickCommand[] {
  try {
    const stored = localStorage.getItem(LS_KEY);
    if (stored == null) return DEFAULT_QUICK_COMMANDS.map((q) => ({ ...q }));
    const parsed = sanitize(JSON.parse(stored));
    return parsed;
  } catch {
    return DEFAULT_QUICK_COMMANDS.map((q) => ({ ...q }));
  }
}

export function saveQuickCommands(list: QuickCommand[]): void {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify(list));
  } catch {
    // 存不下就算了，不影响使用
  }
}

export function newQuickCommand(): QuickCommand {
  return { id: makeId(), label: '', cmd: '', confirm: false };
}
