// In-memory mock of the Tauri backend, used only by the website demo build.
/* eslint-disable @typescript-eslint/no-explicit-any */

const GB = 1024 ** 3;
const MB = 1024 ** 2;
const HOME = '/Users/dev';
const NOW = Math.floor(Date.now() / 1000);
const DAY = 86400;

type Handler = (e: { payload: any }) => void;
const listeners = new Map<string, Set<Handler>>();

export function mockListen<T>(event: string, handler: (e: { payload: T }) => void): () => void {
  const set = listeners.get(event) ?? new Set();
  set.add(handler as Handler);
  listeners.set(event, set);
  return () => set.delete(handler as Handler);
}

function emit(event: string, payload: unknown, delay = 0) {
  setTimeout(() => listeners.get(event)?.forEach((h) => h({ payload })), delay);
}
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

// ---------- state ----------
type Entry = { path: string; file_type: string; size_bytes: number; file_count: number; last_modified: number | null };
const devEntries: Entry[] = [
  ['web-shop/node_modules', 'node_modules', 2.4, 41120, 12],
  ['dashboard/node_modules', 'node_modules', 1.8, 33210, 40],
  ['mobile-app/node_modules', 'node_modules', 1.5, 28900, 90],
  ['engine/target', 'target', 18.9, 9120, 3],
  ['cli-tools/target', 'target', 6.1, 3100, 25],
  ['dashboard/.next', '.next', 1.1, 880, 5],
  ['api/.venv', 'venv', 3.2, 21400, 60],
  ['ios-app/Pods', 'Pods', 1.7, 6400, 45],
  ['game/Library', 'Library', 6.8, 52000, 120],
  ['ml-lab/.venv', 'venv', 4.4, 19000, 150],
].map(([p, t, gb, n, d]) => ({
  path: `${HOME}/projects/${p}`, file_type: t as string, size_bytes: Math.round((gb as number) * GB), file_count: n as number, last_modified: NOW - (d as number) * DAY,
}));

const pkgDef: [string, [string, number, string][]][] = [
  ['npm', [['~/.npm/_cacache', 4.6, 'npm content cache']]],
  ['pnpm', [['~/Library/pnpm/store', 9.2, 'pnpm store']]],
  ['cargo', [['~/.cargo/registry', 7.3, 'Downloaded crates'], ['~/.cargo/git', 1.2, 'Git checkouts']]],
  ['pip', [['~/Library/Caches/pip', 1.9, 'pip wheel cache']]],
  ['brew', [['~/Library/Caches/Homebrew', 3.4, 'Homebrew downloads']]],
  ['go', [['~/go/pkg/mod/cache', 2.6, 'Go module cache']]],
];
let pkgs = pkgDef.map(([m, paths]) => ({
  manager: m,
  is_installed: true,
  cache_paths: paths.map(([p, gb, d]) => ({ path: p.replace('~', HOME), size_bytes: Math.round(gb * GB), description: d, safe_to_clean: true })),
  total_cache_size: 0,
  global_packages: [],
}));

const sysDef: [string, string, string, [string, number, string][]][] = [
  ['user_cache', 'User Caches', 'App caches in ~/Library/Caches', [['~/Library/Caches/Google', 3.8, 'Chrome'], ['~/Library/Caches/com.apple.Safari', 1.2, 'Safari'], ['~/Library/Caches/Spotify', 2.1, 'Spotify'], ['~/Library/Caches/Slack', 1.4, 'Slack']]],
  ['browser_cache', 'Browser Caches', 'Cached web content', [['~/Library/Caches/Firefox', 2.2, 'Firefox']]],
  ['system_logs', 'System Logs', 'Log files', [['~/Library/Logs', 0.9, 'App logs']]],
  ['ios_backups', 'iOS Backups', 'Old device backups', [['~/Library/Application Support/MobileSync/Backup', 21.6, 'iPhone backup']]],
  ['downloads_old', 'Old Downloads', 'Files older than 90 days', [['~/Downloads', 8.1, 'Installers and archives']]],
  ['trash', 'Trash', 'Items already in the Trash', [['~/.Trash', 2.3, 'Trash']]],
];
let sysCats = sysDef.map(([c, name, d, paths]) => ({
  category: c, name, description: d, size_bytes: 0, item_count: paths.length,
  paths: paths.map(([p, gb, desc]) => ({ path: p.replace('~', HOME), size_bytes: Math.round(gb * GB), description: desc })),
}));

let devtools: any[] = [
  { tool: 'xcode', is_installed: true, cache_paths: [
    { path: `${HOME}/Library/Developer/Xcode/DerivedData`, size_bytes: 41 * GB, description: 'Build products', category: 'DerivedData', safe_to_clean: true },
    { path: `${HOME}/Library/Developer/CoreSimulator/Devices`, size_bytes: 38 * GB, description: 'Simulator devices', category: 'Simulators', safe_to_clean: true },
    { path: `${HOME}/Library/Developer/Xcode/Archives`, size_bytes: 19 * GB, description: 'App archives', category: 'Archives', safe_to_clean: true },
  ], total_size: 0 },
  { tool: 'docker', is_installed: true, cache_paths: [
    { path: `${HOME}/Library/Containers/com.docker.docker/Data/vms`, size_bytes: Math.round(27.5 * GB), description: 'Docker VM disk image', category: 'Images', safe_to_clean: true },
  ], total_size: 0 },
  { tool: 'android_studio', is_installed: true, cache_paths: [
    { path: `${HOME}/.android/avd`, size_bytes: Math.round(9.4 * GB), description: 'Virtual devices', category: 'Emulators', safe_to_clean: true },
    { path: `${HOME}/.gradle/caches`, size_bytes: Math.round(5.2 * GB), description: 'Gradle caches', category: 'Gradle', safe_to_clean: true },
  ], total_size: 0 },
];

let aiTools: any[] = [
  { tool: 'ollama', is_installed: true, models: [
    { path: `${HOME}/.ollama/models/llama3.1-70b`, size_bytes: Math.round(39.6 * GB), description: 'llama3.1:70b', category: 'Model', safe_to_clean: true },
    { path: `${HOME}/.ollama/models/mistral-7b`, size_bytes: Math.round(4.1 * GB), description: 'mistral:7b', category: 'Model', safe_to_clean: true },
  ], total_size: 0 },
  { tool: 'comfy_ui', is_installed: true, models: [
    { path: `${HOME}/ComfyUI/models/checkpoints/flux1-dev.safetensors`, size_bytes: Math.round(22.2 * GB), description: 'flux1-dev', category: 'Checkpoint', safe_to_clean: true },
  ], total_size: 0 },
  { tool: 'hugging_face', is_installed: true, models: [
    { path: `${HOME}/.cache/huggingface/hub/models--meta-llama--Llama-3-8B`, size_bytes: 15 * GB, description: 'Llama-3-8B', category: 'Model', safe_to_clean: true },
  ], total_size: 0 },
  { tool: 'lm_studio', is_installed: false, models: [], total_size: 0 },
];

let apps: any[] = [
  ['Xcode', 'com.apple.dt.Xcode', '16.1', 14.2, 3.1, 1.2], ['Docker', 'com.docker.docker', '4.35', 1.1, 27.5, 0.6],
  ['Figma', 'com.figma.Desktop', '124.2', 0.6, 0.9, 1.4], ['Slack', 'com.tinyspeck.slackmacgap', '4.41', 0.4, 0.3, 1.4],
  ['Spotify', 'com.spotify.client', '1.2.5', 0.3, 0.2, 2.1], ['Visual Studio Code', 'com.microsoft.VSCode', '1.95', 0.4, 0.5, 0.8],
  ['Android Studio', 'com.google.android.studio', '2024.2', 3.3, 14.6, 1.1], ['Final Cut Pro', 'com.apple.FinalCut', '10.8', 4.9, 6.2, 0.4],
].map(([name, id, v, app, data, cache]: any) => ({
  name, path: `/Applications/${name}.app`, bundle_id: id, version: v,
  size_bytes: Math.round(app * GB), data_size_bytes: Math.round(data * GB), cache_size_bytes: Math.round(cache * GB), total_size_bytes: Math.round((app + data + cache) * GB),
}));

const projectDefs: [string, string, string, number, number, number | null, boolean][] = [
  // name, group, type, sizeGB, artifactGB, lastCommitDays, git
  ['web-shop', 'work', 'Node', 4.1, 2.4, 2, true], ['dashboard', 'work', 'Node', 3.2, 2.9, 5, true], ['api', 'work', 'Python', 3.9, 3.2, 1, true],
  ['mobile-app', 'work', 'Node', 2.4, 1.5, 90, true], ['engine', 'personal', 'Rust', 21.4, 18.9, 3, true], ['cli-tools', 'personal', 'Rust', 6.8, 6.1, 25, true],
  ['game', 'personal', 'Unknown', 9.2, 6.8, 120, true], ['ios-app', 'personal', 'Swift', 2.6, 1.7, 45, true], ['ml-lab', '', 'Python', 5.1, 4.4, 150, true],
  ['dotfiles', '', 'Unknown', 0.01, 0, 8, true],
];
let projects = projectDefs.map(([name, group, type, gb, art, days, git]) => ({
  name, group, path: `${HOME}/projects/${group ? group + '/' : ''}${name}`, size_bytes: Math.round(gb * GB), last_modified: NOW - ((days ?? 0) * DAY),
  project_type: type, has_git: git, last_commit_time: days == null ? null : NOW - days * DAY, artifact_size: Math.round(art * GB),
}));
let archived: any[] = [];

const recipes = [
  { id: 'dev-weekly', name: 'Weekly dev cleanup', description: 'Build artifacts older than 30 days, package caches and the Trash.', category: 'Developer', estimated_savings: '30-60 GB', steps: [{ action: {}, description: 'Remove stale node_modules and target folders' }, { action: {}, description: 'Clear npm, cargo and pip caches' }, { action: {}, description: 'Empty the Trash' }] },
  { id: 'xcode-deep', name: 'Xcode deep clean', description: 'DerivedData, old simulators and archives.', category: 'Xcode', estimated_savings: '40-90 GB', steps: [{ action: {}, description: 'Delete DerivedData' }, { action: {}, description: 'Remove unavailable simulators' }, { action: {}, description: 'Clear old archives' }] },
  { id: 'docker-prune', name: 'Docker prune', description: 'Dangling images, stopped containers and build cache.', category: 'Docker', estimated_savings: '10-30 GB', steps: [{ action: {}, description: 'Prune images' }, { action: {}, description: 'Prune build cache' }] },
  { id: 'ai-models', name: 'Unused AI models', description: 'Local models you have not touched in 60 days.', category: 'AI', estimated_savings: '20-80 GB', steps: [{ action: {}, description: 'List models by last use' }, { action: {}, description: 'Remove stale models' }] },
];

let schedule = {
  enabled: false, cron: '0 3 * * 0', plist_path: `${HOME}/Library/LaunchAgents/com.macpurge.schedule.plist`, plist_exists: false, is_loaded: false,
  tasks: { clean_dev: true, clean_packages: true, clean_docker: false, clean_system: true, clean_browsers: true, empty_trash: true },
  last_run: null as string | null, next_run: null as string | null,
};

let config: any = {
  config_path: `${HOME}/.config/macpurge/config.toml`,
  project_dirs: [{ path: '~/projects', expanded: `${HOME}/projects`, exists: true }],
  excluded_dirs: [{ path: '~/Library/Mobile Documents', expanded: `${HOME}/Library/Mobile Documents`, exists: true }],
  scan: { max_depth: 5, include_hidden: false, artifact_types: [] },
  cleanup: { confirm_before_delete: true, use_trash: true, log_deletions: true, older_than_days: null },
  schedule_enabled: false, archive_dir: '~/Archive', onboarding_completed: true,
};

// ---------- usage tree ----------
type U = { name: string; path: string; size_bytes: number; file_count: number; is_dir: boolean; is_symlink: boolean; children: U[]; has_more: boolean; aggregated_count: number | null };
const dir = (name: string, parent: string, children: U[], files = 0): U => {
  const path = `${parent}/${name}`;
  const kids = children.map((c) => ({ ...c, path: `${path}/${c.name}`, children: reparent(c.children, `${path}/${c.name}`) }));
  return { name, path, size_bytes: kids.reduce((a, c) => a + c.size_bytes, 0), file_count: kids.reduce((a, c) => a + c.file_count, files), is_dir: true, is_symlink: false, children: kids, has_more: false, aggregated_count: null };
};
const reparent = (cs: U[], parent: string): U[] => cs.map((c) => ({ ...c, path: `${parent}/${c.name}`, children: reparent(c.children, `${parent}/${c.name}`) }));
const file = (name: string, gb: number, n = 1): U => ({ name, path: name, size_bytes: Math.round(gb * GB), file_count: n, is_dir: false, is_symlink: false, children: [], has_more: false, aggregated_count: null });
const leaf = (name: string, gb: number, n: number): U => ({ ...file(name, gb, n), is_dir: true });

function buildUsage(): U {
  const kids = [
    dir('Library', HOME, [
      dir('Developer', '', [leaf('Xcode', 41, 310000), leaf('CoreSimulator', 38, 220000), leaf('Archives', 19, 9000)]),
      leaf('Application Support', 64, 480000), leaf('Caches', 52, 610000), leaf('Containers', 31, 120000),
    ]),
    dir('projects', HOME, [leaf('node_modules (all)', 61, 940000), leaf('target (all)', 47, 31000), leaf('.next (all)', 22, 14000), leaf('sources', 38, 220000)]),
    leaf('Movies', 96, 640), leaf('Downloads', 74, 2100), leaf('.ollama', 58, 40), leaf('Documents', 41, 9800), leaf('Pictures', 27, 14000), leaf('Desktop', 11, 410),
  ];
  const t = dir('dev', '/Users', kids);
  t.path = HOME;
  t.children = reparent(kids as U[], HOME);
  return t;
}
let usageTree = buildUsage();

function findNode(n: U, path: string): U | null {
  if (n.path === path) return n;
  for (const c of n.children) { const f = findNode(c, path); if (f) return f; }
  return null;
}
function removeNode(n: U, path: string): number {
  const i = n.children.findIndex((c) => c.path === path);
  if (i >= 0) { const [r] = n.children.splice(i, 1); recalc(usageTree); return r.size_bytes; }
  for (const c of n.children) { const s = removeNode(c, path); if (s) return s; }
  return 0;
}
function recalc(n: U): number {
  if (n.children.length) { n.size_bytes = n.children.reduce((a, c) => a + recalc(c), 0); n.file_count = n.children.reduce((a, c) => a + c.file_count, 0); }
  return n.size_bytes;
}
const clone = <T,>(x: T): T => JSON.parse(JSON.stringify(x));

// ---------- helpers ----------
const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
const human = (b: number) => (b >= GB ? (b / GB).toFixed(2) + ' GB' : (b / MB).toFixed(1) + ' MB');
const cleanRes = (paths: string[], bytes: number) => ({ items_removed: paths.length, bytes_freed: bytes, errors: [] as string[], removed_paths: paths });

function removeFromAll(path: string): number {
  let freed = 0;
  const di = devEntries.findIndex((e) => e.path === path);
  if (di >= 0) freed += devEntries.splice(di, 1)[0].size_bytes;
  for (const m of pkgs) { const i = m.cache_paths.findIndex((c) => c.path === path); if (i >= 0) freed += m.cache_paths.splice(i, 1)[0].size_bytes; }
  for (const c of sysCats) { const i = c.paths.findIndex((p) => p.path === path); if (i >= 0) freed += c.paths.splice(i, 1)[0].size_bytes; }
  for (const t of devtools) { const i = t.cache_paths.findIndex((p: any) => p.path === path); if (i >= 0) freed += t.cache_paths.splice(i, 1)[0].size_bytes; }
  for (const t of aiTools) { const i = t.models.findIndex((p: any) => p.path === path); if (i >= 0) freed += t.models.splice(i, 1)[0].size_bytes; }
  return freed;
}

let freedTotal = 0;
const disk = () => {
  const total = 994 * GB, used = 640 * GB - freedTotal;
  return { name: 'Macintosh HD', mount_point: '/', total_bytes: total, available_bytes: total - used - 42 * GB, used_bytes: used + 42 * GB, usage_percent: ((used + 42 * GB) / total) * 100, disk_type: 'system', fs_type: 'apfs' };
};

// ---------- dispatcher ----------
export async function mockInvoke<T>(cmd: string, a: Record<string, any>): Promise<T> {
  await wait(120 + Math.random() * 180);
  const r = await handle(cmd, a);
  return r as T;
}

async function handle(cmd: string, a: Record<string, any>): Promise<unknown> {
  switch (cmd) {
    case 'get_disk_info': case 'get_user_disks': return [disk()];
    case 'get_free_space': return disk();
    case 'get_free_space_human': { const d = disk(); return { available_bytes: d.available_bytes, total_bytes: d.total_bytes, available_gb: d.available_bytes / GB, total_gb: d.total_bytes / GB, available_human: human(d.available_bytes), usage_percent: d.usage_percent, is_low: false }; }

    case 'scan_dev': case 'scan_path': {
      await wait(900);
      const by: Record<string, Entry[]> = {};
      devEntries.forEach((e) => (by[e.file_type] ??= []).push(e));
      return a && cmd === 'scan_dev' ? [{ root_path: `${HOME}/projects`, entries: [...devEntries], total_size: sum(devEntries.map((e) => e.size_bytes)), total_files: sum(devEntries.map((e) => e.file_count)), scan_duration_ms: 842, by_type: by }] : { root_path: a.path, entries: [], total_size: 0, total_files: 0, scan_duration_ms: 1, by_type: {} };
    }
    case 'get_dev_artifact_types': return ['node_modules', 'target', '.next', 'venv', 'Pods', 'Library'].map((id) => ({ id, name: id, description: '', icon: '' }));
    case 'estimate_cleanup': { const set = new Set<string>(a.paths); const es = devEntries.filter((e) => set.has(e.path)); const tot = sum(es.map((e) => e.size_bytes)); return { total_bytes: tot, total_human: human(tot), item_count: es.length, by_type: [] }; }
    case 'clean_artifacts': {
      const paths: string[] = a.paths;
      emit('clean-started', { total_items: paths.length, paths });
      let freed = 0, i = 0;
      for (const p of paths) {
        emit('clean-progress', { current_index: i, total_items: paths.length, current_path: p, status: 'deleting', bytes_freed: 0, error: null });
        await wait(450);
        const b = a.dryRun ? (devEntries.find((e) => e.path === p)?.size_bytes ?? 0) : removeFromAll(p);
        freed += b; if (!a.dryRun) freedTotal += b;
        emit('clean-progress', { current_index: i, total_items: paths.length, current_path: p, status: 'deleted', bytes_freed: b, error: null });
        i++;
      }
      emit('clean-completed', { items_removed: paths.length, bytes_freed: freed, errors: [], cancelled: false }, 100);
      return cleanRes(paths, freed);
    }
    case 'clean_path': { const b = a.dryRun ? 0 : removeFromAll(a.path); freedTotal += b; return cleanRes([a.path], b); }
    case 'cancel_cleanup': case 'is_cleanup_cancelled': case 'cancel_usage_scan': return false;

    case 'get_config': return clone(config);
    case 'save_config': return null;
    case 'add_project_dir': case 'remove_project_dir': case 'add_excluded_dir': case 'remove_excluded_dir': case 'set_archive_dir': case 'clear_archive_dir': return clone(config);
    case 'set_onboarding_completed': return null;
    case 'check_full_disk_access': return true;
    case 'open_full_disk_access_settings': return null;
    case 'request_standard_folders': return [];

    case 'list_apps': return clone(apps);
    case 'find_app_files': { const ap = apps.find((x) => x.name === a.appName); return { app_name: a.appName, files: [{ path: ap?.path, size: ap?.size_bytes, size_human: human(ap?.size_bytes ?? 0), exists: true, is_dir: true }], total_size: ap?.total_size_bytes ?? 0, total_size_human: human(ap?.total_size_bytes ?? 0) }; }
    case 'find_remnants': return [
      { app_name: 'Sketch', path: `${HOME}/Library/Application Support/com.bohemiancoding.sketch3`, size_bytes: 1.4 * GB, remnant_type: 'Application Support' },
      { app_name: 'Zoom', path: `${HOME}/Library/Caches/us.zoom.xos`, size_bytes: 620 * MB, remnant_type: 'Caches' },
      { app_name: 'Parallels', path: `${HOME}/Library/Preferences/com.parallels.desktop.plist`, size_bytes: 4 * MB, remnant_type: 'Preferences' },
    ];
    case 'get_app_sizes': { const ap = apps.find((x) => x.name === a.appName); return { app_name: a.appName, data_size_bytes: ap?.data_size_bytes ?? 0, cache_size_bytes: ap?.cache_size_bytes ?? 0 }; }
    case 'trash_app': { apps = apps.filter((x) => x.path !== a.path); return null; }

    case 'scan_packages': case 'scan_package_manager': {
      await wait(500);
      const ms = pkgs.map((m) => ({ ...m, total_cache_size: sum(m.cache_paths.map((c) => c.size_bytes)) }));
      if (cmd === 'scan_package_manager') return ms.find((m) => m.manager === a.manager);
      return { managers: ms, total_cache_size: sum(ms.map((m) => m.total_cache_size)), scan_duration_ms: 410 };
    }
    case 'get_installed_package_managers': return pkgs.map((m) => m.manager);
    case 'clean_package_cache': { const sz = pkgs.flatMap((m) => m.cache_paths).find((c) => c.path === a.path)?.size_bytes ?? 0; if (!a.dryRun) { removeFromAll(a.path); freedTotal += sz; } return { path: a.path, bytes_freed: sz, success: true, message: a.dryRun ? 'Dry run' : 'Moved to Trash' }; }

    case 'scan_system': {
      await wait(500);
      const cats = sysCats.map((c) => ({ ...c, size_bytes: sum(c.paths.map((p) => p.size_bytes)), item_count: c.paths.length }));
      return { categories: cats, total_size: sum(cats.map((c) => c.size_bytes)), scan_duration_ms: 520 };
    }
    case 'clean_system_category': {
      const c = sysCats.find((x) => x.category === a.category); const b = sum((c?.paths ?? []).map((p) => p.size_bytes)); const ps = (c?.paths ?? []).map((p) => p.path);
      if (!a.dryRun && c) { c.paths = []; freedTotal += b; }
      return cleanRes(ps, b);
    }

    case 'scan_devtools': case 'scan_devtool': {
      await wait(500);
      const ts = devtools.map((t) => ({ ...t, total_size: sum(t.cache_paths.map((p: any) => p.size_bytes)) }));
      if (cmd === 'scan_devtool') return ts.find((t) => t.tool === a.tool);
      return { tools: ts, total_size: sum(ts.map((t) => t.total_size)), scan_duration_ms: 380 };
    }
    case 'clean_devtool_cache': case 'clean_ai_model_path': {
      const all = [...devtools.flatMap((t) => t.cache_paths), ...aiTools.flatMap((t) => t.models)];
      const sz = all.find((c: any) => c.path === a.path)?.size_bytes ?? 0;
      if (!a.dryRun) { removeFromAll(a.path); freedTotal += sz; }
      return cleanRes([a.path], sz);
    }
    case 'scan_ai_models': case 'scan_ai_tool': {
      await wait(500);
      const ts = aiTools.map((t) => ({ ...t, total_size: sum(t.models.map((p: any) => p.size_bytes)) }));
      if (cmd === 'scan_ai_tool') return ts.find((t) => t.tool === a.tool);
      return { tools: ts, total_size: sum(ts.map((t) => t.total_size)), scan_duration_ms: 90 };
    }

    case 'list_recipes': return recipes;
    case 'estimate_recipe': case 'run_recipe_cmd': {
      const r = recipes.find((x) => x.id === a.id)!; await wait(cmd === 'run_recipe_cmd' ? 1200 : 300);
      const per = Math.round((14 + (r.id.length % 7) * 3) * GB / r.steps.length);
      const steps = r.steps.map((s) => ({ description: s.description, result: cleanRes([], per), success: true }));
      if (cmd === 'run_recipe_cmd' && !a.dryRun) freedTotal += per * steps.length;
      return { recipe_id: r.id, steps_completed: steps.length, total_steps: steps.length, total_result: cleanRes([], per * steps.length), step_results: steps };
    }

    case 'get_schedule_status_cmd': return { ...schedule };
    case 'enable_schedule_cmd': schedule = { ...schedule, enabled: true, cron: a.cron, tasks: a.tasks, plist_exists: true, is_loaded: true, next_run: new Date(Date.now() + 3 * DAY * 1000).toISOString() }; return null;
    case 'disable_schedule_cmd': schedule = { ...schedule, enabled: false, plist_exists: false, is_loaded: false, next_run: null }; return null;
    case 'run_schedule_now': { schedule.last_run = new Date().toISOString(); return { result: cleanRes([], 9 * GB), timestamp: schedule.last_run, tasks_run: ['dev', 'packages', 'system'] }; }

    case 'list_projects': return clone(projects);
    case 'list_projects_tree': {
      const groups = ['work', 'personal'];
      const tree: any[] = groups.map((g) => ({ kind: 'Group', name: g, path: `${HOME}/projects/${g}`, last_modified: NOW - DAY, size_bytes: null, artifact_size: null, children: projects.filter((p: any) => p.group === g).map(strip) }));
      projects.filter((p: any) => !p.group).forEach((p) => tree.push({ kind: 'Project', ...strip(p), size_bytes: null, artifact_size: null }));
      return tree.map((e) => (e.kind === 'Group' ? { ...e, children: e.children.map((c: any) => ({ ...c, size_bytes: null, artifact_size: null })) } : e));
    }
    case 'calculate_group_size': {
      const ps = projects.filter((p: any) => `${HOME}/projects/${p.group}` === a.path);
      emit('group-size-calculated', { path: a.path, size_bytes: sum(ps.map((p) => p.size_bytes)), artifact_size: sum(ps.map((p) => p.artifact_size)) }, 500);
      return null;
    }
    case 'calculate_project_size': {
      const p = projects.find((x) => x.path === a.path);
      if (p) emit('project-size-calculated', { path: p.path, size_bytes: p.size_bytes, artifact_size: p.artifact_size, last_commit_time: p.last_commit_time }, 300 + Math.random() * 900);
      return null;
    }
    case 'check_git_status': {
      const i = projects.findIndex((x) => x.path === a.path);
      const dirty = i % 3 === 0;
      emit('project-git-status', { path: a.path, status: { has_uncommitted: dirty, uncommitted_count: dirty ? 3 + i : 0, current_branch: i % 4 === 1 ? 'develop' : 'main', remotes: [{ name: 'origin', url: 'git@github.com:dev/' + projects[i]?.name + '.git', ahead: i % 5 === 2 ? 2 : 0, behind: 0 }] } }, 400 + Math.random() * 700);
      return null;
    }
    case 'auto_commit_project': await wait(500); return 'chore: snapshot before archive';
    case 'push_project': return null;
    case 'push_project_all': await wait(600); return [{ remote: 'origin', success: true, error: null }];
    case 'archive_projects': {
      const out: string[] = [];
      for (const path of a.paths as string[]) {
        const p = projects.find((x) => x.path === path); if (!p) continue;
        for (const st of ['cleaning_artifacts', 'compressing', 'deleting_source', 'completed']) { emit('archive-progress', { project_name: p.name, status: st, bytes_copied: 0, total_bytes: p.size_bytes, error: null }); await wait(350); }
        archived.push({ name: p.name, original_path: p.path, archived_path: `${HOME}/Archive/${p.name}.zip`, archived_at: NOW, size_bytes: Math.round(p.size_bytes * 0.18), project_type: p.project_type, archive_format: 'zip' });
        projects = projects.filter((x) => x.path !== path); freedTotal += p.size_bytes; out.push(path);
      }
      return out;
    }
    case 'list_archived_projects': return clone(archived);
    case 'restore_projects': { archived = archived.filter((x) => !(a.archivedPaths as string[]).includes(x.archived_path)); return a.archivedPaths; }
    case 'get_archive_status': return { configured: true, archive_path: `${HOME}/Archive`, exists: true, total_size: sum(archived.map((x) => x.size_bytes)), project_count: archived.length };
    case 'get_archive_config': return { scan_batch_size: 8, clean_artifacts_before_archive: true, compress_to_zip: true };
    case 'save_archive_config': return null;
    case 'delete_project': projects = projects.filter((x) => x.path !== a.path); return null;
    case 'open_project_in_finder': case 'reveal_in_finder': return null;

    case 'get_home_dir': return HOME;
    case 'scan_usage': {
      for (let i = 1; i <= 6; i++) { emit('usage-scan-progress', { scanned_items: i * 94000, current_path: HOME + '/Library', bytes_so_far: i * 90 * GB, unreadable_count: 0, done: false }); await wait(220); }
      emit('usage-scan-progress', { scanned_items: 560000, current_path: '', bytes_so_far: usageTree.size_bytes, unreadable_count: 0, done: true });
      return clone(usageTree);
    }
    case 'get_usage_subtree': { const n = findNode(usageTree, a.path); return clone(n ?? usageTree); }
    case 'trash_usage_item': { const b = removeNode(usageTree, a.path); freedTotal += b; return b; }

    case 'get_cli_status': return { installed: false, path: null, version: null, on_path: false, bundled: true };
    case 'install_cli': await wait(700); return { installed: true, path: '/usr/local/bin/macpurge', version: '0.1.0', on_path: true, bundled: true };
    default: return null;
  }
}

function strip(p: any) { const { group: _g, ...rest } = p; void _g; return rest; }
