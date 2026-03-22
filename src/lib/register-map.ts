import type { ChipModel } from './chips';
import { getChipCapabilities } from './chips';
import {
  decodeAvdd,
  decodeVbk1,
  decodeVgh,
  decodeVgl,
  decodeVss1,
  decodeHavdd,
  decodeVcomLimit,
  decodeVcom2dac,
  formatVoltage,
  formatVoltage1,
} from './voltage-calc';

export interface VoltageOption {
  value: number;
  voltage: number;
  label: string;
}

export interface VoltageRegisterDef {
  address: number;
  name: string;
  description: string;
  hasEnable?: boolean;
  enableBit?: number;
  enableAddress?: number;
  hasSelect?: boolean;
  selectBit?: number;
  valueMask: number;
  staticOptions?: VoltageOption[];
}

export interface ClockOption {
  value: number;
  label: string;
}

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
export const REG_CONFIG3 = 0x31;
export const REG_DISCHARGE2 = 0x32;
export const REG_VCOM2DAC = 0x45;
export const REG_CONFIG4 = 0x46;

const DEFAULTS: Record<ChipModel, Record<number, number>> = {
  ek86317a: {
    [REG_AVDD]: 0x29,
    [REG_VBK1]: 0x1e,
    [REG_HAVDD]: 0x40,
    [REG_VGH_NT]: 0x0a,
    [REG_VGH_LT]: 0x0f,
    [REG_VGL_NT]: 0x0a,
    [REG_VGL_LT_HT]: 0x12,
    [REG_VSS1]: 0x06,
    [REG_VCOM1_NT]: 0x7e,
    [REG_VCOM1_HT]: 0xba,
    [REG_VCOM_MAX]: 0x3f,
    [REG_VCOM_MIN]: 0x26,
    [REG_CONFIG3]: 0x02,
    [REG_DISCHARGE2]: 0x00,
    [REG_VCOM2DAC]: 0x7e,
    [REG_CONFIG4]: 0x01,
  },
  iml8947k: {
    [REG_AVDD]: 0x29,
    [REG_VBK1]: 0x1e,
    [REG_HAVDD]: 0x40,
    [REG_VGH_NT]: 0x0a,
    [REG_VGH_LT]: 0x0f,
    [REG_VGL_NT]: 0x0a,
    [REG_VGL_LT_HT]: 0x12,
    [REG_VSS1]: 0x06,
    [REG_VCOM1_NT]: 0x7e,
    [REG_VCOM_MAX]: 0x3f,
    [REG_VCOM_MIN]: 0x26,
    [REG_CONFIG3]: 0x02,
    [REG_VCOM2DAC]: 0x7e,
    [REG_CONFIG4]: 0x01,
  },
  lp6281: {
    [REG_AVDD]: 0x29,
    [REG_VBK1]: 0x1e,
    [REG_HAVDD]: 0x40,
    [REG_VGH_NT]: 0x0a,
    [REG_VGH_LT]: 0x0f,
    [REG_VGL_NT]: 0x0a,
    [REG_VGL_LT_HT]: 0x12,
    [REG_VSS1]: 0x06,
    [REG_VCOM1_NT]: 0x7e,
    [REG_VCOM1_HT]: 0xba,
    [REG_VCOM_MAX]: 0x3f,
    [REG_VCOM_MIN]: 0x26,
  },
};

export const GAMMA_CHANNELS = 14;

export function gammaRegAddresses(channel: number): { high: number; low: number } {
  const base = 0x0c + (channel - 1) * 2;
  return { high: base, low: base + 1 };
}

export function getDefaultRegisterValue(chipModel: ChipModel, addr: number): number {
  return DEFAULTS[chipModel][addr] ?? 0;
}

export function generateAvddOptions(chipModel: ChipModel, modeValue?: number): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x3f; code += 1) {
    const voltage = decodeAvdd(code, chipModel, modeValue);
    options.push({ value: code, voltage, label: formatVoltage1(voltage) });
  }
  return options;
}

export function generateVbk1Options(chipModel: ChipModel): VoltageOption[] {
  const options: VoltageOption[] = [];
  const maxCode = chipModel === 'lp6281' ? 0x1f : 0x3f;
  for (let code = 0; code <= maxCode; code += 1) {
    const voltage = decodeVbk1(code, chipModel);
    options.push({ value: code, voltage, label: voltage.toFixed(2) + 'V' });
  }
  options.sort((left, right) => left.voltage - right.voltage);
  return options;
}

export function generateHavddOptions(avdd: number): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x7f; code += 1) {
    const voltage = decodeHavdd(code, avdd);
    options.push({ value: code, voltage, label: formatVoltage(voltage) });
  }
  return options;
}

export function generateVghOptions(chipModel: ChipModel): VoltageOption[] {
  const options: VoltageOption[] = [];
  const maxCode = chipModel === 'lp6281' ? 22 : 25;
  for (let code = 0; code <= maxCode; code += 1) {
    const voltage = decodeVgh(code);
    options.push({ value: code, voltage, label: voltage.toFixed(0) + 'V' });
  }
  return options;
}

export function generateVglOptions(): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 30; code += 1) {
    const voltage = decodeVgl(code);
    options.push({ value: code, voltage, label: formatVoltage1(voltage) });
  }
  return options;
}

export function generateVss1Options(): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 26; code += 1) {
    const voltage = decodeVss1(code);
    options.push({ value: code, voltage, label: formatVoltage1(voltage) });
  }
  return options;
}

export function generateVcomLimitOptions(avdd: number): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x7f; code += 1) {
    const voltage = decodeVcomLimit(code, avdd);
    options.push({ value: code, voltage, label: formatVoltage(voltage) });
  }
  return options;
}

export function generateVcom2dacOptions(vcomMinV: number, vcomMaxV: number): VoltageOption[] {
  const options: VoltageOption[] = [];
  for (let code = 0; code <= 0x7f; code += 1) {
    const regValue = code << 1;
    const voltage = decodeVcom2dac(regValue, vcomMinV, vcomMaxV);
    options.push({ value: regValue, voltage, label: formatVoltage(voltage) });
  }
  return options;
}

export const CLOCK_OPTIONS: ClockOption[] = [
  { value: 100_000, label: '100 kHz' },
  { value: 400_000, label: '400 kHz' },
  { value: 1_000_000, label: '1 MHz' },
];

export function getVoltageRegisters(
  chipModel: ChipModel,
  registers: Map<number, number>,
): VoltageRegisterDef[] {
  const modeValue = registers.get(REG_CONFIG4) ?? getDefaultRegisterValue(chipModel, REG_CONFIG4);
  const caps = getChipCapabilities(chipModel);
  const base: VoltageRegisterDef[] = [
    {
      address: REG_AVDD,
      name: 'AVDD',
      description: chipModel === 'iml8947k' && ((modeValue & 0x80) !== 0)
        ? '11.0V~17.3V, step 0.1V (MNT mode)'
        : '13.5V~19.8V, step 0.1V',
      valueMask: 0x3f,
      staticOptions: generateAvddOptions(chipModel, modeValue),
    },
    {
      address: REG_VBK1,
      name: 'VBK1',
      description: chipModel === 'lp6281' ? '1.8V~3.35V, linear 0.05V' : 'family VBK1 table',
      valueMask: chipModel === 'lp6281' ? 0x1f : 0x3f,
      staticOptions: generateVbk1Options(chipModel),
    },
    {
      address: REG_HAVDD,
      name: 'HAVDD',
      description: 'AVDD / 512 * (code + 192)',
      valueMask: 0x7f,
    },
    {
      address: REG_VGH_NT,
      name: 'VGH_NT',
      description: chipModel === 'lp6281' ? '20V~42V, step 1V' : '20V~45V, step 1V',
      valueMask: 0x1f,
      staticOptions: generateVghOptions(chipModel),
    },
    {
      address: REG_VGH_LT,
      name: 'VGH_LT',
      description: 'VGH low-temp value with enable bit',
      hasEnable: true,
      enableBit: 7,
      valueMask: 0x1f,
      staticOptions: generateVghOptions(chipModel),
    },
    {
      address: REG_VGL_NT,
      name: 'VGL_NT',
      description: '-3V~-18V, step 0.5V',
      valueMask: 0x1f,
      staticOptions: generateVglOptions(),
    },
    {
      address: REG_VGL_LT_HT,
      name: 'VGL_LT_HT',
      description: '-3V~-18V with enable and LT/HT select bits',
      hasEnable: true,
      enableBit: 7,
      hasSelect: true,
      selectBit: 6,
      valueMask: 0x1f,
      staticOptions: generateVglOptions(),
    },
    {
      address: REG_VSS1,
      name: 'VSS1',
      description: '-3V~-16V, step 0.5V',
      hasEnable: chipModel !== 'lp6281',
      enableBit: 7,
      valueMask: 0x1f,
      staticOptions: generateVss1Options(),
    },
  ];

  if (caps.supportsVcom2dac) {
    base.push({
      address: REG_VCOM2DAC,
      name: 'VCOM2DAC',
      description: caps.supportsMntMode ? 'Range-based VCOM2DAC, enable on 0x46 bit0' : 'Range-based VCOM2DAC',
      hasEnable: true,
      enableBit: 0,
      enableAddress: REG_CONFIG4,
      valueMask: 0xfe,
    });
  }

  return base;
}

export function getPrimaryVcomAddress(_chipModel: ChipModel): number {
  return REG_VCOM1_NT;
}

export function getPrimaryVcomName(chipModel: ChipModel): string {
  return chipModel === 'lp6281' ? 'VCOM_NT' : 'VCOM1_NT';
}

export function getSecondaryVcomName(chipModel: ChipModel): string {
  return chipModel === 'lp6281' ? 'VCOM_HT' : 'VCOM1_HT';
}
