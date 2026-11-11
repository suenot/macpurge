// Single seam between the UI and the Tauri backend.
// In a normal build this re-exports the real Tauri API. When built with
// NEXT_PUBLIC_DEMO=1 (the interactive demo on the website) it talks to an
// in-memory mock backend instead, so the real UI runs in a plain browser.

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';

export type UnlistenFn = () => void;
type Evt<T> = { payload: T };

export const IS_DEMO = process.env.NEXT_PUBLIC_DEMO === '1';

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (IS_DEMO) {
    const { mockInvoke } = await import('./demo/mock');
    return mockInvoke<T>(cmd, args ?? {});
  }
  return tauriInvoke<T>(cmd, args);
}

export async function listen<T>(event: string, handler: (e: Evt<T>) => void): Promise<UnlistenFn> {
  if (IS_DEMO) {
    const { mockListen } = await import('./demo/mock');
    return mockListen<T>(event, handler);
  }
  return tauriListen<T>(event, handler);
}
