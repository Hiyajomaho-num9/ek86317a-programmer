import { createContext, useCallback, useContext, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import type { ReactNode } from 'react';
import * as cmd from '../lib/tauri-commands';
import { createRegisterStore } from '../lib/register-store';
import type { RegisterBank, RegisterSnapshot, RegisterStore } from '../lib/register-store';
import { getDefaultRegisterValue, REG_AVDD, REG_CONFIG4 } from '../lib/register-map';
import { decodeAvdd } from '../lib/voltage-calc';
import { useDeviceContext } from './DeviceContext';

export interface RegistersContextValue {
  store: RegisterStore;
  registersLoading: boolean;
  readAllDac: () => Promise<void>;
  readAllEeprom: () => Promise<void>;
  setDacValue: (address: number, value: number) => void;
  replaceDacRegisters: (values: Iterable<[number, number]>) => void;
  replaceEepromRegisters: (values: Iterable<[number, number]>) => void;
  resetRegisters: () => void;
  getDacValue: (address: number) => number | undefined;
  getAvddVoltage: () => number;
  readDacRegister: (address: number) => Promise<number>;
  writeDacRegister: (address: number, value: number) => Promise<void>;
}
const RegistersContext = createContext<RegistersContextValue | null>(null);

export function RegistersProvider({ children }: { children: ReactNode }) {
  const { chipModel } = useDeviceContext();
  const storeRef = useRef<RegisterStore | null>(null);
  storeRef.current ??= createRegisterStore(chipModel);
  const store = storeRef.current;
  const [registersLoading, setRegistersLoading] = useState(false);
  const readAllDac = useCallback(async () => {
    setRegistersLoading(true);
    try {
      const values = await cmd.readAllDac();
      store.replace('dac', values.map((register) => [register.address, register.value]));
    } finally { setRegistersLoading(false); }
  }, [store]);
  const readAllEeprom = useCallback(async () => {
    setRegistersLoading(true);
    try {
      const values = await cmd.readAllEeprom();
      store.replace('eeprom', values.map((register) => [register.address, register.value]));
    } finally { setRegistersLoading(false); }
  }, [store]);
  const getDacValue = useCallback((address: number) => store.getValue('dac', address), [store]);
  const getAvddVoltage = useCallback(() => decodeAvdd(
    store.getValue('dac', REG_AVDD) ?? getDefaultRegisterValue(chipModel, REG_AVDD),
    chipModel,
    store.getValue('dac', REG_CONFIG4) ?? getDefaultRegisterValue(chipModel, REG_CONFIG4),
  ), [store, chipModel]);
  const value = useMemo<RegistersContextValue>(() => ({
    store, registersLoading, readAllDac, readAllEeprom,
    setDacValue: (address, byte) => store.setValue('dac', address, byte),
    replaceDacRegisters: (values) => store.replace('dac', values),
    replaceEepromRegisters: (values) => store.replace('eeprom', values),
    resetRegisters: () => store.reset(chipModel),
    getDacValue,
    getAvddVoltage,
    readDacRegister: async (address) => {
      const register = await cmd.readDacRegister(address);
      store.setValue('dac', address, register.value);
      return register.value;
    },
    writeDacRegister: async (address, byte) => {
      await cmd.writeDacRegister(address, byte);
      store.setValue('dac', address, byte);
    },
  }), [store, chipModel, registersLoading, readAllDac, readAllEeprom, getDacValue, getAvddVoltage]);
  return <RegistersContext.Provider value={value}>{children}</RegistersContext.Provider>;
}

export function useRegisters(): RegistersContextValue {
  const value = useContext(RegistersContext);
  if (!value) throw new Error('useRegisters must be used within RegistersProvider');
  return value;
}

export function useRegisterValue(address: number, bank: RegisterBank = 'dac'): number | undefined {
  const { store } = useRegisters();
  const subscribe = useCallback((listener: () => void) => store.subscribe(listener), [store]);
  const getSnapshot = useCallback(() => store.getValue(bank, address), [store, bank, address]);
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function useRegisterValues(bank: RegisterBank = 'dac'): RegisterSnapshot {
  const { store } = useRegisters();
  const subscribe = useCallback((listener: () => void) => store.subscribe(listener), [store]);
  const getSnapshot = useCallback(() => store.getSnapshot(bank), [store, bank]);
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
