'use client';

import { useState } from 'react';
import {
  Shield,
  RefreshCw,
  Trash2,
  Loader2,
  HardDrive,
  Mail,
  Download,
  FileText,
  Smartphone,
  Globe,
} from 'lucide-react';
import { clsx } from 'clsx';
import { motion } from 'framer-motion';
import { scanSystem, cleanSystemCategory, isTauri } from '@/lib/tauri';
import type { SystemScanResult, SystemCategoryInfo } from '@/lib/types';
import { formatBytes } from '@/lib/types';

const categoryIcons: Record<string, typeof Shield> = {
  user_cache: HardDrive,
  system_cache: Shield,
  browser_cache: Globe,
  system_logs: FileText,
  ios_backups: Smartphone,
  trash: Trash2,
  downloads_old: Download,
  mail_caches: Mail,
};

export default function SystemPage() {
  const [scanning, setScanning] = useState(false);
  const [result, setResult] = useState<SystemScanResult | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [cleaning, setCleaning] = useState<Set<string>>(new Set());
  const [cleaned, setCleaned] = useState<Set<string>>(new Set());

  const handleScan = async () => {
    if (!isTauri()) return;
    setScanning(true);
    setResult(null);
    setSelected(new Set());
    setCleaned(new Set());
    try {
      const data = await scanSystem();
      setResult(data);
    } catch (err) {
      console.error('System scan failed:', err);
    } finally {
      setScanning(false);
    }
  };

  const toggleCategory = (cat: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(cat)) next.delete(cat);
      else next.add(cat);
      return next;
    });
  };

  const handleClean = async () => {
    if (!isTauri() || selected.size === 0) return;
    for (const cat of selected) {
      setCleaning((prev) => new Set([...prev, cat]));
      try {
        await cleanSystemCategory(cat, false);
        setCleaned((prev) => new Set([...prev, cat]));
      } catch (err) {
        console.error('Clean failed for', cat, err);
      } finally {
        setCleaning((prev) => {
          const next = new Set(prev);
          next.delete(cat);
          return next;
        });
      }
    }
    // Re-scan
    handleScan();
  };

  const totalSelected = result
    ? result.categories
        .filter((c) => selected.has(c.category))
        .reduce((sum, c) => sum + c.size_bytes, 0)
    : 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">System Caches</h1>
          <p className="text-slate-400 mt-1">Clean system logs, caches, old downloads, and more</p>
        </div>
        <button
          onClick={handleScan}
          disabled={scanning}
          className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 text-white font-medium transition-all disabled:opacity-50"
        >
          {scanning ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCw className="w-4 h-4" />}
          {scanning ? 'Scanning...' : 'Scan System'}
        </button>
      </div>

      {/* Summary */}
      {result && (
        <div className="glass-card p-4 rounded-2xl flex items-center justify-between">
          <span className="text-slate-400">
            Found <span className="text-white font-semibold">{formatBytes(result.total_size)}</span> across{' '}
            {result.categories.length} categories
            <span className="text-slate-600 ml-2">({result.scan_duration_ms}ms)</span>
          </span>
          {selected.size > 0 && (
            <button
              onClick={handleClean}
              disabled={cleaning.size > 0}
              className="flex items-center gap-2 px-4 py-2 rounded-xl bg-danger-600 hover:bg-danger-500 text-white text-sm font-medium transition-all disabled:opacity-50"
            >
              {cleaning.size > 0 ? <Loader2 className="w-4 h-4 animate-spin" /> : <Trash2 className="w-4 h-4" />}
              Clean Selected ({formatBytes(totalSelected)})
            </button>
          )}
        </div>
      )}

      {/* Categories */}
      {result && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {result.categories.map((cat, index) => {
            const Icon = categoryIcons[cat.category] || Shield;
            const isSelected = selected.has(cat.category);
            const isCleaning = cleaning.has(cat.category);
            const isCleaned = cleaned.has(cat.category);

            return (
              <motion.label
                key={cat.category}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.05 }}
                className={clsx(
                  'glass-card p-5 rounded-2xl cursor-pointer transition-all',
                  isSelected ? 'border-brand-500/30 bg-brand-500/5' : 'hover:bg-white/5',
                  isCleaned && 'opacity-40'
                )}
              >
                <div className="flex items-start gap-4">
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onChange={() => toggleCategory(cat.category)}
                    className="mt-1 rounded border-white/20 bg-white/10 text-brand-500 focus:ring-brand-500/50"
                  />
                  <div className={clsx(
                    'p-3 rounded-xl border',
                    isSelected
                      ? 'bg-brand-500/10 border-brand-500/20'
                      : 'bg-slate-800 border-white/5'
                  )}>
                    <Icon className={clsx('w-5 h-5', isSelected ? 'text-brand-400' : 'text-slate-500')} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between">
                      <h3 className="font-semibold text-white">{cat.name}</h3>
                      <span className="text-lg font-bold text-white ml-2">{formatBytes(cat.size_bytes)}</span>
                    </div>
                    <p className="text-sm text-slate-400 mt-1">{cat.description}</p>
                    <div className="flex items-center gap-3 mt-2 text-xs text-slate-500">
                      <span>{cat.item_count} items</span>
                      {isCleaning && (
                        <span className="flex items-center gap-1 text-brand-400">
                          <Loader2 className="w-3 h-3 animate-spin" /> Cleaning...
                        </span>
                      )}
                    </div>
                    {/* Paths */}
                    {cat.paths.length > 0 && (
                      <div className="mt-3 space-y-1">
                        {cat.paths.slice(0, 3).map((p) => (
                          <div key={p.path} className="flex items-center justify-between text-xs">
                            <span className="text-slate-500 truncate mr-2">{p.description}</span>
                            <span className="text-yellow-400 shrink-0">{formatBytes(p.size_bytes)}</span>
                          </div>
                        ))}
                        {cat.paths.length > 3 && (
                          <span className="text-[10px] text-slate-600">+{cat.paths.length - 3} more</span>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </motion.label>
            );
          })}
        </div>
      )}

      {/* Empty state */}
      {!result && !scanning && (
        <div className="glass-card p-12 rounded-2xl text-center">
          <div className="w-16 h-16 rounded-full bg-white/5 flex items-center justify-center mx-auto mb-4 border border-white/5">
            <Shield className="w-8 h-8 text-slate-400" />
          </div>
          <h3 className="font-medium text-white text-lg">Scan System Caches</h3>
          <p className="text-sm text-slate-400 mt-2 max-w-sm mx-auto">
            Click &quot;Scan System&quot; to discover system logs, browser caches, iOS backups, old downloads, and more.
          </p>
        </div>
      )}
    </div>
  );
}
