import type { ReactNode } from 'react';
import { DeviceProvider } from './DeviceContext';
import { RegistersProvider } from './RegistersContext';
import { LogProvider } from './LogContext';
import { OperationsProvider } from './OperationsContext';
import { LegacyAppProvider } from './legacy';

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <DeviceProvider>
      <OperationsProvider>
        <RegistersProvider>
          <LogProvider>
            <LegacyAppProvider>{children}</LegacyAppProvider>
          </LogProvider>
        </RegistersProvider>
      </OperationsProvider>
    </DeviceProvider>
  );
}
