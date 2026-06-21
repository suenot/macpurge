'use client';

import { Trash2, AlertTriangle } from 'lucide-react';

interface ConfirmDialogProps {
  /** Modal title, e.g. "Clean selected caches?" */
  title: string;
  /** Short, human description of what will be removed. */
  message: string;
  /** Optional list of paths that will be affected. */
  paths?: string[];
  /** Optional human-readable total size, e.g. "1.2 GB". */
  totalSize?: string;
  /** Label for the destructive confirm button. */
  confirmLabel?: string;
  onCancel: () => void;
  onConfirm: () => void;
}

// Native window.confirm() is a no-op inside the Tauri WKWebview, so destructive
// cache cleans use this in-app modal instead. It mirrors the trash-confirm modal
// used on the Applications page so the look stays consistent.
export default function ConfirmDialog({
  title,
  message,
  paths,
  totalSize,
  confirmLabel = 'Delete',
  onCancel,
  onConfirm,
}: ConfirmDialogProps) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onCancel}
    >
      <div
        className="glass-card w-full max-w-md mx-4 rounded-2xl border border-white/10 p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3 mb-4">
          <div className="w-10 h-10 rounded-lg bg-danger-500/10 border border-danger-500/20 flex items-center justify-center flex-shrink-0">
            <AlertTriangle className="w-5 h-5 text-danger-400" />
          </div>
          <h2 className="text-lg font-bold text-white">{title}</h2>
        </div>

        <p className="text-sm text-slate-300">{message}</p>

        {paths && paths.length > 0 && (
          <div className="mt-3 max-h-40 overflow-y-auto custom-scrollbar rounded-lg border border-white/5 bg-black/20 p-2 space-y-0.5">
            {paths.map((p) => (
              <p key={p} className="text-xs text-slate-500 font-mono truncate" title={p}>
                {p}
              </p>
            ))}
          </div>
        )}

        <p className="text-xs text-danger-300/90 mt-3">
          This permanently removes the items above
          {totalSize ? ` (${totalSize})` : ''} and cannot be undone.
        </p>

        <div className="flex items-center justify-end gap-3 mt-6">
          <button
            onClick={onCancel}
            className="px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl transition-all font-medium"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className="px-4 py-2 bg-danger-500/90 hover:bg-danger-500 text-white rounded-xl transition-all font-medium flex items-center gap-2"
          >
            <Trash2 className="w-4 h-4" />
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
