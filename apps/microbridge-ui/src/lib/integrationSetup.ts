/** Short host checklist after install / while needs_setup. */
export function setupNextStep(adapterId: string): string | null {
  switch (adapterId) {
    case "cursor":
      return "Reload Cursor’s window (or quit and reopen Cursor) so the bundled hooks load.";
    case "opencode":
      return "Restart OpenCode (CLI or app) so the Microbridge plugin loads.";
    case "factory":
      return "Start or continue a Factory Droid session — lifecycle events connect automatically.";
    case "t3code":
      return "In T3 Code → Settings → Connections, enable Network access, then paste a one-time pairing link below.";
    default:
      return null;
  }
}
