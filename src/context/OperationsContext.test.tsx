import { useState } from 'react';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { OperationsProvider, useOperations } from './OperationsContext';

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

function Harness({ first, second }: { first: Promise<void>; second: Promise<void> }) {
  const { busy, runExclusive } = useOperations();
  const [fail, setFail] = useState(false);
  return <>
    <output data-testid="busy">{busy ?? 'idle'}</output>
    <button onClick={() => { void runExclusive('first active', () => first); }}>first</button>
    <button onClick={() => { void runExclusive('second active', () => second); }}>second</button>
    <button onClick={() => {
      void runExclusive('failing active', async () => { throw new Error('failed'); }).catch(() => setFail(true));
    }}>fail</button>
    {fail && <span>recovered</span>}
  </>;
}

describe('OperationsProvider', () => {
  it('labels the active task instead of the most recently queued task', async () => {
    const user = userEvent.setup();
    const first = deferred();
    const second = deferred();
    render(<OperationsProvider><Harness first={first.promise} second={second.promise} /></OperationsProvider>);
    await user.click(screen.getByRole('button', { name: 'first' }));
    await screen.findByText('first active');
    await user.click(screen.getByRole('button', { name: 'second' }));
    expect(screen.getByTestId('busy')).toHaveTextContent('first active');
    await act(async () => { first.resolve(); });
    await screen.findByText('second active');
    await act(async () => { second.resolve(); });
    await screen.findByText('idle');
  });

  it('clears busy state and remains usable after an operation failure', async () => {
    const user = userEvent.setup();
    render(<OperationsProvider><Harness first={Promise.resolve()} second={Promise.resolve()} /></OperationsProvider>);
    await user.click(screen.getByRole('button', { name: 'fail' }));
    await screen.findByText('recovered');
    expect(screen.getByTestId('busy')).toHaveTextContent('idle');
    await user.click(screen.getByRole('button', { name: 'first' }));
    await screen.findByText('idle');
  });
});
