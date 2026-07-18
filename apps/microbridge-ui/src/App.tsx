import { useEffect, useState } from "react";
import {
  closeSettings,
  openSettings,
  quitUi,
  setConfig,
  subscribeSnapshot,
} from "./lib/bus";
import { resolveAppearance } from "./lib/theme";
import type { DaemonConfig, Snapshot } from "./lib/types";
import { autoCheckEnabled, runUpdateCheck } from "./lib/updater";
import { Hud } from "./surfaces/Hud";
import { Popover } from "./surfaces/Popover";
import { Settings } from "./surfaces/Settings";

type View = "popover" | "settings" | "hud";
type SettingsTab = "keys" | "agent" | "adapters" | "device" | "updates";

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
    void subscribeSnapshot((snap) => {
      if (active) setSnapshot(snap);
    }).then((u) => {
      unsub = u;
    });
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

    if (autoCheckEnabled()) {
      void runUpdateCheck({ silent: true });
    }

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

  if (!snapshot) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "grid",
          placeItems: "center",
          fontFamily: "Inter, system-ui, sans-serif",
          color: "#6E6E73",
          background: "transparent",
        }}
      >
        Connecting to microbridged…
      </div>
    );
  }

  const dark = resolveAppearance(snapshot.config.appearance) === "dark";

  const applyConfig = async (config: DaemonConfig) => {
    const next = await setConfig(config);
    setSnapshot({ ...snapshot, config: next });
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
      onQuit={() => void quitUi()}
    />
  );
}
