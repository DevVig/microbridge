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
import { Hud } from "./surfaces/Hud";
import { Popover } from "./surfaces/Popover";
import { Settings } from "./surfaces/Settings";

type View = "popover" | "settings" | "hud";
type SettingsTab = "keys" | "agent" | "adapters" | "device";

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
