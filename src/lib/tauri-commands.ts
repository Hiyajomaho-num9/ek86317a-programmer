import { invoke } from '@tauri-apps/api/core';
import type { ChipModel } from './chips';

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

export interface ProgramResult {
  success: boolean;
  registers_written: number;
  eeprom_written: boolean;
}

export interface VerifyResult {
  success: boolean;
  total: number;
  matched: number;
  mismatches: [number, number, number][];
}

export interface VerifyAllResult {
  total: number;
  dac_matched: number;
  eeprom_matched: number;
  dac_mismatches: [number, number, number][];
  eeprom_mismatches: [number, number, number][];
}

export interface WriteAllDacResult {
  success: boolean;
  registers_written: number;
}

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

async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error: unknown) {
    if (error instanceof TypeError && String(error).includes('__TAURI_INTERNALS__')) {
      throw new Error(`Tauri IPC not available. Is this running inside Tauri webview? (command: ${cmd})`);
    }
    throw error;
  }
}

export async function scanDevices(): Promise<string[]> {
  return safeInvoke('scan_devices');
}

export async function connectDevice(deviceId: string, clockHz: number, chipModel: ChipModel): Promise<DeviceInfo> {
  return safeInvoke('connect_device', { deviceId, clockHz, chipModel });
}

export async function disconnectDevice(): Promise<void> {
  return safeInvoke('disconnect_device');
}

export async function detectIc(): Promise<DeviceInfo> {
  return safeInvoke('detect_ic');
}

export async function readDacRegister(addr: number): Promise<RegisterData> {
  return safeInvoke('read_dac_register', { addr });
}

export async function writeDacRegister(addr: number, value: number): Promise<void> {
  return safeInvoke('write_dac_register', { addr, value });
}

export async function readAllDac(): Promise<RegisterData[]> {
  return safeInvoke('read_all_dac');
}

export async function readAllEeprom(): Promise<RegisterData[]> {
  return safeInvoke('read_all_eeprom');
}

export async function loadFirmware(path: string, chipModel: ChipModel): Promise<FirmwarePreview> {
  return safeInvoke('load_firmware', { path, chipModel });
}

export async function programFirmware(path: string, writeEeprom: boolean): Promise<ProgramResult> {
  return safeInvoke('program_firmware', { path, writeEeprom });
}

export async function verifyFirmware(path: string): Promise<VerifyResult> {
  return safeInvoke('verify_firmware', { path });
}

export async function verifyAll(path: string): Promise<VerifyAllResult> {
  return safeInvoke('verify_all', { path });
}

export async function writeAllDacRegisters(registers: [number, number][]): Promise<WriteAllDacResult> {
  return safeInvoke('write_all_dac_registers', { entries: registers });
}

export async function exportEeprom(path: string): Promise<void> {
  return safeInvoke('export_eeprom', { path });
}

export async function writeAllToEeprom(): Promise<void> {
  return safeInvoke('write_all_to_eeprom');
}

export async function writeVcom1ToEeprom(): Promise<void> {
  return safeInvoke('write_vcom1_to_eeprom');
}

export async function readFaultFlags(): Promise<FaultFlags> {
  return safeInvoke('read_fault_flags');
}
