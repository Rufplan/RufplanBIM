// Canvas 2D renderer for display lists computed in Rust. Only screen transforms here.
import type { DisplayList } from "../bindings/DisplayList";
import type { FillKind } from "../bindings/FillKind";
import type { Dash } from "../bindings/Dash";
import type { Pt } from "../bindings/Pt";

/** Model point at the canvas center, and pixels per model mm. */
export interface Camera {
  cx: number;
  cy: number;
  zoom: number;
}

export const THEME = {
  paper: "#ffffff",
  ink: "#0a0a0a",
  inkMid: "#1c1c1c",
  muted: "#888888",
  cyan: "#3ECFF7",
  cyanFill: "rgba(62, 207, 247, 0.8)",
  hover: "rgba(62, 207, 247, 0.6)",
};

const FILL: Record<FillKind, string> = {
  Poche: "#3b3b3b",
  Paper: "#ffffff",
  Slab: "#efefeb",
  Ceiling: "#f2fbfe",
  Ink: "#0a0a0a",
  Glass: "#dff5fd",
  Room: "rgba(0, 0, 0, 0)",
};

/** Pen weights 1–6 in screen pixels. */
const PEN = [0, 0.5, 0.8, 1.1, 1.6, 2.4, 3.4];

const DASH: Record<Dash, number[]> = {
  Solid: [],
  Dashed: [6, 4],
  Center: [16, 4, 3, 4],
};

export function toScreen(
  cam: Camera,
  w: number,
  h: number,
  x: number,
  y: number,
): [number, number] {
  return [w / 2 + (x - cam.cx) * cam.zoom, h / 2 - (y - cam.cy) * cam.zoom];
}

export function toModel(cam: Camera, w: number, h: number, sx: number, sy: number): Pt {
  return { x: cam.cx + (sx - w / 2) / cam.zoom, y: cam.cy - (sy - h / 2) / cam.zoom };
}

/** Camera that fits `bounds` into a w × h canvas with a small margin. */
export function fit(bounds: [number, number, number, number], w: number, h: number): Camera {
  const [x0, y0, x1, y1] = bounds;
  const bw = Math.max(x1 - x0, 1);
  const bh = Math.max(y1 - y0, 1);
  const zoom = Math.min((w * 0.92) / bw, (h * 0.92) / bh);
  return { cx: (x0 + x1) / 2, cy: (y0 + y1) / 2, zoom: zoom > 0 && isFinite(zoom) ? zoom : 0.05 };
}

/** Zooms by `factor` keeping the model point under (sx, sy) fixed. */
export function zoomAt(
  cam: Camera,
  w: number,
  h: number,
  sx: number,
  sy: number,
  factor: number,
): Camera {
  const before = toModel(cam, w, h, sx, sy);
  const zoom = Math.min(Math.max(cam.zoom * factor, 0.001), 20);
  const next = { ...cam, zoom };
  const after = toModel(next, w, h, sx, sy);
  return { zoom, cx: cam.cx + before.x - after.x, cy: cam.cy + before.y - after.y };
}

export interface Highlight {
  selected: Set<string>;
  hover: string | null;
}

export function draw(
  ctx: CanvasRenderingContext2D,
  dl: DisplayList,
  cam: Camera,
  w: number,
  h: number,
  hl: Highlight,
) {
  ctx.save();
  ctx.fillStyle = THEME.paper;
  ctx.fillRect(0, 0, w, h);
  ctx.lineJoin = "miter";
  ctx.lineCap = "butt";
  const S = (x: number, y: number) => toScreen(cam, w, h, x, y);

  for (const item of dl.items) {
    const el = item.el;
    const isSel = el !== null && hl.selected.has(el);
    const isHover = !isSel && el !== null && hl.hover === el;
    const p = item.prim;
    switch (p.t) {
      case "Fill": {
        ctx.beginPath();
        for (const ring of p.rings) {
          ring.forEach(([x, y], i) => {
            const [sx, sy] = S(x, y);
            if (i === 0) ctx.moveTo(sx, sy);
            else ctx.lineTo(sx, sy);
          });
          ctx.closePath();
        }
        ctx.fillStyle = FILL[p.fill];
        ctx.fill("evenodd");
        if (isSel || isHover) {
          // Rooms cover whole areas, so their highlight is a light wash.
          const room = p.fill === "Room";
          ctx.fillStyle = isSel
            ? room
              ? "rgba(62, 207, 247, 0.16)"
              : THEME.cyanFill
            : room
              ? "rgba(62, 207, 247, 0.07)"
              : "rgba(62, 207, 247, 0.18)";
          ctx.fill("evenodd");
        }
        break;
      }
      case "Line": {
        ctx.beginPath();
        p.pts.forEach(([x, y], i) => {
          const [sx, sy] = S(x, y);
          if (i === 0) ctx.moveTo(sx, sy);
          else ctx.lineTo(sx, sy);
        });
        if (p.closed) ctx.closePath();
        ctx.setLineDash(DASH[p.dash]);
        ctx.strokeStyle = isSel ? THEME.cyan : isHover ? THEME.hover : THEME.ink;
        ctx.lineWidth = (PEN[p.w] ?? 1) + (isSel ? 1.2 : 0);
        ctx.stroke();
        break;
      }
      case "Circle": {
        const [sx, sy] = S(p.c[0], p.c[1]);
        ctx.beginPath();
        ctx.arc(sx, sy, Math.max(p.r * cam.zoom, 1), 0, Math.PI * 2);
        ctx.setLineDash([]);
        if (p.filled) {
          ctx.fillStyle = isSel ? THEME.cyan : THEME.ink;
          ctx.fill();
        } else {
          ctx.fillStyle = THEME.paper;
          ctx.fill();
          ctx.strokeStyle = isSel ? THEME.cyan : isHover ? THEME.hover : THEME.ink;
          ctx.lineWidth = PEN[p.w] ?? 1;
          ctx.stroke();
        }
        break;
      }
      case "Text": {
        const px = p.size * cam.zoom;
        if (px < 4) break;
        const [sx, sy] = S(p.at[0], p.at[1]);
        ctx.font = `600 ${px}px "Barlow Condensed", "Barlow", sans-serif`;
        ctx.textAlign = p.anchor === "Left" ? "left" : p.anchor === "Right" ? "right" : "center";
        ctx.textBaseline = "middle";
        ctx.fillStyle = isSel ? THEME.cyan : THEME.ink;
        ctx.fillText(p.text, sx, sy);
        break;
      }
    }
  }
  ctx.restore();
}

/** Placement preview items (from Rust) drawn in one color, with a label at the cursor. */
export function drawPreview(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  items: DisplayList["items"],
  color: string,
  label: string | null,
  cursor: Pt | null,
) {
  ctx.save();
  ctx.strokeStyle = color;
  ctx.setLineDash([]);
  for (const item of items) {
    const p = item.prim;
    if (p.t !== "Line") continue;
    ctx.beginPath();
    p.pts.forEach(([x, y], i) => {
      const [sx, sy] = toScreen(cam, w, h, x, y);
      if (i === 0) ctx.moveTo(sx, sy);
      else ctx.lineTo(sx, sy);
    });
    if (p.closed) ctx.closePath();
    ctx.setLineDash(DASH[p.dash]);
    ctx.lineWidth = (PEN[p.w] ?? 1) + 1;
    ctx.stroke();
  }
  if (label && cursor) {
    const [sx, sy] = toScreen(cam, w, h, cursor.x, cursor.y);
    ctx.font = `600 12px "Barlow Condensed", sans-serif`;
    const tw = ctx.measureText(label).width;
    ctx.fillStyle = THEME.ink;
    ctx.fillRect(sx + 14, sy + 12, tw + 14, 20);
    ctx.fillStyle = "#ffffff";
    ctx.textBaseline = "middle";
    ctx.textAlign = "left";
    ctx.fillText(label, sx + 21, sy + 22.5);
  }
  ctx.restore();
}

/** Tool overlay: rubber-band polyline, snap marker and length label. */
export function drawOverlay(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  pts: Pt[],
  cursor: Pt | null,
  snapKind: string | null,
  label: string | null,
  closeRing: boolean,
) {
  ctx.save();
  const S = (p: Pt) => toScreen(cam, w, h, p.x, p.y);
  const chain = cursor ? [...pts, cursor] : pts;
  if (chain.length > 1) {
    ctx.beginPath();
    chain.forEach((p, i) => {
      const [sx, sy] = S(p);
      if (i === 0) ctx.moveTo(sx, sy);
      else ctx.lineTo(sx, sy);
    });
    if (closeRing && chain.length > 2) ctx.closePath();
    ctx.setLineDash([]);
    ctx.strokeStyle = THEME.cyan;
    ctx.lineWidth = 2;
    ctx.stroke();
  }
  for (const p of pts) {
    const [sx, sy] = S(p);
    ctx.fillStyle = THEME.ink;
    ctx.fillRect(sx - 3, sy - 3, 6, 6);
  }
  if (cursor) {
    const [sx, sy] = S(cursor);
    ctx.strokeStyle = THEME.cyan;
    ctx.lineWidth = 1.5;
    ctx.setLineDash([]);
    if (snapKind && snapKind !== "None" && snapKind !== "Angle") {
      ctx.strokeRect(sx - 6, sy - 6, 12, 12);
    }
    ctx.beginPath();
    ctx.moveTo(sx - 10, sy);
    ctx.lineTo(sx + 10, sy);
    ctx.moveTo(sx, sy - 10);
    ctx.lineTo(sx, sy + 10);
    ctx.stroke();
    const text = [label, snapKind && snapKind !== "None" ? snapKind.toUpperCase() : null]
      .filter(Boolean)
      .join("   ");
    if (text) {
      ctx.font = `600 12px "Barlow Condensed", sans-serif`;
      const tw = ctx.measureText(text).width;
      ctx.fillStyle = THEME.ink;
      ctx.fillRect(sx + 14, sy + 12, tw + 14, 20);
      ctx.fillStyle = "#ffffff";
      ctx.textBaseline = "middle";
      ctx.textAlign = "left";
      ctx.fillText(text, sx + 21, sy + 22.5);
    }
  }
  ctx.restore();
}
