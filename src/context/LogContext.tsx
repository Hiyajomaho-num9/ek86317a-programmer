import { createContext, useContext, useMemo } from 'react';
import type { ReactNode } from 'react';
import { useLog } from '../hooks/useLog';

type LogContextValue = ReturnType<typeof useLog>;
const LogContext = createContext<LogContextValue | null>(null);

export function LogProvider({ children }: { children: ReactNode }) {
  const log = useLog();
  const value = useMemo(() => log, [log.logs, log.addLog, log.clearLogs]);
  return <LogContext.Provider value={value}>{children}</LogContext.Provider>;
}

export function useLogContext(): LogContextValue {
  const value = useContext(LogContext);
  if (!value) throw new Error('useLogContext must be used within LogProvider');
  return value;
}
