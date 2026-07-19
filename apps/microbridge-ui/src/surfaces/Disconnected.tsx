import { useEffect, useState } from "react";
import { DARK, LIGHT, type ThemeTokens } from "../lib/theme";
import { usePopoverFit } from "../lib/popoverFit";
import { updateChannel, type UpdateChannel } from "../lib/updater";

/**
 * Shown when microbridged hasn't sent a snapshot yet.
 *
 * This state used to be nearly unreachable: the bus layer fell back to the
 * browser demo snapshot on *any* failure, so a fresh install with no daemon
 * running rendered three fabricated sessions. The fallback is now scoped to
 * "no Tauri runtime", which makes this the honest answer inside the app.
 */
export function Disconnected({
  dark,
  view,
  onQuit,
  onOpenSettings,
  onClose,
}: {
  dark: boolean;
  view: "popover" | "settings";
  onQuit: () => void;
  onOpenSettings: () => void;
  onClose: () => void;
}) {
  const t = dark ? DARK : LIGHT;
  // This surface renders in the settings window too, where it must not touch
  // the popover's size.
  const { ref: cardRef, maxHeight } = usePopoverFit<HTMLDivElement>(
    view === "popover",
  );
  const [channel, setChannel] = useState<UpdateChannel | null>(null);

  useEffect(() => {
    void updateChannel().then(setChannel);
  }, []);

  // Homebrew owns the daemon as a service; direct installs get the launchd
  // agent written by install.sh. Showing the wrong one is worse than waiting,
  // so hold the command back until the channel is known.
  const startCommand =
    channel === "brew"
      ? "brew services start microbridge"
      : channel === "direct"
        ? "launchctl kickstart -k gui/$(id -u)/ai.microbridge.daemon"
        : null;

  return (
    <div
      className="flex h-screen w-full items-start justify-center bg-transparent pt-1"
      style={{ fontFamily: "Inter, system-ui, sans-serif" }}
    >
      <div
        ref={cardRef}
        className="mb-frost flex max-h-[calc(100vh-8px)] w-[360px] flex-col overflow-hidden rounded-2xl"
        style={{
          maxHeight,
          backgroundColor: t.panel,
          border: `1px solid ${t.panelBorder}`,
          boxShadow: t.floatingShadow,
        }}
      >
        <div className="flex items-center gap-2 px-4 pb-3 pt-3.5">
          <span
            className="text-[13px] font-semibold"
            style={{ color: t.text }}
          >
            Microbridge
          </span>
          <span
            className="ml-auto flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium"
            style={{ backgroundColor: t.hoverBg, color: t.textSecondary }}
          >
            <span
              className="h-[6px] w-[6px] rounded-full"
              style={{ backgroundColor: t.textMuted }}
            />
            Not running
          </span>
        </div>

        <div className="px-4 pb-4">
          <p className="text-[14px] font-semibold" style={{ color: t.text }}>
            Waiting for microbridged
          </p>
          <p
            className="mt-1 text-[12px] leading-relaxed"
            style={{ color: t.textSecondary }}
          >
            The menu bar app reads your sessions from the local daemon. It isn't
            answering yet — start it and this window fills in on its own.
          </p>

          {startCommand && (
            <pre
              className="mt-3 overflow-x-auto rounded-lg px-3 py-2 text-[11px]"
              style={{ backgroundColor: t.sunken, color: t.text }}
            >
              {startCommand}
            </pre>
          )}
        </div>

        <div
          className="flex items-center gap-1 px-2.5 py-2"
          style={{ borderTop: `1px solid ${t.hairline}` }}
        >
          <FooterButton label="Settings" onClick={onOpenSettings} t={t} />
          <span className="ml-auto">
            <FooterButton
              label={view === "settings" ? "Close" : "Quit"}
              onClick={view === "settings" ? onClose : onQuit}
              t={t}
            />
          </span>
        </div>
      </div>
    </div>
  );
}

function FooterButton({
  label,
  onClick,
  t,
}: {
  label: string;
  onClick: () => void;
  t: ThemeTokens;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded-md px-2 py-1 text-[12px] font-medium transition-colors"
      style={{ color: t.textSecondary, backgroundColor: "transparent" }}
      onMouseEnter={(e) => {
        e.currentTarget.style.backgroundColor = t.hoverBg;
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = "transparent";
      }}
    >
      {label}
    </button>
  );
}
