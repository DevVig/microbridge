import { useCallback, useEffect, useState } from "react";
import { fetchSnapshot, setConfig } from "./lib/bus";
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
  const [view, setView] = useState<View>(initialView);
  const [tab, setTab] = useState<SettingsTab>("agent");
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [hudFlash, setHudFlash] = useState(false);
  const [prevFocus, setPrevFocus] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const snap = await fetchSnapshot();
    setSnapshot((prev) => {
      if (
        prev &&
        snap.focused_session_id &&
        snap.focused_session_id !== prev.focused_session_id
      ) {
        setHudFlash(true);
        window.setTimeout(() => setHudFlash(false), 2600);
      }
      return snap;
    });
    setPrevFocus(snap.focused_session_id);
  }, []);

  useEffect(() => {
    void refresh();
    // UI polls only as a fallback when Tauri events are unavailable.
    // In Tauri, replace with listen("bus-event").
    const id = window.setInterval(() => void refresh(), 2000);
    return () => window.clearInterval(id);
  }, [refresh]);

  useEffect(() => {
    void prevFocus;
  }, [prevFocus]);

  if (!snapshot) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "grid",
          placeItems: "center",
          fontFamily: "Inter, system-ui, sans-serif",
          color: "#6E6E73",
        }}
      >
        Connecting to microbridged…
      </div>
    );
  }

  const dark =
    resolveAppearance(snapshot.config.appearance) === "dark";

  const applyConfig = async (config: DaemonConfig) => {
    const next = await setConfig(config);
    setSnapshot({ ...snapshot, config: next });
  };

  if (view === "hud" || hudFlash) {
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
        onClose={() => setView("popover")}
      />
    );
  }

  return (
    <Popover
      snapshot={snapshot}
      dark={dark}
      onOpenSettings={() => setView("settings")}
      onTogglePause={() =>
        void applyConfig({
          ...snapshot.config,
          pause_leds: !snapshot.config.pause_leds,
        })
      }
      onQuit={() => {
        void import("@tauri-apps/api/core")
          .then(({ invoke }) => invoke("quit_ui"))
          .catch(() => window.close());
      }}
    />
  );
}
