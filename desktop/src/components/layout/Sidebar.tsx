'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  LayoutDashboard,
  Code2,
  AppWindow,
  Wrench,
  Sparkles,
  Settings,
  HardDrive,
  Package,
  Usb,
  Wifi,
  Database,
  Clock,
  Shield,
  FolderGit2,
  BrainCircuit
} from 'lucide-react';
import { clsx } from 'clsx';
import { useEffect, useState } from 'react';
import { getUserDisks, onDisksChanged, isTauri } from '@/lib/tauri';
import type { DiskInfo, DiskType } from '@/lib/types';
import { formatBytes } from '@/lib/types';

const navItems = [
  { href: '/', label: 'Dashboard', icon: LayoutDashboard },
  { href: '/projects', label: 'Projects', icon: FolderGit2 },
  { href: '/dev', label: 'Dev Artifacts', icon: Code2 },
  { href: '/packages', label: 'Packages', icon: Package },
  { href: '/system', label: 'System', icon: Shield },
  { href: '/apps', label: 'Applications', icon: AppWindow },
  { href: '/devtools', label: 'Dev Tools', icon: Wrench },
  { href: '/ai-models', label: 'AI Models', icon: BrainCircuit },
  { href: '/recipes', label: 'Recipes', icon: Sparkles },
  { href: '/schedule', label: 'Schedule', icon: Clock },
  { href: '/settings', label: 'Settings', icon: Settings },
];

function diskIcon(type: DiskType) {
  switch (type) {
    case 'system': return HardDrive;
    case 'internal': return Database;
    case 'external': return Usb;
    case 'network': return Wifi;
  }
}

function diskTypeLabel(type: DiskType): string {
  switch (type) {
    case 'system': return 'System';
    case 'internal': return 'Internal';
    case 'external': return 'External';
    case 'network': return 'Network';
  }
}

export function Sidebar() {
  const pathname = usePathname();
  const [disks, setDisks] = useState<DiskInfo[]>([]);

  useEffect(() => {
    if (!isTauri()) return;

    let mounted = true;
    let unlisten: (() => void) | undefined;

    const fetchDisks = async () => {
      try {
        const data = await getUserDisks();
        if (!mounted) return;
        // Sort: system disk first, then by name
        data.sort((a, b) => {
          if (a.disk_type === 'system') return -1;
          if (b.disk_type === 'system') return 1;
          return a.name.localeCompare(b.name);
        });
        setDisks(data);
      } catch (err) {
        console.error('Failed to get disks:', err);
      }
    };

    fetchDisks();

    (async () => {
      // Listen for disk change events (mount/unmount/space updates)
      unlisten = await onDisksChanged(() => {
        if (!mounted) return;
        fetchDisks();
      });
      // Unmounted while `await onDisksChanged` was in flight.
      if (!mounted) { unlisten?.(); unlisten = undefined; }
    })();

    // Periodic fallback for space changes
    const interval = setInterval(fetchDisks, 30000);

    return () => {
      mounted = false;
      clearInterval(interval);
      unlisten?.();
    };
  }, []);

  return (
    <aside className="w-64 glass-heavy flex flex-col z-20 h-full">
      {/* Logo */}
      <div className="p-6">
        <div className="flex items-center gap-3">
          <div className="relative group">
            <div className="absolute inset-0 bg-brand-500 blur-lg opacity-40 group-hover:opacity-60 transition-opacity" />
            <div className="relative w-10 h-10 rounded-xl bg-gradient-to-br from-brand-500 to-brand-700 flex items-center justify-center shadow-lg shadow-brand-500/20">
              <HardDrive className="w-5 h-5 text-white" />
            </div>
          </div>
          <div>
            <h1 className="font-bold text-lg text-white tracking-wide">MacPurge</h1>
            <p className="text-xs text-slate-400">System Cleaner</p>
          </div>
        </div>
      </div>

      {/* Disk Status */}
      {disks.length > 0 && (
        <div className="px-4 mb-6 space-y-2">
          {disks.map((disk) => {
            const Icon = diskIcon(disk.disk_type);
            const availableGb = disk.available_bytes / (1024 * 1024 * 1024);
            const totalGb = disk.total_bytes / (1024 * 1024 * 1024);
            const isLow = availableGb < 10;
            const usage = disk.usage_percent ?? 0;

            return (
              <div
                key={disk.mount_point}
                className="p-3 rounded-xl bg-white/5 border border-white/5 backdrop-blur-sm"
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-1.5 min-w-0">
                    <Icon className="w-3 h-3 text-slate-500 shrink-0" />
                    <span className="text-xs font-medium text-slate-400 truncate">
                      {disk.name}
                    </span>
                    {disk.disk_type !== 'system' && (
                      <span className="text-[9px] text-slate-600 shrink-0">
                        {diskTypeLabel(disk.disk_type)}
                      </span>
                    )}
                  </div>
                  <span className={clsx(
                    'text-xs font-bold shrink-0 ml-2',
                    isLow ? 'text-danger-400' : 'text-emerald-400'
                  )}>
                    {formatBytes(disk.available_bytes)}
                  </span>
                </div>
                <div className="h-1.5 bg-white/10 rounded-full overflow-hidden">
                  <div
                    className={clsx(
                      'h-full rounded-full transition-all duration-1000 ease-out relative',
                      usage > 90
                        ? 'bg-danger-500'
                        : usage > 70
                          ? 'bg-yellow-500'
                          : 'bg-gradient-to-r from-emerald-500 to-brand-500'
                    )}
                    style={{ width: `${usage}%` }}
                  >
                    <div className="absolute inset-0 bg-white/20 animate-pulse" />
                  </div>
                </div>
                <div className="flex justify-between mt-1.5">
                  <span className="text-[10px] text-slate-500">{usage.toFixed(1)}% used</span>
                  <span className="text-[10px] text-slate-500">{totalGb.toFixed(0)} GB</span>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Navigation */}
      <nav className="flex-1 px-3 space-y-1 overflow-y-auto custom-scrollbar">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = pathname === item.href;

          return (
            <Link
              key={item.href}
              href={item.href}
              className={clsx(
                'flex items-center gap-3 px-4 py-3 rounded-xl transition-all duration-200 group relative',
                isActive
                  ? 'bg-brand-500/10 text-white'
                  : 'text-slate-400 hover:text-white hover:bg-white/5'
              )}
            >
              {isActive && (
                <div className="absolute left-0 w-1 h-6 bg-brand-500 rounded-r-full shadow-[0_0_10px_rgba(139,92,246,0.5)]" />
              )}
              <Icon className={clsx(
                "w-5 h-5 transition-colors",
                isActive ? "text-brand-400" : "group-hover:text-brand-300"
              )} />
              <span className="font-medium text-sm tracking-wide">{item.label}</span>
            </Link>
          );
        })}
      </nav>

      {/* Version */}
      <div className="p-4 border-t border-white/5">
        <div className="flex items-center justify-between text-[10px] text-slate-600">
          <span>v0.1.0</span>
          <span>MacPurge Desktop</span>
        </div>
      </div>
    </aside>
  );
}
