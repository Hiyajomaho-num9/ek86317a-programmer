import type { ChipModel } from './chips';

const EK_VBK1_BASE = [1.8, 2.6, 0.8, 1.6];

export function isMntMode(modeValue: number | undefined): boolean {
  return ((modeValue ?? 0x01) & 0x80) !== 0;
}

export function decodeAvdd(
  value: number,
  chipModel: ChipModel = 'ek86317a',
  modeValue?: number,
): number {
  if (chipModel === 'iml8947k' && isMntMode(modeValue)) {
    return 11.0 + (value & 0x3f) * 0.1;
  }
  return 13.5 + (value & 0x3f) * 0.1;
}

export function encodeAvdd(
  voltage: number,
  chipModel: ChipModel = 'ek86317a',
  modeValue?: number,
): number | null {
  const base = chipModel === 'iml8947k' && isMntMode(modeValue) ? 11.0 : 13.5;
  const max = base + 0x3f * 0.1;
  if (voltage < base || voltage > max) return null;
  return Math.round((voltage - base) / 0.1);
}

export function decodeVbk1(value: number, chipModel: ChipModel = 'ek86317a'): number {
  if (chipModel === 'lp6281') {
    return 1.8 + (value & 0x1f) * 0.05;
  }
  const seg = (value >> 4) & 0x03;
  const offset = value & 0x0f;
  return EK_VBK1_BASE[seg] + offset * 0.05;
}

export function decodeVgh(value: number): number {
  return 20.0 + (value & 0x1f);
}

export function encodeVgh(voltage: number, chipModel: ChipModel = 'ek86317a'): number | null {
  const max = chipModel === 'lp6281' ? 42.0 : 45.0;
  if (voltage < 20.0 || voltage > max) return null;
  const code = Math.round(voltage - 20.0);
  return code >= 0 && code <= 0x1f ? code : null;
}

export function decodeVgl(value: number): number {
  return -3.0 - (value & 0x1f) * 0.5;
}

export function encodeVgl(voltage: number): number | null {
  if (voltage > -3.0 || voltage < -18.0) return null;
  const code = Math.round((-3.0 - voltage) / 0.5);
  return code >= 0 && code <= 0x1f ? code : null;
}

export function decodeVss1(value: number): number {
  return -3.0 - (value & 0x1f) * 0.5;
}

export function encodeVss1(voltage: number): number | null {
  if (voltage > -3.0 || voltage < -16.0) return null;
  const code = Math.round((-3.0 - voltage) / 0.5);
  return code >= 0 && code <= 0x1f ? code : null;
}

export function decodeHavdd(value: number, avdd: number): number {
  return avdd / 512.0 * ((value & 0x7f) + 192.0);
}

export function encodeHavdd(voltage: number, avdd: number): number | null {
  if (avdd <= 0) return null;
  const code = Math.round((voltage * 512.0) / avdd - 192.0);
  return code >= 0 && code <= 0x7f ? code : null;
}

export function decodeVcomLimit(value: number, avdd: number): number {
  return avdd * (value & 0x7f) / 128.0;
}

export function encodeVcomLimit(voltage: number, avdd: number): number | null {
  if (avdd <= 0) return null;
  const code = Math.round(voltage * 128.0 / avdd);
  return code >= 0 && code <= 0x7f ? code : null;
}

function normalizeVcomRange(vcomMinV: number, vcomMaxV: number): [number, number] {
  return vcomMinV <= vcomMaxV ? [vcomMinV, vcomMaxV] : [vcomMaxV, vcomMinV];
}

export function decodeVcom(value: number, avdd: number): number {
  return avdd * ((value >> 1) & 0x7f) / 128.0;
}

export function encodeVcom(voltage: number, avdd: number): number | null {
  if (avdd <= 0) return null;
  const code = Math.round(voltage * 128.0 / avdd);
  return code >= 0 && code <= 0x7f ? code << 1 : null;
}

export function decodeVcom1Nt(value: number, vcomMinV: number, vcomMaxV: number): number {
  const [minV, maxV] = normalizeVcomRange(vcomMinV, vcomMaxV);
  const dacCode = (value >> 1) & 0x7f;
  return (maxV - minV) * dacCode / 127.0 + minV;
}

export function encodeVcom1Nt(voltage: number, vcomMinV: number, vcomMaxV: number): number | null {
  const [minV, maxV] = normalizeVcomRange(vcomMinV, vcomMaxV);
  const range = maxV - minV;
  if (range <= 0) return null;
  const clamped = Math.min(maxV, Math.max(minV, voltage));
  const dacCode = Math.round((clamped - minV) / range * 127.0);
  return dacCode >= 0 && dacCode <= 127 ? dacCode << 1 : null;
}

export function decodeVcom2dac(value: number, vcomMinV: number, vcomMaxV: number): number {
  return decodeVcom1Nt(value, vcomMinV, vcomMaxV);
}

export function encodeVcom2dac(voltage: number, vcomMinV: number, vcomMaxV: number): number | null {
  return encodeVcom1Nt(voltage, vcomMinV, vcomMaxV);
}

export function decodeGamma(high: number, low: number, avdd: number): number {
  const dacCode = ((high & 0x03) << 8) | (low & 0xff);
  return avdd * dacCode / 1024.0;
}

export function encodeGamma(voltage: number, avdd: number): { high: number; low: number } | null {
  if (avdd <= 0) return null;
  const dacCode = Math.round(voltage * 1024.0 / avdd);
  if (dacCode < 0 || dacCode > 0x3ff) return null;
  return {
    high: (dacCode >> 8) & 0x03,
    low: dacCode & 0xff,
  };
}

export function gammaRegsToDacCode(high: number, low: number): number {
  return ((high & 0x03) << 8) | (low & 0xff);
}

export function dacCodeToGammaRegs(dacCode: number): { high: number; low: number } {
  return {
    high: (dacCode >> 8) & 0x03,
    low: dacCode & 0xff,
  };
}

export function formatVoltage(v: number): string {
  return `${v.toFixed(3)}V`;
}

export function formatVoltage1(v: number): string {
  return `${v.toFixed(1)}V`;
}

export function formatHex(v: number, width = 2): string {
  return `0x${v.toString(16).toUpperCase().padStart(width, '0')}`;
}
