'use client';

import { useEffect, useState } from 'react';
import { Terminal, Download, Check, Copy, Loader2 } from 'lucide-react';
import { getCliStatus, installCli, type CliStatus } from '@/lib/tauri';
import { toMessage } from '@/lib/errors';

const CARGO_CMD = 'cargo install --git https://github.com/suenot/macpurge macpurge';

export function CliInstall() {
  const [status, setStatus] = useState<CliStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    getCliStatus().then(setStatus).catch((e) => setError(toMessage(e)));
  }, []);

  const install = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await installCli());
    } catch (e) {
      setError(toMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(text);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      /* clipboard unavailable */
    }
  };

  const dir = status?.path ? status.path.replace(/\/macpurge$/, '') : '';
  const exportLine = `export PATH="${dir}:$PATH"`;

  return (
    <section className="glass-card p-6 rounded-2xl">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-white mb-2 flex items-center gap-2">
            <Terminal className="w-5 h-5 text-brand-400" />
            Command line tool
          </h2>
          <p className="text-sm text-slate-400 max-w-xl">
            Use the same scanner and cleaner from your terminal: <code className="text-sand-300">macpurge scan-dev</code>,{' '}
            <code className="text-sand-300">macpurge clean ... --dry-run</code> and more.
          </p>
        </div>
        <button
          onClick={install}
          disabled={busy || status?.bundled === false}
          className="shrink-0 px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl flex items-center gap-2 font-medium transition-all shadow-lg shadow-brand-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {busy ? <Loader2 className="w-4 h-4 animate-spin" /> : status?.installed ? <Check className="w-4 h-4" /> : <Download className="w-4 h-4" />}
          {status?.installed ? 'Reinstall' : 'Install CLI'}
        </button>
      </div>

      {status?.installed && (
        <div className="mt-4 text-sm text-slate-300 space-y-2">
          <p>
            Installed at <code className="text-sand-300">{status.path}</code>
            {status.version && <span className="text-slate-500"> · {status.version}</span>}
          </p>
          {!status.on_path && (
            <div className="p-3 rounded-xl bg-yellow-500/10 border border-yellow-500/20 text-yellow-200/90">
              <p className="mb-2"><code>{dir}</code> is not on your PATH. Add this line to <code>~/.zshrc</code>:</p>
              <button onClick={() => copy(exportLine)} className="flex items-center gap-2 font-mono text-xs bg-black/30 px-3 py-2 rounded-lg hover:bg-black/50">
                {exportLine} {copied === exportLine ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
              </button>
            </div>
          )}
        </div>
      )}

      {error && <p className="mt-4 text-sm text-danger-400">{error}</p>}

      <div className="mt-5 pt-5 border-t border-white/5">
        <p className="text-xs text-slate-500 mb-2">Or install with Cargo:</p>
        <button onClick={() => copy(CARGO_CMD)} className="flex items-center gap-2 font-mono text-xs text-slate-300 bg-black/30 px-3 py-2 rounded-lg hover:bg-black/50 transition-colors text-left">
          {CARGO_CMD} {copied === CARGO_CMD ? <Check className="w-3 h-3 shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
        </button>
      </div>
    </section>
  );
}
