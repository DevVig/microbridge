import type { AdapterConnectionState } from "./types";

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

export type GuidancePrimaryAction =
  | "open_app"
  | "enable"
  | "pair"
  | "none";

export interface IntegrationGuidance {
  title: string;
  steps: string[];
  primaryAction: GuidancePrimaryAction;
}

/**
 * Structured next-step guidance for Integrations detail panels.
 * Covers setup, idle hosts, limited community adapters, and always-on watchers.
 */
export function integrationGuidance(
  adapterId: string,
  state: AdapterConnectionState,
  options?: { enabled?: boolean; label?: string },
): IntegrationGuidance | null {
  const enabled = options?.enabled;
  const label = options?.label;

  // Auto-discovered on disk but not yet installed via first click.
  if (
    state === "needs_setup" &&
    enabled === false &&
    (adapterId === "cursor" ||
      adapterId === "factory" ||
      adapterId === "opencode" ||
      adapterId === "t3code")
  ) {
    return {
      title: "Detected on this Mac",
      steps: [
        "Click this tile (or Enable) to install the Microbridge integration.",
        adapterId === "t3code"
          ? "Then enable Network access in T3 Code → Settings → Connections and paste a pairing link."
          : adapterId === "cursor"
            ? "Reload Cursor’s window so the bundled hooks load, then use Cursor normally."
            : adapterId === "opencode"
              ? "Restart OpenCode so the Microbridge plugin loads."
              : "Start or continue a Factory Droid session — lifecycle events connect automatically.",
      ],
      primaryAction: "enable",
    };
  }

  switch (adapterId) {
    case "cursor":
      if (state === "disabled") {
        return {
          title: "Enable Cursor",
          steps: [
            "Click this tile to install the bundled Cursor plugin.",
            "Reload Cursor’s window (or quit and reopen) so hooks load.",
            "Use Cursor — Microbridge turns Connected once lifecycle events arrive.",
          ],
          primaryAction: "enable",
        };
      }
      if (state === "needs_setup" || state === "connecting") {
        return {
          title: "Finish Cursor setup",
          steps: [
            "Reload Cursor’s window (or quit and reopen Cursor) so the bundled hooks load.",
            "Open or continue a Cursor agent thread so Microbridge receives lifecycle events.",
            "Expect Connected once hooks are talking to the daemon (approve/interrupt need Cursor ACP).",
          ],
          primaryAction: "open_app",
        };
      }
      if (state === "limited" || state === "connected") {
        return {
          title:
            state === "connected"
              ? "Cursor lifecycle is connected"
              : "Cursor lifecycle is live",
          steps: [
            "Lifecycle observation matches Claude Code’s ceiling for the IDE composer.",
            "Approve, interrupt, and open-conversation control need Cursor ACP/SDK (Microbridge-owned agents) — not available for the IDE composer yet.",
            "Keep using Cursor; threads appear as hooks and transcript watches fire.",
          ],
          primaryAction: "open_app",
        };
      }
      return null;

    case "factory":
      if (state === "disabled") {
        return {
          title: "Enable Factory",
          steps: [
            "Click this tile to install Factory hooks.",
            "Start or continue a Factory Droid session — lifecycle events connect automatically.",
          ],
          primaryAction: "enable",
        };
      }
      if (state === "needs_setup" || state === "connecting") {
        return {
          title: "Finish Factory setup",
          steps: [
            "Start or continue a Factory Droid session — lifecycle events connect automatically.",
            "No separate pairing step; the status turns Limited/Connected after the first events.",
          ],
          primaryAction: "none",
        };
      }
      if (state === "limited") {
        return {
          title: "Factory is partially connected",
          steps: [
            "Lifecycle is live. Keep Droid sessions running to see threads in Microbridge.",
          ],
          primaryAction: "none",
        };
      }
      return null;

    case "opencode":
      if (state === "disabled") {
        return {
          title: "Enable OpenCode",
          steps: [
            "Click this tile to install the Microbridge OpenCode plugin.",
            "Restart OpenCode (CLI or app) so the plugin loads.",
          ],
          primaryAction: "enable",
        };
      }
      if (state === "needs_setup" || state === "connecting") {
        return {
          title: "Finish OpenCode setup",
          steps: [
            "Restart OpenCode (CLI or app) so the Microbridge plugin loads.",
            "Run an OpenCode session — status turns Connected when the plugin says hello.",
          ],
          primaryAction: "open_app",
        };
      }
      return null;

    case "t3code":
      if (state === "disabled") {
        return {
          title: "Enable T3 Code",
          steps: [
            "Click this tile to enable the T3 Code integration.",
            "In T3 Code → Settings → Connections, enable Network access.",
            "Paste a one-time pairing link below and Pair.",
          ],
          primaryAction: "enable",
        };
      }
      if (state === "needs_setup" || state === "connecting") {
        return {
          title: "Pair T3 Code",
          steps: [
            "In T3 Code → Settings → Connections, enable Network access.",
            "Paste a one-time pairing link below and click Pair.",
            "Microbridge stores the credential and connects to your approved environment.",
          ],
          primaryAction: "pair",
        };
      }
      if (state === "incompatible") {
        return {
          title: "Update required",
          steps: [
            "This T3 Code server version is not supported.",
            "Update Microbridge and/or T3 Code, then pair again with a fresh link.",
          ],
          primaryAction: "pair",
        };
      }
      return null;

    case "cursor_acp":
      if (state === "disabled") {
        return {
          title: "Enable Cursor Agent (ACP)",
          steps: [
            "Install the Cursor CLI so `agent` or `cursor-agent` is on PATH.",
            "Click this tile to enable ACP control for Microbridge-owned agents.",
            "This does not remote-control the IDE Composer — use the Cursor tile for that lifecycle.",
          ],
          primaryAction: "enable",
        };
      }
      if (state === "needs_setup" || state === "connecting") {
        return {
          title: "Install Cursor CLI",
          steps: [
            "Install/authenticate the Cursor CLI (`agent` / `cursor-agent`).",
            "Restart Microbridge after PATH updates, then use New Session / Interrupt from hardware or UI.",
          ],
          primaryAction: "none",
        };
      }
      if (state === "connected" || state === "limited") {
        return {
          title: "ACP control ready",
          steps: [
            "New Session starts a Microbridge-owned ACP agent.",
            "Interrupt / Approve / Reject map to ACP session methods.",
            "IDE Composer chats stay on the Cursor tile (lifecycle only).",
          ],
          primaryAction: "none",
        };
      }
      return null;

    case "cnvs":
      if (state === "needs_setup" || state === "disabled") {
        return {
          title: "Start CNVS",
          steps: [
            "Launch CNVS on this Mac — Microbridge connects automatically via the local loopback API.",
            "No pairing or plugin install is required.",
          ],
          primaryAction: "none",
        };
      }
      if (state === "limited") {
        return {
          title: "CNVS is partially connected",
          steps: [
            "CNVS is reachable but some canvases could not be refreshed.",
            "Check CNVS is running and try focusing a canvas from Microbridge.",
          ],
          primaryAction: "none",
        };
      }
      return null;

    case "codex":
    case "claude":
      if (state === "connected" || state === "limited") {
        return {
          title:
            adapterId === "codex"
              ? "Codex CLI watcher"
              : "Claude Code watcher",
          steps: [
            "This integration is always on — Microbridge watches local session journals.",
            "Run Codex or Claude Code to see threads appear; host apps (ChatGPT, Synara, etc.) share the same journals.",
          ],
          primaryAction: "none",
        };
      }
      if (state === "disabled" || state === "needs_setup") {
        return {
          title: "Re-enable in config",
          steps: [
            "This built-in watcher is disabled in ~/.microbridge/config.toml.",
            "Set the adapter enabled and restart the Microbridge service.",
          ],
          primaryAction: "none",
        };
      }
      return null;

    case "synara":
    case "chatgpt":
    case "claude_desktop":
    case "conductor": {
      const hostName =
        adapterId === "synara"
          ? "Synara"
          : adapterId === "chatgpt"
            ? "ChatGPT"
            : adapterId === "claude_desktop"
              ? "Claude Desktop"
              : "Conductor";
      if (
        label === "Idle" ||
        label === "Ready · idle" ||
        state === "connected" ||
        state === "limited"
      ) {
        return {
          title: `${hostName} via journals`,
          steps: [
            `Ready/Idle is normal — start ${hostName} (or use Claude/Codex through it).`,
            "Sessions appear automatically from Claude & Codex journals; no separate adapter or pairing.",
          ],
          primaryAction: "none",
        };
      }
      if (state === "disabled") {
        return {
          title: "Disabled in config",
          steps: [
            `${hostName} attribution is disabled in ~/.microbridge/config.toml.`,
            "Re-enable it there and restart Microbridge if you want these sessions listed.",
          ],
          primaryAction: "none",
        };
      }
      return null;
    }

    default:
      return null;
  }
}
