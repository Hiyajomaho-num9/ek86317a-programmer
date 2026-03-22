import { useState, useCallback } from 'react';

export interface LogEntry {
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'success';
  message: string;
}

export function useLog() {
  const [logs, setLogs] = useState<LogEntry[]>([
    {
      timestamp: new Date().toLocaleTimeString('en-US', { hour12: false }),
      level: 'info',
      message: 'EK86317A Programmer ready.',
    },
  ]);

  const addLog = useCallback((level: LogEntry['level'], message: string) => {
    setLogs(prev => [
      ...prev,
      {
        timestamp: new Date().toLocaleTimeString('en-US', { hour12: false }),
        level,
        message,
      },
    ]);
  }, []);

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  return { logs, addLog, clearLogs };
}
