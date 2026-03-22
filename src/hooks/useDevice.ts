import { useState, useCallback } from 'react';
import type { DeviceInfo } from '../lib/tauri-commands';
import * as cmd from '../lib/tauri-commands';

export function useDevice() {
  const [connected, setConnected] = useState(false);
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [devices, setDevices] = useState<string[]>([]);
  const [scanning, setScanning] = useState(false);

  const scan = useCallback(async (): Promise<string[]> => {
    setScanning(true);
    try {
      const result = await cmd.scanDevices();
      setDevices(result);
      return result;
    } finally {
      setScanning(false);
    }
  }, []);

  const connect = useCallback(async (deviceId: string, clockHz: number): Promise<DeviceInfo> => {
    const info = await cmd.connectDevice(deviceId, clockHz);
    setDeviceInfo(info);
    setConnected(true);
    return info;
  }, []);

  const disconnect = useCallback(async (): Promise<void> => {
    await cmd.disconnectDevice();
    setDeviceInfo(null);
    setConnected(false);
  }, []);

  const detect = useCallback(async (): Promise<DeviceInfo> => {
    const info = await cmd.detectIc();
    setDeviceInfo(prev => prev ? { ...prev, pmic_detected: info.pmic_detected, vcom_detected: info.vcom_detected } : prev);
    return info;
  }, []);

  return { connected, deviceInfo, devices, scanning, scan, connect, disconnect, detect };
}
