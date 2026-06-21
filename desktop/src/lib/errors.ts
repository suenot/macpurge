/**
 * Normalize an unknown thrown value (Tauri command rejections, Errors, etc.)
 * into a human-readable string. Tauri command errors can come back as a plain
 * string, an `Error`, or a serialized object, so rendering them directly risks
 * showing `[object Object]`.
 */
export function toMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  if (err && typeof err === 'object') {
    // Tauri command errors often come back as a string or a plain object
    const anyErr = err as { message?: unknown };
    if (typeof anyErr.message === 'string') return anyErr.message;
    try {
      return JSON.stringify(err);
    } catch {
      return String(err);
    }
  }
  return String(err);
}
