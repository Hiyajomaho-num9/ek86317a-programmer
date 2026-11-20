import { invoke } from '@tauri-apps/api/core';
import type { InvokeArgs } from '@tauri-apps/api/core';
import type { ChipModel } from './chips';
import { hardwareQueue } from './operation-queue';

export interface DeviceInfo {
  pmic_detected: boolean;
  vcom_detected: boolean | null;
  device_id: string;
  chip_model: ChipModel;
}
export interface RegisterData {
  address: number;
  value: number;
  name: string;
  voltage: number | null;
}
export interface FirmwarePreview {
  file_name: string;
  size: number;
  register_count: number;
  registers: RegisterData[];
}
export interface ProgramResult { success: boolean; registers_written: number; eeprom_written: boolean; }
export interface VerifyResult {
  success: boolean;
  total: number;
  compared: number;
  matched: number;
  missing: number[];
  mismatches: [number, number, number][];
  partial: boolean;
}
export interface VerifyAllResult {
  success: boolean;
  total: number;
  compared: number;
  missing: number[];
  partial: boolean;
  dac_compared: number;
  dac_matched: number;
  dac_missing: number[];
  eeprom_compared: number;
  eeprom_matched: number;
  eeprom_missing: number[];
  dac_mismatches: [number, number, number][];
  eeprom_mismatches: [number, number, number][];
}
export interface WriteAllDacResult { success: boolean; registers_written: number; }
export interface FaultFlags {
  raw: number;
  otp: boolean;
  vbk1: boolean;
  avdd: boolean;
  vgh: boolean;
  vgl: boolean;
  vss1: boolean;
  havdd: boolean;
}

export async function safeInvoke<T>(command: string, args?: unknown): Promise<T> {
  try {
    return await invoke<T>(command, args as InvokeArgs | undefined);
  } catch (error) {
    if (typeof error === 'string') throw new Error(error);
    if (error instanceof Error) throw error;
    throw new Error(String(error));
  }
}

export const HARDWARE_COMMANDS = [
  'scan_devices', 'connect_device', 'detect_ic', 'disconnect_device',
  'read_dac_register', 'write_dac_register', 'read_all_dac', 'read_all_eeprom',
  'program_firmware', 'verify_firmware', 'verify_all', 'write_all_dac_registers',
  'write_all_to_eeprom', 'write_vcom1_to_eeprom', 'read_fault_flags',
  'connect_uart_device', 'uart_list_ports', 'uart_open', 'uart_close',
  'uart_scan_bus', 'uart_read_register', 'uart_write_register', 'uart_dump', 'uart_shell',
  'uart_term_start', 'uart_term_stop', 'uart_term_send', 'uart_term_poll',
] as const;
export type HardwareCommand = (typeof HARDWARE_COMMANDS)[number];

/** FIFO at the transport boundary also protects legacy panels using safeInvoke directly. */
export function invokeHardware<T>(command: HardwareCommand, args?: unknown): Promise<T> {
  return hardwareQueue.enqueue(() => safeInvoke<T>(command, args));
}

export const scanDevices = () => invokeHardware<string[]>('scan_devices');
export const connectDevice = (deviceId: string, clockHz: number, chipModel: ChipModel) =>
  invokeHardware<DeviceInfo>('connect_device', { deviceId, clockHz, chipModel });
export const detectIc = () => invokeHardware<DeviceInfo>('detect_ic');
export const disconnectDevice = () => invokeHardware<void>('disconnect_device');

/** Internal batch transport: the caller owns the outer FIFO transaction. */
export const readDacRegisterUnqueued = async (addr: number) =>
  (await safeInvoke<RegisterData>('read_dac_register', { addr })).value;
export const writeDacRegisterUnqueued = (addr: number, value: number) =>
  safeInvoke<void>('write_dac_register', { addr, value });
export const readDacRegister = (addr: number) => invokeHardware<RegisterData>('read_dac_register', { addr });
export const writeDacRegister = (addr: number, value: number) => invokeHardware<void>('write_dac_register', { addr, value });
export const readAllDac = () => invokeHardware<RegisterData[]>('read_all_dac');
export const readAllEeprom = () => invokeHardware<RegisterData[]>('read_all_eeprom');
export const writeAllToEeprom = () => invokeHardware<void>('write_all_to_eeprom');
export const writeVcom1ToEeprom = () => invokeHardware<void>('write_vcom1_to_eeprom');
export const writeAllDacRegisters = (entries: [number, number][]) => invokeHardware<WriteAllDacResult>('write_all_dac_registers', { entries });

export const loadFirmware = (path: string, chipModel: ChipModel) => safeInvoke<FirmwarePreview>('load_firmware', { path, chipModel });
export const programFirmware = (path: string, writeEeprom: boolean) => invokeHardware<ProgramResult>('program_firmware', { path, writeEeprom });
export const verifyFirmware = (path: string) => invokeHardware<VerifyResult>('verify_firmware', { path });
export const verifyAll = (path: string) => invokeHardware<VerifyAllResult>('verify_all', { path });
export const exportRegisters = (path: string, chipModel: ChipModel, entries: [number, number][]) => safeInvoke<void>('export_registers', { path, chipModel, entries });
export const readFaultFlags = () => invokeHardware<FaultFlags>('read_fault_flags');
