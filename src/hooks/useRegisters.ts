import { useCallback, useState } from 'react';
import * as cmd from '../lib/tauri-commands';
import type { ChipModel } from '../lib/chips';
import { getDefaultRegisterValue, REG_AVDD, REG_CONFIG4 } from '../lib/register-map';
import { decodeAvdd } from '../lib/voltage-calc';

export function useRegisters(chipModel: ChipModel) {
  const [dacRegisters, setDacRegisters] = useState<Map<number, number>>(new Map());
  const [eepromRegisters, setEepromRegisters] = useState<Map<number, number>>(new Map());
  const [loading, setLoading] = useState(false);

  const readAllDac = useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const regs = await cmd.readAllDac();
      setDacRegisters(new Map(regs.map((reg) => [reg.address, reg.value] as [number, number])));
    } finally {
      setLoading(false);
    }
  }, []);

  const readAllEeprom = useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const regs = await cmd.readAllEeprom();
      setEepromRegisters(new Map(regs.map((reg) => [reg.address, reg.value] as [number, number])));
    } finally {
      setLoading(false);
    }
  }, []);

  const readDacRegister = useCallback(async (addr: number): Promise<number> => {
    const reg = await cmd.readDacRegister(addr);
    setDacRegisters((prev) => {
      const next = new Map(prev);
      next.set(addr, reg.value);
      return next;
    });
    return reg.value;
  }, []);

  const writeDacRegister = useCallback(async (addr: number, value: number): Promise<void> => {
    await cmd.writeDacRegister(addr, value);
    setDacRegisters((prev) => {
      const next = new Map(prev);
      next.set(addr, value);
      return next;
    });
  }, []);

  const setDacValue = useCallback((addr: number, value: number) => {
    setDacRegisters((prev) => {
      const next = new Map(prev);
      next.set(addr, value);
      return next;
    });
  }, []);

  const replaceDacRegisters = useCallback((registers: Iterable<[number, number]>) => {
    setDacRegisters(new Map(registers));
  }, []);

  const replaceEepromRegisters = useCallback((registers: Iterable<[number, number]>) => {
    setEepromRegisters(new Map(registers));
  }, []);

  const resetRegisters = useCallback(() => {
    setDacRegisters(new Map());
    setEepromRegisters(new Map());
  }, []);

  const getDacValue = useCallback((addr: number): number | undefined => {
    return dacRegisters.get(addr);
  }, [dacRegisters]);

  const getAvddVoltage = useCallback((): number => {
    const avddReg = dacRegisters.get(REG_AVDD) ?? getDefaultRegisterValue(chipModel, REG_AVDD);
    const modeValue = dacRegisters.get(REG_CONFIG4) ?? getDefaultRegisterValue(chipModel, REG_CONFIG4);
    return decodeAvdd(avddReg, chipModel, modeValue);
  }, [chipModel, dacRegisters]);

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
    replaceEepromRegisters,
    resetRegisters,
    getDacValue,
    getAvddVoltage,
  };
}
