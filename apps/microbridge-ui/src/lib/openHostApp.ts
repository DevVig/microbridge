import { hasTauri } from "./tauri";

/** Best-effort /Applications bundle names for Integrations “Open …” buttons. */
const HOST_APP_BUNDLE: Record<string, string> = {
  cursor: "Cursor.app",
  opencode: "OpenCode.app",
  t3code: "T3 Code.app",
};

export function openableHostApp(adapterId: string): string | null {
  const bundle = HOST_APP_BUNDLE[adapterId];
  return bundle ? bundle.replace(/\.app$/, "") : null;
}

/** Open a macOS app bundle. Ignores errors (missing app, browser preview). */
export async function openHostApp(adapterId: string): Promise<void> {
  const bundle = HOST_APP_BUNDLE[adapterId];
  if (!bundle || !hasTauri()) return;
  try {
    const { open } = await import("@tauri-apps/plugin-shell");
    await open(`/Applications/${bundle}`);
  } catch {
    /* host app missing or shell unavailable — detail checklist still guides the user */
  }
}
