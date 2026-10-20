import { invoke } from '@tauri-apps/api/core';
import type { ChipModel } from './chips';
import type { DeviceInfo } from './tauri-commands';

/**
 * 串口 I2C 桥的连接参数，字段名与 Rust 侧 `UartConfig` 一一对应（snake_case）。
 */
export interface UartConfig {
  port: string;
  baud: number;
  bus: number;
  /** `toybox` 或 `i2ctools`，决定 i2cget 的命令拼法 */
  dialect: string;
  /** 0 = 直接发 7 位地址；1 = 发 7bit << 1（8 位写地址） */
  addr_shift: number;
  /** 行尾，一般是 `\r` */
  line_ending: string;
  /** 逐字节发送时字节间隔（毫秒），0 = 整块发送 */
  char_delay_ms: number;
  timeout_ms: number;
  retries: number;
  settle_ms: number;
  login_user: string;
  login_pass: string;
  /**
   * 提权方式。Android / TV 板上 /dev/i2c-* 通常只有 root 能开。
   *   ''        不提权
   *   'wrap'    每条命令包一层 su -c "..."（推荐）
   *   'session' 打开时切一次 root shell
   */
  su_mode: string;
}

/** 打开串口后的链路信息。root 是握手时用 id 探到的。 */
export interface UartOpenResult {
  path: string;
  root: boolean;
  id_line: string;
  /** 打开时发现的问题（比如提权没成功），UI 用横幅提示 */
  warning: string | null;
}

export interface UartPortInfo {
  path: string;
  description: string;
}

export interface UartScanResult {
  addresses: number[];
  grid: string;
}

export interface UartDumpResult {
  addr: number;
  first: number;
  last: number;
  values: [number, number][];
}

export const DEFAULT_UART_CONFIG: UartConfig = {
  port: '',
  baud: 115200,
  bus: 1,
  dialect: 'toybox',
  addr_shift: 0,
  line_ending: '\r',
  char_delay_ms: 0,
  timeout_ms: 2000,
  retries: 3,
  settle_ms: 120,
  login_user: '',
  login_pass: '',
  // 实测 su -c 在定制 ROM 上不通，会话模式兼容性更好，设为默认
  su_mode: 'session',
};

/**
 * 提权选项。
 * - su (会话)：连接时切一次 root shell，后面所有命令都在 root 里跑。
 *   兼容性最好 —— 不管 su 是交互式的、要回车确认的，还是只认 -c 的，
 *   这里都会挨个试一遍，所以默认选它。
 * - su -c：每条命令单独包一层，无状态。部分 ROM 的 su 不支持 -c。
 * - None：进程本身已经是 root。
 */
export const SU_MODES = [
  { value: 'session', label: 'su (会话) — 推荐' },
  { value: 'wrap', label: 'su -c' },
  { value: '', label: 'None (已经是 root)' },
];

export const BAUD_OPTIONS = [115200, 230400, 460800, 921600];

/** 面板主芯片串口常见 115200；波特率越高越容易丢字节，不建议盲目拉高。 */
export const UART_DIALECTS = [
  { value: 'toybox', label: 'toybox (Android / TV)' },
  { value: 'i2ctools', label: 'i2c-tools (标准版)' },
];

export async function uartListPorts(): Promise<UartPortInfo[]> {
  return invoke<UartPortInfo[]>('uart_list_ports');
}

export async function uartOpen(config: UartConfig): Promise<UartOpenResult> {
  return invoke<UartOpenResult>('uart_open', { config });
}

export async function uartClose(): Promise<void> {
  return invoke<void>('uart_close');
}

export async function uartScanBus(
  config: UartConfig,
  first?: number,
  last?: number,
): Promise<UartScanResult> {
  return invoke<UartScanResult>('uart_scan_bus', {
    config,
    first: first ?? null,
    last: last ?? null,
  });
}

export async function uartReadRegister(config: UartConfig, addr: number, reg: number): Promise<number> {
  return invoke<number>('uart_read_register', { config, addr, reg });
}

export async function uartWriteRegister(
  config: UartConfig,
  addr: number,
  reg: number,
  value: number,
): Promise<number> {
  return invoke<number>('uart_write_register', { config, addr, reg, value });
}

export async function uartDump(
  config: UartConfig,
  addr: number,
  first: number,
  last: number,
): Promise<UartDumpResult> {
  return invoke<UartDumpResult>('uart_dump', { config, addr, first, last });
}

export async function uartShell(config: UartConfig, command: string): Promise<string> {
  return invoke<string>('uart_shell', { config, command });
}

export async function connectUartDevice(config: UartConfig, chipModel: ChipModel): Promise<DeviceInfo> {
  return invoke<DeviceInfo>('connect_uart_device', { config, chipModel });
}

export function hex(value: number, width = 2): string {
  return `0x` + value.toString(16).toUpperCase().padStart(width, '0');
}

export function formatScanSummary(addresses: number[]): string {
  if (addresses.length === 0) return '没有设备应答';
  return addresses.map((a) => hex(a)).join(', ');
}
// ---------------- 交互终端 ----------------

/** 进入交互终端模式（和分帧 I2C 命令互斥）。 */
export async function uartTermStart(config: UartConfig): Promise<void> {
  return invoke<void>('uart_term_start', { config });
}

/** 退出交互终端模式。 */
export async function uartTermStop(): Promise<void> {
  return invoke<void>('uart_term_stop');
}

/** 原样发送一段文本，控制字符直接写在字符串里（如 '\u0003'）。 */
export async function uartTermSend(config: UartConfig, data: string): Promise<void> {
  return invoke<void>('uart_term_send', { config, data });
}

/**
 * 拉取终端输出，返回 base64。
 *
 * 走 base64 而不是数字数组：JSON 数组会把体积放大四倍左右。
 */
export async function uartTermPoll(config: UartConfig, maxMs = 60): Promise<string> {
  return invoke<string>('uart_term_poll', { config, maxMs });
}

/** base64 -> 字节。 */
export function base64ToBytes(b64: string): Uint8Array {
  if (b64 === '') return new Uint8Array(0);
  const binary = atob(b64);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    out[i] = binary.charCodeAt(i);
  }
  return out;
}

