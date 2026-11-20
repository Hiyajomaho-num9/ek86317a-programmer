import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { OperationQueue } from '../lib/operation-queue';

export interface OperationsContextValue {
  busy: string | null;
  runExclusive: <T>(label: string, operation: () => Promise<T>) => Promise<T>;
}

const OperationsContext = createContext<OperationsContextValue | null>(null);

export function OperationsProvider({ children }: { children: ReactNode }) {
  const queue = useRef(new OperationQueue());
  const mounted = useRef(true);
  const [pending, setPending] = useState(0);
  const [label, setLabel] = useState<string | null>(null);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  const runExclusive = useCallback(async <T,>(name: string, operation: () => Promise<T>) => {
    if (mounted.current) {
      setPending((count) => count + 1);
    }
    try {
      return await queue.current.enqueue(async () => {
        if (mounted.current) setLabel(name);
        try {
          return await operation();
        } finally {
          if (mounted.current) setLabel((active) => active === name ? null : active);
        }
      });
    } finally {
      if (mounted.current) setPending((count) => Math.max(0, count - 1));
    }
  }, []);
  const value = useMemo(() => ({ busy: pending > 0 ? (label ?? 'Hardware operation') : null, runExclusive }), [pending, label, runExclusive]);
  return <OperationsContext.Provider value={value}>{children}</OperationsContext.Provider>;
}

export function useOperations(): OperationsContextValue {
  const value = useContext(OperationsContext);
  if (!value) throw new Error('useOperations must be used within OperationsProvider');
  return value;
}
