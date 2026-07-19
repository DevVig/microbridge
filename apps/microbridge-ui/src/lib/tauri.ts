/**
 * Shared Tauri-runtime helpers.
 *
 * Every surface also runs in a plain browser (`npm run dev` + `?view=…`), so
 * Tauri imports are dynamic and guarded. The distinction this module exists to
 * enforce is between "there is no Tauri runtime" and "a command ran and
 * failed" — collapsing those two is what let the browser demo snapshot render
 * inside the real app whenever microbridged wasn't up yet.
 */

/** True when running inside the Tauri webview (v2 injects this global). */
export function hasTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Invoke a Tauri command.
 *
 * Returns `null` **only** when there is no Tauri runtime. A command that runs
 * and fails rejects, so callers can tell a real error from a browser preview.
 */
export async function invokeTauri<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  if (!hasTauri()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<T>(cmd, args);
}

/** Fire-and-forget variant for window controls, where a failure isn't worth surfacing. */
export async function invokeQuiet(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<void> {
  try {
    await invokeTauri(cmd, args);
  } catch {
    /* window control failed — nothing useful to tell the user */
  }
}
