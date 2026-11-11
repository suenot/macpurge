'use client';

import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Code2, Trash2, RefreshCw, Filter, Check, AlertCircle, Square, Loader2, Play } from 'lucide-react';
import Link from 'next/link';
import { clsx } from 'clsx';
import {
  scanDev,
  cleanArtifacts,
  cancelCleanup,
  onCleanStarted,
  onCleanProgress,
  onCleanCompleted,
  isTauri,
} from '@/lib/tauri';
import type { ScanResult, CleanProgressPayload, CleanCompletedPayload } from '@/lib/types';
import { formatBytes } from '@/lib/types';
import { IS_DEMO } from '@/lib/bridge';
import ConfirmDialog from '@/components/ui/ConfirmDialog';
import { useConfirmBeforeDelete } from '@/hooks/useConfirmBeforeDelete';

type ItemStatus = 'pending' | 'deleting' | 'deleted' | 'error' | 'skipped';

interface ItemState {
  path: string;
  status: ItemStatus;
  error?: string;
  bytesFreed?: number;
}

export default function DevPage() {
  const [isScanning, setIsScanning] = useState(false);
  const [isCleaning, setIsCleaning] = useState(false);
  const [scanResults, setScanResults] = useState<ScanResult[]>([]);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState<string>('all');
  const [error, setError] = useState<string | null>(null);
  const [confirmClean, setConfirmClean] = useState(false);
  const confirmBeforeDelete = useConfirmBeforeDelete();

  // Cleanup progress state
  const [cleanProgress, setCleanProgress] = useState<Map<string, ItemState>>(new Map());
  const [currentlyDeleting, setCurrentlyDeleting] = useState<string | null>(null);
  const [cleanStats, setCleanStats] = useState({ removed: 0, bytesFreed: 0, total: 0 });

  // Setup event listeners
  useEffect(() => {
    if (!isTauri()) return;

    let unlistenStarted: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    let unlistenCompleted: (() => void) | undefined;

    const setup = async () => {
      unlistenStarted = await onCleanStarted((payload) => {
        setIsCleaning(true);
        setCleanStats({ removed: 0, bytesFreed: 0, total: payload.total_items });

        // Initialize all paths as pending
        const initial = new Map<string, ItemState>();
        payload.paths.forEach((path) => {
          initial.set(path, { path, status: 'pending' });
        });
        setCleanProgress(initial);
      });

      unlistenProgress = await onCleanProgress((payload: CleanProgressPayload) => {
        setCurrentlyDeleting(payload.status === 'deleting' ? payload.current_path : null);

        setCleanProgress((prev) => {
          const updated = new Map(prev);
          updated.set(payload.current_path, {
            path: payload.current_path,
            status: payload.status as ItemStatus,
            error: payload.error ?? undefined,
            bytesFreed: payload.bytes_freed,
          });
          return updated;
        });

        if (payload.status === 'deleted') {
          setCleanStats((prev) => ({
            ...prev,
            removed: prev.removed + 1,
            bytesFreed: prev.bytesFreed + payload.bytes_freed,
          }));

          // Remove from selected and scan results
          setSelectedPaths((prev) => {
            const updated = new Set(prev);
            updated.delete(payload.current_path);
            return updated;
          });

          setScanResults((prev) =>
            prev.map((result) => ({
              ...result,
              entries: result.entries.filter((e) => e.path !== payload.current_path),
            }))
          );
        }
      });

      unlistenCompleted = await onCleanCompleted((payload: CleanCompletedPayload) => {
        setIsCleaning(false);
        setCurrentlyDeleting(null);

        if (payload.cancelled) {
          setError('Cleanup was cancelled');
        } else if (payload.errors.length > 0) {
          setError(`Completed with ${payload.errors.length} error(s)`);
        }

        // Clear progress after a delay
        setTimeout(() => {
          setCleanProgress(new Map());
        }, 2000);
      });
    };

    setup();

    return () => {
      unlistenStarted?.();
      unlistenProgress?.();
      unlistenCompleted?.();
    };
  }, []);

  const handleScan = async () => {
    if (!isTauri()) return;

    setIsScanning(true);
    setError(null);

    try {
      const results = await scanDev();
      setScanResults(results);
      setSelectedPaths(new Set());
    } catch (err) {
      setError(err as string);
    } finally {
      setIsScanning(false);
    }
  };

  // Website demo only: scan on open so the list is populated.
  useEffect(() => {
    if (IS_DEMO) handleScan();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleClean = () => {
    if (selectedPaths.size === 0) return;
    if (confirmBeforeDelete) {
      setConfirmClean(true);
      return;
    }
    runClean();
  };

  const runClean = async () => {
    if (selectedPaths.size === 0) return;

    setConfirmClean(false);
    setError(null);
    const paths = Array.from(selectedPaths);

    try {
      // Event-driven progress (onCleanProgress/onCleanCompleted) is the primary
      // source of truth. The awaited result is a safety net: if the completion
      // event never fires, this still clears `isCleaning` and reconciles stats so
      // the UI doesn't stay stuck "Cleaning in progress...". Guarded against
      // double-counting by only filling in when the completion event hasn't
      // already done so.
      const result = await cleanArtifacts(paths, false);
      setIsCleaning((stillCleaning) => {
        if (stillCleaning) {
          setCurrentlyDeleting(null);
          setCleanStats((prev) =>
            prev.removed === 0 && prev.bytesFreed === 0
              ? { ...prev, removed: result.items_removed, bytesFreed: result.bytes_freed }
              : prev
          );
          if (result.errors.length > 0) {
            setError(`Completed with ${result.errors.length} error(s)`);
          }
        }
        return false;
      });
    } catch (err) {
      setError(err as string);
      setIsCleaning(false);
    }
  };

  const handleCancel = async () => {
    try {
      await cancelCleanup();
    } catch (err) {
      console.error('Failed to cancel:', err);
    }
  };

  const togglePath = (path: string) => {
    if (isCleaning) return; // Don't allow selection during cleanup

    const newSelected = new Set(selectedPaths);
    if (newSelected.has(path)) {
      newSelected.delete(path);
    } else {
      newSelected.add(path);
    }
    setSelectedPaths(newSelected);
  };

  const selectAll = () => {
    if (isCleaning) return;
    const allPaths = new Set(
      scanResults.flatMap((r) => r.entries.map((e) => e.path))
    );
    setSelectedPaths(allPaths);
  };

  const selectNone = () => {
    if (isCleaning) return;
    setSelectedPaths(new Set());
  };

  // Get all entries with filtering
  const allEntries = scanResults.flatMap((r) =>
    r.entries.map((e) => ({ ...e, root: r.root_path }))
  );

  const filteredEntries = filter === 'all'
    ? allEntries
    : allEntries.filter((e) => e.file_type === filter);

  const selectedSize = filteredEntries
    .filter((e) => selectedPaths.has(e.path))
    .reduce((acc, e) => acc + e.size_bytes, 0);

  // Get unique types for filter
  const types = [...new Set(allEntries.map((e) => e.file_type))];

  const getItemStatus = (path: string): ItemState | undefined => {
    return cleanProgress.get(path);
  };

  return (
    <div className="space-y-6 flex flex-col h-[calc(100vh-6rem)]">
      {/* Header */}
      <div className="flex items-center justify-between flex-shrink-0">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">
            Dev Artifacts
          </h1>
          <p className="text-slate-400 mt-1">
            Clean up node_modules, target/, venv, and other build debris
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={handleScan}
            disabled={isScanning || isCleaning}
            className={clsx(
              "px-4 py-2 rounded-xl text-sm font-medium transition-all flex items-center gap-2",
              isScanning
                ? "bg-white/5 text-slate-400 cursor-not-allowed"
                : "bg-white/10 text-white hover:bg-white/20"
            )}
          >
            <RefreshCw className={clsx('w-4 h-4', isScanning && 'animate-spin')} />
            {isScanning ? 'Scanning...' : 'Rescan'}
          </button>

          {isCleaning ? (
            <button
              onClick={handleCancel}
              className="px-4 py-2 bg-yellow-500/10 hover:bg-yellow-500/20 text-yellow-400 rounded-xl text-sm font-medium transition-all flex items-center gap-2 border border-yellow-500/20"
            >
              <Square className="w-4 h-4 fill-current" />
              Stop ({cleanStats.removed}/{cleanStats.total})
            </button>
          ) : (
            <button
              onClick={handleClean}
              disabled={selectedPaths.size === 0}
              className={clsx(
                'px-4 py-2 rounded-xl text-sm font-medium transition-all flex items-center gap-2 shadow-lg',
                selectedPaths.size === 0
                  ? 'bg-white/5 text-slate-500 cursor-not-allowed'
                  : 'bg-danger-600 hover:bg-danger-500 text-white shadow-danger-500/20'
              )}
            >
              <Trash2 className="w-4 h-4" />
              Clean Selected ({formatBytes(selectedSize)})
            </button>
          )}
        </div>
      </div>

      {/* Progress bar during cleanup */}
      <AnimatePresence>
        {isCleaning && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="flex-shrink-0"
          >
            <div className="p-5 rounded-2xl bg-brand-500/10 border border-brand-500/20 backdrop-blur-sm">
              <div className="flex items-center justify-between mb-3">
                <span className="text-sm font-medium text-brand-300">
                  Cleaning in progress...
                </span>
                <span className="text-sm text-brand-400 font-mono">
                  {cleanStats.removed} / {cleanStats.total} items • {formatBytes(cleanStats.bytesFreed)} freed
                </span>
              </div>
              <div className="h-2 bg-black/20 rounded-full overflow-hidden">
                <motion.div
                  className="h-full bg-brand-500 relative"
                  initial={{ width: 0 }}
                  animate={{ width: cleanStats.total > 0 ? `${(cleanStats.removed / cleanStats.total) * 100}%` : '0%' }}
                  transition={{ duration: 0.3 }}
                >
                  <div className="absolute inset-0 bg-white/20 animate-pulse" />
                </motion.div>
              </div>
              {currentlyDeleting && (
                <p className="text-xs text-brand-400/80 mt-3 truncate font-mono">
                  ➜ Deleting: <span className="text-brand-300">{currentlyDeleting}</span>
                </p>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Error display */}
      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="p-4 rounded-xl border border-yellow-500/30 bg-yellow-500/10 flex-shrink-0"
          >
            <div className="flex items-start gap-3">
              <AlertCircle className="w-5 h-5 text-yellow-500 mt-0.5 flex-shrink-0" />
              <div>
                <p className="text-sm text-yellow-200">{error}</p>
                {error.includes('Settings') && (
                  <Link href="/settings" className="text-sm text-yellow-400 underline mt-1 inline-block hover:text-yellow-300">
                    Go to Settings
                  </Link>
                )}
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Filter and Selection */}
      {allEntries.length > 0 && (
        <div className="flex items-center justify-between flex-shrink-0 bg-black/20 p-3 rounded-xl border border-white/5">
          <div className="flex items-center gap-3">
            <Filter className="w-4 h-4 text-slate-400" />
            <select
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              disabled={isCleaning}
              className="px-3 py-1.5 rounded-lg border border-white/10 bg-white/5 text-slate-200 text-sm focus:bg-white/10 outline-none"
            >
              <option value="all">All Types</option>
              {types.map((type) => (
                <option key={type} value={type}>
                  {type}
                </option>
              ))}
            </select>
            <span className="text-xs text-slate-500 ml-2">{filteredEntries.length} items found</span>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={selectAll}
              disabled={isCleaning}
              className="text-xs font-medium text-brand-400 hover:text-brand-300 hover:underline disabled:opacity-50 transition-colors"
            >
              Select All
            </button>
            <span className="text-slate-600">|</span>
            <button
              onClick={selectNone}
              disabled={isCleaning}
              className="text-xs font-medium text-slate-400 hover:text-white hover:underline disabled:opacity-50 transition-colors"
            >
              Select None
            </button>
          </div>
        </div>
      )}

      {/* Results List - Flex Grow to fill space */}
      <div className="glass-card rounded-2xl flex-1 overflow-hidden flex flex-col min-h-0">
        {filteredEntries.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center p-12 text-center">
            <div className="w-20 h-20 bg-white/5 rounded-full flex items-center justify-center mb-6">
              <Code2 className="w-10 h-10 text-slate-500" />
            </div>
            <h3 className="text-lg font-medium text-white mb-2">No Artifacts Found</h3>
            <p className="text-slate-400 max-w-sm">
              {isScanning
                ? 'Scanning your project directories...'
                : 'Click "Rescan" to check your folders for cleanable items.'}
            </p>
            {!isScanning && (
              <button onClick={handleScan} className="mt-6 px-6 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl text-sm font-medium transition-colors">
                Start Scan
              </button>
            )}
          </div>
        ) : (
          <div className="flex-1 overflow-y-auto custom-scrollbar p-2 space-y-1">
            <AnimatePresence>
              {filteredEntries.map((entry, i) => {
                const itemState = getItemStatus(entry.path);
                const isDeleting = currentlyDeleting === entry.path;
                const isDeleted = itemState?.status === 'deleted';
                const hasError = itemState?.status === 'error';
                const isSelected = selectedPaths.has(entry.path);

                return (
                  <motion.div
                    key={entry.path}
                    initial={{ opacity: 0 }}
                    animate={{
                      opacity: isDeleted ? 0.3 : 1,
                      backgroundColor: isDeleting
                        ? 'rgba(238, 111, 19, 0.1)'
                        : hasError
                          ? 'rgba(239, 68, 68, 0.1)'
                          : isSelected
                            ? 'rgba(238, 111, 19, 0.05)'
                            : 'transparent',
                    }}
                    exit={{ opacity: 0, height: 0 }}
                    transition={{ duration: 0.2 }}
                    className={clsx(
                      'flex items-center gap-4 p-3 rounded-lg cursor-pointer transition-all border border-transparent',
                      !isCleaning && !isDeleted && 'hover:bg-white/5 hover:border-white/5',
                      isSelected && !isCleaning && 'border-brand-500/20',
                      isDeleted && 'grayscale'
                    )}
                    onClick={() => togglePath(entry.path)}
                  >
                    {/* Checkbox / Status Icon */}
                    <div className="w-8 h-8 flex items-center justify-center flex-shrink-0">
                      {isDeleting ? (
                        <Loader2 className="w-5 h-5 text-brand-500 animate-spin" />
                      ) : isDeleted ? (
                        <Check className="w-5 h-5 text-emerald-500" />
                      ) : hasError ? (
                        <AlertCircle className="w-5 h-5 text-danger-500" />
                      ) : (
                        <div
                          className={clsx(
                            'w-5 h-5 rounded flex items-center justify-center transition-all duration-200 border',
                            isSelected
                              ? 'bg-brand-500 border-brand-500 shadow-sm shadow-brand-500/50'
                              : 'border-slate-600 bg-white/5 hover:border-slate-500'
                          )}
                        >
                          {isSelected && (
                            <Check className="w-3.5 h-3.5 text-white stroke-[3]" />
                          )}
                        </div>
                      )}
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-0.5">
                        <p className={clsx(
                          'text-sm font-medium truncate',
                          isDeleted ? 'text-slate-500 line-through' : 'text-slate-200'
                        )}>
                          {entry.path.split('/').pop()}
                        </p>
                        <span className="text-[10px] uppercase font-bold tracking-wider px-1.5 rounded bg-white/10 text-slate-400">
                          {entry.file_type}
                        </span>
                      </div>
                      <p className={clsx("text-xs truncate font-mono", isDeleted ? 'text-slate-600' : 'text-slate-500')}>
                        {entry.path}
                        {hasError && itemState?.error && (
                          <span className="text-danger-400 ml-2">• {itemState.error}</span>
                        )}
                      </p>
                    </div>

                    <span className={clsx(
                      'text-sm font-mono flex-shrink-0',
                      isDeleted
                        ? 'text-slate-600'
                        : 'text-slate-300'
                    )}>
                      {formatBytes(entry.size_bytes)}
                    </span>
                  </motion.div>
                );
              })}
            </AnimatePresence>
          </div>
        )}
      </div>

      {/* Summary Footer */}
      {selectedPaths.size > 0 && !isCleaning && (
        <motion.div
          initial={{ y: 20, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          className="glass-card absolute bottom-6 left-1/2 -translate-x-1/2 px-6 py-3 rounded-full flex items-center gap-4 shadow-2xl border-brand-500/30 z-30"
        >
          <span className="text-sm font-medium text-white">
            {selectedPaths.size} items selected
          </span>
          <span className="h-4 w-px bg-white/20" />
          <span className="text-sm font-bold text-brand-300 font-mono">
            {formatBytes(selectedSize)}
          </span>
        </motion.div>
      )}

      {/* Destructive clean confirmation */}
      {confirmClean && (
        <ConfirmDialog
          title="Clean selected artifacts?"
          message={`Permanently delete ${selectedPaths.size} item${selectedPaths.size === 1 ? '' : 's'} from your project directories.`}
          paths={Array.from(selectedPaths)}
          totalSize={formatBytes(selectedSize)}
          confirmLabel="Clean"
          onCancel={() => setConfirmClean(false)}
          onConfirm={runClean}
        />
      )}
    </div>
  );
}
