import { useCallback, useState } from 'react';
import type { DeviceInfo } from '../lib/tauri-commands';
import type { ChipModel } from '../lib/chips';
import type { UartConfig } from '../lib/uart';
import * as cmd from '../lib/tauri-commands';
import { connectUartDevice, uartClose } from '../lib/uart';

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

  /**
   * 用串口（面板主芯片的 UART 控制台）建立 PMU 会话。
   *
   * 走的是和 FTDI / CH347 完全相同的 ChipDevice 抽象，所以连接成功之后
   * Toolbar 里那些 Read DAC / Write EEPROM / Verify ALL 逻辑一行都不用改。
   */
  const connectUart = useCallback(async (
    config: UartConfig,
    chipModel: ChipModel,
  ): Promise<DeviceInfo> => {
    const info = await connectUartDevice(config, chipModel);
    setDeviceInfo(info);
    setConnected(true);
    return info;
  }, []);

  const disconnect = useCallback(async (): Promise<void> => {
    await cmd.disconnectDevice();
    // 串口是独占资源，断开时顺手关掉，失败无所谓（可能本来就没开）
    try {
      await uartClose();
    } catch {
      // ignore
    }
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

  return { connected, deviceInfo, devices, scanning, scan, connect, connectUart, disconnect, detect };
}
