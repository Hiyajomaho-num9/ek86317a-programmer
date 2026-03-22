import { useState, useCallback } from 'react';
import * as cmd from '../lib/tauri-commands';
import { decodeAvdd } from '../lib/voltage-calc';
import { DEFAULT_AVDD_REG_VALUE, REG_AVDD } from '../lib/register-map';

export function useRegisters() {
  const [dacRegisters, setDacRegisters] = useState<Map<number, number>>(new Map());
  const [eepromRegisters, setEepromRegisters] = useState<Map<number, number>>(new Map());
  const [loading, setLoading] = useState(false);

  /** Read all DAC registers from device. */
  const readAllDac = useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const regs = await cmd.readAllDac();
      const map = new Map<number, number>();
      for (const r of regs) {
        map.set(r.address, r.value);
      }
      setDacRegisters(map);
    } finally {
      setLoading(false);
    }
  }, []);

  /** Read all EEPROM registers from device. */
  const readAllEeprom = useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const regs = await cmd.readAllEeprom();
      const map = new Map<number, number>();
      for (const r of regs) {
        map.set(r.address, r.value);
      }
      setEepromRegisters(map);
    } finally {
      setLoading(false);
    }
  }, []);

  /** Read a single DAC register and update local state. */
  const readDacRegister = useCallback(async (addr: number): Promise<number> => {
    const reg = await cmd.readDacRegister(addr);
    setDacRegisters(prev => {
      const next = new Map(prev);
      next.set(addr, reg.value);
      return next;
    });
    return reg.value;
  }, []);

  /** Write a single DAC register and update local state. */
  const writeDacRegister = useCallback(async (addr: number, value: number): Promise<void> => {
    await cmd.writeDacRegister(addr, value);
    setDacRegisters(prev => {
      const next = new Map(prev);
      next.set(addr, value);
      return next;
    });
  }, []);

  /** Update local DAC register state without I2C write (e.g., after loading firmware preview). */
  const setDacValue = useCallback((addr: number, value: number) => {
    setDacRegisters(prev => {
      const next = new Map(prev);
      next.set(addr, value);
      return next;
    });
  }, []);

  /** Replace the local DAC register cache with a known register set. */
  const replaceDacRegisters = useCallback((registers: Iterable<[number, number]>) => {
    setDacRegisters(new Map(registers));
  }, []);

  /** Clear all cached register data when the active device/session changes. */
  const resetRegisters = useCallback(() => {
    setDacRegisters(new Map());
    setEepromRegisters(new Map());
  }, []);

  /** Get a DAC register value from local state. */
  const getDacValue = useCallback((addr: number): number | undefined => {
    return dacRegisters.get(addr);
  }, [dacRegisters]);

  /** Get current AVDD voltage from local DAC state. Falls back to the datasheet default register value. */
  const getAvddVoltage = useCallback((): number => {
    const avddReg = dacRegisters.get(REG_AVDD);
    return decodeAvdd(avddReg ?? DEFAULT_AVDD_REG_VALUE);
  }, [dacRegisters]);

  return {
    dacRegisters,
    eepromRegisters,
    loading,
    readAllDac,
    readAllEeprom,
    readDacRegister,
    writeDacRegister,
    setDacValue,
    replaceDacRegisters,
    resetRegisters,
    getDacValue,
    getAvddVoltage,
  };
}
