import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import App from './App';
import { getDefaultRegisterValue } from './lib/register-map';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/plugin-shell', () => ({ Command: { create: vi.fn() } }));
const mockedInvoke = vi.mocked(invoke);

function row(label: string): HTMLElement {
  const element = screen.getByText(label, { selector: 'span' });
  if (!element.parentElement) throw new Error(`Missing ${label} row`);
  return element.parentElement;
}

describe('compatible display fallbacks and tab lifecycle', () => {
  beforeEach(() => {
    window.localStorage.clear();
    mockedInvoke.mockReset();
  });

  it('uses chip defaults without populating the unread register store', async () => {
    const user = userEvent.setup();
    render(<App />);
    const avdd = row('AVDD');
    expect(within(avdd).getByRole('combobox')).toHaveValue('41');
    expect(within(avdd).getByText('HEX: 0x29')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'GAMMA' }));
    expect(within(row('GMA1')).getByDisplayValue('0x0000')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Configuration' }));
    expect(screen.getByDisplayValue('0x02')).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText('Chip:'), 'iml8947k');
    await user.click(screen.getByRole('button', { name: 'Voltage' }));
    expect(within(row('AVDD')).getByRole('combobox')).toHaveValue(
      String(getDefaultRegisterValue('iml8947k', 0) & 0x3f),
    );
    await user.click(screen.getByRole('button', { name: 'GAMMA' }));
    expect(within(row('GMA1')).getByDisplayValue('0x0000')).toBeInTheDocument();
  });

  it('invalidates an old UART PMU session when its configuration changes', async () => {
    const user = userEvent.setup();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === 'uart_list_ports') return [{ path: 'COM7', description: 'UART test fixture' }];
      if (command === 'connect_uart_device') return {
        pmic_detected: true, vcom_detected: null,
        device_id: 'bridge:uart:COM7', chip_model: 'ek86317a',
      };
      if (command === 'uart_open') return { path: 'COM7', root: true, id_line: 'uid=0', warning: null };
      if (command === 'uart_term_stop' || command === 'disconnect_device' || command === 'uart_close') return undefined;
      throw new Error(`Unexpected IPC: ${command}`);
    });
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'UART Bridge' }));
    await user.click(screen.getByRole('button', { name: 'Refresh' }));
    await user.selectOptions(screen.getByLabelText('Port:'), 'COM7');
    await user.click(screen.getByRole('button', { name: 'Connect as PMU (EK86317A)' }));
    expect(await screen.findByRole('button', { name: 'Disconnect' })).toBeEnabled();
    expect(screen.getByLabelText('Chip:')).toBeDisabled();

    await user.selectOptions(screen.getByLabelText('Baud:'), '230400');
    expect(await screen.findByRole('button', { name: 'Connect' })).toBeInTheDocument();
    expect(screen.getByLabelText('Chip:')).toBeEnabled();
    expect(screen.getByText(/旧 PMU 会话已失效/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Close' }));
    await user.click(screen.getByRole('button', { name: 'Open / Test' }));
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith('uart_open', {
      config: expect.objectContaining({ port: 'COM7', baud: 230400 }),
    }));
    await user.click(screen.getByRole('button', { name: 'Connect as PMU (EK86317A)' }));
    expect(await screen.findByRole('button', { name: 'Disconnect' })).toBeEnabled();
  });

  it('does not invalidate a USB PMU session from unrelated UART settings', async () => {
    const user = userEvent.setup();
    window.localStorage.setItem('ek86317a_uartConfig_v2', JSON.stringify({ port: 'COM7' }));
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === 'scan_devices') return ['bridge:mock:development'];
      if (command === 'connect_device') return {
        pmic_detected: true, vcom_detected: null,
        device_id: 'bridge:ftdi:0:test', chip_model: 'ek86317a',
      };
      if (command === 'uart_term_stop' || command === 'disconnect_device' || command === 'uart_close') return undefined;
      throw new Error(`Unexpected IPC: ${command}`);
    });
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'Scan' }));
    await user.selectOptions(screen.getByLabelText('Devices:'), 'bridge:mock:development');
    await user.click(screen.getByRole('button', { name: 'Connect' }));
    expect(await screen.findByRole('button', { name: 'Disconnect' })).toBeEnabled();
    await user.click(screen.getByRole('button', { name: 'UART Bridge' }));
    expect(screen.getByRole('button', { name: 'Disconnect' })).toBeEnabled();
    await user.selectOptions(screen.getByLabelText('Baud:'), '230400');
    expect(screen.getByRole('button', { name: 'Disconnect' })).toBeEnabled();
  });
});
