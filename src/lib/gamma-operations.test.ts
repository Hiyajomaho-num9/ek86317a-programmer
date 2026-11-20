import { describe, expect, it, vi } from 'vitest';
import { readAllGamma, writeAllGamma } from './gamma-operations';

describe('GAMMA transactions', () => {
  it('reads high then low for channels 1 through 14 and commits once', async () => {
    const events: string[] = [];
    const onRegisters = vi.fn();
    await readAllGamma(onRegisters, {
      read: async (address) => { events.push(`read:${address}`); return address & 0xff; },
      write: async () => undefined,
    });
    expect(events).toEqual(Array.from({ length: 28 }, (_, index) => `read:${0x0c + index}`));
    expect(onRegisters).toHaveBeenCalledTimes(1);
    expect(onRegisters.mock.calls[0][0]).toEqual(Array.from({ length: 28 }, (_, index) => [0x0c + index, 0x0c + index]));
  });

  it('writes high then low, using the same display fallback for unread values', async () => {
    const values: [number, number][] = [];
    await writeAllGamma((address) => address === 0x0c ? 2 : undefined, {
      read: async () => 0,
      write: async (address, value) => { values.push([address, value]); },
    });
    expect(values).toEqual(Array.from({ length: 28 }, (_, index) => [0x0c + index, index === 0 ? 2 : 0]));
    const imlValues: [number, number][] = [];
    await writeAllGamma(() => undefined, {
      read: async () => 0,
      write: async (address, value) => { imlValues.push([address, value]); },
    }, 'iml8947k');
    expect(imlValues).toHaveLength(28);
    expect(imlValues.every(([, value]) => value === 0)).toBe(true);
  });

  it('does not send the last register after the penultimate read fails', async () => {
    const read = vi.fn(async (address: number) => {
      if (address === 0x26) throw new Error('NACK');
      return address;
    });
    const commit = vi.fn();
    await expect(readAllGamma(commit, { read, write: vi.fn() })).rejects.toThrow('NACK');
    expect(read).toHaveBeenCalledTimes(27);
    expect(read).not.toHaveBeenCalledWith(0x27);
    expect(commit).not.toHaveBeenCalled();
  });

  it('stops the write batch after an access fails', async () => {
    const write = vi.fn(async (address: number) => {
      if (address === 0x26) throw new Error('NACK');
    });
    await expect(writeAllGamma(() => 1, { read: vi.fn(), write })).rejects.toThrow('NACK');
    expect(write).toHaveBeenCalledTimes(27);
    expect(write).not.toHaveBeenCalledWith(0x27, 1);
  });
});
