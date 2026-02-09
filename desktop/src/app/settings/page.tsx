'use client';

import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import {
  FolderPlus,
  Trash2,
  Check,
  X,
  AlertCircle,
  Folder
} from 'lucide-react';
import { clsx } from 'clsx';
import {
  getConfig,
  addProjectDir,
  removeProjectDir,
  isTauri
} from '@/lib/tauri';
import type { ConfigResponse, ProjectDir } from '@/lib/types';

export default function SettingsPage() {
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [newPath, setNewPath] = useState('');
  const [isAdding, setIsAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    const loadConfig = async () => {
      try {
        const cfg = await getConfig();
        setConfig(cfg);
      } catch (err) {
        setError(err as string);
      }
    };

    loadConfig();
  }, []);

  const handleAddDir = async () => {
    if (!newPath.trim()) return;

    setIsAdding(true);
    setError(null);
    setSuccess(null);

    try {
      const cfg = await addProjectDir(newPath.trim());
      setConfig(cfg);
      setNewPath('');
      setSuccess('Directory added successfully');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err as string);
    } finally {
      setIsAdding(false);
    }
  };

  const handleRemoveDir = async (path: string) => {
    setError(null);
    setSuccess(null);

    try {
      const cfg = await removeProjectDir(path);
      setConfig(cfg);
      setSuccess('Directory removed');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err as string);
    }
  };

  if (!isTauri()) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-slate-400">Run in Tauri to access settings</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-4xl mx-auto">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold text-white tracking-tight">
          Settings
        </h1>
        <p className="text-slate-400 mt-1">
          Configure cleaning behavior and locations
        </p>
      </div>

      {/* Success Message */}
      {success && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-4 rounded-xl border border-emerald-500/30 bg-emerald-500/10"
        >
          <div className="flex items-center gap-2">
            <Check className="w-4 h-4 text-emerald-400" />
            <span className="text-emerald-200">{success}</span>
          </div>
        </motion.div>
      )}

      {/* Error Message */}
      {error && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-4 rounded-xl border border-danger-500/30 bg-danger-500/10"
        >
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-danger-400" />
            <span className="text-danger-200">{error}</span>
            <button onClick={() => setError(null)} className="ml-auto hover:text-white">
              <X className="w-4 h-4" />
            </button>
          </div>
        </motion.div>
      )}

      {/* Project Directories */}
      <section className="glass-card p-6 rounded-2xl">
        <h2 className="text-lg font-semibold text-white mb-2">
          Project Directories
        </h2>
        <p className="text-sm text-slate-400 mb-6 max-w-2xl">
          Add the root directories where you keep your code. MacPurge will scan them recursively
          for build artifacts to clean.
        </p>

        {/* Add New Directory */}
        <div className="flex gap-3 mb-6">
          <input
            type="text"
            value={newPath}
            onChange={(e) => setNewPath(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleAddDir()}
            placeholder="~/projects"
            className="flex-1 px-4 py-2.5 rounded-xl border border-white/10 bg-black/20 text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-brand-500/50 focus:border-brand-500 transition-all"
          />
          <button
            onClick={handleAddDir}
            disabled={isAdding || !newPath.trim()}
            className={clsx(
              'px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-medium transition-all shadow-lg shadow-brand-500/20',
              (isAdding || !newPath.trim()) && 'opacity-50 cursor-not-allowed grayscale'
            )}
          >
            <FolderPlus className="w-4 h-4" />
            Add Path
          </button>
        </div>

        {/* Directory List */}
        <div className="space-y-2">
          {config?.project_dirs.length === 0 && (
            <div className="p-8 border border-dashed border-white/10 rounded-xl text-center">
              <Folder className="w-8 h-8 text-slate-600 mx-auto mb-2" />
              <p className="text-slate-400 text-sm">No directories configured yet</p>
            </div>
          )}
          {config?.project_dirs.map((dir: ProjectDir) => (
            <div
              key={dir.path}
              className="flex items-center justify-between py-3 px-4 rounded-xl bg-white/5 hover:bg-white/10 border border-transparent hover:border-white/5 transition-all group"
            >
              <div className="flex items-center gap-3 min-w-0">
                <Folder
                  className={clsx(
                    'w-5 h-5 flex-shrink-0 transition-colors',
                    dir.exists ? 'text-brand-400' : 'text-danger-400'
                  )}
                />
                <div className="min-w-0">
                  <p className="text-sm font-medium text-slate-200 truncate group-hover:text-white transition-colors">
                    {dir.path}
                  </p>
                  {!dir.exists && (
                    <p className="text-xs text-danger-400 mt-0.5">Directory not found on disk</p>
                  )}
                </div>
              </div>
              <button
                onClick={() => handleRemoveDir(dir.path)}
                className="p-2 text-slate-500 hover:text-danger-400 hover:bg-danger-500/10 rounded-lg transition-all opacity-0 group-hover:opacity-100"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          ))}
        </div>
      </section>

      {/* Settings Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Scan Settings */}
        <section className="glass-card p-6 rounded-2xl">
          <h2 className="text-lg font-semibold text-white mb-4">
            Scan Configuration
          </h2>

          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-slate-200">
                  Max Depth
                </p>
                <p className="text-xs text-slate-500">
                  Recursion limit for folder scanning
                </p>
              </div>
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-black/20 border border-white/5">
                <span className="text-slate-300 font-mono text-sm">
                  {config?.scan.max_depth || 10}
                </span>
                <span className="text-slate-600 text-xs">levels</span>
              </div>
            </div>

            <div className="flex items-center justify-between pt-4 border-t border-white/5">
              <div>
                <p className="font-medium text-slate-200">
                  Include Hidden
                </p>
                <p className="text-xs text-slate-500">
                  Scan directories starting with dot
                </p>
              </div>
              <div
                className={clsx(
                  'px-3 py-1 rounded-full text-xs font-semibold border',
                  config?.scan.include_hidden
                    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                    : 'bg-slate-500/10 text-slate-400 border-slate-500/20'
                )}
              >
                {config?.scan.include_hidden ? 'Enabled' : 'Disabled'}
              </div>
            </div>
          </div>
        </section>

        {/* Cleanup Settings */}
        <section className="glass-card p-6 rounded-2xl">
          <h2 className="text-lg font-semibold text-white mb-4">
            Safety & Cleanup
          </h2>

          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-slate-200">
                  Use Trash
                </p>
                <p className="text-xs text-slate-500">
                  Move to Trash instead of delete
                </p>
              </div>
              <div
                className={clsx(
                  'px-3 py-1 rounded-full text-xs font-semibold border',
                  config?.cleanup.use_trash
                    ? 'bg-blue-500/10 text-blue-400 border-blue-500/20'
                    : 'bg-danger-500/10 text-danger-400 border-danger-500/20'
                )}
              >
                {config?.cleanup.use_trash ? 'Trash' : 'Perm Delete'}
              </div>
            </div>

            <div className="flex items-center justify-between pt-4 border-t border-white/5">
              <div>
                <p className="font-medium text-slate-200">
                  Confirm Deletion
                </p>
                <p className="text-xs text-slate-500">
                  Ask before performing actions
                </p>
              </div>
              <div
                className={clsx(
                  'px-3 py-1 rounded-full text-xs font-semibold border',
                  config?.cleanup.confirm_before_delete
                    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                    : 'bg-slate-500/10 text-slate-400 border-slate-500/20'
                )}
              >
                {config?.cleanup.confirm_before_delete ? 'Required' : 'Skipped'}
              </div>
            </div>
          </div>
        </section>
      </div>

      {/* Config Path Footer */}
      {config && (
        <div className="text-center pt-8 pb-4">
          <p className="text-[10px] text-slate-600 font-mono tracking-tight">
            Loaded from: {config.config_path}
          </p>
        </div>
      )}
    </div>
  );
}
