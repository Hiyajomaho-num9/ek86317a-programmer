import { useEffect, useRef } from 'react';
import { useAppContext } from '../App';

const LEVEL_STYLES: Record<string, { icon: string; color: string }> = {
  info: { icon: 'ℹ', color: 'text-blue-400' },
  warn: { icon: '⚠', color: 'text-yellow-400' },
  error: { icon: '✖', color: 'text-red-400' },
  success: { icon: '✓', color: 'text-green-400' },
};

function LogPanel() {
  const { logs, clearLogs } = useAppContext();
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <div className="h-[120px] min-h-[120px] border-t border-gray-700 bg-gray-950 flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1 bg-gray-800 border-b border-gray-700">
        <span className="text-xs font-medium text-gray-400">Log Output</span>
        <button
          onClick={clearLogs}
          className="text-xs text-gray-500 hover:text-gray-300 px-2 py-0.5 hover:bg-gray-700 rounded"
        >
          Clear
        </button>
      </div>

      {/* Log entries */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-auto p-2 font-mono text-xs space-y-0.5"
      >
        {logs.length === 0 ? (
          <div className="text-gray-600 italic">No log entries.</div>
        ) : (
          logs.map((entry, i) => {
            const style = LEVEL_STYLES[entry.level] ?? LEVEL_STYLES.info;
            return (
              <div key={i} className="flex gap-2">
                <span className="text-gray-600 shrink-0">[{entry.timestamp}]</span>
                <span className={`shrink-0 ${style.color}`}>{style.icon}</span>
                <span className={`${entry.level === 'error' ? 'text-red-300' : 'text-gray-300'}`}>
                  {entry.message}
                </span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

export default LogPanel;
