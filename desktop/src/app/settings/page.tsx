'use client';

import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import {
  FolderPlus,
  Trash2,
  Check,
  X,
  AlertCircle,
  Folder,
  Archive,
  HardDrive,
  Shield,
  RefreshCw,
  FolderCheck,
  Ban,
} from 'lucide-react';
import { clsx } from 'clsx';
import {
  getConfig,
  addProjectDir,
  removeProjectDir,
  addExcludedDir,
  removeExcludedDir,
  setArchiveDir,
  clearArchiveDir,
  getArchiveStatus,
  getArchiveConfig,
  saveArchiveConfig,
  checkFullDiskAccess,
  openFullDiskAccessSettings,
  requestStandardFolders,
  isTauri
} from '@/lib/tauri';
import type { ConfigResponse, ProjectDir, ArchiveStatusResponse, ArchiveConfigResponse, FolderAccess } from '@/lib/types';
import { formatBytes } from '@/lib/types';
import { toMessage } from '@/lib/errors';
import { CliInstall } from '@/components/CliInstall';

export default function SettingsPage() {
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [newPath, setNewPath] = useState('');
  const [archivePath, setArchivePath] = useState('');
  const [archiveStatus, setArchiveStatus] = useState<ArchiveStatusResponse | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [isSettingArchive, setIsSettingArchive] = useState(false);
  const [archiveConfig, setArchiveConfig] = useState<ArchiveConfigResponse | null>(null);
  const [newExcludedPath, setNewExcludedPath] = useState('');
  const [isAddingExcluded, setIsAddingExcluded] = useState(false);
  const [fdaGranted, setFdaGranted] = useState<boolean | null>(null);
  const [isCheckingFda, setIsCheckingFda] = useState(false);
  const [folders, setFolders] = useState<FolderAccess[] | null>(null);
  const [isRequestingFolders, setIsRequestingFolders] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    const loadConfig = async () => {
      try {
        const [cfg, status, archCfg] = await Promise.all([
          getConfig(),
          getArchiveStatus(),
          getArchiveConfig(),
        ]);
        setConfig(cfg);
        setArchiveStatus(status);
        setArchiveConfig(archCfg);
      } catch (err) {
        setError(toMessage(err));
      }

      // Permissions: check Full Disk Access separately so a failure here doesn't
      // block the rest of the settings from loading.
      try {
        setFdaGranted(await checkFullDiskAccess());
      } catch (err) {
        setError(toMessage(err));
      }
    };

    loadConfig();
  }, []);

  const handleRecheckFda = async () => {
    setIsCheckingFda(true);
    setError(null);
    try {
      setFdaGranted(await checkFullDiskAccess());
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setIsCheckingFda(false);
    }
  };

  const handleOpenFdaSettings = async () => {
    setError(null);
    try {
      await openFullDiskAccessSettings();
    } catch (err) {
      setError(toMessage(err));
    }
  };

  const handleRequestFolders = async () => {
    setIsRequestingFolders(true);
    setError(null);
    try {
      setFolders(await requestStandardFolders());
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setIsRequestingFolders(false);
    }
  };

  const handleAddExcludedDir = async () => {
    if (!newExcludedPath.trim()) return;

    setIsAddingExcluded(true);
    setError(null);
    setSuccess(null);

    try {
      const cfg = await addExcludedDir(newExcludedPath.trim());
      setConfig(cfg);
      setNewExcludedPath('');
      setSuccess('Excluded folder added');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setIsAddingExcluded(false);
    }
  };

  const handleRemoveExcludedDir = async (path: string) => {
    setError(null);
    setSuccess(null);

    try {
      const cfg = await removeExcludedDir(path);
      setConfig(cfg);
      setSuccess('Excluded folder removed');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(toMessage(err));
    }
  };

  const handleToggleArchiveSetting = async (key: keyof ArchiveConfigResponse) => {
    if (!archiveConfig) return;
    const updated = { ...archiveConfig, [key]: !archiveConfig[key] };
    try {
      await saveArchiveConfig(updated);
      setArchiveConfig(updated);
      setSuccess('Setting updated');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(toMessage(err));
    }
  };

  const handleBatchSizeChange = async (value: number) => {
    if (!archiveConfig || value < 1 || value > 20) return;
    const updated = { ...archiveConfig, scan_batch_size: value };
    try {
      await saveArchiveConfig(updated);
      setArchiveConfig(updated);
    } catch (err) {
      setError(toMessage(err));
    }
  };

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
      setError(toMessage(err));
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
      setError(toMessage(err));
    }
  };

  const handleSetArchiveDir = async () => {
    if (!archivePath.trim()) return;

    setIsSettingArchive(true);
    setError(null);
    setSuccess(null);

    try {
      const cfg = await setArchiveDir(archivePath.trim());
      setConfig(cfg);
      setArchivePath('');
      const status = await getArchiveStatus();
      setArchiveStatus(status);
      setSuccess('Archive directory set');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setIsSettingArchive(false);
    }
  };

  const handleClearArchiveDir = async () => {
    setError(null);
    setSuccess(null);

    try {
      const cfg = await clearArchiveDir();
      setConfig(cfg);
      setArchiveStatus(null);
      setSuccess('Archive directory cleared');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(toMessage(err));
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

      {/* Permissions */}
      <section className="glass-card p-6 rounded-2xl">
        <h2 className="text-lg font-semibold text-white mb-2">
          Permissions
        </h2>
        <p className="text-sm text-slate-400 mb-6 max-w-2xl">
          MacPurge needs disk access to scan for reclaimable space. Granting Full Disk Access once
          avoids repeated macOS prompts.
        </p>

        {/* Full Disk Access */}
        <div className="flex items-center justify-between py-3 px-4 rounded-xl bg-white/5 border border-white/5 mb-4">
          <div className="flex items-center gap-3 min-w-0">
            <Shield className="w-5 h-5 text-brand-400 flex-shrink-0" />
            <div className="min-w-0">
              <p className="text-sm font-medium text-slate-200">Full Disk Access</p>
              <p className="text-xs text-slate-500 mt-0.5">
                {fdaGranted === null
                  ? 'Checking…'
                  : fdaGranted
                    ? 'Granted'
                    : 'Not granted — scans may miss protected folders'}
              </p>
            </div>
          </div>
          <span
            className={clsx(
              'px-3 py-1 rounded-full text-xs font-semibold border flex-shrink-0',
              fdaGranted === null
                ? 'bg-slate-500/10 text-slate-400 border-slate-500/20'
                : fdaGranted
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                  : 'bg-danger-500/10 text-danger-400 border-danger-500/20'
            )}
          >
            {fdaGranted === null ? 'Checking…' : fdaGranted ? '✅ Granted' : '❌ Not granted'}
          </span>
        </div>

        <div className="flex flex-wrap gap-3 mb-6">
          <button
            onClick={handleOpenFdaSettings}
            className="px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-medium transition-all shadow-lg shadow-brand-500/20"
          >
            <Shield className="w-4 h-4" />
            Grant Full Disk Access
          </button>
          <button
            onClick={handleRecheckFda}
            disabled={isCheckingFda}
            className={clsx(
              'px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl flex items-center gap-2 font-medium transition-all',
              isCheckingFda && 'opacity-50 cursor-not-allowed'
            )}
          >
            <RefreshCw className={clsx('w-4 h-4', isCheckingFda && 'animate-spin')} />
            Recheck
          </button>
          <button
            onClick={handleRequestFolders}
            disabled={isRequestingFolders}
            className={clsx(
              'px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl flex items-center gap-2 font-medium transition-all',
              isRequestingFolders && 'opacity-50 cursor-not-allowed'
            )}
          >
            <FolderCheck className="w-4 h-4" />
            Grant access to standard folders
          </button>
        </div>

        {folders && folders.length > 0 && (
          <div className="space-y-2">
            {folders.map((f) => (
              <div
                key={f.path}
                className="flex items-center justify-between py-2.5 px-4 rounded-xl bg-black/20 border border-white/5"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium text-slate-200">{f.name}</p>
                  <p className="text-[11px] text-slate-500 font-mono truncate" title={f.path}>
                    {f.path}
                  </p>
                </div>
                {f.accessible ? (
                  <span className="flex items-center gap-1 text-emerald-400 text-xs font-medium flex-shrink-0">
                    <Check className="w-4 h-4" /> Accessible
                  </span>
                ) : (
                  <span className="flex items-center gap-1 text-danger-400 text-xs font-medium flex-shrink-0">
                    <X className="w-4 h-4" /> Blocked
                  </span>
                )}
              </div>
            ))}
          </div>
        )}
      </section>

      <CliInstall />

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

      {/* Excluded Folders */}
      <section className="glass-card p-6 rounded-2xl">
        <h2 className="text-lg font-semibold text-white mb-2">
          Excluded Folders
        </h2>
        <p className="text-sm text-slate-400 mb-6 max-w-2xl">
          Folders here are never scanned by any feature.
        </p>

        {/* Add New Excluded Folder */}
        <div className="flex gap-3 mb-6">
          <input
            type="text"
            value={newExcludedPath}
            onChange={(e) => setNewExcludedPath(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleAddExcludedDir()}
            placeholder="~/Sensitive"
            className="flex-1 px-4 py-2.5 rounded-xl border border-white/10 bg-black/20 text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-brand-500/50 focus:border-brand-500 transition-all"
          />
          <button
            onClick={handleAddExcludedDir}
            disabled={isAddingExcluded || !newExcludedPath.trim()}
            className={clsx(
              'px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-medium transition-all shadow-lg shadow-brand-500/20',
              (isAddingExcluded || !newExcludedPath.trim()) && 'opacity-50 cursor-not-allowed grayscale'
            )}
          >
            <FolderPlus className="w-4 h-4" />
            Add Path
          </button>
        </div>

        {/* Excluded Folder List */}
        <div className="space-y-2">
          {config?.excluded_dirs.length === 0 && (
            <div className="p-8 border border-dashed border-white/10 rounded-xl text-center">
              <Ban className="w-8 h-8 text-slate-600 mx-auto mb-2" />
              <p className="text-slate-400 text-sm">No excluded folders</p>
            </div>
          )}
          {config?.excluded_dirs.map((dir: ProjectDir) => (
            <div
              key={dir.path}
              className="flex items-center justify-between py-3 px-4 rounded-xl bg-white/5 hover:bg-white/10 border border-transparent hover:border-white/5 transition-all group"
            >
              <div className="flex items-center gap-3 min-w-0">
                <Ban
                  className={clsx(
                    'w-5 h-5 flex-shrink-0 transition-colors',
                    dir.exists ? 'text-amber-400' : 'text-slate-500'
                  )}
                />
                <div className="min-w-0">
                  <p className="text-sm font-medium text-slate-200 truncate group-hover:text-white transition-colors">
                    {dir.path}
                  </p>
                  {!dir.exists && (
                    <p className="text-xs text-slate-500 mt-0.5">Directory not found on disk</p>
                  )}
                </div>
              </div>
              <button
                onClick={() => handleRemoveExcludedDir(dir.path)}
                className="p-2 text-slate-500 hover:text-danger-400 hover:bg-danger-500/10 rounded-lg transition-all opacity-0 group-hover:opacity-100"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          ))}
        </div>
      </section>

      {/* Archive Location */}
      <section className="glass-card p-6 rounded-2xl">
        <h2 className="text-lg font-semibold text-white mb-2">
          Archive Location
        </h2>
        <p className="text-sm text-slate-400 mb-6 max-w-2xl">
          Set the directory where archived projects will be stored. This is typically an external
          drive or cloud storage folder.
        </p>

        {config?.archive_dir ? (
          <div className="space-y-4">
            <div className="flex items-center justify-between py-3 px-4 rounded-xl bg-white/5 border border-white/5">
              <div className="flex items-center gap-3 min-w-0">
                <Archive className="w-5 h-5 text-brand-400 flex-shrink-0" />
                <div className="min-w-0">
                  <p className="text-sm font-medium text-slate-200 truncate">
                    {config.archive_dir}
                  </p>
                  {archiveStatus && (
                    <p className="text-xs text-slate-500 mt-0.5">
                      {archiveStatus.exists
                        ? `${archiveStatus.project_count} projects, ${formatBytes(archiveStatus.total_size)}`
                        : 'Directory not accessible (disk not connected?)'}
                    </p>
                  )}
                </div>
              </div>
              <button
                onClick={handleClearArchiveDir}
                className="p-2 text-slate-500 hover:text-danger-400 hover:bg-danger-500/10 rounded-lg transition-all"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex gap-3">
              <input
                type="text"
                value={archivePath}
                onChange={(e) => setArchivePath(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSetArchiveDir()}
                placeholder="/Volumes/External/archive"
                className="flex-1 px-4 py-2.5 rounded-xl border border-white/10 bg-black/20 text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-brand-500/50 focus:border-brand-500 transition-all"
              />
              <button
                onClick={handleSetArchiveDir}
                disabled={isSettingArchive || !archivePath.trim()}
                className={clsx(
                  'px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-medium transition-all shadow-lg shadow-brand-500/20',
                  (isSettingArchive || !archivePath.trim()) && 'opacity-50 cursor-not-allowed grayscale'
                )}
              >
                <HardDrive className="w-4 h-4" />
                Set Path
              </button>
            </div>
            <div className="p-4 border border-dashed border-white/10 rounded-xl text-center">
              <Archive className="w-6 h-6 text-slate-600 mx-auto mb-1" />
              <p className="text-slate-400 text-sm">No archive location configured</p>
            </div>
          </div>
        )}
      </section>

      {/* Archive Settings */}
      <section className="glass-card p-6 rounded-2xl">
        <h2 className="text-lg font-semibold text-white mb-2">
          Archive Settings
        </h2>
        <p className="text-sm text-slate-400 mb-6 max-w-2xl">
          Control how projects are scanned and archived.
        </p>

        <div className="space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium text-slate-200">
                Concurrent Scans
              </p>
              <p className="text-xs text-slate-500">
                Number of projects to calculate sizes for in parallel
              </p>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => handleBatchSizeChange((archiveConfig?.scan_batch_size ?? 3) - 1)}
                disabled={!archiveConfig || archiveConfig.scan_batch_size <= 1}
                className="px-2 py-1 rounded-lg bg-black/20 border border-white/10 text-slate-300 hover:text-white hover:border-white/20 disabled:opacity-30 transition-all"
              >
                -
              </button>
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-black/20 border border-white/5 min-w-[60px] justify-center">
                <span className="text-slate-300 font-mono text-sm">
                  {archiveConfig?.scan_batch_size ?? 3}
                </span>
              </div>
              <button
                onClick={() => handleBatchSizeChange((archiveConfig?.scan_batch_size ?? 3) + 1)}
                disabled={!archiveConfig || archiveConfig.scan_batch_size >= 20}
                className="px-2 py-1 rounded-lg bg-black/20 border border-white/10 text-slate-300 hover:text-white hover:border-white/20 disabled:opacity-30 transition-all"
              >
                +
              </button>
            </div>
          </div>

          <div className="flex items-center justify-between pt-4 border-t border-white/5">
            <div>
              <p className="font-medium text-slate-200">
                Clean Artifacts Before Archive
              </p>
              <p className="text-xs text-slate-500">
                Remove node_modules, target, venv, etc. before archiving
              </p>
            </div>
            <button
              onClick={() => handleToggleArchiveSetting('clean_artifacts_before_archive')}
              className={clsx(
                'px-3 py-1 rounded-full text-xs font-semibold border cursor-pointer transition-all',
                archiveConfig?.clean_artifacts_before_archive
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20 hover:bg-emerald-500/20'
                  : 'bg-slate-500/10 text-slate-400 border-slate-500/20 hover:bg-slate-500/20'
              )}
            >
              {archiveConfig?.clean_artifacts_before_archive ? 'Enabled' : 'Disabled'}
            </button>
          </div>

          <div className="flex items-center justify-between pt-4 border-t border-white/5">
            <div>
              <p className="font-medium text-slate-200">
                Compress to ZIP
              </p>
              <p className="text-xs text-slate-500">
                Archive projects as .zip files instead of copying directories
              </p>
            </div>
            <button
              onClick={() => handleToggleArchiveSetting('compress_to_zip')}
              className={clsx(
                'px-3 py-1 rounded-full text-xs font-semibold border cursor-pointer transition-all',
                archiveConfig?.compress_to_zip
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20 hover:bg-emerald-500/20'
                  : 'bg-slate-500/10 text-slate-400 border-slate-500/20 hover:bg-slate-500/20'
              )}
            >
              {archiveConfig?.compress_to_zip ? 'Enabled' : 'Disabled'}
            </button>
          </div>
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
