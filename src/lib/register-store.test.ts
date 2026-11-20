import { describe, expect, it, vi } from 'vitest';
import {
  createRegisterStore,
  replaceRegisterValues,
  setRegisterValue,
} from './register-store';

describe('register snapshot updates', () => {
  it('keeps the same reference when setting an identical value', () => {
    const original = new Map([[0x0c, 2]]);

    expect(setRegisterValue(original, 0x0c, 2)).toBe(original);
    expect(setRegisterValue(original, 0x0d, 0)).not.toBe(original);
  });

  it('copies the snapshot on changes without mutating the previous snapshot', () => {
    const original = new Map([[0x0c, 2], [0x0d, 0xff]]);

    const next = setRegisterValue(original, 0x0d, 0x34);

    expect(next).not.toBe(original);
    expect(original).toEqual(new Map([[0x0c, 2], [0x0d, 0xff]]));
    expect(next).toEqual(new Map([[0x0c, 2], [0x0d, 0x34]]));
    expect(next.get(0x0c)).toBe(2);
  });

  it('replaces a batch immutably and preserves identity for identical contents', () => {
    const original = new Map([[0x0c, 1], [0x0d, 2], [0x0e, 3]]);

    const unchanged = replaceRegisterValues(original, [[0x0e, 3], [0x0c, 1], [0x0d, 2]]);
    const changed = replaceRegisterValues(original, [
      [0x0c, 1],
      [0x0d, 0xfe],
      [0x0e, 0],
      [0x0f, 0xff],
    ]);

    expect(unchanged).toBe(original);
    expect(changed).not.toBe(original);
    expect(changed).toEqual(new Map([
      [0x0c, 1], [0x0d, 0xfe], [0x0e, 0], [0x0f, 0xff],
    ]));
    expect(original).toEqual(new Map([[0x0c, 1], [0x0d, 2], [0x0e, 3]]));
  });

  it('removes addresses omitted by a hardware snapshot replacement', () => {
    const original = new Map([[0x0c, 1], [0x0d, 2], [0x0e, 3]]);

    const next = replaceRegisterValues(original, [[0x0c, 1], [0x0d, 2]]);

    expect(next).toEqual(new Map([[0x0c, 1], [0x0d, 2]]));
    expect(original).toEqual(new Map([[0x0c, 1], [0x0d, 2], [0x0e, 3]]));
  });
});

describe('createRegisterStore', () => {
  it('keeps a stable snapshot between updates and isolates DAC from EEPROM', () => {
    const store = createRegisterStore('ek86317a');
    const dac = store.getSnapshot('dac');
    const eeprom = store.getSnapshot('eeprom');

    expect(store.getSnapshot('dac')).toBe(dac);
    expect(store.getSnapshot('eeprom')).toBe(eeprom);
    // The shared immutable empty snapshot means "not read", never a default value.
    expect(dac.size).toBe(0);
    expect(eeprom.size).toBe(0);
    store.setValue('dac', 0x0c, 0x02);
    expect(store.getSnapshot('dac')).not.toBe(dac);
    expect(store.getSnapshot('eeprom')).toBe(eeprom);
    expect(store.getValue('eeprom', 0x0c)).toBeUndefined();
  });

  it('notifies once for each changed write and not for no-op writes', () => {
    const store = createRegisterStore('ek86317a');
    store.replace('dac', [[0x00, 0x29], [0x01, 0x1e], [0x02, 0x40]]);
    const listener = vi.fn();
    const unsubscribe = store.subscribe(listener);
    const original = store.getSnapshot('dac');

    store.setValue('dac', 0x00, 0x29);
    expect(listener).not.toHaveBeenCalled();
    expect(store.getSnapshot('dac')).toBe(original);

    store.setValue('dac', 0x00, 0x2a);
    const afterFirst = store.getSnapshot('dac');
    expect(listener).toHaveBeenCalledTimes(1);
    expect(afterFirst).not.toBe(original);
    expect(original.get(0x00)).toBe(0x29);
    expect(afterFirst.get(0x00)).toBe(0x2a);

    store.replace('dac', [[0x00, 0x2a], [0x01, 0x1e], [0x02, 0x40]]);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(store.getSnapshot('dac')).toBe(afterFirst);

    store.replace('dac', [[0x00, 0x2a], [0x01, 0x1f], [0x02, 0x41], [0x03, 0x0b]]);
    expect(listener).toHaveBeenCalledTimes(2);
    expect(store.getValue('dac', 0x01)).toBe(0x1f);
    expect(store.getValue('dac', 0x02)).toBe(0x41);
    expect(store.getValue('dac', 0x03)).toBe(0x0b);

    unsubscribe();
    store.setValue('dac', 0x04, 0);
    expect(listener).toHaveBeenCalledTimes(2);
  });

  it('isolates bank updates and batches clear/reset notifications', () => {
    const store = createRegisterStore('lp6281');
    store.setValue('dac', 0x00, 0x29);
    const listener = vi.fn();
    store.subscribe(listener);
    const dac = store.getSnapshot('dac');

    store.setValue('eeprom', 0x0c, 3);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(store.getSnapshot('dac')).toBe(dac);
    expect(store.getSnapshot('eeprom')).toEqual(new Map([[0x0c, 3]]));

    const previousDac = store.getSnapshot('dac');
    const previousEeprom = store.getSnapshot('eeprom');
    store.clear('dac');
    expect(listener).toHaveBeenCalledTimes(2);
    expect(store.getSnapshot('dac')).not.toBe(previousDac);
    expect(store.getSnapshot('dac').size).toBe(0);
    expect(store.getSnapshot('eeprom')).toBe(previousEeprom);

    store.reset('ek86317a');
    expect(listener).toHaveBeenCalledTimes(3);
    expect(store.getValue('dac', 0x00)).toBeUndefined();
    expect(store.getValue('dac', 0x45)).toBeUndefined();
    expect(store.getSnapshot('eeprom').size).toBe(0);
  });

  it('notifies every current subscriber once per multi-value replace', () => {
    const store = createRegisterStore('ek86317a');
    const listeners = [vi.fn(), vi.fn(), vi.fn()];
    listeners.forEach((listener) => store.subscribe(listener));

    store.replace('dac', Array.from({ length: 28 }, (_, index) => [0x0c + index, index & 0xff] as const));

    listeners.forEach((listener) => expect(listener).toHaveBeenCalledTimes(1));
  });
});
