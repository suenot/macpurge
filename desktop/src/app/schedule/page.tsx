'use client';

import { useState, useEffect } from 'react';
import {
  Clock,
  Power,
  Play,
  Loader2,
  CheckCircle,
  AlertCircle,
  Calendar,
} from 'lucide-react';
import { clsx } from 'clsx';
import { motion } from 'framer-motion';
import {
  getScheduleStatus,
  enableSchedule,
  disableSchedule,
  runScheduleNow,
  isTauri,
} from '@/lib/tauri';
import type { ScheduleStatus, ScheduleTasks, ScheduleRunResult } from '@/lib/types';
import { formatBytes } from '@/lib/types';

const cronPresets = [
  { label: 'Daily', value: 'daily', description: 'Every day at 3:00 AM' },
  { label: 'Weekly', value: 'weekly', description: 'Every Sunday at 3:00 AM' },
  { label: 'Monthly', value: 'monthly', description: '1st of each month at 3:00 AM' },
];

const taskLabels: { key: keyof ScheduleTasks; label: string; description: string }[] = [
  { key: 'clean_dev', label: 'Dev Caches', description: 'node_modules, target/, .venv' },
  { key: 'clean_packages', label: 'Package Caches', description: 'npm, cargo, pip, brew caches' },
  { key: 'clean_docker', label: 'Docker', description: 'Dangling images and containers' },
  { key: 'clean_system', label: 'System Caches', description: 'User and system cache dirs' },
  { key: 'clean_browsers', label: 'Browser Caches', description: 'Chrome, Safari, Firefox caches' },
  { key: 'empty_trash', label: 'Empty Trash', description: 'Clear ~/.Trash contents' },
];

const defaultTasks: ScheduleTasks = {
  clean_dev: true,
  clean_packages: true,
  clean_docker: false,
  clean_system: false,
  clean_browsers: false,
  empty_trash: false,
};

export default function SchedulePage() {
  const [status, setStatus] = useState<ScheduleStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [runningNow, setRunningNow] = useState(false);
  const [runResult, setRunResult] = useState<ScheduleRunResult | null>(null);

  const [selectedCron, setSelectedCron] = useState('weekly');
  const [tasks, setTasks] = useState<ScheduleTasks>(defaultTasks);

  useEffect(() => {
    if (!isTauri()) return;
    fetchStatus();
  }, []);

  const fetchStatus = async () => {
    setLoading(true);
    try {
      const s = await getScheduleStatus();
      setStatus(s);
      if (s.cron) setSelectedCron(s.cron);
      setTasks(s.tasks);
    } catch (err) {
      console.error('Failed to get schedule status:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleToggle = async () => {
    if (!isTauri() || !status) return;
    setSaving(true);
    try {
      if (status.enabled) {
        await disableSchedule();
      } else {
        await enableSchedule(selectedCron, tasks);
      }
      await fetchStatus();
    } catch (err) {
      console.error('Failed to toggle schedule:', err);
    } finally {
      setSaving(false);
    }
  };

  const handleSave = async () => {
    if (!isTauri()) return;
    setSaving(true);
    try {
      await enableSchedule(selectedCron, tasks);
      await fetchStatus();
    } catch (err) {
      console.error('Failed to save schedule:', err);
    } finally {
      setSaving(false);
    }
  };

  const handleRunNow = async () => {
    if (!isTauri()) return;
    setRunningNow(true);
    setRunResult(null);
    try {
      const res = await runScheduleNow();
      setRunResult(res);
    } catch (err) {
      console.error('Run now failed:', err);
    } finally {
      setRunningNow(false);
    }
  };

  const toggleTask = (key: keyof ScheduleTasks) => {
    setTasks((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  if (loading) {
    return (
      <div className="flex items-center gap-3 text-slate-400 p-12">
        <Loader2 className="w-5 h-5 animate-spin" />
        Loading schedule status...
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">Scheduled Cleanup</h1>
          <p className="text-slate-400 mt-1">Automate cleanup with macOS LaunchAgent</p>
        </div>
        <button
          onClick={handleRunNow}
          disabled={runningNow}
          className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 text-white font-medium transition-all disabled:opacity-50"
        >
          {runningNow ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
          {runningNow ? 'Running...' : 'Run Now'}
        </button>
      </div>

      {/* Run result */}
      {runResult && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="glass-card p-4 rounded-2xl"
        >
          <div className="flex items-center gap-3">
            <CheckCircle className="w-5 h-5 text-emerald-400" />
            <span className="text-white font-medium">
              {formatBytes(runResult.result.bytes_freed)} freed
            </span>
            <span className="text-slate-400 text-sm">
              {runResult.result.items_removed} items &middot; {runResult.tasks_run.join(', ')}
            </span>
          </div>
          {runResult.result.errors.length > 0 && (
            <div className="flex items-center gap-2 mt-2 text-sm text-danger-400">
              <AlertCircle className="w-4 h-4" />
              {runResult.result.errors.length} error(s)
            </div>
          )}
        </motion.div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Toggle + Status */}
        <div className="glass-card p-6 rounded-2xl space-y-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className={clsx(
                'p-3 rounded-xl border',
                status?.enabled
                  ? 'bg-emerald-500/10 border-emerald-500/20'
                  : 'bg-slate-800 border-white/5'
              )}>
                <Power className={clsx('w-5 h-5', status?.enabled ? 'text-emerald-400' : 'text-slate-500')} />
              </div>
              <div>
                <h3 className="font-semibold text-white">Auto Cleanup</h3>
                <span className={clsx('text-xs', status?.enabled ? 'text-emerald-400' : 'text-slate-500')}>
                  {status?.enabled ? 'Enabled' : 'Disabled'}
                </span>
              </div>
            </div>
            <button
              onClick={handleToggle}
              disabled={saving}
              className={clsx(
                'relative w-14 h-7 rounded-full transition-all',
                status?.enabled ? 'bg-emerald-500' : 'bg-white/10'
              )}
            >
              <div className={clsx(
                'absolute top-0.5 w-6 h-6 rounded-full bg-white shadow transition-all',
                status?.enabled ? 'left-7' : 'left-0.5'
              )} />
            </button>
          </div>

          {/* Status info */}
          {status && (
            <div className="space-y-3">
              <div className="flex items-center justify-between text-sm">
                <span className="text-slate-400">LaunchAgent</span>
                <span className={clsx(
                  'text-xs px-2 py-0.5 rounded',
                  status.is_loaded ? 'bg-emerald-500/20 text-emerald-400' : 'bg-white/5 text-slate-500'
                )}>
                  {status.is_loaded ? 'Loaded' : 'Not loaded'}
                </span>
              </div>
              {status.last_run && (
                <div className="flex items-center justify-between text-sm">
                  <span className="text-slate-400">Last run</span>
                  <span className="text-white">{status.last_run}</span>
                </div>
              )}
              {status.next_run && (
                <div className="flex items-center justify-between text-sm">
                  <span className="text-slate-400">Next run</span>
                  <span className="text-white">{status.next_run}</span>
                </div>
              )}
              <div className="flex items-center justify-between text-sm">
                <span className="text-slate-400">Plist path</span>
                <span className="text-slate-500 text-xs truncate ml-4 max-w-[200px]">{status.plist_path}</span>
              </div>
            </div>
          )}
        </div>

        {/* Schedule selection */}
        <div className="glass-card p-6 rounded-2xl space-y-4">
          <h3 className="font-semibold text-white flex items-center gap-2">
            <Calendar className="w-4 h-4 text-brand-400" />
            Schedule
          </h3>
          <div className="space-y-2">
            {cronPresets.map((preset) => (
              <label
                key={preset.value}
                className={clsx(
                  'flex items-center gap-3 p-3 rounded-xl cursor-pointer transition-all',
                  selectedCron === preset.value
                    ? 'bg-brand-500/10 border border-brand-500/30'
                    : 'bg-white/5 border border-transparent hover:bg-white/10'
                )}
              >
                <input
                  type="radio"
                  name="cron"
                  value={preset.value}
                  checked={selectedCron === preset.value}
                  onChange={() => setSelectedCron(preset.value)}
                  className="text-brand-500 focus:ring-brand-500/50"
                />
                <div>
                  <span className="text-sm text-white font-medium">{preset.label}</span>
                  <p className="text-xs text-slate-500">{preset.description}</p>
                </div>
              </label>
            ))}
          </div>
        </div>
      </div>

      {/* Tasks */}
      <div className="glass-card p-6 rounded-2xl space-y-4">
        <h3 className="font-semibold text-white flex items-center gap-2">
          <Clock className="w-4 h-4 text-brand-400" />
          Cleanup Tasks
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {taskLabels.map(({ key, label, description }) => (
            <label
              key={key}
              className={clsx(
                'flex items-center gap-3 p-3 rounded-xl cursor-pointer transition-all',
                tasks[key]
                  ? 'bg-brand-500/10 border border-brand-500/30'
                  : 'bg-white/5 border border-transparent hover:bg-white/10'
              )}
            >
              <input
                type="checkbox"
                checked={tasks[key]}
                onChange={() => toggleTask(key)}
                className="rounded border-white/20 bg-white/10 text-brand-500 focus:ring-brand-500/50"
              />
              <div>
                <span className="text-sm text-white font-medium">{label}</span>
                <p className="text-xs text-slate-500">{description}</p>
              </div>
            </label>
          ))}
        </div>

        {/* Save button */}
        <div className="pt-4 border-t border-white/5">
          <button
            onClick={handleSave}
            disabled={saving}
            className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 text-white font-medium transition-all disabled:opacity-50"
          >
            {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <CheckCircle className="w-4 h-4" />}
            Save & Enable Schedule
          </button>
        </div>
      </div>
    </div>
  );
}
