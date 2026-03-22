/**
 * Tauri command wrappers for communicating with Rust backend.
 * Maps 1:1 to the 13+ Tauri commands defined in src-tauri/src/commands/.
 *
 * All invoke calls are wrapped with try/catch to prevent UI crashes
 * when running outside Tauri webview or when the backend is unavailable.
 */

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// Data Types (matching Rust serde serialization)
// ============================================================================

export interface DeviceInfo {
  pmic_detected: boolean;
  vcom_detected: boolean;
  device_id: string;
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
  mismatches: [number, number, number][]; // [addr, expected, actual]
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

// ============================================================================
// Safe invoke wrapper — prevents crash when Tauri IPC is unavailable
// ============================================================================

/**
 * Safely call a Tauri command. If the Tauri IPC bridge is not available
 * (e.g., running in a regular browser or webview not fully initialized),
 * this throws a descriptive error instead of crashing with TypeError.
 */
async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e: unknown) {
    // Re-throw with more context
    if (e instanceof TypeError && String(e).includes('__TAURI_INTERNALS__')) {
      throw new Error(`Tauri IPC not available. Is this running inside Tauri webview? (command: ${cmd})`);
    }
    throw e;
  }
}

// ============================================================================
// Device Commands
// ============================================================================

/** Scan for available FT232H devices. Returns a list of device identifiers. */
export async function scanDevices(): Promise<string[]> {
  return safeInvoke('scan_devices');
}

/** Connect to a device and probe for PMIC/VCOM slaves. */
export async function connectDevice(deviceId: string, clockHz: number): Promise<DeviceInfo> {
  return safeInvoke('connect_device', { deviceId, clockHz });
}

/** Disconnect from the current device. */
export async function disconnectDevice(): Promise<void> {
  return safeInvoke('disconnect_device');
}

/** Re-probe for PMIC/VCOM slaves without reconnecting. */
export async function detectIc(): Promise<DeviceInfo> {
  return safeInvoke('detect_ic');
}

// ============================================================================
// Register Commands
// ============================================================================

/** Read a single DAC register and return enriched data. */
export async function readDacRegister(addr: number): Promise<RegisterData> {
  return safeInvoke('read_dac_register', { addr });
}

/** Write a single DAC register. */
export async function writeDacRegister(addr: number, value: number): Promise<void> {
  return safeInvoke('write_dac_register', { addr, value });
}

/** Read all DAC registers and return enriched data. */
export async function readAllDac(): Promise<RegisterData[]> {
  return safeInvoke('read_all_dac');
}

/** Read all EEPROM registers and return enriched data. */
export async function readAllEeprom(): Promise<RegisterData[]> {
  return safeInvoke('read_all_eeprom');
}

// ============================================================================
// Firmware Commands
// ============================================================================

/** Load and preview a firmware file without programming. */
export async function loadFirmware(path: string): Promise<FirmwarePreview> {
  return safeInvoke('load_firmware', { path });
}

/** Program firmware to device DAC registers, optionally writing to EEPROM. */
export async function programFirmware(path: string, writeEeprom: boolean): Promise<ProgramResult> {
  return safeInvoke('program_firmware', { path, writeEeprom });
}

/** Verify firmware against device EEPROM contents. */
export async function verifyFirmware(path: string): Promise<VerifyResult> {
  return safeInvoke('verify_firmware', { path });
}

/** Verify firmware against both DAC and EEPROM banks. */
export async function verifyAll(path: string): Promise<VerifyAllResult> {
  return safeInvoke('verify_all', { path });
}

/** Batch write DAC registers from (address, value) pairs. */
export async function writeAllDacRegisters(registers: [number, number][]): Promise<WriteAllDacResult> {
  return safeInvoke('write_all_dac_registers', { registers });
}

/** Export current EEPROM contents to a binary file. */
export async function exportEeprom(path: string): Promise<void> {
  return safeInvoke('export_eeprom', { path });
}

// ============================================================================
// EEPROM Commands
// ============================================================================

/** Write all DAC registers to EEPROM. */
export async function writeAllToEeprom(): Promise<void> {
  return safeInvoke('write_all_to_eeprom');
}

/** Write only VCOM1_NT to EEPROM. */
export async function writeVcom1ToEeprom(): Promise<void> {
  return safeInvoke('write_vcom1_to_eeprom');
}

/** Read fault flags from the VCOM slave. */
export async function readFaultFlags(): Promise<FaultFlags> {
  return safeInvoke('read_fault_flags');
}
