'use client';

import { useEffect, useState } from 'react';
import { Package, Trash2, RefreshCw, AlertCircle, CheckCircle, Info } from 'lucide-react';
import { clsx } from 'clsx';
import { scanPackages, cleanPackageCache, isTauri } from '@/lib/tauri';
import type { PackageScanResult, PackageManagerInfo, CachePath } from '@/lib/types';
import { formatBytes } from '@/lib/types';
import { motion, AnimatePresence } from 'framer-motion';
import ConfirmDialog from '@/components/ui/ConfirmDialog';
import { useConfirmBeforeDelete } from '@/hooks/useConfirmBeforeDelete';

// Package manager icons/colors
const managerConfig: Record<string, { icon: string; color: string }> = {
  npm: { icon: '📦', color: 'bg-red-500/20 text-red-400' },
  yarn: { icon: '🧶', color: 'bg-blue-500/20 text-blue-400' },
  pnpm: { icon: '⚡', color: 'bg-yellow-500/20 text-yellow-400' },
  pip: { icon: '🐍', color: 'bg-emerald-500/20 text-emerald-400' },
  cargo: { icon: '🦀', color: 'bg-orange-500/20 text-orange-400' },
  rustup: { icon: '⚙️', color: 'bg-orange-600/20 text-orange-500' },
  go: { icon: '🐹', color: 'bg-cyan-500/20 text-cyan-400' },
  gem: { icon: '💎', color: 'bg-red-600/20 text-red-500' },
  brew: { icon: '🍺', color: 'bg-amber-500/20 text-amber-400' },
  composer: { icon: '🎼', color: 'bg-purple-500/20 text-purple-400' },
};

export default function PackagesPage() {
  const [result, setResult] = useState<PackageScanResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [cleaning, setCleaning] = useState<string | null>(null);
  const [selectedCaches, setSelectedCaches] = useState<Set<string>>(new Set());
  // Holds the paths awaiting confirmation before a destructive clean runs.
  const [pendingClean, setPendingClean] = useState<string[] | null>(null);
  const confirmBeforeDelete = useConfirmBeforeDelete();

  const fetchData = async () => {
    if (!isTauri()) {
      setError('This feature requires the Tauri desktop app');
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const data = await scanPackages();
      setResult(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to scan packages');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  // Per-row clean: gate behind a confirmation modal when configured to do so.
  const handleCleanCache = (path: string) => {
    if (confirmBeforeDelete) {
      setPendingClean([path]);
      return;
    }
    runCleanCache(path);
  };

  const runCleanCache = async (path: string) => {
    setCleaning(path);
    try {
      const result = await cleanPackageCache(path, false);
      if (result.success) {
        // Refresh data after cleaning
        await fetchData();
      } else {
        setError(result.message);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to clean cache');
    } finally {
      setCleaning(null);
    }
  };

  const handleCleanSelected = () => {
    if (selectedCaches.size === 0) return;
    if (confirmBeforeDelete) {
      setPendingClean(Array.from(selectedCaches));
      return;
    }
    runCleanSelected(Array.from(selectedCaches));
  };

  const runCleanSelected = async (paths: string[]) => {
    for (const path of paths) {
      await runCleanCache(path);
    }
    setSelectedCaches(new Set());
  };

  // Runs the pending clean once the user confirms. A single path came from a
  // row's trash button; multiple paths came from "Clean Selected".
  const confirmPendingClean = () => {
    const paths = pendingClean;
    setPendingClean(null);
    if (!paths) return;
    if (paths.length === 1) {
      runCleanCache(paths[0]);
    } else {
      runCleanSelected(paths);
    }
  };

  // Total size of the caches awaiting confirmation, for the modal label.
  const pendingCleanSize = (): number => {
    if (!result || !pendingClean) return 0;
    const set = new Set(pendingClean);
    let size = 0;
    for (const manager of result.managers) {
      for (const cache of manager.cache_paths) {
        if (set.has(cache.path)) size += cache.size_bytes;
      }
    }
    return size;
  };

  const toggleCacheSelection = (path: string) => {
    const newSelection = new Set(selectedCaches);
    if (newSelection.has(path)) {
      newSelection.delete(path);
    } else {
      newSelection.add(path);
    }
    setSelectedCaches(newSelection);
  };

  const selectAllSafe = () => {
    if (!result) return;
    const safeCaches = new Set<string>();
    for (const manager of result.managers) {
      for (const cache of manager.cache_paths) {
        if (cache.safe_to_clean && cache.size_bytes > 0) {
          safeCaches.add(cache.path);
        }
      }
    }
    setSelectedCaches(safeCaches);
  };

  const selectedSize = (): number => {
    if (!result) return 0;
    let size = 0;
    for (const manager of result.managers) {
      for (const cache of manager.cache_paths) {
        if (selectedCaches.has(cache.path)) {
          size += cache.size_bytes;
        }
      }
    }
    return size;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-[60vh]">
        <div className="text-center">
          <RefreshCw className="w-8 h-8 animate-spin text-brand-500 mx-auto mb-4" />
          <p className="text-slate-400">Scanning package caches...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-[60vh]">
        <div className="text-center bg-danger-500/10 p-8 rounded-2xl border border-danger-500/20">
          <AlertCircle className="w-8 h-8 text-danger-500 mx-auto mb-4" />
          <p className="text-danger-200 mb-6">{error}</p>
          <button
            onClick={fetchData}
            className="px-6 py-2 bg-danger-600 text-white rounded-xl hover:bg-danger-500 transition-colors"
          >
            Retry Scan
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6 pb-20">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">
            Package Caches
          </h1>
          <p className="text-slate-400 mt-1">
            Manage npm, cargo, pip, and other package manager storages
          </p>
        </div>
        <div className="flex gap-3">
          <button
            onClick={fetchData}
            disabled={loading}
            className="flex items-center gap-2 px-4 py-2 bg-white/5 text-white rounded-xl hover:bg-white/10 transition-colors"
          >
            <RefreshCw className={clsx('w-4 h-4', loading && 'animate-spin')} />
            Refresh
          </button>
          {selectedCaches.size > 0 && (
            <button
              onClick={handleCleanSelected}
              className="flex items-center gap-2 px-4 py-2 bg-danger-600 text-white rounded-xl hover:bg-danger-500 transition-colors shadow-lg shadow-danger-500/20"
            >
              <Trash2 className="w-4 h-4" />
              Clean Selected ({formatBytes(selectedSize())})
            </button>
          )}
        </div>
      </div>

      {/* Summary Card */}
      {result && (
        <div className="glass-card mb-8">
          <div className="p-6 flex items-center justify-between">
            <div className="flex items-center gap-6">
              <div>
                <p className="text-sm text-slate-400 uppercase tracking-wider font-semibold">Total Cache Size</p>
                <div className="flex items-baseline gap-2 mt-1">
                  <p className="text-4xl font-bold text-white">
                    {formatBytes(result.total_cache_size).split(' ')[0]}
                  </p>
                  <p className="text-xl text-slate-500 font-medium">
                    {formatBytes(result.total_cache_size).split(' ')[1]}
                  </p>
                </div>
              </div>

              <div className="h-10 w-px bg-white/10" />

              <div className="text-sm text-slate-400">
                <p>{result.managers.filter(m => m.is_installed).length} managers installed</p>
                <p>{result.scan_duration_ms}ms scan time</p>
              </div>
            </div>

            <button
              onClick={selectAllSafe}
              className="px-5 py-2.5 bg-brand-600/20 text-brand-300 border border-brand-500/30 rounded-xl hover:bg-brand-600 hover:text-white hover:border-brand-500 transition-all"
            >
              Select All Safe Items
            </button>
          </div>
        </div>
      )}

      {/* Package Managers Grid */}
      {result && (
        <div className="space-y-6">
          {result.managers
            .filter((m) => m.is_installed || m.cache_paths.length > 0)
            .sort((a, b) => b.total_cache_size - a.total_cache_size)
            .map((manager, idx) => (
              <motion.div
                key={manager.manager}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: idx * 0.1 }}
              >
                <PackageManagerCard
                  manager={manager}
                  selectedCaches={selectedCaches}
                  onToggleCache={toggleCacheSelection}
                  onCleanCache={handleCleanCache}
                  cleaning={cleaning}
                />
              </motion.div>
            ))}
        </div>
      )}

      {/* Destructive clean confirmation */}
      {pendingClean && (
        <ConfirmDialog
          title="Clean selected caches?"
          message={`Permanently delete ${pendingClean.length} package cache${pendingClean.length === 1 ? '' : 's'}. Package managers will re-download these on next use.`}
          paths={pendingClean}
          totalSize={formatBytes(pendingCleanSize())}
          confirmLabel="Clean"
          onCancel={() => setPendingClean(null)}
          onConfirm={confirmPendingClean}
        />
      )}
    </div>
  );
}

function PackageManagerCard({
  manager,
  selectedCaches,
  onToggleCache,
  onCleanCache,
  cleaning,
}: {
  manager: PackageManagerInfo;
  selectedCaches: Set<string>;
  onToggleCache: (path: string) => void;
  onCleanCache: (path: string) => void;
  cleaning: string | null;
}) {
  const config = managerConfig[manager.manager] || { icon: '📦', color: 'bg-slate-500/20 text-slate-400' };

  if (manager.cache_paths.length === 0 && !manager.is_installed) {
    return null;
  }

  return (
    <div className="glass-card rounded-2xl overflow-hidden backdrop-blur-md">
      {/* Header */}
      <div className="flex items-center justify-between p-5 border-b border-white/5 bg-white/[0.02]">
        <div className="flex items-center gap-4">
          <div
            className={clsx(
              'w-12 h-12 rounded-xl flex items-center justify-center text-2xl border border-white/5',
              config.color
            )}
          >
            {config.icon}
          </div>
          <div>
            <h3 className="font-bold text-lg text-white capitalize flex items-center gap-2">
              {manager.manager}
              {manager.is_installed ? (
                <span className="flex items-center gap-1 text-[10px] bg-emerald-500/10 text-emerald-400 px-2 py-0.5 rounded-full border border-emerald-500/20 uppercase tracking-wide">
                  Installed
                </span>
              ) : (
                <span className="text-[10px] bg-slate-800 text-slate-400 px-2 py-0.5 rounded-full border border-white/5 uppercase tracking-wide">
                  Not Installed
                </span>
              )}
            </h3>
            <div className="flex items-center gap-3 text-xs text-slate-400 mt-1">
              <span>{manager.cache_paths.length} cache paths</span>
              {manager.global_packages.length > 0 && (
                <>
                  <span className="w-1 h-1 bg-slate-600 rounded-full" />
                  <span>{manager.global_packages.length} global packages</span>
                </>
              )}
            </div>
          </div>
        </div>
        <div className="text-right">
          <p className="text-2xl font-bold text-white font-mono tracking-tight">
            {formatBytes(manager.total_cache_size)}
          </p>
        </div>
      </div>

      {/* Cache Paths */}
      {manager.cache_paths.length > 0 && (
        <div className="divide-y divide-white/5">
          {manager.cache_paths.map((cache) => (
            <CachePathRow
              key={cache.path}
              cache={cache}
              isSelected={selectedCaches.has(cache.path)}
              onToggle={() => onToggleCache(cache.path)}
              onClean={() => onCleanCache(cache.path)}
              isCleaning={cleaning === cache.path}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function CachePathRow({
  cache,
  isSelected,
  onToggle,
  onClean,
  isCleaning,
}: {
  cache: CachePath;
  isSelected: boolean;
  onToggle: () => void;
  onClean: () => void;
  isCleaning: boolean;
}) {
  const isEmpty = cache.size_bytes === 0;

  return (
    <div
      className={clsx(
        'flex items-center gap-4 p-4 transition-all hover:bg-white/5',
        isSelected && 'bg-brand-500/5',
        isEmpty && 'opacity-40'
      )}
    >
      {/* Checkbox */}
      <div className="relative flex items-center">
        <input
          type="checkbox"
          checked={isSelected}
          onChange={onToggle}
          disabled={isEmpty || !cache.safe_to_clean}
          className="peer w-5 h-5 rounded border border-slate-600 bg-black/40 text-brand-500 focus:ring-brand-500 disabled:opacity-50 appearance-none checked:bg-brand-500 checked:border-brand-500 transition-all cursor-pointer"
        />
        <div className="absolute inset-0 pointer-events-none text-white hidden peer-checked:flex items-center justify-center">
          <CheckCircle className="w-3.5 h-3.5" />
        </div>
      </div>

      {/* Safe indicator */}
      <div className="w-8 flex justify-center">
        {cache.safe_to_clean ? (
          <div title="Safe to clean" className="p-1.5 rounded-lg bg-emerald-500/10">
            <Trash2 className="w-4 h-4 text-emerald-500" />
          </div>
        ) : (
          <div title="Use caution" className="p-1.5 rounded-lg bg-amber-500/10">
            <Info className="w-4 h-4 text-amber-500" />
          </div>
        )}
      </div>

      {/* Path info */}
      <div className="flex-1 min-w-0">
        <p className="text-sm font-mono text-slate-300 truncate" title={cache.path}>
          {cache.path}
        </p>
        <p className="text-xs text-slate-500 mt-0.5">{cache.description || "Cache storage"}</p>
      </div>

      {/* Size */}
      <div className="text-right w-24">
        <p className="text-sm font-bold text-white font-mono">
          {formatBytes(cache.size_bytes)}
        </p>
      </div>

      {/* Clean button */}
      <button
        onClick={onClean}
        disabled={isEmpty || isCleaning}
        className={clsx(
          'px-3.5 py-2 rounded-lg text-sm font-medium transition-colors ml-2',
          isEmpty
            ? 'bg-slate-800 text-slate-600 cursor-not-allowed'
            : 'bg-danger-500/10 text-danger-400 hover:bg-danger-500 hover:text-white border border-danger-500/20'
        )}
      >
        {isCleaning ? (
          <RefreshCw className="w-4 h-4 animate-spin" />
        ) : (
          <Trash2 className="w-4 h-4" />
        )}
      </button>
    </div>
  );
}
