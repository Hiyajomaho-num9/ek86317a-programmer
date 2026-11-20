import { useCallback, useState } from 'react';
import type { DeviceInfo } from '../lib/tauri-commands';
import type { ChipModel } from '../lib/chips';
import type { UartConfig } from '../lib/uart';
import * as cmd from '../lib/tauri-commands';
import { connectUartDevice, uartClose } from '../lib/uart';

export function useDevice(chipModel: ChipModel) {
  const [connected, setConnected] = useState(false);
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [devices, setDevices] = useState<string[]>([]);
  const [scanning, setScanning] = useState(false);
  const [uartConfig, setUartConfig] = useState<UartConfig | null>(null);
  const scan = useCallback(async () => {
    setScanning(true);
    try {
      const result = await cmd.scanDevices();
      setDevices(result);
      return result;
    } finally { setScanning(false); }
  }, []);
  const connect = useCallback(async (deviceId: string, clockHz: number, model: ChipModel) => {
    const info = await cmd.connectDevice(deviceId, clockHz, model);
    setUartConfig(null);
    setDeviceInfo(info);
    setConnected(true);
    return info;
  }, []);
  const connectUart = useCallback(async (config: UartConfig, model: ChipModel) => {
    const info = await connectUartDevice(config, model);
    setUartConfig({ ...config });
    setDeviceInfo(info);
    setConnected(true);
    return info;
  }, []);
  const invalidatePmuConnection = useCallback(() => {
    setUartConfig(null);
    setDeviceInfo(null);
    setConnected(false);
  }, []);
  const disconnect = useCallback(async () => {
    let failure: unknown;
    try { await cmd.disconnectDevice(); } catch (error) { failure = error; }
    try { await uartClose(); } catch (error) { failure ??= error; }
    setDeviceInfo(null);
    setUartConfig(null);
    setConnected(false);
    if (failure !== undefined) throw failure;
  }, []);
  const detect = useCallback(async (): Promise<DeviceInfo> => {
    const info = await cmd.detectIc();
    setDeviceInfo((previous) => ({ ...info, device_id: previous?.device_id ?? info.device_id }));
    return info;
  }, []);
  void chipModel;
  return { connected, deviceInfo, devices, scanning, scan, connect, connectUart, disconnect, detect, invalidatePmuConnection, uartConfig };
}
