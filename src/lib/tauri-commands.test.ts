import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { readAllDac, readDacRegister, writeDacRegister } from './tauri-commands';
import { readAllGamma, writeAllGamma } from './gamma-operations';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe('hardware IPC boundary', () => {
  // A block returns void; returning the mock registers it as a Vitest cleanup callback.
  beforeEach(() => { mockedInvoke.mockReset(); });

  it('serializes separate hardware commands and preserves their DTO arguments', async () => {
    const first = deferred<unknown>();
    const events: string[] = [];
    mockedInvoke.mockImplementation((command: string) => {
      events.push(`start:${command}`);
      return (command === 'read_dac_register' ? first.promise : Promise.resolve([])) as never;
    });
    const pending = readDacRegister(0x0c);
    const batch = readAllDac();
    await vi.waitFor(() => expect(events).toEqual(['start:read_dac_register']));
    first.resolve({ address: 0x0c, value: 2, name: 'GAMMA1_H', voltage: null });
    await expect(pending).resolves.toMatchObject({ value: 2 });
    await expect(batch).resolves.toEqual([]);
    expect(events).toEqual(['start:read_dac_register', 'start:read_all_dac']);
  });

  it('continues the queue after an IPC error and normalizes string errors', async () => {
    mockedInvoke.mockRejectedValueOnce('I2C NACK').mockResolvedValueOnce(undefined);
    await expect(writeDacRegister(0x0c, 2)).rejects.toThrow('I2C NACK');
    await expect(writeDacRegister(0x0d, 0xff)).resolves.toBeUndefined();
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, 'write_dac_register', { addr: 0x0d, value: 0xff });
  });

  it('does not interleave legacy panel IPC with a whole GAMMA transaction', async () => {
    const gates: ReturnType<typeof deferred<unknown>>[] = [];
    const events: string[] = [];
    mockedInvoke.mockImplementation((command: string, args?: unknown) => {
      if (command === 'read_dac_register') {
        const address = (args as { addr: number }).addr;
        const gate = deferred<unknown>();
        gates.push(gate);
        events.push(`gamma:${address}`);
        return gate.promise as never;
      }
      if (command === 'read_all_dac') {
        events.push('voltage:read-all');
        return Promise.resolve([]) as never;
      }
      throw new Error(`Unexpected invoke: ${command}`);
    });

    const gamma = readAllGamma(vi.fn());
    const voltage = readAllDac();
    for (let index = 0; index < 28; index += 1) {
      await vi.waitFor(() => expect(gates).toHaveLength(index + 1));
      expect(events).not.toContain('voltage:read-all');
      gates[index].resolve(index);
    }
    await gamma;
    await expect(voltage).resolves.toEqual([]);
    expect(events).toEqual([
      ...Array.from({ length: 28 }, (_, index) => `gamma:${0x0c + index}`),
      'voltage:read-all',
    ]);
  });

  it('does not interleave legacy panel IPC with a whole GAMMA write transaction', async () => {
    const gates: ReturnType<typeof deferred<unknown>>[] = [];
    const events: string[] = [];
    mockedInvoke.mockImplementation((command: string, args?: unknown) => {
      if (command === 'write_dac_register') {
        const address = (args as { addr: number }).addr;
        const gate = deferred<unknown>();
        gates.push(gate);
        events.push(`gamma:${address}`);
        return gate.promise as never;
      }
      if (command === 'read_all_dac') {
        events.push('voltage:read-all');
        return Promise.resolve([]) as never;
      }
      throw new Error(`Unexpected invoke: ${command}`);
    });

    const gamma = writeAllGamma((address) => address & 0xff);
    const voltage = readAllDac();
    for (let index = 0; index < 28; index += 1) {
      await vi.waitFor(() => expect(gates).toHaveLength(index + 1));
      expect(events).not.toContain('voltage:read-all');
      gates[index].resolve(undefined);
    }
    await gamma;
    await expect(voltage).resolves.toEqual([]);
    expect(events).toEqual([
      ...Array.from({ length: 28 }, (_, index) => `gamma:${0x0c + index}`),
      'voltage:read-all',
    ]);
  });
});
