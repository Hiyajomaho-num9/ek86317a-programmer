import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import App from '../../App';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/plugin-shell', () => ({ Command: { create: vi.fn() } }));
const mockedInvoke = vi.mocked(invoke);

function panel(title: string): HTMLElement {
  const heading = screen.getByText(title);
  const element = heading.parentElement?.parentElement;
  if (!element) throw new Error(`Missing panel: ${title}`);
  return element;
}

describe('Configuration unread fallbacks', () => {
  beforeEach(() => {
    localStorage.clear();
    mockedInvoke.mockReset();
  });

  it('uses chip defaults consistently for bit fields and direct writes, preserving explicit zero', async () => {
    const user = userEvent.setup();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === 'scan_devices') return ['bridge:mock:development'];
      if (command === 'connect_device') return {
        pmic_detected: true, vcom_detected: null,
        device_id: 'bridge:mock:development', chip_model: 'ek86317a',
      };
      if (command === 'write_dac_register') return undefined;
      throw new Error(`Unexpected IPC: ${command}`);
    });
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'Scan' }));
    await user.selectOptions(screen.getByLabelText('Devices:'), 'bridge:mock:development');
    await user.click(screen.getByRole('button', { name: 'Connect' }));
    await screen.findByRole('button', { name: 'Disconnect' });
    await user.click(screen.getByRole('button', { name: 'Configuration' }));

    const config3 = panel('Configuration 3 (31h)');
    const config4 = panel('VCOM2DAC Control (46h)');
    expect(within(config3).getByRole('combobox')).toHaveValue('2');
    expect(within(config4).getByRole('checkbox')).toBeChecked();
    await user.click(within(config3).getByRole('button', { name: 'W' }));
    await user.click(within(config4).getByRole('button', { name: 'W' }));
    expect(mockedInvoke).toHaveBeenCalledWith('write_dac_register', { addr: 0x31, value: 0x02 });
    expect(mockedInvoke).toHaveBeenCalledWith('write_dac_register', { addr: 0x46, value: 0x01 });

    await user.click(within(config4).getByRole('checkbox'));
    expect(within(config4).getByRole('checkbox')).not.toBeChecked();
    expect(within(config4).getByDisplayValue('0x00')).toBeInTheDocument();
    await user.click(within(config4).getByRole('button', { name: 'W' }));
    expect(mockedInvoke).toHaveBeenCalledWith('write_dac_register', { addr: 0x46, value: 0x00 });

    await user.selectOptions(within(config3).getByRole('combobox'), '3');
    expect(within(config3).getByDisplayValue('0x03')).toBeInTheDocument();
    await user.click(within(config3).getByRole('button', { name: 'W' }));
    expect(mockedInvoke).toHaveBeenCalledWith('write_dac_register', { addr: 0x31, value: 0x03 });
  });
});
