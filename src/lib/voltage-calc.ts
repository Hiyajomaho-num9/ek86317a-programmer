/**
 * Voltage calculation utilities for EK86317A.
 * Mirrors the Rust decode/encode functions in src-tauri/src/ek86317a/registers.rs.
 */

// ============================================================================
// AVDD: 0x00, 6-bit [5:0], 13.5V~19.8V, step 0.1V
// ============================================================================

/** Decode AVDD register value to voltage. Formula: 13.5 + (value & 0x3F) * 0.1 */
export function decodeAvdd(value: number): number {
  return 13.5 + (value & 0x3f) * 0.1;
}

/** Encode AVDD voltage to register value. Returns null if out of range. */
export function encodeAvdd(voltage: number): number | null {
  if (voltage < 13.5 || voltage > 19.8) return null;
  const code = Math.round((voltage - 13.5) / 0.1);
  if (code > 0x3f) return null;
  return code;
}

// ============================================================================
// VBK1: 0x01, 6-bit [5:0], non-linear mapping
// ============================================================================

/** VBK1 segment base voltages indexed by bits [5:4] */
const VBK1_BASE = [1.80, 2.60, 0.80, 1.60];

/** Decode VBK1 register value to voltage (non-linear). */
export function decodeVbk1(value: number): number {
  const seg = (value >> 4) & 0x03;
  const offset = value & 0x0f;
  return VBK1_BASE[seg] + offset * 0.05;
}

/** Encode VBK1 voltage to register value. Returns null if out of range. */
export function encodeVbk1(voltage: number): number | null {
  for (let seg = 0; seg < 4; seg++) {
    const base = VBK1_BASE[seg];
    const maxV = base + 15 * 0.05;
    if (voltage >= base - 0.001 && voltage <= maxV + 0.001) {
      const offset = Math.round((voltage - base) / 0.05);
      if (offset >= 0 && offset <= 0x0f) {
        return (seg << 4) | offset;
      }
    }
  }
  return null;
}

// ============================================================================
// VGH: 5-bit [4:0], 20V~45V, step 1V
// ============================================================================

/** Decode VGH register value to voltage. Formula: 20 + (value & 0x1F) */
export function decodeVgh(value: number): number {
  return 20.0 + (value & 0x1f);
}

/** Encode VGH voltage to register value. */
export function encodeVgh(voltage: number): number | null {
  if (voltage < 20.0 || voltage > 45.0) return null;
  const code = Math.round(voltage - 20.0);
  if (code > 0x1f) return null;
  return code;
}

// ============================================================================
// VGL: 5-bit [4:0], -3V~-18V, step 0.5V
// ============================================================================

/** Decode VGL register value to voltage. Formula: -3.0 - (value & 0x1F) * 0.5 */
export function decodeVgl(value: number): number {
  return -3.0 - (value & 0x1f) * 0.5;
}

/** Encode VGL voltage to register value. */
export function encodeVgl(voltage: number): number | null {
  if (voltage > -3.0 || voltage < -18.0) return null;
  const code = Math.round((-3.0 - voltage) / 0.5);
  if (code > 0x1f) return null;
  return code;
}

// ============================================================================
// VSS1: [4:0], -3V~-16V, step 0.5V
// ============================================================================

/** Decode VSS1 register value to voltage. Formula: -3.0 - (value & 0x1F) * 0.5 */
export function decodeVss1(value: number): number {
  return -3.0 - (value & 0x1f) * 0.5;
}

/** Encode VSS1 voltage to register value. */
export function encodeVss1(voltage: number): number | null {
  if (voltage > -3.0 || voltage < -16.0) return null;
  const code = Math.round((-3.0 - voltage) / 0.5);
  if (code > 0x1f) return null;
  return code;
}

// ============================================================================
// HAVDD: 7-bit [6:0], HAVDD = AVDD * DAC_CODE / 128
// ============================================================================

/** Decode HAVDD register value to voltage. Formula: AVDD * (value & 0x7F) / 128 */
export function decodeHavdd(value: number, avdd: number): number {
  return avdd * (value & 0x7f) / 128.0;
}

/** Encode HAVDD voltage to register value. */
export function encodeHavdd(voltage: number, avdd: number): number | null {
  if (avdd <= 0) return null;
  const code = Math.round(voltage * 128.0 / avdd);
  if (code < 0 || code > 0x7f) return null;
  return code;
}

// ============================================================================
// VCOM Limit: VCOM_MAX/VCOM_MIN, 7-bit [6:0]
// Formula: VCOM_LIMIT = AVDD * DAC_CODE / 128
// ============================================================================

/** Decode VCOM limit register value to voltage. */
export function decodeVcomLimit(value: number, avdd: number): number {
  return avdd * (value & 0x7f) / 128.0;
}

/** Encode VCOM limit voltage to register value. */
export function encodeVcomLimit(voltage: number, avdd: number): number | null {
  if (avdd <= 0) return null;
  const code = Math.round(voltage * 128.0 / avdd);
  if (code < 0 || code > 0x7f) return null;
  return code;
}

// ============================================================================
// VCOM1_NT: 7-bit [7:1], bit0 reserved
// Simple formula (used by Rust backend): VCOM = AVDD * DAC_CODE / 128
//   where DAC_CODE = (value >> 1) & 0x7F
//
// Complex formula (for UI with VCOM_MIN/VCOM_MAX):
//   VCOM1_V = (VCOM_MAX_V - VCOM_MIN_V) * DAC_CODE / 127 + VCOM_MIN_V
// ============================================================================

/** Decode VCOM1_NT using AVDD directly (backend compatible). */
export function decodeVcom(value: number, avdd: number): number {
  return avdd * ((value >> 1) & 0x7f) / 128.0;
}

/** Encode VCOM voltage to register value (stored in [7:1]). */
export function encodeVcom(voltage: number, avdd: number): number | null {
  if (avdd <= 0) return null;
  const code = Math.round(voltage * 128.0 / avdd);
  if (code < 0 || code > 0x7f) return null;
  return code << 1;
}

function normalizeVcomRange(vcomMinV: number, vcomMaxV: number): [number, number] {
  return vcomMinV <= vcomMaxV ? [vcomMinV, vcomMaxV] : [vcomMaxV, vcomMinV];
}

/** Decode VCOM1_NT using VCOM_MIN and VCOM_MAX range.
 *  VCOM1_V = (VCOM_MAX_V - VCOM_MIN_V) * DAC_CODE / 127 + VCOM_MIN_V
 */
export function decodeVcom1Nt(value: number, vcomMinV: number, vcomMaxV: number): number {
  const [minV, maxV] = normalizeVcomRange(vcomMinV, vcomMaxV);
  const dacCode = (value >> 1) & 0x7f;
  return (maxV - minV) * dacCode / 127.0 + minV;
}

/** Encode VCOM1_NT voltage using VCOM_MIN/MAX range. Returns register value with [7:1]. */
export function encodeVcom1Nt(voltage: number, vcomMinV: number, vcomMaxV: number): number | null {
  const [minV, maxV] = normalizeVcomRange(vcomMinV, vcomMaxV);
  const range = maxV - minV;
  if (range <= 0) return null;
  const clamped = Math.min(maxV, Math.max(minV, voltage));
  const dacCode = Math.round((clamped - minV) / range * 127.0);
  if (dacCode < 0 || dacCode > 127) return null;
  return dacCode << 1;
}

// ============================================================================
// VCOM2DAC: 7-bit [7:1], same formula as VCOM
// ============================================================================

/** Decode VCOM2DAC register value to voltage using the datasheet VCOM range formula. */
export function decodeVcom2dac(value: number, vcomMinV: number, vcomMaxV: number): number {
  return decodeVcom1Nt(value, vcomMinV, vcomMaxV);
}

/** Encode VCOM2DAC voltage to register value. */
export function encodeVcom2dac(voltage: number, vcomMinV: number, vcomMaxV: number): number | null {
  return encodeVcom1Nt(voltage, vcomMinV, vcomMaxV);
}

// ============================================================================
// GAMMA: 10-bit from high[1:0] + low[7:0]
// Formula: GAMMA_V = AVDD * DAC_CODE / 1024
// ============================================================================

/** Decode GAMMA register pair to voltage. */
export function decodeGamma(high: number, low: number, avdd: number): number {
  const dacCode = ((high & 0x03) << 8) | (low & 0xff);
  return avdd * dacCode / 1024.0;
}

/** Encode GAMMA voltage to register pair {high, low}. */
export function encodeGamma(voltage: number, avdd: number): { high: number; low: number } | null {
  if (avdd <= 0) return null;
  const dacCode = Math.round(voltage * 1024.0 / avdd);
  if (dacCode < 0 || dacCode > 0x3ff) return null;
  return {
    high: (dacCode >> 8) & 0x03,
    low: dacCode & 0xff,
  };
}

/** Get 10-bit DAC code from GAMMA register pair. */
export function gammaRegsToDacCode(high: number, low: number): number {
  return ((high & 0x03) << 8) | (low & 0xff);
}

/** Split 10-bit DAC code into GAMMA register pair. */
export function dacCodeToGammaRegs(dacCode: number): { high: number; low: number } {
  return {
    high: (dacCode >> 8) & 0x03,
    low: dacCode & 0xff,
  };
}

// ============================================================================
// Formatting Utilities
// ============================================================================

/** Format voltage with 3 decimal places and 'V' suffix. */
export function formatVoltage(v: number): string {
  return v.toFixed(3) + 'V';
}

/** Format voltage with 1 decimal place and 'V' suffix (for VGH/VGL). */
export function formatVoltage1(v: number): string {
  return v.toFixed(1) + 'V';
}

/** Format a number as hex string with optional width (e.g., "0x1A"). */
export function formatHex(v: number, width: number = 2): string {
  return '0x' + v.toString(16).toUpperCase().padStart(width, '0');
}
