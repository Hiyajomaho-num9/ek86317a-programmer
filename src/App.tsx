import { useMemo, useState } from 'react';
import { AppProviders } from './context/AppProviders';
import Toolbar from './components/Toolbar';
import VoltageTab from './components/voltage/VoltageTab';
import GammaTab from './components/gamma/GammaTab';
import ConfigTab from './components/config/ConfigTab';
import FaultTab from './components/fault/FaultTab';
import UartTab from './components/uart/UartTab';
import LogPanel from './components/LogPanel';

export type { AppContextType } from './context/legacy';
export { AppContext, useAppContext } from './context/legacy';

type TabId = 'voltage' | 'gamma' | 'config' | 'fault' | 'uart';

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('voltage');
  const tabs = useMemo<{ id: TabId; label: string }[]>(() => [
    { id: 'voltage', label: 'Voltage' },
    { id: 'gamma', label: 'GAMMA' },
    { id: 'config', label: 'Configuration' },
    { id: 'fault', label: 'Fault' },
    { id: 'uart', label: 'UART Bridge' },
  ], []);

  return (
    <AppProviders>
      <div className="flex flex-col h-screen bg-gray-900 text-gray-100">
        <Toolbar />
        <div className="flex border-b border-gray-700 bg-gray-800">
          {tabs.map(tab => (
            <button key={tab.id} onClick={() => setActiveTab(tab.id)}
              className={`px-6 py-2 text-sm font-medium transition-colors ${activeTab === tab.id ? 'text-blue-400 border-b-2 border-blue-400 bg-gray-900' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700'}`}>
              {tab.label}
            </button>
          ))}
        </div>
        <div className="flex-1 overflow-auto p-4">
          {activeTab === 'voltage' && <VoltageTab />}
          {activeTab === 'gamma' && <GammaTab />}
          {activeTab === 'config' && <ConfigTab />}
          {activeTab === 'fault' && <FaultTab />}
          {activeTab === 'uart' && <UartTab />}
        </div>
        <LogPanel />
      </div>
    </AppProviders>
  );
}

export default App;
