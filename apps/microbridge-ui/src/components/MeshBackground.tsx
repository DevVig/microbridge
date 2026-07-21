import { useEffect, useRef } from "react";

type Node = { x: number; y: number };

/** Stable low-density lattice in normalized 0–1 space. */
const NODES: Node[] = (() => {
  const nodes: Node[] = [];
  const cols = 6;
  const rows = 5;
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      const jitterX = ((row * 17 + col * 31) % 7) / 100 - 0.03;
      const jitterY = ((col * 13 + row * 23) % 7) / 100 - 0.03;
      nodes.push({
        x: (col + 0.5) / cols + jitterX,
        y: (row + 0.5) / rows + jitterY,
      });
    }
  }
  return nodes;
})();

const EDGE_DIST = 0.22;
const INFLUENCE_PX = 120;

/**
 * Subtle mouse-reactive mesh behind Settings chrome.
 * pointer-events-none — never blocks sidebar or tile clicks.
 */
export function MeshBackground({
  dark,
  active = true,
}: {
  dark: boolean;
  active?: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseRef = useRef<{ x: number; y: number } | null>(null);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !active) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const parent = canvas.parentElement;
    if (!parent) return;

    const baseAlpha = dark ? 0.055 : 0.07;
    const hotAlpha = dark ? 0.18 : 0.22;
    const stroke = dark ? "245,245,244" : "13,13,13";

    const resize = () => {
      const rect = parent.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.max(1, Math.floor(rect.width * dpr));
      canvas.height = Math.max(1, Math.floor(rect.height * dpr));
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(parent);

    const onMove = (event: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      mouseRef.current = {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top,
      };
    };
    const onLeave = () => {
      mouseRef.current = null;
    };

    parent.addEventListener("mousemove", onMove);
    parent.addEventListener("mouseleave", onLeave);

    const draw = () => {
      if (document.hidden) {
        rafRef.current = requestAnimationFrame(draw);
        return;
      }

      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      ctx.clearRect(0, 0, width, height);

      const points = NODES.map((node) => ({
        x: node.x * width,
        y: node.y * height,
      }));
      const mouse = mouseRef.current;

      for (let i = 0; i < points.length; i += 1) {
        for (let j = i + 1; j < points.length; j += 1) {
          const a = points[i]!;
          const b = points[j]!;
          const dx = a.x - b.x;
          const dy = a.y - b.y;
          const distNorm = Math.hypot(dx / width, dy / height);
          if (distNorm > EDGE_DIST) continue;

          let alpha = baseAlpha * (1 - distNorm / EDGE_DIST);
          if (mouse) {
            const midX = (a.x + b.x) / 2;
            const midY = (a.y + b.y) / 2;
            const near = Math.hypot(midX - mouse.x, midY - mouse.y);
            if (near < INFLUENCE_PX) {
              const t = 1 - near / INFLUENCE_PX;
              alpha = Math.min(hotAlpha, alpha + t * (hotAlpha - baseAlpha));
            }
          }

          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(b.x, b.y);
          ctx.strokeStyle = `rgba(${stroke},${alpha.toFixed(3)})`;
          ctx.lineWidth = 1;
          ctx.stroke();
        }
      }

      for (const point of points) {
        let alpha = baseAlpha * 1.4;
        let radius = 1.15;
        if (mouse) {
          const near = Math.hypot(point.x - mouse.x, point.y - mouse.y);
          if (near < INFLUENCE_PX) {
            const t = 1 - near / INFLUENCE_PX;
            alpha = Math.min(hotAlpha + 0.05, alpha + t * 0.14);
            radius = 1.15 + t * 0.9;
          }
        }
        ctx.beginPath();
        ctx.arc(point.x, point.y, radius, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${stroke},${alpha.toFixed(3)})`;
        ctx.fill();
      }

      rafRef.current = requestAnimationFrame(draw);
    };

    rafRef.current = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(rafRef.current);
      observer.disconnect();
      parent.removeEventListener("mousemove", onMove);
      parent.removeEventListener("mouseleave", onLeave);
    };
  }, [active, dark]);

  if (!active) return null;

  return (
    <canvas
      ref={canvasRef}
      aria-hidden
      className="pointer-events-none absolute inset-0 z-0"
    />
  );
}
