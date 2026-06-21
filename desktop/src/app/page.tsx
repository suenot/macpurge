'use client';

import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Code2,
  AppWindow,
  Wrench,
  Package,
  Trash2,
  RefreshCw,
  Play,
  AlertCircle,
  HardDrive
} from 'lucide-react';
import { clsx } from 'clsx';
import { scanDev, getConfig, isTauri } from '@/lib/tauri';
import type { ScanResult, ConfigResponse } from '@/lib/types';
import { formatBytes, formatBytesParts } from '@/lib/types';
import Link from 'next/link';

interface CategoryCard {
  id: string;
  name: string;
  icon: React.ComponentType<{ className?: string }>;
  href: string;
  color: string;
  bgColor: string;
  size: number;
  count: number;
}

export default function Dashboard() {
  const [isScanning, setIsScanning] = useState(false);
  const [scanResults, setScanResults] = useState<ScanResult[]>([]);
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    const loadConfig = async () => {
      try {
        const cfg = await getConfig();
        setConfig(cfg);
      } catch (err) {
        console.error('Failed to load config:', err);
      }
    };

    loadConfig();
  }, []);

  const handleScan = async () => {
    if (!isTauri()) return;

    setIsScanning(true);
    setError(null);

    try {
      const results = await scanDev();
      setScanResults(results);
    } catch (err) {
      setError(err as string);
    } finally {
      setIsScanning(false);
    }
  };

  // Calculate totals from scan results
  const totalSize = scanResults.reduce((acc, r) => acc + r.total_size, 0);
  const totalItems = scanResults.reduce((acc, r) => acc + r.entries.length, 0);

  const categories: CategoryCard[] = [
    {
      id: 'dev',
      name: 'Dev Artifacts',
      icon: Code2,
      href: '/dev',
      color: 'text-blue-400',
      bgColor: 'bg-blue-500/20',
      size: totalSize, // Currently we only scan dev artifacts in the main scan
      count: totalItems,
    },
    {
      id: 'apps',
      name: 'Applications',
      icon: AppWindow,
      href: '/apps',
      color: 'text-purple-400',
      bgColor: 'bg-purple-500/20',
      size: 0,
      count: 0,
    },
    {
      id: 'devtools',
      name: 'Dev Tools',
      icon: Wrench,
      href: '/devtools',
      color: 'text-orange-400',
      bgColor: 'bg-orange-500/20',
      size: 0,
      count: 0,
    },
    {
      id: 'packages',
      name: 'Package Caches',
      icon: Package,
      href: '/dev',
      color: 'text-green-400',
      bgColor: 'bg-green-500/20',
      size: 0,
      count: 0,
    },
  ];

  const hasProjectDirs = config && config.project_dirs.length > 0;

  return (
    <div className="space-y-8 max-w-7xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white tracking-tight">
            Dashboard
          </h1>
          <p className="text-slate-400 mt-1">
            System health and cleanup summary
          </p>
        </div>
        <button
          onClick={handleScan}
          disabled={isScanning || !hasProjectDirs}
          className={clsx(
            'group relative inline-flex items-center justify-center px-6 py-3 text-sm font-medium text-white transition-all duration-200 bg-brand-600 rounded-xl hover:bg-brand-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-brand-500 shadow-lg shadow-brand-500/30 overflow-hidden',
            (isScanning || !hasProjectDirs) && 'opacity-50 cursor-not-allowed grayscale'
          )}
        >
          {isScanning && (
            <div className="absolute inset-0 w-full h-full bg-white/20 animate-pulse" />
          )}
          {isScanning ? (
            <>
              <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
              Scanning...
            </>
          ) : (
            <>
              <Play className="w-4 h-4 mr-2 fill-current" />
              Start Scan
            </>
          )}
        </button>
      </div>

      {/* Hero Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="md:col-span-2 glass-card p-8 rounded-2xl relative overflow-hidden group"
        >
          <div className="absolute top-0 right-0 -mr-16 -mt-16 w-64 h-64 bg-brand-500/20 rounded-full blur-3xl group-hover:bg-brand-500/30 transition-all duration-1000" />

          <div className="relative z-10 flex flex-col h-full justify-between">
            <div>
              <h2 className="text-slate-400 font-medium mb-1">Potential Space to Free</h2>
              <div className="flex items-baseline gap-2">
                <span className="text-6xl font-extrabold text-transparent bg-clip-text bg-gradient-to-r from-white to-slate-300 tracking-tight">
                  {formatBytesParts(totalSize).value}
                </span>
                <span className="text-2xl font-semibold text-slate-500">
                  {formatBytesParts(totalSize).unit}
                </span>
              </div>
              <p className="text-sm text-slate-500 mt-2">
                {totalItems} reclaimable items found across {scanResults.length} locations
              </p>
            </div>

            {totalSize > 0 && (
              <div className="mt-8">
                <button className="flex items-center gap-2 px-6 py-3 bg-danger-500/10 hover:bg-danger-500/20 text-danger-400 hover:text-danger-300 rounded-xl transition-all border border-danger-500/20 hover:border-danger-500/40 group-hover:scale-105 origin-left">
                  <Trash2 className="w-4 h-4" />
                  Clean All Artifacts
                </button>
              </div>
            )}
          </div>
        </motion.div>

        <div className="space-y-6">
          {/* Quick Status / Alerts */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="glass-card p-6 rounded-2xl h-full flex flex-col justify-center"
          >
            {!hasProjectDirs ? (
              <div className="flex flex-col items-center text-center p-2">
                <div className="w-12 h-12 bg-yellow-500/10 rounded-full flex items-center justify-center mb-4">
                  <AlertCircle className="w-6 h-6 text-yellow-500" />
                </div>
                <h3 className="text-white font-medium">Setup Required</h3>
                <p className="text-sm text-slate-400 mt-2 mb-4">
                  Configure your project directories to start cleaning.
                </p>
                <Link
                  href="/settings"
                  className="text-sm font-medium text-brand-400 hover:text-brand-300 hover:underline"
                >
                  Go to Settings →
                </Link>
              </div>
            ) : (
              <div className="flex flex-col items-center text-center p-2">
                <div className="w-12 h-12 bg-emerald-500/10 rounded-full flex items-center justify-center mb-4">
                  <HardDrive className="w-6 h-6 text-emerald-500" />
                </div>
                <h3 className="text-white font-medium">System Ready</h3>
                <p className="text-sm text-slate-400 mt-2">
                  MacPurge is ready to optimize your disk space.
                </p>
              </div>
            )}
          </motion.div>
        </div>
      </div>

      {/* Categories */}
      <div>
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <span className="w-1 h-5 bg-brand-500 rounded-full" />
          Categories
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {categories.map((category, index) => {
            const Icon = category.icon;
            return (
              <motion.div
                key={category.id}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.2 + index * 0.1 }}
              >
                <Link href={category.href}>
                  <div className="glass-card p-5 rounded-xl group relative overflow-hidden">
                    <div className={clsx("absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500", category.bgColor)} />

                    <div className="relative z-10 flex flex-col h-full bg-transparent">
                      <div className="flex justify-between items-start mb-4">
                        <div className={clsx('p-2 rounded-lg bg-black/20', category.color)}>
                          <Icon className="w-5 h-5" />
                        </div>
                        {category.count > 0 && (
                          <span className="text-[10px] font-bold px-2 py-1 rounded-full bg-white/10 text-white border border-white/5">
                            {category.count}
                          </span>
                        )}
                      </div>

                      <div className="mt-auto">
                        <h4 className="text-slate-400 text-sm font-medium mb-1 group-hover:text-white transition-colors">
                          {category.name}
                        </h4>
                        <p className={clsx(
                          "text-xl font-bold tracking-tight transition-all",
                          category.size > 0 ? "text-white" : "text-slate-600"
                        )}>
                          {category.size > 0 ? formatBytes(category.size) : '—'}
                        </p>
                      </div>
                    </div>
                  </div>
                </Link>
              </motion.div>
            );
          })}
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-4 rounded-xl border border-danger-500/30 bg-danger-500/10 flex items-center gap-3 text-danger-200"
        >
          <AlertCircle className="w-5 h-5" />
          <p>{error}</p>
        </motion.div>
      )}

      {/* Recent Artifacts (Glass List) */}
      {scanResults.length > 0 && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.5 }}
          className="glass-card rounded-2xl overflow-hidden"
        >
          <div className="p-6 border-b border-white/5 flex items-center justify-between">
            <h3 className="font-semibold text-white">Found Artifacts</h3>
            <span className="text-xs text-slate-500 uppercase tracking-wider font-medium">Preview</span>
          </div>
          <div className="max-h-80 overflow-y-auto custom-scrollbar p-2">
            {scanResults.flatMap((result) =>
              result.entries.slice(0, 10).map((entry, i) => (
                <div
                  key={`${result.root_path}-${i}`}
                  className="flex items-center justify-between py-3 px-4 rounded-lg hover:bg-white/5 group transition-colors"
                >
                  <div className="flex items-center gap-4 min-w-0">
                    <div className="w-8 h-8 rounded-lg bg-slate-800/50 flex items-center justify-center text-slate-500 group-hover:text-white group-hover:bg-brand-500/20 transition-all">
                      <Code2 className="w-4 h-4" />
                    </div>
                    <div className="min-w-0">
                      <p className="text-sm text-slate-300 truncate font-medium group-hover:text-white transition-colors">
                        {entry.path.split('/').pop()}
                      </p>
                      <p className="text-xs text-slate-600 truncate max-w-md">
                        {entry.path}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center pl-4">
                    <span className="text-sm font-mono text-slate-400 group-hover:text-brand-300 transition-colors">
                      {formatBytes(entry.size_bytes)}
                    </span>
                  </div>
                </div>
              ))
            )}
            {totalItems > 10 && (
              <div className="p-4 text-center">
                <Link href="/dev" className="text-xs text-brand-400 hover:text-brand-300 font-medium hover:underline">
                  View all {totalItems} items
                </Link>
              </div>
            )}
          </div>
        </motion.div>
      )}
    </div>
  );
}
