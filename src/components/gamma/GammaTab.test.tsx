import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import App from '../../App';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/plugin-shell', () => ({ Command: { create: vi.fn() } }));

const mockedInvoke = vi.mocked(invoke);
const DEVICE_ID = 'bridge:mock:development';

function gammaRow(channel: number): HTMLElement {
  const label = screen.getByText(`GMA${channel}`, { selector: 'span' });
  const row = label.parentElement;
  if (!row) throw new Error(`Missing row for GMA${channel}`);
  return row;
}

describe('GammaTab read-all integration', () => {
  beforeEach(() => {
    window.localStorage.clear();
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      const params = args as { addr?: number } | undefined;
      if (command === 'scan_devices') return [DEVICE_ID];
      if (command === 'connect_device') {
        return {
          device_id: DEVICE_ID,
          chip_model: 'ek86317a',
          pmic_detected: true,
          vcom_detected: true,
        };
      }
      if (command === 'read_dac_register') {
        const address = params?.addr;
        if (address === undefined) throw new Error('read_dac_register called without addr');
        return { address, name: `0x${address.toString(16)}`, value: address === 0x0c ? 2 : address & 0xff, voltage: null };
      }
      throw new Error(`Unexpected invoke: ${command}`);
    });
  });

  it('connects from the toolbar and reads 0x0c through 0x27 into visible Gamma rows', async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole('button', { name: 'Scan' }));
    const deviceSelect = screen.getByDisplayValue('Select Device');
    await user.selectOptions(deviceSelect, DEVICE_ID);
    await user.click(screen.getByRole('button', { name: 'Connect' }));
    await screen.findByRole('button', { name: 'Disconnect' });

    expect(mockedInvoke).toHaveBeenCalledWith('connect_device', {
      deviceId: DEVICE_ID,
      clockHz: 400_000,
      chipModel: 'ek86317a',
    });
    await user.click(screen.getByRole('button', { name: 'GAMMA' }));

    const firstRow = gammaRow(1);
    expect(within(firstRow).getByDisplayValue('0x0000')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Read All' }));

    await screen.findByText('Read all GAMMA channels complete');
    const gammaCalls = mockedInvoke.mock.calls.filter(([command]) => command === 'read_dac_register');
    expect(gammaCalls).toHaveLength(28);
    expect(gammaCalls.map(([, args]) => (args as { addr: number }).addr)).toEqual(
      Array.from({ length: 28 }, (_, index) => 0x0c + index),
    );
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, 'read_dac_register', { addr: 0x0c });
    expect(mockedInvoke).toHaveBeenLastCalledWith('read_dac_register', { addr: 0x27 });

    expect(await within(firstRow).findByDisplayValue('0x020D')).toBeInTheDocument();
    const expectedVoltage = (17.6 * 0x20d / 1024).toFixed(3);
    expect(within(firstRow).getByDisplayValue(expectedVoltage)).toBeInTheDocument();

    const lastRow = gammaRow(14);
    expect(await within(lastRow).findByDisplayValue('0x0227')).toBeInTheDocument();
    expect(within(lastRow).getByDisplayValue((17.6 * 0x227 / 1024).toFixed(3))).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Read All' })).toBeEnabled());
    expect(window.localStorage.getItem('ek86317a_exportPath')).toBe('');
    expect(window.localStorage.getItem('ek86317a_panelName')).toBe('');
  });
});
