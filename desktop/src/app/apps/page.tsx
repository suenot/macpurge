'use client';

import { useEffect, useState, useRef } from 'react';
import { AppWindow, Search, Trash2, RefreshCw, Database } from 'lucide-react';
import { clsx } from 'clsx';
import { listApps, getAppSizes, trashApp, isTauri } from '@/lib/tauri';
import type { AppInfo } from '@/lib/types';
import { formatBytes } from '@/lib/types';
import { motion } from 'framer-motion';

export default function AppsPage() {
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [search, setSearch] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loadingSizes, setLoadingSizes] = useState(false);
  const [trashing, setTrashing] = useState<string | null>(null);
  const [confirmApp, setConfirmApp] = useState<AppInfo | null>(null);
  const abortRef = useRef(false);

  const loadApps = async () => {
    if (!isTauri()) return;

    setIsLoading(true);
    setError(null);
    abortRef.current = true; // cancel any pending size loading

    try {
      const appList = await listApps();
      setApps(appList.sort((a, b) => b.total_size_bytes - a.total_size_bytes));

      // Phase 2: lazily load data/cache sizes in background
      abortRef.current = false;
      loadAppSizes(appList);
    } catch (err) {
      setError(err as string);
    } finally {
      setIsLoading(false);
    }
  };

  const loadAppSizes = async (appList: AppInfo[]) => {
    if (!isTauri()) return;
    setLoadingSizes(true);

    // Load sizes for each app sequentially to avoid overloading
    const updated = [...appList];
    for (let i = 0; i < updated.length; i++) {
      if (abortRef.current) break;

      const app = updated[i];
      try {
        const sizes = await getAppSizes(app.name, app.bundle_id);
        updated[i] = {
          ...app,
          data_size_bytes: sizes.data_size_bytes,
          cache_size_bytes: sizes.cache_size_bytes,
          total_size_bytes: app.size_bytes + sizes.data_size_bytes + sizes.cache_size_bytes,
        };

        // Update state every 5 apps for responsiveness
        if (i % 5 === 0 || i === updated.length - 1) {
          const sorted = [...updated].sort((a, b) => b.total_size_bytes - a.total_size_bytes);
          setApps(sorted);
        }
      } catch {
        // Skip on error
      }
    }

    if (!abortRef.current) {
      const sorted = [...updated].sort((a, b) => b.total_size_bytes - a.total_size_bytes);
      setApps(sorted);
    }
    setLoadingSizes(false);
  };

  // Native window.confirm() is a no-op inside the Tauri WKWebview, so we use an
  // in-app modal instead. Clicking the trash icon opens the modal; the modal's
  // confirm button runs the actual delete.
  const confirmTrash = async () => {
    const app = confirmApp;
    if (!app || !isTauri()) return;

    setConfirmApp(null);
    setTrashing(app.path);
    setError(null);
    try {
      await trashApp(app.path);
      setApps((prev) => prev.filter((a) => a.path !== app.path));
    } catch (err) {
      setError(`Failed to trash ${app.name}: ${err}`);
    } finally {
      setTrashing(null);
    }
  };

  useEffect(() => {
    loadApps();
    return () => { abortRef.current = true; };
  }, []);

  const filteredApps = apps.filter((app) =>
    app.name.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="space-y-6 flex flex-col h-[calc(100vh-6rem)]">
      {/* Header */}
      <div className="flex-shrink-0">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-2xl font-bold text-white tracking-tight">
              Applications
            </h1>
            <p className="text-slate-400 mt-1">
              View and manage installed applications
            </p>
          </div>
          <div className="flex items-center gap-3">
            {loadingSizes && (
              <span className="text-xs text-slate-500 flex items-center gap-1.5">
                <Database className="w-3 h-3 animate-pulse" />
                Loading sizes...
              </span>
            )}
            <button
              onClick={loadApps}
              disabled={isLoading}
              className="px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl flex items-center gap-2 transition-all font-medium disabled:opacity-50"
            >
              <RefreshCw className={clsx('w-4 h-4', isLoading && 'animate-spin')} />
              Refresh
            </button>
          </div>
        </div>

        {/* Search */}
        <div className="relative mb-2">
          <Search className="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-slate-500" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search installed applications..."
            className="w-full pl-10 pr-4 py-3 rounded-xl border border-white/10 bg-black/20 text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-brand-500/50 focus:bg-black/30 transition-all"
          />
        </div>

        {error && (
          <div className="mb-2 px-4 py-2 rounded-xl border border-danger-500/30 bg-danger-500/10 text-sm text-danger-300">
            {error}
          </div>
        )}
      </div>

      {/* Apps List */}
      <div className="glass-card rounded-2xl flex-1 overflow-hidden flex flex-col min-h-0 border border-white/5">
        {isLoading ? (
          <div className="flex-1 flex flex-col items-center justify-center p-12 text-center text-slate-500">
            <RefreshCw className="w-8 h-8 animate-spin mb-4 text-brand-500" />
            <p>Scanning applications...</p>
          </div>
        ) : filteredApps.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center p-12 text-center text-slate-500">
            <AppWindow className="w-12 h-12 mb-4 opacity-50" />
            <p className="text-lg font-medium text-slate-400">
              {search ? 'No matches found' : 'No applications found'}
            </p>
          </div>
        ) : (
          <div className="flex-1 overflow-y-auto custom-scrollbar p-2 space-y-1">
            {filteredApps.map((app, index) => (
              <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.02 }}
                key={app.path}
                className="flex items-center gap-4 p-3 rounded-xl hover:bg-white/5 transition-colors border border-transparent hover:border-white/5 group"
              >
                <div className="w-10 h-10 rounded-lg bg-black/20 border border-white/5 flex items-center justify-center flex-shrink-0 group-hover:scale-105 transition-transform">
                  <AppWindow className="w-5 h-5 text-slate-400 group-hover:text-brand-400 transition-colors" />
                </div>

                <div className="flex-1 min-w-0">
                  <p className="font-medium text-slate-200 group-hover:text-white truncate transition-colors">
                    {app.name}
                  </p>
                  <p className="text-xs text-slate-500 truncate font-mono mt-0.5">
                    {app.path}
                  </p>
                </div>

                <div className="text-right flex-shrink-0">
                  <p className="font-medium text-slate-300 group-hover:text-white transition-colors font-mono">
                    {formatBytes(app.total_size_bytes)}
                  </p>
                  {(app.data_size_bytes > 0 || app.cache_size_bytes > 0) && (
                    <div className="flex items-center gap-2 text-[10px] text-slate-500">
                      {app.size_bytes > 0 && (
                        <span>app {formatBytes(app.size_bytes)}</span>
                      )}
                      {app.data_size_bytes > 0 && (
                        <span className="text-blue-400">data {formatBytes(app.data_size_bytes)}</span>
                      )}
                      {app.cache_size_bytes > 0 && (
                        <span className="text-yellow-400">cache {formatBytes(app.cache_size_bytes)}</span>
                      )}
                    </div>
                  )}
                </div>

                <button
                  onClick={() => setConfirmApp(app)}
                  disabled={trashing !== null}
                  title={`Move ${app.name} to Trash`}
                  className="p-2 text-slate-600 hover:text-danger-400 hover:bg-danger-500/10 rounded-lg transition-all opacity-0 group-hover:opacity-100 focus:opacity-100 disabled:opacity-50"
                >
                  {trashing === app.path ? (
                    <RefreshCw className="w-4 h-4 animate-spin" />
                  ) : (
                    <Trash2 className="w-4 h-4" />
                  )}
                </button>
              </motion.div>
            ))}
          </div>
        )}
      </div>

      {/* Summary */}
      {apps.length > 0 && (
        <div className="flex-shrink-0 text-center">
          <p className="text-xs text-slate-500 font-mono">
            {filteredApps.length} apps found • Using {formatBytes(filteredApps.reduce((acc, a) => acc + a.total_size_bytes, 0))}
          </p>
        </div>
      )}

      {/* Trash confirmation modal */}
      {confirmApp && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
          onClick={() => setConfirmApp(null)}
        >
          <div
            className="glass-card w-full max-w-md mx-4 rounded-2xl border border-white/10 p-6"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-lg bg-danger-500/10 border border-danger-500/20 flex items-center justify-center flex-shrink-0">
                <Trash2 className="w-5 h-5 text-danger-400" />
              </div>
              <h2 className="text-lg font-bold text-white">Move to Trash?</h2>
            </div>
            <p className="text-sm text-slate-300">
              Move <span className="font-semibold text-white">{confirmApp.name}</span> to the Trash?
            </p>
            <p className="text-xs text-slate-500 font-mono mt-1 break-all">{confirmApp.path}</p>
            <p className="text-xs text-slate-400 mt-3">
              The app bundle goes to the Trash and can be restored from there. Its data
              and caches are left untouched.
            </p>
            <div className="flex items-center justify-end gap-3 mt-6">
              <button
                onClick={() => setConfirmApp(null)}
                className="px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl transition-all font-medium"
              >
                Cancel
              </button>
              <button
                onClick={confirmTrash}
                className="px-4 py-2 bg-danger-500/90 hover:bg-danger-500 text-white rounded-xl transition-all font-medium flex items-center gap-2"
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
