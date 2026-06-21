// TypeScript types mirroring Rust structures

export type DiskType = 'system' | 'internal' | 'external' | 'network';

export interface DiskInfo {
  name: string;
  mount_point: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
  disk_type: DiskType;
  fs_type: string;
}

export interface DisksChangedPayload {
  reason: 'mounted' | 'unmounted' | 'space_update';
}

export interface FreeSpaceInfo {
  available_bytes: number;
  total_bytes: number;
  available_gb: number;
  total_gb: number;
  available_human: string;
  usage_percent: number;
  is_low: boolean;
}

export interface ScannedEntry {
  path: string;
  file_type: string;
  size_bytes: number;
  file_count: number;
  last_modified: number | null;
}

export interface ScanResult {
  root_path: string;
  entries: ScannedEntry[];
  total_size: number;
  total_files: number;
  scan_duration_ms: number;
  by_type: Record<string, ScannedEntry[]>;
}

export interface CleanResult {
  items_removed: number;
  bytes_freed: number;
  errors: string[];
  removed_paths: string[];
}

export interface CleanupEstimate {
  total_bytes: number;
  total_human: string;
  item_count: number;
  by_type: TypeEstimate[];
}

export interface TypeEstimate {
  artifact_type: string;
  bytes: number;
  human: string;
}

export interface ArtifactType {
  id: string;
  name: string;
  description: string;
  icon: string;
}

export interface ProjectDir {
  path: string;
  expanded: string;
  exists: boolean;
}

export interface ConfigResponse {
  config_path: string;
  project_dirs: ProjectDir[];
  excluded_dirs: ProjectDir[];
  scan: ScanConfigResponse;
  cleanup: CleanupConfigResponse;
  schedule_enabled: boolean;
  archive_dir: string | null;
  onboarding_completed: boolean;
}

// Result of requesting access to a standard macOS folder (Downloads/Desktop/etc).
export interface FolderAccess {
  name: string;
  path: string;
  accessible: boolean;
}

export interface ScanConfigResponse {
  max_depth: number;
  include_hidden: boolean;
  artifact_types: string[];
}

export interface CleanupConfigResponse {
  confirm_before_delete: boolean;
  use_trash: boolean;
  log_deletions: boolean;
  older_than_days: number | null;
}

export interface ConfigInput {
  project_dirs: string[];
  scan: {
    max_depth: number;
    include_hidden: boolean;
  };
  cleanup: {
    confirm_before_delete: boolean;
    use_trash: boolean;
  };
}

export interface AppInfo {
  name: string;
  path: string;
  bundle_id: string | null;
  version: string | null;
  size_bytes: number;
  data_size_bytes: number;
  cache_size_bytes: number;
  total_size_bytes: number;
}

export interface AppRemnant {
  app_name: string;
  path: string;
  size_bytes: number;
  remnant_type: string;
}

export interface AppFileInfo {
  path: string;
  size: number;
  size_human: string;
  exists: boolean;
  is_dir: boolean;
}

export interface AppFilesResult {
  app_name: string;
  files: AppFileInfo[];
  total_size: number;
  total_size_human: string;
}

// Utility function to format bytes
export function formatBytes(bytes: number | undefined | null): string {
  if (bytes === undefined || bytes === null || isNaN(bytes)) {
    return '0 B';
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let unitIndex = 0;
  let size = bytes;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(unitIndex === 0 ? 0 : 2)} ${units[unitIndex]}`;
}

// Same value/unit split as formatBytes, returned as separate parts so callers
// can style the number and unit independently without fragile `.split(' ')`.
export function formatBytesParts(bytes: number | undefined | null): { value: string; unit: string } {
  const [value, unit] = formatBytes(bytes).split(' ');
  return { value, unit };
}

// Category for dashboard
export interface Category {
  id: string;
  name: string;
  icon: string;
  size_bytes: number;
  item_count: number;
  color: string;
}

// Package manager types
export type PkgManagerType =
  | 'npm'
  | 'yarn'
  | 'pnpm'
  | 'pip'
  | 'cargo'
  | 'rustup'
  | 'go'
  | 'gem'
  | 'brew'
  | 'composer';

export interface CachePath {
  path: string;
  size_bytes: number;
  description: string;
  safe_to_clean: boolean;
}

export interface GlobalPackage {
  name: string;
  version: string | null;
  size_bytes: number | null;
}

export interface PackageManagerInfo {
  manager: PkgManagerType;
  cache_paths: CachePath[];
  total_cache_size: number;
  global_packages: GlobalPackage[];
  is_installed: boolean;
}

export interface PackageScanResult {
  managers: PackageManagerInfo[];
  total_cache_size: number;
  scan_duration_ms: number;
}

export interface CleanCacheResult {
  path: string;
  bytes_freed: number;
  success: boolean;
  message: string;
}

// Cleanup event payloads
export interface CleanStartedPayload {
  total_items: number;
  paths: string[];
}

export interface CleanProgressPayload {
  current_index: number;
  total_items: number;
  current_path: string;
  status: 'deleting' | 'deleted' | 'error' | 'skipped';
  bytes_freed: number;
  error: string | null;
}

export interface CleanCompletedPayload {
  items_removed: number;
  bytes_freed: number;
  errors: string[];
  cancelled: boolean;
}

// ── System scanner types ──────────────────────

export type SystemCacheType =
  | 'user_cache'
  | 'system_cache'
  | 'browser_cache'
  | 'system_logs'
  | 'ios_backups'
  | 'trash'
  | 'downloads_old'
  | 'mail_caches';

export interface SystemCategoryPath {
  path: string;
  size_bytes: number;
  description: string;
}

export interface SystemCategoryInfo {
  category: SystemCacheType;
  name: string;
  description: string;
  size_bytes: number;
  item_count: number;
  paths: SystemCategoryPath[];
}

export interface SystemScanResult {
  categories: SystemCategoryInfo[];
  total_size: number;
  scan_duration_ms: number;
}

// ── DevTools types ────────────────────────────

export type DevToolType = 'xcode' | 'android_studio' | 'docker' | 'virtual_machines';

export interface DevToolCachePath {
  path: string;
  size_bytes: number;
  description: string;
  category: string;
  safe_to_clean: boolean;
}

export interface DevToolInfo {
  tool: DevToolType;
  is_installed: boolean;
  cache_paths: DevToolCachePath[];
  total_size: number;
}

export interface DevToolScanResult {
  tools: DevToolInfo[];
  total_size: number;
  scan_duration_ms: number;
}

// ── AI model types ────────────────────────────

export type AiToolType = 'ollama' | 'lm_studio' | 'hugging_face' | 'comfy_ui';

export interface AiModelPath {
  path: string;
  size_bytes: number;
  description: string;
  category: string;
  safe_to_clean: boolean;
}

export interface AiToolInfo {
  tool: AiToolType;
  is_installed: boolean;
  models: AiModelPath[];
  total_size: number;
}

export interface AiModelScanResult {
  tools: AiToolInfo[];
  total_size: number;
  scan_duration_ms: number;
}

// ── Recipe types ──────────────────────────────

export interface RecipeStep {
  action: Record<string, unknown>;
  description: string;
}

export interface Recipe {
  id: string;
  name: string;
  description: string;
  category: string;
  steps: RecipeStep[];
  estimated_savings: string;
}

export interface StepResult {
  description: string;
  result: CleanResult;
  success: boolean;
}

export interface RecipeRunResult {
  recipe_id: string;
  steps_completed: number;
  total_steps: number;
  total_result: CleanResult;
  step_results: StepResult[];
}

// ── Schedule types ────────────────────────────

export interface ScheduleTasks {
  clean_dev: boolean;
  clean_packages: boolean;
  clean_docker: boolean;
  clean_system: boolean;
  clean_browsers: boolean;
  empty_trash: boolean;
}

export interface ScheduleStatus {
  enabled: boolean;
  cron: string;
  plist_path: string;
  plist_exists: boolean;
  is_loaded: boolean;
  tasks: ScheduleTasks;
  last_run: string | null;
  next_run: string | null;
}

export interface ScheduleRunResult {
  result: CleanResult;
  timestamp: string;
  tasks_run: string[];
}

// ── Project types ─────────────────────────────

export type ProjectType =
  | 'Node'
  | 'Rust'
  | 'Go'
  | 'Python'
  | 'Java'
  | 'DotNet'
  | 'Ruby'
  | 'Php'
  | 'Elixir'
  | 'Haskell'
  | 'Swift'
  | 'Dart'
  | 'Unknown';

export interface ProjectInfo {
  name: string;
  path: string;
  size_bytes: number | null;
  last_modified: number | null;
  project_type: ProjectType;
  has_git: boolean;
  last_commit_time: number | null;
  artifact_size: number | null;
}

export interface ArchivedProjectEntry {
  name: string;
  original_path: string;
  archived_path: string;
  archived_at: number;
  size_bytes: number;
  project_type: ProjectType;
  archive_format: string | null;
}

export interface ProjectSizePayload {
  path: string;
  size_bytes: number;
  artifact_size: number;
  last_commit_time: number | null;
}

export interface ArchiveProgressPayload {
  project_name: string;
  status: 'cleaning_artifacts' | 'compressing' | 'copying' | 'deleting_source' | 'completed' | 'error' | 'extracting';
  bytes_copied: number;
  total_bytes: number;
  error: string | null;
}

export interface ArchiveConfigResponse {
  scan_batch_size: number;
  clean_artifacts_before_archive: boolean;
  compress_to_zip: boolean;
}

export interface ArchiveConfigInput {
  scan_batch_size: number;
  clean_artifacts_before_archive: boolean;
  compress_to_zip: boolean;
}

// ── Project tree types ────────────────────────

export interface ProjectTreeProject {
  kind: 'Project';
  name: string;
  path: string;
  size_bytes: number | null;
  last_modified: number | null;
  project_type: ProjectType;
  has_git: boolean;
  last_commit_time: number | null;
  artifact_size: number | null;
}

export interface ProjectTreeGroup {
  kind: 'Group';
  name: string;
  path: string;
  last_modified: number | null;
  size_bytes: number | null;
  artifact_size: number | null;
  children: ProjectInfo[];
}

export type ProjectTreeEntry = ProjectTreeProject | ProjectTreeGroup;

export interface GroupSizePayload {
  path: string;
  size_bytes: number;
  artifact_size: number;
}

export interface ArchiveStatusResponse {
  configured: boolean;
  archive_path: string | null;
  exists: boolean;
  total_size: number;
  project_count: number;
}

// ── Disk usage (sunburst) types ───────────────

export interface UsageNode {
  name: string;
  path: string;
  size_bytes: number;
  file_count: number;
  is_dir: boolean;
  is_symlink: boolean;
  children: UsageNode[];
  has_more: boolean;
  aggregated_count: number | null;
}

export interface UsageScanProgress {
  scanned_items: number;
  current_path: string;
  bytes_so_far: number;
  unreadable_count: number;
  done: boolean;
}

// ── Git types ────────────────────────────────

export interface GitRemoteStatus {
  name: string;
  url: string;
  ahead: number;
  behind: number;
}

export interface GitStatus {
  has_uncommitted: boolean;
  uncommitted_count: number;
  current_branch: string | null;
  remotes: GitRemoteStatus[];
}

export interface GitStatusPayload {
  path: string;
  status: GitStatus;
}

export interface PushRemoteResult {
  remote: string;
  success: boolean;
  error: string | null;
}
