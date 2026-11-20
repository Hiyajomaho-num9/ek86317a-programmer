import { createContext, useContext, useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { useDevice } from '../hooks/useDevice';
import { getChipCapabilities } from '../lib/chips';
import type { ChipModel } from '../lib/chips';

type DeviceOperations = ReturnType<typeof useDevice>;
export type DeviceContextValue = DeviceOperations & {
  chipModel: ChipModel;
  setChipModel: (model: ChipModel) => void;
  chipCapabilities: ReturnType<typeof getChipCapabilities>;
};
const DeviceContext = createContext<DeviceContextValue | null>(null);
const LS_CHIP_MODEL = 'ek86317a_chipModel';
function initialChipModel(): ChipModel {
  const stored = localStorage.getItem(LS_CHIP_MODEL);
  return stored === 'iml8947k' || stored === 'lp6281' ? stored : 'ek86317a';
}

export function DeviceProvider({ children }: { children: ReactNode }) {
  const [chipModel, setChipModel] = useState<ChipModel>(initialChipModel);
  useEffect(() => { localStorage.setItem(LS_CHIP_MODEL, chipModel); }, [chipModel]);
  const device = useDevice(chipModel);
  useEffect(() => () => { void device.disconnect().catch(() => undefined); }, [device.disconnect]);
  const { connected, deviceInfo, devices, scanning, scan, connect, connectUart, disconnect, detect, invalidatePmuConnection, uartConfig } = device;
  const value = useMemo<DeviceContextValue>(() => ({
    connected, deviceInfo, devices, scanning, scan, connect, connectUart, disconnect, detect, invalidatePmuConnection, uartConfig,
    chipModel, setChipModel, chipCapabilities: getChipCapabilities(chipModel),
  }), [connected, deviceInfo, devices, scanning, scan, connect, connectUart, disconnect, detect, invalidatePmuConnection, uartConfig, chipModel]);
  return <DeviceContext.Provider value={value}>{children}</DeviceContext.Provider>;
}

export function useDeviceContext(): DeviceContextValue {
  const value = useContext(DeviceContext);
  if (!value) throw new Error('useDeviceContext must be used within DeviceProvider');
  return value;
}
