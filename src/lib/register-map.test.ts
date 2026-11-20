import { describe, expect, it } from 'vitest';
import type { ChipModel } from './chips';
import {
  CLOCK_OPTIONS,
  GAMMA_CHANNELS,
  gammaRegAddresses,
  generateAvddOptions,
  generateVghOptions,
  getDefaultRegisterValue,
  getPrimaryVcomAddress,
  getPrimaryVcomName,
  getSecondaryVcomName,
  getVoltageRegisters,
} from './register-map';

// Preserve the original hardware layout and ascending UI order, not source insertion order.
const ORIGINAL_DEFAULTS: Record<ChipModel, [number, number][]> = {
  ek86317a: [
    [0x00, 0x29], [0x01, 0x1e], [0x02, 0x40], [0x03, 0x0a],
    [0x04, 0x0f], [0x05, 0x0a], [0x06, 0x12], [0x07, 0x06],
    [0x08, 0x7e], [0x09, 0xba], [0x0a, 0x3f], [0x0b, 0x26],
    [0x31, 0x02], [0x32, 0x00], [0x45, 0x7e], [0x46, 0x01],
  ],
  iml8947k: [
    [0x00, 0x29], [0x01, 0x1e], [0x02, 0x40], [0x03, 0x0a],
    [0x04, 0x0f], [0x05, 0x0a], [0x06, 0x12], [0x07, 0x06],
    [0x08, 0x7e], [0x0a, 0x3f], [0x0b, 0x26],
    [0x31, 0x02], [0x45, 0x7e], [0x46, 0x01],
  ],
  lp6281: [
    [0x00, 0x29], [0x01, 0x1e], [0x02, 0x40], [0x03, 0x0a],
    [0x04, 0x0f], [0x05, 0x0a], [0x06, 0x12], [0x07, 0x06],
    [0x08, 0x7e], [0x09, 0xba], [0x0a, 0x3f], [0x0b, 0x26],
  ],
};

describe('register-map regressions', () => {
  it.each(Object.keys(ORIGINAL_DEFAULTS) as ChipModel[])(
    'preserves every original default and ascending register order for %s',
    (chipModel) => {
      const defaults = ORIGINAL_DEFAULTS[chipModel];
      const registers = getVoltageRegisters(chipModel, new Map());

      for (const [address, value] of defaults) {
        expect(getDefaultRegisterValue(chipModel, address), `register 0x${address.toString(16)}`).toBe(value);
      }
      expect(registers.map(({ address }) => address)).toEqual(
        [...registers.map(({ address }) => address)].sort((left, right) => left - right),
      );
      expect(registers.map(({ address }) => address)).toEqual(
        defaults.map(([address]) => address).filter((address) => address <= 0x07 || address === 0x45),
      );
      expect(getDefaultRegisterValue(chipModel, 0xff)).toBe(0);
    },
  );

  it('keeps original voltage ordering, VCOM names and clock ordering', () => {
    expect(generateAvddOptions('ek86317a').map(({ voltage }) => voltage)).toEqual(
      [...generateAvddOptions('ek86317a').map(({ voltage }) => voltage)].sort((a, b) => a - b),
    );
    const ekVgh = generateVghOptions('ek86317a');
    const lpVgh = generateVghOptions('lp6281');
    expect(ekVgh[ekVgh.length - 1]?.value).toBe(25);
    expect(lpVgh[lpVgh.length - 1]?.value).toBe(22);
    expect(getPrimaryVcomAddress('lp6281')).toBe(0x08);
    expect(getPrimaryVcomName('lp6281')).toBe('VCOM_NT');
    expect(getSecondaryVcomName('lp6281')).toBe('VCOM_HT');
    expect(CLOCK_OPTIONS.map(({ value }) => value)).toEqual([100_000, 400_000, 1_000_000]);
  });

  it('maps 14 channels to consecutive high/low addresses from 0x0c through 0x27', () => {
    const pairs = Array.from({ length: GAMMA_CHANNELS }, (_, index) => gammaRegAddresses(index + 1));

    expect(GAMMA_CHANNELS).toBe(14);
    expect(pairs[0]).toEqual({ high: 0x0c, low: 0x0d });
    expect(pairs[pairs.length - 1]).toEqual({ high: 0x26, low: 0x27 });
    expect(pairs.flatMap(({ high, low }) => [high, low])).toEqual(
      Array.from({ length: 28 }, (_, index) => 0x0c + index),
    );
  });
});
