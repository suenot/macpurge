'use client';

import { useState } from 'react';
import { Monitor, Smartphone, Box, HardDrive, RefreshCw, Trash2, Loader2 } from 'lucide-react';
import { clsx } from 'clsx';
import { motion } from 'framer-motion';
import { scanDevtools, cleanDevtoolCache, isTauri } from '@/lib/tauri';
import type { DevToolScanResult, DevToolInfo, DevToolCachePath } from '@/lib/types';
import { formatBytes } from '@/lib/types';
import ConfirmDialog from '@/components/ui/ConfirmDialog';
import { useConfirmBeforeDelete } from '@/hooks/useConfirmBeforeDelete';

const toolIcons: Record<string, typeof Monitor> = {
  xcode: Monitor,
  android_studio: Smartphone,
  docker: Box,
  virtual_machines: HardDrive,
};

export default function DevToolsPage() {
  const [scanning, setScanning] = useState(false);
  const [result, setResult] = useState<DevToolScanResult | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [cleaning, setCleaning] = useState(false);
  const [cleanedPaths, setCleanedPaths] = useState<Set<string>>(new Set());
  const [confirmClean, setConfirmClean] = useState(false);
  const confirmBeforeDelete = useConfirmBeforeDelete();

  const handleScan = async () => {
    if (!isTauri()) return;
    setScanning(true);
    setResult(null);
    setSelected(new Set());
    setCleanedPaths(new Set());
    try {
      const data = await scanDevtools();
      setResult(data);
    } catch (err) {
      console.error('Scan failed:', err);
    } finally {
      setScanning(false);
    }
  };

  const togglePath = (path: string) => {
    setSelected(prev => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const handleClean = () => {
    if (!isTauri() || selected.size === 0) return;
    if (confirmBeforeDelete) {
      setConfirmClean(true);
      return;
    }
    runClean();
  };

  const runClean = async () => {
    if (!isTauri() || selected.size === 0) return;
    setConfirmClean(false);
    setCleaning(true);
    for (const path of selected) {
      try {
        await cleanDevtoolCache(path, false);
        setCleanedPaths(prev => new Set([...prev, path]));
      } catch (err) {
        console.error('Clean failed for', path, err);
      }
    }
    setCleaning(false);
    // Re-scan after cleaning
    handleScan();
  };

  const totalSelected = result
    ? result.tools
        .flatMap(t => t.cache_paths)
        .filter(p => selected.has(p.path))
        .reduce((sum, p) => sum + p.size_bytes, 0)
    : 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">Dev Tools</h1>
          <p className="text-slate-400 mt-1">Deep cleaning for heavy development environments</p>
        </div>
        <button
          onClick={handleScan}
          disabled={scanning}
          className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 text-white font-medium transition-all disabled:opacity-50"
        >
          {scanning ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCw className="w-4 h-4" />}
          {scanning ? 'Scanning...' : 'Scan All'}
        </button>
      </div>

      {/* Results */}
      {result && (
        <>
          {/* Summary */}
          <div className="glass-card p-4 rounded-2xl flex items-center justify-between">
            <span className="text-slate-400">
              Found <span className="text-white font-semibold">{formatBytes(result.total_size)}</span> across {result.tools.length} tools
              <span className="text-slate-600 ml-2">({result.scan_duration_ms}ms)</span>
            </span>
            {selected.size > 0 && (
              <button
                onClick={handleClean}
                disabled={cleaning}
                className="flex items-center gap-2 px-4 py-2 rounded-xl bg-danger-600 hover:bg-danger-500 text-white text-sm font-medium transition-all disabled:opacity-50"
              >
                {cleaning ? <Loader2 className="w-4 h-4 animate-spin" /> : <Trash2 className="w-4 h-4" />}
                Clean Selected ({formatBytes(totalSelected)})
              </button>
            )}
          </div>

          {/* Tool Cards */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {result.tools.map((tool, index) => {
              const Icon = toolIcons[tool.tool] || HardDrive;
              return (
                <motion.div
                  key={tool.tool}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: index * 0.05 }}
                  className="glass-card p-6 rounded-2xl"
                >
                  <div className="flex items-center gap-3 mb-4">
                    <div className={clsx(
                      'p-3 rounded-xl border',
                      tool.is_installed
                        ? 'bg-brand-500/10 border-brand-500/20'
                        : 'bg-slate-800 border-white/5'
                    )}>
                      <Icon className={clsx('w-5 h-5', tool.is_installed ? 'text-brand-400' : 'text-slate-500')} />
                    </div>
                    <div className="flex-1">
                      <h3 className="font-semibold text-white">{toolName(tool.tool)}</h3>
                      <span className={clsx('text-xs', tool.is_installed ? 'text-emerald-400' : 'text-slate-500')}>
                        {tool.is_installed ? 'Installed' : 'Not found'}
                      </span>
                    </div>
                    <span className="text-lg font-bold text-white">{formatBytes(tool.total_size)}</span>
                  </div>

                  {tool.cache_paths.length === 0 ? (
                    <p className="text-sm text-slate-500">No caches found</p>
                  ) : (
                    <div className="space-y-2">
                      {tool.cache_paths.map((cache) => (
                        <label
                          key={cache.path}
                          className={clsx(
                            'flex items-center gap-3 p-3 rounded-xl cursor-pointer transition-all',
                            selected.has(cache.path)
                              ? 'bg-brand-500/10 border border-brand-500/30'
                              : 'bg-white/5 border border-transparent hover:bg-white/10',
                            cleanedPaths.has(cache.path) && 'opacity-40'
                          )}
                        >
                          <input
                            type="checkbox"
                            checked={selected.has(cache.path)}
                            onChange={() => togglePath(cache.path)}
                            disabled={cache.path.startsWith('docker://')}
                            className="rounded border-white/20 bg-white/10 text-brand-500 focus:ring-brand-500/50"
                          />
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
                              <span className="text-sm text-white truncate">{cache.description}</span>
                              {cache.safe_to_clean && (
                                <span className="text-[9px] px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-400 shrink-0">safe</span>
                              )}
                            </div>
                            <p className="text-[11px] text-slate-500 truncate mt-0.5">{cache.path}</p>
                          </div>
                          <span className="text-sm font-medium text-yellow-400 shrink-0">{formatBytes(cache.size_bytes)}</span>
                        </label>
                      ))}
                    </div>
                  )}
                </motion.div>
              );
            })}
          </div>
        </>
      )}

      {/* Empty state */}
      {!result && !scanning && (
        <div className="glass-card p-12 rounded-2xl text-center">
          <div className="w-16 h-16 rounded-full bg-white/5 flex items-center justify-center mx-auto mb-4 border border-white/5">
            <Monitor className="w-8 h-8 text-slate-400" />
          </div>
          <h3 className="font-medium text-white text-lg">Scan Development Tools</h3>
          <p className="text-sm text-slate-400 mt-2 max-w-sm mx-auto">
            Click "Scan All" to discover caches from Xcode, Android Studio, Docker, and virtual machines.
          </p>
        </div>
      )}

      {/* Destructive clean confirmation */}
      {confirmClean && (
        <ConfirmDialog
          title="Clean selected caches?"
          message={`Permanently delete ${selected.size} dev tool cache${selected.size === 1 ? '' : 's'}.`}
          paths={Array.from(selected)}
          totalSize={formatBytes(totalSelected)}
          confirmLabel="Clean"
          onCancel={() => setConfirmClean(false)}
          onConfirm={runClean}
        />
      )}
    </div>
  );
}

function toolName(type: string): string {
  switch (type) {
    case 'xcode': return 'Xcode';
    case 'android_studio': return 'Android Studio';
    case 'docker': return 'Docker';
    case 'virtual_machines': return 'Virtual Machines';
    default: return type;
  }
}
