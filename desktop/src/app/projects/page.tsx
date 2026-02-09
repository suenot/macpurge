'use client';

import { useEffect, useState, useMemo, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  FolderGit2,
  RefreshCw,
  Archive,
  ArchiveRestore,
  ArrowUpDown,
  Filter,
  Loader2,
  GitBranch,
  AlertCircle,
  Check,
  X,
  CheckSquare,
  Square,
  Clock,
  HardDrive,
  Settings,
  ArrowUp,
  GitCommitHorizontal,
  Upload,
} from 'lucide-react';
import { clsx } from 'clsx';
import Link from 'next/link';
import {
  listProjects,
  calculateProjectSize,
  archiveProjects,
  listArchivedProjects,
  restoreProjects,
  getArchiveStatus,
  onProjectSizeCalculated,
  onArchiveProgress,
  checkGitStatus,
  autoCommitProject,
  pushProjectAll,
  onProjectGitStatus,
  isTauri,
} from '@/lib/tauri';
import type {
  ProjectInfo,
  ProjectType,
  ArchivedProjectEntry,
  ArchiveProgressPayload,
  ArchiveStatusResponse,
  GitStatus,
} from '@/lib/types';
import { formatBytes } from '@/lib/types';

type SortField = 'last_modified' | 'size' | 'name';
type Tab = 'active' | 'archived';

const projectTypeConfig: Record<ProjectType, { label: string; color: string }> = {
  Node: { label: 'Node.js', color: 'text-green-400 bg-green-500/10 border-green-500/20' },
  Rust: { label: 'Rust', color: 'text-orange-400 bg-orange-500/10 border-orange-500/20' },
  Go: { label: 'Go', color: 'text-cyan-400 bg-cyan-500/10 border-cyan-500/20' },
  Python: { label: 'Python', color: 'text-yellow-400 bg-yellow-500/10 border-yellow-500/20' },
  Java: { label: 'Java', color: 'text-red-400 bg-red-500/10 border-red-500/20' },
  DotNet: { label: '.NET', color: 'text-purple-400 bg-purple-500/10 border-purple-500/20' },
  Ruby: { label: 'Ruby', color: 'text-rose-400 bg-rose-500/10 border-rose-500/20' },
  Php: { label: 'PHP', color: 'text-indigo-400 bg-indigo-500/10 border-indigo-500/20' },
  Elixir: { label: 'Elixir', color: 'text-violet-400 bg-violet-500/10 border-violet-500/20' },
  Haskell: { label: 'Haskell', color: 'text-fuchsia-400 bg-fuchsia-500/10 border-fuchsia-500/20' },
  Swift: { label: 'Swift', color: 'text-orange-300 bg-orange-400/10 border-orange-400/20' },
  Dart: { label: 'Dart', color: 'text-blue-400 bg-blue-500/10 border-blue-500/20' },
  Unknown: { label: 'Other', color: 'text-slate-400 bg-slate-500/10 border-slate-500/20' },
};

function relativeTime(timestamp: number | null): string {
  if (!timestamp) return 'Unknown';
  const now = Date.now() / 1000;
  const diff = now - timestamp;

  if (diff < 60) return 'just now';
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 2592000) return `${Math.floor(diff / 86400)}d ago`;
  if (diff < 31536000) return `${Math.floor(diff / 2592000)}mo ago`;
  return `${Math.floor(diff / 31536000)}y ago`;
}

export default function ProjectsPage() {
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [archivedProjects, setArchivedProjects] = useState<ArchivedProjectEntry[]>([]);
  const [archiveStatus, setArchiveStatus] = useState<ArchiveStatusResponse | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [sortBy, setSortBy] = useState<SortField>('last_modified');
  const [sortAsc, setSortAsc] = useState(false);
  const [filterType, setFilterType] = useState<ProjectType | 'all'>('all');
  const [tab, setTab] = useState<Tab>('active');
  const [isScanning, setIsScanning] = useState(false);
  const [isArchiving, setIsArchiving] = useState(false);
  const [archiveProgress, setArchiveProgress] = useState<Map<string, ArchiveProgressPayload>>(new Map());
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [sizesLoaded, setSizesLoaded] = useState(false);
  const [gitStatuses, setGitStatuses] = useState<Map<string, GitStatus>>(new Map());
  const [gitStatusesLoaded, setGitStatusesLoaded] = useState(false);
  const [actionLoading, setActionLoading] = useState<Map<string, string>>(new Map());

  // Load projects
  const loadProjects = useCallback(async () => {
    setIsScanning(true);
    setError(null);
    setSizesLoaded(false);
    setGitStatusesLoaded(false);
    setGitStatuses(new Map());

    try {
      const [projectList, archived, status] = await Promise.all([
        listProjects(),
        listArchivedProjects(),
        getArchiveStatus(),
      ]);
      setProjects(projectList);
      setArchivedProjects(archived);
      setArchiveStatus(status);
    } catch (err) {
      setError(err as string);
    } finally {
      setIsScanning(false);
    }
  }, []);

  // Initial load
  useEffect(() => {
    if (!isTauri()) return;
    loadProjects();
  }, [loadProjects]);

  // Background size scanning
  useEffect(() => {
    if (projects.length === 0 || sizesLoaded) return;

    let unlisten: (() => void) | undefined;
    let mounted = true;

    const setup = async () => {
      unlisten = await onProjectSizeCalculated((payload) => {
        if (!mounted) return;
        setProjects(prev => prev.map(p =>
          p.path === payload.path
            ? {
                ...p,
                size_bytes: payload.size_bytes,
                artifact_size: payload.artifact_size,
                last_commit_time: payload.last_commit_time ?? p.last_commit_time,
              }
            : p
        ));
      });

      // Fire off size calculations
      for (const project of projects) {
        if (project.size_bytes === null) {
          calculateProjectSize(project.path).catch(console.error);
        }
      }
      setSizesLoaded(true);
    };

    setup();
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [projects.length, sizesLoaded]);

  // Background git status scanning
  useEffect(() => {
    const gitProjects = projects.filter(p => p.has_git);
    if (gitProjects.length === 0 || gitStatusesLoaded) return;

    let unlisten: (() => void) | undefined;
    let mounted = true;

    const setup = async () => {
      unlisten = await onProjectGitStatus((payload) => {
        if (!mounted) return;
        setGitStatuses(prev => {
          const next = new Map(prev);
          next.set(payload.path, payload.status);
          return next;
        });
      });

      for (const project of gitProjects) {
        checkGitStatus(project.path).catch(console.error);
      }
      setGitStatusesLoaded(true);
    };

    setup();
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [projects.length, gitStatusesLoaded]);

  // Archive progress events
  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;
    onArchiveProgress((payload) => {
      setArchiveProgress(prev => {
        const next = new Map(prev);
        next.set(payload.project_name, payload);
        return next;
      });

      if (payload.status === 'completed' || payload.status === 'error') {
        if (payload.status === 'completed') {
          setSuccess(`${payload.project_name} archived successfully`);
          setTimeout(() => setSuccess(null), 3000);
        }
      }
    }).then(fn => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, []);

  // Sorting
  const sortedProjects = useMemo(() => {
    const filtered = filterType === 'all'
      ? projects
      : projects.filter(p => p.project_type === filterType);

    return [...filtered].sort((a, b) => {
      const dir = sortAsc ? 1 : -1;
      switch (sortBy) {
        case 'last_modified': {
          const aTime = a.last_commit_time ?? a.last_modified ?? 0;
          const bTime = b.last_commit_time ?? b.last_modified ?? 0;
          return (bTime - aTime) * dir;
        }
        case 'size':
          return ((b.size_bytes ?? 0) - (a.size_bytes ?? 0)) * dir;
        case 'name':
          return a.name.localeCompare(b.name) * dir;
      }
    });
  }, [projects, sortBy, sortAsc, filterType]);

  // Selection helpers
  const toggleSelect = (path: string) => {
    setSelectedPaths(prev => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const selectAll = () => {
    if (tab === 'active') {
      setSelectedPaths(new Set(sortedProjects.map(p => p.path)));
    } else {
      setSelectedPaths(new Set(archivedProjects.map(p => p.archived_path)));
    }
  };

  const deselectAll = () => setSelectedPaths(new Set());

  const selectedSize = useMemo(() => {
    if (tab === 'active') {
      return projects
        .filter(p => selectedPaths.has(p.path))
        .reduce((sum, p) => sum + (p.size_bytes ?? 0), 0);
    } else {
      return archivedProjects
        .filter(p => selectedPaths.has(p.archived_path))
        .reduce((sum, p) => sum + p.size_bytes, 0);
    }
  }, [selectedPaths, projects, archivedProjects, tab]);

  // Archive handler
  const handleArchive = async () => {
    if (selectedPaths.size === 0) return;
    setIsArchiving(true);
    setError(null);

    try {
      await archiveProjects(Array.from(selectedPaths));
      setSelectedPaths(new Set());
      await loadProjects();
    } catch (err) {
      setError(err as string);
    } finally {
      setIsArchiving(false);
      setArchiveProgress(new Map());
    }
  };

  // Restore handler
  const handleRestore = async () => {
    if (selectedPaths.size === 0) return;
    setIsArchiving(true);
    setError(null);

    try {
      await restoreProjects(Array.from(selectedPaths));
      setSelectedPaths(new Set());
      await loadProjects();
    } catch (err) {
      setError(err as string);
    } finally {
      setIsArchiving(false);
      setArchiveProgress(new Map());
    }
  };

  // Git actions
  const handleAutoCommit = async (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setActionLoading(prev => new Map(prev).set(path, 'commit'));
    setError(null);

    try {
      const hash = await autoCommitProject(path);
      setSuccess(`Committed: ${hash}`);
      setTimeout(() => setSuccess(null), 3000);
      // Refresh git status for this project
      checkGitStatus(path).catch(console.error);
    } catch (err) {
      setError(err as string);
    } finally {
      setActionLoading(prev => {
        const next = new Map(prev);
        next.delete(path);
        return next;
      });
    }
  };

  const handlePushAll = async (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setActionLoading(prev => new Map(prev).set(path, 'push'));
    setError(null);

    try {
      const results = await pushProjectAll(path);
      const failed = results.filter(r => !r.success);
      if (failed.length > 0) {
        setError(`Push failed for: ${failed.map(r => `${r.remote} (${r.error})`).join(', ')}`);
      } else {
        setSuccess(`Pushed to ${results.map(r => r.remote).join(', ')}`);
        setTimeout(() => setSuccess(null), 3000);
      }
      checkGitStatus(path).catch(console.error);
    } catch (err) {
      setError(err as string);
    } finally {
      setActionLoading(prev => {
        const next = new Map(prev);
        next.delete(path);
        return next;
      });
    }
  };

  // Available project types for filter
  const availableTypes = useMemo(() => {
    const types = new Set(projects.map(p => p.project_type));
    return Array.from(types).sort();
  }, [projects]);

  const handleSort = (field: SortField) => {
    if (sortBy === field) {
      setSortAsc(!sortAsc);
    } else {
      setSortBy(field);
      setSortAsc(false);
    }
  };

  if (!isTauri()) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-slate-400">Run in Tauri to access projects</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold text-white tracking-tight">Projects</h1>
          <p className="text-slate-400 mt-1">
            Manage your projects across active and archive locations
          </p>
        </div>
        <button
          onClick={loadProjects}
          disabled={isScanning}
          className={clsx(
            'px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-medium transition-all shadow-lg shadow-brand-500/20',
            isScanning && 'opacity-50 cursor-not-allowed'
          )}
        >
          <RefreshCw className={clsx('w-4 h-4', isScanning && 'animate-spin')} />
          Rescan
        </button>
      </div>

      {/* Notifications */}
      <AnimatePresence>
        {success && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className="mb-4 p-4 rounded-xl border border-emerald-500/30 bg-emerald-500/10"
          >
            <div className="flex items-center gap-2">
              <Check className="w-4 h-4 text-emerald-400" />
              <span className="text-emerald-200">{success}</span>
            </div>
          </motion.div>
        )}
        {error && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className="mb-4 p-4 rounded-xl border border-danger-500/30 bg-danger-500/10"
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
      </AnimatePresence>

      {/* Tabs + Sort/Filter */}
      <div className="flex items-center justify-between mb-4 gap-4 flex-wrap">
        {/* Tabs */}
        <div className="flex gap-1 p-1 rounded-xl bg-white/5 border border-white/5">
          <button
            onClick={() => { setTab('active'); setSelectedPaths(new Set()); }}
            className={clsx(
              'px-4 py-2 rounded-lg text-sm font-medium transition-all',
              tab === 'active'
                ? 'bg-brand-500/20 text-brand-300 shadow-sm'
                : 'text-slate-400 hover:text-white hover:bg-white/5'
            )}
          >
            Active ({projects.length})
          </button>
          <button
            onClick={() => { setTab('archived'); setSelectedPaths(new Set()); }}
            className={clsx(
              'px-4 py-2 rounded-lg text-sm font-medium transition-all',
              tab === 'archived'
                ? 'bg-brand-500/20 text-brand-300 shadow-sm'
                : 'text-slate-400 hover:text-white hover:bg-white/5'
            )}
          >
            Archived ({archivedProjects.length})
          </button>
        </div>

        {/* Sort & Filter (active tab only) */}
        {tab === 'active' && (
          <div className="flex items-center gap-2">
            {/* Sort buttons */}
            {(['last_modified', 'size', 'name'] as SortField[]).map(field => (
              <button
                key={field}
                onClick={() => handleSort(field)}
                className={clsx(
                  'px-3 py-1.5 rounded-lg text-xs font-medium transition-all flex items-center gap-1',
                  sortBy === field
                    ? 'bg-brand-500/20 text-brand-300 border border-brand-500/30'
                    : 'text-slate-400 hover:text-white bg-white/5 border border-white/5'
                )}
              >
                {field === 'last_modified' && <Clock className="w-3 h-3" />}
                {field === 'size' && <HardDrive className="w-3 h-3" />}
                {field === 'name' && <ArrowUpDown className="w-3 h-3" />}
                {field === 'last_modified' ? 'Date' : field === 'size' ? 'Size' : 'Name'}
                {sortBy === field && (
                  <span className="text-[10px]">{sortAsc ? '\u2191' : '\u2193'}</span>
                )}
              </button>
            ))}

            {/* Type filter */}
            <div className="relative ml-2">
              <select
                value={filterType}
                onChange={(e) => setFilterType(e.target.value as ProjectType | 'all')}
                className="appearance-none px-3 py-1.5 rounded-lg text-xs font-medium bg-white/5 border border-white/5 text-slate-300 cursor-pointer pr-6"
              >
                <option value="all">All types</option>
                {availableTypes.map(t => (
                  <option key={t} value={t}>{projectTypeConfig[t].label}</option>
                ))}
              </select>
              <Filter className="w-3 h-3 text-slate-500 absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none" />
            </div>
          </div>
        )}
      </div>

      {/* Archive progress */}
      <AnimatePresence>
        {isArchiving && archiveProgress.size > 0 && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="mb-4 glass-card p-4 rounded-xl"
          >
            <p className="text-sm text-slate-300 mb-2">
              {tab === 'active' ? 'Archiving' : 'Restoring'} projects...
            </p>
            {Array.from(archiveProgress.entries()).map(([name, prog]) => (
              <div key={name} className="flex items-center gap-3 text-sm py-1">
                <span className="text-slate-400 truncate flex-1">{name}</span>
                <span className={clsx(
                  'text-xs px-2 py-0.5 rounded-full',
                  prog.status === 'completed' ? 'bg-emerald-500/10 text-emerald-400' :
                  prog.status === 'error' ? 'bg-danger-500/10 text-danger-400' :
                  'bg-brand-500/10 text-brand-300'
                )}>
                  {prog.status === 'copying' ? 'Copying...' :
                   prog.status === 'deleting_source' ? 'Cleaning up...' :
                   prog.status === 'completed' ? 'Done' :
                   prog.error ?? 'Error'}
                </span>
              </div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>

      {/* Project list */}
      <div className="flex-1 overflow-y-auto custom-scrollbar space-y-1">
        {isScanning ? (
          <div className="flex flex-col items-center justify-center py-20">
            <Loader2 className="w-8 h-8 text-brand-400 animate-spin mb-4" />
            <p className="text-slate-400">Scanning project directories...</p>
          </div>
        ) : tab === 'active' ? (
          sortedProjects.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-20">
              <FolderGit2 className="w-12 h-12 text-slate-600 mb-4" />
              <p className="text-slate-400 mb-2">No projects found</p>
              <Link
                href="/settings"
                className="text-brand-400 hover:text-brand-300 text-sm flex items-center gap-1"
              >
                <Settings className="w-3 h-3" />
                Configure project directories
              </Link>
            </div>
          ) : (
            sortedProjects.map((project) => {
              const typeConf = projectTypeConfig[project.project_type];
              const isSelected = selectedPaths.has(project.path);
              const displayTime = project.last_commit_time ?? project.last_modified;
              const gitStatus = gitStatuses.get(project.path);
              const isCommitting = actionLoading.get(project.path) === 'commit';
              const isPushing = actionLoading.get(project.path) === 'push';
              const totalAhead = gitStatus?.remotes.reduce((s, r) => s + r.ahead, 0) ?? 0;

              return (
                <motion.div
                  key={project.path}
                  layout
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className={clsx(
                    'flex items-center gap-3 px-4 py-3 rounded-xl transition-all group cursor-pointer',
                    isSelected
                      ? 'bg-brand-500/10 border border-brand-500/20'
                      : 'bg-white/5 border border-transparent hover:bg-white/10 hover:border-white/5'
                  )}
                  onClick={() => toggleSelect(project.path)}
                >
                  {/* Checkbox */}
                  <button className="flex-shrink-0 text-slate-500 hover:text-brand-400 transition-colors">
                    {isSelected
                      ? <CheckSquare className="w-4 h-4 text-brand-400" />
                      : <Square className="w-4 h-4" />}
                  </button>

                  {/* Project info */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-white truncate">{project.name}</span>
                      <span className={clsx('text-[10px] px-1.5 py-0.5 rounded-full border font-medium', typeConf.color)}>
                        {typeConf.label}
                      </span>
                      {project.has_git && (
                        <GitBranch className="w-3 h-3 text-slate-500" />
                      )}
                      {gitStatus?.current_branch && (
                        <span className="text-[10px] text-slate-500 font-mono">
                          {gitStatus.current_branch}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 mt-0.5">
                      <p className="text-xs text-slate-500 font-mono truncate">
                        {project.path}
                      </p>
                      {/* Git status indicators */}
                      {gitStatus && gitStatus.has_uncommitted && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-yellow-500/10 text-yellow-400 border border-yellow-500/20 flex-shrink-0 flex items-center gap-1">
                          <span className="w-1.5 h-1.5 rounded-full bg-yellow-400" />
                          {gitStatus.uncommitted_count} changes
                        </span>
                      )}
                      {gitStatus && totalAhead > 0 && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20 flex-shrink-0 flex items-center gap-1">
                          <ArrowUp className="w-2.5 h-2.5" />
                          {totalAhead} unpushed
                        </span>
                      )}
                      {gitStatus && gitStatus.remotes.length > 1 && (
                        <span className="text-[10px] text-slate-600 flex-shrink-0">
                          {gitStatus.remotes.map(r => r.name).join(', ')}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Git action buttons */}
                  {gitStatus && (gitStatus.has_uncommitted || totalAhead > 0) && (
                    <div className="flex items-center gap-1 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                      {gitStatus.has_uncommitted && (
                        <button
                          onClick={(e) => handleAutoCommit(project.path, e)}
                          disabled={isCommitting}
                          className="px-2 py-1 rounded-lg text-[10px] font-medium bg-yellow-500/10 text-yellow-400 border border-yellow-500/20 hover:bg-yellow-500/20 transition-all flex items-center gap-1"
                          title="Auto commit all changes"
                        >
                          {isCommitting ? (
                            <Loader2 className="w-3 h-3 animate-spin" />
                          ) : (
                            <GitCommitHorizontal className="w-3 h-3" />
                          )}
                          Commit
                        </button>
                      )}
                      {totalAhead > 0 && (
                        <button
                          onClick={(e) => handlePushAll(project.path, e)}
                          disabled={isPushing}
                          className="px-2 py-1 rounded-lg text-[10px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-all flex items-center gap-1"
                          title={`Push to ${gitStatus.remotes.map(r => r.name).join(', ')}`}
                        >
                          {isPushing ? (
                            <Loader2 className="w-3 h-3 animate-spin" />
                          ) : (
                            <Upload className="w-3 h-3" />
                          )}
                          Push All
                        </button>
                      )}
                    </div>
                  )}

                  {/* Artifact badge */}
                  {project.artifact_size != null && project.artifact_size > 0 && (
                    <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-yellow-500/10 text-yellow-400 border border-yellow-500/20 flex-shrink-0">
                      {formatBytes(project.artifact_size)} artifacts
                    </span>
                  )}

                  {/* Size */}
                  <div className="text-right flex-shrink-0 w-20">
                    {project.size_bytes != null ? (
                      <span className="text-sm font-medium text-slate-300">
                        {formatBytes(project.size_bytes)}
                      </span>
                    ) : (
                      <Loader2 className="w-3 h-3 text-slate-600 animate-spin ml-auto" />
                    )}
                  </div>

                  {/* Time */}
                  <div className="text-right flex-shrink-0 w-20">
                    <span className="text-xs text-slate-500">
                      {relativeTime(displayTime)}
                    </span>
                  </div>
                </motion.div>
              );
            })
          )
        ) : (
          /* Archived tab */
          archivedProjects.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-20">
              <Archive className="w-12 h-12 text-slate-600 mb-4" />
              <p className="text-slate-400 mb-1">No archived projects</p>
              {!archiveStatus?.configured && (
                <Link
                  href="/settings"
                  className="text-brand-400 hover:text-brand-300 text-sm flex items-center gap-1 mt-2"
                >
                  <Settings className="w-3 h-3" />
                  Set archive location in Settings
                </Link>
              )}
              {archiveStatus?.configured && !archiveStatus.exists && (
                <p className="text-xs text-slate-500 mt-1">
                  Archive disk is not connected
                </p>
              )}
            </div>
          ) : (
            archivedProjects.map((project) => {
              const typeConf = projectTypeConfig[project.project_type];
              const isSelected = selectedPaths.has(project.archived_path);

              return (
                <motion.div
                  key={project.archived_path}
                  layout
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className={clsx(
                    'flex items-center gap-3 px-4 py-3 rounded-xl transition-all group cursor-pointer',
                    isSelected
                      ? 'bg-brand-500/10 border border-brand-500/20'
                      : 'bg-white/5 border border-transparent hover:bg-white/10 hover:border-white/5'
                  )}
                  onClick={() => toggleSelect(project.archived_path)}
                >
                  <button className="flex-shrink-0 text-slate-500 hover:text-brand-400 transition-colors">
                    {isSelected
                      ? <CheckSquare className="w-4 h-4 text-brand-400" />
                      : <Square className="w-4 h-4" />}
                  </button>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-white truncate">{project.name}</span>
                      <span className={clsx('text-[10px] px-1.5 py-0.5 rounded-full border font-medium', typeConf.color)}>
                        {typeConf.label}
                      </span>
                    </div>
                    <p className="text-xs text-slate-500 font-mono truncate mt-0.5">
                      Original: {project.original_path}
                    </p>
                  </div>

                  <div className="text-right flex-shrink-0 w-20">
                    <span className="text-sm font-medium text-slate-300">
                      {formatBytes(project.size_bytes)}
                    </span>
                  </div>

                  <div className="text-right flex-shrink-0 w-24">
                    <span className="text-xs text-slate-500">
                      Archived {relativeTime(project.archived_at)}
                    </span>
                  </div>
                </motion.div>
              );
            })
          )
        )}
      </div>

      {/* Floating selection bar */}
      <AnimatePresence>
        {selectedPaths.size > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50"
          >
            <div className="flex items-center gap-4 px-6 py-3 rounded-2xl glass-heavy shadow-2xl shadow-black/50 border border-white/10">
              <div className="flex items-center gap-2 text-sm">
                <span className="text-slate-400">Selected:</span>
                <span className="font-bold text-white">{selectedPaths.size}</span>
                <span className="text-slate-500">|</span>
                <span className="font-medium text-brand-300">{formatBytes(selectedSize)}</span>
              </div>

              <div className="w-px h-6 bg-white/10" />

              <button
                onClick={selectAll}
                className="text-xs text-slate-400 hover:text-white transition-colors"
              >
                Select all
              </button>
              <button
                onClick={deselectAll}
                className="text-xs text-slate-400 hover:text-white transition-colors"
              >
                Clear
              </button>

              <div className="w-px h-6 bg-white/10" />

              {tab === 'active' ? (
                <button
                  onClick={handleArchive}
                  disabled={isArchiving || !archiveStatus?.configured}
                  className={clsx(
                    'px-4 py-2 rounded-xl flex items-center gap-2 font-medium text-sm transition-all',
                    archiveStatus?.configured
                      ? 'bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/20'
                      : 'bg-slate-600 text-slate-400 cursor-not-allowed',
                    isArchiving && 'opacity-50 cursor-not-allowed'
                  )}
                  title={!archiveStatus?.configured ? 'Set archive directory in Settings first' : ''}
                >
                  {isArchiving ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Archive className="w-4 h-4" />
                  )}
                  Archive
                </button>
              ) : (
                <button
                  onClick={handleRestore}
                  disabled={isArchiving}
                  className={clsx(
                    'px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-xl flex items-center gap-2 font-medium text-sm transition-all shadow-lg shadow-emerald-500/20',
                    isArchiving && 'opacity-50 cursor-not-allowed'
                  )}
                >
                  {isArchiving ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <ArchiveRestore className="w-4 h-4" />
                  )}
                  Restore
                </button>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
