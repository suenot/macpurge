'use client';

import { useState } from 'react';
import { BrainCircuit, Boxes, Box, Cloud, Image as ImageIcon, RefreshCw, Trash2, Loader2 } from 'lucide-react';
import { clsx } from 'clsx';
import { motion } from 'framer-motion';
import { scanAiModels, cleanAiModelPath, isTauri } from '@/lib/tauri';
import type { AiModelScanResult, AiModelPath } from '@/lib/types';
import { formatBytes } from '@/lib/types';
import ConfirmDialog from '@/components/ui/ConfirmDialog';
import { useConfirmBeforeDelete } from '@/hooks/useConfirmBeforeDelete';

const toolIcons: Record<string, typeof BrainCircuit> = {
  ollama: Box,
  lm_studio: Boxes,
  hugging_face: Cloud,
  comfy_ui: ImageIcon,
};

// Badge colours per model category surfaced by the scanner.
const categoryStyles: Record<string, string> = {
  paroquant: 'bg-fuchsia-500/20 text-fuchsia-300',
  vmlx: 'bg-sky-500/20 text-sky-300',
  mlx: 'bg-cyan-500/20 text-cyan-300',
  diffusers: 'bg-pink-500/20 text-pink-300',
  ollama: 'bg-emerald-500/20 text-emerald-300',
  lmstudio: 'bg-indigo-500/20 text-indigo-300',
  comfyui: 'bg-amber-500/20 text-amber-300',
  huggingface: 'bg-yellow-500/20 text-yellow-300',
  xet: 'bg-slate-500/20 text-slate-300',
};

export default function AiModelsPage() {
  const [scanning, setScanning] = useState(false);
  const [result, setResult] = useState<AiModelScanResult | null>(null);
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
      const data = await scanAiModels();
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

  const selectAllInTool = (models: AiModelPath[], allSelected: boolean) => {
    setSelected(prev => {
      const next = new Set(prev);
      for (const m of models) {
        if (allSelected) next.delete(m.path);
        else next.add(m.path);
      }
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
        await cleanAiModelPath(path, false);
        setCleanedPaths(prev => new Set([...prev, path]));
      } catch (err) {
        console.error('Clean failed for', path, err);
      }
    }
    setCleaning(false);
    handleScan();
  };

  const allModels = result ? result.tools.flatMap(t => t.models) : [];
  const totalSelected = allModels
    .filter(m => selected.has(m.path))
    .reduce((sum, m) => sum + m.size_bytes, 0);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">AI Models</h1>
          <p className="text-slate-400 mt-1">Reclaim space from local LLM &amp; diffusion model artifacts</p>
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
              const Icon = toolIcons[tool.tool] || BrainCircuit;
              const allSelected = tool.models.length > 0 && tool.models.every(m => selected.has(m.path));
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

                  {tool.models.length === 0 ? (
                    <p className="text-sm text-slate-500">No models found</p>
                  ) : (
                    <>
                      <button
                        onClick={() => selectAllInTool(tool.models, allSelected)}
                        className="text-xs text-brand-400 hover:text-brand-300 mb-2 transition-colors"
                      >
                        {allSelected ? 'Deselect all' : 'Select all'}
                      </button>
                      <div className="space-y-2">
                        {tool.models.map((model) => (
                          <label
                            key={model.path}
                            className={clsx(
                              'flex items-center gap-3 p-3 rounded-xl cursor-pointer transition-all',
                              selected.has(model.path)
                                ? 'bg-brand-500/10 border border-brand-500/30'
                                : 'bg-white/5 border border-transparent hover:bg-white/10',
                              cleanedPaths.has(model.path) && 'opacity-40'
                            )}
                          >
                            <input
                              type="checkbox"
                              checked={selected.has(model.path)}
                              onChange={() => togglePath(model.path)}
                              className="rounded border-white/20 bg-white/10 text-brand-500 focus:ring-brand-500/50"
                            />
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="text-sm text-white truncate">{model.description}</span>
                                <span className={clsx(
                                  'text-[9px] px-1.5 py-0.5 rounded shrink-0',
                                  categoryStyles[model.category] || 'bg-slate-500/20 text-slate-300'
                                )}>
                                  {model.category}
                                </span>
                              </div>
                              <p className="text-[11px] text-slate-500 truncate mt-0.5">{model.path}</p>
                            </div>
                            <span className="text-sm font-medium text-yellow-400 shrink-0">{formatBytes(model.size_bytes)}</span>
                          </label>
                        ))}
                      </div>
                    </>
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
            <BrainCircuit className="w-8 h-8 text-slate-400" />
          </div>
          <h3 className="font-medium text-white text-lg">Scan AI Models</h3>
          <p className="text-sm text-slate-400 mt-2 max-w-sm mx-auto">
            Click &quot;Scan All&quot; to discover downloaded models from Ollama, LM Studio, the Hugging Face cache (MLX, paroquant, diffusers), and ComfyUI.
          </p>
        </div>
      )}

      {/* Destructive clean confirmation */}
      {confirmClean && (
        <ConfirmDialog
          title="Delete selected models?"
          message={`Permanently delete ${selected.size} model${selected.size === 1 ? '' : 's'}. Re-downloading them later may take significant time and bandwidth.`}
          paths={Array.from(selected)}
          totalSize={formatBytes(totalSelected)}
          confirmLabel="Delete"
          onCancel={() => setConfirmClean(false)}
          onConfirm={runClean}
        />
      )}
    </div>
  );
}

function toolName(type: string): string {
  switch (type) {
    case 'ollama': return 'Ollama';
    case 'lm_studio': return 'LM Studio';
    case 'hugging_face': return 'Hugging Face Cache';
    case 'comfy_ui': return 'ComfyUI';
    default: return type;
  }
}
