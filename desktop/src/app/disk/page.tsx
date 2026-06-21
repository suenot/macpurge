'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  PieChart,
  RefreshCw,
  Loader2,
  ChevronRight,
  FolderOpen,
  Trash2,
  HardDrive,
  Home,
  X,
  Folder,
  File as FileIcon,
  AlertTriangle,
} from 'lucide-react';
import { clsx } from 'clsx';
import {
  scanUsage,
  getUsageSubtree,
  cancelUsageScan,
  trashUsageItem,
  revealInFinder,
  getHomeDir,
  getUserDisks,
  onUsageScanProgress,
  isTauri,
} from '@/lib/tauri';
import type { UsageNode, UsageScanProgress, DiskInfo } from '@/lib/types';
import { formatBytes } from '@/lib/types';
import { toMessage } from '@/lib/errors';
import Sunburst, { hueForChildIndex } from '@/components/Sunburst';
import * as d3 from 'd3';

type Phase = 'idle' | 'scanning' | 'ready';

// Match the Sunburst's depth-1 color so list rows and arcs share a palette.
function swatchColor(index: number, total: number): string {
  const hue = hueForChildIndex(index, total);
  return d3.interpolateRainbow(hue);
}

export default function DiskPage() {
  const [phase, setPhase] = useState<Phase>('idle');
  const [error, setError] = useState<string | null>(null);

  // Scan target selection.
  const [homeDir, setHomeDir] = useState<string | null>(null);
  const [disks, setDisks] = useState<DiskInfo[]>([]);
  const [scanRoot, setScanRoot] = useState<string | null>(null);

  // Scan progress.
  const [progress, setProgress] = useState<UsageScanProgress | null>(null);

  // The scanned tree root and the breadcrumb path down to the current view.
  const [breadcrumb, setBreadcrumb] = useState<UsageNode[]>([]);
  const view = breadcrumb[breadcrumb.length - 1] ?? null;

  // Hovered node (kept in the page so arcs don't re-render on mousemove).
  const [hovered, setHovered] = useState<UsageNode | null>(null);

  // Trash confirmation + status.
  const [pendingTrash, setPendingTrash] = useState<UsageNode | null>(null);
  const [trashing, setTrashing] = useState<string | null>(null);
  const [trashedMsg, setTrashedMsg] = useState<string | null>(null);

  // Bootstrap default scan root + available disks.
  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    (async () => {
      try {
        const home = await getHomeDir();
        if (cancelled) return;
        setHomeDir(home);
        setScanRoot((prev) => prev ?? home);
      } catch {
        /* home unavailable; disk roots can still be chosen */
      }
      try {
        const d = await getUserDisks();
        if (!cancelled) setDisks(d);
      } catch {
        /* disks unavailable */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const handleScan = useCallback(async () => {
    if (!isTauri() || !scanRoot) return;
    setPhase('scanning');
    setError(null);
    setProgress(null);
    setBreadcrumb([]);
    setHovered(null);
    setTrashedMsg(null);

    try {
      const root = await scanUsage(scanRoot, true);
      setBreadcrumb([root]);
      setPhase('ready');
    } catch (err) {
      setError(toMessage(err));
      setPhase('idle');
    }
  }, [scanRoot]);

  // Subscribe to live progress only while scanning.
  useEffect(() => {
    if (phase !== 'scanning' || !isTauri()) return;
    let mounted = true;
    let un: (() => void) | undefined;
    (async () => {
      un = await onUsageScanProgress((p) => {
        if (mounted) setProgress(p);
      });
      if (!mounted) un?.();
    })();
    return () => {
      mounted = false;
      un?.();
    };
  }, [phase]);

  const handleCancel = useCallback(async () => {
    try {
      await cancelUsageScan();
    } catch {
      /* best-effort */
    }
  }, []);

  // Drill into a directory: fetch a deeper subtree if the node is capped/aggregated,
  // otherwise re-root into the in-memory subtree we already have.
  const handleDrill = useCallback(async (node: UsageNode) => {
    if (!node.is_dir) return;
    if (node.has_more) {
      try {
        const deeper = await getUsageSubtree(node.path);
        setBreadcrumb((prev) => [...prev, deeper]);
        return;
      } catch (err) {
        setError(toMessage(err));
        return;
      }
    }
    setBreadcrumb((prev) => [...prev, node]);
  }, []);

  // Breadcrumb navigation: re-root at a crumb index.
  const goToCrumb = useCallback((index: number) => {
    setBreadcrumb((prev) => prev.slice(0, index + 1));
  }, []);

  // Center hub / Up: pop one level off the breadcrumb (no-op at the root).
  const goUp = useCallback(() => {
    setBreadcrumb((prev) => (prev.length > 1 ? prev.slice(0, -1) : prev));
  }, []);

  const confirmTrash = useCallback(async () => {
    const node = pendingTrash;
    if (!node) return;
    setPendingTrash(null);
    setTrashing(node.path);
    try {
      const freed = await trashUsageItem(node.path);
      setTrashedMsg(`Moved ${node.name} (${formatBytes(freed)}) to Trash`);
      // Remove the node from the current in-memory view so the UI updates.
      setBreadcrumb((prev) =>
        prev.map((n) => ({
          ...n,
          children: n.children.filter((c) => c.path !== node.path),
        }))
      );
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setTrashing(null);
    }
  }, [pendingTrash]);

  const children = view?.children ?? [];
  const maxSiblingSize = useMemo(
    () => children.reduce((m, c) => Math.max(m, c.size_bytes), 1),
    [children]
  );

  const targetLabel = (root: string | null) => {
    if (!root) return 'Select target';
    if (homeDir && root === homeDir) return 'Home';
    const disk = disks.find((d) => d.mount_point === root);
    return disk ? disk.name : root;
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">Disk Usage</h1>
          <p className="text-slate-400 mt-1">
            Explore what is taking up space, then move items to the Trash (reversible).
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Scan target selector */}
          <select
            value={scanRoot ?? ''}
            onChange={(e) => setScanRoot(e.target.value)}
            disabled={phase === 'scanning'}
            className="px-3 py-2.5 rounded-xl bg-white/5 border border-white/10 text-sm text-white focus:outline-none focus:ring-2 focus:ring-brand-500/40 disabled:opacity-50"
          >
            {homeDir && <option value={homeDir}>Home ({homeDir})</option>}
            {disks.map((d) => (
              <option key={d.mount_point} value={d.mount_point}>
                {d.name} ({d.mount_point})
              </option>
            ))}
            {!homeDir && disks.length === 0 && <option value="">No targets</option>}
          </select>

          <button
            onClick={handleScan}
            disabled={phase === 'scanning' || !scanRoot}
            className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 text-white font-medium transition-all disabled:opacity-50"
          >
            {phase === 'scanning' ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            {phase === 'scanning' ? 'Scanning...' : 'Scan'}
          </button>
        </div>
      </div>

      {error && (
        <div className="glass-card p-4 rounded-2xl border border-danger-500/30 bg-danger-500/5 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-danger-400 shrink-0 mt-0.5" />
          <span className="text-sm text-danger-200">{error}</span>
          <button onClick={() => setError(null)} className="ml-auto text-slate-400 hover:text-white">
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {trashedMsg && (
        <div className="glass-card p-3 rounded-2xl border border-emerald-500/30 bg-emerald-500/5 flex items-center gap-2 text-sm text-emerald-300">
          <Trash2 className="w-4 h-4" />
          {trashedMsg}
        </div>
      )}

      {/* Scanning progress */}
      {phase === 'scanning' && (
        <div className="glass-card p-6 rounded-2xl">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Loader2 className="w-5 h-5 text-brand-400 animate-spin" />
              <div>
                <p className="text-white font-medium">
                  Scanning {targetLabel(scanRoot)}...
                </p>
                <p className="text-xs text-slate-500 truncate max-w-md mt-0.5">
                  {progress?.current_path ?? 'Starting...'}
                </p>
              </div>
            </div>
            <button
              onClick={handleCancel}
              className="flex items-center gap-2 px-4 py-2 rounded-xl bg-white/5 hover:bg-white/10 text-white text-sm font-medium transition-all"
            >
              <X className="w-4 h-4" /> Cancel
            </button>
          </div>
          {progress && (
            <div className="grid grid-cols-3 gap-4 mt-5">
              <Stat label="Items scanned" value={progress.scanned_items.toLocaleString()} />
              <Stat label="Size so far" value={formatBytes(progress.bytes_so_far)} />
              <Stat label="Unreadable" value={progress.unreadable_count.toLocaleString()} />
            </div>
          )}
        </div>
      )}

      {/* Ready: breadcrumb + sunburst + list */}
      {phase === 'ready' && view && (
        <>
          {/* Breadcrumb */}
          <div className="glass-card px-4 py-2.5 rounded-2xl flex items-center gap-1 flex-wrap text-sm">
            {breadcrumb.map((node, i) => (
              <div key={node.path} className="flex items-center gap-1">
                {i > 0 && <ChevronRight className="w-3.5 h-3.5 text-slate-600" />}
                <button
                  onClick={() => goToCrumb(i)}
                  className={clsx(
                    'px-2 py-1 rounded-lg transition-colors',
                    i === breadcrumb.length - 1
                      ? 'text-white font-medium'
                      : 'text-slate-400 hover:text-white hover:bg-white/5'
                  )}
                >
                  {i === 0 ? (
                    <span className="flex items-center gap-1.5">
                      {homeDir === node.path ? (
                        <Home className="w-3.5 h-3.5" />
                      ) : (
                        <HardDrive className="w-3.5 h-3.5" />
                      )}
                      {node.name || node.path}
                    </span>
                  ) : (
                    node.name
                  )}
                </button>
              </div>
            ))}
            <span className="ml-auto text-xs text-slate-500">
              {formatBytes(view.size_bytes)} · {view.file_count.toLocaleString()} files
            </span>
          </div>

          <div className="flex gap-6 flex-col lg:flex-row">
            {/* Sunburst (≈65%) */}
            <div className="lg:w-[65%] glass-card rounded-2xl p-6 flex flex-col items-center justify-center min-h-[400px]">
              <Sunburst
                root={view}
                onDrill={handleDrill}
                onHover={setHovered}
                onCenter={goUp}
                size={520}
              />
              <div className="mt-4 h-10 text-center">
                {hovered ? (
                  <>
                    <p className="text-white font-medium truncate max-w-md">{hovered.name}</p>
                    <p className="text-xs text-slate-400">
                      {formatBytes(hovered.size_bytes)} · {hovered.file_count.toLocaleString()} files
                    </p>
                  </>
                ) : (
                  <p className="text-xs text-slate-500">
                    Hover a wedge for details · click a folder to drill in · click the center to go up
                  </p>
                )}
              </div>
            </div>

            {/* Largest items (≈35%) */}
            <div className="lg:w-[35%] glass-card rounded-2xl p-4 flex flex-col">
              <h2 className="text-sm font-semibold text-white mb-3 px-1">Largest items</h2>
              <div className="space-y-1 overflow-y-auto custom-scrollbar flex-1 max-h-[520px]">
                {children.length === 0 && (
                  <p className="text-xs text-slate-500 px-2 py-4 text-center">
                    No items in this folder.
                  </p>
                )}
                {children.map((child, i) => {
                  const isAggregate = child.aggregated_count != null;
                  const isTrashing = trashing === child.path;
                  const barPct = Math.max(2, (child.size_bytes / maxSiblingSize) * 100);
                  return (
                    <div
                      key={child.path}
                      className={clsx(
                        'group rounded-xl px-2.5 py-2 transition-colors',
                        child.is_dir || isAggregate
                          ? 'cursor-pointer hover:bg-white/5'
                          : 'hover:bg-white/5'
                      )}
                      onClick={() => {
                        if (isAggregate || child.is_dir) handleDrill(child);
                      }}
                      onMouseEnter={() => setHovered(child)}
                      onMouseLeave={() => setHovered(null)}
                    >
                      <div className="flex items-center gap-2">
                        <span
                          className="w-2.5 h-2.5 rounded-full shrink-0"
                          style={{ backgroundColor: swatchColor(i, children.length) }}
                        />
                        {isAggregate ? (
                          <Folder className="w-3.5 h-3.5 text-slate-500 shrink-0" />
                        ) : child.is_dir ? (
                          <Folder className="w-3.5 h-3.5 text-slate-400 shrink-0" />
                        ) : (
                          <FileIcon className="w-3.5 h-3.5 text-slate-500 shrink-0" />
                        )}
                        <span className="text-sm text-slate-200 truncate flex-1" title={child.path}>
                          {child.name}
                          {isAggregate && (
                            <span className="text-slate-500 ml-1">
                              ({child.aggregated_count} items)
                            </span>
                          )}
                        </span>
                        <span className="text-xs font-medium text-slate-300 shrink-0">
                          {formatBytes(child.size_bytes)}
                        </span>
                      </div>

                      {/* Size bar */}
                      <div className="h-1 bg-white/5 rounded-full overflow-hidden mt-1.5 ml-[18px]">
                        <div
                          className="h-full rounded-full"
                          style={{
                            width: `${barPct}%`,
                            backgroundColor: swatchColor(i, children.length),
                            opacity: 0.6,
                          }}
                        />
                      </div>

                      {/* Actions (not shown for the synthetic aggregate row) */}
                      {!isAggregate && (
                        <div className="flex items-center gap-2 mt-1.5 ml-[18px] opacity-0 group-hover:opacity-100 transition-opacity">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              revealInFinder(child.path).catch((err) => setError(toMessage(err)));
                            }}
                            className="flex items-center gap-1 text-[11px] text-slate-400 hover:text-white"
                          >
                            <FolderOpen className="w-3 h-3" /> Reveal
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setPendingTrash(child);
                            }}
                            disabled={isTrashing}
                            className="flex items-center gap-1 text-[11px] text-danger-400/80 hover:text-danger-300 disabled:opacity-50"
                          >
                            {isTrashing ? (
                              <Loader2 className="w-3 h-3 animate-spin" />
                            ) : (
                              <Trash2 className="w-3 h-3" />
                            )}
                            Trash
                          </button>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </>
      )}

      {/* Empty state */}
      {phase === 'idle' && !error && (
        <div className="glass-card p-12 rounded-2xl text-center">
          <div className="w-16 h-16 rounded-full bg-white/5 flex items-center justify-center mx-auto mb-4 border border-white/5">
            <PieChart className="w-8 h-8 text-slate-400" />
          </div>
          <h3 className="font-medium text-white text-lg">Visualize disk usage</h3>
          <p className="text-sm text-slate-400 mt-2 max-w-sm mx-auto">
            Pick a target and click &quot;Scan&quot; to build a sunburst of what is
            using your space. Drill in by clicking folders.
          </p>
        </div>
      )}

      {/* Trash confirmation — Trash is reversible; never imply permanent deletion. */}
      {pendingTrash && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
          onClick={() => setPendingTrash(null)}
        >
          <div
            className="glass-card w-full max-w-md mx-4 rounded-2xl border border-white/10 p-6"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-lg bg-amber-500/10 border border-amber-500/20 flex items-center justify-center shrink-0">
                <Trash2 className="w-5 h-5 text-amber-400" />
              </div>
              <h2 className="text-lg font-bold text-white">Move to Trash?</h2>
            </div>

            <p className="text-sm text-slate-300">
              Move <span className="font-medium text-white">{pendingTrash.name}</span> (
              {formatBytes(pendingTrash.size_bytes)}) to the macOS Trash.
            </p>
            <p className="text-xs text-slate-500 font-mono truncate mt-2" title={pendingTrash.path}>
              {pendingTrash.path}
            </p>
            <p className="text-xs text-emerald-300/90 mt-3">
              This is reversible — you can restore it from the Trash until it is emptied.
            </p>

            <div className="flex items-center justify-end gap-3 mt-6">
              <button
                onClick={() => setPendingTrash(null)}
                className="px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl transition-all font-medium"
              >
                Cancel
              </button>
              <button
                onClick={confirmTrash}
                className="px-4 py-2 bg-amber-500/90 hover:bg-amber-500 text-white rounded-xl transition-all font-medium flex items-center gap-2"
              >
                <Trash2 className="w-4 h-4" />
                Move to Trash
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl bg-white/5 border border-white/5 p-3">
      <p className="text-xs text-slate-500">{label}</p>
      <p className="text-lg font-bold text-white mt-0.5">{value}</p>
    </div>
  );
}
