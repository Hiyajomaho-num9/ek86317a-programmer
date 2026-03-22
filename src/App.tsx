import { createContext, useContext, useState } from 'react';
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

// ============================================================================
// App Context — shared state for all child components
// ============================================================================

export interface AppContextType {
  // Device
  connected: boolean;
  deviceInfo: DeviceInfo | null;
  devices: string[];
  scanning: boolean;
  scan: () => Promise<string[]>;
  connect: (deviceId: string, clockHz: number) => Promise<DeviceInfo>;
  disconnect: () => Promise<void>;
  detect: () => Promise<DeviceInfo>;
  // Registers
  dacRegisters: Map<number, number>;
  eepromRegisters: Map<number, number>;
  registersLoading: boolean;
  readAllDac: () => Promise<void>;
  readAllEeprom: () => Promise<void>;
  readDacRegister: (addr: number) => Promise<number>;
  writeDacRegister: (addr: number, value: number) => Promise<void>;
  setDacValue: (addr: number, value: number) => void;
  replaceDacRegisters: (registers: Iterable<[number, number]>) => void;
  resetRegisters: () => void;
  getDacValue: (addr: number) => number | undefined;
  getAvddVoltage: () => number;
  // Log
  logs: LogEntry[];
  addLog: (level: LogEntry['level'], message: string) => void;
  clearLogs: () => void;
}

export const AppContext = createContext<AppContextType | null>(null);

/** Hook to access AppContext. Must be used within AppProvider. */
export function useAppContext(): AppContextType {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error('useAppContext must be used within AppProvider');
  return ctx;
}

// ============================================================================
// Tab definitions
// ============================================================================

type TabId = 'voltage' | 'gamma' | 'config' | 'fault';

const TABS: { id: TabId; label: string }[] = [
  { id: 'voltage', label: 'Voltage' },
  { id: 'gamma', label: 'GAMMA' },
  { id: 'config', label: 'Configuration' },
  { id: 'fault', label: 'Fault' },
];

// ============================================================================
// App Component
// ============================================================================

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('voltage');

  const device = useDevice();
  const registers = useRegisters();
  const log = useLog();

  const contextValue: AppContextType = {
    // Device
    connected: device.connected,
    deviceInfo: device.deviceInfo,
    devices: device.devices,
    scanning: device.scanning,
    scan: device.scan,
    connect: device.connect,
    disconnect: device.disconnect,
    detect: device.detect,
    // Registers
    dacRegisters: registers.dacRegisters,
    eepromRegisters: registers.eepromRegisters,
    registersLoading: registers.loading,
    readAllDac: registers.readAllDac,
    readAllEeprom: registers.readAllEeprom,
    readDacRegister: registers.readDacRegister,
    writeDacRegister: registers.writeDacRegister,
    setDacValue: registers.setDacValue,
    replaceDacRegisters: registers.replaceDacRegisters,
    resetRegisters: registers.resetRegisters,
    getDacValue: registers.getDacValue,
    getAvddVoltage: registers.getAvddVoltage,
    // Log
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
    }
  };

  return (
    <AppContext.Provider value={contextValue}>
      <div className="flex flex-col h-screen bg-gray-900 text-gray-100">
        {/* Top Toolbar */}
        <Toolbar />

        {/* Tab Bar */}
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

        {/* Tab Content */}
        <div className="flex-1 overflow-auto p-4">{renderTabContent()}</div>

        {/* Bottom Log Panel */}
        <LogPanel />
      </div>
    </AppContext.Provider>
  );
}

export default App;
