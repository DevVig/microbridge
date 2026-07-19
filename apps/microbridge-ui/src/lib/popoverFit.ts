/**
 * Fitting the popover window to its content and to the screen.
 *
 * The popover window used to be a fixed 380×540 frame that `position_below_tray`
 * only ever moved. That was wrong in both directions: on a short display (or
 * with a large Dock) the card ran past the bottom edge and took the footer with
 * it, and when the card was short the leftover transparent frame stayed
 * hit-testable, swallowing clicks meant for the app underneath.
 *
 * So the window is now a function of two things: what the card actually draws
 * (measured here) and how much room is left below the menu bar (measured in
 * Rust from the monitor work area, which on macOS already excludes the Dock).
 * See `resize_popover` / `popover_max_height` in src-tauri/src/lib.rs.
 */

import { useEffect, useRef, useState } from "react";
import { invokeQuiet, invokeTauri } from "./tauri";

/**
 * Space the window needs beyond the card itself: the wrapper's `pt-1` (4px),
 * plus slack for the card's drop shadow (`0 24px 64px` reaches ~56px below it).
 * Without the slack a window hugging the card would clip its own shadow.
 *
 * One constant governs both directions — it's added when asking for a window
 * size and subtracted when capping the card — so the two can't drift apart.
 */
export const POPOVER_CHROME = 4 + 52;

/**
 * Card height below which the popover has to choose what to spend space on.
 *
 * The device echo costs ~150px with its padding. On a card this short, keeping
 * it leaves under three thread rows — and on the shortest screens it squeezed
 * the list to nothing at all. The thread list is why the popover is open, so
 * below this the echo gives way. Only reachable on a small 1x display or with a
 * very large Dock; every current Mac display clears it comfortably.
 */
export const COMPACT_CARD_HEIGHT = 520;

/**
 * @param enabled Pass false when the surface is rendering into some other
 *   window. `Disconnected` renders in both the popover and the settings window,
 *   and `resize_popover` addresses the popover by label — a settings webview
 *   calling it would resize the wrong window.
 */
export function usePopoverFit<T extends HTMLElement>(enabled = true) {
  const ref = useRef<T | null>(null);
  const [windowHeight, setWindowHeight] = useState<number | null>(null);
  const [viewport, setViewport] = useState(() => {
    const height = typeof window === "undefined" ? 0 : window.innerHeight;
    // SSR and the Node test renderer have no meaningful viewport. Treat that
    // as normal-sized; compact mode should only follow a real measurement.
    return height > 0 ? height : COMPACT_CARD_HEIGHT + POPOVER_CHROME + 1;
  });

  // The cap depends on which monitor the popover is on, so besides the initial
  // read we listen for `popover-fit`, which Rust emits each time the popover is
  // positioned — that's when it may have moved to a different display.
  useEffect(() => {
    if (!enabled) return;
    let active = true;
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        const height = await invokeTauri<number>("popover_max_height");
        if (active && height !== null) setWindowHeight(height);
      } catch {
        /* no cap available — the CSS viewport fallback still applies */
      }
    })();

    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const un = await listen<number>("popover-fit", (event) => {
          if (active) setWindowHeight(event.payload);
        });
        if (active) unlisten = un;
        else un();
      } catch {
        /* not running under Tauri */
      }
    })();

    return () => {
      active = false;
      unlisten?.();
    };
  }, [enabled]);

  // Browser-preview fallback for the cap below. Under Tauri the cap comes from
  // the monitor, not from the window, so tracking the viewport there would only
  // re-render on every resize we ourselves asked for.
  useEffect(() => {
    if (windowHeight !== null) return;
    const onResize = () => setViewport(window.innerHeight);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [windowHeight]);

  // Report the card's height so the window hugs it. This fires while the
  // popover is hidden too, so by the time the tray icon is clicked the window
  // is normally already the right size — no resize flash on open.
  useEffect(() => {
    if (!enabled) return;
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;

    let last = 0;
    const observer = new ResizeObserver(() => {
      // getBoundingClientRect over contentRect: the card has a border, and the
      // window has to contain it.
      const height = el.getBoundingClientRect().height;
      if (height <= 0 || Math.abs(height - last) < 1) return;
      last = height;
      void invokeQuiet("resize_popover", { height: height + POPOVER_CHROME });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [enabled]);

  // What the card is allowed to be, in px. Outside Tauri that's the CSS
  // fallback's value (`max-h-[calc(100vh-8px)]`) so a browser preview is honest
  // about how cramped a given height is.
  const cardCap =
    windowHeight === null ? viewport - 8 : windowHeight - POPOVER_CHROME;

  return {
    ref,
    /** Card cap in px, or undefined outside Tauri (CSS falls back to the viewport). */
    maxHeight: windowHeight === null ? undefined : windowHeight - POPOVER_CHROME,
    /** Too little room to draw everything — drop what isn't the thread list. */
    compact: cardCap < COMPACT_CARD_HEIGHT,
  };
}
