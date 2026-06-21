'use client';

import { useEffect, useState } from 'react';
import { getConfig, isTauri } from '@/lib/tauri';

// Reads cleanup.confirm_before_delete from the backend config so destructive
// cache cleans can gate themselves behind a confirmation modal. Defaults to
// `true` (safe) until the config loads, so a clean never slips through ungated.
export function useConfirmBeforeDelete(): boolean {
  const [confirmBeforeDelete, setConfirmBeforeDelete] = useState(true);

  useEffect(() => {
    if (!isTauri()) return;

    let cancelled = false;
    getConfig()
      .then((cfg) => {
        if (!cancelled) setConfirmBeforeDelete(cfg.cleanup.confirm_before_delete);
      })
      .catch(() => {
        // Keep the safe default (true) if the config can't be read.
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return confirmBeforeDelete;
}
