import type { ChipModel } from './chips';

export type RegisterSnapshot = ReadonlyMap<number, number>;
export type RegisterBank = 'dac' | 'eeprom';

/** Empty means not read. Display components apply the chip-specific fallback. */
export function setRegisterValue(
  registers: RegisterSnapshot,
  address: number,
  value: number,
): RegisterSnapshot {
  if (registers.get(address) === value) return registers;
  const next = new Map(registers);
  next.set(address, value);
  return next;
}

/** One immutable replacement, including removal of addresses absent from the read result. */
export function replaceRegisterValues(
  registers: RegisterSnapshot,
  values: Iterable<readonly [number, number]>,
): RegisterSnapshot {
  const next = new Map(values);
  if (next.size === registers.size && Array.from(next).every(([address, value]) => registers.get(address) === value)) {
    return registers;
  }
  return next;
}

export interface RegisterStore {
  subscribe: (listener: () => void) => () => void;
  getSnapshot: (bank: RegisterBank) => RegisterSnapshot;
  getValue: (bank: RegisterBank, address: number) => number | undefined;
  setValue: (bank: RegisterBank, address: number, value: number) => void;
  replace: (bank: RegisterBank, values: Iterable<readonly [number, number]>) => void;
  clear: (bank: RegisterBank) => void;
  reset: (chipModel: ChipModel) => void;
}

/** Row-level subscriptions share one stable store without a provider value per byte. */
export function createRegisterStore(_chipModel: ChipModel): RegisterStore {
  const listeners = new Set<() => void>();
  const empty: RegisterSnapshot = new Map();
  const snapshots: Record<RegisterBank, RegisterSnapshot> = { dac: empty, eeprom: empty };
  const notify = () => listeners.forEach((listener) => listener());
  const update = (bank: RegisterBank, next: RegisterSnapshot) => {
    if (next === snapshots[bank]) return;
    snapshots[bank] = next;
    notify();
  };
  return {
    subscribe: (listener) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
    getSnapshot: (bank) => snapshots[bank],
    getValue: (bank, address) => snapshots[bank].get(address),
    setValue: (bank, address, value) => update(bank, setRegisterValue(snapshots[bank], address, value)),
    replace: (bank, values) => update(bank, replaceRegisterValues(snapshots[bank], values)),
    clear: (bank) => update(bank, empty),
    reset: (_model) => {
      if (snapshots.dac === empty && snapshots.eeprom === empty) return;
      snapshots.dac = empty;
      snapshots.eeprom = empty;
      notify();
    },
  };
}
