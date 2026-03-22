import { createContext, useContext, useEffect, useMemo, useState } from 'react';
import Toolbar from './components/Toolbar';
import LogPanel from './components/LogPanel';
import VoltageTab from './components/voltage/VoltageTab';
import GammaTab from './components/gamma/GammaTab';
import ConfigTab from './components/config/ConfigTab';
import FaultTab from './components/fault/FaultTab';
import { useDevice } from './hooks/useDevice';
import { useRegisters } from './hooks/useRegisters';
import { useLog } from './hooks/useLog';
import type { DeviceInfo } from './lib/tauri-commands';
import type { LogEntry } from './hooks/useLog';
import type { ChipCapabilities, ChipModel } from './lib/chips';
import { getChipCapabilities } from './lib/chips';

const LS_CHIP_MODEL = 'ek86317a_chipModel';

export interface AppContextType {
  connected: boolean;
  deviceInfo: DeviceInfo | null;
  devices: string[];
  scanning: boolean;
  chipModel: ChipModel;
  chipCapabilities: ChipCapabilities;
  setChipModel: (chipModel: ChipModel) => void;
  scan: () => Promise<string[]>;
  connect: (deviceId: string, clockHz: number, chipModel: ChipModel) => Promise<DeviceInfo>;
  disconnect: () => Promise<void>;
  detect: () => Promise<DeviceInfo>;
  dacRegisters: Map<number, number>;
  eepromRegisters: Map<number, number>;
  registersLoading: boolean;
  readAllDac: () => Promise<void>;
  readAllEeprom: () => Promise<void>;
  readDacRegister: (addr: number) => Promise<number>;
  writeDacRegister: (addr: number, value: number) => Promise<void>;
  setDacValue: (addr: number, value: number) => void;
  replaceDacRegisters: (registers: Iterable<[number, number]>) => void;
  replaceEepromRegisters: (registers: Iterable<[number, number]>) => void;
  resetRegisters: () => void;
  getDacValue: (addr: number) => number | undefined;
  getAvddVoltage: () => number;
  logs: LogEntry[];
  addLog: (level: LogEntry['level'], message: string) => void;
  clearLogs: () => void;
}

export const AppContext = createContext<AppContextType | null>(null);

export function useAppContext(): AppContextType {
  const ctx = useContext(AppContext);
  if (ctx == null) throw new Error('useAppContext must be used within AppProvider');
  return ctx;
}

type TabId = 'voltage' | 'gamma' | 'config' | 'fault';

const TABS: { id: TabId; label: string }[] = [
  { id: 'voltage', label: 'Voltage' },
  { id: 'gamma', label: 'GAMMA' },
  { id: 'config', label: 'Configuration' },
  { id: 'fault', label: 'Fault' },
];

function getInitialChipModel(): ChipModel {
  const stored = localStorage.getItem(LS_CHIP_MODEL);
  return stored === 'iml8947k' || stored === 'lp6281' || stored === 'ek86317a'
    ? stored
    : 'ek86317a';
}

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('voltage');
  const [chipModel, setChipModel] = useState<ChipModel>(getInitialChipModel);

  const device = useDevice();
  const registers = useRegisters(chipModel);
  const log = useLog();

  useEffect(() => {
    localStorage.setItem(LS_CHIP_MODEL, chipModel);
  }, [chipModel]);

  const chipCapabilities = useMemo(() => getChipCapabilities(chipModel), [chipModel]);

  const contextValue: AppContextType = {
    connected: device.connected,
    deviceInfo: device.deviceInfo,
    devices: device.devices,
    scanning: device.scanning,
    chipModel,
    chipCapabilities,
    setChipModel,
    scan: device.scan,
    connect: device.connect,
    disconnect: device.disconnect,
    detect: device.detect,
    dacRegisters: registers.dacRegisters,
    eepromRegisters: registers.eepromRegisters,
    registersLoading: registers.loading,
    readAllDac: registers.readAllDac,
    readAllEeprom: registers.readAllEeprom,
    readDacRegister: registers.readDacRegister,
    writeDacRegister: registers.writeDacRegister,
    setDacValue: registers.setDacValue,
    replaceDacRegisters: registers.replaceDacRegisters,
    replaceEepromRegisters: registers.replaceEepromRegisters,
    resetRegisters: registers.resetRegisters,
    getDacValue: registers.getDacValue,
    getAvddVoltage: registers.getAvddVoltage,
    logs: log.logs,
    addLog: log.addLog,
    clearLogs: log.clearLogs,
  };

  const renderTabContent = () => {
    switch (activeTab) {
      case 'voltage':
        return <VoltageTab />;
      case 'gamma':
        return <GammaTab />;
      case 'config':
        return <ConfigTab />;
      case 'fault':
        return <FaultTab />;
      default:
        return null;
    }
  };

  return (
    <AppContext.Provider value={contextValue}>
      <div className="flex flex-col h-screen bg-gray-900 text-gray-100">
        <Toolbar />
        <div className="flex border-b border-gray-700 bg-gray-800">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-6 py-2 text-sm font-medium transition-colors ${
                activeTab === tab.id
                  ? 'text-blue-400 border-b-2 border-blue-400 bg-gray-900'
                  : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div className="flex-1 overflow-auto p-4">{renderTabContent()}</div>
        <LogPanel />
      </div>
    </AppContext.Provider>
  );
}

export default App;
