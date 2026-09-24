import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  errorMessage,
  ipc,
  type DisplayList,
  type OpeningPreview,
  type Pt,
  type SnapResult,
  type ViewInfo,
} from "../ipc";
import { apply } from "../fileActions";
import { useAppStore } from "../store";
import {
  THEME,
  draw,
  drawOverlay,
  drawPreview,
  fit,
  toModel,
  toScreen,
  zoomAt,
  type Camera,
} from "../canvas/render";
import { closesSketch, promptFor, samePt, toolAllowed } from "../tools";

// Cameras survive tab switches.
const cameras = new Map<string, Camera>();

/** Runs async work one at a time, keeping only the most recent pending request. */
function useLatest<A extends unknown[]>(fn: (...args: A) => Promise<void>) {
  const busy = useRef(false);
  const pending = useRef<A | null>(null);
  const fnRef = useRef(fn);
  useLayoutEffect(() => {
    fnRef.current = fn;
  });
  return useCallback((...args: A) => {
    if (busy.current) {
      pending.current = args;
      return;
    }
    busy.current = true;
    const run = async (a: A) => {
      try {
        await fnRef.current(...a);
      } finally {
        const next = pending.current;
        pending.current = null;
        if (next) void run(next);
        else busy.current = false;
      }
    };
    void run(args);
  }, []);
}

export function ViewCanvas({ view }: { view: ViewInfo }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const tool = useAppStore((s) => s.tool);
  const selection = useAppStore((s) => s.selection);
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [dl, setDl] = useState<DisplayList | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const cam = useRef<Camera | null>(cameras.get(view.id) ?? null);
  const pts = useRef<Pt[]>([]);
  const snapRef = useRef<SnapResult | null>(null);
  const preview = useRef<{ p: OpeningPreview; at: Pt } | null>(null);
  const hover = useRef<string | null>(null);
  const drag = useRef<{ x: number; y: number; moved: boolean; button: number } | null>(null);
  const frame = useRef(0);
  const redrawRef = useRef<() => void>(() => {});

  const redraw = useCallback(() => {
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(() => {
      const canvas = canvasRef.current;
      const ctx = canvas?.getContext("2d");
      if (!canvas || !ctx || !dl || !cam.current) return;
      const dpr = window.devicePixelRatio || 1;
      const { w, h } = size;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      const s = useAppStore.getState();
      draw(ctx, dl, cam.current, w, h, { selected: new Set(s.selection), hover: hover.current });
      const placing = s.tool === "door" || s.tool === "window";
      if (placing && preview.current) {
        const { p, at } = preview.current;
        const color = p.valid ? THEME.cyan : "#c0352b";
        const label = p.valid ? p.label : "Overlaps another opening";
        drawPreview(ctx, cam.current, w, h, p.items, color, label, at);
      }
      const drawing = s.tool !== "select" && !placing && toolAllowed(s.tool, view.viewType);
      if (drawing) {
        const sn = snapRef.current;
        drawOverlay(
          ctx,
          cam.current,
          w,
          h,
          pts.current,
          sn?.pt ?? null,
          sn?.kind ?? null,
          sn?.label ?? null,
          s.tool === "floor" || s.tool === "ceiling",
        );
      }
    });
  }, [dl, size, view.viewType]);
  useLayoutEffect(() => {
    redrawRef.current = redraw;
  });

  // Fetch the display list whenever the model or view changes.
  useEffect(() => {
    let live = true;
    ipc.displayList(view.id).then(
      (d) => live && setDl(d),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [view.id, revision]);

  // Track canvas size.
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const r = el.getBoundingClientRect();
      setSize({ w: Math.floor(r.width), h: Math.floor(r.height) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || size.w === 0) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = size.w * dpr;
    canvas.height = size.h * dpr;
    if (dl && !cam.current) {
      cam.current = fit(dl.bounds, size.w, size.h);
      cameras.set(view.id, cam.current);
    }
    redraw();
  }, [size, dl, redraw, view.id]);

  useEffect(redraw, [selection, redraw]);

  // Tool changes reset the sketch and prompt. (Not on redraw changes: new display lists
  // arrive after every edit, and a wall chain must survive them.)
  useEffect(() => {
    pts.current = [];
    snapRef.current = null;
    preview.current = null;
    useAppStore.getState().setPrompt(promptFor(tool, 0, view.viewType));
    redrawRef.current();
  }, [tool, view.viewType]);

  const setCam = (c: Camera) => {
    cam.current = c;
    cameras.set(view.id, c);
    redraw();
  };

  const local = (e: { clientX: number; clientY: number }): [number, number] => {
    const r = canvasRef.current?.getBoundingClientRect();
    return r ? [e.clientX - r.left, e.clientY - r.top] : [0, 0];
  };

  const modelAt = (sx: number, sy: number) =>
    cam.current ? toModel(cam.current, size.w, size.h, sx, sy) : { x: 0, y: 0 };

  const hoverPick = useLatest(async (p: Pt, tol: number) => {
    const id = await ipc.pick(view.id, p, tol);
    if (id !== hover.current) {
      hover.current = id;
      redraw();
    }
  });

  const snapAt = useLatest(async (p: Pt, tol: number) => {
    const from = pts.current[pts.current.length - 1] ?? null;
    snapRef.current = await ipc.snap(view.id, p, from, tol);
    redraw();
  });

  const previewAt = useLatest(async (p: Pt, tol: number) => {
    const s = useAppStore.getState();
    const typeId = s.tool === "door" ? s.toolTypes.door : s.toolTypes.window;
    const result = typeId ? await ipc.openingPreview(view.id, typeId, p, tol) : null;
    preview.current = result ? { p: result, at: p } : null;
    redraw();
  });

  const finishSketch = useCallback(async () => {
    const s = useAppStore.getState();
    const ring = pts.current;
    pts.current = [];
    snapRef.current = null;
    if (ring.length >= 3) {
      if (s.tool === "floor" && s.toolTypes.floor)
        await apply(() => ipc.createFloor(view.id, s.toolTypes.floor!, ring));
      if (s.tool === "ceiling" && s.toolTypes.ceiling)
        await apply(() => ipc.createCeiling(view.id, s.toolTypes.ceiling!, ring, null));
    }
    s.setPrompt(promptFor(s.tool, 0, view.viewType));
    redraw();
  }, [view.id, view.viewType, redraw]);

  const cancelSketch = useCallback(() => {
    const s = useAppStore.getState();
    if (pts.current.length === 0 && s.tool !== "select") s.setTool("select");
    else if (pts.current.length === 0) s.select([]);
    pts.current = [];
    snapRef.current = null;
    s.setPrompt(promptFor(useAppStore.getState().tool, 0, view.viewType));
    redraw();
  }, [view.viewType, redraw]);

  // Keyboard events from App (Esc / Enter) reach the active canvas through window events.
  useEffect(() => {
    const onCancel = () => {
      const s = useAppStore.getState();
      if ((s.tool === "floor" || s.tool === "ceiling") && pts.current.length >= 3) {
        pts.current = [];
        redraw();
        return;
      }
      cancelSketch();
    };
    const onFinish = () => void finishSketch();
    const onFit = () => dl && size.w && setCam(fit(dl.bounds, size.w, size.h));
    window.addEventListener("tool-cancel", onCancel);
    window.addEventListener("tool-finish", onFinish);
    window.addEventListener("view-fit", onFit);
    return () => {
      window.removeEventListener("tool-cancel", onCancel);
      window.removeEventListener("tool-finish", onFinish);
      window.removeEventListener("view-fit", onFit);
    };
  });

  async function click(sx: number, sy: number, shift: boolean) {
    const s = useAppStore.getState();
    if (!cam.current) return;
    const raw = modelAt(sx, sy);
    const tol = 12 / cam.current.zoom;
    if (s.tool === "select") {
      const id = await ipc.pick(view.id, raw, 6 / cam.current.zoom);
      if (!id) s.select(shift ? s.selection : []);
      else if (shift)
        s.select(
          s.selection.includes(id) ? s.selection.filter((x) => x !== id) : [...s.selection, id],
        );
      else s.select([id]);
      return;
    }
    if (!toolAllowed(s.tool, view.viewType)) return;
    if (s.tool === "door" || s.tool === "window") {
      const typeId = s.tool === "door" ? s.toolTypes.door : s.toolTypes.window;
      const pv = typeId ? await ipc.openingPreview(view.id, typeId, raw, tol) : null;
      if (typeId && pv?.valid) {
        await apply(() => ipc.createOpening(typeId, pv.host, pv.offset, pv.flipFacing));
      } else if (pv) {
        s.setError("That spot overlaps another door or window in this wall.");
      }
      preview.current = null;
      redraw();
      return;
    }
    const from = pts.current[pts.current.length - 1] ?? null;
    const p = (await ipc.snap(view.id, raw, from, tol)).pt;
    switch (s.tool) {
      case "wall": {
        if (from && !samePt(from, p) && s.toolTypes.wall) {
          const ok = await apply(() => ipc.createWall(view.id, s.toolTypes.wall!, from, p));
          pts.current = ok ? [p] : pts.current;
        } else if (!from) {
          pts.current = [p];
        }
        break;
      }
      case "grid": {
        if (from && !samePt(from, p)) {
          await apply(() => ipc.createGrid(from, p));
          pts.current = [];
        } else {
          pts.current = [p];
        }
        break;
      }
      case "floor":
      case "ceiling": {
        const first = pts.current[0];
        if (
          first &&
          closesSketch(
            toScreen(cam.current, size.w, size.h, first.x, first.y),
            [sx, sy],
            pts.current.length,
          )
        ) {
          await finishSketch();
          return;
        }
        if (!from || !samePt(from, p)) pts.current = [...pts.current, p];
        break;
      }
      case "floorAuto":
        if (s.toolTypes.floor) await apply(() => ipc.createFloor(view.id, s.toolTypes.floor!, []));
        break;
      case "ceilingAuto":
        if (s.toolTypes.ceiling)
          await apply(() => ipc.createCeiling(view.id, s.toolTypes.ceiling!, [], raw));
        break;
      case "level":
        await apply(() => ipc.createLevel(p.y));
        break;
    }
    s.setPrompt(promptFor(s.tool, pts.current.length, view.viewType));
    redraw();
  }

  async function doubleClick(sx: number, sy: number) {
    const s = useAppStore.getState();
    if (s.tool !== "select" || !cam.current || !s.app) return;
    const id = await ipc.pick(view.id, modelAt(sx, sy), 6 / cam.current.zoom);
    if (!id) return;
    const asView = s.app.views.find((v) => v.id === id);
    const levelPlan = s.app.views.find((v) => v.level === id && v.viewType === "Plan");
    const target = asView ?? levelPlan;
    if (target) s.openView(target.id);
  }

  return (
    <div
      ref={wrapRef}
      className="canvas-wrap"
      data-tool={tool}
      onContextMenu={(e) => e.preventDefault()}
    >
      <canvas
        ref={canvasRef}
        style={{ width: size.w, height: size.h }}
        onWheel={(e) => {
          if (!cam.current) return;
          const [sx, sy] = local(e);
          setCam(zoomAt(cam.current, size.w, size.h, sx, sy, Math.exp(-e.deltaY * 0.0015)));
        }}
        onMouseDown={(e) => {
          const [x, y] = local(e);
          drag.current = { x, y, moved: false, button: e.button };
        }}
        onMouseMove={(e) => {
          const [sx, sy] = local(e);
          const d = drag.current;
          if (d && (d.button === 1 || d.button === 2) && cam.current) {
            const dx = sx - d.x;
            const dy = sy - d.y;
            if (Math.abs(dx) + Math.abs(dy) > 2) d.moved = true;
            if (d.moved) {
              setCam({
                ...cam.current,
                cx: cam.current.cx - dx / cam.current.zoom,
                cy: cam.current.cy + dy / cam.current.zoom,
              });
              drag.current = { ...d, x: sx, y: sy };
            }
            return;
          }
          if (!cam.current) return;
          const p = modelAt(sx, sy);
          const ft = (mm: number) => (mm / 304.8).toFixed(2);
          useAppStore
            .getState()
            .setCursor(`X ${ft(p.x)}'   ${view.viewType === "Elevation" ? "Z" : "Y"} ${ft(p.y)}'`);
          const s = useAppStore.getState();
          if (s.tool === "select") hoverPick(p, 6 / cam.current.zoom);
          else if (s.tool === "door" || s.tool === "window") previewAt(p, 12 / cam.current.zoom);
          else if (toolAllowed(s.tool, view.viewType)) snapAt(p, 12 / cam.current.zoom);
        }}
        onMouseUp={(e) => {
          const d = drag.current;
          drag.current = null;
          if (!d || d.moved) return;
          const [sx, sy] = local(e);
          if (e.button === 0) void click(sx, sy, e.shiftKey);
          if (e.button === 2) {
            const s = useAppStore.getState();
            if ((s.tool === "floor" || s.tool === "ceiling") && pts.current.length >= 3)
              void finishSketch();
            else cancelSketch();
          }
        }}
        onDoubleClick={(e) => {
          const [sx, sy] = local(e);
          void doubleClick(sx, sy);
        }}
        onMouseLeave={() => {
          hover.current = null;
          snapRef.current = null;
          preview.current = null;
          redraw();
        }}
      />
      {!dl && <div className="canvas-loading">Generating view…</div>}
    </div>
  );
}
