// Tauri API wrappers

import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type {
  DiskInfo,
  FreeSpaceInfo,
  DisksChangedPayload,
  ScanResult,
  CleanResult,
  CleanupEstimate,
  ArtifactType,
  ConfigResponse,
  ConfigInput,
  AppInfo,
  AppRemnant,
  AppFilesResult,
  PackageScanResult,
  PackageManagerInfo,
  CleanCacheResult,
  CleanStartedPayload,
  CleanProgressPayload,
  CleanCompletedPayload,
  SystemScanResult,
  DevToolScanResult,
  DevToolInfo,
  Recipe,
  RecipeRunResult,
  ScheduleStatus,
  ScheduleTasks,
  ScheduleRunResult,
} from './types';

// Disk commands
export async function getDiskInfo(): Promise<DiskInfo[]> {
  return invoke<DiskInfo[]>('get_disk_info');
}

export async function getFreeSpace(): Promise<DiskInfo> {
  return invoke<DiskInfo>('get_free_space');
}

export async function getFreeSpaceHuman(): Promise<FreeSpaceInfo> {
  return invoke<FreeSpaceInfo>('get_free_space_human');
}

export async function getUserDisks(): Promise<DiskInfo[]> {
  return invoke<DiskInfo[]>('get_user_disks');
}

export function onDisksChanged(callback: (payload: DisksChangedPayload) => void): Promise<UnlistenFn> {
  return listen<DisksChangedPayload>('disks-changed', (event) => callback(event.payload));
}

// Scan commands
export async function scanDev(): Promise<ScanResult[]> {
  return invoke<ScanResult[]>('scan_dev');
}

export async function scanPath(path: string): Promise<ScanResult> {
  return invoke<ScanResult>('scan_path', { path });
}

export async function getDevArtifactTypes(): Promise<ArtifactType[]> {
  return invoke<ArtifactType[]>('get_dev_artifact_types');
}

// Clean commands
export async function cleanArtifacts(
  paths: string[],
  dryRun: boolean = false
): Promise<CleanResult> {
  return invoke<CleanResult>('clean_artifacts', { paths, dryRun });
}

export async function cleanPath(
  path: string,
  dryRun: boolean = false
): Promise<CleanResult> {
  return invoke<CleanResult>('clean_path', { path, dryRun });
}

export async function estimateCleanup(paths: string[]): Promise<CleanupEstimate> {
  return invoke<CleanupEstimate>('estimate_cleanup', { paths });
}

export async function cancelCleanup(): Promise<boolean> {
  return invoke<boolean>('cancel_cleanup');
}

export async function isCleanupCancelled(): Promise<boolean> {
  return invoke<boolean>('is_cleanup_cancelled');
}

// Config commands
export async function getConfig(): Promise<ConfigResponse> {
  return invoke<ConfigResponse>('get_config');
}

export async function saveConfig(config: ConfigInput): Promise<void> {
  return invoke('save_config', { config });
}

export async function addProjectDir(path: string): Promise<ConfigResponse> {
  return invoke<ConfigResponse>('add_project_dir', { path });
}

export async function removeProjectDir(path: string): Promise<ConfigResponse> {
  return invoke<ConfigResponse>('remove_project_dir', { path });
}

// App commands
export async function listApps(): Promise<AppInfo[]> {
  return invoke<AppInfo[]>('list_apps');
}

export async function findAppFiles(appName: string): Promise<AppFilesResult> {
  return invoke<AppFilesResult>('find_app_files', { appName });
}

export async function findRemnants(): Promise<AppRemnant[]> {
  return invoke<AppRemnant[]>('find_remnants');
}

export async function getAppSizes(appName: string, bundleId: string | null): Promise<{ app_name: string; data_size_bytes: number; cache_size_bytes: number }> {
  return invoke('get_app_sizes', { appName, bundleId });
}

// Package manager commands
export async function scanPackages(): Promise<PackageScanResult> {
  return invoke<PackageScanResult>('scan_packages');
}

export async function scanPackageManager(manager: string): Promise<PackageManagerInfo> {
  return invoke<PackageManagerInfo>('scan_package_manager', { manager });
}

export async function getInstalledPackageManagers(): Promise<string[]> {
  return invoke<string[]>('get_installed_package_managers');
}

export async function cleanPackageCache(
  path: string,
  dryRun: boolean = false
): Promise<CleanCacheResult> {
  return invoke<CleanCacheResult>('clean_package_cache', { path, dryRun });
}

// Event listeners
export function onTriggerScan(callback: () => void): Promise<UnlistenFn> {
  return listen('trigger-scan', callback);
}

export function onCleanStarted(callback: (payload: CleanStartedPayload) => void): Promise<UnlistenFn> {
  return listen<CleanStartedPayload>('clean-started', (event) => callback(event.payload));
}

export function onCleanProgress(callback: (payload: CleanProgressPayload) => void): Promise<UnlistenFn> {
  return listen<CleanProgressPayload>('clean-progress', (event) => callback(event.payload));
}

export function onCleanCompleted(callback: (payload: CleanCompletedPayload) => void): Promise<UnlistenFn> {
  return listen<CleanCompletedPayload>('clean-completed', (event) => callback(event.payload));
}

// System commands
export async function scanSystem(): Promise<SystemScanResult> {
  return invoke<SystemScanResult>('scan_system');
}

export async function cleanSystemCategory(
  category: string,
  dryRun: boolean = false,
  minAgeDays?: number,
): Promise<CleanResult> {
  return invoke<CleanResult>('clean_system_category', { category, dryRun, minAgeDays });
}

// DevTools commands
export async function scanDevtools(): Promise<DevToolScanResult> {
  return invoke<DevToolScanResult>('scan_devtools');
}

export async function scanDevtool(tool: string): Promise<DevToolInfo> {
  return invoke<DevToolInfo>('scan_devtool', { tool });
}

export async function cleanDevtoolCache(
  path: string,
  dryRun: boolean = false,
): Promise<CleanResult> {
  return invoke<CleanResult>('clean_devtool_cache', { path, dryRun });
}

// Recipe commands
export async function listRecipes(): Promise<Recipe[]> {
  return invoke<Recipe[]>('list_recipes');
}

export async function runRecipe(id: string, dryRun: boolean = false): Promise<RecipeRunResult> {
  return invoke<RecipeRunResult>('run_recipe_cmd', { id, dryRun });
}

export async function estimateRecipe(id: string): Promise<RecipeRunResult> {
  return invoke<RecipeRunResult>('estimate_recipe', { id });
}

// Schedule commands
export async function getScheduleStatus(): Promise<ScheduleStatus> {
  return invoke<ScheduleStatus>('get_schedule_status_cmd');
}

export async function enableSchedule(cron: string, tasks: ScheduleTasks): Promise<void> {
  return invoke('enable_schedule_cmd', { cron, tasks });
}

export async function disableSchedule(): Promise<void> {
  return invoke('disable_schedule_cmd');
}

export async function runScheduleNow(): Promise<ScheduleRunResult> {
  return invoke<ScheduleRunResult>('run_schedule_now');
}

// Helper to check if running in Tauri
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}
