'use client';

import { useState, useEffect } from 'react';
import { Sparkles, Play, Clock, Loader2, CheckCircle, AlertCircle } from 'lucide-react';
import { clsx } from 'clsx';
import { motion } from 'framer-motion';
import { listRecipes, runRecipe, estimateRecipe, isTauri } from '@/lib/tauri';
import type { Recipe, RecipeRunResult } from '@/lib/types';
import { formatBytes } from '@/lib/types';

const categoryEmoji: Record<string, string> = {
  dev: '🛠️',
  system: '🖥️',
  devtools: '🐳',
};

export default function RecipesPage() {
  const [recipes, setRecipes] = useState<Recipe[]>([]);
  const [loading, setLoading] = useState(false);
  const [estimating, setEstimating] = useState<Set<string>>(new Set());
  const [running, setRunning] = useState<Set<string>>(new Set());
  const [results, setResults] = useState<Record<string, RecipeRunResult>>({});
  const [confirmId, setConfirmId] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    setLoading(true);
    listRecipes()
      .then(setRecipes)
      .catch((err) => console.error('Failed to load recipes:', err))
      .finally(() => setLoading(false));
  }, []);

  const handleEstimate = async (id: string) => {
    if (!isTauri()) return;
    setEstimating((prev) => new Set([...prev, id]));
    try {
      const res = await estimateRecipe(id);
      setResults((prev) => ({ ...prev, [id]: res }));
    } catch (err) {
      console.error('Estimate failed:', err);
    } finally {
      setEstimating((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  };

  const handleRun = async (id: string) => {
    if (!isTauri()) return;
    setConfirmId(null);
    setRunning((prev) => new Set([...prev, id]));
    try {
      const res = await runRecipe(id, false);
      setResults((prev) => ({ ...prev, [id]: res }));
    } catch (err) {
      console.error('Run failed:', err);
    } finally {
      setRunning((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-white tracking-tight">Cleanup Recipes</h1>
        <p className="text-slate-400 mt-1">One-click scripts for common cleanup tasks</p>
      </div>

      {/* Loading */}
      {loading && (
        <div className="flex items-center gap-3 text-slate-400">
          <Loader2 className="w-5 h-5 animate-spin" />
          Loading recipes...
        </div>
      )}

      {/* Recipes Grid */}
      {recipes.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {recipes.map((recipe, index) => {
            const isEstimating = estimating.has(recipe.id);
            const isRunning = running.has(recipe.id);
            const result = results[recipe.id];
            const emoji = categoryEmoji[recipe.category] || '📦';

            return (
              <motion.div
                key={recipe.id}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.05 }}
                className="glass-card p-6 rounded-2xl group hover:border-white/20 transition-all hover:bg-white/5"
              >
                <div className="flex items-start gap-4">
                  <div className="flex items-center justify-center w-12 h-12 rounded-xl bg-black/30 border border-white/5 text-2xl group-hover:scale-110 transition-transform">
                    {emoji}
                  </div>
                  <div className="flex-1">
                    <h3 className="font-semibold text-white group-hover:text-brand-300 transition-colors">
                      {recipe.name}
                    </h3>
                    <p className="text-sm text-slate-400 mt-1 leading-relaxed">{recipe.description}</p>
                    <div className="flex items-center gap-4 mt-3">
                      <span className="text-xs text-brand-400 font-medium bg-brand-500/10 px-2 py-0.5 rounded-lg border border-brand-500/20">
                        ~{recipe.estimated_savings}
                      </span>
                      <span className="text-[10px] uppercase tracking-wider font-bold text-slate-500">
                        {recipe.category}
                      </span>
                      <span className="text-[10px] text-slate-600">
                        {recipe.steps.length} step{recipe.steps.length !== 1 ? 's' : ''}
                      </span>
                    </div>
                  </div>
                </div>

                {/* Result */}
                {result && (
                  <div className={clsx(
                    'mt-4 p-3 rounded-xl text-sm',
                    result.total_result.errors.length > 0
                      ? 'bg-danger-500/10 border border-danger-500/20'
                      : 'bg-emerald-500/10 border border-emerald-500/20'
                  )}>
                    <div className="flex items-center gap-2">
                      {result.total_result.errors.length > 0 ? (
                        <AlertCircle className="w-4 h-4 text-danger-400" />
                      ) : (
                        <CheckCircle className="w-4 h-4 text-emerald-400" />
                      )}
                      <span className="text-white font-medium">
                        {formatBytes(result.total_result.bytes_freed)} freed
                      </span>
                      <span className="text-slate-400">
                        ({result.steps_completed}/{result.total_steps} steps)
                      </span>
                    </div>
                    {result.total_result.errors.length > 0 && (
                      <p className="text-xs text-danger-400 mt-1 truncate">
                        {result.total_result.errors[0]}
                      </p>
                    )}
                  </div>
                )}

                {/* Confirm dialog */}
                {confirmId === recipe.id && (
                  <div className="mt-4 p-3 rounded-xl bg-yellow-500/10 border border-yellow-500/20">
                    <p className="text-sm text-yellow-200">Run this recipe? This will delete files.</p>
                    <div className="flex gap-2 mt-2">
                      <button
                        onClick={() => handleRun(recipe.id)}
                        className="px-3 py-1 text-xs rounded-lg bg-danger-600 hover:bg-danger-500 text-white font-medium transition-all"
                      >
                        Yes, Run
                      </button>
                      <button
                        onClick={() => setConfirmId(null)}
                        className="px-3 py-1 text-xs rounded-lg bg-white/10 hover:bg-white/20 text-slate-300 font-medium transition-all"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                )}

                {/* Action buttons */}
                <div className="flex items-center gap-3 mt-4 pt-4 border-t border-white/5">
                  <button
                    onClick={() => handleEstimate(recipe.id)}
                    disabled={isEstimating || isRunning}
                    className="flex-1 flex items-center justify-center gap-2 text-sm py-2 rounded-xl text-slate-400 hover:text-white hover:bg-white/5 transition-all font-medium disabled:opacity-50"
                  >
                    {isEstimating ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <Clock className="w-4 h-4" />
                    )}
                    Dry Run
                  </button>
                  <button
                    onClick={() => setConfirmId(recipe.id)}
                    disabled={isRunning || isEstimating}
                    className="flex-1 flex items-center justify-center gap-2 text-sm py-2 rounded-xl bg-white/10 hover:bg-brand-600 text-white transition-all font-medium shadow-lg hover:shadow-brand-500/20 disabled:opacity-50"
                  >
                    {isRunning ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <Play className="w-4 h-4 fill-current" />
                    )}
                    {isRunning ? 'Running...' : 'Run Now'}
                  </button>
                </div>
              </motion.div>
            );
          })}
        </div>
      )}

      {/* Custom Recipe placeholder */}
      <div className="glass-card p-8 border-dashed border-2 border-white/10 rounded-2xl hover:border-white/20 transition-colors bg-transparent">
        <div className="text-center">
          <div className="w-16 h-16 rounded-full bg-white/5 flex items-center justify-center mx-auto mb-4 border border-white/5">
            <Sparkles className="w-8 h-8 text-slate-400" />
          </div>
          <h3 className="font-medium text-white text-lg">Create Custom Recipe</h3>
          <p className="text-sm text-slate-400 mt-2 max-w-sm mx-auto">
            Build your own cleanup workflows with custom file patterns, retention rules, and schedules.
          </p>
          <button className="mt-6 px-6 py-2 rounded-full border border-white/10 text-slate-300 hover:text-white hover:bg-white/5 text-sm font-medium transition-all">
            Coming Soon
          </button>
        </div>
      </div>
    </div>
  );
}
