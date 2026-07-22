import { useEffect, useState } from "react";
import {
  activateAgentKey,
  closeSettings,
  openSettings,
  quitUi,
  setConfig,
  subscribeSnapshot,
} from "./lib/bus";
import { promptLaunchAtLoginOnce } from "./lib/autostart";
import { resolveAppearance } from "./lib/theme";
import type { DaemonConfig, Snapshot } from "./lib/types";
import { runAutomaticUpdateCheck, runUpdateCheck } from "./lib/updater";
import { Disconnected } from "./surfaces/Disconnected";
import { Hud } from "./surfaces/Hud";
import { Popover } from "./surfaces/Popover";
import { Settings } from "./surfaces/Settings";

type View = "popover" | "settings" | "hud";
type SettingsTab =
  | "general"
  | "keys"
  | "agent"
  | "integrations"
  | "device"
  | "updates";

function initialView(): View {
  const q = new URLSearchParams(window.location.search).get("view");
  if (q === "settings" || q === "hud" || q === "popover") return q;
  return "popover";
}

export default function App() {
  const [view] = useState<View>(initialView);
  const [tab, setTab] = useState<SettingsTab>("keys");
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);

  useEffect(() => {
    let active = true;
    let unsub: (() => void) | undefined;
    subscribeSnapshot((snap) => {
      if (active) setSnapshot(snap);
    }).then(
      (u) => {
        unsub = u;
      },
      () => {
        /* no bus — the disconnected surface already says so */
      },
    );
    return () => {
      active = false;
      unsub?.();
    };
  }, []);

  // Updates run only from the popover (always loaded) so they fire once, not
  // once per window. The tray "Check for Updates…" item emits the event; the
  // opt-in launch check is silent so a current install shows nothing.
  useEffect(() => {
    if (view !== "popover") return;
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void (async () => {
      await promptLaunchAtLoginOnce();
      await runAutomaticUpdateCheck();
    })();

    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const un = await listen("menu://check-updates", () => {
          void runUpdateCheck({ silent: false });
        });
        if (disposed) un();
        else unlisten = un;
      } catch {
        /* not running under Tauri */
      }
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [view]);

  // No snapshot means microbridged hasn't sent one yet. The HUD is a transient
  // overlay with nothing to say here, so it stays blank; the other two surfaces
  // explain the situation and offer a way out.
  if (!snapshot) {
    if (view === "hud") return null;
    return (
      <Disconnected
        dark={resolveAppearance("system") === "dark"}
        view={view}
        onQuit={() => void quitUi()}
        onOpenSettings={() => void openSettings()}
        onClose={() => void closeSettings()}
      />
    );
  }

  const dark = resolveAppearance(snapshot.config.appearance) === "dark";

  // On failure, leave local state alone so the control snaps back to what the
  // daemon actually has, rather than showing a change that didn't take.
  const applyConfig = async (config: DaemonConfig) => {
    try {
      const next = await setConfig(config);
      setSnapshot({ ...snapshot, config: next });
    } catch {
      /* daemon rejected the write — keep showing its last known config */
    }
  };

  if (view === "hud") {
    return <Hud snapshot={snapshot} dark={dark} />;
  }

  if (view === "settings") {
    return (
      <Settings
        snapshot={snapshot}
        dark={dark}
        tab={tab}
        onTab={setTab}
        onConfig={(c) => void applyConfig(c)}
        onClose={() => void closeSettings()}
        onAgentKey={(index, open) => void activateAgentKey(index, open)}
      />
    );
  }

  return (
    <Popover
      snapshot={snapshot}
      dark={dark}
      onOpenSettings={() => void openSettings()}
      onTogglePause={() =>
        void applyConfig({
          ...snapshot.config,
          pause_leds: !snapshot.config.pause_leds,
        })
      }
      onHardwareControl={(enabled) =>
        void applyConfig({
          ...snapshot.config,
          hardware_control_enabled: enabled,
        })
      }
      onQuit={() => void quitUi()}
      onAgentKey={(index, open) => void activateAgentKey(index, open)}
    />
  );
}
