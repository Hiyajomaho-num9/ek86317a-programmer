import { GAMMA_CHANNELS, gammaRegAddresses, getDefaultRegisterValue } from './register-map';
import type { ChipModel } from './chips';
import { readDacRegisterUnqueued, writeDacRegisterUnqueued } from './tauri-commands';
import { hardwareQueue } from './operation-queue';

/** A whole GAMMA batch owns the hardware FIFO; its inner requests must not re-enter it. */
export interface GammaTransport {
  read: (address: number) => Promise<number>;
  write: (address: number, value: number) => Promise<void>;
}

const transport: GammaTransport = { read: readDacRegisterUnqueued, write: writeDacRegisterUnqueued };

export async function readAllGamma(
  onRegisters: (values: [number, number][]) => void,
  io: GammaTransport = transport,
): Promise<void> {
  return hardwareQueue.enqueue(async () => {
    const values: [number, number][] = [];
    for (let channel = 1; channel <= GAMMA_CHANNELS; channel += 1) {
      const { high, low } = gammaRegAddresses(channel);
      values.push([high, await io.read(high)]);
      values.push([low, await io.read(low)]);
    }
    onRegisters(values);
  });
}

export async function writeAllGamma(
  getValue: (address: number) => number | undefined,
  io: GammaTransport = transport,
  chipModel: ChipModel = 'ek86317a',
): Promise<void> {
  return hardwareQueue.enqueue(async () => {
    for (let channel = 1; channel <= GAMMA_CHANNELS; channel += 1) {
      const { high, low } = gammaRegAddresses(channel);
      await io.write(high, getValue(high) ?? getDefaultRegisterValue(chipModel, high));
      await io.write(low, getValue(low) ?? getDefaultRegisterValue(chipModel, low));
    }
  });
}
