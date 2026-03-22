/**
 * EK86317A Register Map Definition
 * Complete register metadata for building the frontend UI.
 * Mirrors Rust backend's registers.rs definitions.
 */

import {
  decodeAvdd, decodeVbk1, decodeVgh, decodeVgl, decodeVss1,
  decodeHavdd, decodeVcomLimit, decodeVcom2dac,
  formatVoltage, formatVoltage1,
} from './voltage-calc';

// ============================================================================
// Types
// ============================================================================

export interface VoltageOption {
  value: number;      // Register hex value
  voltage: number;    // Corresponding voltage
  label: string;      // Display text, e.g. "16.0V"
}

export interface VoltageRegister {
  address: number;
  name: string;
  description: string;
  defaultValue: number;
  options: VoltageOption[];
}

// ============================================================================
// Register Address Constants (matching Rust)
// ============================================================================

export const REG_AVDD = 0x00;
export const REG_VBK1 = 0x01;
export const REG_HAVDD = 0x02;
export const REG_VGH_NT = 0x03;
export const REG_VGH_LT = 0x04;
export const REG_VGL_NT = 0x05;
export const REG_VGL_LT_HT = 0x06;
export const REG_VSS1 = 0x07;
export const REG_VCOM1_NT = 0x08;
export const REG_VCOM1_HT = 0x09;
export const REG_VCOM_MAX = 0x0a;
export const REG_VCOM_MIN = 0x0b;
export const REG_VCOM2DAC = 0x45;
export const REG_VCOM2DAC_EN = 0x46;

export const DEFAULT_AVDD_REG_VALUE = 0x00;
export const DEFAULT_VCOM1_NT_REG_VALUE = 0x7e;
export const DEFAULT_VCOM1_HT_REG_VALUE = 0xba;
export const DEFAULT_VCOM_MAX_REG_VALUE = 0x3f;
export const DEFAULT_VCOM_MIN_REG_VALUE = 0x26;
export const DEFAULT_VCOM2DAC_REG_VALUE = 0x7e;

// ============================================================================
// GAMMA constants
// ============================================================================

export const GAMMA_CHANNELS = 14;

/** Get the register addresses for a GAMMA channel (1-based index). */
export function gammaRegAddresses(channel: number): { high: number; low: number } {
  const base = 0x0c + (channel - 1) * 2;
  return { high: base, low: base + 1 };
}

// ============================================================================
// Voltage Option Generators
// ============================================================================

/** AVDD: 6-bit [5:0], 13.5V~19.8V, step 0.1V → 64 options */
export function generateAvddOptions(): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x3f; code++) {
    const v = decodeAvdd(code);
    options.push({ value: code, voltage: v, label: formatVoltage1(v) });
  }
  return options;
}

/** VBK1: 6-bit, non-linear lookup table.
 *  Segment 0 (0x00~0x0F): 1.80V~2.55V
 *  Segment 1 (0x10~0x1F): 2.60V~3.35V
 *  Segment 2 (0x20~0x2F): 0.80V~1.55V
 *  Segment 3 (0x30~0x3F): 1.60V~1.75V (only 0x30~0x33 are valid per spec, but we list all)
 */
export function generateVbk1Options(): VoltageOption[] {
  const options: VoltageOption[] = [];
  // Generate all valid VBK1 codes (0x00~0x3F)
  for (let code = 0; code <= 0x3f; code++) {
    const v = decodeVbk1(code);
    options.push({ value: code, voltage: v, label: v.toFixed(2) + 'V' });
  }
  // Sort by voltage for user-friendly display
  options.sort((a, b) => a.voltage - b.voltage);
  return options;
}

/** HAVDD: 7-bit [6:0], HAVDD = AVDD * DAC_CODE / 128 — depends on AVDD */
export function generateHavddOptions(avdd: number): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x7f; code++) {
    const v = decodeHavdd(code, avdd);
    options.push({ value: code, voltage: v, label: formatVoltage(v) });
  }
  return options;
}

/** VGH_NT: 5-bit [4:0], 20V~45V, step 1V → 26 options */
export function generateVghOptions(): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 25; code++) {
    const v = decodeVgh(code);
    options.push({ value: code, voltage: v, label: v.toFixed(0) + 'V' });
  }
  return options;
}

/** VGL_NT: 5-bit [4:0], -3V~-18V, step 0.5V → 31 options */
export function generateVglOptions(): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 30; code++) {
    const v = decodeVgl(code);
    options.push({ value: code, voltage: v, label: formatVoltage1(v) });
  }
  return options;
}

/** VSS1: [4:0], -3V~-16V, step 0.5V → 27 options */
export function generateVss1Options(): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 26; code++) {
    const v = decodeVss1(code);
    options.push({ value: code, voltage: v, label: formatVoltage1(v) });
  }
  return options;
}

/** VCOM_MAX / VCOM_MIN: 7-bit [6:0], VCOM_LIMIT = AVDD * DAC_CODE / 128 */
export function generateVcomLimitOptions(avdd: number): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x7f; code++) {
    const v = decodeVcomLimit(code, avdd);
    options.push({ value: code, voltage: v, label: formatVoltage(v) });
  }
  return options;
}

/** VCOM2DAC: 7-bit [7:1], same output range formula as VCOM1_NT */
export function generateVcom2dacOptions(vcomMinV: number, vcomMaxV: number): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x7f; code++) {
    const regValue = code << 1; // stored in [7:1]
    const v = decodeVcom2dac(regValue, vcomMinV, vcomMaxV);
    options.push({ value: regValue, voltage: v, label: formatVoltage(v) });
  }
  return options;
}

// ============================================================================
// Pre-generated static options (for registers without AVDD dependency)
// ============================================================================

export const AVDD_OPTIONS = generateAvddOptions();
export const VBK1_OPTIONS = generateVbk1Options();
export const VGH_OPTIONS = generateVghOptions();
export const VGL_OPTIONS = generateVglOptions();
export const VSS1_OPTIONS = generateVss1Options();

// ============================================================================
// SCL Clock Frequency Options
// ============================================================================

export interface ClockOption {
  value: number;
  label: string;
}

export const CLOCK_OPTIONS: ClockOption[] = [
  { value: 100_000, label: '100 kHz' },
  { value: 400_000, label: '400 kHz' },
  { value: 1_000_000, label: '1 MHz' },
];

// ============================================================================
// Voltage Register Definitions (for VoltageTab rows)
// ============================================================================

export interface VoltageRegisterDef {
  address: number;
  name: string;
  description: string;
  /** Whether this register has an enable bit */
  hasEnable?: boolean;
  /** Bit position of the enable bit */
  enableBit?: number;
  /** Separate register address for enable bit (if different from main address) */
  enableAddress?: number;
  /** Whether this register has a select bit (e.g., LT/HT) */
  hasSelect?: boolean;
  /** Bit position of the select bit */
  selectBit?: number;
  /** Bit mask for the voltage value portion */
  valueMask: number;
  /** Static options (non-AVDD dependent) or 'dynamic' for AVDD-dependent */
  staticOptions?: VoltageOption[];
  /** Whether options depend on AVDD */
  avddDependent?: boolean;
}

export const VOLTAGE_REGISTERS: VoltageRegisterDef[] = [
  {
    address: REG_AVDD,
    name: 'AVDD',
    description: '13.5V~19.8V, step 0.1V',
    valueMask: 0x3f,
    staticOptions: AVDD_OPTIONS,
  },
  {
    address: REG_VBK1,
    name: 'VBK1',
    description: '0.8V~3.35V, non-linear',
    valueMask: 0x3f,
    staticOptions: VBK1_OPTIONS,
  },
  {
    address: REG_HAVDD,
    name: 'HAVDD',
    description: 'AVDD × DAC/128',
    valueMask: 0x7f,
    avddDependent: true,
  },
  {
    address: REG_VGH_NT,
    name: 'VGH_NT',
    description: '20V~45V, step 1V',
    valueMask: 0x1f,
    staticOptions: VGH_OPTIONS,
  },
  {
    address: REG_VGH_LT,
    name: 'VGH_LT',
    description: '20V~45V, step 1V + Enable',
    hasEnable: true,
    enableBit: 7,
    valueMask: 0x1f,
    staticOptions: VGH_OPTIONS,
  },
  {
    address: REG_VGL_NT,
    name: 'VGL_NT',
    description: '-3V~-18V, step 0.5V',
    valueMask: 0x1f,
    staticOptions: VGL_OPTIONS,
  },
  {
    address: REG_VGL_LT_HT,
    name: 'VGL_LT/HT',
    description: '-3V~-18V + EN + LT/HT select',
    hasEnable: true,
    enableBit: 7,
    hasSelect: true,
    selectBit: 6,
    valueMask: 0x1f,
    staticOptions: VGL_OPTIONS,
  },
  {
    address: REG_VSS1,
    name: 'VSS1',
    description: '-3V~-16V, step 0.5V (bit7=VCOM1_HT_EN)',
    hasEnable: true,
    enableBit: 7,
    valueMask: 0x1f,
    staticOptions: VSS1_OPTIONS,
  },
  {
    address: REG_VCOM2DAC,
    name: 'VCOM2DAC',
    description: 'AVDD × DAC/128, [7:1], EN=0x46 bit0',
    hasEnable: true,
    enableBit: 0,
    enableAddress: REG_VCOM2DAC_EN,
    valueMask: 0xfe,
    avddDependent: true,
  },
];

/** Find a VoltageOption by register value in an options array. */
export function findOptionByValue(options: VoltageOption[], value: number): VoltageOption | undefined {
  return options.find(opt => opt.value === value);
}

/** Find a VoltageOption closest to a given voltage in an options array. */
export function findClosestOption(options: VoltageOption[], voltage: number): VoltageOption | undefined {
  if (options.length === 0) return undefined;
  let closest = options[0];
  let minDiff = Math.abs(voltage - closest.voltage);
  for (const opt of options) {
    const diff = Math.abs(voltage - opt.voltage);
    if (diff < minDiff) {
      closest = opt;
      minDiff = diff;
    }
  }
  return closest;
}
