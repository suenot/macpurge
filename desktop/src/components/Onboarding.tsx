'use client';

import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Shield,
  HardDrive,
  FolderCheck,
  Check,
  X,
  AlertCircle,
  Sparkles,
  RefreshCw,
} from 'lucide-react';
import { clsx } from 'clsx';
import {
  getConfig,
  setOnboardingCompleted,
  checkFullDiskAccess,
  openFullDiskAccessSettings,
  requestStandardFolders,
  isTauri,
} from '@/lib/tauri';
import type { FolderAccess } from '@/lib/types';
import { toMessage } from '@/lib/errors';

// First-launch overlay that walks the user through granting disk permissions.
// Shown once: gated on the persisted `onboarding_completed` config flag. After
// the user clicks "Get started" or "Skip for now" we set the flag so it never
// reappears. Renders nothing outside of Tauri (e.g. browser dev preview).
export default function Onboarding() {
  const [visible, setVisible] = useState(false);
  const [fdaGranted, setFdaGranted] = useState<boolean | null>(null);
  const [folders, setFolders] = useState<FolderAccess[] | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [isRequesting, setIsRequesting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    let mounted = true;

    (async () => {
      try {
        const cfg = await getConfig();
        if (!mounted) return;
        if (cfg.onboarding_completed) return;
        setVisible(true);
        // Kick off an initial Full Disk Access check so status is fresh.
        const granted = await checkFullDiskAccess();
        if (mounted) setFdaGranted(granted);
      } catch (err) {
        if (mounted) setError(toMessage(err));
      }
    })();

    return () => {
      mounted = false;
    };
  }, []);

  const handleRecheck = async () => {
    setIsChecking(true);
    setError(null);
    try {
      const granted = await checkFullDiskAccess();
      setFdaGranted(granted);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setIsChecking(false);
    }
  };

  const handleOpenSettings = async () => {
    setError(null);
    try {
      await openFullDiskAccessSettings();
    } catch (err) {
      setError(toMessage(err));
    }
  };

  const handleRequestFolders = async () => {
    setIsRequesting(true);
    setError(null);
    try {
      const results = await requestStandardFolders();
      setFolders(results);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setIsRequesting(false);
    }
  };

  const dismiss = async () => {
    // Persist first so the overlay does not reappear, then hide regardless.
    try {
      await setOnboardingCompleted(true);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setVisible(false);
    }
  };

  if (!isTauri()) return null;

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-[60] flex items-center justify-center bg-black/70 backdrop-blur-md p-6 bg-gradient-glow"
        >
          <motion.div
            initial={{ opacity: 0, y: 16, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 16, scale: 0.98 }}
            transition={{ type: 'spring', stiffness: 260, damping: 26 }}
            className="glass-card w-full max-w-lg rounded-2xl border border-white/10 p-8 max-h-[90vh] overflow-y-auto custom-scrollbar"
          >
            {/* Header */}
            <div className="flex items-center gap-3 mb-2">
              <div className="relative">
                <div className="absolute inset-0 bg-brand-500 blur-lg opacity-40" />
                <div className="relative w-11 h-11 rounded-xl bg-gradient-to-br from-brand-500 to-brand-700 flex items-center justify-center shadow-lg shadow-brand-500/20">
                  <Sparkles className="w-5 h-5 text-white" />
                </div>
              </div>
              <div>
                <h2 className="text-2xl font-bold text-white tracking-tight">Welcome to MacPurge</h2>
                <p className="text-sm text-slate-400">Let&apos;s set up disk access</p>
              </div>
            </div>

            <p className="text-sm text-slate-300 mt-4 leading-relaxed">
              MacPurge needs access to your disk to find space you can reclaim. Granting
              <span className="text-white font-medium"> Full Disk Access</span> once means macOS
              won&apos;t prompt you over and over while scanning.
            </p>

            {/* Error */}
            {error && (
              <div className="mt-5 p-3 rounded-xl border border-danger-500/30 bg-danger-500/10 flex items-center gap-2">
                <AlertCircle className="w-4 h-4 text-danger-400 flex-shrink-0" />
                <span className="text-danger-200 text-sm">{error}</span>
                <button onClick={() => setError(null)} className="ml-auto hover:text-white">
                  <X className="w-4 h-4" />
                </button>
              </div>
            )}

            {/* Full Disk Access */}
            <section className="mt-6 p-5 rounded-xl bg-white/5 border border-white/5">
              <div className="flex items-center gap-3 mb-3">
                <Shield className="w-5 h-5 text-brand-400 flex-shrink-0" />
                <h3 className="text-base font-semibold text-white">Full Disk Access</h3>
                <span
                  className={clsx(
                    'ml-auto px-3 py-1 rounded-full text-xs font-semibold border',
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
              <p className="text-xs text-slate-400 mb-4">
                Opens System Settings → Privacy &amp; Security → Full Disk Access. Toggle MacPurge on,
                then return here and recheck.
              </p>
              <div className="flex gap-3">
                <button
                  onClick={handleOpenSettings}
                  className="px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-medium transition-all shadow-lg shadow-brand-500/20"
                >
                  <Shield className="w-4 h-4" />
                  Grant Full Disk Access
                </button>
                <button
                  onClick={handleRecheck}
                  disabled={isChecking}
                  className={clsx(
                    'px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl flex items-center gap-2 font-medium transition-all',
                    isChecking && 'opacity-50 cursor-not-allowed'
                  )}
                >
                  <RefreshCw className={clsx('w-4 h-4', isChecking && 'animate-spin')} />
                  Recheck
                </button>
              </div>
            </section>

            {/* Standard folders */}
            <section className="mt-4 p-5 rounded-xl bg-white/5 border border-white/5">
              <div className="flex items-center gap-3 mb-3">
                <HardDrive className="w-5 h-5 text-brand-400 flex-shrink-0" />
                <h3 className="text-base font-semibold text-white">Folder access</h3>
              </div>
              <p className="text-xs text-slate-400 mb-4">
                If you&apos;d rather not grant full access, you can allow Downloads, Desktop, and
                Documents individually. macOS will prompt for each.
              </p>
              <button
                onClick={handleRequestFolders}
                disabled={isRequesting}
                className={clsx(
                  'px-4 py-2 bg-white/5 hover:bg-white/10 text-white rounded-xl flex items-center gap-2 font-medium transition-all',
                  isRequesting && 'opacity-50 cursor-not-allowed'
                )}
              >
                <FolderCheck className="w-4 h-4" />
                Grant folder access
              </button>

              {folders && folders.length > 0 && (
                <div className="mt-4 space-y-2">
                  {folders.map((f) => (
                    <div
                      key={f.path}
                      className="flex items-center justify-between py-2 px-3 rounded-lg bg-black/20 border border-white/5"
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

            {/* Actions */}
            <div className="flex items-center justify-between mt-8">
              <button
                onClick={dismiss}
                className="text-sm text-slate-500 hover:text-slate-300 transition-colors"
              >
                Skip for now
              </button>
              <button
                onClick={dismiss}
                className="px-6 py-2.5 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-semibold transition-all shadow-lg shadow-brand-500/20"
              >
                <Check className="w-4 h-4" />
                Get started
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
