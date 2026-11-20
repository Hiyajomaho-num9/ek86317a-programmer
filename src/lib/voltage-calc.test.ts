import { describe, expect, it } from 'vitest';
import {
  decodeAvdd,
  decodeGamma,
  decodeHavdd,
  decodeVcom,
  decodeVgl,
  decodeVgh,
  dacCodeToGammaRegs,
  encodeAvdd,
  encodeGamma,
  encodeVcom,
  encodeVgl,
  encodeVgh,
  gammaRegsToDacCode,
} from './voltage-calc';

describe('voltage regressions', () => {
  it('decodes and encodes AVDD with its low six-bit DAC code', () => {
    expect(decodeAvdd(0x00)).toBe(13.5);
    expect(decodeAvdd(0x3f)).toBeCloseTo(19.8, 10);
    expect(decodeAvdd(0x40 | 0x29)).toBeCloseTo(17.6, 10);
    expect(encodeAvdd(13.5)).toBe(0);
    expect(encodeAvdd(19.8)).toBe(0x3f);
    expect(encodeAvdd(13.49)).toBeNull();
    expect(encodeAvdd(19.81)).toBeNull();
  });

  it('keeps VGH, VGL, HAVDD and VCOM conversions compatible', () => {
    expect(decodeVgh(0)).toBe(20);
    expect(decodeVgh(0x19)).toBe(45);
    expect(encodeVgh(20)).toBe(0);
    expect(encodeVgh(45)).toBe(0x19);
    expect(encodeVgh(46)).toBeNull();
    expect(encodeVgh(42, 'lp6281')).toBe(22);
    expect(encodeVgh(43, 'lp6281')).toBeNull();

    expect(decodeVgl(0)).toBe(-3);
    expect(decodeVgl(0x1e)).toBe(-18);
    expect(encodeVgl(-3)).toBe(0);
    expect(encodeVgl(-18)).toBe(30);
    expect(encodeVgl(-18.5)).toBeNull();

    expect(decodeHavdd(0, 17.6)).toBeCloseTo(6.6, 10);
    expect(decodeVcom(0, 17.6)).toBe(0);
    expect(decodeVcom(0xfe, 17.6)).toBeCloseTo(17.4625, 10);
    expect(encodeVcom(0, 17.6)).toBe(0);
    expect(encodeVcom(17.6 * 127 / 128, 17.6)).toBe(0xfe);
    expect(encodeVcom(1, 0)).toBeNull();
  });
});

describe('Gamma high and low byte regressions', () => {
  it.each([
    { code: 0x000, high: 0x00, low: 0x00 },
    { code: 0x001, high: 0x00, low: 0x01 },
    { code: 0x0ff, high: 0x00, low: 0xff },
    { code: 0x100, high: 0x01, low: 0x00 },
    { code: 0x1ab, high: 0x01, low: 0xab },
    { code: 0x2ff, high: 0x02, low: 0xff },
    { code: 0x3ff, high: 0x03, low: 0xff },
  ])('splits $code into high=$high and low=$low and recombines it', ({ code, high, low }) => {
    expect(dacCodeToGammaRegs(code)).toEqual({ high, low });
    expect(gammaRegsToDacCode(high, low)).toBe(code);
  });

  it('only uses the high register lowest two bits', () => {
    expect(gammaRegsToDacCode(0xfc | 0x03, 0xff)).toBe(0x3ff);
    expect(gammaRegsToDacCode(0xfc | 0x01, 0x34)).toBe(0x134);
    expect(gammaRegsToDacCode(0x00, 0x1ff)).toBe(0xff);
    expect(dacCodeToGammaRegs(0x123)).toEqual({ high: 1, low: 0x23 });
  });

  it('computes voltage using the full ten-bit code rather than the low byte only', () => {
    expect(decodeGamma(0x01, 0x00, 16)).toBe(4);
    expect(decodeGamma(0x03, 0xff, 16)).toBeCloseTo(15.984375, 10);
    expect(encodeGamma(4, 16)).toEqual({ high: 1, low: 0 });
    expect(encodeGamma(0, 16)).toEqual({ high: 0, low: 0 });
    expect(encodeGamma(16 * 1023 / 1024, 16)).toEqual({ high: 3, low: 0xff });
    expect(encodeGamma(-0.01, 16)).toBeNull();
    expect(encodeGamma(16.1, 16)).toBeNull();
    expect(encodeGamma(1, 0)).toBeNull();
  });
});
