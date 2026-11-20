import { createRef } from 'react';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LogProvider } from '../../context/LogContext';
import { OperationsProvider } from '../../context/OperationsContext';
import UartTerminal from './UartTerminal';
import type { UartTerminalHandle } from './UartTerminal';
import { DEFAULT_UART_CONFIG } from '../../lib/uart';

const uart = vi.hoisted(() => ({
  start: vi.fn(), stop: vi.fn(), poll: vi.fn(), send: vi.fn(),
}));
vi.mock('../../lib/uart', async (importOriginal) => ({
  ...await importOriginal<typeof import('../../lib/uart')>(),
  uartTermStart: uart.start,
  uartTermStop: uart.stop,
  uartTermPoll: uart.poll,
  uartTermSend: uart.send,
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

function renderTerminal() {
  const ref = createRef<UartTerminalHandle>();
  const config = { ...DEFAULT_UART_CONFIG, port: 'COM7' };
  const result = render(<LogProvider><OperationsProvider>
    <UartTerminal ref={ref} config={config} />
  </OperationsProvider></LogProvider>);
  return { ...result, ref };
}

async function startTerminal(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: '启动终端' }));
  await screen.findByRole('button', { name: '停止终端' });
  await waitFor(() => expect(uart.poll).toHaveBeenCalled());
}

describe('UartTerminal lifecycle', () => {
  beforeEach(() => {
    uart.start.mockReset().mockResolvedValue(undefined);
    uart.stop.mockReset().mockResolvedValue(undefined);
    uart.poll.mockReset().mockResolvedValue('');
    uart.send.mockReset().mockResolvedValue(undefined);
  });

  it('can stop while a poll is pending and start again without a deadlock', async () => {
    const user = userEvent.setup();
    const pending = deferred<string>();
    uart.poll.mockReturnValueOnce(pending.promise);
    renderTerminal();
    await startTerminal(user);
    await user.click(screen.getByRole('button', { name: '停止终端' }));
    await waitFor(() => expect(uart.stop).toHaveBeenCalled());
    await act(async () => { pending.resolve(''); });
    await startTerminal(user);
    expect(uart.start).toHaveBeenCalledTimes(2);
  });

  it('releases the interactive lease after a poll error', async () => {
    const user = userEvent.setup();
    uart.poll.mockRejectedValueOnce(new Error('serial disconnected'));
    renderTerminal();
    await user.click(screen.getByRole('button', { name: '启动终端' }));
    await waitFor(() => expect(uart.poll).toHaveBeenCalled());
    await screen.findByRole('button', { name: '启动终端' });
    await waitFor(() => expect(uart.stop).toHaveBeenCalled());
  });

  it('stops polling and releases the backend when unmounted', async () => {
    const user = userEvent.setup();
    const pending = deferred<string>();
    uart.poll.mockReturnValueOnce(pending.promise);
    const { unmount } = renderTerminal();
    await startTerminal(user);
    unmount();
    await waitFor(() => expect(uart.stop).toHaveBeenCalled());
    await act(async () => { pending.resolve(''); });
    expect(uart.poll).toHaveBeenCalledTimes(1);
  });

  it('coalesces repeated explicit Stop calls', async () => {
    const { ref } = renderTerminal();
    await act(async () => {
      await Promise.all([ref.current!.stop(), ref.current!.stop(), ref.current!.stop()]);
    });
    expect(uart.stop).toHaveBeenCalledTimes(1);
  });
});
