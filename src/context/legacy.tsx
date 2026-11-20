import { createContext, useContext, useMemo } from 'react';
import type { ReactNode } from 'react';
import type { LogEntry } from '../hooks/useLog';
import type { ChipCapabilities, ChipModel } from '../lib/chips';
import type { DeviceInfo } from '../lib/tauri-commands';
import type { RegisterSnapshot } from '../lib/register-store';
import type { UartConfig } from '../lib/uart';
import { useDeviceContext } from './DeviceContext';
import { useLogContext } from './LogContext';
import { useOperations } from './OperationsContext';
import { useRegisterValues, useRegisters } from './RegistersContext';

/** Kept as a migration bridge; domain hooks avoid subscribing to unrelated state. */
export interface AppContextType {
  chipModel: ChipModel;
  setChipModel: (model: ChipModel) => void;
  chipCapabilities: ChipCapabilities;
  connected: boolean;
  deviceInfo: DeviceInfo | null;
  devices: string[];
  scanning: boolean;
  scan: () => Promise<string[]>;
  connect: (deviceId: string, clockHz: number, chipModel: ChipModel) => Promise<DeviceInfo>;
  connectUart: (config: UartConfig, chipModel: ChipModel) => Promise<DeviceInfo>;
  disconnect: () => Promise<void>;
  /** Sync the UI after a raw-UART configuration change invalidates its PMU backend. */
  invalidatePmuConnection: () => void;
  /** Configuration used by the active UART PMU session; null for USB bridges. */
  uartConfig: UartConfig | null;
  detect: () => Promise<DeviceInfo>;
  dacRegisters: RegisterSnapshot;
  eepromRegisters: RegisterSnapshot;
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
  logs: LogEntry[];
  addLog: (level: LogEntry['level'], message: string) => void;
  clearLogs: () => void;
  busy: string | null;
}
export const AppContext = createContext<AppContextType | null>(null);
export const LegacyFieldsContext = createContext<Omit<AppContextType, 'dacRegisters' | 'eepromRegisters'> | null>(null);

export function LegacyAppProvider({ children }: { children: ReactNode }) {
  const device = useDeviceContext();
  const registers = useRegisters();
  const dacRegisters = useRegisterValues('dac');
  const eepromRegisters = useRegisterValues('eeprom');
  const log = useLogContext();
  const { busy } = useOperations();
  const fields = useMemo(() => ({ ...device, ...log, ...registers, busy }),
    [device, registers, log, busy]);
  const value = useMemo<AppContextType>(() => ({ ...fields, dacRegisters, eepromRegisters }),
    [fields, dacRegisters, eepromRegisters]);
  return <LegacyFieldsContext.Provider value={fields}>
    <AppContext.Provider value={value}>{children}</AppContext.Provider>
  </LegacyFieldsContext.Provider>;
}

export function useAppContext(): AppContextType {
  const value = useContext(AppContext);
  if (!value) throw new Error('useAppContext must be used within AppProviders');
  return value;
}

/** Stable commands/state for panels that do not render the register maps. */
export function useLegacyFields() {
  const value = useContext(LegacyFieldsContext);
  if (!value) throw new Error('useLegacyFields must be used within AppProviders');
  return value;
}
