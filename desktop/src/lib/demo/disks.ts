import type { DiskInfo } from '../types';

const GB = 1024 ** 3;

// Initial disk shown in the sidebar of the website demo, before the mock backend answers.
export const DEMO_DISKS: DiskInfo[] = [
  { name: 'Macintosh HD', mount_point: '/', total_bytes: 994 * GB, available_bytes: 312 * GB, used_bytes: 682 * GB, usage_percent: 68.6, disk_type: 'system', fs_type: 'apfs' },
];
