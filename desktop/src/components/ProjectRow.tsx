'use client';

import { memo } from 'react';
import { motion } from 'framer-motion';
import {
  GitBranch,
  ArrowUp,
  Loader2,
  GitCommitHorizontal,
  Upload,
  FolderOpen,
  Trash2,
  CheckSquare,
  Square,
  X,
} from 'lucide-react';
import { clsx } from 'clsx';
import type { ProjectInfo, ProjectType, GitStatus } from '@/lib/types';
import { formatBytes } from '@/lib/types';

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

export interface ProjectRowProps {
  project: ProjectInfo;
  /** Child rows render indented, without checkbox / selection / open-delete actions. */
  indent?: boolean;
  /** Git status for this project, if it has been scanned yet. */
  gitStatus?: GitStatus;
  /** Whether this standalone row is selected (ignored for child rows). */
  isSelected?: boolean;
  /** Pending action for this project: 'commit' | 'push' | 'delete' | undefined. */
  actionKind?: string;
  /** Whether the inline delete confirmation is showing for this standalone row. */
  confirmingDelete?: boolean;
  // Stable callbacks (keyed by path) — kept out of memo comparison via useCallback in the parent.
  onToggleSelect?: (path: string) => void;
  onCommit?: (path: string, e: React.MouseEvent) => void;
  onPush?: (path: string, e: React.MouseEvent) => void;
  onOpenInFinder?: (path: string, e: React.MouseEvent) => void;
  onRequestDelete?: (path: string) => void;
  onCancelDelete?: () => void;
  onConfirmDelete?: (path: string) => void;
}

// Shared row used for both standalone project rows (with checkbox + open/delete
// actions) and group child rows (indented, read-only). Wrapped in React.memo so
// streamed size/git updates only re-render the rows whose props actually changed.
function ProjectRowInner({
  project,
  indent = false,
  gitStatus,
  isSelected = false,
  actionKind,
  confirmingDelete = false,
  onToggleSelect,
  onCommit,
  onPush,
  onOpenInFinder,
  onRequestDelete,
  onCancelDelete,
  onConfirmDelete,
}: ProjectRowProps) {
  const typeConf = projectTypeConfig[project.project_type];
  const displayTime = project.last_commit_time ?? project.last_modified;
  const isCommitting = actionKind === 'commit';
  const isPushing = actionKind === 'push';
  const isDeleting = actionKind === 'delete';
  const totalAhead = gitStatus?.remotes.reduce((s, r) => s + r.ahead, 0) ?? 0;

  // Shared inner content (project info, git badges, git actions, artifact, size, time).
  const content = (
    <>
      {/* Indent spacer for children (no checkbox) */}
      {indent && <div className="w-4 flex-shrink-0" />}

      {/* Project info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className={clsx('font-medium truncate', indent ? 'text-slate-300 text-sm' : 'text-white')}>
            {project.name}
          </span>
          <span className={clsx('text-[10px] px-1.5 py-0.5 rounded-full border font-medium', typeConf.color)}>
            {typeConf.label}
          </span>
          {project.has_git && <GitBranch className="w-3 h-3 text-slate-500" />}
          {gitStatus?.current_branch && (
            <span className="text-[10px] text-slate-500 font-mono">{gitStatus.current_branch}</span>
          )}
        </div>
        <div className="flex items-center gap-2 mt-0.5">
          <p className="text-xs text-slate-500 font-mono truncate">{project.path}</p>
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
              onClick={(e) => onCommit?.(project.path, e)}
              disabled={isCommitting}
              className="px-2 py-1 rounded-lg text-[10px] font-medium bg-yellow-500/10 text-yellow-400 border border-yellow-500/20 hover:bg-yellow-500/20 transition-all flex items-center gap-1"
              title="Auto commit all changes"
            >
              {isCommitting ? <Loader2 className="w-3 h-3 animate-spin" /> : <GitCommitHorizontal className="w-3 h-3" />}
              Commit
            </button>
          )}
          {totalAhead > 0 && (
            <button
              onClick={(e) => onPush?.(project.path, e)}
              disabled={isPushing}
              className="px-2 py-1 rounded-lg text-[10px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-all flex items-center gap-1"
              title={`Push to ${gitStatus.remotes.map(r => r.name).join(', ')}`}
            >
              {isPushing ? <Loader2 className="w-3 h-3 animate-spin" /> : <Upload className="w-3 h-3" />}
              Push All
            </button>
          )}
        </div>
      )}

      {/* Open / Delete buttons (standalone rows only) */}
      {!indent && (
        <div className="flex items-center gap-1 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
          <button
            onClick={(e) => onOpenInFinder?.(project.path, e)}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition-all"
            title="Open in Finder"
          >
            <FolderOpen className="w-3.5 h-3.5" />
          </button>
          {confirmingDelete ? (
            <div className="flex items-center gap-1">
              <button
                onClick={(e) => { e.stopPropagation(); onConfirmDelete?.(project.path); }}
                className="px-2 py-1 rounded-lg text-[10px] font-medium bg-danger-500/20 text-danger-400 border border-danger-500/30 hover:bg-danger-500/30 transition-all"
              >
                Confirm
              </button>
              <button
                onClick={(e) => { e.stopPropagation(); onCancelDelete?.(); }}
                className="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition-all"
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          ) : (
            <button
              onClick={(e) => { e.stopPropagation(); onRequestDelete?.(project.path); }}
              disabled={isDeleting}
              className="p-1.5 rounded-lg text-slate-400 hover:text-danger-400 hover:bg-danger-500/10 transition-all"
              title="Delete project"
            >
              {isDeleting ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Trash2 className="w-3.5 h-3.5" />}
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
          <span className="text-sm font-medium text-slate-300">{formatBytes(project.size_bytes)}</span>
        ) : (
          <Loader2 className="w-3 h-3 text-slate-600 animate-spin ml-auto" />
        )}
      </div>

      {/* Time */}
      <div className="text-right flex-shrink-0 w-20">
        <span className="text-xs text-slate-500">{relativeTime(displayTime)}</span>
      </div>
    </>
  );

  // Child rows: plain indented div, read-only.
  if (indent) {
    return (
      <div className="flex items-center gap-3 py-2.5 rounded-xl transition-all group px-4 pl-12 bg-white/[0.02] hover:bg-white/[0.05]">
        {content}
      </div>
    );
  }

  // Standalone rows: selectable, with a light fade-in (no `layout` to avoid
  // re-measuring every row's bounding box as sizes stream in).
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className={clsx(
        'flex items-center gap-3 px-4 py-3 rounded-xl transition-all group cursor-pointer',
        isSelected
          ? 'bg-brand-500/10 border border-brand-500/20'
          : 'bg-white/5 border border-transparent hover:bg-white/10 hover:border-white/5'
      )}
      onClick={() => onToggleSelect?.(project.path)}
    >
      {/* Checkbox */}
      <button className="flex-shrink-0 text-slate-500 hover:text-brand-400 transition-colors">
        {isSelected ? <CheckSquare className="w-4 h-4 text-brand-400" /> : <Square className="w-4 h-4" />}
      </button>
      {content}
    </motion.div>
  );
}

export const ProjectRow = memo(ProjectRowInner);
