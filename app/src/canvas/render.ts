// Canvas 2D renderer for display lists computed in Rust. Only screen transforms here.
import type { DisplayList } from "../bindings/DisplayList";
import type { FillKind } from "../bindings/FillKind";
import type { Dash } from "../bindings/Dash";
import type { Pt } from "../bindings/Pt";
import { renderImage } from "./images";

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
  /** Revit's sketch line color. */
  sketch: "#c832b4",
  cyanFill: "rgba(62, 207, 247, 0.8)",
  hover: "rgba(62, 207, 247, 0.6)",
};

const FILL: Record<FillKind, string> = {
  Poche: "#3b3b3b",
  PocheLight: "#b4b4ae",
  Paper: "#ffffff",
  Slab: "#efefeb",
  Ceiling: "#ffffff",
  Ink: "#0a0a0a",
  Glass: "#dff5fd",
  Room: "rgba(0, 0, 0, 0)",
  Accent: "#3ECFF7",
};

/** Pen weights 1–6 in screen pixels. */
const PEN = [0, 0.8, 1.0, 1.2, 1.6, 2.4, 3.4];

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
  /** Pre-highlighted under the cursor: one element, or Tab's candidate (ADR-056). */
  hover: string | ReadonlySet<string> | null;
  /** Sketch mode: the model draws faded (Revit's halftone), minus the element edited. */
  faded?: boolean;
  hidden?: string | null;
  /** Temporary Hide/Isolate: which elements to draw (ADR-024). */
  visible?: (el: string | null) => boolean;
  /** Thin Lines (TL): every line one pixel. */
  thin?: boolean;
  /** Drawn on the paper, under everything (the satellite overlay, ADR-026). */
  underlay?: (ctx: CanvasRenderingContext2D, S: (x: number, y: number) => [number, number]) => void;
  /** Drawn grayed out (halftone): Gray Inactive Workset Graphics, and the architecture
   * under the structural overlay (ADR-079, ADR-080). */
  grayed?: (el: string | null) => boolean;
  /** Drawn over everything at full strength (the structural overlay, ADR-080). */
  overlay?: (ctx: CanvasRenderingContext2D, S: (x: number, y: number) => [number, number]) => void;
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
  if (hl.underlay) {
    ctx.save();
    hl.underlay(ctx, S);
    ctx.restore();
  }
  const base = hl.faded ? 0.4 : 1;
  ctx.globalAlpha = base;

  for (const item of dl.items) {
    const el = item.el;
    if (hl.hidden && el === hl.hidden) continue;
    if (hl.visible && !hl.visible(el)) continue;
    if (hl.grayed) ctx.globalAlpha = hl.grayed(el) ? base * 0.3 : base;
    const isSel = el !== null && hl.selected.has(el);
    const isHover =
      !isSel &&
      el !== null &&
      (typeof hl.hover === "string" ? hl.hover === el : (hl.hover?.has(el) ?? false));
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
        ctx.lineWidth = (hl.thin ? 1 : (PEN[p.w] ?? 1)) + (isSel ? 1.2 : 0);
        ctx.stroke();
        break;
      }
      case "Image": {
        const img = renderImage(p.image);
        const [ax, ay] = S(p.min[0], p.max[1]);
        const [bx, by] = S(p.max[0], p.min[1]);
        if (img) ctx.drawImage(img, ax, ay, bx - ax, by - ay);
        else {
          ctx.fillStyle = "#e8e6e1";
          ctx.fillRect(ax, ay, bx - ax, by - ay);
        }
        if (isSel || isHover) {
          ctx.setLineDash([]);
          ctx.strokeStyle = isSel ? THEME.cyan : THEME.hover;
          ctx.lineWidth = 2;
          ctx.strokeRect(ax, ay, bx - ax, by - ay);
        }
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
        if (px < 0.4) break;
        const [sx, sy] = S(p.at[0], p.at[1]);
        if (px < 3 && !p.angle) {
          // Too small to read: a gray bar the text's length, as Revit greeks text when
          // zoomed out (ADR-106), so a sheet's plans keep their rooms, tags and notes.
          const tw = p.text.length * px * 0.5;
          const x0 = p.anchor === "Left" ? sx : p.anchor === "Right" ? sx - tw : sx - tw / 2;
          ctx.fillStyle = isSel ? THEME.cyan : "rgba(10, 10, 10, 0.45)";
          ctx.fillRect(x0, sy - px * 0.35, tw, Math.max(px * 0.7, 0.6));
          break;
        }
        ctx.font = `600 ${px}px "Barlow Condensed", "Barlow", sans-serif`;
        ctx.textAlign = p.anchor === "Left" ? "left" : p.anchor === "Right" ? "right" : "center";
        ctx.textBaseline = "middle";
        ctx.fillStyle = isSel ? THEME.cyan : THEME.ink;
        if (p.angle) {
          // Model angles are counter-clockwise with y up; the canvas y axis points down.
          ctx.save();
          ctx.translate(sx, sy);
          ctx.rotate(-p.angle);
          ctx.fillText(p.text, 0, 0);
          ctx.restore();
        } else {
          ctx.fillText(p.text, sx, sy);
        }
        break;
      }
    }
  }
  if (hl.overlay) {
    ctx.globalAlpha = 1;
    ctx.setLineDash([]);
    hl.overlay(ctx, S);
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

/** Grips for the selection (hovered grip filled cyan): squares, and a text note's as Revit
 * draws them (ADR-108): the four-arrow move grip and the rotate grip on its box's top
 * corners, and round grips for its width and leaders. */
export function drawGrips(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  grips: { at: Pt; key?: string }[],
  hover: number | null,
) {
  ctx.save();
  ctx.setLineDash([]);
  grips.forEach((g, i) => {
    const [sx, sy] = toScreen(cam, w, h, g.at.x, g.at.y);
    const key = g.key ?? "";
    if (key === "text_move_grip") {
      drawMoveGrip(ctx, sx, sy, i === hover);
      return;
    }
    if (key === "text_rotate") {
      drawRotateGrip(ctx, sx, sy, i === hover);
      return;
    }
    if (key.startsWith("text_width") || key.startsWith("leader:")) {
      ctx.beginPath();
      ctx.arc(sx, sy, 4.5, 0, Math.PI * 2);
      ctx.fillStyle = i === hover ? THEME.cyan : "#2f7fd8";
      ctx.fill();
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.2;
      ctx.stroke();
      return;
    }
    ctx.fillStyle = i === hover ? THEME.cyan : "#ffffff";
    ctx.strokeStyle = THEME.ink;
    ctx.lineWidth = 1.5;
    ctx.fillRect(sx - 5, sy - 5, 10, 10);
    ctx.strokeRect(sx - 5, sy - 5, 10, 10);
  });
  ctx.restore();
}

/** How near (px) a click must be to a flip control's center. */
export const FLIP_HIT = 11;

/** Revit's flip controls: a blue double-headed arrow along each control's direction
 * (left/right for a door's hand, up/down for its facing), a fixed size on screen. */
export function drawFlipControls(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  flips: { at: Pt; dir: Pt }[],
  hover: number | null,
) {
  ctx.save();
  ctx.setLineDash([]);
  ctx.lineJoin = "round";
  flips.forEach((f, i) => {
    const [cx, cy] = toScreen(cam, w, h, f.at.x, f.at.y);
    const [tx, ty] = toScreen(cam, w, h, f.at.x + f.dir.x, f.at.y + f.dir.y);
    const len = Math.hypot(tx - cx, ty - cy) || 1;
    const ux = (tx - cx) / len;
    const uy = (ty - cy) / len;
    const [px, py] = [-uy, ux];
    // Shaft and two heads, as one closed outline: 20 px long, heads 5 px wide.
    const L = 10;
    const head = 5;
    const shaft = 1.6;
    const pt = (a: number, b: number): [number, number] => [
      cx + ux * a + px * b,
      cy + uy * a + py * b,
    ];
    const outline = [
      pt(L, 0),
      pt(L - head, head),
      pt(L - head, shaft),
      pt(-L + head, shaft),
      pt(-L + head, head),
      pt(-L, 0),
      pt(-L + head, -head),
      pt(-L + head, -shaft),
      pt(L - head, -shaft),
      pt(L - head, -head),
    ];
    if (i === hover) {
      ctx.fillStyle = "rgba(0, 120, 215, 0.14)";
      ctx.beginPath();
      ctx.arc(cx, cy, FLIP_HIT, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.beginPath();
    outline.forEach(([x, y], k) => (k ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
    ctx.fillStyle = i === hover ? "#0a84ff" : "#1f6fd1";
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1;
    ctx.fill();
    ctx.stroke();
  });
  ctx.restore();
}

/** Screen-space box of a temporary dimension's value, for drawing and clicking. */
export function tempDimBox(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  labelAt: Pt,
  value: string,
): [number, number, number, number] {
  ctx.save();
  ctx.font = `600 12px "Barlow Condensed", sans-serif`;
  const tw = ctx.measureText(value).width;
  ctx.restore();
  const [sx, sy] = toScreen(cam, w, h, labelAt.x, labelAt.y);
  return [sx - tw / 2 - 6, sy - 10, tw + 12, 20];
}

/** Temporary dimensions: thin cyan dimension lines and a clickable value box. */
export function drawTempDims(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  dims: { items: DisplayList["items"]; labelAt: Pt; value: string }[],
) {
  ctx.save();
  for (const d of dims) {
    ctx.strokeStyle = THEME.cyan;
    ctx.setLineDash([]);
    ctx.lineWidth = 1;
    for (const item of d.items) {
      const p = item.prim;
      if (p.t !== "Line") continue;
      ctx.beginPath();
      p.pts.forEach(([x, y], i) => {
        const [sx, sy] = toScreen(cam, w, h, x, y);
        if (i === 0) ctx.moveTo(sx, sy);
        else ctx.lineTo(sx, sy);
      });
      ctx.stroke();
    }
    const [bx, by, bw, bh] = tempDimBox(ctx, cam, w, h, d.labelAt, d.value);
    ctx.fillStyle = "#ffffff";
    ctx.strokeStyle = THEME.cyan;
    ctx.lineWidth = 1.5;
    ctx.fillRect(bx, by, bw, bh);
    ctx.strokeRect(bx, by, bw, bh);
    ctx.fillStyle = THEME.ink;
    ctx.font = `600 12px "Barlow Condensed", sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(d.value, bx + bw / 2, by + bh / 2 + 0.5);
  }
  ctx.restore();
}

/** A highlighted reference line (Align's first pick). */
export function drawRefLine(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  a: Pt,
  b: Pt,
) {
  ctx.save();
  const [ax, ay] = toScreen(cam, w, h, a.x, a.y);
  const [bx, by] = toScreen(cam, w, h, b.x, b.y);
  ctx.strokeStyle = THEME.cyan;
  ctx.lineWidth = 3;
  ctx.setLineDash([8, 4]);
  ctx.beginPath();
  ctx.moveTo(ax, ay);
  ctx.lineTo(bx, by);
  ctx.stroke();
  ctx.restore();
}

/** A boundary sketch (ADR-021): magenta lines as Revit draws them; selected lines cyan,
 * lines Finish complained about red, the draw tool's preview dashed, and vertex grips. */
export function drawSketch(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  curves: { pts: Pt[]; locked: boolean }[],
  selected: Set<number>,
  bad: Set<number>,
  preview: Pt[][],
  grips: Pt[],
) {
  ctx.save();
  const S = (p: Pt) => toScreen(cam, w, h, p.x, p.y);
  const path = (pts: Pt[]) => {
    ctx.beginPath();
    pts.forEach((p, i) => {
      const [sx, sy] = S(p);
      if (i === 0) ctx.moveTo(sx, sy);
      else ctx.lineTo(sx, sy);
    });
  };
  curves.forEach((c, i) => {
    path(c.pts);
    ctx.setLineDash([]);
    ctx.lineWidth = selected.has(i) ? 3 : 2;
    ctx.strokeStyle = bad.has(i) ? "#e0261d" : selected.has(i) ? THEME.cyan : THEME.sketch;
    ctx.stroke();
    if (c.locked && selected.has(i) && c.pts.length >= 2) {
      // A lock mark at the middle of a line locked to its wall.
      const a = c.pts[0]!;
      const b = c.pts[c.pts.length - 1]!;
      const [mx, my] = S({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = THEME.ink;
      ctx.strokeRect(mx - 4, my - 1, 8, 6);
      ctx.beginPath();
      ctx.arc(mx, my - 1, 3, Math.PI, 0);
      ctx.stroke();
    }
  });
  ctx.setLineDash([6, 4]);
  ctx.lineWidth = 1.5;
  ctx.strokeStyle = THEME.sketch;
  for (const p of preview) {
    path(p);
    ctx.stroke();
  }
  ctx.setLineDash([]);
  for (const g of grips) {
    const [sx, sy] = S(g);
    ctx.fillStyle = "#ffffff";
    ctx.strokeStyle = THEME.ink;
    ctx.lineWidth = 1.5;
    ctx.fillRect(sx - 4, sy - 4, 8, 8);
    ctx.strokeRect(sx - 4, sy - 4, 8, 8);
  }
  ctx.restore();
}

/** Revit's Temporary Hide/Isolate frame: a cyan border with its label. */
export function drawTempFrame(ctx: CanvasRenderingContext2D, w: number, h: number) {
  ctx.save();
  ctx.strokeStyle = THEME.cyan;
  ctx.lineWidth = 4;
  ctx.setLineDash([]);
  ctx.strokeRect(2, 2, w - 4, h - 4);
  ctx.font = "600 11px Barlow, sans-serif";
  const label = "TEMPORARY HIDE/ISOLATE";
  const tw = ctx.measureText(label).width + 12;
  ctx.fillStyle = THEME.cyan;
  ctx.fillRect(8, 8, tw, 18);
  ctx.fillStyle = THEME.ink;
  ctx.fillText(label, 14, 21);
  ctx.restore();
}

/** Revit's selection box: a window (dragged left to right) drawn solid, a crossing (right
 * to left) dashed. */
export function drawSelectBox(
  ctx: CanvasRenderingContext2D,
  a: [number, number],
  b: [number, number],
) {
  const crossing = b[0] < a[0];
  const [x, y, w, h] = [
    Math.min(a[0], b[0]),
    Math.min(a[1], b[1]),
    Math.abs(b[0] - a[0]),
    Math.abs(b[1] - a[1]),
  ];
  ctx.save();
  ctx.fillStyle = crossing ? "rgba(62, 207, 247, 0.06)" : "rgba(62, 207, 247, 0.1)";
  ctx.fillRect(x, y, w, h);
  ctx.setLineDash(crossing ? [5, 4] : []);
  ctx.strokeStyle = THEME.cyan;
  ctx.lineWidth = 1.25;
  ctx.strokeRect(x, y, w, h);
  ctx.restore();
}

/** Zoom region rubber band. */
export function drawZoomBox(
  ctx: CanvasRenderingContext2D,
  a: [number, number],
  b: [number, number],
) {
  ctx.save();
  ctx.setLineDash([4, 3]);
  ctx.strokeStyle = THEME.cyan;
  ctx.lineWidth = 1.5;
  ctx.strokeRect(
    Math.min(a[0], b[0]),
    Math.min(a[1], b[1]),
    Math.abs(b[0] - a[0]),
    Math.abs(b[1] - a[1]),
  );
  ctx.restore();
}

/** An image placed by its corners in plan (lower left, lower right, upper right, upper left),
 * faded like an underlay, with Google's attribution. */
export function drawImageUnder(
  ctx: CanvasRenderingContext2D,
  S: (x: number, y: number) => [number, number],
  image: CanvasImageSource & { width: number; height: number },
  corners: { x: number; y: number }[],
  alpha: number,
  credit: string,
) {
  const [ll, , ur, ul] = corners.map((c) => S(c.x, c.y)) as [number, number][];
  const w = image.width || 1;
  const h = image.height || 1;
  ctx.save();
  ctx.globalAlpha = alpha;
  // Image x runs from its upper left to upper right, y from upper left down to lower left.
  ctx.transform(
    (ur![0] - ul![0]) / w,
    (ur![1] - ul![1]) / w,
    (ll![0] - ul![0]) / h,
    (ll![1] - ul![1]) / h,
    ul![0],
    ul![1],
  );
  ctx.drawImage(image, 0, 0);
  ctx.restore();
  ctx.save();
  ctx.font = "10px Barlow, sans-serif";
  ctx.fillStyle = "rgba(28,28,28,0.75)";
  ctx.fillText(credit, 8, ctx.canvas.height / (window.devicePixelRatio || 1) - 8);
  ctx.restore();
}

/** Placing a camera: before the first click the camera sits at the cursor facing up the
 * screen; after it, the camera is at the eye with its view cone reaching the cursor (the
 * target). 50° vertical field of view on a 16:9 view, as a new camera gets. */
export function drawCameraGhost(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  eye: { x: number; y: number } | null,
  cursor: { x: number; y: number },
) {
  const [cx, cy] = toScreen(cam, w, h, cursor.x, cursor.y);
  const [ex, ey] = eye ? toScreen(cam, w, h, eye.x, eye.y) : [cx, cy];
  const d = eye ? Math.hypot(cx - ex, cy - ey) : 0;
  // Screen direction the camera looks (up the screen until the target is picked).
  const ux = d > 1 ? (cx - ex) / d : 0;
  const uy = d > 1 ? (cy - ey) / d : -1;
  const nx = -uy;
  const ny = ux;
  ctx.save();
  ctx.strokeStyle = THEME.cyan;
  ctx.fillStyle = THEME.cyan;
  ctx.lineWidth = 1.5;
  if (d > 1) {
    const half = Math.atan(Math.tan((25 * Math.PI) / 180) * (16 / 9));
    const reach = d / Math.cos(half);
    const edge = (s: number) => {
      const c = Math.cos(half);
      const sn = Math.sin(half) * s;
      return [ex + (ux * c - nx * sn) * reach, ey + (uy * c - ny * sn) * reach] as const;
    };
    const [lx, ly] = edge(1);
    const [rx, ry] = edge(-1);
    ctx.setLineDash([6, 4]);
    ctx.beginPath();
    ctx.moveTo(lx, ly);
    ctx.lineTo(ex, ey);
    ctx.lineTo(rx, ry);
    ctx.moveTo(lx, ly);
    ctx.lineTo(rx, ry);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.arc(cx, cy, 4, 0, Math.PI * 2);
    ctx.stroke();
  }
  // The camera body behind the eye, and the lens toward the target.
  const back = [ex - ux * 7, ey - uy * 7];
  const body = [
    [back[0]! + nx * 7, back[1]! + ny * 7],
    [back[0]! - nx * 7, back[1]! - ny * 7],
    [back[0]! - nx * 7 - ux * 16, back[1]! - ny * 7 - uy * 16],
    [back[0]! + nx * 7 - ux * 16, back[1]! + ny * 7 - uy * 16],
  ];
  ctx.beginPath();
  body.forEach(([x, y], i) => (i ? ctx.lineTo(x!, y!) : ctx.moveTo(x!, y!)));
  ctx.closePath();
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(ex, ey);
  ctx.lineTo(back[0]! + nx * 5, back[1]! + ny * 5);
  ctx.lineTo(back[0]! - nx * 5, back[1]! - ny * 5);
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

/** Revit's move grip: a four-way arrow, 18 px across. */
function drawMoveGrip(ctx: CanvasRenderingContext2D, x: number, y: number, hover: boolean) {
  const L = 9;
  const H = 3.5;
  ctx.save();
  ctx.strokeStyle = hover ? THEME.cyan : "#2f7fd8";
  ctx.fillStyle = ctx.strokeStyle;
  ctx.lineWidth = 1.6;
  ctx.beginPath();
  ctx.moveTo(x - L, y);
  ctx.lineTo(x + L, y);
  ctx.moveTo(x, y - L);
  ctx.lineTo(x, y + L);
  ctx.stroke();
  for (const [dx, dy] of [
    [1, 0],
    [-1, 0],
    [0, 1],
    [0, -1],
  ] as const) {
    const tx = x + dx * L;
    const ty = y + dy * L;
    ctx.beginPath();
    ctx.moveTo(tx, ty);
    ctx.lineTo(tx - dx * H - dy * H, ty - dy * H - dx * H);
    ctx.lineTo(tx - dx * H + dy * H, ty - dy * H + dx * H);
    ctx.closePath();
    ctx.fill();
  }
  ctx.restore();
}

/** Revit's rotate grip: a round arrow, 16 px across. */
function drawRotateGrip(ctx: CanvasRenderingContext2D, x: number, y: number, hover: boolean) {
  const r = 7;
  ctx.save();
  ctx.strokeStyle = hover ? THEME.cyan : "#2f7fd8";
  ctx.fillStyle = ctx.strokeStyle;
  ctx.lineWidth = 1.8;
  ctx.beginPath();
  ctx.arc(x, y, r, -Math.PI * 0.15, Math.PI * 1.35);
  ctx.stroke();
  // The arrowhead at the arc's start, pointing along it.
  const a = -Math.PI * 0.15;
  const ex = x + r * Math.cos(a);
  const ey = y + r * Math.sin(a);
  ctx.beginPath();
  ctx.moveTo(ex + 4, ey + 1);
  ctx.lineTo(ex - 2.5, ey - 3.5);
  ctx.lineTo(ex - 1, ey + 4);
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

/** Selected text notes' boxes (ADR-108): a thin blue outline, turned with each note;
 * `ghost` draws one dashed where a drag will leave it. */
export function drawTextFrames(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  w: number,
  h: number,
  frames: { corners: Pt[] }[],
  ghost: Pt[] | null,
) {
  ctx.save();
  const poly = (pts: Pt[]) => {
    ctx.beginPath();
    pts.forEach((p, i) => {
      const [sx, sy] = toScreen(cam, w, h, p.x, p.y);
      if (i === 0) ctx.moveTo(sx, sy);
      else ctx.lineTo(sx, sy);
    });
    ctx.closePath();
    ctx.stroke();
  };
  ctx.strokeStyle = "#2f7fd8";
  ctx.lineWidth = 1;
  ctx.setLineDash([]);
  for (const f of frames) poly(f.corners);
  if (ghost) {
    ctx.strokeStyle = THEME.cyan;
    ctx.lineWidth = 1.5;
    ctx.setLineDash([5, 3]);
    poly(ghost);
  }
  ctx.restore();
}

/** Where a text note's box goes while one of its grips is dragged (ADR-108), for the live
 * preview: moved, turned about its centre, or widened on the dragged side. */
export function textFrameGhost(corners: Pt[], key: string, from: Pt, to: Pt): Pt[] | null {
  if (corners.length !== 4) return null;
  const [a, b, , d] = corners as [Pt, Pt, Pt, Pt];
  if (key === "text_move_grip" || key === "text_move") {
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    return corners.map((p) => ({ x: p.x + dx, y: p.y + dy }));
  }
  if (key === "text_rotate") {
    const c = {
      x: corners.reduce((s, p) => s + p.x, 0) / 4,
      y: corners.reduce((s, p) => s + p.y, 0) / 4,
    };
    let t = Math.atan2(to.y - c.y, to.x - c.x) - Math.atan2(from.y - c.y, from.x - c.x);
    // Snaps near multiples of 15°, as the drop does.
    const turned = Math.atan2(b.y - a.y, b.x - a.x) + t;
    const step = Math.PI / 12;
    const snap = Math.round(turned / step) * step;
    if (Math.abs(turned - snap) < (3 * Math.PI) / 180) t += snap - turned;
    const [s, co] = [Math.sin(t), Math.cos(t)];
    return corners.map((p) => ({
      x: c.x + (p.x - c.x) * co - (p.y - c.y) * s,
      y: c.y + (p.x - c.x) * s + (p.y - c.y) * co,
    }));
  }
  if (key === "text_width" || key === "text_width_left") {
    const len = Math.hypot(b.x - a.x, b.y - a.y) || 1;
    const u = { x: (b.x - a.x) / len, y: (b.y - a.y) / len };
    const along = (to.x - from.x) * u.x + (to.y - from.y) * u.y;
    const shift = (p: Pt) => ({ x: p.x + u.x * along, y: p.y + u.y * along });
    const out = corners.slice();
    if (key === "text_width") {
      out[1] = shift(corners[1]!);
      out[2] = shift(corners[2]!);
    } else {
      out[0] = shift(corners[0]!);
      out[3] = shift(d);
    }
    return out;
  }
  return null;
}
