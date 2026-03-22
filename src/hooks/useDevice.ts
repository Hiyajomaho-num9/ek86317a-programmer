import { useCallback, useState } from 'react';
import type { DeviceInfo } from '../lib/tauri-commands';
import type { ChipModel } from '../lib/chips';
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

  const connect = useCallback(async (
    deviceId: string,
    clockHz: number,
    chipModel: ChipModel,
  ): Promise<DeviceInfo> => {
    const info = await cmd.connectDevice(deviceId, clockHz, chipModel);
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
    setDeviceInfo((prev) => (prev
      ? {
          ...prev,
          pmic_detected: info.pmic_detected,
          vcom_detected: info.vcom_detected,
          chip_model: info.chip_model,
        }
      : info));
    return info;
  }, []);

  return { connected, deviceInfo, devices, scanning, scan, connect, disconnect, detect };
}
